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
//!
//! # Verification
//!
//! The specification lives in `verus/admission_spec.rs` and the proofs that
//! lift each scan step to it in `verus/admission_lemmas.rs`. `verus/
//! admission_recognizer.rs` pulls this module into the Verus entry point with
//! `#[path]`, so this file is the same source Cargo compiles as test code and
//! Verus verifies. The proof establishes kernel soundness only: it says an
//! `Ok` decomposes the source as the tokens say, not that Rego assigns those
//! tokens the same meaning.

#[path = "../../../verus/admission_spec.rs"]
mod spec;

#[path = "../../../verus/admission_lemmas.rs"]
mod lemmas;

use vstd::prelude::*;

// The specification and the lemmas are erased by plain `rustc`, so these
// imports exist only for the verifier and would otherwise be unused.
#[cfg(verus_keep_ghost)]
use self::lemmas::*;
#[cfg(verus_keep_ghost)]
use self::spec::*;

/// The bytes a comment may not spell.
pub const METADATA: &[u8] = b"METADATA";

verus! {

broadcast use vstd::seq::group_seq_axioms;

/// Where a gap of trivia ends and whether it contains a line feed.
pub struct Gap {
    /// The offset at which the gap ends, which is the next token's start.
    pub end: usize,
    /// Whether the gap holds a line feed.
    pub newline: bool,
}

/// The byte at `i`, which must be inside `src`.
pub fn byte_at(src: &[u8], i: usize) -> (b: u8)
    requires
        i < src@.len(),
    ensures
        b == src@[i as int],
{
    *vstd::slice::slice_index_get(src, i)
}

/// The metadata marker's byte at `j`.
const fn metadata_byte(j: usize) -> (b: u8)
    requires
        j < 8,
    ensures
        b == metadata_bytes()[j as int],
{
    // One arm per spelling; the shared letters are grouped.
    match j {
        0 => b'M',
        1 => b'E',
        2 | 6 => b'T',
        4 => b'D',
        _ => b'A',
    }
}

/// Whether the metadata marker sits at `at`.
fn metadata_matches_at(src: &[u8], at: usize) -> (r: bool)
    requires
        at + 8 <= src@.len(),
    ensures
        r == metadata_at(src@, at as int),
{
    proof {
        lemma_slice_len_fits(src);
    }
    let mut j: usize = 0;
    while j < 8
        invariant
            j <= 8,
            at + 8 <= src@.len(),
            forall|k: int| 0 <= k < j as int ==> #[trigger] src@[at as int + k] == metadata_bytes()[k],
        decreases 8 - j,
    {
        proof {
            lemma_slice_len_fits(src);
            assert((at as int + j as int) < src@.len());
        }
        if byte_at(src, at + j) != metadata_byte(j) {
            assert(!metadata_at(src@, at as int)) by {
                assert(src@[at as int + j as int] != metadata_bytes()[j as int]);
            }
            return false;
        }
        j += 1;
    }
    assert(metadata_at(src@, at as int)) by {
        assert(metadata_bytes().len() == 8);
    }
    true
}

/// Whether `from..to` holds the metadata marker.
fn contains_metadata(src: &[u8], from: usize, to: usize) -> (r: bool)
    requires
        from <= to <= src@.len(),
    ensures
        r == metadata_in(src@, from as int, to as int),
{
    proof {
        lemma_slice_len_fits(src);
    }
    let mut at = from;
    // A countdown avoids both `at + 8` and `to - at` in the loop condition:
    // either could overflow or underflow when the offsets are near the end of
    // the integer range.
    let mut remaining = to - from;
    while remaining >= 8
        invariant
            from <= at <= to,
            to <= src@.len(),
            at as int + remaining as int == to as int,
            forall|i: int| from as int <= i < at as int ==> #[trigger] metadata_at(src@, i) == false,
        decreases remaining,
    {
        proof {
            assert((at as int + 8) <= src@.len());
        }
        if metadata_matches_at(src, at) {
            assert(metadata_in(src@, from as int, to as int)) by {
                assert(metadata_at(src@, at as int));
            }
            return true;
        }
        at += 1;
        remaining -= 1;
    }
    assert(!metadata_in(src@, from as int, to as int)) by {
        assert(metadata_bytes().len() == 8);
        assert forall|i: int| from as int <= i && #[trigger] metadata_at(src@, i) implies i + 8
            > to as int by
        {
            if i + 8 <= to as int {
                assert(i < at as int);
                assert(metadata_at(src@, i) == false);
            }
        };
    }
    false
}

/// Whether the byte at `at` starts a line ending.
fn is_line_end(src: &[u8], at: usize) -> (r: bool)
    requires
        at < src@.len(),
    ensures
        r == line_end_at(src@, at as int),
{
    proof {
        lemma_slice_len_fits(src);
    }
    if byte_at(src, at) == b'\n' {
        true
    } else if byte_at(src, at) != b'\r' {
        false
    } else if at + 1 < src.len() {
        byte_at(src, at + 1) == b'\n'
    } else {
        false
    }
}

/// Whether a byte may appear in a comment body.
const fn is_body_byte(byte: u8) -> (r: bool)
    ensures
        r == body_byte(byte),
{
    matches!(byte, b'\t' | 0x20..=0x7e)
}

/// Whether `src` holds `pat` starting at `at`.
fn matches_at(src: &[u8], at: usize, pat: &[u8]) -> (r: bool)
    ensures
        r == token_at(src@, at as int, pat),
{
    let Some(suffix) = src.get(at..) else {
        return false;
    };
    if suffix.len() < pat.len() {
        return false;
    }
    let mut j: usize = 0;
    while j < pat.len()
        invariant
            j <= pat@.len(),
            suffix@.len() == src@.len() - at as int,
            (forall|k: int| 0 <= k < suffix@.len() ==> src@[at as int + k] == suffix@[k]),
            (forall|k: int| 0 <= k < j as int ==> suffix@[k] == pat@[k]),
        decreases pat@.len() - j as int,
    {
        let (Some(actual), Some(expected)) = (suffix.get(j), pat.get(j)) else {
            return false;
        };
        if actual != expected {
            assert(!(forall|k: int| 0 <= k < pat@.len() ==> src@[at as int + k] == pat@[k])) by {
                assert(src@[at as int + j as int] == suffix@[j as int]);
                assert(suffix@[j as int] == *actual);
                assert(pat@[j as int] == *expected);
            }
            return false;
        }
        j += 1;
    }
    true
}

/// Skip the comment that starts at `hash`, returning the offset of its line
/// feed, carriage return, or the end of the source.
fn skip_comment(src: &[u8], hash: usize) -> (r: Result<usize, usize>)
    requires
        hash < src@.len(),
        src@[hash as int] == b'#',
    ensures
        match r {
            Ok(end) => {
                &&& hash < end <= src@.len()
                &&& end == comment_end(src@, hash as int)
                &&& comment_ok(src@, hash as int)
            },
            Err(stop) => stop <= src@.len(),
        },
{
    proof {
        lemma_slice_len_fits(src);
    }
    let start = hash + 1;
    let mut end = start;
    // The line ending is part of the loop condition rather than a `break`, so
    // that the exit state records why the scan stopped.
    while end < src.len() && !is_line_end(src, end)
        invariant
            start <= end <= src@.len(),
            comment_end_from(src@, start as int) == comment_end_from(src@, end as int),
        decreases src@.len() - end,
    {
        if !is_body_byte(byte_at(src, end)) {
            return Err(end);
        }
        proof {
            lemma_comment_end_step(src@, end as int);
        }
        end += 1;
    }
    if contains_metadata(src, start, end) {
        return Err(hash);
    }
    proof {
        lemma_comment_end_stops(src@, end as int);
    }
    Ok(end)
}

/// Skip the trivia that starts at `at`.
///
/// `last` says no token follows, which permits a comment to end at the end of
/// the source. Without it, a comment must be followed by its line feed before
/// the gap ends, because a token on a comment's line belongs to the comment.
fn skip_trivia(src: &[u8], at: usize, last: bool) -> (r: Result<Gap, usize>)
    requires
        at <= src@.len(),
    ensures
        match r {
            Ok(gap) => {
                &&& at <= gap.end <= src@.len()
                &&& trivia(src@, at as int, gap.end as int, last)
                &&& gap.newline == gap_has_newline(src@, at as int, gap.end as int)
            },
            Err(stop) => stop <= src@.len(),
        },
{
    proof {
        lemma_trivia_empty(src@, at as int, last);
        lemma_slice_len_fits(src);
    }
    let mut index = at;
    let mut newline = false;
    while index < src.len()
        invariant
            at <= index <= src@.len(),
            trivia(src@, at as int, index as int, last),
            newline == gap_has_newline(src@, at as int, index as int),
        decreases src@.len() - index,
    {
        let byte = byte_at(src, index);
        if byte == b' ' || byte == b'\t' {
            proof {
                lemma_trivia_item_ws(src@, at as int, index as int, last);
                lemma_newline_unchanged(src@, at as int, index as int, index as int + 1);
            }
            index += 1;
        } else if byte == b'\n' {
            proof {
                lemma_trivia_item_ws(src@, at as int, index as int, last);
                lemma_newline_gained(src@, at as int, index as int, index as int + 1);
            }
            newline = true;
            index += 1;
        } else if byte == b'\r' && index + 1 < src.len() && byte_at(src, index + 1) == b'\n' {
            proof {
                lemma_trivia_item_ws(src@, at as int, index as int, last);
                lemma_newline_unchanged(src@, at as int, index as int, index as int + 1);
            }
            index += 1;
        } else if byte == b'#' {
            let end = skip_comment(src, index)?;
            if end == src.len() && !last {
                return Err(end);
            }
            if end == src.len() {
                proof {
                    lemma_trivia_after_comment_eof(src@, at as int, index as int);
                }
                index = end;
            } else if byte_at(src, end) == b'\n' {
                proof {
                    lemma_trivia_after_comment_lf(src@, at as int, index as int, last);
                }
                newline = true;
                index = end + 1;
            } else {
                proof {
                    lemma_trivia_after_comment_crlf(src@, at as int, index as int, last);
                }
                newline = true;
                index = end + 2;
            }
        } else {
            break;
        }
    }
    Ok(Gap { end: index, newline })
}

/// Recognize `toks` in `src`, returning each token's start offset.
///
/// # Errors
///
/// Returns the offset at which recognition stopped.
pub fn recognize(src: &[u8], toks: &[&[u8]], needs_newline: &[bool]) -> (r: Result<Vec<usize>, usize>)
    requires
        forall|j: int| 0 <= j < toks@.len() ==> #[trigger] toks@[j]@.len() > 0,
        needs_newline@.len() == toks@.len(),
    ensures
        match r {
            Ok(offsets) => {
                &&& offsets@.len() == toks@.len()
                &&& decomposes_at(src@, toks@, offsets@)
                &&& flags_hold(src@, toks@, offsets@, needs_newline@)
            },
            Err(stop) => stop <= src@.len(),
        },
{
    proof {
        lemma_slice_len_fits(src);
        reveal(flags_prefix);
    }
    let mut offsets: Vec<usize> = Vec::new();
    let mut at = 0;
    let mut index = 0;
    while index < toks.len()
        invariant
            index <= toks@.len(),
            offsets@.len() == index,
            at as int == prev_end(src@, toks@, offsets@, index as int),
            at <= src@.len(),
            src@.len() <= usize::MAX as int,
            needs_newline@.len() == toks@.len(),
            prefix_decomposes(src@, toks@, offsets@, index as int),
            flags_prefix(src@, toks@, offsets@, needs_newline@, index as int),
        decreases toks@.len() - index,
    {
        let token = *vstd::slice::slice_index_get(toks, index);
        if token.is_empty() {
            return Err(at);
        }
        let gap = skip_trivia(src, at, false)?;
        if index > 0 && gap.end == at {
            return Err(at);
        }
        proof {
            assert((index as int) < toks@.len());
        }
        let flag = *vstd::slice::slice_index_get(needs_newline, index);
        if flag && !gap.newline {
            return Err(gap.end);
        }
        if flag {
            proof {
                assert(gap_has_newline(src@, at as int, gap.end as int));
            }
        }
        if !matches_at(src, gap.end, token) {
            return Err(gap.end);
        }
        proof {
            assert((gap.end as int + token@.len()) <= src@.len());
        }
        let ghost prior: Seq<usize> = offsets@;
        offsets.push(gap.end);
        proof {
            lemma_flags_prefix_push(src@, toks@, prior, needs_newline@, index as int, gap.end);
        }
        at = gap.end + token.len();
        index += 1;
    }
    let trailing = skip_trivia(src, at, true)?;
    if trailing.end == src.len() {
        Ok(offsets)
    } else {
        Err(trailing.end)
    }
}

} // verus!

