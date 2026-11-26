//! Proofs for the capture specification.
//!
//! The obligations are the four properties the plan names. Bound and prefix are
//! one induction over the chunk count ([`lemma_captured_prefix`]); the trip
//! characterization is one arithmetic step of it ([`lemma_trip_step`]);
//! monotonicity is a second induction from the first tripped point
//! ([`lemma_captured_stable`]). Nothing here is assumed: every lemma is proved,
//! and the executable kernel's per-call contract is checked by the verifier
//! against [`step`] directly.
//!
//! The load-bearing sequence fact is [`lemma_prefix_concat`]: a prefix of a
//! concatenation either lies inside the first sequence or continues into the
//! second. Together with [`lemma_prefix_extension`] it turns "the kept bytes are
//! the stream's prefix" into a step that appends exactly this chunk's kept bytes.

use vstd::prelude::*;

verus! {

use super::spec::*;

broadcast use vstd::seq::group_seq_lemmas;

/// The empty sequence is a right identity for concatenation.
pub proof fn lemma_add_empty(s: Seq<u8>)
    ensures s + Seq::<u8>::empty() == s
{
    assert(s + Seq::<u8>::empty() =~= s);
}

/// The empty sequence is a left identity for concatenation.
pub proof fn lemma_empty_add(s: Seq<u8>)
    ensures Seq::<u8>::empty() + s == s
{
    assert(Seq::<u8>::empty() + s =~= s);
}

/// A sequence is its own full prefix.
pub proof fn lemma_prefix_full(s: Seq<u8>)
    ensures prefix(s, s.len() as int) == s
{
    assert(prefix(s, s.len() as int) =~= s);
}

/// The zero-length prefix is empty.
pub proof fn lemma_prefix_zero(s: Seq<u8>)
    ensures prefix(s, 0) == Seq::<u8>::empty()
{
    assert(prefix(s, 0) =~= Seq::<u8>::empty());
}

/// A prefix of a concatenation splits at the first sequence's end.
///
/// If `n` fits inside `s`, the prefix is `s`'s own prefix. Otherwise it is all of
/// `s` followed by the part of `c` that reaches `n`.
pub proof fn lemma_prefix_concat(s: Seq<u8>, c: Seq<u8>, n: int)
    requires
        0 <= n <= s.len() + c.len(),
    ensures
        prefix(s + c, n) == if n <= s.len() {
            prefix(s, n)
        } else {
            s + prefix(c, n - s.len() as int)
        },
{
    if n <= s.len() {
        let left = prefix(s + c, n);
        let right = prefix(s, n);
        assert(left.len() == n);
        assert(right.len() == n);
        assert forall|i: int| 0 <= i < n implies left[i] == right[i] by {
            assert(left[i] == (s + c)[i]);
            assert((s + c)[i] == s[i]);
            assert(right[i] == s[i]);
        };
        assert(left =~= right);
    } else {
        let left = prefix(s + c, n);
        let right = s + prefix(c, n - s.len() as int);
        assert(left.len() == n);
        assert(right.len() == n);
        assert forall|i: int| 0 <= i < n implies left[i] == right[i] by {
            assert(left[i] == (s + c)[i]);
            if i < s.len() {
                assert((s + c)[i] == s[i]);
                assert(right[i] == s[i]);
            } else {
                assert((s + c)[i] == c[i - s.len() as int]);
                assert(right[i] == prefix(c, n - s.len() as int)[i - s.len() as int]);
                assert(
                    prefix(c, n - s.len() as int)[i - s.len() as int] == c[i - s.len() as int]
                );
            }
        };
        assert(left =~= right);
    }
}

/// One chunk extends the kept prefix by exactly the bytes that fit below the cap.
///
/// The premise `s.len() <= cap` is the state invariant: the previous prefix is
/// never longer than the cap. The conclusion is the induction step for the prefix
/// property, and it also states that `keep_count` is a valid subrange length.
pub proof fn lemma_prefix_extension(cap: nat, s: Seq<u8>, c: Seq<u8>)
    requires
        s.len() <= cap,
    ensures
        ({
            let kept = keep_count(cap, s, c);
            let combined = s + c;
            0 <= kept <= c.len() as int && prefix(combined, prefix_len(cap, combined))
                == prefix(s, prefix_len(cap, s)) + prefix(c, kept)
        }),
{
    let total = s.len() as int + c.len() as int;
    let room = room(cap, s);
    assert(room == cap as int - s.len() as int);
    assert(room >= 0);
    if total <= cap as int {
        assert(prefix_len(cap, s) == s.len() as int);
        assert(prefix_len(cap, s + c) == total);
        assert(keep_count(cap, s, c) == c.len() as int);
        lemma_prefix_full(s);
        lemma_prefix_full(c);
        lemma_prefix_full(s + c);
    } else {
        assert(prefix_len(cap, s + c) == cap as int);
        assert(keep_count(cap, s, c) == room);
        assert(prefix_len(cap, s) == s.len() as int);
        lemma_prefix_concat(s, c, cap as int);
        lemma_prefix_full(s);
        if cap as int <= s.len() as int {
            assert(cap as int == s.len() as int);
            lemma_prefix_zero(c);
            lemma_add_empty(s);
        }
    }
}

/// One chunk extends the kept prefix, given that the prefix matches the stream.
///
/// This is the induction step for the prefix property in the form the
/// induction needs: the old kept bytes are the stream's prefix, and the new kept
/// bytes are the new stream's prefix. The two cases are the stream still being
/// below the cap, where the chunk continues the same prefix, and the stream
/// already being at or over it, where the prefix is full and the chunk adds
/// nothing.
pub proof fn lemma_prefix_step(cap: nat, s: Seq<u8>, kept: Seq<u8>, c: Seq<u8>)
    requires
        kept.len() <= cap,
        kept == prefix(s, prefix_len(cap, s)),
    ensures
        ({
            let combined = s + c;
            kept + prefix(c, keep_count(cap, kept, c)) == prefix(combined, prefix_len(cap, combined))
        }),
{
    if (s.len() as int) < (cap as int) {
        assert(prefix_len(cap, s) == s.len() as int);
        lemma_prefix_full(s);
        lemma_prefix_extension(cap, s, c);
    } else {
        assert(prefix_len(cap, s) == cap as int);
        assert(kept.len() == cap);
        assert(keep_count(cap, kept, c) == 0);
        lemma_prefix_zero(c);
        lemma_add_empty(kept);
        lemma_prefix_concat(s, c, cap as int);
        assert(prefix(s + c, prefix_len(cap, s + c)) == prefix(s, cap as int));
    }
}

/// The trip flag after one chunk is exactly "the stream now exceeds the cap".
///
/// `kept` is the state's kept prefix and `s` the stream it belongs to, so
/// `tripped` must already equal "the stream is longer than the cap". A stream
/// that lands exactly on the cap does not trip, which is why the comparison is
/// strict.
pub proof fn lemma_trip_step(
    cap: nat,
    s: Seq<u8>,
    kept: Seq<u8>,
    c: Seq<u8>,
    tripped: bool,
)
    requires
        kept.len() <= cap,
        kept == prefix(s, prefix_len(cap, s)),
        tripped == (s.len() as int > cap as int),
    ensures
        step(cap, (kept, tripped), c).1 == ((s + c).len() as int > cap as int),
{
    assert((s + c).len() as int == s.len() as int + c.len() as int);
    if s.len() as int > cap as int {
        assert(tripped);
        assert(step(cap, (kept, tripped), c).1);
    } else {
        assert(!tripped);
        assert(prefix_len(cap, s) == s.len() as int);
        lemma_prefix_full(s);
        assert(kept == s);
        assert(step(cap, (kept, tripped), c).1 == (c.len() as int > room(cap, s)));
    }
}

/// At the cap, a further chunk changes nothing and the state stays tripped.
pub proof fn lemma_keep_count_at_cap(cap: nat, kept: Seq<u8>, c: Seq<u8>)
    requires
        kept.len() == cap,
    ensures
        keep_count(cap, kept, c) == 0,
        step(cap, (kept, true), c).0 == kept,
        step(cap, (kept, true), c).1,
{
    assert(room(cap, kept) == 0);
    lemma_prefix_zero(c);
    lemma_add_empty(kept);
}

/// Bound, prefix, and trip hold after every prefix of the chunk sequence.
pub proof fn lemma_captured_prefix(cap: nat, chunks: Seq<Seq<u8>>, index: nat)
    requires
        index <= chunks.len(),
    ensures
        captured_upto(cap, chunks, index),
    decreases index,
{
    if index == 0 {
        lemma_prefix_zero(Seq::<u8>::empty());
    } else {
        lemma_captured_prefix(cap, chunks, (index - 1) as nat);
        let c = chunks[(index - 1) as int];
        let s = stream_at(chunks, (index - 1) as nat);
        let prior = fold_at(cap, chunks, (index - 1) as nat);
        lemma_prefix_step(cap, s, prior.0, c);
        lemma_trip_step(cap, s, prior.0, c, prior.1);
    }
}

/// Once tripped, the state at `to` is the state at `at` for every later `to`.
pub proof fn lemma_stable_from(
    cap: nat,
    chunks: Seq<Seq<u8>>,
    at: nat,
    to: nat,
    kept: Seq<u8>,
)
    requires
        at <= to <= chunks.len(),
        kept.len() == cap,
        kept == fold_at(cap, chunks, at).0,
        fold_at(cap, chunks, at).1,
    ensures
        fold_at(cap, chunks, to).0 == kept,
        fold_at(cap, chunks, to).1,
    decreases (to - at) as nat,
{
    if at < to {
        let c = chunks[at as int];
        lemma_keep_count_at_cap(cap, kept, c);
        assert(fold_at(cap, chunks, (at + 1) as nat) == step(cap, (kept, true), c));
        lemma_stable_from(cap, chunks, (at + 1) as nat, to, kept);
    }
}

/// Monotonicity: a tripped state keeps both its bytes and its flag.
pub proof fn lemma_captured_stable(cap: nat, chunks: Seq<Seq<u8>>, at: nat, to: nat)
    requires
        at <= to <= chunks.len(),
        captured_upto(cap, chunks, at),
        fold_at(cap, chunks, at).1,
    ensures
        fold_at(cap, chunks, to).0 == fold_at(cap, chunks, at).0,
        fold_at(cap, chunks, to).1,
{
    let s = stream_at(chunks, at);
    assert(s.len() as int > cap as int);
    assert(prefix_len(cap, s) == cap as int);
    assert(fold_at(cap, chunks, at).0.len() == cap);
    lemma_stable_from(cap, chunks, at, to, fold_at(cap, chunks, at).0);
}

/// The whole-stream specification: bound, prefix, and trip after every chunk.
pub proof fn lemma_captured(cap: nat, chunks: Seq<Seq<u8>>)
    ensures
        captured(cap, chunks),
{
    lemma_captured_prefix(cap, chunks, chunks.len());
}

} // verus!
