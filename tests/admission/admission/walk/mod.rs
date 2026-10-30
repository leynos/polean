//! The profile walk over a decoded module.
//!
//! The checks run in the precedence D16 fixes, so a source with more than one
//! defect always reports the same one: package, import, `__target__`, default,
//! order, rule limit, rule head, rule body, body limit, literal kind, `with`
//! modifier, expression kind, operator kind, then the atom lookup. Each check
//! is a small function over an already-decoded tree; none of them looks at the
//! source text.
//!
//! The walk admits nothing the decoder did not first validate. It reads only
//! the parts of the tree a decision needs and makes no new trust claim: a walk
//! admission is a statement about the frontend's AST, not about the source.
//!
//! The module-level checks live here; the checks over a rule's statements, and
//! the canonical rendering that feeds the atom lookup, live in [`body`].

mod body;

use body::{
    check_body_limits,
    check_expressions,
    check_literals,
    check_operators,
    check_with,
    collect_atoms,
};

use super::{ByteSpan, Rejection};
use crate::{
    ast::{AssignOp, Body, Expr, ExprKind, Head, Member, Module, Rule, Span},
    corpus::{Profile, ReasonCode},
};

/// One admitted module's rule list, with one atom name per literal.
pub(super) type Ir = Vec<Vec<String>>;

/// Check a decoded module and map it to its rule list.
///
/// # Errors
///
/// Returns the first profile-stage rejection, in the order D16 fixes.
pub(super) fn map(profile: &Profile, module: &Module) -> Result<Ir, Rejection> {
    check_package(profile, module)?;
    check_imports(profile, module)?;
    check_target(module)?;
    check_default(profile, module)?;
    check_order(module)?;
    let specs = specs(module);
    check_rule_limit(profile, &specs)?;
    check_heads(profile, &specs)?;
    let bodies = only_bodies(&specs)?;
    check_body_limits(profile, &bodies)?;
    check_literals(&bodies)?;
    check_with(&bodies)?;
    check_expressions(&bodies)?;
    check_operators(&bodies)?;
    collect_atoms(profile, &bodies)
}

/// The parts of a normal rule the walk reads.
struct Spec<'a> {
    /// The rule's source range.
    span: Span,
    /// The rule head.
    head: &'a Head,
    /// The rule bodies, in source order.
    bodies: &'a [Body],
}

/// Collect every normal rule, in source order.
fn specs(module: &Module) -> Vec<Spec<'_>> {
    module
        .rules
        .iter()
        .filter_map(|rule| match rule {
            Rule::Spec { span, head, bodies } => Some(Spec {
                span: *span,
                head,
                bodies,
            }),
            Rule::Default { .. } => None,
        })
        .collect()
}

/// Check that the module declares the profile's package.
fn check_package(profile: &Profile, module: &Module) -> Result<(), Rejection> {
    if is_var(&module.package.refr, &profile.package) {
        return Ok(());
    }
    Err(Rejection::profile(
        ReasonCode::PackageShape,
        Some(range(module.package.span)),
        format!("the package is not `{}`", profile.package),
    ))
}

/// Check that the module declares the profile's one import, unaliased.
fn check_imports(profile: &Profile, module: &Module) -> Result<(), Rejection> {
    let [only] = module.imports.as_slice() else {
        return Err(Rejection::profile(
            ReasonCode::ImportShape,
            module.imports.get(1).map(|import| range(import.span)),
            format!("the module has {} imports, not one", module.imports.len()),
        ));
    };
    if only.alias.is_some() {
        return Err(Rejection::profile(
            ReasonCode::ImportShape,
            Some(range(only.span)),
            "the import is aliased",
        ));
    }
    if text(&only.refr).as_deref() != Some(profile.import.as_str()) {
        return Err(Rejection::profile(
            ReasonCode::ImportShape,
            Some(range(only.span)),
            format!("the import is not `{}`", profile.import),
        ));
    }
    Ok(())
}

/// Check that the module does not set `__target__`.
fn check_target(module: &Module) -> Result<(), Rejection> {
    module.target.as_deref().map_or(Ok(()), |target| {
        Err(Rejection::profile(
            ReasonCode::ModuleTarget,
            None,
            format!("the module sets `__target__` to `{target}`"),
        ))
    })
}

/// Check that the module declares exactly the profile's default rule.
fn check_default(profile: &Profile, module: &Module) -> Result<(), Rejection> {
    let defaults: Vec<&Rule> = module
        .rules
        .iter()
        .filter(|rule| matches!(rule, Rule::Default { .. }))
        .collect();
    let [only] = defaults.as_slice() else {
        return Err(Rejection::profile(
            ReasonCode::DefaultShape,
            defaults.get(1).map(|rule| range(rule.span())),
            format!("the module has {} default rules, not one", defaults.len()),
        ));
    };
    let Rule::Default {
        span,
        refr,
        args,
        op,
        value,
    } = only
    else {
        return Err(Rejection::profile(
            ReasonCode::DefaultShape,
            Some(range(only.span())),
            "the single default candidate is not a default rule",
        ));
    };
    let fault =
        |detail: &str| Rejection::profile(ReasonCode::DefaultShape, Some(range(*span)), detail);
    if !is_var(refr, &profile.rule_name) {
        return Err(fault("the default rule is not for the profile's rule"));
    }
    if !args.is_empty() {
        return Err(fault("the default rule declares arguments"));
    }
    if *op != AssignOp::ColEq {
        return Err(fault("the default rule does not use `:=`"));
    }
    match &value.kind {
        ExprKind::Bool(actual) if *actual == profile.default => Ok(()),
        _ => Err(fault(
            "the default rule does not assign the profile's value",
        )),
    }
}

