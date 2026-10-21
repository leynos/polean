//! In-process Regorus observations.
//!
//! Two properties are checked here. The first is that a policy calling a
//! built-in from a disabled feature group cannot produce a decision, which is
//! evidence that the compiled feature set is the recorded one. The second is
//! that the adapter input for the baseline source has the structure 1.2.2
//! relies on, snapshotted so that a change to Regorus's AST JSON is visible
//! rather than silent.
//!
//! Both are observations of a frontend, not admission: a policy that parses is
//! not thereby admitted, and node-by-node AST coverage belongs to 1.2.2.

use rstest::rstest;
use serde_json::Value as Json;

use crate::{
    engine::{POLICY_FILE, decide, engine_for},
    json::{array, boolean, expect_equal, field, parse_json, single_tag, string},
    support::{BASELINE_SOURCE, DENY_ALL_SOURCE},
};

/// A policy that calls `regex.match`, a built-in from the disabled `regex`
/// feature group.
const REGEX_CALLING_SOURCE: &str = r#"package authz

import rego.v1

default allow := false

allow if {
    regex.match("^subject-.*$", input.subject.id)
}
"#;

/// The same policy shape without a built-in call, so the negative test is not
/// vacuous for want of a well-formed module.
const BUILTIN_FREE_SOURCE: &str = r#"package authz

import rego.v1

default allow := false

allow if {
    input.subject.id == "subject-a"
}
"#;

/// A request matching both probe policies.
const SUBJECT_A_REQUEST: &str = r#"{
  "subject": {
    "tenant": "tenant-a",
    "id": "subject-a",
    "role": "member"
  },
  "resource": {
    "tenant": "tenant-a",
    "owner": "subject-a",
    "locked": false
  },
  "action": "write"
}"#;

/// The AST JSON for a single source.
///
/// # Errors
///
/// Returns a diagnostic when the source does not parse.
fn ast_json(source: &str) -> Result<String, String> {
    engine_for(source)?.get_ast_as_json().map_err(|error| {
        format!(
            "get_ast_as_json failed; the `ast` feature is required for this observation: {error}"
        )
    })
}

/// The payload under a single-tag rule object.
///
/// # Errors
///
/// Returns a diagnostic when the rule has no field named `kind`.
fn rule_payload<'a>(rule: &'a Json, kind: &str, context: &str) -> Result<&'a Json, String> {
    field(rule, kind, context)
}

/// O3: the `regex` feature group is not compiled in.
///
/// A policy calling `regex.match` must not produce a decision, while the same
/// policy shape without a built-in call must. The test asserts only that the
/// combined parse-and-evaluate step fails: the message wording is Regorus's,
/// and it already changed between 0.12.0 and 0.13.0.
#[rstest]
fn disabled_feature_group_cannot_decide() {
    let regex_result = decide(REGEX_CALLING_SOURCE, SUBJECT_A_REQUEST);
    assert!(
        regex_result.is_err(),
        "a `regex.match` call must not decide under the pinned feature set, got {regex_result:?}"
    );

    let builtin_free_result = decide(BUILTIN_FREE_SOURCE, SUBJECT_A_REQUEST);
    let Ok(decision) = builtin_free_result else {
        let reason = builtin_free_result
            .err()
            .unwrap_or_else(|| "unexpected failure".to_owned());
        panic!("the built-in-free probe must decide, but failed: {reason}");
    };
    assert_eq!(
        decision,
        regorus::Value::Bool(true),
        "the built-in-free probe must decide `true`"
    );
}

/// The module records the exact source name and bytes it was built from.
///
/// # Errors
///
/// Returns a diagnostic when either recorded field differs.
fn check_recorded_source(module: &Json) -> Result<(), String> {
    let source = field(module, "source", "module")?;
    expect_equal(
        &string(source, "file", "module.source")?,
        &POLICY_FILE,
        "the recorded source name",
    )?;
    expect_equal(
        &string(source, "contents", "module.source")?,
        &BASELINE_SOURCE,
        "the recorded source bytes",
    )
}

/// The module declares package `authz` and records Rego v1.
///
/// # Errors
///
/// Returns a diagnostic when either observation differs.
fn check_package(module_ast: &Json) -> Result<(), String> {
    let package = field(module_ast, "package", "module.ast")?;
    let package_ref = field(package, "refr", "module.ast.package")?;
    expect_equal(
        &package_ref.pointer("/Var/value").and_then(Json::as_str),
        &Some("authz"),
        "the package name",
    )?;
    if !boolean(module_ast, "rego_v1", "module.ast")? {
        return Err("the module must record Rego v1".to_owned());
    }
    Ok(())
}

/// The number of rules that are a `Default`.
///
/// # Errors
///
/// Returns a diagnostic when a rule has no single tag.
fn count_default_rules(rules: &[Json]) -> Result<u8, String> {
    let mut count = 0_u8;
    for rule in rules {
        if single_tag(rule, "rule", "rule")? == "Default" {
            count += 1;
        }
    }
    Ok(count)
}

