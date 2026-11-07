//! Byte-level predicates the settlement rules rest on.
//!
//! These helpers answer questions about slices and declaration kinds. Each has
//! a postcondition tying it to the corresponding specification predicate, so the
//! proofs can reason about the executable decision without unfolding it.

use vstd::prelude::*;

verus! {

use super::*;

/// Whether `name` is one of the three pinned foundations.
pub(super) fn pinned(name: &[u8]) -> (result: bool)
    ensures result == pinned_spec(name@),
{
    if name == K_PROPEXT.as_slice() {
        assert(name@ == K_PROPEXT@);
        assert(pinned_spec(name@));
        return true;
    }
    if name == K_QUOT.as_slice() {
        assert(name@ == K_QUOT@);
        assert(pinned_spec(name@));
        return true;
    }
    if name == K_CHOICE.as_slice() {
        assert(name@ == K_CHOICE@);
        assert(pinned_spec(name@));
        return true;
    }
    assert(name@ != K_PROPEXT@);
    assert(name@ != K_QUOT@);
    assert(name@ != K_CHOICE@);
    assert(!pinned_spec(name@));
    false
}

/// Whether a declaration kind is the kind a root must have.
pub(super) const fn is_theorem(kind: DeclKind) -> (result: bool)
    ensures result == is_theorem_spec(kind),
{
    matches!(kind, DeclKind::Theorem)
}

/// Whether `name` appears in `keys`.
#[expect(clippy::indexing_slicing, reason = "the loop guard bounds the index")]
pub(super) fn contains(keys: &[Key], name: &[u8]) -> (result: bool)
    ensures result == contains_spec(keys@, name@),
{
    let mut index = 0;
    while index < keys.len()
        invariant
            index <= keys.len(),
            forall|i: int| 0 <= i < index ==> keys@[i]@ != name@,
        decreases keys.len() - index,
    {
        let candidate = &keys[index];
        if candidate.as_slice() == name {
            assert(candidate@ == name@);
            return true;
        }
        index += 1;
    }
    false
}

/// Whether the request declares `name` an approved foundation.
pub(super) fn approved(request: &AuditRequest, name: &[u8]) -> (result: bool)
    ensures result == approved_spec(request, name@),
{
    contains(request.foundations.as_slice(), name)
}

}
