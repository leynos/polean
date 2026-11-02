//! Proofs about comment ends and slice lengths.
//!
//! These are the scan primitives the gap rules and T1 build on.

use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use super::super::spec::*;

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


} // verus!
