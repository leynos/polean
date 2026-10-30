//! Polean's own tree for the AST JSON the pinned frontend emits.
//!
//! The decoder that builds this tree is closed: every object it accepts has a
//! fixed, tabled key set and every externally tagged object has a fixed tag set.
//! A key the frontend does not emit, a missing key it always emits, a value of
//! the wrong JSON type, and a tag outside the table are all rejected with an
//! `ast.*` reason code before any profile decision is made.
//!
//! The tree mirrors the typed Regorus AST only where the profile walk or the
//! inventory guard needs it. Spans are kept on the nodes whose position may be
//! reported in a rejection; identifiers the frontend emits for its own use
//! (`eidx`, `sidx`, `qidx`, the statement counters) are validated and dropped.
//!
//! Nothing here decides admission. A decoded module is a statement about the
//! frontend's output, not about the source policy.

mod count;
mod decode;
mod decode_body;
mod decode_expr;
mod expr;
mod expr_variants;
mod module;
mod schema;
mod tests;

pub use count::count_module;
pub use decode::decode;
pub use expr::{ArithOp, AssignOp, BinOp, BoolOp, Expr, ExprKind, Member};
pub use module::{Assign, Body, Head, Literal, Module, Package, Query, Rule, Stmt, WithMod};
pub use schema::Fault;

/// A source range the frontend reported.
///
/// The offsets are byte offsets into the submitted text, validated to be
/// ordered, inside the source, and on UTF-8 boundaries.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Span {
    /// The one-based line the range starts on.
    pub line: u32,
    /// The one-based column the range starts at.
    pub col: u32,
    /// The byte offset the range starts at.
    pub start: u32,
    /// The byte offset the range ends at, exclusive.
    pub end: u32,
}

impl Span {
    /// Build a span from its four reported parts.
    #[must_use]
    pub const fn new(line: u32, col: u32, start: u32, end: u32) -> Self {
        Self {
            line,
            col,
            start,
            end,
        }
    }
}
