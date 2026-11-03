//! The recognizer kernel: a bounded check that a source is a canonical token
//! sequence separated by permitted trivia.
//!
//! The kernel sees bytes and tokens only. It has no access to the rule list,
//! the profile, Regorus, or the decoded AST, which is what lets EP-M4 verify it
//! once for inputs of any length (O6). Everything that depends on the module's
//! shape is rendered into the token list beforehand.
//!
//! # Contract
//!
//! ```text
//! recognize(src, toks, needs_newline)
//!   requires every token is non-empty and `needs_newline` gives one flag per
//!            token
//!   ensures  Ok(offsets) gives one start offset per token, each token's exact
//!            bytes sit at its offset, the gaps between consecutive tokens are
//!            non-empty permitted trivia, the gap before a token whose flag is
//!            set contains a line feed, and the bytes after the last token are
//!            permitted trivia;
//!            Err(offset) names where recognition stopped, and that offset is
//!            never past the end of the source.
//! ```
//!
//! Permitted trivia is a space, a tab, a line feed, a carriage return that
//! immediately precedes a line feed, or a comment. A comment starts with `#`,
//! runs to the end of its line, and its body may hold only tabs and bytes
//! `0x20`–`0x7E`. A comment body containing `METADATA` is refused outright
//! (D9), and a token may not begin on a comment's line (D10).
//!
//! # Verification
//!
//! The specification lives in `verus/admission_spec.rs` and the proofs that
//! lift each scan step to it in `verus/admission_lemmas.rs`. `verus/
//! admission_recognizer.rs` pulls this module into the Verus entry point with
//! `#[path]`, so this file is the same source Cargo compiles as test code and
//! Verus verifies. The proof establishes kernel soundness only: it says an
//! `Ok` decomposes the source as the tokens say, not that Rego assigns those
//! tokens the same meaning.

#[path = "../../../../verus/admission_spec.rs"]
mod spec;

#[path = "../../../../verus/admission_spec_completeness.rs"]
mod completeness;

#[path = "../../../../verus/admission_lemmas.rs"]
mod lemmas;

mod baseline;
pub mod scan;
mod trivia;

use vstd::prelude::*;

pub use self::baseline::baseline_is_recognised;
// The specification and the lemmas are erased by plain `rustc`, so these
// imports exist only for the verifier and would otherwise be unused.
#[cfg(verus_keep_ghost)]
use self::completeness::*;
#[cfg(verus_keep_ghost)]
use self::lemmas::*;
#[cfg(verus_keep_ghost)]
use self::spec::*;
use self::{scan::*, trivia::*};

/// The bytes a comment may not spell.
pub const METADATA: &[u8] = b"METADATA";

verus! {

broadcast use vstd::seq::group_seq_axioms;

/// Where a gap of trivia ends and whether it contains a line feed.
pub struct Gap {
    /// The offset at which the gap ends, which is the next token's start.
    pub end: usize,
    /// Whether the gap holds a line feed.
    pub newline: bool,
}

/// Extend a pointwise agreement by one matching element.
///
/// The recognizer's loop appends one offset per token. Stating the extension as
/// a lemma keeps the quantified obligation out of the loop body, which is where
/// the verifier's budget is tightest.
pub proof fn lemma_offsets_agree_snoc(a: Seq<usize>, b: Seq<usize>, j: int, v: usize)
    requires
        0 <= j,
        a.len() == j,
        j < b.len(),
        v == b[j],
        offsets_agree(a, b, j),
    ensures
        offsets_agree(a.push(v), b, j + 1),
{
    assert forall|i: int| 0 <= i < j + 1 implies #[trigger] a.push(v)[i] == b[i] by {
        if i < j {
            assert(a.push(v)[i] == a[i]);
        } else {
            assert(a.push(v)[i] == v);
        }
    }
}


