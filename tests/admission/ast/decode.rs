//! The closed decoder for modules, imports, rules, and rule heads.
//!
//! The entry point checks the outer envelope first: the AST JSON must parse, the
//! top-level value must be an array of exactly one entry, that entry must carry
//! a source and a version of 1, and the source text it echoes must be the text
//! that was submitted. Only then is the module decoded.
//!
//! Each node's key set and tag set is a fixed table in this file; see
//! [`super::schema`] for the reader that enforces them.

use serde_json::Value as Json;

use super::{
    Module,
    Package,
    Rule,
    Span,
    decode_body::{body, expr, nullable_expr},
    expr::AssignOp,
    module::{Head, Import},
    schema::{
        Fault,
        Field,
        Fields,
        Kind,
        Variant,
        as_array,
        as_boolean,
        as_number,
        as_string,
        object,
        opt,
        req,
        span,
        variant,
        variant_of,
    },
};
use crate::corpus::ReasonCode;

/// Decode one module from the frontend's AST JSON.
///
/// `source` is the exact text that was submitted to the frontend; the echoes in
/// the AST and every reported span are checked against it.
///
/// # Errors
///
/// Returns a fault when the JSON is not an AST for `source`, when any object
/// carries an unknown or missing key, when a value has the wrong JSON kind,
/// when a tag is outside the table, or when a span is not a range inside
/// `source`.
pub fn decode(source: &str, ast_json: &str) -> Result<Module, Fault> {
    let root = parsed(ast_json)?;
    let fields = object(single_entry(&root)?, ENTRY_FIELDS)?;
    check_version(fields.value("version")?)?;
    check_source(fields.value("source")?, source)?;
    module(object(fields.value("ast")?, MODULE_FIELDS)?, source)
}

/// Parse the frontend's AST JSON, reporting a serialization fault.
fn parsed(ast_json: &str) -> Result<Json, Fault> {
    serde_json::from_str(ast_json).map_err(|error| {
        Fault::new(
            ReasonCode::AstSerialize,
            format!("the frontend's AST is not JSON: {error}"),
        )
    })
}

/// Require the document to hold exactly one module entry.
fn single_entry(root: &Json) -> Result<&Json, Fault> {
    let Json::Array(entries) = root else {
        return Err(Fault::new(
            ReasonCode::AstModuleCount,
            "the frontend's AST is not an array of modules",
        ));
    };
    let [entry] = entries.as_slice() else {
        return Err(Fault::new(
            ReasonCode::AstModuleCount,
            format!("the AST holds {} entries, not one", entries.len()),
        ));
    };
    Ok(entry)
}

/// Require the only AST version the pinned frontend emits.
fn check_version(value: &Json) -> Result<(), Fault> {
    let version = as_number(value)?;
    if version == SUPPORTED_VERSION {
        return Ok(());
    }
    Err(Fault::new(
        ReasonCode::AstType,
        format!("the AST version is {version}, not {SUPPORTED_VERSION}"),
    ))
}

/// Require the AST's echoed source text to be the text that was submitted.
fn check_source(value: &Json, source: &str) -> Result<(), Fault> {
    let echo = object(value, SOURCE_FIELDS)?;
    as_string(echo.value("file")?)?;
    if as_string(echo.value("contents")?)? == source {
        return Ok(());
    }
    Err(Fault::new(
        ReasonCode::AstSourceMismatch,
        "the source text in the AST is not the submitted text",
    ))
}

/// The only AST version the pinned frontend is known to emit.
const SUPPORTED_VERSION: u32 = 1;

/// The envelope of one serialized module.
const ENTRY_FIELDS: &[Field] = &[
    req("source", Kind::Object),
    req("version", Kind::Number),
    req("ast", Kind::Object),
];

/// The source text the frontend echoes back.
const SOURCE_FIELDS: &[Field] = &[req("file", Kind::String), req("contents", Kind::String)];

