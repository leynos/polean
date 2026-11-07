//! Shared request and report pairs for the audit tests.
//!
//! Every test names the same corpus fixtures the plan's Table 2 lists, and the
//! builders here are the single place their bytes are constructed. A feature
//! scenario, a unit test, and a property test that all say "`Sorry`" therefore
//! decide the same report.
//!
//! These are fixtures, not a corpus: nothing here compiles Lean or runs the
//! worker. The provisioned corpus in `tests/toolchain/smoke/audit/` is what
//! exercises the real interface.
//!
//! The module holds the vocabulary every fixture shares and re-exports the
//! case builders in [`cases`], so callers keep naming `fixtures::accepted_ground`
//! and friends.

pub use crate::audit::wire::PIN;
use crate::audit::{
    settlement::decision::{
        AuditReport,
        AuditRequest,
        Axiom,
        Decl,
        DeclKind,
        Replay,
        RequestRoot,
        RootObs,
    },
    wire::{Component, Name, Tree, is_native, name_key, tree_key},
};

mod cases;

pub use cases::*;

/// The module every fixture lives in.
pub const MODULE: &str = "AuditFixture";

/// A request and the report that answers it.
pub struct Case {
    /// What the request asks.
    pub request: AuditRequest,
    /// What the worker reported.
    pub report: AuditReport,
    /// The toolchain identity settlement compares against.
    pub pin: Vec<u8>,
}

/// Build a name from its components.
#[must_use]
pub fn name(parts: &[&str]) -> Name {
    parts
        .iter()
        .map(|part| Component::Text((*part).to_owned()))
        .collect()
}

/// Build the key of a name from its components.
#[must_use]
pub fn key(parts: &[&str]) -> Vec<u8> { name_key(&name(parts)) }

/// The module a fixture lives in, as a name.
#[must_use]
pub fn module(fixture: &str) -> Name { name(&[MODULE, fixture]) }

/// The root a fixture defines, as a name.
#[must_use]
pub fn root_name(fixture: &str) -> Name { name(&[MODULE, fixture, "root"]) }

/// The module the corpus places the expected proposition in.
#[must_use]
pub fn expected_module() -> Name { name(&[MODULE, "Expected"]) }

/// The proposition every accepted fixture proves.
#[must_use]
pub fn goal() -> Tree {
    Tree::Const {
        name: name(&[MODULE, "Goal"]),
        levels: Vec::new(),
    }
}

/// The proposition a statement-mismatch fixture reports instead.
#[must_use]
pub fn other_goal() -> Tree {
    Tree::Const {
        name: name(&[MODULE, "Other"]),
        levels: Vec::new(),
    }
}

/// Build one reported axiom.
#[must_use]
pub fn axiom(parts: &[&str]) -> Axiom {
    let name = name(parts);
    Axiom {
        native: is_native(&name),
        name: name_key(&name),
    }
}

/// Build a theorem whose type is `expected` and whose axioms are `axioms`.
#[must_use]
pub fn theorem(expected: &Tree, axioms: &[&[&str]]) -> Decl {
    Decl {
        kind: DeclKind::Theorem,
        statement: Some(tree_key(expected)),
        axioms: axioms.iter().map(|parts| axiom(parts)).collect(),
    }
}

/// The request that asks a fixture's root to prove the goal.
#[must_use]
pub fn request(fixture: &str, foundations: &[&[&str]]) -> AuditRequest {
    AuditRequest {
        module: name_key(&module(fixture)),
        foundations: foundations
            .iter()
            .map(|parts| name_key(&name(parts)))
            .collect(),
        roots: vec![RequestRoot {
            name: name_key(&root_name(fixture)),
            expected: tree_key(&goal()),
        }],
    }
}

/// A report whose replay accepted the fixture and the expected module.
#[must_use]
pub fn accepted_report(fixture: &str, roots: Vec<RootObs>) -> AuditReport {
    AuditReport {
        githash: PIN.as_bytes().to_vec(),
        module: name_key(&module(fixture)),
        replay: Replay::Accepted(vec![
            name_key(&expected_module()),
            name_key(&module(fixture)),
        ]),
        roots,
    }
}

/// One observed root holding `decl`.
#[must_use]
pub fn observed_root(fixture: &str, decl: Option<Decl>) -> RootObs {
    RootObs {
        name: name_key(&root_name(fixture)),
        decl,
    }
}

/// The three pinned foundations, as the request spells them.
pub const FOUNDATIONS: &[&[&str]] = &[&["propext"], &["Quot", "sound"], &["Classical", "choice"]];
