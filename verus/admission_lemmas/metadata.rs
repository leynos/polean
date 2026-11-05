//! T2: an accepted source has no metadata annotation.

use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use super::super::spec::*;
#[cfg(verus_keep_ghost)]
use super::comments::{lemma_comment_body_byte, lemma_comment_end_ge};
#[cfg(verus_keep_ghost)]
use super::permitted::{lemma_prefix_bounds, lemma_prefix_step_down};

verus! {

// ---------------------------------------------------------------------------
// T2: an accepted source has no metadata annotation
// ---------------------------------------------------------------------------

/// Comment-end scans agree from two starts inside the same comment.
pub proof fn lemma_comment_end_suffix(s: Seq<u8>, a: int, q: int)
    requires
        0 <= a <= q <= comment_end_from(s, a),
    ensures
        comment_end_from(s, a) == comment_end_from(s, q),
    decreases q - a,
{
    reveal_with_fuel(comment_end_from, 1);
    if a < q {
        assert(a < s.len() as int && !line_end_at(s, a) && body_byte(s[a]));
        lemma_comment_end_suffix(s, a + 1, q);
    }
}

/// A comment-body scan never stops before the position it started from.
pub proof fn lemma_comment_end_at_least_start(s: Seq<u8>, i: int)
    requires
        0 <= i,
    ensures
        comment_end_from(s, i) >= i,
    decreases s.len() as int - i,
{
    reveal_with_fuel(comment_end_from, 2);
    if i < s.len() as int && !line_end_at(s, i) && body_byte(s[i]) {
        lemma_comment_end_at_least_start(s, i + 1);
    }
}

/// A run of body bytes that are not line endings extends the comment body.
pub proof fn lemma_comment_end_beyond_body(s: Seq<u8>, i: int, n: int)
    requires
        0 <= i,
        i + n <= s.len() as int,
        forall|j: int| i <= j < i + n ==> #[trigger] body_byte(s[j]) && !line_end_at(s, j),
    ensures
        comment_end_from(s, i) >= i + n,
    decreases n,
{
    reveal_with_fuel(comment_end_from, 3);
    if n > 0 {
        assert(i < s.len() as int);
        assert(body_byte(s[i]) && !line_end_at(s, i));
        assert(!(i >= s.len() as int || line_end_at(s, i) || !body_byte(s[i])));
        assert(comment_end_from(s, i) == comment_end_from(s, i + 1));
        assert forall|j: int| i + 1 <= j < i + n implies #[trigger] body_byte(s[j])
            && !line_end_at(s, j) by {
        };
        lemma_comment_end_beyond_body(s, i + 1, n - 1);
    } else {
        lemma_comment_end_at_least_start(s, i);
    }
}

/// A metadata marker immediately after a `#` lies wholly inside its comment.
pub proof fn lemma_metadata_marker_fits(s: Seq<u8>, hash: int)
    requires
        0 <= hash,
        hash + 1 + metadata_bytes().len() <= s.len() as int,
        s[hash] == b'#',
        metadata_at(s, hash + 1),
    ensures
        hash + 1 + metadata_bytes().len() <= comment_end(s, hash),
{
    assert forall|j: int|
        hash + 1 <= j < hash + 1 + metadata_bytes().len() implies #[trigger] body_byte(
        s[j],
    ) && !line_end_at(s, j) by {
        let k = j - (hash + 1);
        assert(0 <= k < metadata_bytes().len());
        assert(s[(hash + 1) + k] == metadata_bytes()[k]);
        assert((hash + 1) + k == j);
    };
    lemma_comment_end_beyond_body(s, hash + 1, metadata_bytes().len() as int);
}

/// Every byte of the metadata marker is a body byte and starts no line ending.
pub proof fn lemma_metadata_bytes_letters()
    ensures
        forall|k: int| 0 <= k < metadata_bytes().len() ==> body_byte(
            #[trigger] metadata_bytes()[k],
        ) && !line_end_at(metadata_bytes(), k),
{
    // `metadata_bytes` is the eight letters of `METADATA`; every one lies in
    // printable ASCII, so it is a body byte and starts no line ending.
    assert forall|k: int| 0 <= k < metadata_bytes().len() implies body_byte(
        #[trigger] metadata_bytes()[k],
    ) && !line_end_at(metadata_bytes(), k) by {
        if k == 0 {
            assert(metadata_bytes()[k] == b'M');
        } else if k == 1 {
            assert(metadata_bytes()[k] == b'E');
        } else if k == 2 {
            assert(metadata_bytes()[k] == b'T');
        } else if k == 3 {
            assert(metadata_bytes()[k] == b'A');
        } else if k == 4 {
            assert(metadata_bytes()[k] == b'D');
        } else if k == 5 {
            assert(metadata_bytes()[k] == b'A');
        } else if k == 6 {
            assert(metadata_bytes()[k] == b'T');
        } else {
            assert(0 <= k < metadata_bytes().len());
            assert(k == 7);
            assert(metadata_bytes()[k] == b'A');
        }
        assert(0x20 <= metadata_bytes()[k] && metadata_bytes()[k] <= 0x7e);
        assert(metadata_bytes()[k] != b'\n' && metadata_bytes()[k] != b'\r');
    };
}