/// Recognize `toks` in `src`, returning each token's start offset.
///
/// # Errors
///
/// Returns the offset at which recognition stopped.
///
/// The loop is the proof. Each `proof` block inside it discharges one clause of
/// the contract for the branch that follows, and the invariants name exactly
/// the state the next iteration may assume, so extracting them would separate
/// the run from the facts that justify it. The function's length is that
/// structure rather than incidental code.
#[expect(
    clippy::too_many_lines,
    reason = "the verified loop and its proof blocks are the unit of assurance"
)]
#[verifier::rlimit(120)]
pub fn recognize(src: &[u8], toks: &[&[u8]], needs_newline: &[bool]) -> (r: Result<Vec<usize>, usize>)
    requires
        forall|j: int| 0 <= j < toks@.len() ==> #[trigger] toks@[j]@.len() > 0,
        needs_newline@.len() == toks@.len(),
    ensures
        r is Ok ==> {
            &&& r->Ok_0@.len() == toks@.len()
            &&& decomposes_at(src@, toks@, r->Ok_0@)
            &&& flags_hold(src@, toks@, r->Ok_0@, needs_newline@)
            &&& admits_canonical(src@, toks@, needs_newline@) ==> ws_canonical(
                src@,
                toks@,
                needs_newline@,
                r->Ok_0@,
            )
        },
        r is Err ==> {
            &&& r->Err_0 <= src@.len()
            &&& admits_canonical(src@, toks@, needs_newline@) ==> false
        },
{
    proof {
        lemma_slice_len_fits(src);
        reveal(flags_prefix);
    }
    // The completeness argument needs one fixed decomposition to reason about.
    // The condition and the witness are written as the specification's own
    // predicates so the loop can prove the postcondition verbatim.
    let ghost witness = if admits_canonical(src@, toks@, needs_newline@) {
        choose|q: Seq<usize>| ws_canonical(src@, toks@, needs_newline@, q)
    } else {
        Seq::<usize>::empty()
    };
    let mut offsets: Vec<usize> = Vec::new();
    let mut at = 0;
    let mut index = 0;
    while index < toks.len()
        invariant
            index <= toks@.len(),
            offsets@.len() == index,
            at as int == prev_end(src@, toks@, offsets@, index as int),
            at <= src@.len(),
            src@.len() <= usize::MAX as int,
            needs_newline@.len() == toks@.len(),
            prefix_decomposes(src@, toks@, offsets@, index as int),
            flags_prefix(src@, toks@, offsets@, needs_newline@, index as int),
            admits_canonical(src@, toks@, needs_newline@) ==> ws_canonical(src@, toks@, needs_newline@, witness),
            admits_canonical(src@, toks@, needs_newline@) ==> offsets_agree(offsets@, witness, index as int),
        decreases toks@.len() - index,
    {
        let ghost prior: Seq<usize> = offsets@;
        let token = *vstd::slice::slice_index_get(toks, index);
        if token.is_empty() {
            proof {
                if admits_canonical(src@, toks@, needs_newline@) {
                    lemma_ws_canonical_token_len(
                        src@,
                        toks@,
                        needs_newline@,
                        witness,
                        index as int,
                    );
                    assert(token@ == toks@[index as int]@);
                    assert(token@.len() == 0);
                    assert(false);
                }
            }
            proof {
                assert(!admits_canonical(src@, toks@, needs_newline@));
                assert(at as int <= src@.len());
            }
            return Err(at);
        }
        let gap = match skip_trivia(src, at, false) {
            Ok(gap) => gap,
            Err(stop) => {
                proof {
                    if admits_canonical(src@, toks@, needs_newline@) {
                        lemma_ws_canonical_no_hash(src@, toks@, needs_newline@, witness);
                        // `skip_trivia` refuses exactly when the source holds
                        // no comment marker, which a canonical decomposition
                        // rules out.
                        assert(false);
                    }
                }
                return Err(stop);
            },
        };
        proof {
            if admits_canonical(src@, toks@, needs_newline@) {
                lemma_ws_canonical_no_hash(src@, toks@, needs_newline@, witness);
                lemma_ws_canonical_lengths(src@, toks@, needs_newline@, witness);
                assert(gap.end as int == ws_extent(src@, at as int));
                lemma_prev_end_agree(src@, toks@, offsets@, witness, index as int);
                assert(at as int == prev_end(src@, toks@, witness, index as int));
                lemma_ws_canonical_gap(src@, toks@, needs_newline@, witness, index as int);
                assert(gap.end as int == witness[index as int] as int);
            }
        }
        if index > 0 && gap.end == at {
            proof {
                if admits_canonical(src@, toks@, needs_newline@) {
                    lemma_ws_canonical_gap_positive(
                        src@,
                        toks@,
                        needs_newline@,
                        witness,
                        index as int,
                    );
                    assert(at as int == prev_end(src@, toks@, witness, index as int));
                    assert(witness[index as int - 1] as int + toks@[index as int - 1]@.len()
                        < witness[index as int] as int);
                    assert((at as int) < (gap.end as int));
                    assert(false);
                }
            }
            proof {
                assert(!admits_canonical(src@, toks@, needs_newline@));
                assert(at as int <= src@.len());
            }
            return Err(at);
        }
        proof {
            assert((index as int) < toks@.len());
        }
        let flag = *vstd::slice::slice_index_get(needs_newline, index);
        if flag && !gap.newline {
            proof {
                if admits_canonical(src@, toks@, needs_newline@) {
                    reveal(flags_prefix);
                    lemma_ws_canonical_flags(src@, toks@, needs_newline@, witness);
                    assert(flag == needs_newline@[index as int]);
                    assert(flags_hold(src@, toks@, witness, needs_newline@));
                    assert(flags_prefix(
                        src@,
                        toks@,
                        witness,
                        needs_newline@,
                        toks@.len() as int,
                    ));
                    assert(gap_has_newline(
                        src@,
                        prev_end(src@, toks@, witness, index as int),
                        witness[index as int] as int,
                    ));
                    assert(gap.newline == gap_has_newline(src@, at as int, gap.end as int));
                    assert(false);
                }
            }
            return Err(gap.end);
        }
        if flag {
            proof {
                assert(gap_has_newline(src@, at as int, gap.end as int));
            }
        }
        if !matches_at(src, gap.end, token) {
            proof {
                if admits_canonical(src@, toks@, needs_newline@) {
                    lemma_ws_canonical_token_at(
                        src@,
                        toks@,
                        needs_newline@,
                        witness,
                        index as int,
                    );
                    assert(token@ == toks@[index as int]@);
                    assert(token_at(src@, witness[index as int] as int, toks@[index as int]));
                    assert(gap.end as int == witness[index as int] as int);
                    assert(false);
                }
            }
            return Err(gap.end);
        }
        proof {
            assert((gap.end as int + token@.len()) <= src@.len());
        }
        offsets.push(gap.end);
        proof {
            lemma_flags_prefix_push(src@, toks@, prior, needs_newline@, index as int, gap.end);
            if admits_canonical(src@, toks@, needs_newline@) {
                assert(gap.end as int == witness[index as int] as int);
                lemma_offsets_agree_snoc(prior, witness, index as int, gap.end);
            }
        }
        at = gap.end + token.len();
        index += 1;
    }
    let trailing = match skip_trivia(src, at, true) {
        Ok(trailing) => trailing,
        Err(stop) => {
            proof {
                if admits_canonical(src@, toks@, needs_newline@) {
                    lemma_ws_canonical_no_hash(src@, toks@, needs_newline@, witness);
                    assert(false);
                }
            }
            return Err(stop);
        },
    };
    if trailing.end == src.len() {
        proof {
            if admits_canonical(src@, toks@, needs_newline@) {
                assert(offsets_agree(offsets@, witness, toks@.len() as int));
                lemma_ws_canonical_lengths(src@, toks@, needs_newline@, witness);
                assert(offsets@.len() == toks@.len());
                assert(witness.len() == toks@.len());
                assert(offsets@ =~= witness);
                assert(at as int == prev_end(src@, toks@, witness, toks@.len() as int));
                lemma_ws_canonical_no_hash(src@, toks@, needs_newline@, witness);
                lemma_ws_canonical_trailing(src@, toks@, needs_newline@, witness);
                assert(ws_extent(src@, at as int) == src@.len() as int);
                assert(trailing.end as int == ws_extent(src@, at as int));
                assert(ws_canonical(src@, toks@, needs_newline@, offsets@));
            }
        }
        Ok(offsets)
    } else {
        proof {
            if admits_canonical(src@, toks@, needs_newline@) {
                assert(offsets_agree(offsets@, witness, toks@.len() as int));
                lemma_ws_canonical_lengths(src@, toks@, needs_newline@, witness);
                assert(offsets@.len() == toks@.len());
                assert(witness.len() == toks@.len());
                assert(offsets@ =~= witness);
                lemma_ws_canonical_no_hash(src@, toks@, needs_newline@, witness);
                lemma_ws_canonical_trailing(src@, toks@, needs_newline@, witness);
                assert(ws_extent(src@, at as int) == src@.len() as int);
                assert(trailing.end as int == ws_extent(src@, at as int));
                assert(false);
            }
        }
        Err(trailing.end)
    }
}


} // verus!

