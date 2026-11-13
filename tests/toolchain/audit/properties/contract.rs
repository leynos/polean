//! A2: the decoder refuses every document outside the report contract.
//!
//! A malformed report must never reach settlement as either an acceptance or a
//! rejection, so each property mutates one aspect of the shared sample and
//! requires a `Malformed` refusal. The last test pins the list of required
//! fields to the sample itself, so a contract change cannot leave the strategy
//! drawing from a stale list.

use proptest::{arbitrary::any, collection::vec as strategy_vec, prelude::prop_assert};

use super::support::{
    REQUIRED,
    check,
    delete_field,
    rename_field,
    required,
    retype_field,
    valid_document,
};
use crate::audit::wire::{DecodeError, decode_report};

/// Removing a required field is refused.
#[test]
fn deleting_a_required_field_is_refused() {
    check(0..REQUIRED.len(), |index| {
        let document = delete_field(index);
        prop_assert!(
            matches!(decode_report(&document), Err(DecodeError::Malformed(_))),
            "a report without {} was not refused",
            required(index)
        );
        Ok(())
    });
}

/// Renaming a required field is refused.
#[test]
fn renaming_a_required_field_is_refused() {
    check(0..REQUIRED.len(), |index| {
        let document = rename_field(index);
        prop_assert!(
            matches!(decode_report(&document), Err(DecodeError::Malformed(_))),
            "a report with {} renamed was not refused",
            required(index)
        );
        Ok(())
    });
}

/// Retyping a required field is refused.
#[test]
fn retyping_a_required_field_is_refused() {
    check(0..REQUIRED.len(), |index| {
        let document = retype_field(index);
        prop_assert!(
            matches!(decode_report(&document), Err(DecodeError::Malformed(_))),
            "a report with {} retyped was not refused",
            required(index)
        );
        Ok(())
    });
}

/// Arbitrary bytes are decided, never fatal.
#[test]
fn arbitrary_bytes_are_decided() {
    check(strategy_vec(any::<u8>(), 0..256), |bytes| {
        drop(decode_report(&bytes));
        Ok(())
    });
}

/// The required field list matches the top-level keys of the shared sample.
///
/// A property over `0..REQUIRED.len()` is only as good as the list it draws
/// from, so the list is pinned to the sample the contract tests share. A
/// contract change then forces this list to change, which a literal count
/// could not detect.
#[test]
fn the_required_field_list_is_the_contract() {
    let document = valid_document();
    let mut actual: Vec<&str> = document.keys().map(String::as_str).collect();
    actual.sort_unstable();
    let mut expected = REQUIRED.to_vec();
    expected.sort_unstable();
    assert_eq!(
        actual, expected,
        "the report contract gained or lost a field"
    );
}
