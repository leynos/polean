//! A1: the byte keys settlement compares are injective.
//!
//! Each property pairs the real key with a tag-free, length-free reference
//! encoding. The reference is what a careless implementation would produce, and
//! every seeded case shows it colliding on inputs the real key separates.

use proptest::prelude::prop_assert;

use super::support::{
    check,
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
