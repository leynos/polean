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

verus! {

broadcast use vstd::seq::group_seq_axioms;

// -- The executable constants ----------------------------------------------

// The constants are written in element form rather than as byte-string
// literals. Verus does not expose a byte-string-literal constant's elements to
// a specification: `*b"abc"@[0] == b'a'` is unprovable, while the same array
// written `[b'a', b'b', b'c']` reduces fully. Every proof of the canonical
// encoding rests on that, so the form is load-bearing and cannot be simplified.
#[expect(
    clippy::byte_char_slices,
    reason = "element-form arrays are visible to Verus, byte-string literals are not"
)]
mod tokens {
    //! The byte constants the encoder writes and the specification names.
    //!
    //! Element form is load-bearing; see the comment on the module.

    /// The profile identifier, as the encoder writes it between its quotes.
    pub const PROFILE: [u8; 22] = [
        b'p', b'o', b'l', b'e', b'a', b'n', b'.', b't', b'e', b'n', b'a', b'n', b't', b'-', b'w',
        b'r', b'i', b't', b'e', b'.', b'v', b'0',
    ];

    /// The name of `same_tenant`, as the encoder writes it between its quotes.
    pub const SAME_TENANT: [u8; 11] =
        [b's', b'a', b'm', b'e', b'_', b't', b'e', b'n', b'a', b'n', b't'];

    /// The name of `write_action`.
    pub const WRITE_ACTION: [u8; 12] =
        [b'w', b'r', b'i', b't', b'e', b'_', b'a', b'c', b't', b'i', b'o', b'n'];

    /// The name of `admin_role`.
    pub const ADMIN_ROLE: [u8; 10] =
        [b'a', b'd', b'm', b'i', b'n', b'_', b'r', b'o', b'l', b'e'];

    /// The name of `owns_resource`.
    pub const OWNS_RESOURCE: [u8; 13] = [
        b'o', b'w', b'n', b's', b'_', b'r', b'e', b's', b'o', b'u', b'r', b'c', b'e',
    ];

    /// The name of `locked`.
    pub const LOCKED: [u8; 6] = [b'l', b'o', b'c', b'k', b'e', b'd'];

    /// The name of `unlocked`.
    pub const UNLOCKED: [u8; 8] = [b'u', b'n', b'l', b'o', b'c', b'k', b'e', b'd'];

    /// The opening brace of the document object.
    pub const LBRACE: [u8; 1] = [b'{'];

    /// The closing brace of the document object.
    pub const RBRACE: [u8; 1] = [b'}'];

    /// The opening bracket of the rule list and of a rule body.
    pub const LBRACKET: [u8; 1] = [b'['];

    /// The closing bracket of the rule list and of a rule body.
    pub const RBRACKET: [u8; 1] = [b']'];

    /// The separator between two rules or two atoms.
    pub const COMMA: [u8; 1] = [b','];

    /// The quote that delimits every string the encoder writes.
    pub const QUOTE: [u8; 1] = [b'"'];

    /// The key `"profile":` including its quotes and colon.
    pub const PROFILE_KEY: [u8; 10] =
        [b'"', b'p', b'r', b'o', b'f', b'i', b'l', b'e', b'"', b':'];

    /// The key `"rules":` including its quotes and colon.
    pub const RULES_KEY: [u8; 8] = [b'"', b'r', b'u', b'l', b'e', b's', b'"', b':'];

    /// The line feed the document ends with.
    pub const LF: [u8; 1] = [b'\n'];
}

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

/// The first `count` atoms of `atoms`, each quoted, separated by commas.
///
/// This is written in append form, `prefix(count + 1) = prefix(count) ++ "," ++
/// quoted(atoms[count])`, because that is the shape the encoder's loop builds.
/// A definition written as a fold from the front would need a bridging lemma to
/// relate the two views; writing it this way makes the loop invariant the
/// definition itself.
pub open spec fn prefix_atoms(atoms: Seq<Atom>, count: int) -> Seq<u8>
    decreases count,
{
    if count <= 0 {
        seq![]
    } else if count == 1 {
        quoted(atoms[0])
    } else {
        prefix_atoms(atoms, count - 1) + COMMA@ + quoted(atoms[count - 1])
    }
}

/// One atom's name wrapped in quotes.
pub open spec fn quoted(atom: Atom) -> Seq<u8> { QUOTE@ + atom.name() + QUOTE@ }

/// The first `count` bodies of `rules`, each bracketed, separated by commas.
pub open spec fn prefix_rules(rules: Seq<Rule>, count: int) -> Seq<u8>
    decreases count,
{
    if count <= 0 {
        seq![]
    } else if count == 1 {
        rules[0].body()
    } else {
        prefix_rules(rules, count - 1) + COMMA@ + rules[count - 1].body()
    }
}

/// The canonical bytes of `policy`, exactly as the encoder must produce them.
///
/// The specification is total over every sequence of rules, well formed or
/// not, so the uniqueness theorem quantifies over the whole type rather than
/// only the values the constructors admit.
pub open spec fn render(policy: Policy) -> Seq<u8> {
    header() + prefix_rules(policy.rules_spec(), policy.rules_spec().len() as int) + footer()
}

/// The bytes before the rule list: the opening object, both keys, and the
/// list's opening bracket.
pub open spec fn header() -> Seq<u8> {
    LBRACE@ + PROFILE_KEY@ + QUOTE@ + PROFILE@ + QUOTE@ + COMMA@ + RULES_KEY@ + LBRACKET@
}

/// The bytes after the rule list: the list's closing bracket, the object's
/// closing brace, and the final line feed.
pub open spec fn footer() -> Seq<u8> { RBRACKET@ + RBRACE@ + LF@ }

} // verus!

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
