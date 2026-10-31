//! The canonical-token recognizer: admission's second conjunct.
//!
//! The profile walk decides what the AST means; it cannot see what the AST
//! discards. A comment, a redundant parenthesis, a `;` separator, or an escaped
//! spelling of a registered atom is invisible by the time the walk runs, so the
//! recognizer compares the source's bytes with the tokens the walk's result
//! must have been written as.
//!
//! The stage is one `Ok` or one byte offset per admitted module. Every token
//! comes from [`canonical_tokens`], which renders the profile's atom texts, and
//! every offset comes from [`kernel::recognize`], which is the function EP-M4
//! verifies. The recognizer never produces IR, so a source it accepts is still
//! admitted only by the walk's own result.
//!
//! A refusal is an offset, not a reason. [`rejection`] turns the offset into
//! the reason code the manifest names, by looking at the byte recognition
//! stopped on: a `#` whose comment spells `METADATA` is `trivia.metadata`, a
//! byte the profile does not permit is `trivia.byte`, and anything else is a
//! `token.mismatch`.

use crate::{
    admission::{ByteSpan, Rejection},
    corpus::{Profile, ReasonCode, Stage},
};

mod kernel;
mod lexer;
mod render;
#[cfg(test)]
mod tests;
mod witness;

pub use kernel::{METADATA, has_metadata, recognize};
pub use lexer::cross_check;
pub use render::{Tokens, canonical_tokens};
pub use witness::confirm;

/// Why the recognizer did not accept an admitted module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// Recognition stopped at this byte offset.
    Offset(usize),
    /// The profile or the rule list cannot be rendered. Admission fails closed.
    Harness(String),
}

/// Recognize one admitted rule list in the source it came from.
///
/// # Errors
///
/// Returns [`Refusal::Offset`] when the source is not the canonical token
/// sequence, or [`Refusal::Harness`] when the rule list cannot be rendered.
pub fn check(profile: &Profile, source: &str, ir: &[Vec<String>]) -> Result<Vec<usize>, Refusal> {
    let tokens = canonical_tokens(profile, ir).map_err(Refusal::Harness)?;
    let bytes = tokens.bytes();
    recognize(source.as_bytes(), &bytes, &tokens.needs_newline).map_err(Refusal::Offset)
}

/// The rejection a recognizer refusal reports.
#[must_use]
pub fn rejection(source: &str, offset: usize) -> Rejection {
    let code = refusal_code(source.as_bytes(), offset);
    Rejection::new(
        Stage::Recognizer,
        code,
        Some(refusal_span(source, offset)),
        format!("recognition stopped at byte {offset}"),
    )
}

/// The reason code for a refusal at `offset`.
fn refusal_code(source: &[u8], offset: usize) -> ReasonCode {
    match source.get(offset) {
        Some(&b'#') if metadata_comment(source, offset) => ReasonCode::TriviaMetadata,
        Some(&byte) if forbidden_byte(source, offset, byte) => ReasonCode::TriviaByte,
        _ => ReasonCode::TokenMismatch,
    }
}

/// Whether the comment starting at `hash` spells `METADATA`.
fn metadata_comment(source: &[u8], hash: usize) -> bool {
    kernel::has_metadata(kernel::comment_body(source, hash))
}

/// Whether `byte` is a byte the profile does not permit outside a token.
///
/// A carriage return is permitted only as the first half of a line ending.
fn forbidden_byte(source: &[u8], offset: usize, byte: u8) -> bool {
    if byte == b'\r' {
        return source.get(offset.saturating_add(1)) != Some(&b'\n');
    }
    !matches!(byte, b'\t' | b'\n' | 0x20..=0x7e)
}

/// The source range a refusal points at.
///
/// The range covers the whole character that starts at `offset`, so a multi-byte
/// character is never split, and is empty when `offset` is the end of the
/// source.
fn refusal_span(source: &str, offset: usize) -> ByteSpan {
    let end = source.get(offset..).map_or(offset, |tail| {
        tail.chars().next().map_or(offset, |character| {
            offset.saturating_add(character.len_utf8())
        })
    });
    (offset, end)
}
