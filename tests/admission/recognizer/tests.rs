//! Tests for the recognizer: the baseline, the refusal mapping, and the seeded
//! kernel faults the witness must catch.
//!
//! The seeded faults are kernel functions, not feature flags. Each one is a
//! real function with the kernel's signature that omits one rule, or takes the
//! same shortcut a kernel without that rule would take. The harness runs it on
//! a source it accepts and asks the witness about the real source, so a witness
//! that merely repeats the kernel's loop cannot catch them.

use super::{Refusal, Tokens, canonical_tokens, confirm, cross_check, rejection};
use crate::corpus::{Profile, ReasonCode, Stage};

/// The kernel signature the harness injects.
type Kernel = fn(&[u8], &[&[u8]], &[bool]) -> Result<Vec<usize>, usize>;

/// The profile contract, or a panic naming the read failure.
fn profile() -> Profile {
    match crate::corpus::profile_contract() {
        Ok(profile) => profile,
        Err(error) => panic!("the profile must parse: {error}"),
    }
}

/// The canonical tokens of `ir` under `profile`.
fn tokens_for<'a>(profile: &'a Profile, ir: &[Vec<String>]) -> Tokens<'a> {
    match canonical_tokens(profile, ir) {
        Ok(tokens) => tokens,
        Err(error) => panic!("the rule list must render: {error}"),
    }
}

/// The canonical single-literal module, with `header` as its first line.
fn one_literal(header: &str) -> String {
    format!(
        "{header}\npackage authz\n\nimport rego.v1\n\ndefault allow := false\n\nallow if {{\n    \
         input.subject.tenant == input.resource.tenant\n}}\n"
    )
}

/// The rule list of one single-atom rule.
fn same_tenant_ir() -> Vec<Vec<String>> { vec![vec!["same_tenant".to_owned()]] }

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

/// Run `kernel` on `source` and require the witness to refuse its offsets.
///
/// A faulty kernel must still recognise the source; if it does not, it was
/// never given the chance to produce a wrong witness and the test proves
/// nothing.
fn witness_refusal(kernel: Kernel, source: &str, tokens: &Tokens<'_>) -> String {
    let bytes = tokens.bytes();
    let offsets = match kernel(source.as_bytes(), &bytes, &tokens.needs_newline) {
        Ok(offsets) => offsets,
        Err(at) => panic!("the faulty kernel must recognise the source, stopped at {at}"),
    };
    match confirm(source.as_bytes(), &bytes, &tokens.needs_newline, &offsets) {
        Ok(()) => panic!("the witness accepted offsets from a faulty kernel"),
        Err(detail) => detail,
    }
}

/// A kernel that does not scan comment bodies for `METADATA`.
fn skips_the_metadata_rule(
    src: &[u8],
    toks: &[&[u8]],
    flags: &[bool],
) -> Result<Vec<usize>, usize> {
    super::recognize(&without_metadata(src), toks, flags)
}

/// A kernel that treats a comment as ending at the next whitespace, so a token
/// may follow a comment on the same line.
fn lets_a_token_follow_a_comment(
    src: &[u8],
    toks: &[&[u8]],
    flags: &[bool],
) -> Result<Vec<usize>, usize> {
    super::recognize(&short_comments(src), toks, flags)
}

/// A kernel without the non-empty-gap rule: it searches for each token at or
/// after the cursor, so two adjacent tokens are accepted.
fn accepts_empty_trivia(src: &[u8], toks: &[&[u8]], flags: &[bool]) -> Result<Vec<usize>, usize> {
    let mut at = 0;
    let mut offsets = Vec::with_capacity(toks.len());
    for (index, token) in toks.iter().enumerate() {
        let Some(offset) = find_from(src, at, token) else {
            return Err(at);
        };
        if flags.get(index).copied().unwrap_or(false)
            && !src.get(at..offset).unwrap_or_default().contains(&b'\n')
        {
            return Err(offset);
        }
        offsets.push(offset);
        at = offset.saturating_add(token.len());
    }
    Ok(offsets)
}

/// A kernel that treats a bare carriage return as a line feed.
fn accepts_a_bare_carriage_return(
    src: &[u8],
    toks: &[&[u8]],
    flags: &[bool],
) -> Result<Vec<usize>, usize> {
    super::recognize(&without_bare_carriage_returns(src), toks, flags)
}

