//! The audit requests the corpus settles against.
//!
//! Each entry names a fixture, the module the worker audits, the roots the
//! request asks for, and the expected statement tree. The trees are built from
//! Lean's term encoding rather than copied from a report: `Rule.mk` applied to
//! its three fields, `List.cons` and `List.nil` with their implicit element
//! type, and nullary constructors for the leaves. An accepted statement is
//! therefore an independent claim about what Lean elaborates, not a
//! transcription of what the worker said.
//!
//! The expected trees are data the settlement compares exactly. A change in
//! Lean's elaboration shows up here as a failing settlement, which is the
//! signal that the pinned toolchain moved.

use crate::audit::{
    settlement::decision::{AuditRequest, RequestRoot},
    wire::{Component, Name, Tree, name_key, tree_key},
};

/// The module every fixture lives in.
const MODULE: &str = "AuditFixture";

/// The three pinned foundations, as the request spells them.
const FOUNDATIONS: &[&[&str]] = &[&["propext"], &["Quot", "sound"], &["Classical", "choice"]];

/// The foundations a narrowed request approves: everything but `propext`.
const NARROWED: &[&[&str]] = &[&["Quot", "sound"], &["Classical", "choice"]];

/// One corpus entry: what to run and what the request asks.
pub struct Entry {
    /// The manifest's fixture name.
    pub fixture: String,
    /// The module to audit, in dotted form.
    pub module: String,
    /// The roots to request, in report order.
    pub roots: Vec<String>,
    /// The request settlement decides against.
    pub request: AuditRequest,
}

/// The corpus entry for one manifest fixture.
///
/// # Errors
///
/// Returns a diagnostic when the fixture has no request, which means the
/// manifest and this table have drifted apart.
pub fn entry(fixture: &str) -> Result<Entry, String> {
    let module = format!("{MODULE}.{fixture}");
    let goal = goal();
    let (roots, expected, foundations): (Vec<String>, Vec<Tree>, &[&[&str]]) = match fixture {
        // These fixtures ask for the same thing — one `root`, the goal as the
        // expected statement, no approved foundations — whether the report is
        // expected to be accepted or rejected. Only the report differs, so one
        // arm states the shared request rather than repeating it per outcome.
        "AcceptedGround" | "InitSentinel" | "ImportedBase" | "Sorry" | "SorryImported"
        | "Native" | "Undeclared" | "ReachImported" | "WrongStatement" | "Unfolded"
        | "AxiomRoot" | "DefRoot" | "RootAbsent" | "ForgeInit" | "ForgeImported"
        | "ModuleAbsent" | "WorkerGarbled" => (roots(&module, &["root"]), vec![goal], &[]),
        "AcceptedFoundations" => (roots(&module, &["root"]), vec![goal], FOUNDATIONS),
        "AcceptedClaimShape" | "WrongPolicyLiteral" => {
            (roots(&module, &["root"]), vec![claim_shape_policy()], &[])
        }
        "MaximalPolicy" => (roots(&module, &["root"]), vec![maximal_policy()], &[]),
        "MixedRoots" => (
            roots(&module, &["clean", "root"]),
            vec![goal.clone(), goal],
            &[],
        ),
        "NarrowedFoundations" => (roots(&module, &["root"]), vec![goal], NARROWED),
        other => return Err(format!("manifest fixture `{other}` has no corpus request")),
    };
    let request = AuditRequest {
        module: dotted_key(&module),
        foundations: foundations.iter().map(|parts| key(parts)).collect(),
        roots: roots
            .iter()
            .zip(expected.iter())
            .map(|(name, tree)| RequestRoot {
                name: dotted_key(name),
                expected: tree_key(tree),
            })
            .collect(),
    };
    Ok(Entry {
        fixture: fixture.to_owned(),
        module,
        roots,
        request,
    })
}

/// The dotted names of `leaves`, each under `module`.
#[must_use]
fn roots(module: &str, leaves: &[&str]) -> Vec<String> {
    leaves
        .iter()
        .map(|leaf| format!("{module}.{leaf}"))
        .collect()
}

/// A name from its components.
#[must_use]
fn name(parts: &[&str]) -> Name {
    parts
        .iter()
        .map(|part| Component::Text((*part).to_owned()))
        .collect()
}

/// The key of a name from its components.
#[must_use]
fn key(parts: &[&str]) -> Vec<u8> { name_key(&name(parts)) }

/// The key of a dotted name.
#[must_use]
fn dotted_key(dotted: &str) -> Vec<u8> {
    let parts: Vec<&str> = dotted.split('.').collect();
    key(&parts)
}

/// A constant tree.
#[must_use]
fn constant(parts: &[&str], levels: &[u32]) -> Tree {
    Tree::Const {
        name: name(parts),
        levels: levels.to_vec(),
    }
}

/// An application tree.
#[must_use]
fn apply(function: Tree, argument: Tree) -> Tree {
    Tree::App(Box::new(function), Box::new(argument))
}

/// A `List` literal of `items` over the element type `element`.
#[must_use]
fn list(element: &[&str], items: &[Tree]) -> Tree {
    let nil = apply(constant(&["List", "nil"], &[0]), constant(element, &[]));
    items.iter().rev().fold(nil, |tail, item| {
        let cons = apply(
            apply(constant(&["List", "cons"], &[0]), constant(element, &[])),
            item.clone(),
        );
        apply(cons, tail)
    })
}

/// The atom with the given index, as `Atom.aNN`.
#[must_use]
fn atom(index: usize) -> Tree {
    let leaf = format!("a{index:02}");
    constant(&["AuditFixture", "Claim", "Atom", leaf.as_str()], &[])
}

/// One rule structure literal.
#[must_use]
fn rule(effect: &str, subject: usize, actions: &[usize]) -> Tree {
    let effect_tree = constant(&["AuditFixture", "Claim", "Effect", effect], &[]);
    let action_trees: Vec<Tree> = actions.iter().copied().map(atom).collect();
    let actions_tree = list(&["AuditFixture", "Claim", "Atom"], &action_trees);
    let subject_tree = atom(subject);
    apply(
        apply(
            apply(
                constant(&["AuditFixture", "Claim", "Rule", "mk"], &[]),
                effect_tree,
            ),
            subject_tree,
        ),
        actions_tree,
    )
}

/// A `Claim.Holds` application over `rules`.
#[must_use]
fn holds(rules: &[Tree]) -> Tree {
    apply(
        constant(&["AuditFixture", "Claim", "Holds"], &[]),
        list(&["AuditFixture", "Claim", "Rule"], rules),
    )
}

/// The proposition every accepted fixture's root proves.
#[must_use]
fn goal() -> Tree { constant(&["AuditFixture", "Goal"], &[]) }

/// The two-rule policy `AcceptedClaimShape` states and `WrongPolicyLiteral`
/// is measured against.
#[must_use]
fn claim_shape_policy() -> Tree { holds(&[rule("allow", 0, &[0, 1]), rule("deny", 1, &[2])]) }

/// The sixteen-rule, sixteen-atom policy `MaximalPolicy` states.
#[must_use]
fn maximal_policy() -> Tree {
    let rules: Vec<Tree> = (0..16)
        .map(|index| {
            let effect = if index & 1 == 0 { "allow" } else { "deny" };
            let actions: Vec<usize> = (0..16).collect();
            rule(effect, index, &actions)
        })
        .collect();
    holds(&rules)
}
