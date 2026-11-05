//! The concrete accepting baseline instance.

use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use super::super::completeness::*;
#[cfg(verus_keep_ghost)]
use super::super::spec::*;
#[cfg(verus_keep_ghost)]
use super::canonical::*;
#[cfg(verus_keep_ghost)]
use super::concrete::*;

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

/// The baseline's canonical decomposition, stated over token byte sequences.
///
/// [`lemma_baseline_accepts`] shows the specification admits the bytes. This
/// lemma shows the same source is *unique*: the offsets it fixes leave only
/// whitespace in every gap, so [`lemma_ws_canonical_t_to_slices`] turns it into
/// the kernel's completeness premise for the concrete bytes.
///
/// The scan results below are the offsets of the first byte after each gap.
/// Stating them as constants is what lets the evaluator dispose of the
/// universal claims without the solver unfolding a 146-byte sequence.
///
/// One query carries nineteen evaluated scans and two quantified gap claims
/// over the literal offset and flag lists; the limit reflects that.
#[verifier::rlimit(400)]
pub proof fn lemma_baseline_is_canonical()
    ensures
        ws_canonical_t(
            baseline_src(),
            baseline_tokens(),
            baseline_flags(),
            baseline_offsets(),
        ),
{
    reveal(ws_canonical_t);
    reveal(flags_hold_t);
    reveal(flags_prefix_t);
    lemma_baseline_accepts();
    lemma_no_hash_scan(baseline_src());
    assert(!has_hash_from(baseline_src(), 0)) by (compute);
    assert(no_hash(baseline_src()));
    assert(baseline_flags().len() == baseline_tokens().len()) by (compute);
    // The gap before each token, and the trailing gap, is a maximal run of
    // whitespace. Each start is the end of the preceding token.
    assert(ws_extent_from(baseline_src(), 0) == 0) by (compute);
    lemma_ws_extent_between(baseline_src(), 0, 0);
    assert(ws_extent_from(baseline_src(), 7) == 8) by (compute);
    lemma_ws_extent_between(baseline_src(), 7, 8);
    assert(ws_extent_from(baseline_src(), 13) == 15) by (compute);
    lemma_ws_extent_between(baseline_src(), 13, 15);
    assert(ws_extent_from(baseline_src(), 21) == 22) by (compute);
    lemma_ws_extent_between(baseline_src(), 21, 22);
    assert(ws_extent_from(baseline_src(), 29) == 31) by (compute);
    lemma_ws_extent_between(baseline_src(), 29, 31);
    assert(ws_extent_from(baseline_src(), 38) == 39) by (compute);
    lemma_ws_extent_between(baseline_src(), 38, 39);
    assert(ws_extent_from(baseline_src(), 44) == 45) by (compute);
    lemma_ws_extent_between(baseline_src(), 44, 45);
    assert(ws_extent_from(baseline_src(), 47) == 48) by (compute);
    lemma_ws_extent_between(baseline_src(), 47, 48);
    assert(ws_extent_from(baseline_src(), 53) == 55) by (compute);
    lemma_ws_extent_between(baseline_src(), 53, 55);
    assert(ws_extent_from(baseline_src(), 60) == 61) by (compute);
    lemma_ws_extent_between(baseline_src(), 60, 61);
    assert(ws_extent_from(baseline_src(), 63) == 64) by (compute);
    lemma_ws_extent_between(baseline_src(), 63, 64);
    assert(ws_extent_from(baseline_src(), 65) == 70) by (compute);
    lemma_ws_extent_between(baseline_src(), 65, 70);
    assert(ws_extent_from(baseline_src(), 90) == 91) by (compute);
    lemma_ws_extent_between(baseline_src(), 90, 91);
    assert(ws_extent_from(baseline_src(), 93) == 94) by (compute);
    lemma_ws_extent_between(baseline_src(), 93, 94);
    assert(ws_extent_from(baseline_src(), 115) == 120) by (compute);
    lemma_ws_extent_between(baseline_src(), 115, 120);
    assert(ws_extent_from(baseline_src(), 132) == 133) by (compute);
    lemma_ws_extent_between(baseline_src(), 132, 133);
    assert(ws_extent_from(baseline_src(), 135) == 136) by (compute);
    lemma_ws_extent_between(baseline_src(), 135, 136);
    assert(ws_extent_from(baseline_src(), 143) == 144) by (compute);
    lemma_ws_extent_between(baseline_src(), 143, 144);
    assert(ws_extent_from(baseline_src(), 145) == 146) by (compute);
    lemma_ws_extent_between(baseline_src(), 145, 146);
    assert forall|j: int|
        0 < j < baseline_tokens().len() implies baseline_offsets()[j - 1] as int
            + baseline_tokens()[j - 1].len() < #[trigger] baseline_offsets()[j] as int
            && ws_between(
            baseline_src(),
            baseline_offsets()[j - 1] as int + baseline_tokens()[j - 1].len(),
            baseline_offsets()[j] as int,
        ) by {
    };
    assert forall|j: int|
        0 <= j < baseline_tokens().len() && baseline_flags()[j] implies gap_has_newline(
            baseline_src(),
            prev_end_t(baseline_tokens(), baseline_offsets(), j),
            #[trigger] baseline_offsets()[j] as int,
        ) by {
    };
}


} // verus!
