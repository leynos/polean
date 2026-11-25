//! The byte constants the encoder writes and the specification names.
//!
//! They are written in element form rather than as byte-string literals.
//! Verus does not expose a byte-string-literal constant's elements to a
//! specification: `*b"abc"@[0] == b'a'` is unprovable, while the same array
//! written `[b'a', b'b', b'c']` reduces fully. Every proof of the canonical
//! encoding rests on that, so the form is load-bearing.

#![expect(
    clippy::byte_char_slices,
    reason = "element-form arrays are visible to Verus, byte-string literals are not"
)]

use vstd::prelude::*;

verus! {

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

} // verus!
