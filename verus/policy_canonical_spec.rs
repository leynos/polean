//! The specification the canonical-encoding proofs reason about.
//!
//! `render` in the kernel is the specification of the bytes the encoder must
//! produce; this file supplies the parser that is its left inverse, and the
//! lemmas that let the two be related. Everything here is ghost code.
//!
//! # Prefix-freeness
//!
//! The six atom names begin with six distinct letters — `s`, `w`, `a`, `o`,
//! `l`, and `u` — so for atoms `a != b`, the quoted forms `q(a)` and `q(b)`
//! differ at their second byte. No quoted name is a prefix of another, and the
//! quotes are not what makes the set prefix-free. That is what lets the parser
//! consume exactly one atom and return what follows.
//!
//! # Shape
//!
//! `parse_quoted` reads one quoted atom; `parse_atoms` reads a comma-separated
//! run of them; `parse_body` wraps that in brackets; `parse_rules` reads a
//! comma-separated run of bodies; and `parse_render` strips the fixed header
//! and requires the fixed footer. `parse_render(render(p)) = Some(p)` for every
//! policy, so `render` is injective.

use vstd::prelude::*;

verus! {

use super::*;

/// Whether `bytes` begins with `token`.
pub open spec fn begins_with(bytes: Seq<u8>, token: Seq<u8>) -> bool {
    bytes.len() >= token.len() && bytes.subrange(0, token.len() as int) == token
}

/// The byte after `token`, when `bytes` begins with `token`.
pub open spec fn after(bytes: Seq<u8>, token: Seq<u8>) -> Seq<u8> {
    bytes.subrange(token.len() as int, bytes.len() as int)
}

/// One quoted atom at the front of `bytes`, if any.
///
/// The order of the alternatives does not matter: no two quoted forms can both
/// match, because the names are prefix-free.
pub open spec fn parse_quoted(bytes: Seq<u8>) -> Option<(Atom, Seq<u8>)> {
    if begins_with(bytes, quoted(Atom::SameTenant)) {
        Some((Atom::SameTenant, after(bytes, quoted(Atom::SameTenant))))
    } else if begins_with(bytes, quoted(Atom::WriteAction)) {
        Some((Atom::WriteAction, after(bytes, quoted(Atom::WriteAction))))
    } else if begins_with(bytes, quoted(Atom::AdminRole)) {
        Some((Atom::AdminRole, after(bytes, quoted(Atom::AdminRole))))
    } else if begins_with(bytes, quoted(Atom::OwnsResource)) {
        Some((Atom::OwnsResource, after(bytes, quoted(Atom::OwnsResource))))
    } else if begins_with(bytes, quoted(Atom::Locked)) {
        Some((Atom::Locked, after(bytes, quoted(Atom::Locked))))
    } else if begins_with(bytes, quoted(Atom::Unlocked)) {
        Some((Atom::Unlocked, after(bytes, quoted(Atom::Unlocked))))
    } else {
        None
    }
}

/// `count` quoted atoms separated by commas, at the front of `bytes`.
pub open spec fn parse_atoms(bytes: Seq<u8>, count: int) -> Option<(Seq<Atom>, Seq<u8>)>
    decreases bytes.len(),
{
    if count <= 0 {
        Some((seq![], bytes))
    } else {
        match parse_quoted(bytes) {
            None => None,
            Some((atom, rest)) => {
                if count == 1 {
                    Some((seq![atom], rest))
                } else if rest.len() > 0 && rest[0] == COMMA@[0] {
                    match parse_atoms(rest.subrange(1, rest.len() as int), count - 1) {
                        None => None,
                        Some((atoms, tail)) => Some((seq![atom] + atoms, tail)),
                    }
                } else {
                    None
                }
            },
        }
    }
}

/// A body at the front of `bytes`: an opening bracket, `count` quoted atoms,
/// and a closing bracket.
pub open spec fn parse_body(bytes: Seq<u8>, count: int) -> Option<(Seq<Atom>, Seq<u8>)>
    decreases bytes.len(),
{
    if begins_with(bytes, LBRACKET@) {
        match parse_atoms(after(bytes, LBRACKET@), count) {
            None => None,
            Some((atoms, rest)) => {
                if begins_with(rest, RBRACKET@) {
                    Some((atoms, after(rest, RBRACKET@)))
                } else {
                    None
                }
            },
        }
    } else {
        None
    }
}

} // verus!
