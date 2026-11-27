//! The closed policy intermediate representation and its canonical encoding.
//!
//! A [`Policy`] is an ordered sequence of zero to the profile's rule maximum of
//! [`Rule`]s, and a [`Rule`] is an ordered sequence of one to the profile's
//! body maximum of [`Atom`]s. The bounds are type invariants: the constructors
//! return an error rather than a value that breaks them, and every field is
//! private, so no other way to obtain a value exists. The types implement
//! neither [`serde::Serialize`] nor [`serde::Deserialize`], so a derive cannot
//! bypass the bounds.
//!
//! Nothing here admits a Rego source, proves a model claim, or shows that a
//! runtime agrees with the model. Canonical bytes are the identity evidence
//! binding hashes, and the encoder that produces them is verified; see
//! [`model`].
//!
//! # Examples
//!
//! ```
//! use polean::ir::{Atom, Policy, Rule};
//!
//! let rule = Rule::new(vec![Atom::SameTenant, Atom::WriteAction])
//!     .expect("a body of two atoms is within the bounds");
//! let policy = Policy::new(vec![rule]).expect("one rule is within the bound");
//!
//! let canonical = policy.to_canonical_bytes();
//! assert_eq!(
//!     canonical.as_bytes(),
//!     b"{\"profile\":\"polean.tenant-write.v0\",\"rules\":[[\"same_tenant\",\"write_action\"]]}\n",
//!     "the encoder writes the canonical form"
//! );
//! ```

pub mod model;

pub use self::model::{ATOM_NAMES, Atom, CanonicalIr, Policy, PolicyError, Rule, RuleError};

/// The largest policy document either decoder will parse, in bytes.
///
/// Equal to the profile's `limits.input_bytes`: technical design §11 sets one
/// 64 KiB budget for a request and for the IR document that answers it. The
/// largest valid canonical document is far smaller — 4,175 bytes for 16 rules
/// of 16 of the longest atom name — so the cap never refuses a well-formed
/// policy.
pub const IR_DOCUMENT_BYTES_MAX: usize = crate::profile::INPUT_BYTES_MAX;
