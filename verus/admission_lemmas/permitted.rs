//! T1: every byte of an accepted source is permitted.

use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use super::super::spec::*;
#[cfg(verus_keep_ghost)]
use super::comments::lemma_comment_body_byte;

verus! {

// ---------------------------------------------------------------------------
// T1: every byte of an accepted source is permitted
// ---------------------------------------------------------------------------

/// A comment-end scan never runs past the end of the source.
pub proof fn lemma_comment_end_bounded(s: Seq<u8>, i: int)
    requires
        0 <= i <= s.len() as int,
    ensures
        comment_end_from(s, i) <= s.len() as int,
    decreases s.len() as int - i,
{
    if i < s.len() as int && !line_end_at(s, i) && body_byte(s[i]) {
        lemma_comment_end_bounded(s, i + 1);
    }
}

/// A byte strictly inside a well-formed comment's body is permitted.
pub proof fn lemma_comment_body_permitted(s: Seq<u8>, hash: int, i: int)
    requires
        0 <= hash < s.len() as int,
        s[hash] == b'#',
        comment_ok(s, hash),
        hash < i < comment_end(s, hash),
    ensures
        permitted_byte(s, i),
{
    lemma_comment_body_byte(s, hash, i);
    lemma_comment_end_bounded(s, hash + 1);
    assert(body_byte(s[i]));
}

/// Every byte of a trivia gap is permitted.
pub proof fn lemma_trivia_core_permitted(s: Seq<u8>, a: int, b: int)
    requires
        0 <= a <= b <= s.len() as int,
        trivia_core(s, a, b),
    ensures
        permitted_range(s, a, b),
    decreases b - a,
{
    reveal_with_fuel(trivia_core, 3);
    if a >= b {
    } else if ws_at(s, a) {
        assert(permitted_byte(s, a));
        lemma_trivia_core_permitted(s, a + 1, b);
    } else if s[a] == b'#' && comment_ok(s, a) && a < comment_end(s, a) && comment_end(s, a) <= b {
        assert(permitted_byte(s, a));
        assert forall|i: int| a < i < comment_end(s, a) implies permitted_byte(s, i) by {
            lemma_comment_body_permitted(s, a, i);
        };
        lemma_trivia_core_permitted(s, comment_end(s, a), b);
    }
}

/// Every byte of a complete trivia gap is permitted.
pub proof fn lemma_trivia_permitted(s: Seq<u8>, a: int, b: int, last: bool)
    requires
        0 <= a <= b <= s.len() as int,
        trivia(s, a, b, last),
    ensures
        permitted_range(s, a, b),
{
    reveal(trivia);
    lemma_trivia_core_permitted(s, a, b);
}

/// The first `m` tokens of a decomposition, with their gaps, cover a prefix.
///
/// The lemma is stated over the same offsets as the decomposition so that the
/// caller needs no separate partition argument: each byte of the prefix is in
/// one of the gaps or in one of the tokens.
pub proof fn lemma_prefix_step_down(s: Seq<u8>, k: Seq<&[u8]>, p: Seq<usize>, m: int)
    requires
        prefix_decomposes(s, k, p, m),
        m > 0,
    ensures
        prefix_decomposes(s, k, p.take(m - 1), m - 1),
{
    assert(p.take(m - 1).len() == m - 1);
    assert forall|j: int| 0 <= j < m - 1 implies #[trigger] token_at(
        s,
        p.take(m - 1)[j] as int,
        k[j],
    ) by {
        assert(p.take(m - 1)[j] == p[j]);
    };
    if m - 1 > 0 {
        assert forall|j: int|
            0 < j < m - 1 implies p.take(m - 1)[j - 1] as int + k[j - 1]@.len() < #[trigger] p.take(
                m - 1,
            )[j] as int && trivia(
                s,
                p.take(m - 1)[j - 1] as int + k[j - 1]@.len(),
                p.take(m - 1)[j] as int,
                false,
            ) by {
            assert(p.take(m - 1)[j - 1] == p[j - 1]);
            assert(p.take(m - 1)[j] == p[j]);
        };
    }
}

