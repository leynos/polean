//! Name components and the injective byte key settlement compares.
//!
//! A Lean name is a sequence of components, each text or a number, because
//! private names contain numeric components. The key is injective: a tag
//! distinguishes text from number, and every variable-length part is preceded
//! by its length in bytes. Two distinct names therefore never share a key,
//! which is what lets settlement compare keys with `==` and mean name
//! equality.
//!
//! Settlement treats keys as opaque. This module is the only place a name's
//! structure is interpreted, apart from the `native` classification, which
//! selects a diagnostic and never an acceptance decision.

/// One component of a Lean name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Component {
    /// A textual component, which may be empty in a private name.
    Text(String),
    /// A numeric component, as a private name uses.
    Number(u64),
}

/// A Lean name as the worker reports it.
pub type Name = Vec<Component>;

/// The tag that starts a name key.
const NAME_TAG: u8 = 0x4e;

/// The tag that starts a textual component.
const TEXT_TAG: u8 = 0x53;

/// The tag that starts a numeric component.
const NUMBER_TAG: u8 = 0x49;

/// The prefix a native-computation axiom's component begins with.
const NATIVE_PREFIX: &str = "_native";

/// Encode a name as an injective byte key.
///
/// The encoding is a tag, the component count, then each component as its tag
/// and either a length-prefixed byte string or a little-endian `u64`. Counts
/// are `u64` so no conversion from `usize` can truncate on any supported host.
#[expect(
    clippy::little_endian_bytes,
    reason = "the key encoding is defined as little-endian so it is identical on every host"
)]
#[must_use]
pub fn name_key(name: &[Component]) -> Vec<u8> {
    let mut key = Vec::new();
    key.push(NAME_TAG);
    key.extend_from_slice(&(name.len() as u64).to_le_bytes());
    for component in name {
        match component {
            Component::Text(text) => {
                key.push(TEXT_TAG);
                key.extend_from_slice(&(text.len() as u64).to_le_bytes());
                key.extend_from_slice(text.as_bytes());
            }
            Component::Number(number) => {
                key.push(NUMBER_TAG);
                key.extend_from_slice(&number.to_le_bytes());
            }
        }
    }
    key
}

/// Whether any component of `name` begins with the native-computation prefix.
///
/// The flag only chooses between two rejection diagnostics, so a defect here
/// cannot admit an axiom: acceptance rests on the approved-foundation table.
#[must_use]
pub fn is_native(name: &[Component]) -> bool {
    name.iter().any(|component| match component {
        Component::Text(text) => text.starts_with(NATIVE_PREFIX),
        Component::Number(_) => false,
    })
}
