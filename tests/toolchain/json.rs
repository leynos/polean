//! JSON accessors for the Regorus AST observations.
//!
//! Every accessor either returns the requested value or a diagnostic naming
//! the context and the field it expected, so a failure says which part of the
//! AST differed rather than only that something did. `panic_in_result_fn`
//! forbids the assertion macros in a `Result`-returning function, so the
//! checks that use these accessors return diagnostics too.

use serde_json::{Map, Value as Json};

/// Parse JSON text, naming the source when it is malformed.
///
/// # Errors
///
/// Returns a diagnostic when `text` is not valid JSON.
pub fn parse_json(text: &str, source: &str) -> Result<Json, String> {
    serde_json::from_str(text).map_err(|error| format!("{source} is not valid JSON: {error}"))
}

/// A JSON object, or a diagnostic when the value is another shape.
///
/// # Errors
///
/// Returns a diagnostic when `value` is not an object.
pub fn object<'a>(value: &'a Json, context: &str) -> Result<&'a Map<String, Json>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{context}: expected a JSON object"))
}

/// A named JSON field, or a diagnostic naming the missing field.
///
/// # Errors
///
/// Returns a diagnostic when `value` has no field called `name`.
pub fn field<'a>(value: &'a Json, name: &str, context: &str) -> Result<&'a Json, String> {
    value
        .get(name)
        .ok_or_else(|| format!("{context}: missing field `{name}`"))
}

/// A named JSON array, or a diagnostic naming the wrong type.
///
/// # Errors
///
/// Returns a diagnostic when the field is absent or is not an array.
pub fn array<'a>(value: &'a Json, name: &str, context: &str) -> Result<&'a Vec<Json>, String> {
    field(value, name, context)?
        .as_array()
        .ok_or_else(|| format!("{context}: field `{name}` is not an array"))
}

/// A named JSON string, or a diagnostic naming the wrong type.
///
/// # Errors
///
/// Returns a diagnostic when the field is absent or is not a string.
pub fn string<'a>(value: &'a Json, name: &str, context: &str) -> Result<&'a str, String> {
    field(value, name, context)?
        .as_str()
        .ok_or_else(|| format!("{context}: field `{name}` is not a string"))
}

/// A named JSON Boolean, or a diagnostic naming the wrong type.
///
/// # Errors
///
/// Returns a diagnostic when the field is absent or is not a Boolean.
pub fn boolean(value: &Json, name: &str, context: &str) -> Result<bool, String> {
    field(value, name, context)?
        .as_bool()
        .ok_or_else(|| format!("{context}: field `{name}` is not a boolean"))
}

/// The single tag key of a tagged object, such as a rule or a rule head.
///
/// `kind` names the tagged object in diagnostics, for example `rule` or
/// `rule head`.
///
/// # Errors
///
/// Returns a diagnostic when `value` is not an object or does not have exactly
/// one key.
pub fn single_tag<'a>(value: &'a Json, kind: &str, context: &str) -> Result<&'a str, String> {
    let keys = object(value, context)?;
    if keys.len() == 1 {
        return keys
            .keys()
            .next()
            .map(String::as_str)
            .ok_or_else(|| format!("{context}: {kind} has no tag"));
    }
    Err(format!(
        "{context}: expected exactly one {kind} tag, found {}",
        keys.len()
    ))
}

/// Require an observed value to equal the expected one, naming the context.
///
/// # Errors
///
/// Returns a diagnostic naming `context` when the values differ.
pub fn expect_equal<T>(actual: &T, expected: &T, context: &str) -> Result<(), String>
where
    T: PartialEq + std::fmt::Debug,
{
    if actual == expected {
        return Ok(());
    }
    Err(format!(
        "{context}: expected {expected:?}, found {actual:?}"
    ))
}