/// The module's own keys.
///
/// `rules` is the frontend's serialized name for the rule list; the counters
/// are validated and dropped because no decision reads them.
const MODULE_FIELDS: &[Field] = &[
    req("package", Kind::Object),
    req("imports", Kind::Array),
    req("rules", Kind::Array),
    req("rego_v1", Kind::Bool),
    opt("target", Kind::String),
    req("num_expressions", Kind::Number),
    req("num_statements", Kind::Number),
    req("num_queries", Kind::Number),
];

/// A package declaration's keys.
const PACKAGE_FIELDS: &[Field] = &[req("span", Kind::Object), req("refr", Kind::Object)];

/// An import declaration's keys.
const IMPORT_FIELDS: &[Field] = &[
    req("span", Kind::Object),
    req("refr", Kind::Object),
    opt("as", Kind::Object),
];

/// A rule's variants.
const RULE_VARIANTS: &[Variant] = &[
    variant_of(
        "Spec",
        &[
            req("span", Kind::Object),
            req("head", Kind::Object),
            req("bodies", Kind::Array),
        ],
    ),
    variant_of(
        "Default",
        &[
            req("span", Kind::Object),
            req("refr", Kind::Object),
            req("args", Kind::Array),
            req("op", Kind::String),
            req("value", Kind::Object),
        ],
    ),
];

/// A rule head's variants.
///
/// `assign` and `key` are required even when null: the frontend always emits
/// them, and a body without an assignment is how it spells "no assignment".
const HEAD_VARIANTS: &[Variant] = &[
    variant_of(
        "Compr",
        &[
            req("span", Kind::Object),
            req("refr", Kind::Object),
            req("assign", Kind::NullOrObject),
        ],
    ),
    variant_of(
        "Set",
        &[
            req("span", Kind::Object),
            req("refr", Kind::Object),
            req("key", Kind::NullOrObject),
        ],
    ),
    variant_of(
        "Func",
        &[
            req("span", Kind::Object),
            req("refr", Kind::Object),
            req("args", Kind::Array),
            req("assign", Kind::NullOrObject),
        ],
    ),
];

/// Decode the module body.
fn module(fields: Fields<'_>, source: &str) -> Result<Module, Fault> {
    as_boolean(fields.value("rego_v1")?)?;
    as_number(fields.value("num_expressions")?)?;
    as_number(fields.value("num_statements")?)?;
    as_number(fields.value("num_queries")?)?;
    Ok(Module {
        package: package(object(fields.value("package")?, PACKAGE_FIELDS)?, source)?,
        imports: each(fields.value("imports")?, source, import)?,
        rules: each(fields.value("rules")?, source, rule)?,
        target: target(fields)?,
    })
}

/// Decode the module's optional `__target__`.
///
/// The frontend omits `target` when the module sets none. A present but empty
/// target is therefore not something the frontend emits, and is rejected
/// rather than accepted as "no target" (O2).
fn target(fields: Fields<'_>) -> Result<Option<String>, Fault> {
    match fields.optional("target") {
        None => Ok(None),
        Some(value) => {
            let target = as_string(value)?;
            if target.is_empty() {
                return Err(Fault::new(
                    ReasonCode::AstType,
                    "`target` is present but empty",
                ));
            }
            Ok(Some(target.to_owned()))
        }
    }
}

/// Decode a package declaration.
fn package(fields: Fields<'_>, source: &str) -> Result<Package, Fault> {
    Ok(Package {
        span: span(fields.value("span")?, source)?,
        refr: expr(fields.value("refr")?, source)?,
    })
}

/// Decode one import declaration.
fn import(value: &Json, source: &str) -> Result<Import, Fault> {
    let fields = object(value, IMPORT_FIELDS)?;
    Ok(Import {
        span: span(fields.value("span")?, source)?,
        refr: expr(fields.value("refr")?, source)?,
        alias: fields
            .optional("as")
            .map(|alias| span(alias, source))
            .transpose()?,
    })
}

