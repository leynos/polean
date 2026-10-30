//! Count the node kinds of a decoded module.
//!
//! This is the JSON side of O1. It mirrors [`crate::inventory::walk`] node for
//! node, so the multiset a decoded module produces equals the multiset the
//! typed walk produces whenever the decoder loses or invents nothing.
//!
//! Every `match` is exhaustive and has no wildcard, so a new decoded variant
//! stops compilation until it is given a kind.

use super::{
    ArithOp,
    AssignOp,
    BinOp,
    Body,
    BoolOp,
    Expr,
    ExprKind,
    Head,
    Literal,
    Member,
    Module,
    Query,
    Rule,
    Stmt,
};
use crate::inventory::{KindCounts, NodeKind};

/// Count the node kinds of one decoded module.
#[must_use]
pub fn count_module(module: &Module) -> KindCounts {
    let mut counts = KindCounts::new();
    visit_expr(&module.package.refr, &mut counts);
    for import in &module.imports {
        visit_expr(&import.refr, &mut counts);
    }
    for rule in &module.rules {
        visit_rule(rule, &mut counts);
    }
    counts
}

/// Count one occurrence of `kind`.
fn bump(counts: &mut KindCounts, kind: NodeKind) { *counts.entry(kind).or_default() += 1; }

/// Count one rule.
fn visit_rule(rule: &Rule, counts: &mut KindCounts) {
    match rule {
        Rule::Spec { head, bodies, .. } => {
            bump(counts, NodeKind::RuleSpec);
            visit_head(head, counts);
            for body in bodies {
                visit_body(body, counts);
            }
        }
        Rule::Default {
            refr,
            args,
            op,
            value,
            ..
        } => {
            bump(counts, NodeKind::RuleDefault);
            bump(counts, assign_op_kind(*op));
            visit_expr(refr, counts);
            for arg in args {
                visit_expr(arg, counts);
            }
            visit_expr(value, counts);
        }
    }
}

/// Count one rule head, including any value assignment it carries.
fn visit_head(head: &Head, counts: &mut KindCounts) {
    match head {
        Head::Compr { refr, assign, .. } => {
            bump(counts, NodeKind::HeadCompr);
            visit_expr(refr, counts);
            visit_optional_assign(assign.as_ref(), counts);
        }
        Head::Set { refr, key, .. } => {
            bump(counts, NodeKind::HeadSet);
            visit_expr(refr, counts);
            visit_optional_expr(key.as_ref(), counts);
        }
        Head::Func {
            refr, args, assign, ..
        } => {
            bump(counts, NodeKind::HeadFunc);
            visit_expr(refr, counts);
            for arg in args {
                visit_expr(arg, counts);
            }
            visit_optional_assign(assign.as_ref(), counts);
        }
    }
}

/// Count one rule body: its optional assignment and its query.
fn visit_body(body: &Body, counts: &mut KindCounts) {
    visit_optional_assign(body.assign.as_ref(), counts);
    visit_query(&body.query, counts);
}

/// Count an optional assignment.
fn visit_optional_assign(assign: Option<&super::Assign>, counts: &mut KindCounts) {
    if let Some(pending) = assign {
        bump(counts, assign_op_kind(pending.op));
        visit_expr(&pending.value, counts);
    }
}

/// Count every statement of a query.
fn visit_query(query: &Query, counts: &mut KindCounts) {
    for statement in &query.stmts {
        visit_stmt(statement, counts);
    }
}

/// Count one statement: its literal and its `with` modifiers.
fn visit_stmt(statement: &Stmt, counts: &mut KindCounts) {
    visit_literal(&statement.literal, counts);
    for with_mod in &statement.with_mods {
        visit_expr(&with_mod.refr, counts);
        visit_expr(&with_mod.target, counts);
    }
}

