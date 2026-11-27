//! The public surface of the IR types, and their fallible constructors.
//!
//! These items sit outside `verus!` because they are ordinary Rust: the error
//! enums, the canonical-bytes newtype, and the constructors that turn a
//! caller's `Vec` into a bounded value. The types they build are declared in
//! [`super`] inside `verus!`, so the verified encoder and the shipped code are
//! the same text.
//!
//! Every field is private. [`Rule::new`] and [`Policy::new`] are the only ways
//! to obtain a value, and each returns an error rather than a value that breaks
//! the profile's bounds.

use super::{Atom, Policy, Rule, encode};

// The bounds are restated here, outside `verus!`, so the unverified formatting
// code does not reach into the verified block. `tests/ingestion/contracts.rs`
// ties all three to `contracts/profile.json`; the kernel has its own copies for
// the same reason, and both halves are checked against the one contract.
/// The fewest atoms a rule body may hold.
const BODY_ATOMS_MIN: usize = 1;

/// The most atoms a rule body may hold.
const BODY_ATOMS_MAX: usize = 16;

/// The most rules a policy may hold.
const RULES_MAX: usize = 16;
/// The atom names, in profile order.
///
/// This is the one place outside `verus!` that lists the atoms, and a contract
/// test compares it with `contracts/profile.json`'s key order. The kernel's
/// own names are separate constants; a test ties the two together.
pub const ATOM_NAMES: [&str; 6] = [
    "same_tenant",
    "write_action",
    "admin_role",
    "owns_resource",
    "locked",
    "unlocked",
];

impl Atom {
    /// Every atom, in profile order.
    pub const ALL: [Self; 6] = [
        Self::SameTenant,
        Self::WriteAction,
        Self::AdminRole,
        Self::OwnsResource,
        Self::Locked,
        Self::Unlocked,
    ];

    /// The atom's name as text.
    ///
    /// The kernel writes the name from its own constant bytes; a test ties
    /// this accessor to them, so the two cannot drift.
    #[must_use]
    pub const fn name_str(self) -> &'static str {
        match self {
            Self::SameTenant => "same_tenant",
            Self::WriteAction => "write_action",
            Self::AdminRole => "admin_role",
            Self::OwnsResource => "owns_resource",
            Self::Locked => "locked",
            Self::Unlocked => "unlocked",
        }
    }
}

/// One rule body that is not a valid [`Rule`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleError {
    /// The body held no atoms. A rule is a conjunction, and the empty
    /// conjunction is not a rule of this profile.
    Empty,
    /// The body held more atoms than the profile allows.
    TooManyAtoms {
        /// The number of atoms supplied.
        count: usize,
    },
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(
                formatter,
                "a rule body must hold at least {BODY_ATOMS_MIN} atom"
            ),
            Self::TooManyAtoms { count } => write!(
                formatter,
                "a rule body holds {count} atoms, over the {BODY_ATOMS_MAX} the profile allows"
            ),
        }
    }
}

impl std::error::Error for RuleError {}

/// A policy that holds more rules than the profile allows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyError {
    /// The policy held more rules than the profile allows.
    TooManyRules {
        /// The number of rules supplied.
        count: usize,
    },
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyRules { count } => write!(
                formatter,
                "the policy holds {count} rules, over the {RULES_MAX} the profile allows"
            ),
        }
    }
}

impl std::error::Error for PolicyError {}

/// The canonical bytes of one policy.
///
/// A distinct type from the source bytes roadmap 4.1 binds, so a caller cannot
/// pass one where the other is meant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalIr(Vec<u8>);

impl CanonicalIr {
    /// The bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] { &self.0 }

    /// The bytes, owned.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> { self.0 }
}

impl Rule {
    /// Build a rule body from `atoms`, preserving their order.
    ///
    /// # Errors
    ///
    /// Returns [`RuleError::Empty`] when `atoms` is empty, and
    /// [`RuleError::TooManyAtoms`] when it holds more than the profile's body
    /// maximum.
    ///
    /// # Examples
    ///
    /// ```
    /// use polean::ir::{Atom, Rule, RuleError};
    ///
    /// assert_eq!(Rule::new(Vec::new()), Err(RuleError::Empty));
    /// assert!(Rule::new(vec![Atom::Locked]).is_ok());
    /// ```
    pub fn new(atoms: Vec<Atom>) -> Result<Self, RuleError> {
        if atoms.len() > BODY_ATOMS_MAX {
            return Err(RuleError::TooManyAtoms { count: atoms.len() });
        }
        Self::from_atoms(atoms).ok_or(RuleError::Empty)
    }

    /// The atoms, in order.
    #[must_use]
    pub fn atoms(&self) -> &[Atom] { self.atoms_slice() }
}

impl Policy {
    /// Build a policy from `rules`, preserving their order.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyError::TooManyRules`] when `rules` holds more than the
    /// profile's rule maximum.
    pub fn new(rules: Vec<Rule>) -> Result<Self, PolicyError> {
        if rules.len() > RULES_MAX {
            return Err(PolicyError::TooManyRules { count: rules.len() });
        }
        Ok(Self::from_rules(rules))
    }

    /// The rules, in order.
    #[must_use]
    pub fn rules(&self) -> &[Rule] { &self.rules }

    /// The canonical bytes of this policy.
    ///
    /// The bytes are `{"profile":"<id>","rules":[...]}` plus one line feed,
    /// with no other whitespace, preserving rule and atom order. Distinct
    /// policies have distinct canonical bytes; the proof is in
    /// `verus/policy_canonical.rs`.
    #[must_use]
    pub fn to_canonical_bytes(&self) -> CanonicalIr {
        let mut bytes = Vec::new();
        encode(&mut bytes, self);
        CanonicalIr(bytes)
    }
}
