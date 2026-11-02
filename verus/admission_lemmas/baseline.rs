//! The concrete accepting baseline instance.

use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use super::super::spec::*;

verus! {

// ---------------------------------------------------------------------------
// The concrete baseline
// ---------------------------------------------------------------------------

/// Transfer the sequence-token decomposition to slice tokens with those bytes.
///
/// The kernel's relation is stated over `&[u8]` tokens. A proof about a
/// concrete source can only inspect token bytes stated as sequences, because a
/// byte-literal slice has opaque contents. This lemma closes that gap: any
/// slice whose view is the sequence the concrete proof inspected satisfies the
/// kernel's relation too.
pub proof fn lemma_decomposes_seq_to_slices(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    t: Seq<Seq<u8>>,
    p: Seq<usize>,
)
    requires
        t.len() == k.len(),
        forall|j: int| 0 <= j < k.len() ==> #[trigger] k[j]@ =~= t[j],
        decomposes_seq(s, t, p),
    ensures
        decomposes_at(s, k, p),
{
    reveal(trivia);
    assert forall|j: int| 0 <= j < k.len() implies #[trigger] token_at(s, p[j] as int, k[j]) by {
        assert(token_seq_at(s, p[j] as int, t[j]));
    };
}

/// Recognition accepts the canonical baseline.
///
/// The source, tokens, offsets, and flags are the values the executable test
/// beside the kernel checks, so the proof and the run exercise the same bytes.
///
/// The relation proved here is [`decomposes_seq`], the decomposition stated
/// over token byte sequences. [`lemma_decomposes_seq_to_slices`] turns it into
/// the kernel's own [`decomposes_at`] for any slice carrying those bytes.
pub proof fn lemma_baseline_accepts()
    ensures
        decomposes_seq(baseline_src(), baseline_tokens(), baseline_offsets()),
        gap_has_newline(baseline_src(), 115, 120),
{
    reveal(trivia);
    reveal_with_fuel(trivia_core, 10);
    reveal_with_fuel(ends_in_comment, 10);
    reveal_with_fuel(comment_end_from, 10);
    reveal_with_fuel(gap_has_newline, 10);
    assert(decomposes_seq(baseline_src(), baseline_tokens(), baseline_offsets()));
    assert(gap_has_newline(baseline_src(), 115, 120));
}


} // verus!