/// The statement count of the single body of one `Spec` rule.
///
/// The head and its reference are checked here too, so a `Spec` rule for a
/// different name cannot satisfy the structure.
///
/// # Errors
///
/// Returns a diagnostic when the rule's shape differs.
fn spec_body_length(rule: &Json, kind: &str) -> Result<usize, String> {
    const CONTEXT: &str = "rule.Spec";
    let payload = rule_payload(rule, kind, CONTEXT)?;
    let head = field(payload, "head", CONTEXT)?;
    expect_equal(
        &single_tag(head, "rule head", "rule.Spec.head")?,
        &"Compr",
        &format!("{CONTEXT}: head"),
    )?;
    expect_equal(
        &head.pointer("/Compr/refr/Var/value").and_then(Json::as_str),
        &Some("allow"),
        &format!("{CONTEXT}: head refers to `allow`"),
    )?;
    let bodies = array(payload, "bodies", CONTEXT)?;
    expect_equal(&bodies.len(), &1, &format!("{CONTEXT}: one body expected"))?;
    let body = bodies
        .first()
        .ok_or_else(|| format!("{CONTEXT}: body missing"))?;
    let query = field(body, "query", CONTEXT)?;
    Ok(array(query, "stmts", "rule.Spec.body.query")?.len())
}

/// Body statement counts for each `Spec` rule, in source order.
///
/// # Errors
///
/// Returns a diagnostic when a rule has no single tag or an unexpected shape.
fn spec_body_lengths(rules: &[Json]) -> Result<Vec<usize>, String> {
    let mut lengths = Vec::new();
    for rule in rules {
        let kind = single_tag(rule, "rule", "rule")?;
        if kind == "Spec" {
            lengths.push(spec_body_length(rule, kind)?);
        }
    }
    Ok(lengths)
}

/// The module has one default and two spec rules with the expected shape.
///
/// # Errors
///
/// Returns a diagnostic naming the first observation that differs.
fn check_rules(module_ast: &Json) -> Result<(), String> {
    let rules = array(module_ast, "rules", "module.ast")?;
    expect_equal(&rules.len(), &3, "one default and two spec rules expected")?;
    expect_equal(
        &count_default_rules(rules)?,
        &1,
        "one default rule expected",
    )?;
    let mut body_lengths = spec_body_lengths(rules)?;
    body_lengths.sort_unstable();
    expect_equal(
        &body_lengths,
        &vec![3, 4],
        "the two spec bodies carry three and four statements",
    )
}

/// O5: structural facts about the baseline adapter input.
///
/// These assertions are independent of the snapshot: they name what 1.2.2 will
/// consume, so a snapshot accepted without reading it cannot hide a change of
/// shape.
///
/// # Errors
///
/// Returns a diagnostic naming the first observation that differs.
#[test]
fn baseline_ast_has_the_expected_structure() -> Result<(), String> {
    let ast = ast_json(BASELINE_SOURCE)?;
    let root = parse_json(&ast, "baseline AST JSON")?;
    let documents = root
        .as_array()
        .ok_or_else(|| "baseline AST JSON is not an array".to_owned())?;
    expect_equal(&documents.len(), &1, "one policy module expected")?;
    let module = documents
        .first()
        .ok_or_else(|| "baseline AST JSON is empty".to_owned())?;
    check_recorded_source(module)?;
    let module_ast = field(module, "ast", "module")?;
    check_package(module_ast)?;
    check_rules(module_ast)
}

/// O5, non-vacuity: the deny-all mutation has no spec rule.
///
/// Without this, the structural assertions above could pass on an AST shape
/// that is insensitive to the source it is given.
///
/// # Errors
///
/// Returns a diagnostic when the module or rule differs.
#[test]
fn deny_all_ast_has_only_a_default_rule() -> Result<(), String> {
    let ast = ast_json(DENY_ALL_SOURCE)?;
    let root = parse_json(&ast, "deny-all AST JSON")?;
    let documents = root
        .as_array()
        .ok_or_else(|| "deny-all AST JSON is not an array".to_owned())?;
    let module = documents
        .first()
        .ok_or_else(|| "deny-all AST JSON has no module".to_owned())?;
    let rules = array(field(module, "ast", "module")?, "rules", "module.ast")?;
    expect_equal(&rules.len(), &1, "only the default rule remains")?;
    let remaining = rules
        .first()
        .ok_or_else(|| "deny-all rule missing".to_owned())?;
    expect_equal(
        &single_tag(remaining, "rule", "rule")?,
        &"Default",
        "the remaining rule is a default",
    )
}

/// O5: the raw adapter input for the exact baseline bytes, snapshotted.
///
/// The snapshot is a change detector, not a conformance claim. Review it with
/// `cargo insta review`; see the developers' guide.
#[test]
fn baseline_ast_json_matches_the_reviewed_snapshot() -> Result<(), String> {
    let ast = ast_json(BASELINE_SOURCE)?;
    insta::assert_snapshot!("baseline_ast_json", ast);
    Ok(())
}
