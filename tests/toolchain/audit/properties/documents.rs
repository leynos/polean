//! A2 across whole documents: generated valid documents decode to exactly the
//! keys their parts encode to, and generated near-boundary or malformed trees
//! are refused where the contract says they must be.
//!
//! [`super::keys`] proves the encoders injective, and [`super::contract`] proves
//! a required field is never defaulted. This module closes the gap between them:
//! conversion preserves the keys, and the node and depth bounds are enforced at
//! the boundary. A decode that reordered roots, dropped an axiom, or admitted an
//! over-bound tree would pass both of those modules.

use proptest::{
    arbitrary::any,
    collection::vec as strategy_vec,
    option::of as optional,
    prelude::{Strategy, TestCaseError, prop_assert, prop_assert_eq},
    sample::select,
};
use serde_json::{Value, json};

use super::support::{check, name_strategy, tree_strategy};
use crate::audit::{
    settlement::decision::{Axiom, Decl, DeclKind, Replay, RequestRoot, RootObs},
    wire::{
        Component,
        DecodeError,
        MAX_DEPTH,
        MAX_NODES,
        Name,
        PIN,
        Tree,
        decode_report,
        decode_request,
        is_native,
        name_key,
        tree_key,
    },
};

/// Components the request grammar admits.
///
/// A request name becomes a command-line argument, so every component matches
/// `[A-Za-z_][A-Za-z0-9_']*`. The report grammar is looser and draws from
/// [`name_strategy`] instead.
const REQUEST_COMPONENTS: &[&str] = &[
    "AuditFixture",
    "AcceptedGround",
    "Goal",
    "root",
    "x",
    "Root_1",
    "A'",
    "_private",
];

/// A request name of one to three components, every one argument-legal.
fn request_name_strategy() -> impl Strategy<Value = Name> {
    strategy_vec(
        select(REQUEST_COMPONENTS).prop_map(|text| Component::Text(text.to_owned())),
        1..4,
    )
}

/// A bounded tree whose constants are request names.
fn request_tree_strategy() -> impl Strategy<Value = Tree> {
    let leaf = (request_name_strategy(), strategy_vec(any::<u32>(), 0..3))
        .prop_map(|(name, levels)| Tree::Const { name, levels });
    leaf.prop_recursive(4, 32, 2, |inner| {
        (inner.clone(), inner).prop_map(|(left, right)| Tree::App(Box::new(left), Box::new(right)))
    })
}

/// One root in a generated report.
#[derive(Debug)]
struct GeneratedRoot {
    /// The observed root's name.
    name: Name,
    /// Whether the environment held a declaration for the root.
    declared: bool,
    /// The declaration's type, or `None` for an out-of-grammar statement.
    statement: Option<Tree>,
    /// Every axiom reachable from the declaration.
    axioms: Vec<Name>,
}

/// A report root, varied over declaration presence, statement, and axioms.
fn generated_root_strategy() -> impl Strategy<Value = GeneratedRoot> {
    (
        name_strategy(),
        any::<bool>(),
        optional(tree_strategy()),
        strategy_vec(name_strategy(), 0..3),
    )
        .prop_map(|(name, declared, statement, axioms)| GeneratedRoot {
            name,
            declared,
            statement: if declared { statement } else { None },
            axioms: if declared { axioms } else { Vec::new() },
        })
}

/// A name built from literal text components.
fn path(components: &[&str]) -> Name {
    components
        .iter()
        .map(|text| Component::Text((*text).to_owned()))
        .collect()
}

/// The JSON the contracts spell one name component as.
fn component_json(component: &Component) -> Value {
    match component {
        Component::Text(text) => Value::from(text.clone()),
        Component::Number(number) => Value::from(*number),
    }
}

/// The JSON the contracts spell a name as.
fn name_json(name: &Name) -> Value { Value::Array(name.iter().map(component_json).collect()) }

/// The JSON the contracts spell a tree as.
fn tree_json(tree: &Tree) -> Value {
    match tree {
        Tree::Const { name, levels } => json!({"const": name_json(name), "levels": levels}),
        Tree::App(left, right) => json!({"app": [tree_json(left), tree_json(right)]}),
    }
}

/// Serialize a generated document, which must always succeed.
fn serialized(document: Value) -> Vec<u8> {
    match serde_json::to_vec(&document) {
        Ok(bytes) => bytes,
        Err(error) => panic!("a generated document must serialize: {error}"),
    }
}

