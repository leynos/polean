//! The strict-decoding negatives: every malformed shape is a `tool_failure`.
//!
//! Each case is a mutation of a valid document that a lenient decoder would
//! accept. The point is not that the mutation is unusual, but that the adapter
//! refuses it *before* settlement, so a malformed report can never be read as
//! a rejection or an acceptance. `serde`'s derive with `deny_unknown_fields`
//! does the field work; the extra cases cover the bounds JSON Schema cannot
//! state and the presence checks a derive would otherwise default away.

use rstest::rstest;

use super::{DecodeError, MAX_REPORT_BYTES, decode_report, decode_request};

/// A mutation of a valid report, built fresh for each case.
type ReportMutation = fn() -> String;

/// A mutation of a valid request, built fresh for each case.
type RequestMutation = fn() -> String;

/// The module the valid report answers for.
const MODULE: &str = r#"["AuditFixture","AcceptedGround"]"#;

/// The accepted replay of the valid report.
const REPLAY: &str = r#"{"status":"accepted","modules":[["AuditFixture","Expected"],["AuditFixture","AcceptedGround"]]}"#;

/// The root of the valid report.
const ROOT: &str = r#"{"name":["AuditFixture","AcceptedGround","root"],"declaration":{"kind":"theorem","statement":{"const":["AuditFixture","Goal"],"levels":[]},"axioms":[]}}"#;

/// A constant statement tree.
const LEAF: &str = r#"{"const":["AuditFixture","Goal"],"levels":[]}"#;

/// The request contract's valid module.
const REQUEST_MODULE: &str = r#"["AuditFixture","AcceptedGround"]"#;

/// The request contract's valid root.
const REQUEST_ROOT: &str = r#"{"name":["AuditFixture","AcceptedGround","root"],"expected":{"const":["AuditFixture","Goal"],"levels":[]}}"#;

/// The request contract's valid foundations.
const REQUEST_FOUNDATIONS: &str = r#"[["propext"]]"#;

/// Compose a report from a replay and a roots array.
fn report(replay: &str, roots: &str) -> String {
    format!(
        r#"{{"schema_version":"polean.audit-report.v0","githash":"{}","module":{MODULE},"replay":{replay},"roots":{roots}}}"#,
        super::PIN
    )
}

/// The valid report.
fn valid_report() -> String { report(REPLAY, &format!("[{ROOT}]")) }

/// A report whose single root holds `declaration`.
fn report_with_declaration(declaration: &str) -> String {
    report(
        REPLAY,
        &format!(
            r#"[{{"name":["AuditFixture","AcceptedGround","root"],"declaration":{declaration}}}]"#
        ),
    )
}

/// A report whose single root holds `statement`.
fn report_with_statement(statement: &str) -> String {
    report_with_declaration(&format!(
        r#"{{"kind":"theorem","statement":{statement},"axioms":[]}}"#
    ))
}

/// A report whose single root has no `declaration` key.
fn missing_declaration() -> String {
    report(
        REPLAY,
        r#"[{"name":["AuditFixture","AcceptedGround","root"]}]"#,
    )
}

