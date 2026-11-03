//! The verified executable instance: a run of the recognizer over the concrete
//! baseline.
//!
//! The kernel's contract is one-way, so an implementation that always refused
//! would satisfy it. This instance supplies the other direction: given the
//! baseline's own bytes, tokens, and flags, `recognize` returns exactly the
//! baseline's offsets.
//!
//! This module is compiled twice, as test code by Cargo and for the proof by
//! Verus, exactly as its parent is.

use vstd::prelude::*;

// The specification and the lemmas are erased by plain `rustc`, so these
// imports exist only for the verifier and would otherwise be unused.
#[cfg(verus_keep_ghost)]
use super::completeness::*;
#[cfg(verus_keep_ghost)]
use super::lemmas::*;
use super::recognize;
#[cfg(verus_keep_ghost)]
use super::spec::*;

verus! {

/// A verified run of the recognizer over the concrete baseline.
///
/// The caller supplies the bytes the executable test beside the kernel checks;
/// the postcondition fixes the exact offsets, so no implementation that
/// answered with any other list could satisfy it. The proof needs the inputs'
/// views, which a byte-literal slice does not expose on its own.
///
/// The premise [`admits_canonical_t`] is established by
/// `lemma_baseline_admits_canonical`, and transferred to these slices by
/// `lemma_ws_canonical_t_to_slices`.
pub fn baseline_is_recognised(
    src: &[u8],
    toks: &[&[u8]],
    needs_newline: &[bool],
) -> (r: Result<Vec<usize>, usize>)
    requires
        src@ =~= baseline_src(),
        toks@.len() == baseline_tokens().len(),
        forall|j: int| 0 <= j < toks@.len() ==> #[trigger] toks@[j]@ =~= baseline_tokens()[j],
        needs_newline@ =~= baseline_flags(),
    ensures
        r is Ok,
        r->Ok_0@ == baseline_offsets(),
{
    proof {
        lemma_baseline_is_canonical();
        lemma_ws_canonical_t_to_slices(
            src@,
            toks@,
            baseline_tokens(),
            needs_newline@,
            baseline_offsets(),
        );
        assert(ws_canonical(src@, toks@, needs_newline@, baseline_offsets()));
        assert(admits_canonical(src@, toks@, needs_newline@));
    }
    match recognize(src, toks, needs_newline) {
        Ok(offsets) => {
            proof {
                lemma_ws_canonical_unique(
                    src@,
                    toks@,
                    needs_newline@,
                    offsets@,
                    baseline_offsets(),
                );
            }
            Ok(offsets)
        }
        Err(stop) => {
            proof {
                assert(false);
            }
            Err(stop)
        }
    }
}

} // verus!
