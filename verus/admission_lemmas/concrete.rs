//! Concrete scans and transport for the completeness proof.
//!
//! The lemmas here turn an evaluated whitespace scan into the specification's
//! maximality claim, so a proof about one concrete source need not reason over
//! quantifiers. The transport lemma then moves a decomposition stated over
//! token byte sequences to the kernel's slice tokens.

use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use super::super::completeness::*;
#[cfg(verus_keep_ghost)]
use super::super::spec::*;

verus! {

// ---------------------------------------------------------------------------
// Concrete scans
// ---------------------------------------------------------------------------

/// The whitespace extent never precedes its starting point.
pub proof fn lemma_ws_extent_ge(s: Seq<u8>, i: int)
    requires
        0 <= i <= s.len(),
    ensures
        ws_extent_from(s, i) >= i,
    decreases s.len() - i,
{
    if i >= s.len() || !ws_at(s, i) {
    } else {
        lemma_ws_extent_ge(s, i + 1);
    }
}

/// A scan that stops at `b` certifies `a..b` as a maximal whitespace run.
///
/// This is the converse of [`lemma_ws_between_extent`]. A concrete source can
/// evaluate the scan on constant offsets, so it lets a proof about particular
/// bytes establish [`ws_between`] without arguing about quantifiers.
pub proof fn lemma_ws_extent_between(s: Seq<u8>, a: int, b: int)
    requires
        0 <= a <= b <= s.len(),
        ws_extent_from(s, a) == b,
    ensures
        ws_between(s, a, b),
    decreases b - a,
{
    if a >= b {
        assert(a == b);
        if a < s.len() && ws_at(s, a) {
            // The scan would advance past `a`, contradicting the constant
            // result.
            lemma_ws_extent_ge(s, a + 1);
            assert(false);
        }
        assert(!ws_at(s, a));
    } else {
        assert(ws_at(s, a));
        assert(ws_extent_from(s, a + 1) == b);
        lemma_ws_extent_between(s, a + 1, b);
        assert forall|i: int| a <= i < b implies #[trigger] ws_at(s, i) by {
            if i > a {
            }
        };
        assert(!ws_at(s, b));
    }
}

/// The scan from `i` finds a comment marker exactly where `i..` holds one.
pub proof fn lemma_no_hash_from(s: Seq<u8>, i: int)
    requires
        0 <= i <= s.len(),
    ensures
        (forall|k: int| i <= k < s.len() ==> #[trigger] s[k] != b'#') <==> !has_hash_from(
            s,
            i,
        ),
    decreases s.len() - i,
{
    if i >= s.len() {
    } else {
        lemma_no_hash_from(s, i + 1);
        assert(has_hash_from(s, i) == (s[i] == b'#' || has_hash_from(s, i + 1)));
    }
}

/// A source holds no comment marker exactly when a scan from zero finds none.
pub proof fn lemma_no_hash_scan(s: Seq<u8>)
    ensures
        no_hash(s) <==> !has_hash_from(s, 0),
{
    lemma_no_hash_from(s, 0);
}

// ---------------------------------------------------------------------------
// Transport to slice tokens
// ---------------------------------------------------------------------------

/// A canonical decomposition over token byte sequences is canonical for any
/// slices carrying those bytes.
///
/// The kernel holds `&[u8]` tokens, whose contents the verifier cannot inspect
/// when they come from a byte literal. A concrete proof therefore states its
/// tokens as sequences and transfers the result here.
///
/// The relation is opaque and the two token types have separate projections,
/// so one query carries four unfolds; the limit reflects that.
#[verifier::rlimit(240)]
pub proof fn lemma_ws_canonical_t_to_slices(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    t: Seq<Seq<u8>>,
    flags: Seq<bool>,
    p: Seq<usize>,
)
    requires
        t.len() == k.len(),
        forall|j: int| 0 <= j < k.len() ==> #[trigger] k[j]@ =~= t[j],
        ws_canonical_t(s, t, flags, p),
    ensures
        ws_canonical(s, k, flags, p),
{
    reveal(ws_canonical_t);
    reveal(ws_canonical);
    reveal(flags_prefix);
    reveal(flags_prefix_t);
    reveal(flags_hold);
    reveal(flags_hold_t);
    assert forall|j: int| 0 <= j < k.len() implies #[trigger] token_at(
        s,
        p[j] as int,
        k[j],
    ) by {
        assert(token_seq_at(s, p[j] as int, t[j]));
        assert(k[j]@ =~= t[j]);
    };
    assert forall|j: int| 0 <= j < k.len() && flags[j] implies gap_has_newline(
        s,
        prev_end(s, k, p, j),
        #[trigger] p[j] as int,
    ) by {
        assert(prev_end(s, k, p, j) == prev_end_t(t, p, j));
    };
}

} // verus!
