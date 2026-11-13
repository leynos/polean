//! A1: the byte keys settlement compares are injective and faithful.
//!
//! Injectivity is necessary but not sufficient: an encoding can separate every
//! pair and still lose information. Each property therefore pairs the real key
//! with a tag-free, length-free reference encoding, which is what a careless
//! implementation would produce, and with a test-only decoder that must return
//! the value the key was made from.

use proptest::prelude::{prop_assert, prop_assert_eq};

use super::support::{
    check,
    decode_name_key,
    decode_tree_key,
    naive_name,
    naive_tree,
    name_strategy,
    near_name,
    near_tree,
    tree_strategy,
};
use crate::audit::wire::{Component, name_key, tree_key};

/// Distinct names never share a key.
#[test]
fn name_keys_are_injective() {
    check((name_strategy(), name_strategy()), |(left, right)| {
        prop_assert!(
            name_key(&left) != name_key(&right) || left == right,
            "two distinct names shared a key: {:?} and {:?}",
            left,
            right
        );
        Ok(())
    });
}

/// Distinct statements never share a key.
#[test]
fn tree_keys_are_injective() {
    check((tree_strategy(), tree_strategy()), |(left, right)| {
        prop_assert!(
            tree_key(&left) != tree_key(&right) || left == right,
            "two distinct statements shared a key: {:?} and {:?}",
            left,
            right
        );
        Ok(())
    });
}

/// Every generated name survives a decode of its own key.
///
/// Injectivity alone would be satisfied by an encoding that is not a faithful
/// reading of the name, so the decoder repeats the wire tags rather than
/// importing them: a format change on either side breaks this property.
#[test]
fn name_keys_round_trip() {
    check(name_strategy(), |name| {
        prop_assert_eq!(decode_name_key(&name_key(&name)), Some(name.clone()));
        Ok(())
    });
}

/// Every generated statement survives a decode of its own key.
///
/// The decoder must also consume every byte: a trailing suffix would fail
/// here even though a prefix-only reading would still be injective.
#[test]
fn tree_keys_round_trip() {
    check(tree_strategy(), |tree| {
        prop_assert_eq!(decode_tree_key(&tree_key(&tree)), Some(tree.clone()));
        Ok(())
    });
}

/// The seeded pair collides without length prefixes and not with them.
///
/// `["ab", "c"]` and `["a", "bc"]` concatenate to the same bytes, so an
/// encoding that merely joins its components cannot tell them apart. The key
/// must, because settlement compares keys and nothing else.
#[test]
fn length_prefixes_prevent_the_naive_collision() {
    let left = vec![
        Component::Text("ab".to_owned()),
        Component::Text("c".to_owned()),
    ];
    let right = vec![
        Component::Text("a".to_owned()),
        Component::Text("bc".to_owned()),
    ];
    assert_eq!(
        naive_name(&left),
        naive_name(&right),
        "the seeded pair must collide without length prefixes"
    );
    assert_ne!(
        name_key(&left),
        name_key(&right),
        "the length prefix did not separate the seeded pair"
    );
}

/// A near-name pair collides without length prefixes and not with them.
///
/// Independent draws almost never produce two distinct names that concatenate
/// to the same bytes, so this property derives the second name from the first
/// by an edit a length-free encoding cannot see. It is the case that catches a
/// dropped length prefix or a dropped component tag.
#[test]
fn near_names_collide_without_prefixes_and_not_with_them() {
    check(name_strategy(), |left| {
        let right = near_name(&left);
        prop_assert!(left != right, "the near-name edit did not change the name");
        prop_assert!(
            naive_name(&left) == naive_name(&right),
            "the near-name edit changed the naive bytes"
        );
        prop_assert!(
            name_key(&left) != name_key(&right),
            "two distinct near names shared a key: {:?} and {:?}",
            left,
            right
        );
        Ok(())
    });
}

/// A near-tree pair collides without tags and not with them.
///
/// The tree key composes the name key, so a name collision that the tree key
/// failed to separate would be a defect in the composition rather than in the
/// name encoding.
#[test]
fn near_trees_collide_without_tags_and_not_with_them() {
    check(tree_strategy(), |left| {
        let right = near_tree(&left);
        prop_assert!(left != right, "the near-tree edit did not change the tree");
        prop_assert!(
            naive_tree(&left) == naive_tree(&right),
            "the near-tree edit changed the naive bytes"
        );
        prop_assert!(
            tree_key(&left) != tree_key(&right),
            "two distinct near trees shared a key: {:?} and {:?}",
            left,
            right
        );
        Ok(())
    });
}