/// Whether `body` spells `METADATA` anywhere.
///
/// Both the kernel and the witness use this one definition, so a comment the
/// witness refuses is a comment the kernel refuses.
pub fn has_metadata(body: &[u8]) -> bool {
    body.windows(METADATA.len())
        .any(|window| window == METADATA)
}

/// The body of the comment that starts at `hash`, or the empty slice.
///
/// The classifier in [`super`] uses the same rule the kernel applies, so a
/// refusal at a `#` offset and the reason code it reports cannot disagree.
///
/// This helper is outside the `verus!` block: it classifies a refusal that has
/// already happened, so it carries no admission contract.
pub fn comment_body(src: &[u8], hash: usize) -> &[u8] {
    let start = hash.saturating_add(1);
    let mut end = start;
    while let Some(&byte) = src.get(end) {
        if is_line_end_unchecked(src, end) || !is_body_byte_unchecked(byte) {
            break;
        }
        end += 1;
    }
    src.get(start..end).unwrap_or_default()
}

/// Whether the byte at `at` starts a line ending, for the classifier.
fn is_line_end_unchecked(src: &[u8], at: usize) -> bool {
    match src.get(at) {
        Some(&b'\n') => true,
        Some(&b'\r') => src.get(at.saturating_add(1)) == Some(&b'\n'),
        Some(_) | None => false,
    }
}

/// Whether a byte may appear in a comment body, for the classifier.
const fn is_body_byte_unchecked(byte: u8) -> bool { matches!(byte, b'\t' | 0x20..=0x7e) }
