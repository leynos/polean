//! The closed object and variant readers every decoded node goes through.
//!
//! Two tables drive decoding. A [`Field`] names one member of a fixed JSON
//! object, the JSON kind it must have, and whether the frontend always emits
//! it. A [`Variant`] names one tag of an externally tagged object and the
//! fields that tag's body must have. [`object`] and [`variant`] are the only
//! entry points, so every object in the tree is checked for unknown keys,
//! missing required keys, wrong JSON kinds, and tags outside the table in one
//! place.
//!
//! [`scalars`] reads a value of one expected kind, and [`spans`] validates a
//! reported range against the submitted text: a range that is reversed, past
//! the end of the source, or off a UTF-8 boundary is a decoding fault rather
//! than a position a later stage might trust.

mod scalars;
mod spans;

pub use scalars::{as_array, as_boolean, as_number, as_number_text, as_string};
use serde_json::{Map, Value as Json};
pub use spans::{nullable_span, span};

use crate::corpus::ReasonCode;

/// A decoding fault: the reason code to report and a reviewable description.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fault {
    /// The reason code the decoder reports.
    pub code: ReasonCode,
    /// A description of the first failed check.
    pub message: String,
}

impl Fault {
    /// Build a fault from a reason code and a description.
    #[must_use]
    pub fn new(code: ReasonCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Render the fault as `code: description`.
    #[must_use]
    pub fn describe(&self) -> String { format!("{}: {}", self.code.as_wire(), self.message) }
}

/// The JSON kind a field must have.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    /// JSON null.
    Null,
    /// A boolean.
    Bool,
    /// A number.
    Number,
    /// A string.
    String,
    /// An array.
    Array,
    /// An object.
    Object,
    /// JSON null or an object.
    NullOrObject,
}

impl Kind {
    /// Whether `value` has this kind.
    #[must_use]
    fn accepts(self, value: &Json) -> bool {
        match self {
            Self::Null => value.is_null(),
            Self::Bool => value.is_boolean(),
            Self::Number => value.is_number(),
            Self::String => value.is_string(),
            Self::Array => value.is_array(),
            Self::Object => value.is_object(),
            Self::NullOrObject => value.is_null() || value.is_object(),
        }
    }
}

/// Whether a field may be absent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Presence {
    /// The frontend always emits the field.
    Required,
    /// The frontend omits the field when it has nothing to say.
    Optional,
}

/// One member of a fixed JSON object shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Field {
    /// The member's name, as the frontend spells it.
    pub name: &'static str,
    /// The JSON kind the member must have.
    pub kind: Kind,
    /// Whether the member may be absent.
    pub presence: Presence,
}

/// One tag of an externally tagged object and the fields its body must have.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Variant {
    /// The variant's serialized tag.
    pub tag: &'static str,
    /// The fields of the variant's body.
    pub fields: &'static [Field],
}

/// Declare a required field.
#[must_use]
pub const fn req(name: &'static str, kind: Kind) -> Field {
    Field {
        name,
        kind,
        presence: Presence::Required,
    }
}

/// Declare a field the frontend may omit.
#[must_use]
pub const fn opt(name: &'static str, kind: Kind) -> Field {
    Field {
        name,
        kind,
        presence: Presence::Optional,
    }
}

/// Declare a variant.
#[must_use]
pub const fn variant_of(tag: &'static str, fields: &'static [Field]) -> Variant {
    Variant { tag, fields }
}

/// An object whose keys and value kinds have been checked.
#[derive(Clone, Copy, Debug)]
pub struct Fields<'a> {
    /// The matched tag, when the object was externally tagged.
    tag: Option<&'static str>,
    /// The checked object.
    map: &'a Map<String, Json>,
}

impl<'a> Fields<'a> {
    /// The matched tag, when the object was externally tagged.
    #[must_use]
    pub const fn tag(self) -> Option<&'static str> { self.tag }

