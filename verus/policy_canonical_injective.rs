//! The injectivity proof for the canonical policy encoding.
//!
//! `render` is injective: distinct policies have distinct canonical bytes, so
//! hashing canonical bytes identifies exactly one policy. The route is a
//! specification-level parser that is a left inverse of `render`, so
//! `render(p) = render(p')` gives `p = p'` by applying the parser to both
//! sides and reading off the rule sequences.
//!
//! # Shape of the argument
//!
//! The atom names begin with six distinct letters — `s`, `w`, `a`, `o`, `l`, and
//! `u` — so for atoms `a != b`, the quoted forms differ at their second byte.
//! [`lemma_quoted_prefix_free`] states that, and it is what lets the parser
//! consume exactly one atom and hand back what follows. The body parser feeds
//! the rule parser the same way, because a body begins with an opening bracket
//! and the list's terminator is a closing bracket; the brackets are what keep
//! `[[a],[b]]` and `[[a,b]]` apart.
//!
//! # What this establishes
//!
//! Only that `render` is injective over `Policy` values. It says nothing about
//! Rego, admission, or a runtime.
//!
//! Everything here is ghost code, erased by `verus!` from the shipped library.

use vstd::prelude::*;

verus! {

use super::*;

/// `bytes` begins with `token`.
pub proof fn lemma_begins_self(bytes: Seq<u8>, token: Seq<u8>, rest: Seq<u8>)
    requires bytes == token + rest,
    ensures begins_with(bytes, token),
{
    assert(bytes.len() == token.len() + rest.len());
    assert(bytes.subrange(0, token.len() as int) == token);
}

/// The remainder after a token the sequence begins with.
pub proof fn lemma_after_self(token: Seq<u8>, rest: Seq<u8>)
    ensures after(token + rest, token) == rest,
{
    assert(token.len() <= (token + rest).len());
}

/// A sequence that begins with `token` does not begin with a token whose first
/// differing position falls inside both.
pub proof fn lemma_not_begins_of_differing(
    bytes: Seq<u8>,
    token: Seq<u8>,
    other: Seq<u8>,
    index: int,
)
    requires
        0 <= index < token.len(),
        index < other.len(),
        bytes.len() >= token.len(),
        token[index] != other[index],
        bytes.subrange(0, token.len() as int) == token,
    ensures !begins_with(bytes, other),
{
    if begins_with(bytes, other) {
        assert(bytes.subrange(0, other.len() as int) == other);
        assert(bytes[index] == token[index]);
        assert(bytes[index] == other[index]);
    }
}

/// The six names have six distinct first letters, so two different atoms never
/// have quoted forms where one begins at the other's start.
///
/// This is the property the parser relies on: a comma or a closing bracket
/// cannot begin a quoted name, and no quoted name begins another. The proof is
/// by cases on the two atoms, since the enum is closed and has six values.
pub proof fn lemma_quoted_prefix_free(a: Atom, b: Atom)
    requires a != b,
    ensures
        !(quoted(a).len() <= quoted(b).len()
            && quoted(b).subrange(0, quoted(a).len() as int) == quoted(a)),
        quoted(a) != quoted(b),
{
    // Each atom's second byte is its name's first letter, because the quote
    // occupies index 0 of the quoted form. Six distinct letters make every
    // pair distinguishable there, so no two quoted forms can begin alike. The
    // match is exhaustive over the closed enum, which is why the six atoms are
    // not `#[non_exhaustive]`: a seventh atom must break this proof.
    match a {
        Atom::SameTenant => match b {
            Atom::SameTenant => {},
            Atom::WriteAction => {
                assert(quoted(a)[1] == b's');
                assert(quoted(b)[1] == b'w');
            },
            Atom::AdminRole => {
                assert(quoted(a)[1] == b's');
                assert(quoted(b)[1] == b'a');
            },
            Atom::OwnsResource => {
                assert(quoted(a)[1] == b's');
                assert(quoted(b)[1] == b'o');
            },
            Atom::Locked => {
                assert(quoted(a)[1] == b's');
                assert(quoted(b)[1] == b'l');
            },
            Atom::Unlocked => {
                assert(quoted(a)[1] == b's');
                assert(quoted(b)[1] == b'u');
            },
        },
        Atom::WriteAction => match b {
            Atom::SameTenant => {
                assert(quoted(a)[1] == b'w');
                assert(quoted(b)[1] == b's');
            },
            Atom::WriteAction => {},
            Atom::AdminRole => {
                assert(quoted(a)[1] == b'w');
                assert(quoted(b)[1] == b'a');
            },
            Atom::OwnsResource => {
                assert(quoted(a)[1] == b'w');
                assert(quoted(b)[1] == b'o');
            },
            Atom::Locked => {
                assert(quoted(a)[1] == b'w');
                assert(quoted(b)[1] == b'l');
            },
            Atom::Unlocked => {
                assert(quoted(a)[1] == b'w');
                assert(quoted(b)[1] == b'u');
            },
        },
        Atom::AdminRole => match b {
            Atom::SameTenant => {
                assert(quoted(a)[1] == b'a');
                assert(quoted(b)[1] == b's');
            },
            Atom::WriteAction => {
                assert(quoted(a)[1] == b'a');
                assert(quoted(b)[1] == b'w');
            },
            Atom::AdminRole => {},
            Atom::OwnsResource => {
                assert(quoted(a)[1] == b'a');
                assert(quoted(b)[1] == b'o');
            },
            Atom::Locked => {
                assert(quoted(a)[1] == b'a');
                assert(quoted(b)[1] == b'l');
            },
            Atom::Unlocked => {
                assert(quoted(a)[1] == b'a');
                assert(quoted(b)[1] == b'u');
            },
        },
        Atom::OwnsResource => match b {
            Atom::SameTenant => {
                assert(quoted(a)[1] == b'o');
                assert(quoted(b)[1] == b's');
            },
            Atom::WriteAction => {
                assert(quoted(a)[1] == b'o');
                assert(quoted(b)[1] == b'w');
            },
            Atom::AdminRole => {
                assert(quoted(a)[1] == b'o');
                assert(quoted(b)[1] == b'a');
            },
            Atom::OwnsResource => {},
            Atom::Locked => {
                assert(quoted(a)[1] == b'o');
                assert(quoted(b)[1] == b'l');
            },
            Atom::Unlocked => {
                assert(quoted(a)[1] == b'o');
                assert(quoted(b)[1] == b'u');
            },
        },
        Atom::Locked => match b {
            Atom::SameTenant => {
                assert(quoted(a)[1] == b'l');
                assert(quoted(b)[1] == b's');
            },
            Atom::WriteAction => {
                assert(quoted(a)[1] == b'l');
                assert(quoted(b)[1] == b'w');
            },
            Atom::AdminRole => {
                assert(quoted(a)[1] == b'l');
                assert(quoted(b)[1] == b'a');
            },
            Atom::OwnsResource => {
                assert(quoted(a)[1] == b'l');
                assert(quoted(b)[1] == b'o');
            },
            Atom::Locked => {},
            Atom::Unlocked => {
                assert(quoted(a)[1] == b'l');
                assert(quoted(b)[1] == b'u');
            },
        },
        Atom::Unlocked => match b {
            Atom::SameTenant => {
                assert(quoted(a)[1] == b'u');
                assert(quoted(b)[1] == b's');
            },
            Atom::WriteAction => {
                assert(quoted(a)[1] == b'u');
                assert(quoted(b)[1] == b'w');
            },
            Atom::AdminRole => {
                assert(quoted(a)[1] == b'u');
                assert(quoted(b)[1] == b'a');
            },
            Atom::OwnsResource => {
                assert(quoted(a)[1] == b'u');
                assert(quoted(b)[1] == b'o');
            },
            Atom::Locked => {
                assert(quoted(a)[1] == b'u');
                assert(quoted(b)[1] == b'l');
            },
            Atom::Unlocked => {},
        },
    }
}

/// `parse_quoted` reads `quoted(atom)` from the front of `quoted(atom) + rest`.
///
/// The two claims are that the right alternative is taken, which needs
/// prefix-freeness, and that what follows is exactly `rest`.
pub proof fn lemma_parse_quoted_self(atom: Atom, rest: Seq<u8>)
    ensures parse_quoted(quoted(atom) + rest) == Some((atom, rest)),
{
    lemma_begins_self(quoted(atom) + rest, quoted(atom), rest);

    // No earlier alternative can match: each names a different atom, and
    // prefix-freeness rules it out.
    match atom {
        Atom::SameTenant => {},
        Atom::WriteAction => {
            lemma_not_begins_for(quoted(Atom::WriteAction) + rest, Atom::WriteAction, rest, Atom::SameTenant);
        },
        Atom::AdminRole => {
            lemma_not_begins_for(quoted(Atom::AdminRole) + rest, Atom::AdminRole, rest, Atom::SameTenant);
            lemma_not_begins_for(quoted(Atom::AdminRole) + rest, Atom::AdminRole, rest, Atom::WriteAction);
        },
        Atom::OwnsResource => {
            lemma_not_begins_for(quoted(Atom::OwnsResource) + rest, Atom::OwnsResource, rest, Atom::SameTenant);
            lemma_not_begins_for(quoted(Atom::OwnsResource) + rest, Atom::OwnsResource, rest, Atom::WriteAction);
            lemma_not_begins_for(quoted(Atom::OwnsResource) + rest, Atom::OwnsResource, rest, Atom::AdminRole);
        },
        Atom::Locked => {
            lemma_not_begins_for(quoted(Atom::Locked) + rest, Atom::Locked, rest, Atom::SameTenant);
            lemma_not_begins_for(quoted(Atom::Locked) + rest, Atom::Locked, rest, Atom::WriteAction);
            lemma_not_begins_for(quoted(Atom::Locked) + rest, Atom::Locked, rest, Atom::AdminRole);
            lemma_not_begins_for(quoted(Atom::Locked) + rest, Atom::Locked, rest, Atom::OwnsResource);
        },
        Atom::Unlocked => {
            lemma_not_begins_for(quoted(Atom::Unlocked) + rest, Atom::Unlocked, rest, Atom::SameTenant);
            lemma_not_begins_for(quoted(Atom::Unlocked) + rest, Atom::Unlocked, rest, Atom::WriteAction);
            lemma_not_begins_for(quoted(Atom::Unlocked) + rest, Atom::Unlocked, rest, Atom::AdminRole);
            lemma_not_begins_for(quoted(Atom::Unlocked) + rest, Atom::Unlocked, rest, Atom::OwnsResource);
            lemma_not_begins_for(quoted(Atom::Unlocked) + rest, Atom::Unlocked, rest, Atom::Locked);
        },
    }

    lemma_after_self(quoted(atom), rest);
}

/// `bytes` does not begin with `other`, when `bytes` begins with a different
/// quoted atom.
///
/// This is the "wrong alternative" case: the parser tries the atoms in a fixed
/// order, and each earlier atom must be ruled out. The two quoted forms differ
/// at index 1, and both are long enough to reach it, so `bytes` cannot begin
/// both.
pub proof fn lemma_not_begins_for(bytes: Seq<u8>, atom: Atom, rest: Seq<u8>, other: Atom)
    requires
        atom != other,
        bytes == quoted(atom) + rest,
    ensures !begins_with(bytes, quoted(other)),
{
    lemma_quoted_prefix_free(atom, other);
    // The quoted forms differ at index 1, and index 1 is inside both.
    assert(quoted(atom)[1] != quoted(other)[1]);
    assert(quoted(atom).len() > 1);
    assert(quoted(other).len() > 1);
    assert(bytes.subrange(0, quoted(atom).len() as int) == quoted(atom));
    lemma_not_begins_of_differing(
        bytes,
        quoted(atom),
        quoted(other),
        1,
    );
}

} // verus!
