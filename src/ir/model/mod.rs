//! The closed policy IR and the canonical encoding, verified where it ships.
//!
//! This module is written inside `verus!`, so the executable encoder and the
//! specification Verus checks are the same text. Cargo compiles it normally,
//! and `verus/policy_canonical.rs` verifies the same file through `#[path]`.
//!
//! # Shape
//!
//! A [`Policy`] is an ordered sequence of zero to [`RULES_MAX`] [`Rule`]s, and
//! a [`Rule`] is an ordered sequence of one to [`BODY_ATOMS_MAX`] [`Atom`]s.
//! An [`Atom`] is one of exactly six values, so no free string, span, or source
//! text can enter the IR. The bounds are type invariants: the constructors
//! return an error rather than a value that breaks them.
//!
//! # What the proof establishes
//!
//! `verus/policy_canonical.rs` proves that the encoder returns exactly the
//! specification's bytes for every policy, and that distinct policies never
//! share canonical bytes. It establishes nothing about Rego, admission, or a
//! runtime; those are separate claims that [`crate`]'s documentation keeps
//! apart.
//!
//! # Why the constants live here
//!
//! The atom names, the profile identifier, and the structural tokens are
//! constants of this module rather than of the specification. The
//! specification reads them through their views and the encoder matches on
//! them, so changing a constant changes both at once. A test ties the bytes to
//! `contracts/profile.json`, and the negative controls change these constants
//! to show the injectivity proof depends on them.

use vstd::prelude::*;

mod interface;

pub use self::interface::{ATOM_NAMES, CanonicalIr, PolicyError, RuleError};

// The injectivity proof and its specification exist only for the verifier.
// `verus!` erases ghost items under plain `rustc`, so compiling these modules
// would leave their imports unused and `-D warnings` would fail the build.
// They are gated together for that reason, exactly as the settlement decision
// gates its specification, lemmas, and witnesses.
#[cfg(verus_keep_ghost)]
#[path = "../../../verus/policy_canonical_spec.rs"]
mod spec;

#[cfg(verus_keep_ghost)]
#[path = "../../../verus/policy_canonical_injective.rs"]
mod injective;

#[cfg(verus_keep_ghost)]
pub use self::injective::*;
#[cfg(verus_keep_ghost)]
pub use self::spec::*;

verus! {

broadcast use vstd::seq::group_seq_axioms;

// -- The executable constants ----------------------------------------------

// The constants are written in element form rather than as byte-string
// literals. Verus does not expose a byte-string-literal constant's elements to
// a specification: `*b"abc"@[0] == b'a'` is unprovable, while the same array
// written `[b'a', b'b', b'c']` reduces fully. Every proof of the canonical
// encoding rests on that, so the form is load-bearing and cannot be simplified.
/// The byte constants the encoder writes and the specification names.
pub mod tokens;

#[cfg(verus_keep_ghost)]
use self::tokens::*;

use self::tokens::{
    ADMIN_ROLE,
    COMMA,
    LBRACE,
    LBRACKET,
    LF,
    LOCKED,
    OWNS_RESOURCE,
    PROFILE,
    PROFILE_KEY,
    QUOTE,
    RBRACE,
    RBRACKET,
    RULES_KEY,
    SAME_TENANT,
    UNLOCKED,
    WRITE_ACTION,
};

/// The most rules a policy may hold.
///
/// Restates `contracts/profile.json`'s `rules.max`; a contract test compares
/// the two. It is declared here rather than imported so the kernel is
/// self-contained, which is what lets Verus compile it as its own crate.
pub const RULES_MAX: usize = 16;

/// The fewest atoms a rule body may hold.
pub const BODY_ATOMS_MIN: usize = 1;

/// The most atoms a rule body may hold.
pub const BODY_ATOMS_MAX: usize = 16;

/// One atom of the profile, and nothing else.
///
/// There are exactly six. The enum is not `#[non_exhaustive]`: adding an atom
/// must break every exhaustive match, because a new atom is a new profile with
/// a new semantic contract, not a compatible extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Atom {
    /// `input.subject.tenant == input.resource.tenant`
    SameTenant,
    /// `input.action == "write"`
    WriteAction,
    /// `input.subject.role == "admin"`
    AdminRole,
    /// `input.subject.id == input.resource.owner`
    OwnsResource,
    /// `input.resource.locked == true`
    Locked,
    /// `input.resource.locked == false`
    Unlocked,
}

impl Atom {
    /// The atom's name, which is what the canonical encoding writes.
    ///
    /// The specification reads the same constants through their views, so the
    /// proof and the encoder cannot disagree about a name.
    pub open spec fn name(self) -> Seq<u8> {
        match self {
            Atom::SameTenant => SAME_TENANT@,
            Atom::WriteAction => WRITE_ACTION@,
            Atom::AdminRole => ADMIN_ROLE@,
            Atom::OwnsResource => OWNS_RESOURCE@,
            Atom::Locked => LOCKED@,
            Atom::Unlocked => UNLOCKED@,
        }
    }
}

/// One rule body: a non-empty ordered sequence of atoms.
///
/// The field is private, so a value always satisfies the body bound. The
/// encoding order is the order the atoms are held in.
#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    /// The atoms, whose length is one to [`BODY_ATOMS_MAX`].
    atoms: Vec<Atom>,
}

impl Rule {
    /// A rule body, or `None` when `atoms` is empty.
    ///
    /// The bound is a type invariant: a value is only built here, and only
    /// from a non-empty vector.
    #[must_use]
    pub fn from_atoms(atoms: Vec<Atom>) -> (rule: Option<Self>)
        ensures
            rule is Some ==> rule->Some_0.atoms_spec() == atoms@,
            rule is None ==> atoms@.len() == 0,
    {
        if atoms.is_empty() {
            None
        } else {
            Some(Self { atoms })
        }
    }

