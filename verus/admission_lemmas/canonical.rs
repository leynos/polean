//! Proofs that the recognizer succeeds on whitespace-only decompositions.
//!
//! The kernel's main contract is a soundness statement: an `Ok` decomposes the
//! source as the tokens say. An implementation that always answered `Err`
//! would satisfy it, so the proof alone would not show that the recognizer
//! accepts anything. The lemmas here supply the other direction for the shape
//! the baseline has: tokens separated by maximal runs of whitespace. They also
//! show that such a decomposition is unique, which lets the executable
//! instance state the exact offsets it expects.

use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use super::super::completeness::*;
#[cfg(verus_keep_ghost)]
use super::super::spec::*;

verus! {

// ---------------------------------------------------------------------------
// The whitespace extent
// ---------------------------------------------------------------------------

/// Consuming one whitespace byte does not change where the run ends.
pub proof fn lemma_ws_extent_step(s: Seq<u8>, i: int)
    requires
        0 <= i < s.len() as int,
        ws_at(s, i),
    ensures
        ws_extent_from(s, i) == ws_extent_from(s, i + 1),
{
    reveal_with_fuel(ws_extent_from, 1);
}

/// A byte that is not whitespace ends the run at itself.
pub proof fn lemma_ws_extent_stops(s: Seq<u8>, i: int)
    requires
        0 <= i <= s.len() as int,
        !ws_at(s, i),
    ensures
        ws_extent_from(s, i) == i,
{
    reveal_with_fuel(ws_extent_from, 1);
}

/// A maximal run of whitespace ends exactly where the extent scan stops.
pub proof fn lemma_ws_between_extent(s: Seq<u8>, a: int, b: int)
    requires
        ws_between(s, a, b),
    ensures
        ws_extent_from(s, a) == b,
    decreases b - a,
{
    if a >= b {
        assert(a == b);
        assert(!ws_at(s, a));
        lemma_ws_extent_stops(s, a);
    } else {
        assert(ws_at(s, a));
        lemma_ws_extent_step(s, a);
        assert(ws_between(s, a + 1, b)) by {
            assert forall|i: int| a + 1 <= i < b implies #[trigger] ws_at(s, i) by {
            };
            assert(!ws_at(s, b));
        }
        lemma_ws_between_extent(s, a + 1, b);
    }
}

// ---------------------------------------------------------------------------
// The gap a canonical decomposition prescribes
// ---------------------------------------------------------------------------

/// The gap before token `j` starts where the previous token ended.
pub proof fn lemma_ws_canonical_gap_start(s: Seq<u8>, k: Seq<&[u8]>, p: Seq<usize>, j: int)
    requires
        p.len() == k.len(),
        j <= k.len(),
    ensures
        prev_end(s, k, p, j) == if j == 0 {
            0
        } else {
            p[j - 1] as int + k[j - 1]@.len()
        },
{
}

/// A canonical decomposition's gap before token `j` ends at that token.
pub proof fn lemma_ws_canonical_gap(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    flags: Seq<bool>,
    p: Seq<usize>,
    j: int,
)
    requires
        ws_canonical(s, k, flags, p),
        0 <= j < k.len(),
    ensures
        ws_extent(s, prev_end(s, k, p, j)) == p[j] as int,
{
    reveal(ws_canonical);
    if j == 0 {
        assert(ws_between(s, 0, p[0] as int));
        assert(prev_end(s, k, p, 0) == 0);
        lemma_ws_between_extent(s, 0, p[0] as int);
    } else {
        let start = p[j - 1] as int + k[j - 1]@.len();
        assert(prev_end(s, k, p, j) == start);
        assert(p[j - 1] as int + k[j - 1]@.len() < p[j] as int);
        assert(ws_between(s, start, p[j] as int));
        lemma_ws_between_extent(s, start, p[j] as int);
    }
}

// ---------------------------------------------------------------------------
// Projections of the canonical predicate
// ---------------------------------------------------------------------------
//
// Each projection reveals the definition in its own goal and exports one
// consequence. The recognizer's loop uses them one at a time, so no single
// obligation has to unfold every quantifier in the predicate at once.

/// A canonical decomposition holds no comment marker.
pub proof fn lemma_ws_canonical_no_hash(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    flags: Seq<bool>,
    p: Seq<usize>,
)
    requires
        ws_canonical(s, k, flags, p),
    ensures
        no_hash(s),
{
    reveal(ws_canonical);
}

/// A canonical decomposition has one offset and one flag per token.
pub proof fn lemma_ws_canonical_lengths(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    flags: Seq<bool>,
    p: Seq<usize>,
)
    requires
        ws_canonical(s, k, flags, p),
    ensures
        p.len() == k.len(),
        flags.len() == k.len(),
{
    reveal(ws_canonical);
}