/// A prefix's last offset and its end lie within the source.
pub proof fn lemma_prefix_bounds(s: Seq<u8>, k: Seq<&[u8]>, p: Seq<usize>, m: int)
    requires
        prefix_decomposes(s, k, p, m),
    ensures
        0 <= prev_end(s, k, p, m) <= s.len() as int,
        m > 0 ==> 0 <= p[m - 1] as int <= s.len() as int,
{
    if m == 0 {
        assert(prev_end(s, k, p, m) == 0);
    } else {
        assert(token_at(s, p[m - 1] as int, k[m - 1]));
        assert(p[m - 1] as int + k[m - 1]@.len() <= s.len() as int);
        assert(prev_end(s, k, p, m) == p[m - 1] as int + k[m - 1]@.len());
    }
}

/// Every byte before the last token of a prefix is permitted.
pub proof fn lemma_prefix_permitted(s: Seq<u8>, k: Seq<&[u8]>, p: Seq<usize>, m: int)
    requires
        prefix_decomposes(s, k, p, m),
        forall|j: int, i: int| 0 <= j < k.len() && 0 <= i < k[j]@.len() ==> 0x20
            <= #[trigger] k[j]@[i] <= 0x7e,
    ensures
        permitted_range(s, 0, prev_end(s, k, p, m)),
    decreases m,
{
    if m == 0 {
    } else {
        let previous = p.take(m - 1);
        lemma_prefix_step_down(s, k, p, m);
        lemma_prefix_permitted(s, k, previous, m - 1);
        assert(prev_end(s, k, previous, m - 1) == prev_end(s, k, p, m - 1));
        assert(trivia(s, prev_end(s, k, p, m - 1), p[m - 1] as int, false));
        lemma_prefix_bounds(s, k, previous, m - 1);
        lemma_prefix_bounds(s, k, p, m);
        if m == 1 {
            assert(prev_end(s, k, p, 0) == 0);
        } else {
            assert(prev_end(s, k, p, m - 1) == p[m - 2] as int + k[m - 2]@.len());
            assert(p[m - 2] as int + k[m - 2]@.len() < p[m - 1] as int);
        }
        assert(prev_end(s, k, p, m - 1) <= p[m - 1] as int);
        lemma_trivia_permitted(s, prev_end(s, k, p, m - 1), p[m - 1] as int, false);
        assert forall|i: int|
            p[m - 1] as int <= i < p[m - 1] as int + k[m - 1]@.len() implies permitted_byte(
            s,
            i,
        ) by {
            assert(token_at(s, p[m - 1] as int, k[m - 1]));
            assert(s[i] == k[m - 1]@[i - p[m - 1] as int]);
        };
        assert forall|i: int| 0 <= i < prev_end(s, k, p, m) implies permitted_byte(s, i) by {
            if i < prev_end(s, k, p, m - 1) {
            } else if i < p[m - 1] as int {
            }
        };
    }
}

/// T1: if the tokens are printable ASCII, every byte of an accepted source is
/// a tab, printable ASCII, a line feed, or a carriage return that starts a line
/// ending.
pub proof fn lemma_no_hidden_bytes(s: Seq<u8>, k: Seq<&[u8]>, p: Seq<usize>)
    requires
        decomposes_at(s, k, p),
        forall|j: int, i: int| 0 <= j < k.len() && 0 <= i < k[j]@.len() ==> 0x20
            <= #[trigger] k[j]@[i] <= 0x7e,
    ensures
        permitted_range(s, 0, s.len() as int),
{
    let start = prev_end(s, k, p, k.len() as int);
    lemma_prefix_permitted(s, k, p, k.len() as int);
    lemma_prefix_bounds(s, k, p, k.len() as int);
    if k.len() == 0 {
        lemma_trivia_permitted(s, 0, s.len() as int, true);
    } else {
        lemma_trivia_permitted(s, start, s.len() as int, true);
    }
    assert forall|i: int| 0 <= i < s.len() as int implies permitted_byte(s, i) by {
        if i < start {
        }
    };
}


} // verus!
