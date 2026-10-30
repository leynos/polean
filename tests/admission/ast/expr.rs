//! Expressions, and the four operator tables.
//!
//! The shape follows the pinned frontend's `Expr` enum one variant for one
//! variant, so the inventory guard can compare multisets mechanically. Each
//! expression carries the span of its own node; the frontend's `eidx` is
//! validated and dropped because no decision reads it.
//!
//! The operator enums are Polean's copies of the frontend's. They are parsed
//! from the serialized tag rather than constructed, so an operator the table
//! does not name is a decoding fault instead of a silently reused variant.

use super::Span;

/// An expression and the source range it occupies.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Expr {
    /// The expression's source range.
    pub span: Span,
    /// The expression's shape.
    pub kind: ExprKind,
}

impl Expr {
    /// Build an expression from its range and shape.
    #[must_use]
    pub const fn new(span: Span, kind: ExprKind) -> Self { Self { span, kind } }
}

/// One key and value of an object literal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Member {
    /// The key expression.
    pub key: Expr,
    /// The value expression.
    pub value: Expr,
}

/// The shape of an expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExprKind {
    /// A quoted string.
    String(String),
    /// A raw string.
    RawString(String),
    /// A number, kept as the frontend's JSON spelling.
    Number(String),
    /// A boolean.
    Bool(bool),
    /// A null.
    Null,
    /// A variable.
    Var(String),
    /// An array literal.
    Array(Vec<Expr>),
    /// A set literal.
    Set(Vec<Expr>),
    /// An object literal.
    Object(Vec<Member>),
    /// An array comprehension.
    ArrayCompr {
        /// The term each element takes.
        term: Box<Expr>,
        /// The comprehension's body.
        query: Query,
    },
    /// A set comprehension.
    SetCompr {
        /// The term each element takes.
        term: Box<Expr>,
        /// The comprehension's body.
        query: Query,
    },
    /// An object comprehension.
    ObjectCompr {
        /// The key each member takes.
        key: Box<Expr>,
        /// The value each member takes.
        value: Box<Expr>,
        /// The comprehension's body.
        query: Query,
    },
    /// A call.
    Call {
        /// The called function.
        fcn: Box<Expr>,
        /// The arguments.
        params: Vec<Expr>,
    },
    /// A unary expression.
    Unary {
        /// The operand.
        expr: Box<Expr>,
    },
    /// A dotted reference.
    RefDot {
        /// The reference being extended.
        refr: Box<Expr>,
        /// The field name.
        field: String,
    },
    /// A bracketed reference.
    RefBrack {
        /// The reference being indexed.
        refr: Box<Expr>,
        /// The index expression.
        index: Box<Expr>,
    },
    /// A binary set operation.
    Bin {
        /// The operator.
        op: BinOp,
        /// The left operand.
        lhs: Box<Expr>,
        /// The right operand.
        rhs: Box<Expr>,
    },
    /// A comparison.
    Compare {
        /// The operator.
        op: BoolOp,
        /// The left operand.
        lhs: Box<Expr>,
        /// The right operand.
        rhs: Box<Expr>,
    },
    /// An arithmetic expression.
    Arith {
        /// The operator.
        op: ArithOp,
        /// The left operand.
        lhs: Box<Expr>,
        /// The right operand.
        rhs: Box<Expr>,
    },
    /// An assignment or unification.
    Assign {
        /// The operator.
        op: AssignOp,
        /// The left operand.
        lhs: Box<Expr>,
        /// The right operand.
        rhs: Box<Expr>,
    },
    /// A membership test.
    Membership {
        /// The optional key.
        key: Option<Box<Expr>>,
        /// The value tested for membership.
        value: Box<Expr>,
        /// The collection searched.
        collection: Box<Expr>,
    },
}

/// A comparison operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoolOp {
    /// `<`.
    Lt,
    /// `<=`.
    Le,
    /// `==`.
    Eq,
    /// `>=`.
    Ge,
    /// `>`.
    Gt,
    /// `!=`.
    Ne,
}

impl BoolOp {
    /// The operator named by a serialized tag, if the table names one.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "Lt" => Some(Self::Lt),
            "Le" => Some(Self::Le),
            "Eq" => Some(Self::Eq),
            "Ge" => Some(Self::Ge),
            "Gt" => Some(Self::Gt),
            "Ne" => Some(Self::Ne),
            _ => None,
        }
    }
}

/// An assignment operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssignOp {
    /// `=`.
    Eq,
    /// `:=`.
    ColEq,
}

impl AssignOp {
    /// The operator named by a serialized tag, if the table names one.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "Eq" => Some(Self::Eq),
            "ColEq" => Some(Self::ColEq),
            _ => None,
        }
    }
}

/// A binary set operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinOp {
    /// `&`.
    Intersection,
    /// `|`.
    Union,
}

impl BinOp {
    /// The operator named by a serialized tag, if the table names one.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "Intersection" => Some(Self::Intersection),
            "Union" => Some(Self::Union),
            _ => None,
        }
    }
}

/// An arithmetic operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArithOp {
    /// `+`.
    Add,
    /// `-`.
    Sub,
    /// `*`.
    Mul,
    /// `/`.
    Div,
    /// `%`.
    Mod,
}

impl ArithOp {
    /// The operator named by a serialized tag, if the table names one.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "Add" => Some(Self::Add),
            "Sub" => Some(Self::Sub),
            "Mul" => Some(Self::Mul),
            "Div" => Some(Self::Div),
            "Mod" => Some(Self::Mod),
            _ => None,
        }
    }
}

/// A comprehension's body.
///
/// Comprehensions reuse the query type, so the alias points at the rule
/// module's type: an expression can contain a query, and a query can contain
/// expressions.
pub type Query = super::module::Query;
