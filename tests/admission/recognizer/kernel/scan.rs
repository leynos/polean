//! Byte-level predicates shared by the recognizer kernel's scan steps.
//!
//! These are the parts of the kernel that read one byte or a short byte
//! pattern: the bound-checked byte read, the metadata-marker search, the line
//! ending and comment-body tests, and the token match. The scanning loops and
//! the gap and token invariants stay in the parent module.
//!
//! This module is compiled twice, as test code by Cargo and for the proof by
//! Verus, exactly as its parent is.

use vstd::prelude::*;

// The specification and the lemmas are erased by plain `rustc`, so these
// imports exist only for the verifier and would otherwise be unused.
#[cfg(verus_keep_ghost)]
use super::lemmas::*;
#[cfg(verus_keep_ghost)]
use super::spec::*;

verus! {

broadcast use vstd::seq::group_seq_axioms;

/// The byte at `i`, which must be inside `src`.
pub fn byte_at(src: &[u8], i: usize) -> (b: u8)
    requires
        i < src@.len(),
    ensures
        b == src@[i as int],
{
    *vstd::slice::slice_index_get(src, i)
}

/// The metadata marker's byte at `j`.
pub(super) const fn metadata_byte(j: usize) -> (b: u8)
    requires
        j < 8,
    ensures
        b == metadata_bytes()[j as int],
{
    // One arm per spelling; the shared letters are grouped.
    match j {
        0 => b'M',
        1 => b'E',
        2 | 6 => b'T',
        4 => b'D',
        _ => b'A',
    }
}

/// Whether the metadata marker sits at `at`.
pub(super) fn metadata_matches_at(src: &[u8], at: usize) -> (r: bool)
    requires
        at + 8 <= src@.len(),
    ensures
        r == metadata_at(src@, at as int),
{
    proof {
        lemma_slice_len_fits(src);
    }
    let mut j: usize = 0;
    while j < 8
        invariant
            j <= 8,
            at + 8 <= src@.len(),
            forall|k: int| 0 <= k < j as int ==> #[trigger] src@[at as int + k] == metadata_bytes()[k],
        decreases 8 - j,
    {
        proof {
            lemma_slice_len_fits(src);
            assert((at as int + j as int) < src@.len());
        }
        if byte_at(src, at + j) != metadata_byte(j) {
            assert(!metadata_at(src@, at as int)) by {
                assert(src@[at as int + j as int] != metadata_bytes()[j as int]);
            }
            return false;
        }
        j += 1;
    }
    assert(metadata_at(src@, at as int)) by {
        assert(metadata_bytes().len() == 8);
    }
    true
}

/// Whether `from..to` holds the metadata marker.
pub(super) fn contains_metadata(src: &[u8], from: usize, to: usize) -> (r: bool)
    requires
        from <= to <= src@.len(),
    ensures
        r == metadata_in(src@, from as int, to as int),
{
    proof {
        lemma_slice_len_fits(src);
    }
    let mut at = from;
    // A countdown avoids both `at + 8` and `to - at` in the loop condition:
    // either could overflow or underflow when the offsets are near the end of
    // the integer range.
    let mut remaining = to - from;
    while remaining >= 8
        invariant
            from <= at <= to,
            to <= src@.len(),
            at as int + remaining as int == to as int,
            forall|i: int| from as int <= i < at as int ==> #[trigger] metadata_at(src@, i) == false,
        decreases remaining,
    {
        proof {
            assert((at as int + 8) <= src@.len());
        }
        if metadata_matches_at(src, at) {
            assert(metadata_in(src@, from as int, to as int)) by {
                assert(metadata_at(src@, at as int));
            }
            return true;
        }
        at += 1;
        remaining -= 1;
    }
    assert(!metadata_in(src@, from as int, to as int)) by {
        assert(metadata_bytes().len() == 8);
        assert forall|i: int| from as int <= i && #[trigger] metadata_at(src@, i) implies i + 8
            > to as int by
        {
            if i + 8 <= to as int {
                assert(i < at as int);
                assert(metadata_at(src@, i) == false);
            }
        };
    }
    false
}

/// Whether the byte at `at` starts a line ending.
pub(super) fn is_line_end(src: &[u8], at: usize) -> (r: bool)
    requires
        at < src@.len(),
    ensures
        r == line_end_at(src@, at as int),
{
    proof {
        lemma_slice_len_fits(src);
    }
    if byte_at(src, at) == b'\n' {
        true
    } else if byte_at(src, at) != b'\r' {
        false
    } else if at + 1 < src.len() {
        byte_at(src, at + 1) == b'\n'
    } else {
        false
    }
}

/// Whether a byte may appear in a comment body.
pub const fn is_body_byte(byte: u8) -> (r: bool)
    ensures
        r == body_byte(byte),
{
    matches!(byte, b'\t' | 0x20..=0x7e)
}

/// Whether `src` holds `pat` starting at `at`.
pub fn matches_at(src: &[u8], at: usize, pat: &[u8]) -> (r: bool)
    ensures
        r == token_at(src@, at as int, pat),
{
    let Some(suffix) = src.get(at..) else {
        return false;
    };
    if suffix.len() < pat.len() {
        return false;
    }
    let mut j: usize = 0;
    while j < pat.len()
        invariant
            j <= pat@.len(),
            suffix@.len() == src@.len() - at as int,
            (forall|k: int| 0 <= k < suffix@.len() ==> src@[at as int + k] == suffix@[k]),
            (forall|k: int| 0 <= k < j as int ==> suffix@[k] == pat@[k]),
        decreases pat@.len() - j as int,
    {
        let (Some(actual), Some(expected)) = (suffix.get(j), pat.get(j)) else {
            return false;
        };
        if actual != expected {
            assert(!(forall|k: int| 0 <= k < pat@.len() ==> src@[at as int + k] == pat@[k])) by {
                assert(src@[at as int + j as int] == suffix@[j as int]);
                assert(suffix@[j as int] == *actual);
                assert(pat@[j as int] == *expected);
            }
            return false;
        }
        j += 1;
    }
    true
}

} // verus!