/// A request document over a module, roots, and foundations.
fn request_document(module: &Name, roots: &[(Name, Tree)], foundations: &[Name]) -> Vec<u8> {
    let roots: Vec<Value> = roots
        .iter()
        .map(|(name, tree)| json!({"name": name_json(name), "expected": tree_json(tree)}))
        .collect();
    let foundations: Vec<Value> = foundations.iter().map(name_json).collect();
    serialized(json!({
        "schema_version": "polean.audit-request.v0",
        "module": name_json(module),
        "roots": roots,
        "foundations": foundations,
    }))
}

/// The JSON one generated root's declaration takes.
fn declaration_json(root: &GeneratedRoot) -> Value {
    if !root.declared {
        return Value::Null;
    }
    let statement = root.statement.as_ref().map_or(Value::Null, tree_json);
    let axioms: Vec<Value> = root.axioms.iter().map(name_json).collect();
    json!({"kind": "theorem", "statement": statement, "axioms": axioms})
}

/// A report document with an accepted replay over `modules`.
fn report_document(module: &Name, modules: &[Name], roots: &[GeneratedRoot]) -> Vec<u8> {
    let modules: Vec<Value> = modules.iter().map(name_json).collect();
    let roots: Vec<Value> = roots
        .iter()
        .map(|root| json!({"name": name_json(&root.name), "declaration": declaration_json(root)}))
        .collect();
    serialized(json!({
        "schema_version": "polean.audit-report.v0",
        "githash": PIN,
        "module": name_json(module),
        "replay": {"status": "accepted", "modules": modules},
        "roots": roots,
    }))
}

/// A report whose one root has a declaration with `tree` as its statement.
fn statement_document(tree: &Tree) -> Vec<u8> {
    let module = path(&["AuditFixture", "AcceptedGround"]);
    let root = GeneratedRoot {
        name: path(&["AuditFixture", "AcceptedGround", "root"]),
        declared: true,
        statement: Some(tree.clone()),
        axioms: Vec::new(),
    };
    report_document(&module, &[], &[root])
}

/// The foundations without repeats, in first-seen order.
///
/// The decoder refuses a repeated foundation and independent draws can repeat
/// one, so deduplication keeps the document one the contract admits.
fn distinct(names: Vec<Name>) -> Vec<Name> {
    let mut kept: Vec<Name> = Vec::new();
    for name in names {
        if !kept.contains(&name) {
            kept.push(name);
        }
    }
    kept
}

/// A full tree of `levels` application levels, with `2^(levels + 1) - 1` nodes.
///
/// The depths below straddle the node bound while staying inside the depth bound.
fn balanced(levels: u32) -> Tree {
    if levels == 0 {
        return Tree::Const {
            name: path(&["AuditFixture", "Leaf"]),
            levels: Vec::new(),
        };
    }
    let child = balanced(levels - 1);
    Tree::App(Box::new(child.clone()), Box::new(child))
}

/// A right-leaning spine of `apps` applications, with the given depth minus one.
fn spine(apps: u32) -> Tree {
    let leaf = Tree::Const {
        name: path(&["AuditFixture", "Leaf"]),
        levels: Vec::new(),
    };
    let mut tree = leaf.clone();
    for _ in 0..apps {
        tree = Tree::App(Box::new(tree), Box::new(leaf.clone()));
    }
    tree
}

/// A request decodes to the keys its parts encode to, in request order.
#[test]
fn valid_requests_decode_to_their_keys() {
    let strategy = (
        request_name_strategy(),
        strategy_vec((request_name_strategy(), request_tree_strategy()), 1..3),
        strategy_vec(request_name_strategy(), 0..3),
    );
    check(strategy, |(module, roots, foundations)| {
        let foundations = distinct(foundations);
        let bytes = request_document(&module, &roots, &foundations);
        let request = decode_request(&bytes).map_err(|error| {
            TestCaseError::fail(format!("a generated request was refused: {error:?}"))
        })?;
        prop_assert_eq!(&request.module, &name_key(&module));
        let expected_roots: Vec<RequestRoot> = roots
            .iter()
            .map(|(name, tree)| RequestRoot {
                name: name_key(name),
                expected: tree_key(tree),
            })
            .collect();
        prop_assert_eq!(&request.roots, &expected_roots);
        let expected_foundations: Vec<Vec<u8>> =
            foundations.iter().map(|name| name_key(name)).collect();
        prop_assert_eq!(&request.foundations, &expected_foundations);
        Ok(())
    });
}

