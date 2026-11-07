//! Strict decoding and injective key encoding for the audit contracts.
//!
//! [`decode`] turns contract JSON into settlement's value types and refuses
//! anything outside the contract. [`name`] and [`tree`] turn the decoded names
//! and statements into byte keys, which are the only representation settlement
//! compares. Keys are injective, so equality of keys is equality of names and
//! statements, and the settlement proof can treat them as opaque.
//!
//! The encoding is deliberately not JSON-shaped. A report name crosses the
//! contract as an array of components, so a name that literally contains a dot
//! such as `Quot.sound` cannot be confused with the two-component name
//! `Quot` `sound`; the key then preserves that distinction with a tag and a
//! length prefix.

mod decode;
mod document;
mod name;
mod negative;
mod tests;
mod tree;

pub use self::{
    decode::{DecodeError, MAX_REPORT_BYTES, decode_report, decode_request},
    name::{Component, Name, is_native, name_key},
    tree::{Tree, tree_key},
};

/// The commit the pinned Lean build reports.
///
/// The wire layer owns this constant because the shared samples and the
/// fixtures both need it, and neither may depend on the other.
pub const PIN: &str = "eb6cafb7290e8bd5f2937522d8d804f58f02bafc";