/// Check that the default rule is the module's first rule.
fn check_order(module: &Module) -> Result<(), Rejection> {
    match module.rules.first() {
        Some(Rule::Default { .. }) | None => Ok(()),
        Some(rule) => Err(Rejection::profile(
            ReasonCode::OrderShape,
            Some(range(rule.span())),
            "a rule appears before the default",
        )),
    }
}

/// Check that the module declares no more rules than the profile permits.
fn check_rule_limit(profile: &Profile, specs: &[Spec<'_>]) -> Result<(), Rejection> {
    let count = specs.len();
    if (profile.rule_min..=profile.rule_max).contains(&count) {
        return Ok(());
    }
    Err(Rejection::profile(
        ReasonCode::RuleLimit,
        specs.get(profile.rule_max).map(|spec| range(spec.span)),
        format!(
            "the module has {count} rules; the profile admits {}..={}",
            profile.rule_min, profile.rule_max
        ),
    ))
}

/// Check that every rule head is the profile's bare complete-rule head.
fn check_heads(profile: &Profile, specs: &[Spec<'_>]) -> Result<(), Rejection> {
    for spec in specs {
        let bare = matches!(
            spec.head,
            Head::Compr {
                refr,
                assign: None,
                ..
            } if is_var(refr, &profile.rule_name)
        );
        if bare {
            continue;
        }
        return Err(Rejection::profile(
            ReasonCode::RuleHead,
            Some(range(spec.head.span())),
            format!("the rule head is not a bare `{}`", profile.rule_name),
        ));
    }
    Ok(())
}

/// Require exactly one non-`else` body with no assignment per rule.
fn only_bodies<'a>(specs: &'a [Spec<'a>]) -> Result<Vec<&'a Body>, Rejection> {
    let mut bodies = Vec::with_capacity(specs.len());
    for spec in specs {
        match spec.bodies {
            [body] if !body.is_else && body.assign.is_none() => bodies.push(body),
            _ => {
                return Err(Rejection::profile(
                    ReasonCode::RuleBody,
                    Some(range(spec.span)),
                    format!(
                        "the rule has {} bodies; the profile admits one plain body",
                        spec.bodies.len()
                    ),
                ));
            }
        }
    }
    Ok(bodies)
}

/// Whether an expression is a variable named `name`.
fn is_var(expr: &Expr, name: &str) -> bool {
    matches!(&expr.kind, ExprKind::Var(value) if value == name)
}

/// Render an expression as the profile's canonical text.
///
/// Only the shapes a registered atom can use are rendered; a comprehension, a
/// call, or any other expression kind returns `None`, which the atom lookup
/// reports as an unregistered comparison. Rendering never consults the source
/// text, so the walk cannot be spoofed by spelling.
fn text(expr: &Expr) -> Option<String> {
    Some(match &expr.kind {
        ExprKind::String(value) => format!("\"{value}\""),
        ExprKind::RawString(value) => format!("`{value}`"),
        ExprKind::Number(value) | ExprKind::Var(value) => value.clone(),
        ExprKind::Bool(value) => value.to_string(),
        ExprKind::Null => "null".to_owned(),
        ExprKind::Array(items) => collection(items, '[', ']')?,
        ExprKind::Set(items) => collection(items, '{', '}')?,
        ExprKind::Object(members) => members_text(members)?,
        ExprKind::RefDot { refr, field } => format!("{}.{}", text(refr)?, field),
        ExprKind::RefBrack { refr, index } => format!("{}[{}]", text(refr)?, text(index)?),
        ExprKind::ArrayCompr { .. }
        | ExprKind::SetCompr { .. }
        | ExprKind::ObjectCompr { .. }
        | ExprKind::Call { .. }
        | ExprKind::Unary { .. }
        | ExprKind::Bin { .. }
        | ExprKind::Compare { .. }
        | ExprKind::Arith { .. }
        | ExprKind::Assign { .. }
        | ExprKind::Membership { .. } => return None,
    })
}

/// Render a collection literal.
fn collection(items: &[Expr], open: char, close: char) -> Option<String> {
    let rendered = items.iter().map(text).collect::<Option<Vec<_>>>()?;
    Some(format!("{open}{}{close}", rendered.join(", ")))
}

/// Render an object literal.
fn members_text(members: &[Member]) -> Option<String> {
    let rendered = members
        .iter()
        .map(|member| Some(format!("{}: {}", text(&member.key)?, text(&member.value)?)))
        .collect::<Option<Vec<_>>>()?;
    Some(format!("{{{}}}", rendered.join(", ")))
}

/// A reported span as a byte range.
fn range(span: Span) -> ByteSpan { (index(span.start), index(span.end)) }

/// A 32-bit offset as a host index.
fn index(offset: u32) -> usize { usize::try_from(offset).unwrap_or(0) }
