//! Tests for the Polean-owned inventory table.
//!
//! These check the table itself, not the frontend: that every kind has its own
//! ordinal and name, and that a manifest name resolves back to the same kind.
//! Frontend observations live in [`crate::observations`].

use std::collections::BTreeSet;

use crate::inventory::NodeKind;

#[test]
fn every_kind_has_a_distinct_ordinal() {
    let ordinals: BTreeSet<usize> = NodeKind::ALL.iter().map(|kind| kind.ordinal()).collect();
    let expected: BTreeSet<usize> = (0..NodeKind::ALL.len()).collect();
    assert_eq!(
        ordinals,
        expected,
        "ordinals must cover 0..{} exactly once",
        NodeKind::ALL.len()
    );
}

#[test]
fn every_kind_has_a_distinct_name() {
    let names: BTreeSet<&str> = NodeKind::ALL.iter().map(|kind| kind.name()).collect();
    assert_eq!(
        names.len(),
        NodeKind::ALL.len(),
        "kind names must be unique; duplicates cannot be written to the manifest"
    );
}

#[test]
fn a_manifest_name_resolves_to_its_kind() {
    for kind in NodeKind::ALL {
        assert_eq!(
            NodeKind::from_name(kind.name()),
            Some(kind),
            "{} must resolve to itself",
            kind.name()
        );
    }
}

#[test]
fn an_unknown_manifest_name_resolves_to_nothing() {
    assert_eq!(NodeKind::from_name("ExprOr"), None);
}
