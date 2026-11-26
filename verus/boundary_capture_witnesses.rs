//! Witnesses showing the capture specification is reachable and not vacuous.
//!
//! Each witness runs the model on a concrete chunk sequence and asserts the
//! exact bytes it keeps. Together they cover every case the properties turn on:
//! a stream under the cap, one landing exactly on it, one crossing it inside a
//! chunk, empty chunks interspersed, and the degenerate zero cap with and
//! without a byte. Each also invokes [`lemma_captured`], so the witnesses do not
//! merely agree with a hand-written expectation: they satisfy the proved
//! specification.
//!
//! [`witness_crossing_unique`] is the non-vacuity argument for the prefix
//! clause. It states that for the crossing stream, any state meeting the
//! capture clauses keeps exactly the expected bytes. Without the prefix clause
//! the bound and trip clauses admit many byte sequences and that proof fails,
//! which is what the specification negative control exercises.

use vstd::prelude::*;

verus! {

use super::{lemmas::*, spec::*};

/// Two chunks, three bytes: under a cap of four.
pub open spec fn under_cap() -> Seq<Seq<u8>> { seq![seq![1u8, 2u8], seq![3u8]] }

/// Two chunks, four bytes: exactly on a cap of four.
pub open spec fn exactly_cap() -> Seq<Seq<u8>> { seq![seq![1u8, 2u8], seq![3u8, 4u8]] }

/// Two chunks, six bytes: the second crosses a cap of four.
pub open spec fn crosses_cap() -> Seq<Seq<u8>> { seq![seq![1u8, 2u8, 3u8], seq![4u8, 5u8, 6u8]] }

/// Empty chunks before, between, and after the bytes: under a cap of two.
pub open spec fn empty_chunks() -> Seq<Seq<u8>> {
    seq![
        Seq::<u8>::empty(),
        Seq::<u8>::empty(),
        seq![1u8],
        Seq::<u8>::empty(),
        seq![2u8],
        Seq::<u8>::empty(),
    ]
}

/// A stream under the cap is kept whole and does not trip.
pub proof fn witness_under_cap()
    ensures
        fold(4, seq![seq![1u8, 2u8], seq![3u8]]).0 == seq![1u8, 2u8, 3u8],
        !fold(4, seq![seq![1u8, 2u8], seq![3u8]]).1,
{
    reveal_with_fuel(fold_at, 8);
    assert(under_cap() == seq![seq![1u8, 2u8], seq![3u8]]);
    lemma_captured(4, under_cap());
}

/// A stream landing exactly on the cap is kept whole and does not trip.
pub proof fn witness_exactly_cap()
    ensures
        fold(4, seq![seq![1u8, 2u8], seq![3u8, 4u8]]).0 == seq![1u8, 2u8, 3u8, 4u8],
        !fold(4, seq![seq![1u8, 2u8], seq![3u8, 4u8]]).1,
{
    reveal_with_fuel(fold_at, 8);
    assert(exactly_cap() == seq![seq![1u8, 2u8], seq![3u8, 4u8]]);
    lemma_captured(4, exactly_cap());
}

/// A chunk crossing the cap keeps only the bytes that fit and trips.
pub proof fn witness_crosses_cap()
    ensures
        fold(4, seq![seq![1u8, 2u8, 3u8], seq![4u8, 5u8, 6u8]]).0 == seq![
            1u8,
            2u8,
            3u8,
            4u8,
        ],
        fold(4, seq![seq![1u8, 2u8, 3u8], seq![4u8, 5u8, 6u8]]).1,
{
    reveal_with_fuel(fold_at, 8);
    assert(crosses_cap() == seq![seq![1u8, 2u8, 3u8], seq![4u8, 5u8, 6u8]]);
    lemma_captured(4, crosses_cap());
}

/// Empty chunks neither extend the kept bytes nor trip the cap.
pub proof fn witness_empty_chunks()
    ensures
        fold(
            2,
            seq![
                Seq::<u8>::empty(),
                Seq::<u8>::empty(),
                seq![1u8],
                Seq::<u8>::empty(),
                seq![2u8],
                Seq::<u8>::empty(),
            ],
        ).0 == seq![1u8, 2u8],
        !fold(
            2,
            seq![
                Seq::<u8>::empty(),
                Seq::<u8>::empty(),
                seq![1u8],
                Seq::<u8>::empty(),
                seq![2u8],
                Seq::<u8>::empty(),
            ],
        ).1,
{
    reveal_with_fuel(fold_at, 8);
    assert(empty_chunks() == seq![
        Seq::<u8>::empty(),
        Seq::<u8>::empty(),
        seq![1u8],
        Seq::<u8>::empty(),
        seq![2u8],
        Seq::<u8>::empty(),
    ]);
    lemma_captured(2, empty_chunks());
}

/// A zero cap keeps nothing from a non-empty stream and trips.
pub proof fn witness_zero_cap()
    ensures
        fold(0, seq![seq![1u8]]).0 == Seq::<u8>::empty(),
        fold(0, seq![seq![1u8]]).1,
{
    reveal_with_fuel(fold_at, 8);
    lemma_captured(0, seq![seq![1u8]]);
}

/// A zero cap does not trip on an empty stream, because the cap is passed, not
/// reached.
pub proof fn witness_zero_cap_empty()
    ensures
        fold(0, seq![Seq::<u8>::empty()]).0 == Seq::<u8>::empty(),
        !fold(0, seq![Seq::<u8>::empty()]).1,
{
    reveal_with_fuel(fold_at, 8);
    lemma_captured(0, seq![Seq::<u8>::empty()]);
}

/// The crossing stream's capture clauses are satisfiable.
///
/// Without this, [`witness_crossing_unique`]'s premise could be unsatisfiable
/// and its conclusion vacuous.
pub proof fn witness_crossing_meets()
    ensures
        meets_capture(4, crosses_cap(), 2, seq![1u8, 2u8, 3u8, 4u8], true),
{
    reveal_with_fuel(fold_at, 8);
    reveal_with_fuel(stream_at, 8);
    assert(crosses_cap() == seq![seq![1u8, 2u8, 3u8], seq![4u8, 5u8, 6u8]]);
    assert(stream_at(crosses_cap(), 2) == seq![1u8, 2u8, 3u8, 4u8, 5u8, 6u8]);
    assert(prefix(stream_at(crosses_cap(), 2), 4) == seq![1u8, 2u8, 3u8, 4u8]);
    lemma_captured(4, crosses_cap());
}

/// The prefix clause pins the kept bytes: no other state meets the clauses.
///
/// This is the witness lemma the specification negative control breaks. It
/// needs the prefix clause; with only a length bound and a trip flag, a state
/// could keep any four bytes of the right length.
pub proof fn witness_crossing_unique(kept: Seq<u8>, tripped: bool)
    requires
        meets_capture(4, crosses_cap(), 2, kept, tripped),
    ensures
        kept == seq![1u8, 2u8, 3u8, 4u8],
        tripped,
{
    reveal_with_fuel(stream_at, 8);
    let stream = stream_at(crosses_cap(), 2);
    assert(crosses_cap() == seq![seq![1u8, 2u8, 3u8], seq![4u8, 5u8, 6u8]]);
    assert(stream == seq![1u8, 2u8, 3u8, 4u8, 5u8, 6u8]);
    assert(prefix_len(4, stream) == 4);
    assert(prefix(stream, 4) == seq![1u8, 2u8, 3u8, 4u8]);
    assert(kept == prefix(stream, prefix_len(4, stream)));
    assert(tripped == (stream.len() as int > 4));
}

} // verus!