/// Whether `body` spells `METADATA` anywhere.
///
/// Both the kernel and the witness use this one definition, so a comment the
/// witness refuses is a comment the kernel refuses.
pub fn has_metadata(body: &[u8]) -> bool {
    body.windows(METADATA.len())
        .any(|window| window == METADATA)
}

/// The body of the comment that starts at `hash`, or the empty slice.
///
/// The classifier in [`super`] uses the same rule the kernel applies, so a
/// refusal at a `#` offset and the reason code it reports cannot disagree.
///
/// This helper is outside the `verus!` block: it classifies a refusal that has
/// already happened, so it carries no admission contract.
pub fn comment_body(src: &[u8], hash: usize) -> &[u8] {
    let start = hash.saturating_add(1);
    let mut end = start;
    while let Some(&byte) = src.get(end) {
        if is_line_end_unchecked(src, end) || !is_body_byte_unchecked(byte) {
            break;
        }
        end += 1;
    }
    src.get(start..end).unwrap_or_default()
}

/// Whether the byte at `at` starts a line ending, for the classifier.
fn is_line_end_unchecked(src: &[u8], at: usize) -> bool {
    match src.get(at) {
        Some(&b'\n') => true,
        Some(&b'\r') => src.get(at.saturating_add(1)) == Some(&b'\n'),
        Some(_) | None => false,
    }
}

/// Whether a byte may appear in a comment body, for the classifier.
const fn is_body_byte_unchecked(byte: u8) -> bool { matches!(byte, b'\t' | 0x20..=0x7e) }
