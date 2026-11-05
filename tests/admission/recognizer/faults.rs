//! The seeded kernel faults the witness must catch.
//!
//! Each fault is a real function with the kernel's signature that omits one
//! rule, or takes the same shortcut a kernel without that rule would take. The
//! harness runs it on a source it accepts and asks the witness about the real
//! source, so a witness that merely repeats the kernel's loop cannot catch
//! them.

use super::{
    Tokens,
    confirm,
    tests::{one_literal, same_tenant_ir, tokens_for},
};
use crate::support::profile;

/// The kernel signature the harness injects.
type Kernel = fn(&[u8], &[&[u8]], &[bool]) -> Result<Vec<usize>, usize>;

/// Run `kernel` on `source` and require the witness to refuse its offsets.
///
/// A faulty kernel must still recognize the source; if it does not, it was
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
    while rewritten.get(at).is_some() {
        rewrite_bare_carriage_return(&mut rewritten, at);
        at = at.saturating_add(1);
    }
    rewritten
}

/// Replace a bare carriage return at `at` with a line feed, keeping offsets.
fn rewrite_bare_carriage_return(text: &mut [u8], at: usize) {
    if !is_bare_carriage_return(text, at) {
        return;
    }
    if let Some(slot) = text.get_mut(at) {
        *slot = b'\n';
    }
}

/// Whether `text` holds a carriage return at `at` that is not part of CRLF.
fn is_bare_carriage_return(text: &[u8], at: usize) -> bool {
    text.get(at) == Some(&b'\r') && text.get(at.saturating_add(1)) != Some(&b'\n')
}

/// The first offset at or after `from` where `token` occurs.
fn find_from(src: &[u8], from: usize, token: &[u8]) -> Option<usize> {
    src.get(from..)?
        .windows(token.len())
        .position(|window| window == token)
        .map(|index| from.saturating_add(index))
}