/// Count one literal.
fn visit_literal(literal: &Literal, counts: &mut KindCounts) {
    match literal {
        Literal::SomeVars(_) => bump(counts, NodeKind::LiteralSomeVars),
        Literal::SomeIn {
            key,
            value,
            collection,
        } => {
            bump(counts, NodeKind::LiteralSomeIn);
            visit_optional_expr(key.as_ref(), counts);
            visit_expr(value, counts);
            visit_expr(collection, counts);
        }
        Literal::Expr(expr) => {
            bump(counts, NodeKind::LiteralExpr);
            visit_expr(expr, counts);
        }
        Literal::NotExpr(expr) => {
            bump(counts, NodeKind::LiteralNotExpr);
            visit_expr(expr, counts);
        }
        Literal::Every { domain, query } => {
            bump(counts, NodeKind::LiteralEvery);
            visit_expr(domain, counts);
            visit_query(query, counts);
        }
    }
}

/// Count an optional expression.
fn visit_optional_expr(expr: Option<&Expr>, counts: &mut KindCounts) {
    if let Some(node) = expr {
        visit_expr(node, counts);
    }
}

/// Count an expression and everything it contains.
fn visit_expr(expr: &Expr, counts: &mut KindCounts) {
    match &expr.kind {
        ExprKind::String(_) => bump(counts, NodeKind::ExprString),
        ExprKind::RawString(_) => bump(counts, NodeKind::ExprRawString),
        ExprKind::Number(_) => bump(counts, NodeKind::ExprNumber),
        ExprKind::Bool(_) => bump(counts, NodeKind::ExprBool),
        ExprKind::Null => bump(counts, NodeKind::ExprNull),
        ExprKind::Var(_) => bump(counts, NodeKind::ExprVar),
        ExprKind::Array(items) => visit_collection(counts, NodeKind::ExprArray, items),
        ExprKind::Set(items) => visit_collection(counts, NodeKind::ExprSet, items),
        ExprKind::Object(members) => visit_object(members, counts),
        ExprKind::ArrayCompr { term, query } => {
            visit_comprehension(counts, NodeKind::ExprArrayCompr, term, query);
        }
        ExprKind::SetCompr { term, query } => {
            visit_comprehension(counts, NodeKind::ExprSetCompr, term, query);
        }
        ExprKind::ObjectCompr { key, value, query } => {
            visit_object_comprehension(counts, key, value, query);
        }
        ExprKind::Call { fcn, params } => visit_call(fcn, params, counts),
        ExprKind::Unary { expr: operand } => visit_unary(operand, counts),
        ExprKind::RefDot { refr, .. } => visit_ref_dot(refr, counts),
        ExprKind::RefBrack { refr, index } => visit_ref_brack(refr, index, counts),
        ExprKind::Bin { op, lhs, rhs } => {
            visit_pair(counts, (NodeKind::ExprBin, bin_op_kind(*op)), (lhs, rhs));
        }
        ExprKind::Compare { op, lhs, rhs } => {
            visit_pair(
                counts,
                (NodeKind::ExprCompare, bool_op_kind(*op)),
                (lhs, rhs),
            );
        }
        ExprKind::Arith { op, lhs, rhs } => {
            visit_pair(
                counts,
                (NodeKind::ExprArith, arith_op_kind(*op)),
                (lhs, rhs),
            );
        }
        ExprKind::Assign { op, lhs, rhs } => {
            visit_pair(
                counts,
                (NodeKind::ExprAssign, assign_op_kind(*op)),
                (lhs, rhs),
            );
        }
        ExprKind::Membership {
            key,
            value,
            collection,
        } => visit_membership(key.as_deref(), value, collection, counts),
    }
}

/// Count one object literal: its kind, then each member's key and value.
fn visit_object(members: &[Member], counts: &mut KindCounts) {
    bump(counts, NodeKind::ExprObject);
    for member in members {
        visit_expr(&member.key, counts);
        visit_expr(&member.value, counts);
    }
}

/// Count one object comprehension: its kind, key, value, and body.
fn visit_object_comprehension(counts: &mut KindCounts, key: &Expr, value: &Expr, query: &Query) {
    bump(counts, NodeKind::ExprObjectCompr);
    visit_expr(key, counts);
    visit_expr(value, counts);
    visit_query(query, counts);
}

