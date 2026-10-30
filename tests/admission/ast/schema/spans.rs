//! Span readers.
//!
//! A span is a fixed four-number object. Its range is checked against the
//! submitted text here, once, so no later stage has to wonder whether a
//! reported position is usable.

use serde_json::Value as Json;

use super::{Fault, Field, Kind, as_number, object, req};
use crate::{ast::Span, corpus::ReasonCode};

/// Read a span object.
///
/// # Errors
///
/// Returns a fault when the object is malformed or the range is not a range
/// inside `source` on character boundaries.
pub fn span(value: &Json, source: &str) -> Result<Span, Fault> {
    let fields = object(value, SPAN)?;
    let span = Span::new(
        as_number(fields.value("line")?)?,
        as_number(fields.value("col")?)?,
        as_number(fields.value("start")?)?,
        as_number(fields.value("end")?)?,
    );
    check_span(span, source)?;
    Ok(span)
}

/// Read a span object that the frontend may have written as null.
///
/// # Errors
///
/// Returns a fault when the value is neither null nor a valid span.
pub fn nullable_span(value: &Json, source: &str) -> Result<Option<Span>, Fault> {
    if value.is_null() {
        return Ok(None);
    }
    span(value, source).map(Some)
}

/// The four members of a serialized span.
const SPAN: &[Field] = &[
    req("line", Kind::Number),
    req("col", Kind::Number),
    req("start", Kind::Number),
    req("end", Kind::Number),
];

/// Check that a reported range is a range inside `source`.
fn check_span(span: Span, source: &str) -> Result<(), Fault> {
    if span.start > span.end {
        return Err(span_fault("the range starts after it ends"));
    }
    let length = u32::try_from(source.len())
        .map_err(|_| span_fault("the source is longer than a span can address"))?;
    if span.end > length {
        return Err(span_fault("the range ends beyond the source"));
    }
    for offset in [span.start, span.end] {
        let index =
            usize::try_from(offset).map_err(|_| span_fault("the offset does not fit the host"))?;
        if !source.is_char_boundary(index) {
            return Err(span_fault("the range is not on a character boundary"));
        }
    }
    Ok(())
}

/// Build a span fault.
fn span_fault(message: &str) -> Fault { Fault::new(ReasonCode::AstSpan, message) }
