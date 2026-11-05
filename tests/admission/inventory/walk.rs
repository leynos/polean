//! The exhaustive walk over the pinned Regorus AST.
//!
//! Every `match` here is exhaustive and has no wildcard, so a Regorus release
//! that adds a variant stops compilation. The walk only counts node kinds; it
//! makes no admission decision.

use regorus::unstable::{
    ArithOp,
    AssignOp,
    BinOp,
    BoolOp,
    Expr,
    Literal,
    Module,
    NodeRef,
    Query,
    Rule,
    RuleAssign,
    RuleBody,
    RuleHead,
};

use super::{KindCounts, NodeKind};

/// Count one occurrence of `kind`.
fn bump(counts: &mut KindCounts, kind: NodeKind) { *counts.entry(kind).or_default() += 1; }

/// Walk a module: its package reference, imports, and rules.
pub(super) fn visit_module(module: &Module, counts: &mut KindCounts) {
    visit_expr(&module.package.refr, counts);
    for import in &module.imports {
        visit_expr(&import.refr, counts);
    }
    for rule in &module.policy {
        visit_rule(rule, counts);
    }
}

/// Walk one rule.
fn visit_rule(rule: &Rule, counts: &mut KindCounts) {
    match rule {
        Rule::Spec { head, bodies, .. } => {
            bump(counts, NodeKind::RuleSpec);
            visit_head(head, counts);
            for body in bodies {
                visit_body(body, counts);
            }
        }
        Rule::Default {
            refr,
            args,
            op,
            value,
            ..
        } => {
            bump(counts, NodeKind::RuleDefault);
            bump(counts, assign_op_kind(op));
            visit_expr(refr, counts);
            for arg in args {
                visit_expr(arg, counts);
            }
            visit_expr(value, counts);
        }
    }
}

/// Walk a rule head, including any value assignment it carries.
fn visit_head(head: &RuleHead, counts: &mut KindCounts) {
    match head {
        RuleHead::Compr { refr, assign, .. } => {
            bump(counts, NodeKind::HeadCompr);
            visit_expr(refr, counts);
            visit_optional_assign(assign.as_ref(), counts);
        }
        RuleHead::Set { refr, key, .. } => {
            bump(counts, NodeKind::HeadSet);
            visit_expr(refr, counts);
            visit_optional_expr(key.as_ref(), counts);
        }
        RuleHead::Func {
            refr, args, assign, ..
        } => {
            bump(counts, NodeKind::HeadFunc);
            visit_expr(refr, counts);
            visit_all(args, counts);
            visit_optional_assign(assign.as_ref(), counts);
        }
    }
}

/// Walk a rule body: its optional value assignment and its query.
fn visit_body(body: &RuleBody, counts: &mut KindCounts) {
    visit_optional_assign(body.assign.as_ref(), counts);
    visit_query(&body.query, counts);
}

/// Walk an optional rule-body assignment.
fn visit_optional_assign(assign: Option<&RuleAssign>, counts: &mut KindCounts) {
    if let Some(pending) = assign {
        bump(counts, assign_op_kind(&pending.op));
        visit_expr(&pending.value, counts);
    }
}

/// Walk a literal statement.
fn visit_literal(literal: &Literal, counts: &mut KindCounts) {
    match literal {
        Literal::SomeVars { .. } => bump(counts, NodeKind::LiteralSomeVars),
        Literal::SomeIn {
            key,
            value,
            collection,
            ..
        } => {
            bump(counts, NodeKind::LiteralSomeIn);
            visit_optional_expr(key.as_ref(), counts);
            visit_expr(value, counts);
            visit_expr(collection, counts);
        }
        Literal::Expr { expr, .. } => {
            bump(counts, NodeKind::LiteralExpr);
            visit_expr(expr, counts);
        }
        Literal::NotExpr { expr, .. } => {
            bump(counts, NodeKind::LiteralNotExpr);
            visit_expr(expr, counts);
        }
        Literal::Every { domain, query, .. } => {
            bump(counts, NodeKind::LiteralEvery);
            visit_expr(domain, counts);
            visit_query(query, counts);
        }
    }
}

