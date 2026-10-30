//! The profile walk over a rule's statements, and the atom lookup.
//!
//! These checks run after every module-level check in [`super`] has passed, so
//! they may assume a module with the profile's package, import, default, and
//! rule heads. They still validate each statement's shape rather than trust
//! it: the decoder accepted the frontend's tree, not the profile's subset.

use super::{Ir, Rejection, range, text};
use crate::{
    ast::{Body, BoolOp, Expr, ExprKind, Literal, Span, Stmt},
    corpus::{Profile, ReasonCode},
};

/// Check that every body's literal count is within the profile's bounds.
pub(super) fn check_body_limits(profile: &Profile, bodies: &[&Body]) -> Result<(), Rejection> {
    for body in bodies {
        let count = body.query.stmts.len();
        if (profile.body_min..=profile.body_max).contains(&count) {
            continue;
        }
        return Err(Rejection::profile(
            ReasonCode::BodyLimit,
            Some(range(body.span)),
            format!(
                "the body has {count} literals; the profile admits {}..={}",
                profile.body_min, profile.body_max
            ),
        ));
    }
    Ok(())
}

/// Check that every statement is an expression statement.
pub(super) fn check_literals(bodies: &[&Body]) -> Result<(), Rejection> {
    for stmt in statements(bodies) {
        if matches!(&stmt.literal, Literal::Expr(_)) {
            continue;
        }
        return Err(Rejection::profile(
            ReasonCode::LiteralKind,
            Some(range(stmt.span)),
            "the statement is not an expression",
        ));
    }
    Ok(())
}

/// Check that no statement carries a `with` modifier.
pub(super) fn check_with(bodies: &[&Body]) -> Result<(), Rejection> {
    for stmt in statements(bodies) {
        if stmt.with_mods.is_empty() {
            continue;
        }
        let span = stmt
            .with_mods
            .first()
            .map_or(stmt.span, |modifier| modifier.span);
        return Err(Rejection::profile(
            ReasonCode::WithModifier,
            Some(range(span)),
            "the statement carries a `with` modifier",
        ));
    }
    Ok(())
}

/// Check that every statement is a comparison over admissible operands.
pub(super) fn check_expressions(bodies: &[&Body]) -> Result<(), Rejection> {
    for stmt in statements(bodies) {
        let Some(expr) = expression(stmt) else {
            continue;
        };
        let ExprKind::Compare { lhs, rhs, .. } = &expr.kind else {
            return Err(Rejection::profile(
                ReasonCode::ExprKind,
                Some(range(expr.span)),
                "the statement is not a comparison",
            ));
        };
        for side in [lhs, rhs] {
            if is_rejected_operand(&side.kind) {
                return Err(Rejection::profile(
                    ReasonCode::ExprKind,
                    Some(range(side.span)),
                    "a comparison operand is not a variable, reference, or literal",
                ));
            }
        }
    }
    Ok(())
}

/// Check that every comparison uses `==`.
pub(super) fn check_operators(bodies: &[&Body]) -> Result<(), Rejection> {
    for stmt in statements(bodies) {
        match expression(stmt).map(|expr| (&expr.kind, expr.span)) {
            Some((ExprKind::Compare { op, .. }, span)) if *op != BoolOp::Eq => {
                return Err(Rejection::profile(
                    ReasonCode::OperatorKind,
                    Some(range(span)),
                    "the comparison does not use `==`",
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

/// Map every statement to its atom name.
pub(super) fn collect_atoms(profile: &Profile, bodies: &[&Body]) -> Result<Ir, Rejection> {
    let mut rules = Vec::with_capacity(bodies.len());
    for body in bodies {
        let mut atoms = Vec::with_capacity(body.query.stmts.len());
        for stmt in &body.query.stmts {
            let Some(expr) = expression(stmt) else {
                continue;
            };
            let ExprKind::Compare { lhs, rhs, .. } = &expr.kind else {
                continue;
            };
            let name = atom_name(profile, lhs, rhs, expr.span)?;
            atoms.push(name.to_owned());
        }
        rules.push(atoms);
    }
    Ok(rules)
}

/// Look up one comparison's canonical text in the profile's atom table.
fn atom_name<'a>(
    profile: &'a Profile,
    lhs: &Expr,
    rhs: &Expr,
    span: Span,
) -> Result<&'a str, Rejection> {
    let canonical = match (text(lhs), text(rhs)) {
        (Some(left), Some(right)) => format!("{left} == {right}"),
        _ => {
            return Err(Rejection::profile(
                ReasonCode::AtomUnknown,
                Some(range(span)),
                "the comparison is not a registered atom",
            ));
        }
    };
    profile.atom_name(&canonical).ok_or_else(|| {
        Rejection::profile(
            ReasonCode::AtomUnknown,
            Some(range(span)),
            format!("`{canonical}` is not a registered atom"),
        )
    })
}

/// Whether a comparison operand is an expression kind the profile excludes.
const fn is_rejected_operand(kind: &ExprKind) -> bool {
    matches!(
        kind,
        ExprKind::Call { .. }
            | ExprKind::Unary { .. }
            | ExprKind::Bin { .. }
            | ExprKind::Arith { .. }
            | ExprKind::Assign { .. }
            | ExprKind::Membership { .. }
    )
}

/// Every statement of every body, in source order.
fn statements<'a>(bodies: &'a [&'a Body]) -> impl Iterator<Item = &'a Stmt> {
    bodies.iter().flat_map(|body| body.query.stmts.iter())
}

/// A statement's expression, when its literal is an expression statement.
const fn expression(stmt: &Stmt) -> Option<&Expr> {
    match &stmt.literal {
        Literal::Expr(expr) => Some(expr),
        Literal::SomeVars(_)
        | Literal::SomeIn { .. }
        | Literal::NotExpr(_)
        | Literal::Every { .. } => None,
    }
}
