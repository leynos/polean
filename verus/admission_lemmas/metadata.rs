//! T2: an accepted source has no metadata annotation.

use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use super::super::spec::*;
#[cfg(verus_keep_ghost)]
use super::permitted::{lemma_comment_end_bounded, lemma_prefix_bounds, lemma_prefix_step_down};

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

/// A metadata annotation cannot start inside a well-formed comment.
pub proof fn lemma_comment_no_metadata(s: Seq<u8>, hash: int, q: int)
    requires
        0 <= hash < s.len() as int,
        s[hash] == b'#',
        comment_ok(s, hash),
        hash <= q < comment_end(s, hash),
        annotates_metadata(s, q),
    ensures
        false,
{
    assert(metadata_at(s, q + 1));
    lemma_comment_end_bounded(s, hash + 1);
    lemma_metadata_marker_fits(s, q);
    if q == hash {
    } else {
        lemma_comment_end_suffix(s, hash + 1, q + 1);
    }
    assert(metadata_in(s, hash + 1, comment_end(s, hash))) by {
        assert(metadata_at(s, q + 1));
        assert(q + 1 + metadata_bytes().len() <= comment_end(s, hash));
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
                    lemma_comment_no_metadata(s, a, q);
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
