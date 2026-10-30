//! Unit tests for the closed decoder.
//!
//! The exhaustive mutation sweep lives in [`crate::ast_mutations`]. These tests
//! pin the decoder's shape on well-formed input and one representative fault
//! per check, so a decoder change that stops reading a field, or that quietly
//! accepts a malformed one, fails here before it reaches the sweep.

use serde_json::{Map, Value as Json};

use crate::{
    ast::{self, ExprKind, Module, Rule},
    corpus::ReasonCode,
    frontend,
};

/// The admitted baseline source.
fn source() -> String {
    match std::fs::read_to_string(crate::corpus::repository_path(
        "examples/tenant-write/policy.rego",
    )) {
        Ok(source) => source,
        Err(error) => panic!("the baseline policy must be readable: {error}"),
    }
}

/// The baseline source's AST JSON.
fn ast_json() -> String {
    let source = source();
    match frontend::parse(&source) {
        Ok(engine) => match engine.get_ast_as_json() {
            Ok(ast_json) => ast_json,
            Err(error) => panic!("the baseline policy must serialize: {error}"),
        },
        Err(message) => panic!("the baseline policy must parse: {message}"),
    }
}

/// Parse AST JSON into a mutable value.
fn value(text: &str) -> Json {
    match serde_json::from_str(text) {
        Ok(value) => value,
        Err(error) => panic!("the baseline AST must be JSON: {error}"),
    }
}

/// The first entry's module object.
fn module_of(root: &mut Json) -> &mut Map<String, Json> {
    let Some(entries) = root.as_array_mut() else {
        panic!("the AST must be an array");
    };
    let Some(entry) = entries.first_mut() else {
        panic!("the AST must hold one entry");
    };
    let Some(ast) = entry.get_mut("ast").and_then(Json::as_object_mut) else {
        panic!("the entry must hold a module object");
    };
    ast
}

/// The first rule's object.
fn first_rule(module: &mut Map<String, Json>) -> &mut Map<String, Json> {
    let Some(rules) = module.get_mut("rules").and_then(Json::as_array_mut) else {
        panic!("the module must hold a rule array");
    };
    let Some(rule) = rules.first_mut().and_then(Json::as_object_mut) else {
        panic!("the module must hold one rule object");
    };
    rule
}

/// Serialize a mutated AST, or fail the test.
fn serialized(root: &Json) -> String {
    match serde_json::to_string(root) {
        Ok(json) => json,
        Err(error) => panic!("the mutated AST must serialize: {error}"),
    }
}

/// Decode `root` and return the fault it must produce.
fn fault(root: &Json, text: &str) -> ast::Fault {
    match ast::decode(text, &serialized(root)) {
        Ok(module) => panic!("the decoder must reject the mutation, decoded {module:?}"),
        Err(fault) => fault,
    }
}

/// Decode the baseline source, or fail the test.
fn baseline() -> Module {
    let text = source();
    match ast::decode(&text, &ast_json()) {
        Ok(module) => module,
        Err(fault) => panic!("the baseline must decode: {}", fault.describe()),
    }
}

#[test]
fn the_baseline_decodes() {
    let module = baseline();
    assert!(
        matches!(&module.package.refr.kind, ExprKind::Var(name) if name == "authz"),
        "the package must be `authz`"
    );
    assert_eq!(module.imports.len(), 1);
    assert_eq!(module.rules.len(), 3, "a default and two normal rules");
    assert!(matches!(module.rules.first(), Some(Rule::Default { .. })));
    assert_eq!(module.target, None);
}

#[test]
fn malformed_json_is_a_serialize_rejection() {
    let text = source();
    for broken in ["{", "not json", ""] {
        match ast::decode(&text, broken) {
            Ok(module) => panic!("the decoder must reject {broken:?}, decoded {module:?}"),
            Err(fault) => assert_eq!(
                fault.code,
                ReasonCode::AstSerialize,
                "{broken:?} must be a serialization fault"
            ),
        }
    }
}

#[test]
fn an_unknown_key_is_rejected() {
    let text = source();
    let mut root = value(&ast_json());
    module_of(&mut root).insert("surprise".to_owned(), Json::Null);
    assert_eq!(fault(&root, &text).code, ReasonCode::AstUnknownKey);
}

#[test]
fn a_missing_required_key_is_rejected() {
    let text = source();
    let mut root = value(&ast_json());
    module_of(&mut root).remove("rego_v1");
    assert_eq!(fault(&root, &text).code, ReasonCode::AstMissingKey);
}

#[test]
fn an_unknown_tag_is_rejected() {
    let text = source();
    let mut root = value(&ast_json());
    let rule = first_rule(module_of(&mut root));
    let Some(body) = rule.remove("Default") else {
        panic!("the first rule must be the default rule");
    };
    rule.insert("Bogus".to_owned(), body);
    assert_eq!(fault(&root, &text).code, ReasonCode::AstUnknownTag);
}

#[test]
fn a_key_beside_a_tag_is_rejected() {
    let text = source();
    let mut root = value(&ast_json());
    first_rule(module_of(&mut root)).insert("surprise".to_owned(), Json::Null);
    assert_eq!(fault(&root, &text).code, ReasonCode::AstUnknownKey);
}

#[test]
fn a_span_outside_the_source_is_rejected() {
    let text = source();
    let mut root = value(&ast_json());
    let Some(package) = module_of(&mut root).get_mut("package") else {
        panic!("the module must declare a package");
    };
    let Some(span) = package.get_mut("span").and_then(Json::as_object_mut) else {
        panic!("the package must carry a span");
    };
    span.insert("end".to_owned(), Json::from(u32::MAX));
    assert_eq!(fault(&root, &text).code, ReasonCode::AstSpan);
}

#[test]
fn a_second_module_is_rejected() {
    let text = source();
    let mut root = value(&ast_json());
    let Some(entries) = root.as_array_mut() else {
        panic!("the AST must be an array");
    };
    let Some(entry) = entries.first().cloned() else {
        panic!("the AST must hold one entry");
    };
    entries.push(entry);
    assert_eq!(fault(&root, &text).code, ReasonCode::AstModuleCount);
}

#[test]
fn a_source_echo_mismatch_is_rejected() {
    let text = source();
    let mut root = value(&ast_json());
    let Some(entries) = root.as_array_mut() else {
        panic!("the AST must be an array");
    };
    let Some(entry) = entries.first_mut() else {
        panic!("the AST must hold one entry");
    };
    let Some(echo) = entry.get_mut("source").and_then(Json::as_object_mut) else {
        panic!("the entry must echo its source");
    };
    echo.insert("contents".to_owned(), Json::from("package authz"));
    assert_eq!(fault(&root, &text).code, ReasonCode::AstSourceMismatch);
}