/// A canonical decomposition's tokens are non-empty.
pub proof fn lemma_ws_canonical_token_len(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    flags: Seq<bool>,
    p: Seq<usize>,
    j: int,
)
    requires
        ws_canonical(s, k, flags, p),
        0 <= j < k.len(),
    ensures
        k[j]@.len() > 0,
{
    reveal(ws_canonical);
}

/// A canonical decomposition's token sits at its offset.
pub proof fn lemma_ws_canonical_token_at(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    flags: Seq<bool>,
    p: Seq<usize>,
    j: int,
)
    requires
        ws_canonical(s, k, flags, p),
        0 <= j < k.len(),
    ensures
        token_at(s, p[j] as int, k[j]),
{
    reveal(ws_canonical);
}

/// A canonical decomposition's gap before token `j` is non-empty.
pub proof fn lemma_ws_canonical_gap_positive(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    flags: Seq<bool>,
    p: Seq<usize>,
    j: int,
)
    requires
        ws_canonical(s, k, flags, p),
        0 < j < k.len(),
    ensures
        p[j - 1] as int + k[j - 1]@.len() < p[j] as int,
{
    reveal(ws_canonical);
}

/// A canonical decomposition honours every line-feed flag.
pub proof fn lemma_ws_canonical_flags(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    flags: Seq<bool>,
    p: Seq<usize>,
)
    requires
        ws_canonical(s, k, flags, p),
    ensures
        flags_hold(s, k, p, flags),
{
    reveal(ws_canonical);
}

/// A canonical decomposition's trailing gap runs to the end of the source.
pub proof fn lemma_ws_canonical_trailing(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    flags: Seq<bool>,
    p: Seq<usize>,
)
    requires
        ws_canonical(s, k, flags, p),
    ensures
        ws_extent(s, prev_end(s, k, p, k.len() as int)) == s.len() as int,
{
    reveal(ws_canonical);
    let n = k.len() as int;
    if n == 0 {
        assert(ws_between(s, 0, s.len() as int));
        assert(prev_end(s, k, p, 0) == 0);
        lemma_ws_between_extent(s, 0, s.len() as int);
    } else {
        let start = p[n - 1] as int + k[n - 1]@.len();
        assert(prev_end(s, k, p, n) == start);
        assert(ws_between(s, start, s.len() as int));
        lemma_ws_between_extent(s, start, s.len() as int);
    }
}

// ---------------------------------------------------------------------------
// Uniqueness
// ---------------------------------------------------------------------------

/// Whether the first `j` offsets of two lists agree.
///
/// The recognizer builds its offsets one at a time, so the prefixes it needs to
/// relate are pointwise agreement rather than a `take`.
pub open spec fn offsets_agree(a: Seq<usize>, b: Seq<usize>, j: int) -> bool {
    forall|i: int| 0 <= i < j ==> #[trigger] a[i] == b[i]
}

/// Agreeing offset prefixes give the same previous end.
pub proof fn lemma_prev_end_agree(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    a: Seq<usize>,
    b: Seq<usize>,
    j: int,
)
    requires
        0 <= j <= k.len(),
        j <= a.len(),
        j <= b.len(),
        offsets_agree(a, b, j),
    ensures
        prev_end(s, k, a, j) == prev_end(s, k, b, j),
{
    if j > 0 {
        assert(a[j - 1] == b[j - 1]);
    }
}

/// Two canonical decompositions of one source agree on their first `j`
/// offsets.
pub proof fn lemma_ws_canonical_prefix_eq(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    flags: Seq<bool>,
    a: Seq<usize>,
    b: Seq<usize>,
    j: int,
)
    requires
        ws_canonical(s, k, flags, a),
        ws_canonical(s, k, flags, b),
        0 <= j <= k.len(),
    ensures
        offsets_agree(a, b, j),
    decreases j,
{
    reveal(ws_canonical);
    if j == 0 {
        assert(offsets_agree(a, b, 0));
    } else {
        lemma_ws_canonical_prefix_eq(s, k, flags, a, b, j - 1);
        assert(a.len() == k.len());
        assert(b.len() == k.len());
        lemma_prev_end_agree(s, k, a, b, j - 1);
        lemma_ws_canonical_gap(s, k, flags, a, j - 1);
        lemma_ws_canonical_gap(s, k, flags, b, j - 1);
        assert(a[j - 1] == b[j - 1]);
        assert(offsets_agree(a, b, j));
    }
}

/// A whitespace-only decomposition has exactly one offset for each token.
pub proof fn lemma_ws_canonical_unique(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    flags: Seq<bool>,
    a: Seq<usize>,
    b: Seq<usize>,
)
    requires
        ws_canonical(s, k, flags, a),
        ws_canonical(s, k, flags, b),
    ensures
        a == b,
{
    reveal(ws_canonical);
    lemma_ws_canonical_prefix_eq(s, k, flags, a, b, k.len() as int);
    assert(a.len() == k.len());
    assert(b.len() == k.len());
    assert(a =~= b);
}

} // verus!
