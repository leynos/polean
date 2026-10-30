//! The closed decoder for expressions.
//!
//! The variant table in [`super::expr_variants`] names every tag the pinned
//! frontend can emit and the fields of each tag's body. Two tags need special
//! care because their bodies are tuples rather than objects: a dotted
//! reference's `field` is a `[span, name]` pair, and an object literal's
//! `fields` are `[span, key, value]` triples. Both are matched as slices, so a
//! wrong arity is a decoding fault instead of a mis-read position.

use serde_json::Value as Json;

use super::{
    ArithOp,
    AssignOp,
    BinOp,
    BoolOp,
    Expr,
    ExprKind,
    Member,
    Span,
    decode::untabled,
    expr_variants::EXPR_VARIANTS,
    schema::{Fault, Fields, as_boolean, as_number, as_number_text, as_string, span, variant},
};
use crate::corpus::ReasonCode;

/// Decode one expression.
///
/// # Errors
///
/// Returns a fault when the value is not an expression the table names, when a
/// member is missing or of the wrong JSON kind, when a tuple array has the
/// wrong arity, or when a span is not a range inside `source`.
pub(super) fn expr(value: &Json, source: &str) -> Result<Expr, Fault> {
    let fields = variant(value, EXPR_VARIANTS)?;
    let Some(tag) = fields.tag() else {
        return Err(untabled("expression"));
    };
    let span = span(fields.value("span")?, source)?;
    as_number(fields.value("eidx")?)?;
    match tag {
        "String" => text(fields, span, ExprKind::String),
        "RawString" => text(fields, span, ExprKind::RawString),
        "Var" => text(fields, span, ExprKind::Var),
        "Number" => number(fields, span),
        "Bool" => boolean(fields, span),
        "Null" => Ok(Expr::new(span, ExprKind::Null)),
        "Array" => collection(fields, source, span, ExprKind::Array),
        "Set" => collection(fields, source, span, ExprKind::Set),
        "Object" => literal_object(fields, source, span),
        "ArrayCompr" | "SetCompr" => comprehension(fields, source, span, tag),
        "ObjectCompr" => object_comprehension(fields, source, span),
        "Call" => call(fields, source, span),
        "UnaryExpr" => unary(fields, source, span),
        "RefDot" => ref_dot(fields, source, span),
        "RefBrack" => ref_brack(fields, source, span),
        "BinExpr" | "BoolExpr" | "ArithExpr" | "AssignExpr" => operator(fields, source, span, tag),
        "Membership" => membership(fields, source, span),
        _ => Err(untabled("expression")),
    }
}

/// Decode a string-shaped value.
fn text(fields: Fields<'_>, span: Span, ctor: fn(String) -> ExprKind) -> Result<Expr, Fault> {
    Ok(Expr::new(
        span,
        ctor(as_string(fields.value("value")?)?.to_owned()),
    ))
}

/// Decode a number, keeping the frontend's spelling.
fn number(fields: Fields<'_>, span: Span) -> Result<Expr, Fault> {
    Ok(Expr::new(
        span,
        ExprKind::Number(as_number_text(fields.value("value")?)?),
    ))
}

/// Decode a boolean.
fn boolean(fields: Fields<'_>, span: Span) -> Result<Expr, Fault> {
    Ok(Expr::new(
        span,
        ExprKind::Bool(as_boolean(fields.value("value")?)?),
    ))
}

/// Decode an array or set literal.
fn collection(
    fields: Fields<'_>,
    source: &str,
    span: Span,
    ctor: fn(Vec<Expr>) -> ExprKind,
) -> Result<Expr, Fault> {
    let items = super::decode::each(fields.value("items")?, source, expr)?;
    Ok(Expr::new(span, ctor(items)))
}

/// Decode an object literal.
fn literal_object(fields: Fields<'_>, source: &str, span: Span) -> Result<Expr, Fault> {
    let members = super::decode::each(fields.value("fields")?, source, member)?;
    Ok(Expr::new(span, ExprKind::Object(members)))
}

/// Decode one `[span, key, value]` triple of an object literal.
fn member(node: &Json, source: &str) -> Result<Member, Fault> {
    let [reported, key, value] = super::schema::as_array(node)? else {
        return Err(arity("an object field is not a [span, key, value] triple"));
    };
    span(reported, source)?;
    Ok(Member {
        key: expr(key, source)?,
        value: expr(value, source)?,
    })
}