/// Every byte of a metadata marker in `s` is a body byte and starts no line
/// ending.
pub proof fn lemma_metadata_span_body(s: Seq<u8>, at: int)
    requires
        metadata_at(s, at),
    ensures
        forall|i: int| at <= i < at + metadata_bytes().len() ==> body_byte(
            #[trigger] s[i],
        ) && !line_end_at(s, i),
{
    lemma_metadata_bytes_letters();
    assert forall|i: int| at <= i < at + metadata_bytes().len() implies body_byte(
        #[trigger] s[i],
    ) && !line_end_at(s, i) by {
        let k = i - at;
        assert(0 <= k < metadata_bytes().len());
        assert(s[at + k] == metadata_bytes()[k]);
        assert(at + k == i);
        assert(s[i] == metadata_bytes()[k]);
        assert(body_byte(metadata_bytes()[k]));
        assert(metadata_bytes()[k] != b'\n' && metadata_bytes()[k] != b'\r');
        assert(s[i] != b'\n' && s[i] != b'\r');
        assert(i < s.len() as int);
    };
}

/// A newline inside `a..b` makes the gap report one.
pub proof fn lemma_gap_newline_witness(s: Seq<u8>, a: int, b: int, i: int)
    requires
        0 <= a <= i < b <= s.len() as int,
        s[i] == b'\n',
    ensures
        gap_has_newline(s, a, b),
    decreases b - a,
{
    reveal_with_fuel(gap_has_newline, 2);
    if a < i {
        lemma_gap_newline_witness(s, a + 1, b, i);
    }
}

/// A gap reports a newline only if one of its bytes is a newline.
pub proof fn lemma_gap_newline_witness_back(s: Seq<u8>, a: int, b: int)
    requires
        0 <= a <= b <= s.len() as int,
        gap_has_newline(s, a, b),
    ensures
        exists|i: int| a <= i < b && #[trigger] s[i] == b'\n',
    decreases b - a,
{
    reveal_with_fuel(gap_has_newline, 2);
    if s[a] == b'\n' {
        assert(s[a as int] == b'\n');
    } else {
        lemma_gap_newline_witness_back(s, a + 1, b);
    }
}

/// A metadata annotation cannot start inside a well-formed comment, including
/// when whitespace separates the `#` from the marker.
pub proof fn lemma_comment_no_metadata(s: Seq<u8>, hash: int, at: int)
    requires
        0 <= hash < s.len() as int,
        s[hash] == b'#',
        comment_ok(s, hash),
        hash + 1 <= at,
        metadata_at(s, at),
        !gap_has_newline(s, hash + 1, at),
    ensures
        false,
{
    let end = comment_end(s, hash);
    let width = metadata_bytes().len();
    lemma_comment_end_ge(s, hash + 1);
    lemma_metadata_span_body(s, at);
    assert(at + width <= s.len());
    if at > end {
        // The comment ends before the marker starts, so its line ending lies
        // inside the gap that is said to hold no newline.
        assert(end < s.len());
        assert(line_end_at(s, end));
        if s[end] == b'\n' {
            lemma_gap_newline_witness(s, hash + 1, at, end);
        } else {
            assert(s[end] == b'\r' && s[end + 1] == b'\n');
            if end + 1 == at {
                assert(false);
            } else {
                lemma_gap_newline_witness(s, hash + 1, at, end + 1);
            }
        }
    } else if at == end {
        // The comment ends exactly where the marker would begin, but its end
        // is a line ending and the marker's first byte is a letter.
        assert(end < s.len());
        assert(line_end_at(s, end));
        assert(false);
    } else if at + width <= end {
        // The whole marker lies in the comment body, which `comment_ok`
        // forbids.
        assert(metadata_in(s, hash + 1, end)) by {
            assert(metadata_at(s, at));
            assert(hash + 1 <= at);
            assert(at + width <= end);
        }
        assert(false);
    } else {
        // The comment ends inside the marker, whose bytes are all body bytes
        // that start no line ending.
        assert(at <= end < at + width);
        assert(!line_end_at(s, end));
        assert(end < s.len());
        assert(line_end_at(s, end));
        assert(false);
    }
}