    /// A member the table declared required.
    ///
    /// The table has already accepted the object, so a failure here means the
    /// decoder's own tables disagree; it is reported as a decoding fault so the
    /// failure is closed rather than a panic.
    ///
    /// # Errors
    ///
    /// Returns a fault when the table and the caller disagree about the shape.
    pub fn value(self, name: &str) -> Result<&'a Json, Fault> {
        self.map.get(name).ok_or_else(|| {
            Fault::new(
                ReasonCode::AstMissingKey,
                format!("the decoder read `{name}` but its table did not require it"),
            )
        })
    }

    /// A member the table declared optional, whether or not it is present.
    #[must_use]
    pub fn optional(self, name: &str) -> Option<&'a Json> { self.map.get(name) }
}

/// Read a JSON object against a fixed field table.
///
/// # Errors
///
/// Returns a fault for a non-object value, an unknown member, a missing
/// required member, or a member of the wrong JSON kind.
pub fn object<'a>(value: &'a Json, fields: &'static [Field]) -> Result<Fields<'a>, Fault> {
    let map = as_object(value)?;
    check(map, fields)?;
    Ok(Fields { tag: None, map })
}

/// Read an externally tagged object against a fixed variant table.
///
/// The frontend's enums are externally tagged, so the object must hold exactly
/// one key: the matched variant's tag, whose value is the body object. The body
/// is checked against the variant's field table and returned as the [`Fields`],
/// so callers read the body's members, not the tag's siblings.
///
/// # Errors
///
/// Returns a fault for a non-object value, no variant tag, more than one
/// variant tag, a key beside the tag, a body that is not an object, or a body
/// member that is unknown, missing, or of the wrong JSON kind.
pub fn variant<'a>(value: &'a Json, variants: &'static [Variant]) -> Result<Fields<'a>, Fault> {
    let map = as_object(value)?;
    let mut matched = variants
        .iter()
        .filter(|candidate| map.contains_key(candidate.tag));
    let Some(found) = matched.next() else {
        return Err(Fault::new(
            ReasonCode::AstUnknownTag,
            format!(
                "no tag names a variant: {:?}",
                map.keys().collect::<Vec<_>>()
            ),
        ));
    };
    if matched.next().is_some() {
        return Err(Fault::new(
            ReasonCode::AstUnknownTag,
            "more than one tag names a variant",
        ));
    }
    if map.len() != 1 {
        return Err(Fault::new(
            ReasonCode::AstUnknownKey,
            "a tagged object holds a key beside its tag",
        ));
    }
    let raw = map
        .get(found.tag)
        .map_or_else(|| Err(untagged(found.tag)), Ok)?;
    let body = as_object(raw)?;
    check(body, found.fields)?;
    Ok(Fields {
        tag: Some(found.tag),
        map: body,
    })
}

/// The fault for a tag whose body has vanished between matching and reading.
fn untagged(tag: &str) -> Fault {
    Fault::new(
        ReasonCode::AstUnknownTag,
        format!("the matched tag `{tag}` has no body"),
    )
}

/// Check one object's members against a field table.
fn check(map: &Map<String, Json>, fields: &'static [Field]) -> Result<(), Fault> {
    for key in map.keys() {
        if !fields.iter().any(|field| field.name == key) {
            return Err(Fault::new(
                ReasonCode::AstUnknownKey,
                format!("`{key}` is not a field of this object"),
            ));
        }
    }
    for field in fields {
        match map.get(field.name) {
            Some(value) => {
                if !field.kind.accepts(value) {
                    return Err(Fault::new(
                        ReasonCode::AstType,
                        format!("`{}` has the wrong JSON kind", field.name),
                    ));
                }
            }
            None => {
                if field.presence == Presence::Required {
                    return Err(Fault::new(
                        ReasonCode::AstMissingKey,
                        format!("`{}` is missing", field.name),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Read a JSON object without a field table.
///
/// # Errors
///
/// Returns a fault when the value is not an object.
fn as_object(value: &Json) -> Result<&Map<String, Json>, Fault> {
    value.as_object().ok_or_else(|| {
        Fault::new(
            ReasonCode::AstType,
            format!("expected an object, found {}", type_name(value)),
        )
    })
}

/// Describe a JSON value's kind for a diagnostic.
const fn type_name(value: &Json) -> &'static str {
    match value {
        Json::Null => "null",
        Json::Bool(_) => "a boolean",
        Json::Number(_) => "a number",
        Json::String(_) => "a string",
        Json::Array(_) => "an array",
        Json::Object(_) => "an object",
    }
}
