//! The closed decoder for rule bodies, queries, statements, and literals.
//!
//! A body owns a query; a query owns statements; a statement owns one literal
//! and its `with` modifiers. Comprehensions reuse [`query`] from this module,
//! so a query nested inside an expression is decoded by the same table as a
//! rule body's.
//!
//! The frontend's statement and query indices (`sidx`, `qidx`) are validated
//! and dropped: vector order already is source order, and no decision reads an
//! index.

use serde_json::Value as Json;

pub(super) use super::decode_expr::expr;
use super::{
    Body,
    Literal,
    Query,
    Stmt,
    WithMod,
    decode::{each, nullable_assign, spans, untabled},
    schema::{
        Fault,
        Field,
        Fields,
        Kind,
        Variant,
        as_boolean,
        as_number,
        nullable_span,
        object,
        opt,
        req,
        span,
        variant,
        variant_of,
    },
};
use crate::corpus::ReasonCode;

/// Decode one rule body.
///
/// # Errors
///
/// Returns a fault when the value is not a body the table names, when a member
/// is missing or of the wrong JSON kind, or when a span is not a range inside
/// `source`.
pub(super) fn body(value: &Json, source: &str) -> Result<Body, Fault> {
    let fields = object(value, BODY_FIELDS)?;
    Ok(Body {
        span: span(fields.value("span")?, source)?,
        assign: nullable_assign(fields.value("assign")?, source)?,
        is_else: as_boolean(fields.value("is_else")?)?,
        query: query(fields.value("query")?, source)?,
    })
}

/// Decode one query.
///
/// # Errors
///
/// Returns a fault when the value is not a query the table names, when a member
/// is missing or of the wrong JSON kind, or when a span is not a range inside
/// `source`.
pub(super) fn query(value: &Json, source: &str) -> Result<Query, Fault> {
    let fields = object(value, QUERY_FIELDS)?;
    as_number(fields.value("qidx")?)?;
    Ok(Query {
        span: span(fields.value("span")?, source)?,
        stmts: each(fields.value("stmts")?, source, stmt)?,
    })
}

/// Decode one statement.
fn stmt(value: &Json, source: &str) -> Result<Stmt, Fault> {
    let fields = object(value, STMT_FIELDS)?;
    as_number(fields.value("sidx")?)?;
    Ok(Stmt {
        span: span(fields.value("span")?, source)?,
        literal: literal(fields.value("literal")?, source)?,
        with_mods: optional_array(fields, "with_mods", source, with_mod)?,
    })
}

/// Decode one `with` modifier.
fn with_mod(value: &Json, source: &str) -> Result<WithMod, Fault> {
    let fields = object(value, WITH_MOD_FIELDS)?;
    Ok(WithMod {
        span: span(fields.value("span")?, source)?,
        refr: expr(fields.value("refr")?, source)?,
        target: expr(fields.value("as")?, source)?,
    })
}

/// Decode one literal.
///
/// Every literal variant carries its own `span`. Reading it here rather than in
/// each arm keeps the object inside the closed walk: a mutated or malformed
/// range on the literal itself is a decoding fault, not a position a profile
/// stage might later trust.
fn literal(value: &Json, source: &str) -> Result<Literal, Fault> {
    let fields = variant(value, LITERAL_VARIANTS)?;
    span(fields.value("span")?, source)?;
    match fields.tag() {
        Some("SomeVars") => Ok(Literal::SomeVars(spans(fields.value("vars")?, source)?)),
        Some("SomeIn") => Ok(Literal::SomeIn {
            key: nullable_expr(fields.value("key")?, source)?,
            value: expr(fields.value("value")?, source)?,
            collection: expr(fields.value("collection")?, source)?,
        }),
        Some("Expr") => Ok(Literal::Expr(expr(fields.value("expr")?, source)?)),
        Some("NotExpr") => Ok(Literal::NotExpr(expr(fields.value("expr")?, source)?)),
        Some("Every") => Ok(Literal::Every {
            domain: expr(fields.value("domain")?, source)?,
            query: every_query(fields, source)?,
        }),
        _ => Err(untabled("literal")),
    }
}

/// Decode an `every` statement's quantified query.
///
/// The quantified variable and its optional key are reported as bare spans
/// rather than expressions; both are validated against the source and dropped,
/// because the quantified body is what a decision reads.
fn every_query(fields: Fields<'_>, source: &str) -> Result<Query, Fault> {
    nullable_span(fields.value("key")?, source)?;
    span(fields.value("value")?, source)?;
    query(fields.value("query")?, source)
}

/// Decode an expression the frontend may have written as null.
///
/// # Errors
///
/// Returns a fault when the value is neither null nor a decodable expression.
pub(super) fn nullable_expr(value: &Json, source: &str) -> Result<Option<super::Expr>, Fault> {
    if value.is_null() {
        return Ok(None);
    }
    expr(value, source).map(Some)
}

/// Decode an optional array member the frontend omits when it is empty.
///
/// Presence with an empty array is a shape the frontend never emits, so it is
/// rejected: a decoder that accepted it would admit two JSON spellings for one
/// module (O2).
fn optional_array<T>(
    fields: Fields<'_>,
    name: &str,
    source: &str,
    read: fn(&Json, &str) -> Result<T, Fault>,
) -> Result<Vec<T>, Fault> {
    match fields.optional(name) {
        None => Ok(Vec::new()),
        Some(value) => {
            let items = each(value, source, read)?;
            if items.is_empty() {
                return Err(Fault::new(
                    ReasonCode::AstType,
                    format!("`{name}` is present but empty"),
                ));
            }
            Ok(items)
        }
    }
}

/// A rule body's keys.
const BODY_FIELDS: &[Field] = &[
    req("span", Kind::Object),
    req("assign", Kind::NullOrObject),
    req("is_else", Kind::Bool),
    req("query", Kind::Object),
];

/// A query's keys.
const QUERY_FIELDS: &[Field] = &[
    req("span", Kind::Object),
    req("stmts", Kind::Array),
    req("qidx", Kind::Number),
];

/// A statement's keys.
///
/// `with_mods` is optional because the frontend omits it when a statement has
/// no modifiers.
const STMT_FIELDS: &[Field] = &[
    req("span", Kind::Object),
    req("literal", Kind::Object),
    opt("with_mods", Kind::Array),
    req("sidx", Kind::Number),
];

/// A `with` modifier's keys.
const WITH_MOD_FIELDS: &[Field] = &[
    req("span", Kind::Object),
    req("refr", Kind::Object),
    req("as", Kind::Object),
];

/// Every literal variant the pinned frontend can emit.
const LITERAL_VARIANTS: &[Variant] = &[
    variant_of(
        "SomeVars",
        &[req("span", Kind::Object), req("vars", Kind::Array)],
    ),
    variant_of(
        "SomeIn",
        &[
            req("span", Kind::Object),
            req("key", Kind::NullOrObject),
            req("value", Kind::Object),
            req("collection", Kind::Object),
        ],
    ),
    variant_of(
        "Expr",
        &[req("span", Kind::Object), req("expr", Kind::Object)],
    ),
    variant_of(
        "NotExpr",
        &[req("span", Kind::Object), req("expr", Kind::Object)],
    ),
    variant_of(
        "Every",
        &[
            req("span", Kind::Object),
            req("key", Kind::NullOrObject),
            req("value", Kind::Object),
            req("domain", Kind::Object),
            req("query", Kind::Object),
        ],
    ),
];
