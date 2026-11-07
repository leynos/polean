//! Ground statement trees and their injective byte key.
//!
//! A statement is a constant applied to a spine of arguments. There is no
//! binder, literal, or variable form, so the tree is finite, closed, and
//! comparable by value. The key encodes the two forms with distinct tags and
//! prefixes every variable-length part with its length, so distinct trees
//! never share a key.
//!
//! The decoder bounds a tree at [`MAX_NODES`] nodes and [`MAX_DEPTH`] levels,
//! counted the way JSON nests them: one for a constant and two for an
//! application. Settlement does not re-check the bound, because the adapter
//! refuses an over-bound tree before a report reaches it.

use super::name::{Name, name_key};

/// The tag that starts a constant node.
const CONST_TAG: u8 = 0x43;

/// The tag that starts an application node.
const APP_TAG: u8 = 0x41;

/// The largest tree the decoder admits, in nodes.
pub const MAX_NODES: usize = 4_096;

/// The largest tree the decoder admits, in JSON nesting levels.
///
/// `serde_json` refuses nesting beyond 127. An application spends two levels,
/// one for its object and one for the two-element array, so the deepest tree
/// admitted here stays well inside that limit.
pub const MAX_DEPTH: usize = 60;

/// A ground statement tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tree {
    /// A constant applied to zero or more universe levels.
    Const {
        /// The constant's name.
        name: Name,
        /// The universe levels the constant is instantiated at.
        levels: Vec<u32>,
    },
    /// One tree applied to another.
    App(Box<Self>, Box<Self>),
}

/// Encode a tree as an injective byte key.
#[must_use]
pub fn tree_key(tree: &Tree) -> Vec<u8> {
    let mut key = Vec::new();
    encode_into(tree, &mut key);
    key
}

/// Append one tree's encoding to `key`.
#[expect(
    clippy::little_endian_bytes,
    reason = "the key encoding is defined as little-endian so it is identical on every host"
)]
fn encode_into(tree: &Tree, key: &mut Vec<u8>) {
    match tree {
        Tree::Const { name, levels } => {
            key.push(CONST_TAG);
            key.extend_from_slice(&name_key(name));
            key.extend_from_slice(&(levels.len() as u64).to_le_bytes());
            for level in levels {
                key.extend_from_slice(&u64::from(*level).to_le_bytes());
            }
        }
        Tree::App(left, right) => {
            key.push(APP_TAG);
            encode_into(left, key);
            encode_into(right, key);
        }
    }
}

/// The number of nodes in a tree.
#[must_use]
pub fn nodes(tree: &Tree) -> usize {
    match tree {
        Tree::Const { .. } => 1,
        Tree::App(left, right) => 1 + nodes(left) + nodes(right),
    }
}

/// The JSON nesting depth of a tree's encoding.
#[must_use]
pub fn depth(tree: &Tree) -> usize {
    match tree {
        Tree::Const { .. } => 1,
        Tree::App(left, right) => 2 + depth(left).max(depth(right)),
    }
}