/// A report whose declaration has no `statement` key.
fn missing_statement() -> String { report_with_declaration(r#"{"kind":"theorem","axioms":[]}"#) }

/// A report whose root name has no components.
fn empty_name() -> String {
    report(
        REPLAY,
        &format!(
            r#"[{{"name":[],"declaration":{{"kind":"theorem","statement":{LEAF},"axioms":[]}}}}]"#
        ),
    )
}

/// A report whose declaration has no `axioms` key.
fn missing_axioms() -> String {
    report_with_declaration(&format!(r#"{{"kind":"theorem","statement":{LEAF}}}"#))
}

/// A report with a field the contract does not name.
fn unknown_field() -> String { valid_report().replace(r#""roots":"#, r#""extra":0,"roots":"#) }

/// A report that repeats a field, which a `Value` decoder would silently keep.
fn duplicate_field() -> String {
    valid_report().replace(r#""githash":"#, r#""githash":"0","githash":"#)
}

/// A report whose `githash` is a number.
///
/// The whole quoted value is replaced, not just the pin text inside it: a
/// substitution that leaves the quotes in place would still be a string, and
/// the case would pass for the wrong reason.
fn wrong_type() -> String {
    valid_report().replace(&format!(r#""githash":"{}""#, super::PIN), r#""githash":5"#)
}

/// A report whose `githash` is the empty string.
///
/// The contract requires at least one character, so the empty value must be
/// malformed rather than reach settlement as a toolchain mismatch.
fn empty_githash() -> String {
    valid_report().replace(&format!(r#""githash":"{}""#, super::PIN), r#""githash":"""#)
}

/// A report followed by a second document.
fn trailing_content() -> String { format!("{}{}", valid_report(), "{}") }

/// A report naming a schema version this decoder does not read.
fn unknown_schema_version() -> String {
    valid_report().replace("polean.audit-report.v0", "polean.audit-report.v1")
}

/// A report whose statement level exceeds `u64`.
fn integer_above_u64() -> String {
    report_with_statement(r#"{"const":["AuditFixture","Goal"],"levels":[18446744073709551616]}"#)
}

/// A report whose statement level is negative.
fn negative_level() -> String {
    report_with_statement(r#"{"const":["AuditFixture","Goal"],"levels":[-1]}"#)
}

/// A report whose statement level is fractional.
fn fractional_level() -> String {
    report_with_statement(r#"{"const":["AuditFixture","Goal"],"levels":[1.5]}"#)
}

/// A report whose declaration kind is outside the closed set.
fn unknown_kind() -> String {
    report_with_declaration(&format!(
        r#"{{"kind":"lemma","statement":{LEAF},"axioms":[]}}"#
    ))
}

/// A report whose statement carries both ground forms.
fn both_tree_forms() -> String {
    report_with_statement(&format!(
        r#"{{"const":["AuditFixture","Goal"],"levels":[],"app":[{LEAF},{LEAF}]}}"#
    ))
}

/// A report whose application has three operands.
fn three_way_app() -> String {
    report_with_statement(&format!(r#"{{"app":[{LEAF},{LEAF},{LEAF}]}}"#))
}

/// A report whose replay status is outside the contract.
fn unknown_replay_status() -> String {
    report(
        r#"{"status":"maybe","modules":[["AuditFixture","AcceptedGround"]]}"#,
        &format!("[{ROOT}]"),
    )
}

/// A report whose rejected replay omits its diagnostic.
fn rejected_without_message() -> String { report(r#"{"status":"rejected"}"#, "[]") }

/// A report whose replay message exceeds the byte bound.
fn oversized_message() -> String {
    let message = "é".repeat(2_049);
    report(
        &format!(r#"{{"status":"rejected","message":"{message}"}}"#),
        "[]",
    )
}

/// Build a perfectly balanced tree of the given depth.
fn balanced(depth: usize) -> String {
    if depth == 0 {
        return LEAF.to_owned();
    }
    let child = balanced(depth - 1);
    format!(r#"{{"app":[{child},{child}]}}"#)
}

/// Build a right-leaning spine of the given number of applications.
fn spine(depth: usize) -> String {
    if depth == 0 {
        return LEAF.to_owned();
    }
    format!(r#"{{"app":[{},{LEAF}]}}"#, spine(depth - 1))
}

/// A report whose statement exceeds the node bound but not the depth bound.
fn over_node_bound() -> String { report_with_statement(&balanced(13)) }

/// A report whose statement exceeds the depth bound but not the node bound.
///
/// The spine has 60 applications and 61 leaves, so its depth is 61 while its
/// node count is 121. It is also the deepest encoding `serde_json` must parse
/// for this test, which checks that the bound is reachable inside the parser's
/// nesting limit.
fn over_depth_bound() -> String { report_with_statement(&spine(60)) }

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

/// A report whose name component is a number that does not fit `u64`.
///
/// `ComponentDocument` is untagged, so the refusal is a generic "no variant
/// matched" error. The case pins that the oversized number is still refused
/// rather than read as text or silently defaulted.
fn numeric_over_u64_component() -> String {
    report_with_statement(r#"{"const":["AuditFixture",18446744073709551616],"levels":[]}"#)
}

/// A report whose name component is a negative number.
fn negative_component() -> String {
    report_with_statement(r#"{"const":["AuditFixture",-1],"levels":[]}"#)
}

/// A report whose name component is a fractional number.
fn fractional_component() -> String {
    report_with_statement(r#"{"const":["AuditFixture",1.5],"levels":[]}"#)
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

/// Every report mutation the decoder must refuse as malformed.
#[rstest]
#[case("unknown field", unknown_field as ReportMutation)]
#[case("duplicate field", duplicate_field as ReportMutation)]
#[case("missing declaration", missing_declaration as ReportMutation)]
#[case("missing statement", missing_statement as ReportMutation)]
#[case("missing axioms", missing_axioms as ReportMutation)]
#[case("empty name", empty_name as ReportMutation)]
#[case("wrong field type", wrong_type as ReportMutation)]
#[case("empty githash", empty_githash as ReportMutation)]
#[case("trailing content", trailing_content as ReportMutation)]
#[case("unknown schema version", unknown_schema_version as ReportMutation)]
#[case("integer above u64", integer_above_u64 as ReportMutation)]
#[case("negative level", negative_level as ReportMutation)]
#[case("fractional level", fractional_level as ReportMutation)]
#[case("numeric component above u64", numeric_over_u64_component as ReportMutation)]
#[case("negative numeric component", negative_component as ReportMutation)]
#[case("fractional numeric component", fractional_component as ReportMutation)]
#[case("unknown declaration kind", unknown_kind as ReportMutation)]
#[case("both tree forms", both_tree_forms as ReportMutation)]
#[case("three-operand application", three_way_app as ReportMutation)]
#[case("unknown replay status", unknown_replay_status as ReportMutation)]
#[case("rejected without message", rejected_without_message as ReportMutation)]
#[case("oversized message", oversized_message as ReportMutation)]
#[case("over node bound", over_node_bound as ReportMutation)]
#[case("over depth bound", over_depth_bound as ReportMutation)]
fn malformed_reports_are_refused(#[case] label: &str, #[case] mutate: ReportMutation) {
    let document = mutate();
    assert!(
        matches!(
            decode_report(document.as_bytes()),
            Err(DecodeError::Malformed(_))
        ),
        "{label} was not refused as malformed\n{document}"
    );
}

/// Every request mutation the decoder must refuse as malformed.
#[rstest]
#[case("empty root component", empty_root_component as RequestMutation)]
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

/// The valid report decodes, so each refusal above is about its mutation.
///
/// A negatives-only suite passes for a decoder that refuses everything, which
/// would leave the corpus's accepted cases to fail for a reason this module
/// hides. The three controls are the floor: the unmutated report, the
/// unmutated request, and a spine at exactly the depth bound, which is the
/// largest document the recursion may accept.
#[test]
fn valid_report_decodes() {
    assert!(
        decode_report(valid_report().as_bytes()).is_ok(),
        "the valid report was refused\n{}",
        valid_report()
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

/// A spine at exactly the depth bound decodes, so the bound admits its edge.
#[test]
fn deepest_admitted_spine_decodes() {
    let document = report_with_statement(&spine(59));
    assert!(
        decode_report(document.as_bytes()).is_ok(),
        "a spine at the depth bound was refused\n{document}"
    );
}

/// Output larger than the report bound is refused before it is parsed.
#[test]
fn oversized_output_is_refused_before_parsing() {
    let document = vec![b' '; MAX_REPORT_BYTES + 1];
    assert_eq!(
        decode_report(&document),
        Err(DecodeError::OutputOverflow),
        "an oversized report was parsed rather than refused"
    );
}
