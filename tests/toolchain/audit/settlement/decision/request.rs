//! Request admission: the rules that decide whether a request is admissible.
//!
//! A request is admissible when it names at least one root, every root has a
//! distinct name, and every declared foundation is pinned. Each scan below is a
//! loop that indexes under its guard, because the proof needs the negated guard
//! to see the whole slice.

use vstd::prelude::*;

verus! {

use super::*;

/// Refuse a request that names no roots, repeats a root, or approves a
/// foundation outside the pinned table.
///
/// # Errors
///
/// Returns the first refusal in that order, naming the offending root or
/// foundation by position.
pub fn validate_request(request: &AuditRequest) -> (result: Result<(), RequestRefusal>)
    ensures result.is_ok() == valid_request_spec(request),
{
    if request.roots.is_empty() {
        assert(!valid_request_spec(request));
        return Err(RequestRefusal::NoRoots);
    }
    if let Some(index) = duplicate_root(request) {
        assert(!valid_request_spec(request));
        return Err(RequestRefusal::DuplicateRoot(index));
    }
    assert(roots_distinct_spec(request));
    if let Some(index) = unapproved_foundation(request) {
        assert(!valid_request_spec(request));
        return Err(RequestRefusal::UnapprovedFoundation(index));
    }
    assert(foundations_pinned_spec(request));
    assert(valid_request_spec(request));
    Ok(())
}

/// The position of the first root whose name repeats an earlier root's.
#[expect(clippy::indexing_slicing, reason = "the loop guard bounds the index")]
fn duplicate_root(request: &AuditRequest) -> (result: Option<usize>)
    ensures result.is_none() == roots_distinct_spec(request),
{
    let mut index = 0;
    while index < request.roots.len()
        invariant
            index <= request.roots@.len(),
            forall|j: int| #![trigger request.roots@[j].name@]
                0 <= j < index ==> forall|k: int| #![trigger request.roots@[k].name@]
                    0 <= k < j ==> request.roots@[k].name@ != request.roots@[j].name@,
        decreases request.roots.len() - index,
    {
        let root = &request.roots[index];
        if repeats_earlier_root(request, root, index) {
            assert(!roots_distinct_spec(request));
            return Some(index);
        }
        index += 1;
    }
    assert(roots_distinct_spec(request));
    None
}

/// Whether `root` has the same name as a root before position `index`.
#[expect(clippy::indexing_slicing, reason = "the loop guard bounds the index")]
fn repeats_earlier_root(request: &AuditRequest, root: &RequestRoot, index: usize) -> (result: bool)
    requires index <= request.roots@.len(),
    ensures result == (exists|earlier: int|
        0 <= earlier < index as int && request.roots@[earlier].name@ == root.name@),
{
    let mut earlier = 0;
    while earlier < index
        invariant
            earlier <= index,
            index <= request.roots@.len(),
            forall|e: int| 0 <= e < earlier ==> request.roots@[e].name@ != root.name@,
        decreases index - earlier,
    {
        let other = &request.roots[earlier];
        if other.name == root.name {
            assert(request.roots@[earlier as int].name@ == root.name@);
            return true;
        }
        earlier += 1;
    }
    assert(forall|e: int| 0 <= e < index ==> request.roots@[e].name@ != root.name@);
    false
}

/// The position of the first declared foundation outside the pinned table.
#[expect(clippy::indexing_slicing, reason = "the loop guard bounds the index")]
fn unapproved_foundation(request: &AuditRequest) -> (result: Option<usize>)
    ensures result.is_none() == foundations_pinned_spec(request),
{
    let mut index = 0;
    while index < request.foundations.len()
        invariant
            index <= request.foundations@.len(),
            forall|j: int| #![trigger request.foundations@[j]@]
                0 <= j < index ==> pinned_spec(request.foundations@[j]@),
        decreases request.foundations.len() - index,
    {
        let name = &request.foundations[index];
        if !pinned(name.as_slice()) {
            assert(!foundations_pinned_spec(request));
            return Some(index);
        }
        index += 1;
    }
    assert(foundations_pinned_spec(request));
    None
}

}
