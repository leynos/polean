//! Report mutations: every malformed report is refused as `Malformed`.
//!
//! Each case is a mutation of [`valid_report`] that a lenient decoder would
//! accept, and each must be refused before settlement can read it as a
//! rejection or an acceptance.

use rstest::rstest;

use super::super::{DecodeError, MAX_REPORT_BYTES, decode_report};

/// A mutation of a valid report, built fresh for each case.
type ReportMutation = fn() -> String;

/// The module the valid report answers for.
const MODULE: &str = r#"["AuditFixture","AcceptedGround"]"#;

/// The accepted replay of the valid report.
const REPLAY: &str = r#"{"status":"accepted","modules":[["AuditFixture","Expected"],["AuditFixture","AcceptedGround"]]}"#;

/// The root of the valid report.
const ROOT: &str = r#"{"name":["AuditFixture","AcceptedGround","root"],"declaration":{"kind":"theorem","statement":{"const":["AuditFixture","Goal"],"levels":[]},"axioms":[]}}"#;

/// A constant statement tree.
const LEAF: &str = r#"{"const":["AuditFixture","Goal"],"levels":[]}"#;

/// Compose a report from a replay and a roots array.
fn report(replay: &str, roots: &str) -> String {
    format!(
        r#"{{"schema_version":"polean.audit-report.v0","githash":"{}","module":{MODULE},"replay":{replay},"roots":{roots}}}"#,
        super::super::PIN
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
    valid_report().replace(
        &format!(r#""githash":"{}""#, super::super::PIN),
        r#""githash":5"#,
    )
}

/// A report whose `githash` is the empty string.
///
/// The contract requires at least one character, so the empty value must be
/// malformed rather than reach settlement as a toolchain mismatch.
fn empty_githash() -> String {
    valid_report().replace(
        &format!(r#""githash":"{}""#, super::super::PIN),
        r#""githash":"""#,
    )
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