    /// The atoms, in order.
    #[must_use]
    pub fn atoms_slice(&self) -> (atoms: &[Atom])
        ensures atoms@ == self.atoms_spec(),
    {
        &self.atoms
    }

    /// The atoms in order.
    ///
    /// Closed, so the field stays private: a `pub open` specification cannot
    /// read a private field, because it must be well formed everywhere. Proofs
    /// that need the definition call `reveal(Rule::atoms_spec)`.
    pub closed spec fn atoms_spec(self) -> Seq<Atom> { self.atoms@ }

    /// The rule's body, one atom per position.
    pub closed spec fn body(self) -> Seq<u8> {
        LBRACKET@ + prefix_atoms(self.atoms_spec(), self.atoms_spec().len() as int)
            + RBRACKET@
    }
}

/// One policy: an ordered sequence of zero to [`RULES_MAX`] rules.
#[verifier::allow(autoderive_clone_without_spec)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    /// The rules, whose length is zero to [`RULES_MAX`].
    rules: Vec<Rule>,
}

impl Policy {
    /// A policy over `rules`, whose length the caller has already bounded.
    #[must_use]
    pub const fn from_rules(rules: Vec<Rule>) -> (policy: Self)
        ensures policy.rules_spec() == rules@,
    {
        Self { rules }
    }

    /// The rules in order.
    ///
    /// Closed for the same reason as [`Rule::atoms_spec`].
    pub closed spec fn rules_spec(self) -> Seq<Rule> { self.rules@ }
}

/// Append `bytes` to `out`.
///
/// vstd's `extend_from_slice` promises only cloned elements, so the encoder
/// appends one byte at a time and the postcondition can name the exact
/// sequence. The loop is the proof: its invariant is the prefix already
/// written.
pub fn push_bytes(out: &mut Vec<u8>, bytes: &[u8])
    ensures final(out)@ == old(out)@ + bytes@
{
    let mut index = 0;
    while index < bytes.len()
        invariant
            index <= bytes@.len(),
            out@ == old(out)@ + bytes@.subrange(0, index as int),
        decreases bytes@.len() - index,
    {
        out.push(*vstd::slice::slice_index_get(bytes, index));
        index += 1;
    }
}

/// Append `atom`'s quoted name to `out`.
pub fn encode_atom(out: &mut Vec<u8>, atom: Atom)
    ensures final(out)@ == old(out)@ + quoted(atom)
{
    push_bytes(out, &QUOTE);
    match atom {
        Atom::SameTenant => push_bytes(out, &SAME_TENANT),
        Atom::WriteAction => push_bytes(out, &WRITE_ACTION),
        Atom::AdminRole => push_bytes(out, &ADMIN_ROLE),
        Atom::OwnsResource => push_bytes(out, &OWNS_RESOURCE),
        Atom::Locked => push_bytes(out, &LOCKED),
        Atom::Unlocked => push_bytes(out, &UNLOCKED),
    }
    push_bytes(out, &QUOTE);
}

/// Append `rule`'s body to `out`.
pub fn encode_body(out: &mut Vec<u8>, rule: &Rule)
    ensures final(out)@ == old(out)@ + rule.body()
{
    push_bytes(out, &LBRACKET);
    let mut index = 0;
    while index < rule.atoms.len()
        invariant
            index <= rule.atoms@.len(),
            out@ == old(out)@ + LBRACKET@ + prefix_atoms(rule.atoms@, index as int),
        decreases rule.atoms@.len() - index,
    {
        if index > 0 {
            push_bytes(out, &COMMA);
        }
        encode_atom(out, *vstd::slice::slice_index_get(&rule.atoms, index));
        index += 1;
    }
    push_bytes(out, &RBRACKET);
}

/// Append the header to `out`.
pub fn push_header(out: &mut Vec<u8>)
    ensures final(out)@ == old(out)@ + header()
{
    push_bytes(out, &LBRACE);
    push_bytes(out, &PROFILE_KEY);
    push_bytes(out, &QUOTE);
    push_bytes(out, &PROFILE);
    push_bytes(out, &QUOTE);
    push_bytes(out, &COMMA);
    push_bytes(out, &RULES_KEY);
    push_bytes(out, &LBRACKET);
}

/// Append the footer to `out`.
pub fn push_footer(out: &mut Vec<u8>)
    ensures final(out)@ == old(out)@ + footer()
{
    push_bytes(out, &RBRACKET);
    push_bytes(out, &RBRACE);
    push_bytes(out, &LF);
}

/// Append `policy`'s canonical bytes to `out`.
///
/// The postcondition is the specification verbatim: the encoder's whole
/// obligation is that its result is `render`. Nothing here is assumed; the
/// loop invariant is what discharges it.
pub fn encode(out: &mut Vec<u8>, policy: &Policy)
    ensures final(out)@ == old(out)@ + render(*policy)
{
    proof { reveal(Policy::rules_spec); }
    push_header(out);
    let mut index = 0;
    while index < policy.rules.len()
        invariant
            index <= policy.rules@.len(),
            out@ == old(out)@ + header() + prefix_rules(policy.rules@, index as int),
        decreases policy.rules@.len() - index,
    {
        if index > 0 {
            push_bytes(out, &COMMA);
        }
        encode_body(out, vstd::slice::slice_index_get(&policy.rules, index));
        index += 1;
    }
    push_footer(out);
}

/// The specification of the canonical byte form.
pub mod bytes;

// The specification items feed the encoder's postconditions, which plain
// `rustc` erases, so the re-export is meaningful to the verifier only.
#[cfg(verus_keep_ghost)]
pub use self::bytes::*;

} // verus!
