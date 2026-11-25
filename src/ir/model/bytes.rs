//! The specification of the canonical byte form, inside `verus!`.
//!
//! `render` is what the encoder's postcondition names and what the injectivity
//! proof parses. It is written in append form, `prefix(n) = prefix(n - 1) ++
//! separator ++ element(n)`, because that is the shape the encoder's loop
//! builds: a definition folded from the front would need a bridging lemma to
//! relate the two views, and writing it this way makes the loop invariant the
//! definition itself.

use vstd::prelude::*;

verus! {

// These imports exist only for the verifier: `verus!` erases the specification
// items under plain `rustc`, so the names would otherwise be unused and
// `-D warnings` would fail the build. The settlement decision gates its
// specification imports the same way.
#[cfg(verus_keep_ghost)]
use super::*;
#[cfg(verus_keep_ghost)]
use super::{Atom, Policy, Rule};

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