/// Decode an array or set comprehension.
fn comprehension(fields: Fields<'_>, source: &str, span: Span, tag: &str) -> Result<Expr, Fault> {
    let term = Box::new(expr(fields.value("term")?, source)?);
    let query = super::decode_body::query(fields.value("query")?, source)?;
    let kind = if tag == "ArrayCompr" {
        ExprKind::ArrayCompr { term, query }
    } else {
        ExprKind::SetCompr { term, query }
    };
    Ok(Expr::new(span, kind))
}

/// Decode an object comprehension.
fn object_comprehension(fields: Fields<'_>, source: &str, span: Span) -> Result<Expr, Fault> {
    Ok(Expr::new(
        span,
        ExprKind::ObjectCompr {
            key: Box::new(expr(fields.value("key")?, source)?),
            value: Box::new(expr(fields.value("value")?, source)?),
            query: super::decode_body::query(fields.value("query")?, source)?,
        },
    ))
}

/// Decode a call.
fn call(fields: Fields<'_>, source: &str, span: Span) -> Result<Expr, Fault> {
    Ok(Expr::new(
        span,
        ExprKind::Call {
            fcn: Box::new(expr(fields.value("fcn")?, source)?),
            params: super::decode::each(fields.value("params")?, source, expr)?,
        },
    ))
}

/// Decode a unary expression.
fn unary(fields: Fields<'_>, source: &str, span: Span) -> Result<Expr, Fault> {
    Ok(Expr::new(
        span,
        ExprKind::Unary {
            expr: Box::new(expr(fields.value("expr")?, source)?),
        },
    ))
}

/// Decode a dotted reference, including its `[span, name]` field tuple.
fn ref_dot(fields: Fields<'_>, source: &str, at: Span) -> Result<Expr, Fault> {
    let [reported, name] = super::schema::as_array(fields.value("field")?)? else {
        return Err(arity("a dotted reference field is not a [span, name] pair"));
    };
    super::schema::span(reported, source)?;
    Ok(Expr::new(
        at,
        ExprKind::RefDot {
            refr: Box::new(expr(fields.value("refr")?, source)?),
            field: as_string(name)?.to_owned(),
        },
    ))
}

/// Decode a bracketed reference.
fn ref_brack(fields: Fields<'_>, source: &str, span: Span) -> Result<Expr, Fault> {
    Ok(Expr::new(
        span,
        ExprKind::RefBrack {
            refr: Box::new(expr(fields.value("refr")?, source)?),
            index: Box::new(expr(fields.value("index")?, source)?),
        },
    ))
}

/// Decode a binary, comparison, arithmetic, or assignment expression.
fn operator(fields: Fields<'_>, source: &str, span: Span, tag: &str) -> Result<Expr, Fault> {
    let lhs = Box::new(expr(fields.value("lhs")?, source)?);
    let rhs = Box::new(expr(fields.value("rhs")?, source)?);
    let op = as_string(fields.value("op")?)?;
    let kind = match tag {
        "BinExpr" => ExprKind::Bin {
            op: BinOp::from_tag(op).ok_or_else(|| unknown_operator(op))?,
            lhs,
            rhs,
        },
        "BoolExpr" => ExprKind::Compare {
            op: BoolOp::from_tag(op).ok_or_else(|| unknown_operator(op))?,
            lhs,
            rhs,
        },
        "ArithExpr" => ExprKind::Arith {
            op: ArithOp::from_tag(op).ok_or_else(|| unknown_operator(op))?,
            lhs,
            rhs,
        },
        _ => ExprKind::Assign {
            op: AssignOp::from_tag(op).ok_or_else(|| unknown_operator(op))?,
            lhs,
            rhs,
        },
    };
    Ok(Expr::new(span, kind))
}

/// Decode a membership test.
fn membership(fields: Fields<'_>, source: &str, span: Span) -> Result<Expr, Fault> {
    let declared = fields.value("key")?;
    let key = if declared.is_null() {
        None
    } else {
        Some(Box::new(expr(declared, source)?))
    };
    Ok(Expr::new(
        span,
        ExprKind::Membership {
            key,
            value: Box::new(expr(fields.value("value")?, source)?),
            collection: Box::new(expr(fields.value("collection")?, source)?),
        },
    ))
}

/// The fault for a tag the table matched but the dispatch does not handle.
fn unknown_operator(tag: &str) -> Fault {
    Fault::new(
        ReasonCode::AstUnknownTag,
        format!("`{tag}` is not an operator of this expression"),
    )
}

/// The fault for a tuple array of the wrong length.
fn arity(message: &str) -> Fault { Fault::new(ReasonCode::AstArity, message) }
