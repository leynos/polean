//! Polean's names for every AST variant the pinned Regorus can produce.
//!
//! The names are Polean's, not Regorus's: two Regorus variants may share a
//! serialized tag (`Set` is both a rule head and an expression), so an
//! observation cannot use the JSON tag alone as an identity. Keeping the
//! inventory in its own module also keeps the walk that must match every
//! variant in one reviewable place.

/// Every AST variant class the pinned Regorus can produce.
///
/// The names are Polean's, not Regorus's: two Regorus variants may share a
/// serialized tag (`Set` is both a rule head and an expression), so the
/// observation cannot use the JSON tag alone as an identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum NodeKind {
    /// A normal rule with a head and one or more bodies.
    RuleSpec,
    /// A default rule.
    RuleDefault,
    /// A rule head that assigns a value.
    HeadCompr,
    /// A partial-set rule head.
    HeadSet,
    /// A function rule head.
    HeadFunc,
    /// A `some x, y` statement.
    LiteralSomeVars,
    /// A `some x in xs` statement.
    LiteralSomeIn,
    /// A plain expression statement.
    LiteralExpr,
    /// A negated expression statement.
    LiteralNotExpr,
    /// An `every` statement.
    LiteralEvery,
    /// A string literal.
    ExprString,
    /// A raw string literal.
    ExprRawString,
    /// A number literal.
    ExprNumber,
    /// A boolean literal.
    ExprBool,
    /// A null literal.
    ExprNull,
    /// A variable reference.
    ExprVar,
    /// An array literal.
    ExprArray,
    /// A set literal.
    ExprSet,
    /// An object literal.
    ExprObject,
    /// An array comprehension.
    ExprArrayCompr,
    /// A set comprehension.
    ExprSetCompr,
    /// An object comprehension.
    ExprObjectCompr,
    /// A call expression.
    ExprCall,
    /// A unary expression.
    ExprUnary,
    /// A dotted reference.
    ExprRefDot,
    /// A bracketed reference.
    ExprRefBrack,
    /// A binary set operation.
    ExprBin,
    /// A comparison.
    ExprCompare,
    /// An arithmetic expression.
    ExprArith,
    /// An assignment or unification expression.
    ExprAssign,
    /// A membership expression.
    ExprMembership,
    /// The `<` comparison operator.
    BoolOpLt,
    /// The `<=` comparison operator.
    BoolOpLe,
    /// The `==` comparison operator.
    BoolOpEq,
    /// The `>=` comparison operator.
    BoolOpGe,
    /// The `>` comparison operator.
    BoolOpGt,
    /// The `!=` comparison operator.
    BoolOpNe,
    /// The `=` assignment operator.
    AssignOpEq,
    /// The `:=` assignment operator.
    AssignOpColEq,
    /// The `&` set-intersection operator.
    BinOpIntersection,
    /// The `|` set-union operator.
    BinOpUnion,
    /// The `+` arithmetic operator.
    ArithOpAdd,
    /// The `-` arithmetic operator.
    ArithOpSub,
    /// The `*` arithmetic operator.
    ArithOpMul,
    /// The `/` arithmetic operator.
    ArithOpDiv,
    /// The `%` arithmetic operator.
    ArithOpMod,
}

impl NodeKind {
    /// Every kind, in a fixed order.
    pub const ALL: [Self; 46] = [
        Self::RuleSpec,
        Self::RuleDefault,
        Self::HeadCompr,
        Self::HeadSet,
        Self::HeadFunc,
        Self::LiteralSomeVars,
        Self::LiteralSomeIn,
        Self::LiteralExpr,
        Self::LiteralNotExpr,
        Self::LiteralEvery,
        Self::ExprString,
        Self::ExprRawString,
        Self::ExprNumber,
        Self::ExprBool,
        Self::ExprNull,
        Self::ExprVar,
        Self::ExprArray,
        Self::ExprSet,
        Self::ExprObject,
        Self::ExprArrayCompr,
        Self::ExprSetCompr,
        Self::ExprObjectCompr,
        Self::ExprCall,
        Self::ExprUnary,
        Self::ExprRefDot,
        Self::ExprRefBrack,
        Self::ExprBin,
        Self::ExprCompare,
        Self::ExprArith,
        Self::ExprAssign,
        Self::ExprMembership,
        Self::BoolOpLt,
        Self::BoolOpLe,
        Self::BoolOpEq,
        Self::BoolOpGe,
        Self::BoolOpGt,
        Self::BoolOpNe,
        Self::AssignOpEq,
        Self::AssignOpColEq,
        Self::BinOpIntersection,
        Self::BinOpUnion,
        Self::ArithOpAdd,
        Self::ArithOpSub,
        Self::ArithOpMul,
        Self::ArithOpDiv,
        Self::ArithOpMod,
    ];

