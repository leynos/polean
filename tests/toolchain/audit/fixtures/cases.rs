//! One builder per corpus fixture.
//!
//! Each builder states only what distinguishes its fixture from the shared
//! vocabulary in the parent module: the request's foundations, the declaration
//! the worker reported, and the replay. Everything else comes from
//! [`super::accepted_report`] and [`super::theorem`], so an accepted fixture and
//! a rejected one differ in exactly one field.

use super::{
    Case,
    FOUNDATIONS,
    MODULE,
    PIN,
    accepted_report,
    goal,
    module,
    name,
    observed_root,
    other_goal,
    request,
    root_name,
    theorem,
};
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
    wire::{name_key, tree_key},
};

/// A pair whose single root is a theorem of the goal with no axioms.
#[must_use]
pub fn accepted_ground() -> Case {
    Case {
        request: request("AcceptedGround", &[]),
        report: accepted_report(
            "AcceptedGround",
            vec![observed_root("AcceptedGround", Some(theorem(&goal(), &[])))],
        ),
        pin: PIN.as_bytes().to_vec(),
    }
}

/// A pair whose theorem depends only on the three approved foundations.
#[must_use]
pub fn accepted_foundations() -> Case {
    Case {
        request: request("AcceptedFoundations", FOUNDATIONS),
        report: accepted_report(
            "AcceptedFoundations",
            vec![observed_root(
                "AcceptedFoundations",
                Some(theorem(&goal(), FOUNDATIONS)),
            )],
        ),
        pin: PIN.as_bytes().to_vec(),
    }
}

/// A pair whose theorem depends on `sorryAx`.
#[must_use]
pub fn sorry(fixture: &str) -> Case {
    Case {
        request: request(fixture, FOUNDATIONS),
        report: accepted_report(
            fixture,
            vec![observed_root(
                fixture,
                Some(theorem(&goal(), &[&["sorryAx"]])),
            )],
        ),
        pin: PIN.as_bytes().to_vec(),
    }
}

/// A pair whose theorem depends on a native-computation axiom.
#[must_use]
pub fn native() -> Case {
    Case {
        request: request("Native", FOUNDATIONS),
        report: accepted_report(
            "Native",
            vec![observed_root(
                "Native",
                Some(theorem(&goal(), &[&["_native", "decide"]])),
            )],
        ),
        pin: PIN.as_bytes().to_vec(),
    }
}

/// A pair whose theorem depends on an axiom the request never declares.
#[must_use]
pub fn undeclared(fixture: &str) -> Case {
    Case {
        request: request(fixture, FOUNDATIONS),
        report: accepted_report(
            fixture,
            vec![observed_root(
                fixture,
                Some(theorem(&goal(), &[&["Fancy", "assumption"]])),
            )],
        ),
        pin: PIN.as_bytes().to_vec(),
    }
}

/// A pair whose request approves only `propext` while the root depends on
/// `Classical.choice`.
#[must_use]
pub fn narrowed_foundations() -> Case {
    Case {
        request: request("NarrowedFoundations", &[&["propext"]]),
        report: accepted_report(
            "NarrowedFoundations",
            vec![observed_root(
                "NarrowedFoundations",
                Some(theorem(&goal(), &[&["Classical", "choice"]])),
            )],
        ),
        pin: PIN.as_bytes().to_vec(),
    }
}

/// A pair whose theorem proves a different proposition.
#[must_use]
pub fn wrong_statement(fixture: &str) -> Case {
    Case {
        request: request(fixture, FOUNDATIONS),
        report: accepted_report(
            fixture,
            vec![observed_root(
                fixture,
                Some(Decl {
                    kind: DeclKind::Theorem,
                    statement: Some(tree_key(&other_goal())),
                    axioms: Vec::new(),
                }),
            )],
        ),
        pin: PIN.as_bytes().to_vec(),
    }
}

/// A pair whose root is an axiom rather than a theorem.
///
/// The axiom's own name is reachable from itself, so the report carries it as
/// an axiom and settlement reports both the wrong kind and the unapproved
/// dependency.
#[must_use]
pub fn axiom_root() -> Case {
    let name = root_name("AxiomRoot");
    Case {
        request: request("AxiomRoot", FOUNDATIONS),
        report: accepted_report(
            "AxiomRoot",
            vec![RootObs {
                name: name_key(&name),
                decl: Some(Decl {
                    kind: DeclKind::Axiom,
                    statement: Some(tree_key(&goal())),
                    axioms: vec![Axiom {
                        name: name_key(&name),
                        native: false,
                    }],
                }),
            }],
        ),
        pin: PIN.as_bytes().to_vec(),
    }
}

