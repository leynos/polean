//! Tests for the recognizer: the baseline, the refusal mapping, and adversarial
//! trivia the normal kernel must refuse.
//!
//! The seeded kernel faults the witness must catch live in [`super::faults`].

use super::{Refusal, Tokens, canonical_tokens, confirm, cross_check, rejection};
use crate::{
    corpus::{Profile, ReasonCode, Stage},
    support::profile,
};

/// The canonical tokens of `ir` under `profile`.
pub(super) fn tokens_for<'a>(profile: &'a Profile, ir: &[Vec<String>]) -> Tokens<'a> {
    match canonical_tokens(profile, ir) {
        Ok(tokens) => tokens,
        Err(error) => panic!("the rule list must render: {error}"),
    }
}

/// The canonical single-literal module, with `header` as its first line.
pub(super) fn one_literal(header: &str) -> String {
    format!(
        "{header}\npackage authz\n\nimport rego.v1\n\ndefault allow := false\n\nallow if {{\n    \
         input.subject.tenant == input.resource.tenant\n}}\n"
    )
}

/// The rule list of one single-atom rule.
pub(super) fn same_tenant_ir() -> Vec<Vec<String>> { vec![vec!["same_tenant".to_owned()]] }

#[test]
fn a_canonical_source_is_recognised_and_witnessed() {
    let profile = profile();
    let tokens = tokens_for(&profile, &same_tenant_ir());
    let source = one_literal("# admission test");
    let offsets = match super::check(&profile, &source, &same_tenant_ir()) {
        Ok(offsets) => offsets,
        Err(refusal) => panic!("the canonical source must be recognised: {refusal:?}"),
    };
    assert_eq!(
        offsets.len(),
        tokens.text.len(),
        "there must be one offset per canonical token"
    );
    assert_eq!(
        offsets.first().copied(),
        source.find("package"),
        "the first token must be the package keyword"
    );
    assert_eq!(
        confirm(
            source.as_bytes(),
            &tokens.bytes(),
            &tokens.needs_newline,
            &offsets
        ),
        Ok(()),
        "the witness must confirm the recognizer's offsets"
    );
    assert_eq!(
        cross_check(&source, &tokens.text),
        Ok(()),
        "the pinned lexer must agree with the canonical tokens"
    );
}

/// O6's concrete accepting instance, run through the real kernel.
///
/// The bytes, tokens, offsets, and flags asserted here are the values
/// `lemma_baseline_accepts` proves for the specification's baseline, so the
/// proof and this run exercise one baseline rather than two similar ones.
#[test]
fn the_concrete_baseline_is_recognised_at_the_proved_offsets() {
    let profile = profile();
    let ir = vec![vec!["same_tenant".to_owned(), "write_action".to_owned()]];
    let source = crate::corpus::BASELINE;
    assert_eq!(source.len(), 146, "the baseline is the proved source");
    let tokens = tokens_for(&profile, &ir);
    assert_eq!(
        tokens.text,
        vec![
            "package",
            "authz",
            "import",
            "rego.v1",
            "default",
            "allow",
            ":=",
            "false",
            "allow",
            "if",
            "{",
            "input.subject.tenant",
            "==",
            "input.resource.tenant",
            "input.action",
            "==",
            "\"write\"",
            "}",
        ],
        "the baseline tokens must be the proved tokens"
    );
    let flags = [
        false, false, false, false, false, false, false, false, false, false, false, false, false,
        false, true, false, false, false,
    ];
    assert_eq!(
        tokens.needs_newline, flags,
        "only the second literal begins a line"
    );
    let offsets = match super::check(&profile, source, &ir) {
        Ok(offsets) => offsets,
        Err(refusal) => panic!("the baseline must be recognised: {refusal:?}"),
    };
    assert_eq!(
        offsets,
        vec![
            0, 8, 15, 22, 31, 39, 45, 48, 55, 61, 64, 70, 91, 94, 120, 133, 136, 144
        ],
        "recognition must return the proved offsets"
    );
    // The verified instance runs the same kernel on the same bytes, and its
    // postcondition fixes these offsets.
    let instance =
        super::baseline_is_recognised(source.as_bytes(), &tokens.bytes(), &tokens.needs_newline);
    assert_eq!(
        instance,
        Ok(offsets),
        "the verified instance must return the proved offsets"
    );
}

#[test]
fn a_second_body_literal_must_start_after_a_line_feed() {
    let profile = profile();
    let ir = vec![vec!["same_tenant".to_owned(), "write_action".to_owned()]];
    let separated = "package authz\n\nimport rego.v1\n\ndefault allow := false\n\nallow if {\n    \
                     input.subject.tenant == input.resource.tenant\n    input.action == \
                     \"write\"\n}\n";
    assert!(
        super::check(&profile, separated, &ir).is_ok(),
        "two literals on separate lines must be recognised"
    );
    let joined = "package authz\n\nimport rego.v1\n\ndefault allow := false\n\nallow if { \
                  input.subject.tenant == input.resource.tenant input.action == \"write\" }\n";
    let stopped = match super::check(&profile, joined, &ir) {
        Ok(offsets) => panic!("both literals on one line must be refused: {offsets:?}"),
        Err(Refusal::Offset(at)) => at,
        Err(Refusal::Harness(detail)) => panic!("the rule list must render: {detail}"),
    };
    assert_eq!(
        Some(stopped),
        joined.find("input.action"),
        "recognition must stop at the second literal"
    );
}