/// A trivia gap holds no metadata annotation.
pub proof fn lemma_trivia_core_no_metadata(s: Seq<u8>, a: int, b: int)
    requires
        0 <= a <= b <= s.len() as int,
        trivia_core(s, a, b),
    ensures
        forall|q: int| a <= q < b && #[trigger] s[q] == b'#' ==> !annotates_metadata(s, q),
    decreases b - a,
{
    reveal_with_fuel(trivia_core, 3);
    if a >= b {
    } else if ws_at(s, a) {
        assert(s[a] != b'#');
        lemma_trivia_core_no_metadata(s, a + 1, b);
        assert forall|q: int| a <= q < b && #[trigger] s[q] == b'#' implies !annotates_metadata(
            s,
            q,
        ) by {
            if q == a {
                assert(s[a] != b'#');
            }
        };
    } else if s[a] == b'#' && comment_ok(s, a) && a < comment_end(s, a) && comment_end(s, a) <= b {
        lemma_trivia_core_no_metadata(s, comment_end(s, a), b);
        assert forall|q: int| a <= q < b && #[trigger] s[q] == b'#' implies !annotates_metadata(
            s,
            q,
        ) by {
            if q < comment_end(s, a) {
                if annotates_metadata(s, q) {
                    let at = choose|at: int|
                        q + 1 <= at && #[trigger] metadata_at(s, at) && !gap_has_newline(
                            s,
                            q + 1,
                            at,
                        );
                    // The bytes between the `#` and `q` are body bytes, and
                    // `q` itself is `#`, so the gap from `a + 1` to `at` also
                    // holds no newline.
                    assert forall|i: int| a < i <= q implies #[trigger] s[i] != b'\n' by {
                        if i < comment_end(s, a) {
                            lemma_comment_body_byte(s, a, i);
                        }
                    };
                    if gap_has_newline(s, a + 1, at) {
                        lemma_gap_newline_witness_back(s, a + 1, at);
                        let i = choose|i: int| a + 1 <= i < at && #[trigger] s[i] == b'\n';
                        assert(i > q);
                        assert(at <= s.len() as int);
                        lemma_gap_newline_witness(s, q + 1, at, i);
                        assert(false);
                    }
                    lemma_comment_no_metadata(s, a, at);
                    assert(false);
                }
            }
        };
    }
}

/// A token holds no metadata annotation if its bytes are not `#`.
pub proof fn lemma_prefix_no_metadata(s: Seq<u8>, k: Seq<&[u8]>, p: Seq<usize>, m: int)
    requires
        prefix_decomposes(s, k, p, m),
        forall|j: int, i: int| 0 <= j < k.len() && 0 <= i < k[j]@.len() ==> #[trigger] k[j]@[i]
            != b'#',
    ensures
        forall|q: int|
            0 <= q < prev_end(s, k, p, m) && #[trigger] s[q] == b'#' ==> !annotates_metadata(s, q),
    decreases m,
{
    if m == 0 {
    } else {
        let previous = p.take(m - 1);
        lemma_prefix_step_down(s, k, p, m);
        lemma_prefix_no_metadata(s, k, previous, m - 1);
        assert(trivia(s, prev_end(s, k, p, m - 1), p[m - 1] as int, false));
        reveal(trivia);
        lemma_prefix_bounds(s, k, previous, m - 1);
        lemma_prefix_bounds(s, k, p, m);
        if m == 1 {
            assert(prev_end(s, k, p, 0) == 0);
        } else {
            assert(prev_end(s, k, p, m - 1) == p[m - 2] as int + k[m - 2]@.len());
            assert(p[m - 2] as int + k[m - 2]@.len() < p[m - 1] as int);
        }
        assert(prev_end(s, k, p, m - 1) <= p[m - 1] as int);
        lemma_trivia_core_no_metadata(s, prev_end(s, k, p, m - 1), p[m - 1] as int);
        assert forall|i: int|
            p[m - 1] as int <= i < p[m - 1] as int + k[m - 1]@.len() implies s[i]
            != b'#' by {
            assert(token_at(s, p[m - 1] as int, k[m - 1]));
            assert(s[i] == k[m - 1]@[i - p[m - 1] as int]);
        };
        assert forall|q: int|
            0 <= q < prev_end(s, k, p, m) && #[trigger] s[q] == b'#' implies !annotates_metadata(
            s,
            q,
        ) by {
            if q < p[m - 1] as int {
            } else {
                assert(s[q] != b'#');
            }
        };
    }
}

/// T2: no byte of an accepted source begins a metadata annotation, provided no
/// token byte is `#`.
pub proof fn lemma_no_metadata_outside_tokens(s: Seq<u8>, k: Seq<&[u8]>, p: Seq<usize>)
    requires
        decomposes_at(s, k, p),
        forall|j: int, i: int| 0 <= j < k.len() && 0 <= i < k[j]@.len() ==> #[trigger] k[j]@[i]
            != b'#',
    ensures
        no_metadata_annotation(s),
{
    let start = prev_end(s, k, p, k.len() as int);
    lemma_prefix_no_metadata(s, k, p, k.len() as int);
    reveal(trivia);
    if k.len() == 0 {
        assert(trivia_core(s, 0, s.len() as int));
        lemma_trivia_core_no_metadata(s, 0, s.len() as int);
    } else {
        assert(trivia_core(s, start, s.len() as int));
        lemma_trivia_core_no_metadata(s, start, s.len() as int);
    }
    assert forall|q: int| 0 <= q < s.len() as int && #[trigger] s[q] == b'#'
        implies !annotates_metadata(s, q) by {
        if q < start {
        }
    };
}


} // verus!
