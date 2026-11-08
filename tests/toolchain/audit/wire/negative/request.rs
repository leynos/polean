//! Request mutations: every malformed request is refused as `Malformed`.
//!
//! Each case is a mutation of [`valid_request`] that a lenient decoder would
//! accept. Settlement refuses some of these too, but a document the contract
//! rejects must never reach it.

use rstest::rstest;

use super::super::{DecodeError, decode_request};

/// A mutation of a valid request, built fresh for each case.
type RequestMutation = fn() -> String;

/// The request contract's valid module.
const REQUEST_MODULE: &str = r#"["AuditFixture","AcceptedGround"]"#;

/// The request contract's valid root.
const REQUEST_ROOT: &str = r#"{"name":["AuditFixture","AcceptedGround","root"],"expected":{"const":["AuditFixture","Goal"],"levels":[]}}"#;

/// The request contract's valid foundations.
const REQUEST_FOUNDATIONS: &str = r#"[["propext"]]"#;

/// Compose a request from a module, roots, and foundations.
fn request(module: &str, roots: &str) -> String {
    format!(
        r#"{{"schema_version":"polean.audit-request.v0","module":{module},"roots":{roots},"foundations":{REQUEST_FOUNDATIONS}}}"#
    )
}

/// The valid request.
fn valid_request() -> String { request(REQUEST_MODULE, &format!("[{REQUEST_ROOT}]")) }

/// A request naming a root component that is not an argument token.
fn empty_root_component() -> String { valid_request().replace(r#""root"]"#, r#"""]"#) }

/// A request that names no roots.
///
/// The contract's `roots` carries `minItems: 1`, so an empty list is a document
/// the schema refuses. Settlement would refuse it as `NoRoots`, but the decoder
/// refuses it first so a malformed request never reaches settlement.
fn no_roots() -> String { request(REQUEST_MODULE, "[]") }

/// A request whose module has no components.
fn empty_module() -> String { request("[]", &format!("[{REQUEST_ROOT}]")) }

/// A request whose root name has no components.
fn empty_root_name() -> String {
    request(
        REQUEST_MODULE,
        r#"[{"name":[],"expected":{"const":["AuditFixture","Goal"],"levels":[]}}]"#,
    )
}

/// A request whose expected tree names a constant with a numeric component.
fn numeric_expected_component() -> String {
    request(
        REQUEST_MODULE,
        r#"[{"name":["AuditFixture","AcceptedGround","root"],"expected":{"const":["AuditFixture",7],"levels":[]}}]"#,
    )
}

/// A request whose module holds a numeric component.
fn numeric_module_component() -> String {
    valid_request().replace(REQUEST_MODULE, r#"["AuditFixture",7]"#)
}

/// A request whose module begins with a digit.
fn leading_digit_component() -> String {
    valid_request().replace(REQUEST_MODULE, r#"["AuditFixture","9Goal"]"#)
}

/// A request whose root has no `expected` key.
fn root_without_expected() -> String {
    request(
        REQUEST_MODULE,
        r#"[{"name":["AuditFixture","AcceptedGround","root"]}]"#,
    )
}

/// A request with a top-level entry the contract does not name.
fn extra_request_entry() -> String {
    valid_request().replace(r#""roots":"#, r#""unexpected":[],"roots":"#)
}

/// A request that declares the same foundation twice.
///
/// The contract says `uniqueItems`, so the repeated entry is a document the
/// schema refuses; the decoder must refuse it too rather than hand settlement a
/// list it reads as a set.
fn duplicate_foundation() -> String {
    valid_request().replace(REQUEST_FOUNDATIONS, r#"[["propext"],["propext"]]"#)
}

/// Every request mutation the decoder must refuse as malformed.
#[rstest]
#[case("empty root component", empty_root_component as RequestMutation)]
#[case("no roots", no_roots as RequestMutation)]
#[case("empty module", empty_module as RequestMutation)]
#[case("empty root name", empty_root_name as RequestMutation)]
#[case("numeric expected component", numeric_expected_component as RequestMutation)]
#[case("numeric module component", numeric_module_component as RequestMutation)]
#[case("leading digit component", leading_digit_component as RequestMutation)]
#[case("root without expected", root_without_expected as RequestMutation)]
#[case("extra top-level entry", extra_request_entry as RequestMutation)]
#[case("duplicate foundation", duplicate_foundation as RequestMutation)]
fn malformed_requests_are_refused(#[case] label: &str, #[case] mutate: RequestMutation) {
    let document = mutate();
    assert!(
        matches!(
            decode_request(document.as_bytes()),
            Err(DecodeError::Malformed(_))
        ),
        "{label} was not refused as malformed\n{document}"
    );
}

/// The valid request decodes, so each refusal above is about its mutation.
#[test]
fn valid_request_decodes() {
    assert!(
        decode_request(valid_request().as_bytes()).is_ok(),
        "the valid request was refused\n{}",
        valid_request()
    );
}
