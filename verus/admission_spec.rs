//! The admission recognizer's specification, independent of the kernel.
//!
//! A source is a byte sequence. The kernel's postcondition says that an `Ok`
//! offset list decomposes the source into the rendered tokens, with permitted
//! trivia in every gap, and that the flag list's line-feed demands hold.
//!
//! The specification is written over `Seq<u8>` so the proof is about any
//! length. It is the same relation the witness checks directly, so O4's oracle
//! and O6's theorem state one property in two ways.

use vstd::prelude::*;

verus! {

/// The bytes a comment body may not spell.
///
/// `open` so that proofs can see the sequence's length, which is a constant
/// eight.
pub open spec fn metadata_bytes() -> Seq<u8> {
    seq![b'M', b'E', b'T', b'A', b'D', b'A', b'T', b'A']
}

/// Whether byte `i` is whitespace permitted between tokens.
///
/// A carriage return counts only as the first half of a line ending.
pub open spec fn ws_at(s: Seq<u8>, i: int) -> bool {
    0 <= i < s.len()
        && (s[i] == b' ' || s[i] == b'\t' || s[i] == b'\n'
            || (s[i] == b'\r' && i + 1 < s.len() && s[i + 1] == b'\n'))
}

/// Whether `b` may appear in a comment body.
pub open spec fn body_byte(b: u8) -> bool { b == b'\t' || (0x20 <= b && b <= 0x7e) }

/// Whether `i` starts a line ending.
pub open spec fn line_end_at(s: Seq<u8>, i: int) -> bool {
    0 <= i < s.len()
        && (s[i] == b'\n' || (s[i] == b'\r' && i + 1 < s.len() && s[i + 1] == b'\n'))
}

/// The offset at which the comment body that starts at `i` stops.
///
/// The scan stops at the first line ending, the first byte that cannot be in a
/// body, or the end of the source, so the result never precedes `i`.
pub open spec fn comment_end_from(s: Seq<u8>, i: int) -> int
    decreases s.len() - i
{
    if i >= s.len() || line_end_at(s, i) || !body_byte(s[i]) {
        i
    } else {
        comment_end_from(s, i + 1)
    }
}

/// The offset at which the comment that starts at `hash` stops.
pub open spec fn comment_end(s: Seq<u8>, hash: int) -> int {
    comment_end_from(s, hash + 1)
}

/// Whether the metadata marker sits at `i`.
pub open spec fn metadata_at(s: Seq<u8>, i: int) -> bool {
    i + metadata_bytes().len() <= s.len() && (forall|k: int|
        0 <= k < metadata_bytes().len() ==> #[trigger] s[i + k] == metadata_bytes()[k])
}

/// Whether `from..to` holds the metadata marker.
pub open spec fn metadata_in(s: Seq<u8>, from: int, to: int) -> bool {
    exists|i: int| from <= i && #[trigger] metadata_at(s, i) && i + metadata_bytes().len() <= to
}

/// Whether the comment that starts at `hash` is well formed and silent.
///
/// A well-formed comment ends at a line ending or the end of the source, and its
/// body spells no metadata marker (D9).
pub open spec fn comment_ok(s: Seq<u8>, hash: int) -> bool {
    hash < s.len() && s[hash] == b'#'
        && (comment_end(s, hash) == s.len() || line_end_at(s, comment_end(s, hash)))
        && !metadata_in(s, hash + 1, comment_end(s, hash))
}

/// Whether `a..b` is a sequence of permitted trivia.
///
/// The recursion is on the gap's start, so the last-trivia rule is stated
/// separately: [`trivia`] adds it. Ending a gap just after a comment is refused
/// there, because a token on a comment's line belongs to the comment (D10).
pub open spec fn trivia_core(s: Seq<u8>, a: int, b: int) -> bool
    decreases b - a
{
    if a >= b {
        a == b
    } else if ws_at(s, a) {
        trivia_core(s, a + 1, b)
    } else if s[a] == b'#' && comment_ok(s, a) && a < comment_end(s, a) && comment_end(s, a)
        <= b
    {
        trivia_core(s, comment_end(s, a), b)
    } else {
        false
    }
}

/// Whether the last item of the trivia gap `a..b` is a comment.
pub open spec fn ends_in_comment(s: Seq<u8>, a: int, b: int) -> bool
    decreases b - a
{
    if a >= b {
        false
    } else if ws_at(s, a) {
        ends_in_comment(s, a + 1, b)
    } else if s[a] == b'#' && comment_ok(s, a) && a < comment_end(s, a) && comment_end(s, a) <= b {
        comment_end(s, a) == b || ends_in_comment(s, comment_end(s, a), b)
    } else {
        false
    }
}

/// Whether `a..b` is a complete gap of permitted trivia.
///
/// `last` says no token follows. Unless it holds, the gap may not end with a
/// comment, so a token can never begin on a comment's line (D10).
///
/// The body is opaque: the recognizer's invariant mentions this predicate once
/// per token, and unfolding it in every proof obligation is the dominant cost
/// of verifying the scan. Lemmas that need the definition reveal it.
#[verifier::opaque]
pub open spec fn trivia(s: Seq<u8>, a: int, b: int, last: bool) -> bool {
    trivia_core(s, a, b) && (last || !ends_in_comment(s, a, b))
}

/// Whether `tok` sits at `at`.
pub open spec fn token_at(s: Seq<u8>, at: int, tok: &[u8]) -> bool {
    at + tok@.len() <= s.len() && (forall|k: int| 0 <= k < tok@.len() ==> s[at + k] == tok@[k])
}

/// Whether the first `m` tokens sit at their offsets with trivia in the gaps.
pub open spec fn prefix_decomposes(s: Seq<u8>, k: Seq<&[u8]>, p: Seq<usize>, m: int) -> bool {
    &&& m <= k.len()
    &&& p.len() == m
    &&& (forall|j: int| 0 <= j < m ==> #[trigger] k[j]@.len() > 0)
    &&& (forall|j: int| 0 <= j < m ==> #[trigger] token_at(s, p[j] as int, k[j]))
    &&& if m == 0 {
        true
    } else {
        &&& trivia(s, 0, p[0] as int, false)
        &&& (forall|j: int|
            0 < j < m ==> p[j - 1] as int + k[j - 1]@.len() < #[trigger] p[j] as int && trivia(
                s,
                p[j - 1] as int + k[j - 1]@.len(),
                p[j] as int,
                false,
            ))
    }
}

/// Whether `k` sits at `p` with trivia filling the whole source elsewhere.
pub open spec fn decomposes_at(s: Seq<u8>, k: Seq<&[u8]>, p: Seq<usize>) -> bool {
    prefix_decomposes(s, k, p, k.len() as int) && if k.len() == 0 {
        trivia(s, 0, s.len() as int, true)
    } else {
        trivia(s, p[k.len() - 1] as int + k[k.len() - 1]@.len(), s.len() as int, true)
    }
}

/// Whether `a..b` holds a line feed.
///
/// A scan rather than an existential: the recognizer's invariant mentions this
/// for every token, and an existential would be instantiated for every byte
/// position in every gap.
pub open spec fn gap_has_newline(s: Seq<u8>, a: int, b: int) -> bool
    decreases b - a
{
    if a >= b { false } else { s[a] == b'\n' || gap_has_newline(s, a + 1, b) }
}

/// The offset at which the token before index `j` ends.
pub open spec fn prev_end(s: Seq<u8>, k: Seq<&[u8]>, p: Seq<usize>, j: int) -> int {
    if j == 0 {
        0
    } else {
        p[j - 1] as int + k[j - 1]@.len()
    }
}

/// Whether the first `m` line-feed flags are honoured by their gaps.
///
/// Opaque because the recognizer states this once per token over a growing
/// vector, and a transparent quantifier over the vector's view has to be
/// transferred across every push.
#[verifier::opaque]
pub open spec fn flags_prefix(
    s: Seq<u8>,
    k: Seq<&[u8]>,
    p: Seq<usize>,
    flags: Seq<bool>,
    m: int,
) -> bool {
    forall|j: int| 0 <= j < m && flags[j] ==> gap_has_newline(
        s,
        prev_end(s, k, p, j),
        #[trigger] p[j] as int,
    )
}

/// Whether every line-feed flag is honoured by the gap before its token.
pub open spec fn flags_hold(s: Seq<u8>, k: Seq<&[u8]>, p: Seq<usize>, flags: Seq<bool>) -> bool {
    flags.len() == k.len() && flags_prefix(s, k, p, flags, k.len() as int)
}

// ---------------------------------------------------------------------------
// The concrete baseline
// ---------------------------------------------------------------------------

/// The baseline source every fixture is derived from, byte for byte.
///
/// This is the string `tests/admission/corpus/generated.rs` names
/// `BASELINE`. The source is stated as a specification sequence, not as an
/// exec constant: Verus refuses to coerce a byte literal to a slice
/// reference in a constant initializer, and a slice built that way has
/// opaque contents, so the proof could not inspect its bytes.
pub open spec fn baseline_src() -> Seq<u8> {
    seq![b'p', b'a', b'c', b'k', b'a', b'g', b'e', b' ', b'a', b'u', b't', b'h', b'z',
    b'\n', b'\n', b'i', b'm', b'p', b'o', b'r', b't', b' ', b'r', b'e', b'g', b'o',
    b'.', b'v', b'1', b'\n', b'\n', b'd', b'e', b'f', b'a', b'u', b'l', b't', b' ',
    b'a', b'l', b'l', b'o', b'w', b' ', b':', b'=', b' ', b'f', b'a', b'l', b's', b'e',
    b'\n', b'\n', b'a', b'l', b'l', b'o', b'w', b' ', b'i', b'f', b' ', b'{', b'\n',
    b' ', b' ', b' ', b' ', b'i', b'n', b'p', b'u', b't', b'.', b's', b'u', b'b', b'j',
    b'e', b'c', b't', b'.', b't', b'e', b'n', b'a', b'n', b't', b' ', b'=', b'=', b' ',
    b'i', b'n', b'p', b'u', b't', b'.', b'r', b'e', b's', b'o', b'u', b'r', b'c', b'e',
    b'.', b't', b'e', b'n', b'a', b'n', b't', b'\n', b' ', b' ', b' ', b' ', b'i',
    b'n', b'p', b'u', b't', b'.', b'a', b'c', b't', b'i', b'o', b'n', b' ', b'=', b'=',
    b' ', b'"', b'w', b'r', b'i', b't', b'e', b'"', b'\n', b'}', b'\n']
}

/// The baseline's canonical tokens, as byte sequences.
pub open spec fn baseline_tokens() -> Seq<Seq<u8>> {
    seq![
        seq![b'p', b'a', b'c', b'k', b'a', b'g', b'e'],
        seq![b'a', b'u', b't', b'h', b'z'],
        seq![b'i', b'm', b'p', b'o', b'r', b't'],
        seq![b'r', b'e', b'g', b'o', b'.', b'v', b'1'],
        seq![b'd', b'e', b'f', b'a', b'u', b'l', b't'],
        seq![b'a', b'l', b'l', b'o', b'w'],
        seq![b':', b'='],
        seq![b'f', b'a', b'l', b's', b'e'],
        seq![b'a', b'l', b'l', b'o', b'w'],
        seq![b'i', b'f'],
        seq![b'{'],
        seq![b'i', b'n', b'p', b'u', b't', b'.', b's', b'u', b'b', b'j', b'e', b'c',
        b't', b'.', b't', b'e', b'n', b'a', b'n', b't'],
        seq![b'=', b'='],
        seq![b'i', b'n', b'p', b'u', b't', b'.', b'r', b'e', b's', b'o', b'u', b'r',
        b'c', b'e', b'.', b't', b'e', b'n', b'a', b'n', b't'],
        seq![b'i', b'n', b'p', b'u', b't', b'.', b'a', b'c', b't', b'i', b'o', b'n'],
        seq![b'=', b'='],
        seq![b'"', b'w', b'r', b'i', b't', b'e', b'"'],
        seq![b'}'],
    ]
}

/// The offsets recognition returns for the baseline.
pub open spec fn baseline_offsets() -> Seq<usize> {
    seq![0usize, 8usize, 15usize, 22usize, 31usize, 39usize, 45usize, 48usize, 55usize,
    61usize, 64usize, 70usize, 91usize, 94usize, 120usize, 133usize, 136usize, 144usize]
}

/// The line-feed flags of the baseline's canonical tokens.
///
/// Only the second literal of the rule body begins a line; every other
/// token is on its predecessor's line or begins a module section.
pub open spec fn baseline_flags() -> Seq<bool> {
    seq![false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, true, false, false, false]
}

/// Whether the token byte sequence `tok` sits at `at`.
///
/// This is [`token_at`] with the token stated as its bytes. A concrete proof
/// can inspect such a sequence; the contents of a byte-literal slice are opaque
/// to the verifier, so no concrete instance could be stated with one.
pub open spec fn token_seq_at(s: Seq<u8>, at: int, tok: Seq<u8>) -> bool {
    at + tok.len() <= s.len() && (forall|k: int| 0 <= k < tok.len() ==> s[at + k] == tok[k])
}

/// Whether the first `m` sequence tokens sit at their offsets with trivia in
/// the gaps.
///
/// The counterpart of [`prefix_decomposes`] for token byte sequences.
pub open spec fn prefix_decomposes_seq(
    s: Seq<u8>,
    k: Seq<Seq<u8>>,
    p: Seq<usize>,
    m: int,
) -> bool {
    &&& m <= k.len()
    &&& p.len() == m
    &&& (forall|j: int| 0 <= j < m ==> #[trigger] k[j].len() > 0)
    &&& (forall|j: int| 0 <= j < m ==> #[trigger] token_seq_at(s, p[j] as int, k[j]))
    &&& if m == 0 {
        true
    } else {
        &&& trivia(s, 0, p[0] as int, false)
        &&& (forall|j: int|
            0 < j < m ==> p[j - 1] as int + k[j - 1].len() < #[trigger] p[j] as int && trivia(
                s,
                p[j - 1] as int + k[j - 1].len(),
                p[j] as int,
                false,
            ))
    }
}

/// Whether `k` sits at `p` with trivia filling the whole source elsewhere.
///
/// The counterpart of [`decomposes_at`] for token byte sequences.
pub open spec fn decomposes_seq(s: Seq<u8>, k: Seq<Seq<u8>>, p: Seq<usize>) -> bool {
    prefix_decomposes_seq(s, k, p, k.len() as int) && if k.len() == 0 {
        trivia(s, 0, s.len() as int, true)
    } else {
        trivia(s, p[k.len() - 1] as int + k[k.len() - 1].len(), s.len() as int, true)
    }
}


// ---------------------------------------------------------------------------
// T1 and T2: what an accepted source may contain
// ---------------------------------------------------------------------------

/// Whether byte `i` may appear anywhere in an accepted source.
///
/// This is the permitted set of T1: a tab, printable ASCII, a line feed, or a
/// carriage return that starts a line ending. A token byte is printable ASCII
/// by assumption; trivia is covered by this predicate from the specification.
pub open spec fn permitted_byte(s: Seq<u8>, i: int) -> bool {
    0 <= i < s.len()
        && (s[i] == b'\t' || (0x20 <= s[i] && s[i] <= 0x7e) || s[i] == b'\n' || (s[i]
            == b'\r' && i + 1 < s.len() && s[i + 1] == b'\n'))
}

/// Whether every byte of `a..b` is permitted.
pub open spec fn permitted_range(s: Seq<u8>, a: int, b: int) -> bool {
    forall|i: int| a <= i < b ==> #[trigger] permitted_byte(s, i)
}

/// Whether the `#` at `q` is followed by the metadata marker on its line.
///
/// The marker may start anywhere after the `#` before the next line feed, so
/// `#METADATA`, `# METADATA`, and `#\tMETADATA` are all recognized. Such a
/// comment is exactly what [`comment_ok`] refuses, so an accepted source has
/// none. The line-feed exclusion is what makes this the inline and block forms
/// OPA treats as annotations (D9).
pub open spec fn annotates_metadata(s: Seq<u8>, q: int) -> bool {
    0 <= q < s.len() && s[q] == b'#' && exists|at: int|
        q + 1 <= at && #[trigger] metadata_at(s, at) && !gap_has_newline(s, q + 1, at)
}

/// Whether no byte of `s` begins a metadata annotation.
pub open spec fn no_metadata_annotation(s: Seq<u8>) -> bool {
    forall|q: int| 0 <= q < s.len() && #[trigger] s[q] == b'#' ==> !annotates_metadata(s, q)
}

} // verus!
