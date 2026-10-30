//! Scalar readers with a fixed expected JSON kind.
//!
//! Each reader reports [`ReasonCode::AstType`] for a value of another kind, so
//! a decoder that expects a string never silently accepts a number. Numbers
//! are read two ways: [`as_number`] for the frontend's indices, which must fit
//! a 32-bit unsigned integer, and [`as_number_text`] for literal values, which
//! keeps the frontend's serialized spelling because no decision may re-round a
//! number.

use serde_json::Value as Json;

use super::{Fault, type_name};
use crate::corpus::ReasonCode;

/// Read a JSON array.
///
/// # Errors
///
/// Returns a fault when the value is not an array.
pub fn as_array(value: &Json) -> Result<&[Json], Fault> {
    value.as_array().map(Vec::as_slice).ok_or_else(|| {
        Fault::new(
            ReasonCode::AstType,
            format!("expected an array, found {}", type_name(value)),
        )
    })
}

/// Read a JSON string.
///
/// # Errors
///
/// Returns a fault when the value is not a string.
pub fn as_string(value: &Json) -> Result<&str, Fault> {
    value.as_str().ok_or_else(|| {
        Fault::new(
            ReasonCode::AstType,
            format!("expected a string, found {}", type_name(value)),
        )
    })
}

/// Read a JSON boolean.
///
/// # Errors
///
/// Returns a fault when the value is not a boolean.
pub fn as_boolean(value: &Json) -> Result<bool, Fault> {
    value.as_bool().ok_or_else(|| {
        Fault::new(
            ReasonCode::AstType,
            format!("expected a boolean, found {}", type_name(value)),
        )
    })
}

/// Read a JSON number that fits a 32-bit index.
///
/// # Errors
///
/// Returns a fault when the value is not a number or does not fit.
pub fn as_number(value: &Json) -> Result<u32, Fault> {
    let number = value.as_u64().ok_or_else(|| {
        Fault::new(
            ReasonCode::AstType,
            format!(
                "expected a non-negative integer, found {}",
                type_name(value)
            ),
        )
    })?;
    u32::try_from(number).map_err(|_| {
        Fault::new(
            ReasonCode::AstType,
            format!("`{number}` does not fit a 32-bit index"),
        )
    })
}

/// Read a JSON number as the frontend's serialized spelling.
///
/// # Errors
///
/// Returns a fault when the value is not a number.
pub fn as_number_text(value: &Json) -> Result<String, Fault> {
    value.as_number().map(ToString::to_string).ok_or_else(|| {
        Fault::new(
            ReasonCode::AstType,
            format!("expected a number, found {}", type_name(value)),
        )
    })
}
