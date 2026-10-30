//! The recognizer kernel: a bounded check that a source is a canonical token
//! sequence separated by permitted trivia.
//!
//! The kernel sees bytes and tokens only. It has no access to the rule list,
//! the profile, Regorus, or the decoded AST, which is what lets EP-M4 verify it
//! once for inputs of any length (O6). Everything that depends on the module's
//! shape is rendered into the token list beforehand.
//!
//! # Contract
//!
//! ```text
//! recognize(src, toks, needs_newline)
//!   requires every token is non-empty and `needs_newline` gives one flag per
//!            token
//!   ensures  Ok(offsets) gives one start offset per token, each token's exact
//!            bytes sit at its offset, the gaps between consecutive tokens are
//!            non-empty permitted trivia, the gap before a token whose flag is
//!            set contains a line feed, and the bytes after the last token are
//!            permitted trivia;
//!            Err(offset) names where recognition stopped, and that offset is
//!            never past the end of the source.
//! ```
//!
//! Permitted trivia is a space, a tab, a line feed, a carriage return that
//! immediately precedes a line feed, or a comment. A comment starts with `#`,
//! runs to the end of its line, and its body may hold only tabs and bytes
//! `0x20`–`0x7E`. A comment body containing `METADATA` is refused outright
//! (D9), and a token may not begin on a comment's line (D10).

use super::has_metadata;

/// Where a gap of trivia ends and whether it contains a line feed.
struct Gap {
    /// The offset at which the gap ends, which is the next token's start.
    end: usize,
    /// Whether the gap holds a line feed.
    newline: bool,
}

/// Recognize `toks` in `src`, returning each token's start offset.
///
/// # Errors
///
/// Returns the offset at which recognition stopped.
pub fn recognize(src: &[u8], toks: &[&[u8]], needs_newline: &[bool]) -> Result<Vec<usize>, usize> {
    let mut offsets = Vec::with_capacity(toks.len());
    let mut at = 0;
    for (index, token) in toks.iter().enumerate() {
        if token.is_empty() {
            return Err(at);
        }
        let gap = skip_trivia(src, at, false)?;
        if index > 0 && gap.end == at {
            return Err(at);
        }
        if needs_newline.get(index).copied().unwrap_or(false) && !gap.newline {
            return Err(gap.end);
        }
        if !starts_with(src, gap.end, token) {
            return Err(gap.end);
        }
        offsets.push(gap.end);
        at = gap.end.saturating_add(token.len());
    }
    let trailing = skip_trivia(src, at, true)?;
    if trailing.end == src.len() {
        Ok(offsets)
    } else {
        Err(trailing.end)
    }
}

/// Whether `src` holds `token` starting at `offset`.
fn starts_with(src: &[u8], offset: usize, token: &[u8]) -> bool {
    src.get(offset..offset.saturating_add(token.len())) == Some(token)
}

/// Skip the trivia that starts at `at`.
///
/// `last` says no token follows, which permits a comment to end at the end of
/// the source. Without it, a comment must be followed by its line feed before
/// the gap ends, because a token on a comment's line belongs to the comment.
fn skip_trivia(src: &[u8], at: usize, last: bool) -> Result<Gap, usize> {
    let mut index = at;
    let mut newline = false;
    while let Some(&byte) = src.get(index) {
        match byte {
            b' ' | b'\t' => index += 1,
            b'\n' => {
                newline = true;
                index += 1;
            }
            b'\r' if src.get(index.saturating_add(1)) == Some(&b'\n') => {
                newline = true;
                index += 1;
            }
            b'#' => {
                let end = skip_comment(src, index)?;
                if end == src.len() && !last {
                    return Err(end);
                }
                index = end;
            }
            _ => break,
        }
    }
    Ok(Gap {
        end: index,
        newline,
    })
}

/// Skip the comment that starts at `hash`, returning the offset of its line
/// feed, carriage return, or the end of the source.
fn skip_comment(src: &[u8], hash: usize) -> Result<usize, usize> {
    let start = hash.saturating_add(1);
    let mut end = start;
    while let Some(&byte) = src.get(end) {
        if is_line_end(src, end) {
            break;
        }
        if !is_body_byte(byte) {
            return Err(end);
        }
        end += 1;
    }
    let body = src.get(start..end).unwrap_or_default();
    if has_metadata(body) {
        return Err(hash);
    }
    Ok(end)
}

/// Whether the byte at `at` ends a line: a line feed, or a carriage return
/// that precedes one.
fn is_line_end(src: &[u8], at: usize) -> bool {
    match src.get(at) {
        Some(&b'\n') => true,
        Some(&b'\r') => src.get(at.saturating_add(1)) == Some(&b'\n'),
        Some(_) | None => false,
    }
}

/// Whether a byte may appear in a comment body.
const fn is_body_byte(byte: u8) -> bool { matches!(byte, b'\t' | 0x20..=0x7e) }

/// The body of the comment that starts at `hash`, or the empty slice.
///
/// The classifier in [`super`] uses the same rule the kernel applies, so a
/// refusal at a `#` offset and the reason code it reports cannot disagree.
pub(super) fn comment_body(src: &[u8], hash: usize) -> &[u8] {
    let start = hash.saturating_add(1);
    let mut end = start;
    while let Some(&byte) = src.get(end) {
        if is_line_end(src, end) || !is_body_byte(byte) {
            break;
        }
        end += 1;
    }
    src.get(start..end).unwrap_or_default()
}