/// A report decodes to the keys its parts encode to, in report order.
#[test]
fn valid_reports_decode_to_their_keys() {
    let strategy = (
        name_strategy(),
        strategy_vec(name_strategy(), 0..3),
        strategy_vec(generated_root_strategy(), 0..3),
    );
    check(strategy, |(module, modules, roots)| {
        let bytes = report_document(&module, &modules, &roots);
        let report = decode_report(&bytes).map_err(|error| {
            TestCaseError::fail(format!("a generated report was refused: {error:?}"))
        })?;
        prop_assert_eq!(&report.githash, PIN.as_bytes());
        prop_assert_eq!(&report.module, &name_key(&module));
        let replayed: Vec<Vec<u8>> = modules.iter().map(|name| name_key(name)).collect();
        prop_assert_eq!(&report.replay, &Replay::Accepted(replayed));
        let expected: Vec<RootObs> = roots
            .iter()
            .map(|root| RootObs {
                name: name_key(&root.name),
                decl: root.declared.then(|| Decl {
                    kind: DeclKind::Theorem,
                    statement: root.statement.as_ref().map(tree_key),
                    axioms: root
                        .axioms
                        .iter()
                        .map(|name| Axiom {
                            name: name_key(name),
                            native: is_native(name),
                        })
                        .collect(),
                }),
            })
            .collect();
        prop_assert_eq!(&report.roots, &expected);
        Ok(())
    });
}

/// A tree is admitted exactly while its node count is inside the bound.
///
/// Depths 10 to 12 give 2,047, 4,095, and 8,191 nodes: the nearest admissible
/// count below the bound and the first above it. A binary tree's node count is
/// always odd, so 4,096 is unreachable and 4,095 is the edge the contract admits.
#[test]
fn trees_are_admitted_exactly_up_to_the_node_bound() {
    check(10u32..=12, |levels| {
        let tree = balanced(levels);
        let count = 2usize.pow(levels + 1) - 1;
        let document = statement_document(&tree);
        let decoded = decode_report(&document);
        if count <= MAX_NODES {
            prop_assert!(
                decoded.is_ok(),
                "a tree of {count} nodes was refused: {decoded:?}"
            );
        } else {
            prop_assert!(
                matches!(decoded, Err(DecodeError::Malformed(_))),
                "a tree of {count} nodes was admitted"
            );
        }
        Ok(())
    });
}

/// A tree is admitted exactly while its depth is inside the bound.
///
/// A spine of `apps` applications has depth `apps + 1`, so the range crosses the
/// bound at 59 applications. Its node count stays at 119, so a refusal is
/// attributable to the depth.
#[test]
fn trees_are_admitted_exactly_up_to_the_depth_bound() {
    check(57u32..=61, |apps| {
        let tree = spine(apps);
        let depth = apps as usize + 1;
        let document = statement_document(&tree);
        let decoded = decode_report(&document);
        if depth <= MAX_DEPTH {
            prop_assert!(
                decoded.is_ok(),
                "a tree of depth {depth} was refused: {decoded:?}"
            );
        } else {
            prop_assert!(
                matches!(decoded, Err(DecodeError::Malformed(_))),
                "a tree of depth {depth} was admitted"
            );
        }
        Ok(())
    });
}

/// Every malformed tree shape is refused, whatever tree it wraps.
///
/// The mutations are the three shapes a lenient decoder would read as a tree:
/// an application of more than two operands, a constant that also carries an
/// application, and a level list of the wrong type.
#[test]
fn malformed_tree_shapes_are_refused() {
    let strategy = (tree_strategy(), 0usize..3);
    check(strategy, |(tree, shape)| {
        let object = tree_json(&tree);
        let mutated = match shape {
            0 => json!({"app": [object.clone(), object.clone(), object]}),
            1 => {
                json!({"const": ["AuditFixture", "Goal"], "levels": [], "app": [object.clone(), object]})
            }
            _ => json!({"const": ["AuditFixture", "Goal"], "levels": ["zero"]}),
        };
        let bytes = serialized(json!({
            "schema_version": "polean.audit-report.v0",
            "githash": PIN,
            "module": name_json(&path(&["AuditFixture", "AcceptedGround"])),
            "replay": {"status": "accepted", "modules": []},
            "roots": [{
                "name": name_json(&path(&["AuditFixture", "AcceptedGround", "root"])),
                "declaration": {"kind": "theorem", "statement": mutated, "axioms": []},
            }],
        }));
        prop_assert!(
            matches!(decode_report(&bytes), Err(DecodeError::Malformed(_))),
            "malformed tree shape {shape} was admitted"
        );
        Ok(())
    });
}