/// A pair whose root is a definition rather than a theorem.
#[must_use]
pub fn def_root() -> Case {
    Case {
        request: request("DefRoot", FOUNDATIONS),
        report: accepted_report(
            "DefRoot",
            vec![observed_root(
                "DefRoot",
                Some(Decl {
                    kind: DeclKind::Definition,
                    statement: Some(tree_key(&goal())),
                    axioms: Vec::new(),
                }),
            )],
        ),
        pin: PIN.as_bytes().to_vec(),
    }
}

/// A pair whose report says the environment has no such constant.
#[must_use]
pub fn root_absent() -> Case {
    Case {
        request: request("RootAbsent", FOUNDATIONS),
        report: accepted_report("RootAbsent", vec![observed_root("RootAbsent", None)]),
        pin: PIN.as_bytes().to_vec(),
    }
}

/// A pair whose replay the kernel rejected.
#[must_use]
pub fn rejected_replay(fixture: &str) -> Case {
    Case {
        request: request(fixture, FOUNDATIONS),
        report: AuditReport {
            githash: PIN.as_bytes().to_vec(),
            module: name_key(&module(fixture)),
            replay: Replay::Rejected,
            roots: Vec::new(),
        },
        pin: PIN.as_bytes().to_vec(),
    }
}

/// A pair whose report names a different toolchain.
#[must_use]
pub fn toolchain_mismatch() -> Case {
    let mut case = accepted_ground();
    case.report.githash = b"0000000000000000000000000000000000000000".to_vec();
    case
}

/// A pair whose report answers for a different module.
#[must_use]
pub fn module_mismatch() -> Case {
    let mut case = accepted_ground();
    case.report.module = name_key(&module("Other"));
    case
}

/// A pair whose report omits the job module from the replayed set.
#[must_use]
pub fn coverage_mismatch() -> Case {
    let mut case = accepted_ground();
    case.report.replay = Replay::Accepted(vec![name_key(&super::expected_module())]);
    case
}

/// A pair whose report answers a different root than the request named.
#[must_use]
pub fn root_mismatch() -> Case {
    let mut case = accepted_ground();
    case.report.roots = vec![observed_root("Other", Some(theorem(&goal(), &[])))];
    case
}

/// A pair whose rejected replay still reports roots.
#[must_use]
pub fn rejected_with_roots() -> Case {
    let mut case = rejected_replay("ForgeInit");
    case.report.roots = vec![observed_root("ForgeInit", None)];
    case
}

/// A pair with one clean root and one root depending on `sorryAx`.
#[must_use]
pub fn mixed_roots() -> Case {
    let fixture = "MixedRoots";
    let clean = name(&[MODULE, fixture, "clean"]);
    let unsound = name(&[MODULE, fixture, "unsound"]);
    Case {
        request: AuditRequest {
            module: name_key(&module(fixture)),
            foundations: FOUNDATIONS
                .iter()
                .map(|parts| name_key(&name(parts)))
                .collect(),
            roots: vec![
                RequestRoot {
                    name: name_key(&clean),
                    expected: tree_key(&goal()),
                },
                RequestRoot {
                    name: name_key(&unsound),
                    expected: tree_key(&goal()),
                },
            ],
        },
        report: accepted_report(
            fixture,
            vec![
                RootObs {
                    name: name_key(&clean),
                    decl: Some(theorem(&goal(), &[])),
                },
                RootObs {
                    name: name_key(&unsound),
                    decl: Some(theorem(&goal(), &[&["sorryAx"]])),
                },
            ],
        ),
        pin: PIN.as_bytes().to_vec(),
    }
}

/// Build the case a feature scenario names.
///
/// # Panics
///
/// Panics when `fixture` is not one of the corpus fixtures this module knows,
/// because a scenario naming an unknown fixture is a defect in the feature
/// file rather than a policy outcome.
#[must_use]
pub fn case(fixture: &str) -> Case {
    match fixture {
        "AcceptedGround" => accepted_ground(),
        "AcceptedFoundations" => accepted_foundations(),
        "Sorry" | "SorryImported" => sorry(fixture),
        "Native" => native(),
        "Undeclared" | "ReachImported" => undeclared(fixture),
        "WrongStatement" | "WrongPolicyLiteral" | "Unfolded" => wrong_statement(fixture),
        "NarrowedFoundations" => narrowed_foundations(),
        "AxiomRoot" => axiom_root(),
        "DefRoot" => def_root(),
        "RootAbsent" => root_absent(),
        "ForgeInit" | "ForgeImported" => rejected_replay(fixture),
        "MixedRoots" => mixed_roots(),
        other => panic!("unknown audit fixture {other}"),
    }
}
