//! Proofs for the admission recognizer's specification.
//!
//! Each lemma lifts one step of the kernel's scan to the specification: a
//! whitespace byte or a whole comment extends a trivia gap, and consuming bytes
//! that are not line feeds leaves the gap's line-feed record unchanged. The
//! kernel calls these in its loop invariants; nothing here is assumed.

use vstd::prelude::*;

// The specification's items are erased by plain `rustc`, which would leave
// this import unused outside Verus, so it exists only for the verifier.
#[cfg(verus_keep_ghost)]
use super::spec::*;

verus! {

// ---------------------------------------------------------------------------
// Lengths
// ---------------------------------------------------------------------------

/// A slice's view fits the integer type of its own length.
///
/// The axiom that links a slice's view to `usize` is broadcast, so it fires
/// only once a `len()` term exists. Proofs that add two offsets need this
/// bound to show the sum cannot overflow.
pub proof fn lemma_slice_len_fits<T>(s: &[T])
    ensures
        s@.len() <= usize::MAX as int,
{
    assert(s.len() as int == s@.len());
}

// ---------------------------------------------------------------------------
// Comment ends
// ---------------------------------------------------------------------------

/// The comment-end scan never returns an offset before its start.
pub proof fn lemma_comment_end_ge(s: Seq<u8>, i: int)
    requires
        0 <= i <= s.len() as int,
    ensures
        comment_end_from(s, i) >= i,
    decreases s.len() as int - i,
{
    if i < s.len() as int && !line_end_at(s, i) && body_byte(s[i]) {
        lemma_comment_end_ge(s, i + 1);
    }
}

/// A comment-end scan stops at the offset it is given once that offset stops it.
pub proof fn lemma_comment_end_stops(s: Seq<u8>, i: int)
    requires
        0 <= i <= s.len() as int,
        i == s.len() as int || line_end_at(s, i) || !body_byte(s[i]),
    ensures
        comment_end_from(s, i) == i,
{
}

/// A comment-end scan is unchanged by stepping over one body byte.
pub proof fn lemma_comment_end_step(s: Seq<u8>, i: int)
    requires
        0 <= i < s.len() as int,
        !line_end_at(s, i),
        body_byte(s[i]),
    ensures
        comment_end_from(s, i) == comment_end_from(s, i + 1),
{
}

/// Every byte strictly inside a comment body is a body byte.
pub proof fn lemma_comment_end_from_body(s: Seq<u8>, start: int, i: int)
    requires
        0 <= start <= i < comment_end_from(s, start),
    ensures
        body_byte(s[i]),
        !line_end_at(s, i),
    decreases i - start,
{
    reveal_with_fuel(comment_end_from, 2);
    if start == i {
        assert(comment_end_from(s, start) != start);
    } else {
        lemma_comment_end_from_body(s, start + 1, i);
    }
}

/// Every byte strictly inside a well-formed comment's body is a body byte.
pub proof fn lemma_comment_body_byte(s: Seq<u8>, hash: int, i: int)
    requires
        0 <= hash < s.len() as int,
        s[hash] == b'#',
        comment_ok(s, hash),
        hash < i < comment_end(s, hash),
    ensures
        body_byte(s[i]),
        s[i] != b'\n',
{
    lemma_comment_end_from_body(s, hash + 1, i);
}

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

// ---------------------------------------------------------------------------
// Line-feed flags
// ---------------------------------------------------------------------------

/// Extending a flag prefix with one token keeps it, given that token's gap is
/// fed when its flag is set.
pub proof fn lemma_flags_prefix_push(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    p: Seq<usize>,
    flags: Seq<bool>,
    m: int,
    end: usize,
)
    requires
        flags_prefix(s, k, p, flags, m),
        0 <= m < k.len(),
        p.len() == m,
        flags[m] ==> gap_has_newline(s, prev_end(s, k, p, m), end as int),
    ensures
        flags_prefix(s, k, p.push(end), flags, m + 1),
{
    reveal(flags_prefix);
    assert forall|j: int|
        0 <= j < m + 1 && flags[j] implies gap_has_newline(
            s,
            prev_end(s, k, p.push(end), j),
            #[trigger] p.push(end)[j] as int,
        ) by
    {
        if j < m {
            assert(p.push(end)[j] == p[j]);
            assert(prev_end(s, k, p.push(end), j) == prev_end(s, k, p, j));
            assert(gap_has_newline(s, prev_end(s, k, p, j), p[j] as int));
        } else {
            assert(prev_end(s, k, p.push(end), m) == prev_end(s, k, p, m));
            assert(p.push(end)[m] == end);
        }
    };
}

// ---------------------------------------------------------------------------
// Line-feed records
// ---------------------------------------------------------------------------

/// A gap with no line feed has no line-feed record.
pub proof fn lemma_newline_absent(s: Seq<u8>, a: int, b: int)
    requires
        0 <= a <= b <= s.len() as int,
        forall|i: int| a <= i < b ==> #[trigger] s[i] != b'\n',
    ensures
        !gap_has_newline(s, a, b),
    decreases b - a,
{
    reveal_with_fuel(gap_has_newline, 2);
    if a < b {
        lemma_newline_absent(s, a + 1, b);
    }
}

/// A line feed inside a gap gives the gap a line-feed record.
pub proof fn lemma_newline_present(s: Seq<u8>, a: int, b: int, i: int)
    requires
        0 <= a <= i < b <= s.len() as int,
        s[i] == b'\n',
    ensures
        gap_has_newline(s, a, b),
    decreases i - a,
{
    reveal_with_fuel(gap_has_newline, 2);
    if a < i {
        lemma_newline_present(s, a + 1, b, i);
    }
}

/// Appending bytes that are not line feeds does not change a gap's record.
pub proof fn lemma_newline_unchanged(s: Seq<u8>, a: int, b: int, c: int)
    requires
        0 <= a <= b <= c <= s.len() as int,
        forall|i: int| b <= i < c ==> #[trigger] s[i] != b'\n',
    ensures
        gap_has_newline(s, a, c) == gap_has_newline(s, a, b),
    decreases b - a,
{
    reveal_with_fuel(gap_has_newline, 2);
    if a < b {
        lemma_newline_unchanged(s, a + 1, b, c);
    } else {
        lemma_newline_absent(s, a, c);
    }
}

/// Appending a line feed gives a gap a line-feed record.
pub proof fn lemma_newline_gained(s: Seq<u8>, a: int, b: int, c: int)
    requires
        0 <= a <= b < c <= s.len() as int,
        s[b] == b'\n',
    ensures
        gap_has_newline(s, a, c),
{
    lemma_newline_present(s, a, c, b);
}

} // verus!