/// Decode one rule.
fn rule(value: &Json, source: &str) -> Result<Rule, Fault> {
    let fields = variant(value, RULE_VARIANTS)?;
    match fields.tag() {
        Some("Spec") => Ok(Rule::Spec {
            span: span(fields.value("span")?, source)?,
            head: head(fields.value("head")?, source)?,
            bodies: each(fields.value("bodies")?, source, body)?,
        }),
        Some("Default") => Ok(Rule::Default {
            span: span(fields.value("span")?, source)?,
            refr: expr(fields.value("refr")?, source)?,
            args: each(fields.value("args")?, source, expr)?,
            op: assign_op(fields.value("op")?)?,
            value: expr(fields.value("value")?, source)?,
        }),
        _ => Err(untabled("rule")),
    }
}

/// Decode one rule head.
fn head(value: &Json, source: &str) -> Result<Head, Fault> {
    let fields = variant(value, HEAD_VARIANTS)?;
    match fields.tag() {
        Some("Compr") => Ok(Head::Compr {
            span: span(fields.value("span")?, source)?,
            refr: expr(fields.value("refr")?, source)?,
            assign: nullable_assign(fields.value("assign")?, source)?,
        }),
        Some("Set") => Ok(Head::Set {
            span: span(fields.value("span")?, source)?,
            refr: expr(fields.value("refr")?, source)?,
            key: nullable_expr(fields.value("key")?, source)?,
        }),
        Some("Func") => Ok(Head::Func {
            span: span(fields.value("span")?, source)?,
            refr: expr(fields.value("refr")?, source)?,
            args: each(fields.value("args")?, source, expr)?,
            assign: nullable_assign(fields.value("assign")?, source)?,
        }),
        _ => Err(untabled("rule head")),
    }
}

/// Decode an optional value assignment the frontend writes as null.
pub(super) fn nullable_assign(value: &Json, source: &str) -> Result<Option<super::Assign>, Fault> {
    if value.is_null() {
        return Ok(None);
    }
    assign(value, source).map(Some)
}

/// Decode a value assignment.
pub(super) fn assign(value: &Json, source: &str) -> Result<super::Assign, Fault> {
    let fields = object(value, ASSIGN_FIELDS)?;
    Ok(super::Assign {
        span: span(fields.value("span")?, source)?,
        op: assign_op(fields.value("op")?)?,
        value: expr(fields.value("value")?, source)?,
    })
}

/// A value assignment's keys.
const ASSIGN_FIELDS: &[Field] = &[
    req("span", Kind::Object),
    req("op", Kind::String),
    req("value", Kind::Object),
];

/// Decode an assignment operator tag.
fn assign_op(value: &Json) -> Result<AssignOp, Fault> {
    let tag = as_string(value)?;
    AssignOp::from_tag(tag).ok_or_else(|| {
        Fault::new(
            ReasonCode::AstUnknownTag,
            format!("`{tag}` is not an assignment operator"),
        )
    })
}

/// Decode every element of an array with `read`.
pub(super) fn each<T>(
    value: &Json,
    source: &str,
    read: fn(&Json, &str) -> Result<T, Fault>,
) -> Result<Vec<T>, Fault> {
    as_array(value)?
        .iter()
        .map(|item| read(item, source))
        .collect()
}

/// The fault for a tag the table matched but the decoder did not handle.
///
/// Unreachable through [`variant`], which only accepts tags the table lists;
/// reporting it as a decoding fault keeps the decoder closed even if the table
/// and the dispatch are edited apart.
pub(super) fn untabled(what: &str) -> Fault {
    Fault::new(
        ReasonCode::AstUnknownTag,
        format!("the {what} table matched a tag the decoder does not handle"),
    )
}

/// Decode an array of spans.
pub(super) fn spans(value: &Json, source: &str) -> Result<Vec<Span>, Fault> {
    as_array(value)?
        .iter()
        .map(|item| span(item, source))
        .collect()
}
