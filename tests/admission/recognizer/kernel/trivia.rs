//! The kernel's trivia scan: the loop that consumes whitespace and comments.
//!
//! A gap between two tokens may hold spaces, tabs, line endings, and comments,
//! but nothing else, and the scan must stop exactly where the gap ends. The
//! loop invariants here are what carry the specification's [`trivia`] predicate
//! and line-feed record from one consumed item to the next.
//!
//! This module is compiled twice, as test code by Cargo and for the proof by
//! Verus, exactly as its parent is.

use vstd::prelude::*;

// The specification and the lemmas are erased by plain `rustc`, so these
// imports exist only for the verifier and would otherwise be unused.
#[cfg(verus_keep_ghost)]
use super::completeness::*;
#[cfg(verus_keep_ghost)]
use super::lemmas::*;
#[cfg(verus_keep_ghost)]
use super::spec::*;
use super::{Gap, scan::*};

verus! {

broadcast use vstd::seq::group_seq_axioms;

/// Whether `src` holds a carriage return and line feed at `at`.
pub(super) fn crlf_at(src: &[u8], at: usize) -> (r: bool)
    requires
        at < src@.len(),
    ensures
        r == (at + 1 < src@.len() && src@[at as int] == b'\r' && src@[at as int + 1] == b'\n'),
{
    proof {
        // `src@.len()` fits in a `usize`, so the `at + 1` below cannot overflow.
        lemma_slice_len_fits(src);
    }
    if byte_at(src, at) != b'\r' {
        return false;
    }
    if at + 1 < src.len() {
        return byte_at(src, at + 1) == b'\n';
    }
    false
}

/// Skip the comment that starts at `hash`, returning the offset of its line
/// feed, carriage return, or the end of the source.
pub(super) fn skip_comment(src: &[u8], hash: usize) -> (r: Result<usize, usize>)
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
///
/// The scan is one verified loop whose invariant states the gap's own
/// specification clause by clause. Each branch discharges one clause, and
/// moving a branch out would separate it from the exit state that justifies it,
/// so the length is the proof's structure rather than incidental code.
#[expect(
    clippy::too_many_lines,
    reason = "one verified loop per trivia item; its branches carry the invariants"
)]
pub(super) fn skip_trivia(src: &[u8], at: usize, last: bool) -> (r: Result<Gap, usize>)
    requires
        at <= src@.len(),
    ensures
        match r {
            Ok(gap) => {
                &&& at <= gap.end <= src@.len()
                &&& trivia(src@, at as int, gap.end as int, last)
                &&& gap.newline == gap_has_newline(src@, at as int, gap.end as int)
                &&& no_hash(src@) ==> gap.end as int == ws_extent(src@, at as int)
            },
            Err(stop) => stop <= src@.len() && (no_hash(src@) ==> false),
        },
{
    proof {
        lemma_trivia_empty(src@, at as int, last);
        lemma_slice_len_fits(src);
    }
    let mut index = at;
    let mut newline = false;
    // The loop test covers both reasons to stop, so after the loop the scan
    // knows why it stopped: the end of the source, or a byte that starts no
    // trivia item. That is the maximality of the gap.
    while index < src.len() && is_trivia_start(src, index)
        invariant
            at <= index <= src@.len(),
            trivia(src@, at as int, index as int, last),
            newline == gap_has_newline(src@, at as int, index as int),
            no_hash(src@) ==> ws_extent(src@, at as int) == ws_extent(src@, index as int),
        decreases src@.len() - index,
    {
        let byte = byte_at(src, index);
        if byte == b' ' || byte == b'\t' {
            proof {
                lemma_trivia_item_ws(src@, at as int, index as int, last);
                lemma_newline_unchanged(src@, at as int, index as int, index as int + 1);
                assert(ws_at(src@, index as int));
                lemma_ws_extent_step(src@, index as int);
            }
            index += 1;
        } else if byte == b'\n' {
            proof {
                lemma_trivia_item_ws(src@, at as int, index as int, last);
                lemma_newline_gained(src@, at as int, index as int, index as int + 1);
                assert(ws_at(src@, index as int));
                lemma_ws_extent_step(src@, index as int);
            }
            newline = true;
            index += 1;
        } else if crlf_at(src, index) {
            proof {
                lemma_trivia_item_ws(src@, at as int, index as int, last);
                lemma_newline_unchanged(src@, at as int, index as int, index as int + 1);
                assert(ws_at(src@, index as int));
                lemma_ws_extent_step(src@, index as int);
            }
            index += 1;
        } else {
            // The loop test says one of the whitespace cases or `#` holds, so
            // this branch is the comment marker. A source with no `#` never
            // takes it, which is why the extent invariant cannot lose track of
            // the whitespace run here.
            proof {
                assert(byte == b'#');
                if no_hash(src@) {
                    assert(src@[index as int] == b'#');
                    assert(src@[index as int] != b'#');
                    assert(false);
                }
            }
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
        }
    }
    proof {
        if no_hash(src@) {
            // The loop stopped either at the end of the source or at a byte
            // that starts no trivia item, so the byte at `index` is not
            // whitespace.
            assert(!ws_at(src@, index as int));
            lemma_ws_extent_stops(src@, index as int);
            assert(ws_extent(src@, index as int) == index as int);
        }
    }
    Ok(Gap { end: index, newline })
}

} // verus!
