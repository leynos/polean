//! Completeness of the recognizer's acceptance on whitespace-only sources.
//!
//! The kernel's soundness contract is one-way: every accepted offset list is a
//! decomposition. These predicates state the converse for the restricted case
//! the executable baseline instance needs, where the source holds no comment
//! marker and every gap is a maximal run of whitespace.
//!
//! The first half states the relation over the kernel's slice tokens. A
//! concrete source can only state its token bytes as sequences, because a
//! byte-literal slice has opaque contents, so the second half mirrors the same
//! relation over `Seq<Seq<u8>>`.

use vstd::prelude::*;

// The specification is erased by plain `rustc`, so this import exists only for
// the verifier and would otherwise be unused.
#[cfg(verus_keep_ghost)]
use super::spec::*;

verus! {

// ---------------------------------------------------------------------------
// Completeness on whitespace-only decompositions
// ---------------------------------------------------------------------------

/// The offset of the first byte at or after `i` that is not whitespace, or the
/// end of the source.
///
/// This is the result [`ws_between`] requires of a gap end, so it makes the
/// maximality of a whitespace gap a functional fact.
pub open spec fn ws_extent_from(s: Seq<u8>, i: int) -> int
    decreases s.len() - i
{
    if i >= s.len() || !ws_at(s, i) {
        i
    } else {
        ws_extent_from(s, i + 1)
    }
}

/// The offset at which the whitespace run that starts at `i` ends.
pub open spec fn ws_extent(s: Seq<u8>, i: int) -> int { ws_extent_from(s, i) }

/// Whether no byte of `s` is a comment marker.
///
/// Recognition under an empty `#` set has no comment branch at all, which is
/// what makes its completeness argument a scan over whitespace. The baseline
/// contains no `#`, so the completeness claim covers it.
pub open spec fn no_hash(s: Seq<u8>) -> bool {
    forall|i: int| 0 <= i < s.len() ==> #[trigger] s[i] != b'#'
}

/// Whether `a..b` is a maximal run of whitespace.
///
/// The byte at `b` is not whitespace, so the run cannot be extended at its
/// right-hand end. At the end of the source [`ws_at`] is false, so the last
/// clause holds there too.
pub open spec fn ws_between(s: Seq<u8>, a: int, b: int) -> bool {
    &&& 0 <= a <= b <= s.len()
    &&& (forall|i: int| a <= i < b ==> #[trigger] ws_at(s, i))
    &&& !ws_at(s, b)
}

/// Whether some decomposition of `s` into `k` is canonical.
///
/// The recognizer's completeness contract and the ghost witness it reasons
/// about must be the same proposition, so both name it here rather than
/// spelling out the existential separately.
pub open spec fn admits_canonical(s: Seq<u8>, k: Seq<&[u8]>, flags: Seq<bool>) -> bool {
    exists|q: Seq<usize>| #[trigger] ws_canonical(s, k, flags, q)
}

/// Whether `i..` holds a comment marker.
///
/// The scan form of [`no_hash`], so a concrete source can discharge the
/// universal claim by evaluating a recursion.
pub open spec fn has_hash_from(s: Seq<u8>, i: int) -> bool
    decreases s.len() - i
{
    if i >= s.len() {
        false
    } else {
        s[i] == b'#' || has_hash_from(s, i + 1)
    }
}

/// Whether `k` sits at `p` with only whitespace in the gaps.
///
/// This is the completeness counterpart of [`decomposes_at`]: it pins every
/// offset, because each gap ends at the first byte that is not whitespace.
/// It is a sufficient condition for the recognizer to succeed, not a
/// characterization of its successful inputs, which may also hold comments.
///
/// Opaque because the recognizer's loop carries it as an assumption about a
/// chosen witness; unfolding its quantifiers in every proof obligation is the
/// dominant cost of verifying completeness. Proofs that need the definition
/// reveal it.
#[verifier::opaque]
pub open spec fn ws_canonical(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    flags: Seq<bool>,
    p: Seq<usize>,
) -> bool {
    &&& no_hash(s)
    &&& p.len() == k.len()
    &&& flags.len() == k.len()
    &&& (forall|j: int| 0 <= j < k.len() ==> #[trigger] k[j]@.len() > 0)
    &&& (forall|j: int| 0 <= j < k.len() ==> #[trigger] token_at(s, p[j] as int, k[j]))
    &&& if k.len() == 0 {
        ws_between(s, 0, s.len() as int)
    } else {
        &&& ws_between(s, 0, p[0] as int)
        &&& (forall|j: int|
            0 < j < k.len() ==> p[j - 1] as int + k[j - 1]@.len() < #[trigger] p[j] as int
                && ws_between(s, p[j - 1] as int + k[j - 1]@.len(), p[j] as int))
        &&& ws_between(s, p[k.len() - 1] as int + k[k.len() - 1]@.len(), s.len() as int)
    }
    &&& flags_hold(s, k, p, flags)
}

// ---------------------------------------------------------------------------
// The canonical predicate over token byte sequences
// ---------------------------------------------------------------------------
//
// [`ws_canonical`] is stated over slice tokens because that is what the kernel
// holds. A concrete source can only state its token bytes as sequences, because
// a byte-literal slice has opaque contents. These mirrors let a concrete proof
// inspect the bytes, and a transport lemma moves the result to the kernel's
// type.

/// The offset at which the token before index `j` ends, for sequence tokens.
pub open spec fn prev_end_t(t: Seq<Seq<u8>>, p: Seq<usize>, j: int) -> int {
    if j == 0 {
        0
    } else {
        p[j - 1] as int + t[j - 1].len()
    }
}

/// The sequence-token counterpart of [`flags_prefix`].
#[verifier::opaque]
pub open spec fn flags_prefix_t(
    s: Seq<u8>,
    t: Seq<Seq<u8>>,
    p: Seq<usize>,
    flags: Seq<bool>,
    m: int,
) -> bool {
    forall|j: int| 0 <= j < m && flags[j] ==> gap_has_newline(
        s,
        prev_end_t(t, p, j),
        #[trigger] p[j] as int,
    )
}

/// The sequence-token counterpart of [`flags_hold`].
pub open spec fn flags_hold_t(
    s: Seq<u8>,
    t: Seq<Seq<u8>>,
    p: Seq<usize>,
    flags: Seq<bool>,
) -> bool {
    flags.len() == t.len() && flags_prefix_t(s, t, p, flags, t.len() as int)
}

/// The sequence-token counterpart of [`ws_canonical`].
#[verifier::opaque]
pub open spec fn ws_canonical_t(
    s: Seq<u8>,
    t: Seq<Seq<u8>>,
    flags: Seq<bool>,
    p: Seq<usize>,
) -> bool {
    &&& no_hash(s)
    &&& p.len() == t.len()
    &&& flags.len() == t.len()
    &&& (forall|j: int| 0 <= j < t.len() ==> #[trigger] t[j].len() > 0)
    &&& (forall|j: int| 0 <= j < t.len() ==> #[trigger] token_seq_at(s, p[j] as int, t[j]))
    &&& if t.len() == 0 {
        ws_between(s, 0, s.len() as int)
    } else {
        &&& ws_between(s, 0, p[0] as int)
        &&& (forall|j: int|
            0 < j < t.len() ==> p[j - 1] as int + t[j - 1].len() < #[trigger] p[j] as int
                && ws_between(s, p[j - 1] as int + t[j - 1].len(), p[j] as int))
        &&& ws_between(s, p[t.len() - 1] as int + t[t.len() - 1].len(), s.len() as int)
    }
    &&& flags_hold_t(s, t, p, flags)
}

/// Whether some decomposition of `s` into `t` is canonical.
pub open spec fn admits_canonical_t(s: Seq<u8>, t: Seq<Seq<u8>>, flags: Seq<bool>) -> bool {
    exists|q: Seq<usize>| #[trigger] ws_canonical_t(s, t, flags, q)
}

} // verus!
