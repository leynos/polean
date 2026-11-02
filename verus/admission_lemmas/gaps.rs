//! Proofs that adding permitted trivia extends a gap.

use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use super::super::spec::*;
#[cfg(verus_keep_ghost)]
use super::comments::{lemma_comment_body_byte, lemma_comment_end_ge};
#[cfg(verus_keep_ghost)]
use super::records::{lemma_newline_gained, lemma_newline_unchanged};

verus! {

// ---------------------------------------------------------------------------
// Extending a trivia gap
// ---------------------------------------------------------------------------

/// Adding one whitespace byte to the end of a trivia gap keeps it trivia.
pub proof fn lemma_trivia_core_ws(s: Seq<u8>, a: int, b: int)
    requires
        trivia_core(s, a, b),
        ws_at(s, b),
    ensures
        trivia_core(s, a, b + 1),
    decreases b - a,
{
    reveal_with_fuel(trivia_core, 3);
    if a >= b {
    } else if ws_at(s, a) {
        lemma_trivia_core_ws(s, a + 1, b);
    } else if s[a] == b'#' && comment_ok(s, a) && a < comment_end(s, a) && comment_end(s, a) <= b {
        lemma_trivia_core_ws(s, comment_end(s, a), b);
    }
}

/// Adding one well-formed comment to the end of a trivia gap keeps it trivia.
pub proof fn lemma_trivia_core_comment(s: Seq<u8>, a: int, b: int)
    requires
        trivia_core(s, a, b),
        0 <= b < s.len() as int,
        s[b] == b'#',
        comment_ok(s, b),
    ensures
        trivia_core(s, a, comment_end(s, b)),
    decreases b - a,
{
    reveal_with_fuel(trivia_core, 3);
    lemma_comment_end_ge(s, b + 1);
    if a >= b {
    } else if ws_at(s, a) {
        lemma_trivia_core_comment(s, a + 1, b);
    } else if s[a] == b'#' && comment_ok(s, a) && a < comment_end(s, a) && comment_end(s, a) <= b {
        lemma_trivia_core_comment(s, comment_end(s, a), b);
    }
}

/// Adding one whitespace byte does not leave a gap ending in a comment.
pub proof fn lemma_ends_ws(s: Seq<u8>, a: int, b: int)
    requires
        0 <= a <= b,
        trivia_core(s, a, b),
        ws_at(s, b),
    ensures
        !ends_in_comment(s, a, b + 1),
    decreases b - a,
{
    reveal_with_fuel(ends_in_comment, 3);
    reveal_with_fuel(trivia_core, 3);
    if a >= b {
    } else if ws_at(s, a) {
        lemma_ends_ws(s, a + 1, b);
    } else if s[a] == b'#' && comment_ok(s, a) && a < comment_end(s, a) && comment_end(s, a) <= b {
        lemma_ends_ws(s, comment_end(s, a), b);
    }
}

/// A comment followed by a line feed does not leave the gap ending in a
/// comment, because the line feed joins the gap after it.
pub proof fn lemma_ends_comment_newline(s: Seq<u8>, a: int, b: int)
    requires
        0 <= a,
        trivia_core(s, a, b),
        b < s.len() as int,
        ws_at(s, b),
    ensures
        !ends_in_comment(s, a, b + 1),
    decreases b - a,
{
    reveal_with_fuel(ends_in_comment, 3);
    reveal_with_fuel(trivia_core, 3);
    if a >= b {
    } else if ws_at(s, a) {
        lemma_ends_comment_newline(s, a + 1, b);
    } else if s[a] == b'#' && comment_ok(s, a) && a < comment_end(s, a) && comment_end(s, a) <= b {
        lemma_ends_comment_newline(s, comment_end(s, a), b);
    }
}

/// The empty gap is trivia.
pub proof fn lemma_trivia_empty(s: Seq<u8>, a: int, last: bool)
    requires
        0 <= a <= s.len() as int,
    ensures
        trivia(s, a, a, last),
{
    reveal(trivia);
    reveal_with_fuel(trivia_core, 1);
    reveal_with_fuel(ends_in_comment, 1);
}

/// Adding one whitespace byte keeps a gap trivia.
pub proof fn lemma_trivia_item_ws(s: Seq<u8>, a: int, b: int, last: bool)
    requires
        0 <= a <= b,
        trivia(s, a, b, last),
        ws_at(s, b),
    ensures
        trivia(s, a, b + 1, last),
{
    reveal(trivia);
    lemma_trivia_core_ws(s, a, b);
    if !last {
        lemma_ends_ws(s, a, b);
    }
}

/// A comment that reaches the end of the source completes a trailing gap.
pub proof fn lemma_trivia_after_comment_eof(s: Seq<u8>, a: int, b: int)
    requires
        0 <= a <= b,
        trivia(s, a, b, true),
        0 <= b < s.len() as int,
        s[b] == b'#',
        comment_ok(s, b),
        comment_end(s, b) == s.len() as int,
    ensures
        trivia(s, a, s.len() as int, true),
        gap_has_newline(s, a, comment_end(s, b)) == gap_has_newline(s, a, b),
{
    reveal(trivia);
    lemma_trivia_core_comment(s, a, b);
    assert forall|i: int| b <= i < comment_end(s, b) implies #[trigger] s[i] != b'\n' by {
        if i != b {
            lemma_comment_body_byte(s, b, i);
        }
    };
    lemma_newline_unchanged(s, a, b, comment_end(s, b));
}

/// A comment followed by a line feed keeps a gap trivia and fed.
pub proof fn lemma_trivia_after_comment_lf(s: Seq<u8>, a: int, b: int, last: bool)
    requires
        0 <= a <= b,
        trivia(s, a, b, last),
        0 <= b < s.len() as int,
        s[b] == b'#',
        comment_ok(s, b),
        comment_end(s, b) < s.len() as int,
        s[comment_end(s, b)] == b'\n',
    ensures
        trivia(s, a, comment_end(s, b) + 1, last),
        gap_has_newline(s, a, comment_end(s, b) + 1),
{
    reveal(trivia);
    lemma_trivia_core_comment(s, a, b);
    lemma_trivia_core_ws(s, a, comment_end(s, b));
    if !last {
        lemma_ends_comment_newline(s, a, comment_end(s, b));
    }
    lemma_newline_gained(s, a, comment_end(s, b), comment_end(s, b) + 1);
}

/// A comment followed by a carriage-return line feed keeps a gap trivia and
/// fed.
pub proof fn lemma_trivia_after_comment_crlf(s: Seq<u8>, a: int, b: int, last: bool)
    requires
        0 <= a <= b,
        trivia(s, a, b, last),
        0 <= b < s.len() as int,
        s[b] == b'#',
        comment_ok(s, b),
        comment_end(s, b) + 1 < s.len() as int,
        s[comment_end(s, b)] == b'\r',
        s[comment_end(s, b) + 1] == b'\n',
    ensures
        trivia(s, a, comment_end(s, b) + 2, last),
        gap_has_newline(s, a, comment_end(s, b) + 2),
{
    reveal(trivia);
    lemma_trivia_core_comment(s, a, b);
    lemma_trivia_core_ws(s, a, comment_end(s, b));
    lemma_trivia_core_ws(s, a, comment_end(s, b) + 1);
    if !last {
        lemma_ends_comment_newline(s, a, comment_end(s, b));
        lemma_ends_ws(s, a, comment_end(s, b) + 1);
    }
    lemma_newline_gained(s, a, comment_end(s, b) + 1, comment_end(s, b) + 2);
}


} // verus!