/// Count a call: its kind, the callee, and each argument.
fn visit_call(fcn: &Expr, params: &[Expr], counts: &mut KindCounts) {
    bump(counts, NodeKind::ExprCall);
    visit_expr(fcn, counts);
    visit_all(params, counts);
}

/// Count a unary expression: its kind and its operand.
fn visit_unary(operand: &Expr, counts: &mut KindCounts) {
    bump(counts, NodeKind::ExprUnary);
    visit_expr(operand, counts);
}

/// Count a dotted reference: its kind and the reference it extends.
fn visit_ref_dot(refr: &Expr, counts: &mut KindCounts) {
    bump(counts, NodeKind::ExprRefDot);
    visit_expr(refr, counts);
}

/// Count a bracketed reference: its kind, the reference, and the index.
fn visit_ref_brack(refr: &Expr, index: &Expr, counts: &mut KindCounts) {
    bump(counts, NodeKind::ExprRefBrack);
    visit_expr(refr, counts);
    visit_expr(index, counts);
}

/// Count a two-sided expression: its own kind, its operator's kind, and its sides.
fn visit_pair(counts: &mut KindCounts, kinds: (NodeKind, NodeKind), sides: (&Expr, &Expr)) {
    bump(counts, kinds.0);
    bump(counts, kinds.1);
    visit_expr(sides.0, counts);
    visit_expr(sides.1, counts);
}

/// Count a membership expression: its kind, optional key, value, and collection.
fn visit_membership(key: Option<&Expr>, value: &Expr, collection: &Expr, counts: &mut KindCounts) {
    bump(counts, NodeKind::ExprMembership);
    visit_optional_expr(key, counts);
    visit_expr(value, counts);
    visit_expr(collection, counts);
}

/// Count every expression in a sequence.
fn visit_all(exprs: &[Expr], counts: &mut KindCounts) {
    for expr in exprs {
        visit_expr(expr, counts);
    }
}

/// Count a collection literal: its kind and its items.
fn visit_collection(counts: &mut KindCounts, kind: NodeKind, items: &[Expr]) {
    bump(counts, kind);
    visit_all(items, counts);
}

/// Count a comprehension: its kind, its term, and its body.
fn visit_comprehension(counts: &mut KindCounts, kind: NodeKind, term: &Expr, query: &Query) {
    bump(counts, kind);
    visit_expr(term, counts);
    visit_query(query, counts);
}

/// Map a binary set operator to its kind.
const fn bin_op_kind(op: BinOp) -> NodeKind {
    match op {
        BinOp::Intersection => NodeKind::BinOpIntersection,
        BinOp::Union => NodeKind::BinOpUnion,
    }
}

/// Map an arithmetic operator to its kind.
const fn arith_op_kind(op: ArithOp) -> NodeKind {
    match op {
        ArithOp::Add => NodeKind::ArithOpAdd,
        ArithOp::Sub => NodeKind::ArithOpSub,
        ArithOp::Mul => NodeKind::ArithOpMul,
        ArithOp::Div => NodeKind::ArithOpDiv,
        ArithOp::Mod => NodeKind::ArithOpMod,
    }
}

/// Map a comparison operator to its kind.
const fn bool_op_kind(op: BoolOp) -> NodeKind {
    match op {
        BoolOp::Lt => NodeKind::BoolOpLt,
        BoolOp::Le => NodeKind::BoolOpLe,
        BoolOp::Eq => NodeKind::BoolOpEq,
        BoolOp::Ge => NodeKind::BoolOpGe,
        BoolOp::Gt => NodeKind::BoolOpGt,
        BoolOp::Ne => NodeKind::BoolOpNe,
    }
}

/// Map an assignment operator to its kind.
const fn assign_op_kind(op: AssignOp) -> NodeKind {
    match op {
        AssignOp::Eq => NodeKind::AssignOpEq,
        AssignOp::ColEq => NodeKind::AssignOpColEq,
    }
}
