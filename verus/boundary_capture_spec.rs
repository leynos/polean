//! Specification for the stream-capture kernel.
//!
//! The model is a fold over the stream's chunks. `stream_at(chunks, i)` is the
//! first `i` chunks concatenated, and `fold_at(cap, chunks, i)` is the capture
//! state after those chunks. [`captured_upto`] states what that state must be:
//! at most `cap` bytes, exactly the stream's first `prefix_len` bytes, and
//! tripped exactly when the stream is longer than the cap.
//!
//! Both functions recur on the chunk count, so the model's step is the same
//! shape as the kernel's: one chunk appended to the previous prefix. The
//! lemmas in `verus/boundary_capture_lemmas.rs` prove the model theorem for
//! every finite stream, and the kernel's `feed` is checked against one model
//! step. The step from repeated `feed` calls to the fold is a manual
//! composition of those two checked results; no verified trace harness
//! performs it.
//!
//! These definitions name no executable function. They are the model the proof
//! reasons about, so executable code that agreed with itself could not make the
//! model agree with it. The kernel's `feed` is verified against one model step,
//! [`step`].
//!
//! # Why a fold and not a counter
//!
//! The kernel keeps no count of bytes seen. Before the first trip the count
//! equals the kept length, and after it the state cannot change again, so the
//! whole-stream property needs no second number and no arithmetic can overflow.
//! The price is that "how many bytes were seen" is not recoverable from a
//! tripped capture, which the supervisor does not need: it stops the job.

use vstd::prelude::*;

verus! {

/// The room left below the cap, which is non-negative whenever the state's
/// invariant holds.
pub open spec fn room(cap: nat, kept: Seq<u8>) -> int { cap as int - kept.len() as int }

/// How many of a chunk's bytes a step keeps.
///
/// This is the smaller of the chunk's length and the room left. It needs
/// `0 <= room(cap, kept)`, which [`captured_upto`] maintains by keeping the
/// state at or below the cap.
pub open spec fn keep_count(cap: nat, kept: Seq<u8>, chunk: Seq<u8>) -> int {
    if chunk.len() as int <= room(cap, kept) {
        chunk.len() as int
    } else {
        room(cap, kept)
    }
}

/// One model step: keep the chunk's first `keep_count` bytes and trip when the
/// chunk does not fit whole.
pub open spec fn step(cap: nat, state: (Seq<u8>, bool), chunk: Seq<u8>) -> (Seq<u8>, bool) {
    (
        state.0 + chunk.subrange(0, keep_count(cap, state.0, chunk)),
        state.1 || chunk.len() as int > room(cap, state.0),
    )
}

/// The state after the first `index` chunks, starting from an empty capture.
pub open spec fn fold_at(cap: nat, chunks: Seq<Seq<u8>>, index: nat) -> (Seq<u8>, bool)
    decreases index
{
    if index == 0 {
        (Seq::<u8>::empty(), false)
    } else {
        step(cap, fold_at(cap, chunks, (index - 1) as nat), chunks[(index - 1) as int])
    }
}

/// The state after every chunk, starting from an empty capture.
pub open spec fn fold(cap: nat, chunks: Seq<Seq<u8>>) -> (Seq<u8>, bool) {
    fold_at(cap, chunks, chunks.len())
}

/// The first `index` chunks concatenated, in order.
pub open spec fn stream_at(chunks: Seq<Seq<u8>>, index: nat) -> Seq<u8>
    decreases index
{
    if index == 0 {
        Seq::<u8>::empty()
    } else {
        stream_at(chunks, (index - 1) as nat) + chunks[(index - 1) as int]
    }
}

/// Every chunk concatenated, in order.
pub open spec fn stream(chunks: Seq<Seq<u8>>) -> Seq<u8> { stream_at(chunks, chunks.len()) }

/// How many bytes of a stream are kept: the smaller of its length and the cap.
pub open spec fn prefix_len(cap: nat, s: Seq<u8>) -> int {
    if s.len() as int <= cap as int {
        s.len() as int
    } else {
        cap as int
    }
}

/// The first `n` bytes of `s`.
pub open spec fn prefix(s: Seq<u8>, n: int) -> Seq<u8> { s.subrange(0, n) }

/// What any capture state after `index` chunks must be.
///
/// The three conjuncts are the bound, the prefix property, and the trip
/// property. The prefix clause is the load-bearing one: the other two admit
/// many byte sequences for one stream, and `witness_crossing_unique` in
/// `verus/boundary_capture_witnesses.rs` shows that it pins exactly one.
///
/// Monotonicity after a trip is not a fourth conjunct: it follows from the
/// prefix property at every index, and is proved as `lemma_captured_stable` for
/// pairs of indices.
pub open spec fn meets_capture(
    cap: nat,
    chunks: Seq<Seq<u8>>,
    index: nat,
    kept: Seq<u8>,
    tripped: bool,
) -> bool {
    let s = stream_at(chunks, index);
    kept.len() <= cap && kept == prefix(s, prefix_len(cap, s))
        && tripped == (s.len() as int > cap as int)
}

/// What the capture state after `index` chunks must be.
pub open spec fn captured_upto(cap: nat, chunks: Seq<Seq<u8>>, index: nat) -> bool {
    index <= chunks.len() && meets_capture(
        cap,
        chunks,
        index,
        fold_at(cap, chunks, index).0,
        fold_at(cap, chunks, index).1,
    )
}

/// What the capture state after every chunk must be.
pub open spec fn captured(cap: nat, chunks: Seq<Seq<u8>>) -> bool {
    captured_upto(cap, chunks, chunks.len())
}

} // verus!