/// Walk an expression and every expression it contains.
fn visit_expr(expr: &Expr, counts: &mut KindCounts) {
    match expr {
        Expr::String { .. } => bump(counts, NodeKind::ExprString),
        Expr::RawString { .. } => bump(counts, NodeKind::ExprRawString),
        Expr::Number { .. } => bump(counts, NodeKind::ExprNumber),
        Expr::Bool { .. } => bump(counts, NodeKind::ExprBool),
        Expr::Null { .. } => bump(counts, NodeKind::ExprNull),
        Expr::Var { .. } => bump(counts, NodeKind::ExprVar),
        Expr::Array { items, .. } => visit_collection(counts, NodeKind::ExprArray, items),
        Expr::Set { items, .. } => visit_collection(counts, NodeKind::ExprSet, items),
        Expr::Object { fields, .. } => {
            bump(counts, NodeKind::ExprObject);
            for (_, key, value) in fields {
                visit_expr(key, counts);
                visit_expr(value, counts);
            }
        }
        Expr::ArrayCompr { term, query, .. } => {
            visit_comprehension(counts, NodeKind::ExprArrayCompr, term, query);
        }
        Expr::SetCompr { term, query, .. } => {
            visit_comprehension(counts, NodeKind::ExprSetCompr, term, query);
        }
        Expr::ObjectCompr {
            key, value, query, ..
        } => {
            visit_comprehension(counts, NodeKind::ExprObjectCompr, key, query);
            visit_expr(value, counts);
        }
        Expr::Call { fcn, params, .. } => visit_call(counts, fcn, params),
        Expr::UnaryExpr { expr: inner, .. } => visit_single(counts, NodeKind::ExprUnary, inner),
        Expr::RefDot { refr, .. } => visit_single(counts, NodeKind::ExprRefDot, refr),
        Expr::RefBrack { refr, index, .. } => {
            bump(counts, NodeKind::ExprRefBrack);
            visit_expr(refr, counts);
            visit_expr(index, counts);
        }
        Expr::BinExpr { op, lhs, rhs, .. } => {
            visit_operator(
                counts,
                &Binary::new(NodeKind::ExprBin, bin_op_kind(op), lhs, rhs),
            );
        }
        Expr::BoolExpr { op, lhs, rhs, .. } => {
            visit_operator(
                counts,
                &Binary::new(NodeKind::ExprCompare, bool_op_kind(op), lhs, rhs),
            );
        }
        Expr::ArithExpr { op, lhs, rhs, .. } => {
            visit_operator(
                counts,
                &Binary::new(NodeKind::ExprArith, arith_op_kind(op), lhs, rhs),
            );
        }
        Expr::AssignExpr { op, lhs, rhs, .. } => {
            visit_operator(
                counts,
                &Binary::new(NodeKind::ExprAssign, assign_op_kind(op), lhs, rhs),
            );
        }
        Expr::Membership {
            key,
            value,
            collection,
            ..
        } => visit_membership(counts, key.as_ref(), value, collection),
    }
}

/// Walk every expression in a sequence.
fn visit_all(exprs: &[NodeRef<Expr>], counts: &mut KindCounts) {
    for expr in exprs {
        visit_expr(expr, counts);
    }
}

/// Walk an optional expression.
fn visit_optional_expr(expr: Option<&NodeRef<Expr>>, counts: &mut KindCounts) {
    if let Some(node) = expr {
        visit_expr(node, counts);
    }
}

/// Walk a collection literal: its kind and its items.
fn visit_collection(counts: &mut KindCounts, kind: NodeKind, items: &[NodeRef<Expr>]) {
    bump(counts, kind);
    visit_all(items, counts);
}

/// Walk a call: its function and its parameters.
fn visit_call(counts: &mut KindCounts, function: &NodeRef<Expr>, params: &[NodeRef<Expr>]) {
    bump(counts, NodeKind::ExprCall);
    visit_expr(function, counts);
    visit_all(params, counts);
}

