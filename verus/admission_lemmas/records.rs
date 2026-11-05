//! Proofs about the line-feed flag list and a gap's line-feed record.

use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use super::super::spec::*;

verus! {

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
