//! Every expression variant the pinned frontend can emit, and its fields.
//!
//! The table is data, kept apart from the decoder's dispatch so it can be
//! compared with the frontend's `Expr` enum in one place. A tag the frontend
//! gains without a matching entry here is rejected as `ast.unknown_tag`, which
//! is the fail-closed direction.

use super::schema::{Kind, Variant, req, variant_of};

/// Every expression variant the pinned frontend can emit.
pub(super) const EXPR_VARIANTS: &[Variant] = &[
    variant_of(
        "String",
        &[
            req("span", Kind::Object),
            req("value", Kind::String),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "RawString",
        &[
            req("span", Kind::Object),
            req("value", Kind::String),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "Number",
        &[
            req("span", Kind::Object),
            req("value", Kind::Number),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "Bool",
        &[
            req("span", Kind::Object),
            req("value", Kind::Bool),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "Null",
        &[
            req("span", Kind::Object),
            req("value", Kind::Null),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "Var",
        &[
            req("span", Kind::Object),
            req("value", Kind::String),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "Array",
        &[
            req("span", Kind::Object),
            req("items", Kind::Array),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "Set",
        &[
            req("span", Kind::Object),
            req("items", Kind::Array),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "Object",
        &[
            req("span", Kind::Object),
            req("fields", Kind::Array),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "ArrayCompr",
        &[
            req("span", Kind::Object),
            req("term", Kind::Object),
            req("query", Kind::Object),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "SetCompr",
        &[
            req("span", Kind::Object),
            req("term", Kind::Object),
            req("query", Kind::Object),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "ObjectCompr",
        &[
            req("span", Kind::Object),
            req("key", Kind::Object),
            req("value", Kind::Object),
            req("query", Kind::Object),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "Call",
        &[
            req("span", Kind::Object),
            req("fcn", Kind::Object),
            req("params", Kind::Array),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "UnaryExpr",
        &[
            req("span", Kind::Object),
            req("expr", Kind::Object),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "RefDot",
        &[
            req("span", Kind::Object),
            req("refr", Kind::Object),
            req("field", Kind::Array),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "RefBrack",
        &[
            req("span", Kind::Object),
            req("refr", Kind::Object),
            req("index", Kind::Object),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "BinExpr",
        &[
            req("span", Kind::Object),
            req("op", Kind::String),
            req("lhs", Kind::Object),
            req("rhs", Kind::Object),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "BoolExpr",
        &[
            req("span", Kind::Object),
            req("op", Kind::String),
            req("lhs", Kind::Object),
            req("rhs", Kind::Object),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "ArithExpr",
        &[
            req("span", Kind::Object),
            req("op", Kind::String),
            req("lhs", Kind::Object),
            req("rhs", Kind::Object),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "AssignExpr",
        &[
            req("span", Kind::Object),
            req("op", Kind::String),
            req("lhs", Kind::Object),
            req("rhs", Kind::Object),
            req("eidx", Kind::Number),
        ],
    ),
    variant_of(
        "Membership",
        &[
            req("span", Kind::Object),
            req("key", Kind::NullOrObject),
            req("value", Kind::Object),
            req("collection", Kind::Object),
            req("eidx", Kind::Number),
        ],
    ),
];