#[test]
fn a_kernel_that_skips_the_metadata_rule_is_caught() {
    let profile = profile();
    let tokens = tokens_for(&profile, &same_tenant_ir());
    let source = one_literal("# METADATA note");
    let detail = witness_refusal(skips_the_metadata_rule, &source, &tokens);
    assert!(
        detail.contains("METADATA"),
        "the witness must name the metadata rule, got: {detail}"
    );
}

#[test]
fn a_kernel_that_lets_a_token_follow_a_comment_is_caught() {
    let profile = profile();
    let tokens = tokens_for(&profile, &same_tenant_ir());
    let source = "package #c authz\nimport rego.v1\n\ndefault allow := false\n\nallow if {\n    \
                  input.subject.tenant == input.resource.tenant\n}\n";
    let detail = witness_refusal(lets_a_token_follow_a_comment, source, &tokens);
    assert!(
        detail.contains("not followed by a line feed"),
        "the witness must name the comment rule, got: {detail}"
    );
}

#[test]
fn a_kernel_that_accepts_empty_trivia_is_caught() {
    let tokens = Tokens {
        text: vec!["package", "authz"],
        needs_newline: vec![false, false],
    };
    let detail = witness_refusal(accepts_empty_trivia, "packageauthz", &tokens);
    assert!(
        detail.contains("empty"),
        "the witness must name the empty gap, got: {detail}"
    );
}

#[test]
fn a_kernel_that_accepts_a_bare_carriage_return_is_caught() {
    let profile = profile();
    let tokens = tokens_for(&profile, &same_tenant_ir());
    let source = one_literal("# admission test").replacen("package ", "package\r ", 1);
    let detail = witness_refusal(accepts_a_bare_carriage_return, &source, &tokens);
    assert!(
        detail.contains("not trivia"),
        "the witness must name the bare carriage return, got: {detail}"
    );
}

/// Rewrite every `METADATA` so the metadata rule cannot fire, keeping every
/// offset.
fn without_metadata(src: &[u8]) -> Vec<u8> {
    let mut masked = src.to_vec();
    let starts: Vec<usize> = masked
        .windows(super::METADATA.len())
        .enumerate()
        .filter(|(_, window)| *window == super::METADATA)
        .map(|(start, _)| start)
        .collect();
    for start in starts {
        if let Some(slot) = start
            .checked_add(super::METADATA.len())
            .and_then(|end| end.checked_sub(1))
            .and_then(|last| masked.get_mut(last))
        {
            *slot = b'X';
        }
    }
    masked
}

/// Blank every comment up to its next whitespace, keeping every offset.
fn short_comments(src: &[u8]) -> Vec<u8> {
    let mut blanked = src.to_vec();
    let mut at = 0;
    while let Some(&byte) = blanked.get(at) {
        at = if byte == b'#' {
            blank_to_whitespace(&mut blanked, at)
        } else {
            at.saturating_add(1)
        };
    }
    blanked
}

/// Blank from `hash` to the next whitespace, returning that offset.
fn blank_to_whitespace(text: &mut [u8], hash: usize) -> usize {
    let mut at = hash;
    while let Some(&byte) = text.get(at) {
        if matches!(byte, b' ' | b'\t' | b'\n' | b'\r') {
            return at;
        }
        if let Some(slot) = text.get_mut(at) {
            *slot = b' ';
        }
        at = at.saturating_add(1);
    }
    at
}

/// Rewrite every bare carriage return as a line feed, keeping every offset.
fn without_bare_carriage_returns(src: &[u8]) -> Vec<u8> {
    let mut rewritten = src.to_vec();
    let mut at = 0;
    while let Some(&byte) = rewritten.get(at) {
        if byte == b'\r'
            && rewritten.get(at.saturating_add(1)) != Some(&b'\n')
            && let Some(slot) = rewritten.get_mut(at)
        {
            *slot = b'\n';
        }
        at = at.saturating_add(1);
    }
    rewritten
}

/// The first offset at or after `from` where `token` occurs.
fn find_from(src: &[u8], from: usize, token: &[u8]) -> Option<usize> {
    src.get(from..)?
        .windows(token.len())
        .position(|window| window == token)
        .map(|index| from.saturating_add(index))
}
