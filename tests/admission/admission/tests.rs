//! Tests for the profile walk's shape and precedence.
//!
//! Each test starts from the admitted baseline module and breaks exactly one
//! thing, so a failure says which check ran and which one did not. The whole
//! corpus is exercised by the O3 test in [`crate::corpus`]; these tests pin the
//! precedence itself, which one fixture per defect cannot observe.

use super::walk;
use crate::{
    ast::{self, AssignOp, Body, Expr, ExprKind, Head, Literal, Module, Rule, Span},
    corpus::{self, Profile, ReasonCode},
    frontend,
};

/// The baseline module, decoded from the admitted policy.
fn baseline() -> Module {
    let source =
        match std::fs::read_to_string(corpus::repository_path("examples/tenant-write/policy.rego"))
        {
            Ok(source) => source,
            Err(error) => panic!("the baseline policy must be readable: {error}"),
        };
    let ast_json = match frontend::parse(&source) {
        Ok(engine) => match engine.get_ast_as_json() {
            Ok(ast_json) => ast_json,
            Err(error) => panic!("the baseline policy must serialize: {error}"),
        },
        Err(message) => panic!("the baseline policy must parse: {message}"),
    };
    match ast::decode(&source, &ast_json) {
        Ok(module) => module,
        Err(fault) => panic!("the baseline policy must decode: {}", fault.describe()),
    }
}

/// The profile the walk enforces.
fn profile() -> Profile {
    match corpus::profile_contract() {
        Ok(profile) => profile,
        Err(error) => panic!("the profile must parse: {error}"),
    }
}

/// A variable expression with an empty but valid span.
fn var(name: &str) -> Expr { Expr::new(Span::new(1, 1, 0, 0), ExprKind::Var(name.to_owned())) }

/// The first normal rule's first body.
fn first_body(module: &mut Module) -> &mut Body {
    let rule = module
        .rules
        .iter_mut()
        .find(|rule| matches!(rule, Rule::Spec { .. }));
    let Some(Rule::Spec { bodies, .. }) = rule else {
        panic!("the baseline must contain a normal rule");
    };
    let Some(body) = bodies.first_mut() else {
        panic!("the baseline rule must contain a body");
    };
    body
}

/// The first statement's expression.
fn first_expression(body: &mut Body) -> &mut Expr {
    let Some(stmt) = body.query.stmts.first_mut() else {
        panic!("the baseline body must contain a statement");
    };
    let Literal::Expr(expr) = &mut stmt.literal else {
        panic!("the baseline statement must be an expression");
    };
    expr
}

/// The rejection a broken module must produce.
fn rejection(module: &Module) -> super::Rejection {
    match walk::map(&profile(), module) {
        Ok(ir) => panic!("the walk must reject the module, admitted {ir:?}"),
        Err(rejection) => rejection,
    }
}

#[test]
fn the_baseline_walks_to_the_profile_ir() {
    let ir = match walk::map(&profile(), &baseline()) {
        Ok(ir) => ir,
        Err(rejection) => panic!("the baseline must be admitted: {}", rejection.describe()),
    };
    let expected = match corpus::ir_rules("examples/tenant-write/policy.json") {
        Ok(rules) => rules,
        Err(error) => panic!("{error}"),
    };
    assert_eq!(ir, expected, "the walk must reproduce the hand-written IR");
}

#[test]
fn the_package_is_checked_before_the_import() {
    let mut module = baseline();
    module.package.refr = var("other");
    module.imports.clear();
    let rejection = rejection(&module);
    assert_eq!(rejection.code, ReasonCode::PackageShape);
    assert_eq!(rejection.stage, corpus::Stage::Profile);
}

#[test]
fn the_target_is_checked_before_the_default() {
    let mut module = baseline();
    module.target = Some("rego".to_owned());
    module
        .rules
        .retain(|rule| !matches!(rule, Rule::Default { .. }));
    assert_eq!(rejection(&module).code, ReasonCode::ModuleTarget);
}

#[test]
fn the_rule_head_is_checked_before_the_body() {
    let mut module = baseline();
    let rule = module
        .rules
        .iter_mut()
        .find(|rule| matches!(rule, Rule::Spec { .. }));
    let Some(Rule::Spec { head, bodies, .. }) = rule else {
        panic!("the baseline must contain a normal rule");
    };
    *head = Head::Compr {
        span: head.span(),
        refr: var("other"),
        assign: None,
    };
    bodies.clear();
    assert_eq!(rejection(&module).code, ReasonCode::RuleHead);
}

#[test]
fn a_comparison_whose_value_is_unregistered_is_an_unknown_atom() {
    let mut module = baseline();
    let expr = first_expression(first_body(&mut module));
    let ExprKind::Compare { rhs, .. } = &mut expr.kind else {
        panic!("the baseline statement must be a comparison");
    };
    rhs.kind = ExprKind::String("read".to_owned());
    assert_eq!(rejection(&module).code, ReasonCode::AtomUnknown);
}

#[test]
fn a_default_with_the_wrong_operator_is_a_default_shape_fault() {
    let mut module = baseline();
    let rule = module
        .rules
        .iter_mut()
        .find(|rule| matches!(rule, Rule::Default { .. }));
    let Some(Rule::Default { op, .. }) = rule else {
        panic!("the baseline must contain a default rule");
    };
    *op = AssignOp::Eq;
    assert_eq!(rejection(&module).code, ReasonCode::DefaultShape);
}
