//! Modules, rules, bodies, and the literals inside them.
//!
//! The shape mirrors the pinned frontend's `Module`, `Rule`, `RuleHead`,
//! `RuleBody`, `Query`, `LiteralStmt`, `Literal`, and `RuleAssign`, reduced to
//! the parts a decision reads. Statement and query indices are validated by the
//! decoder and then dropped, because ordering is already the vector order.

use super::{AssignOp, Expr, Span};

/// One decoded module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Module {
    /// The module's package declaration.
    pub package: Package,
    /// The module's imports, in source order.
    pub imports: Vec<Import>,
    /// The module's rules, in source order.
    pub rules: Vec<Rule>,
    /// The module's `__target__`, when it sets one.
    pub target: Option<String>,
}

/// A package declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Package {
    /// The declaration's source range.
    pub span: Span,
    /// The package reference.
    pub refr: Expr,
}

/// An import declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Import {
    /// The declaration's source range.
    pub span: Span,
    /// The imported reference.
    pub refr: Expr,
    /// The alias's source range, when the import is aliased.
    pub alias: Option<Span>,
}

/// One rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Rule {
    /// A normal rule: a head and one or more bodies.
    Spec {
        /// The rule's source range.
        span: Span,
        /// The rule head.
        head: Head,
        /// The rule bodies, in source order.
        bodies: Vec<Body>,
    },
    /// A default rule.
    Default {
        /// The rule's source range.
        span: Span,
        /// The assigned reference.
        refr: Expr,
        /// The head's arguments.
        args: Vec<Expr>,
        /// The assignment operator.
        op: AssignOp,
        /// The assigned value.
        value: Expr,
    },
}

impl Rule {
    /// The rule's source range.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Spec { span, .. } | Self::Default { span, .. } => *span,
        }
    }
}

/// A rule head.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Head {
    /// A complete rule head.
    Compr {
        /// The head's source range.
        span: Span,
        /// The head's reference.
        refr: Expr,
        /// The value the head assigns, when it assigns one.
        assign: Option<Assign>,
    },
    /// A partial set rule head.
    Set {
        /// The head's source range.
        span: Span,
        /// The head's reference.
        refr: Expr,
        /// The head's key, when it declares one.
        key: Option<Expr>,
    },
    /// A function rule head.
    Func {
        /// The head's source range.
        span: Span,
        /// The head's reference.
        refr: Expr,
        /// The head's parameters.
        args: Vec<Expr>,
        /// The value the head assigns, when it assigns one.
        assign: Option<Assign>,
    },
}

impl Head {
    /// The head's source range.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Compr { span, .. } | Self::Set { span, .. } | Self::Func { span, .. } => *span,
        }
    }
}

/// A value assignment on a head or body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Assign {
    /// The assignment's source range.
    pub span: Span,
    /// The assignment operator.
    pub op: AssignOp,
    /// The assigned value.
    pub value: Expr,
}

/// One rule body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Body {
    /// The body's source range.
    pub span: Span,
    /// The value the body assigns, when it assigns one.
    pub assign: Option<Assign>,
    /// Whether the body is an `else` body.
    pub is_else: bool,
    /// The body's query.
    pub query: Query,
}

/// A query: the statements one body or comprehension evaluates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Query {
    /// The query's source range.
    pub span: Span,
    /// The statements, in source order.
    pub stmts: Vec<Stmt>,
}

/// One statement of a query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stmt {
    /// The statement's source range.
    pub span: Span,
    /// The statement's literal.
    pub literal: Literal,
    /// The statement's `with` modifiers, in source order.
    pub with_mods: Vec<WithMod>,
}

/// A `with` modifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithMod {
    /// The modifier's source range.
    pub span: Span,
    /// The reference being replaced.
    pub refr: Expr,
    /// The value it is replaced with.
    pub target: Expr,
}

/// A statement's literal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Literal {
    /// A `some x, y` declaration.
    SomeVars(Vec<Span>),
    /// A `some x in xs` declaration.
    SomeIn {
        /// The optional key.
        key: Option<Expr>,
        /// The value bound by the declaration.
        value: Expr,
        /// The collection searched.
        collection: Expr,
    },
    /// A plain expression statement.
    Expr(Expr),
    /// A negated expression statement.
    NotExpr(Expr),
    /// An `every` statement.
    Every {
        /// The variable's domain.
        domain: Expr,
        /// The quantified body.
        query: Query,
    },
}