/// Walk a wrapper expression: its kind and the expression it wraps.
fn visit_single(counts: &mut KindCounts, kind: NodeKind, inner: &NodeRef<Expr>) {
    bump(counts, kind);
    visit_expr(inner, counts);
}

/// Walk a membership expression: its optional key, value, and collection.
fn visit_membership(
    counts: &mut KindCounts,
    key: Option<&NodeRef<Expr>>,
    value: &NodeRef<Expr>,
    collection: &NodeRef<Expr>,
) {
    bump(counts, NodeKind::ExprMembership);
    visit_optional_expr(key, counts);
    visit_expr(value, counts);
    visit_expr(collection, counts);
}

/// Walk a comprehension: its term (or key) and its body.
fn visit_comprehension(
    counts: &mut KindCounts,
    kind: NodeKind,
    term: &NodeRef<Expr>,
    query: &Query,
) {
    bump(counts, kind);
    visit_expr(term, counts);
    visit_query(query, counts);
}

/// A binary expression's contribution: its kinds and its operands.
struct Binary<'a> {
    /// The expression kind.
    expression: NodeKind,
    /// The operator kind.
    operator: NodeKind,
    /// The left operand.
    lhs: &'a NodeRef<Expr>,
    /// The right operand.
    rhs: &'a NodeRef<Expr>,
}

impl<'a> Binary<'a> {
    /// Build one binary expression's contribution.
    const fn new(
        expression: NodeKind,
        operator: NodeKind,
        lhs: &'a NodeRef<Expr>,
        rhs: &'a NodeRef<Expr>,
    ) -> Self {
        Self {
            expression,
            operator,
            lhs,
            rhs,
        }
    }
}

/// Walk a binary expression: its kinds and both operands.
fn visit_operator(counts: &mut KindCounts, binary: &Binary<'_>) {
    bump(counts, binary.expression);
    bump(counts, binary.operator);
    visit_expr(binary.lhs, counts);
    visit_expr(binary.rhs, counts);
}

/// Walk every statement in a query.
fn visit_query(query: &Query, counts: &mut KindCounts) {
    for statement in &query.stmts {
        visit_literal(&statement.literal, counts);
        for with_mod in &statement.with_mods {
            visit_expr(&with_mod.refr, counts);
            visit_expr(&with_mod.r#as, counts);
        }
    }
}

/// Map a binary set operator to its kind.
const fn bin_op_kind(op: &BinOp) -> NodeKind {
    match op {
        BinOp::Intersection => NodeKind::BinOpIntersection,
        BinOp::Union => NodeKind::BinOpUnion,
    }
}

/// Map an arithmetic operator to its kind.
const fn arith_op_kind(op: &ArithOp) -> NodeKind {
    match op {
        ArithOp::Add => NodeKind::ArithOpAdd,
        ArithOp::Sub => NodeKind::ArithOpSub,
        ArithOp::Mul => NodeKind::ArithOpMul,
        ArithOp::Div => NodeKind::ArithOpDiv,
        ArithOp::Mod => NodeKind::ArithOpMod,
    }
}

/// Map a comparison operator to its kind.
const fn bool_op_kind(op: &BoolOp) -> NodeKind {
    match op {
        BoolOp::Lt => NodeKind::BoolOpLt,
        BoolOp::Le => NodeKind::BoolOpLe,
        BoolOp::Eq => NodeKind::BoolOpEq,
        BoolOp::Ge => NodeKind::BoolOpGe,
        BoolOp::Gt => NodeKind::BoolOpGt,
        BoolOp::Ne => NodeKind::BoolOpNe,
    }
}

/// Map an assignment operator to its kind.
const fn assign_op_kind(op: &AssignOp) -> NodeKind {
    match op {
        AssignOp::Eq => NodeKind::AssignOpEq,
        AssignOp::ColEq => NodeKind::AssignOpColEq,
    }
}