#[test]
fn an_unregistered_atom_fails_closed() {
    let profile = profile();
    let ir = vec![vec!["no_such_atom".to_owned()]];
    assert!(
        matches!(
            super::check(&profile, one_literal("# admission test").as_str(), &ir),
            Err(Refusal::Harness(_))
        ),
        "an atom the profile does not register must be a harness fault"
    );
}

#[test]
fn a_metadata_comment_is_reported_at_its_hash() {
    let source = "# METADATA\npackage authz\n";
    let reported = rejection(source, 0);
    assert_eq!(
        reported.stage,
        Stage::Recognizer,
        "a metadata comment is a recognizer refusal"
    );
    assert_eq!(
        reported.code,
        ReasonCode::TriviaMetadata,
        "a comment spelling METADATA is trivia.metadata"
    );
    assert_eq!(
        reported.span,
        Some((0, 1)),
        "the span must cover the hash that starts the comment"
    );
}

#[test]
fn a_forbidden_byte_is_reported_as_trivia_byte() {
    let source = "# caf\u{e9}\n";
    let Some(at) = source.find('\u{e9}') else {
        panic!("the source must contain the non-ASCII character")
    };
    let reported = rejection(source, at);
    assert_eq!(
        reported.code,
        ReasonCode::TriviaByte,
        "a byte outside a comment body is trivia.byte"
    );
    assert_eq!(
        reported.span,
        Some((at, at + 2)),
        "the span must cover the whole character, not one byte of it"
    );
}

/// O7's adversarial trivia: a token that follows `#` on the same line.
///
/// Regorus reads the rest of the line as comment text, so a canonical token
/// moved there is not a token to the frontend. The recognizer must not treat
/// it as one either, and the source must be refused rather than accepted.
#[test]
fn a_token_after_a_hash_is_refused() {
    let profile = profile();
    let source = one_literal("# admission test").replacen("package authz", "package # authz", 1);
    let refused_at = match super::check(&profile, &source, &same_tenant_ir()) {
        Ok(offsets) => panic!("a token on a comment line must be refused: {offsets:?}"),
        Err(Refusal::Offset(at)) => at,
        Err(Refusal::Harness(detail)) => panic!("the rule list must render: {detail}"),
    };
    assert_eq!(
        Some(refused_at),
        source.find("import"),
        "recognition must stop at the token that follows the commented-out package name"
    );
}

/// O7's adversarial trivia: a NUL inside a comment body.
///
/// The frontend reads the NUL as the end of its input; the recognizer reads the
/// submitted byte and must refuse it rather than accept the truncated module.
#[test]
fn a_nul_inside_a_comment_is_refused_at_the_nul() {
    let profile = profile();
    let source =
        one_literal("# admission test").replacen("# admission test", "# admission\u{0}test", 1);
    let Some(nul) = source.find('\u{0}') else {
        panic!("the source must contain the NUL")
    };
    match super::check(&profile, &source, &same_tenant_ir()) {
        Ok(offsets) => panic!("a NUL in a comment must be refused: {offsets:?}"),
        Err(Refusal::Offset(at)) => assert_eq!(at, nul, "recognition must stop at the NUL"),
        Err(Refusal::Harness(detail)) => panic!("the rule list must render: {detail}"),
    }
    assert_eq!(
        rejection(&source, nul).code,
        ReasonCode::TriviaByte,
        "a NUL outside a comment body is trivia.byte"
    );
}

/// O7's adversarial trivia: a `//` comment.
///
/// Rego has no `//` comment, so the bytes are not permitted trivia and the
/// recognizer must stop where they start rather than skip over them.
#[test]
fn a_slash_comment_is_refused() {
    let profile = profile();
    let source = one_literal("# admission test").replacen(
        "\nimport rego.v1",
        "\n// admission test\nimport rego.v1",
        1,
    );
    let Some(slashes) = source.find("//") else {
        panic!("the source must contain the slash comment")
    };
    match super::check(&profile, &source, &same_tenant_ir()) {
        Ok(offsets) => panic!("a slash comment must be refused: {offsets:?}"),
        Err(Refusal::Offset(at)) => assert_eq!(at, slashes, "recognition must stop at the slashes"),
        Err(Refusal::Harness(detail)) => panic!("the rule list must render: {detail}"),
    }
}

#[test]
fn a_mismatch_is_reported_as_token_mismatch() {
    let source = "(input.subject.tenant == input.resource.tenant)\n";
    let reported = rejection(source, 0);
    assert_eq!(
        reported.code,
        ReasonCode::TokenMismatch,
        "a parenthesis where a token must start is token.mismatch"
    );
    assert_eq!(
        reported.span,
        Some((0, 1)),
        "the span must cover the offending character"
    );
}

#[test]
fn the_end_of_the_source_is_an_empty_token_mismatch_span() {
    let source = "package";
    let reported = rejection(source, source.len());
    assert_eq!(
        reported.code,
        ReasonCode::TokenMismatch,
        "running out of source is token.mismatch"
    );
    assert_eq!(
        reported.span,
        Some((source.len(), source.len())),
        "the span must be empty at the end of the source"
    );
}