    /// The kinds every committed admission fixture inherits from its baseline.
    ///
    /// Each fixture is the baseline with one deliberate difference, so no
    /// single entry declares these kinds. Naming them here keeps the declared
    /// coverage test independent of the observed coverage test: the first says
    /// what the corpus intends to exercise, the second what Regorus actually
    /// produced.
    pub const BASELINE_KINDS: [Self; 11] = [
        Self::RuleSpec,
        Self::RuleDefault,
        Self::HeadCompr,
        Self::LiteralExpr,
        Self::ExprString,
        Self::ExprBool,
        Self::ExprVar,
        Self::ExprRefDot,
        Self::ExprCompare,
        Self::BoolOpEq,
        Self::AssignOpColEq,
    ];

    /// The kind's name, as used by the fixture manifest and the snapshot.
    pub const fn name(self) -> &'static str {
        match self {
            Self::RuleSpec => "RuleSpec",
            Self::RuleDefault => "RuleDefault",
            Self::HeadCompr => "HeadCompr",
            Self::HeadSet => "HeadSet",
            Self::HeadFunc => "HeadFunc",
            Self::LiteralSomeVars => "LiteralSomeVars",
            Self::LiteralSomeIn => "LiteralSomeIn",
            Self::LiteralExpr => "LiteralExpr",
            Self::LiteralNotExpr => "LiteralNotExpr",
            Self::LiteralEvery => "LiteralEvery",
            Self::ExprString => "ExprString",
            Self::ExprRawString => "ExprRawString",
            Self::ExprNumber => "ExprNumber",
            Self::ExprBool => "ExprBool",
            Self::ExprNull => "ExprNull",
            Self::ExprVar => "ExprVar",
            Self::ExprArray => "ExprArray",
            Self::ExprSet => "ExprSet",
            Self::ExprObject => "ExprObject",
            Self::ExprArrayCompr => "ExprArrayCompr",
            Self::ExprSetCompr => "ExprSetCompr",
            Self::ExprObjectCompr => "ExprObjectCompr",
            Self::ExprCall => "ExprCall",
            Self::ExprUnary => "ExprUnary",
            Self::ExprRefDot => "ExprRefDot",
            Self::ExprRefBrack => "ExprRefBrack",
            Self::ExprBin => "ExprBin",
            Self::ExprCompare => "ExprCompare",
            Self::ExprArith => "ExprArith",
            Self::ExprAssign => "ExprAssign",
            Self::ExprMembership => "ExprMembership",
            Self::BoolOpLt => "BoolOpLt",
            Self::BoolOpLe => "BoolOpLe",
            Self::BoolOpEq => "BoolOpEq",
            Self::BoolOpGe => "BoolOpGe",
            Self::BoolOpGt => "BoolOpGt",
            Self::BoolOpNe => "BoolOpNe",
            Self::AssignOpEq => "AssignOpEq",
            Self::AssignOpColEq => "AssignOpColEq",
            Self::BinOpIntersection => "BinOpIntersection",
            Self::BinOpUnion => "BinOpUnion",
            Self::ArithOpAdd => "ArithOpAdd",
            Self::ArithOpSub => "ArithOpSub",
            Self::ArithOpMul => "ArithOpMul",
            Self::ArithOpDiv => "ArithOpDiv",
            Self::ArithOpMod => "ArithOpMod",
        }
    }

    /// The kind's position in [`Self::ALL`].
    pub const fn ordinal(self) -> usize {
        match self {
            Self::RuleSpec => 0,
            Self::RuleDefault => 1,
            Self::HeadCompr => 2,
            Self::HeadSet => 3,
            Self::HeadFunc => 4,
            Self::LiteralSomeVars => 5,
            Self::LiteralSomeIn => 6,
            Self::LiteralExpr => 7,
            Self::LiteralNotExpr => 8,
            Self::LiteralEvery => 9,
            Self::ExprString => 10,
            Self::ExprRawString => 11,
            Self::ExprNumber => 12,
            Self::ExprBool => 13,
            Self::ExprNull => 14,
            Self::ExprVar => 15,
            Self::ExprArray => 16,
            Self::ExprSet => 17,
            Self::ExprObject => 18,
            Self::ExprArrayCompr => 19,
            Self::ExprSetCompr => 20,
            Self::ExprObjectCompr => 21,
            Self::ExprCall => 22,
            Self::ExprUnary => 23,
            Self::ExprRefDot => 24,
            Self::ExprRefBrack => 25,
            Self::ExprBin => 26,
            Self::ExprCompare => 27,
            Self::ExprArith => 28,
            Self::ExprAssign => 29,
            Self::ExprMembership => 30,
            Self::BoolOpLt => 31,
            Self::BoolOpLe => 32,
            Self::BoolOpEq => 33,
            Self::BoolOpGe => 34,
            Self::BoolOpGt => 35,
            Self::BoolOpNe => 36,
            Self::AssignOpEq => 37,
            Self::AssignOpColEq => 38,
            Self::BinOpIntersection => 39,
            Self::BinOpUnion => 40,
            Self::ArithOpAdd => 41,
            Self::ArithOpSub => 42,
            Self::ArithOpMul => 43,
            Self::ArithOpDiv => 44,
            Self::ArithOpMod => 45,
        }
    }

    /// Resolve a manifest kind name back to its kind.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|kind| kind.name() == name)
    }
}
