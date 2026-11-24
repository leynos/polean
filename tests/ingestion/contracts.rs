//! The profile constants agree with the machine-readable contracts.
//!
//! `contracts/profile.json` is the authority for the profile identifier and the
//! size and rule bounds; `contracts/policy.schema.json` is the authority for the
//! atom names and order. A constant that drifts from either would make the
//! library enforce a bound the profile does not state, so each comparison names
//! the field it reads and a missing field is a failure rather than a default.

use polean::profile::{
    BODY_ATOMS_MAX,
    BODY_ATOMS_MIN,
    INPUT_BYTES_MAX,
    PROFILE_ID,
    RULES_MAX,
    SOURCE_BYTES_MAX,
};
use serde_json::Value;

use crate::support::fixture_text;

/// The profile contract, parsed.
fn profile_contract() -> Value {
    let text = fixture_text("contracts/profile.json").expect("the profile contract is readable");
    serde_json::from_str(&text).expect("the profile contract is JSON")
}

/// A contract field that must be present.
///
/// A missing field is a failure rather than a default: the comparison would
/// otherwise pass against a value the contract does not state.
fn required<'a>(value: Option<&'a Value>, field: &str) -> &'a Value {
    // `let ... else` keeps the panic out of a combinator closure, which is the
    // shape Whitaker's `no_unwrap_or_else_panic` targets, while still naming
    // the missing field.
    let Some(found) = value else {
        panic!("contracts/profile.json has no `{field}`");
    };
    found
}

/// A JSON number that a `usize` constant is compared against.
///
/// `field` names the contract field in the failure, so a missing or non-integer
/// value says which field is wrong rather than only that one is.
fn as_usize(value: &Value, field: &str) -> usize {
    let Some(number) = value.as_u64() else {
        panic!("the contract field `{field}` is not a non-negative integer");
    };
    usize::try_from(number).expect("a contract count fits a usize")
}

#[test]
fn the_profile_identifier_agrees_with_the_contract() {
    let contract = profile_contract();

    assert_eq!(
        contract.get("id").and_then(Value::as_str),
        Some(PROFILE_ID),
        "the profile identifier in src/profile.rs must equal contracts/profile.json's `id`"
    );
}

#[test]
fn the_source_limit_agrees_with_the_contract() {
    let contract = profile_contract();
    let limit = required(
        contract
            .get("limits")
            .and_then(|limits| limits.get("source_bytes")),
        "limits.source_bytes",
    );

    assert_eq!(
        as_usize(limit, "limits.source_bytes"),
        SOURCE_BYTES_MAX,
        "the source limit in src/profile.rs must equal the contract's"
    );
}

#[test]
fn the_input_limit_agrees_with_the_contract() {
    let contract = profile_contract();
    let limit = required(
        contract
            .get("limits")
            .and_then(|limits| limits.get("input_bytes")),
        "limits.input_bytes",
    );

    assert_eq!(
        as_usize(limit, "limits.input_bytes"),
        INPUT_BYTES_MAX,
        "the input limit in src/profile.rs must equal the contract's"
    );
}

#[test]
fn the_rule_and_body_bounds_agree_with_the_contract() {
    let contract = profile_contract();
    let rules = required(contract.get("rules"), "rules");
    let field = |name: &str| required(rules.get(name), name);

    assert_eq!(
        as_usize(field("max"), "rules.max"),
        RULES_MAX,
        "the rule limit in src/profile.rs must equal the contract's"
    );
    assert_eq!(
        as_usize(field("body_min"), "rules.body_min"),
        BODY_ATOMS_MIN,
        "the body minimum in src/profile.rs must equal the contract's"
    );
    assert_eq!(
        as_usize(field("body_max"), "rules.body_max"),
        BODY_ATOMS_MAX,
        "the body maximum in src/profile.rs must equal the contract's"
    );
}
