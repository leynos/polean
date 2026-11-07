//! The pure settlement function: the only authority that accepts a report.
//!
//! Settlement compares a decoded request with a decoded report and returns one
//! of three outcomes. It never trusts the worker's exit status, and the report
//! carries no verdict of its own, so acceptance is a property of the data
//! alone.
//!
//! - [`Settlement::Accepted`] means the request is valid, the report answers exactly that request,
//!   the kernel accepted the replay, and every root is a theorem of the expected proposition whose
//!   transitive axioms are all approved.
//! - [`Settlement::Rejected`] means the report is a faithful answer to the request and the answer
//!   is no. Every root keeps its own findings, so a mixed report is not reduced to its first fault.
//! - [`Settlement::ToolFailure`] means the report does not answer the request at all: a different
//!   toolchain, a different module, a coverage mismatch, or a rejected replay that still claims
//!   roots. A protocol breach takes precedence over findings, because findings about a report that
//!   answers a different question would be misleading.
//!
//! [`RequestRefusal`] is separate from all three. It says the request itself
//! is not admissible, so no report could settle it.
//!
//! # Verification
//!
//! This file is the source `verus/audit_settlement.rs` verifies. It is written
//! inside `verus!` so the executable code and the verified code are the same
//! text, and it depends on nothing outside the standard library: no `serde`,
//! no I/O, and no Lean. The values it compares are in [`types`]; the pinned
//! foundation keys are literal bytes that a wire test reproduces from real
//! names.

use vstd::prelude::*;

mod types;

pub use self::types::*;

verus! {

/// Whether `name` is one of the three pinned foundations.
fn pinned(name: &[u8]) -> bool {
    name == K_PROPEXT.as_slice() || name == K_QUOT.as_slice() || name == K_CHOICE.as_slice()
}

/// Whether `name` appears in `keys`.
fn contains(keys: &[Key], name: &[u8]) -> bool {
    let mut index = 0;
    while index < keys.len() {
        let Some(candidate) = keys.get(index) else {
            break;
        };
        if candidate.as_slice() == name {
            return true;
        }
        index += 1;
    }
    false
}

/// Whether the request declares `name` an approved foundation.
fn approved(request: &AuditRequest, name: &[u8]) -> bool {
    contains(&request.foundations, name)
}

/// Refuse a request that names no roots, repeats a root, or approves a
/// foundation outside the pinned table.
///
/// # Errors
///
/// Returns the first refusal in that order, naming the offending root or
/// foundation by position.
pub fn validate_request(request: &AuditRequest) -> Result<(), RequestRefusal> {
    if request.roots.is_empty() {
        return Err(RequestRefusal::NoRoots);
    }
    if let Some(index) = duplicate_root(request) {
        return Err(RequestRefusal::DuplicateRoot(index));
    }
    if let Some(index) = unapproved_foundation(request) {
        return Err(RequestRefusal::UnapprovedFoundation(index));
    }
    Ok(())
}

/// The position of the first root whose name repeats an earlier root's.
fn duplicate_root(request: &AuditRequest) -> Option<usize> {
    let mut index = 0;
    while index < request.roots.len() {
        let Some(root) = request.roots.get(index) else {
            break;
        };
        if repeats_earlier_root(request, root, index) {
            return Some(index);
        }
        index += 1;
    }
    None
}

/// Whether `root` has the same name as a root before position `index`.
fn repeats_earlier_root(request: &AuditRequest, root: &RequestRoot, index: usize) -> bool {
    let mut earlier = 0;
    while earlier < index {
        let Some(other) = request.roots.get(earlier) else {
            break;
        };
        if other.name == root.name {
            return true;
        }
        earlier += 1;
    }
    false
}

/// The position of the first declared foundation outside the pinned table.
fn unapproved_foundation(request: &AuditRequest) -> Option<usize> {
    let mut index = 0;
    while index < request.foundations.len() {
        let Some(name) = request.foundations.get(index) else {
            break;
        };
        if !pinned(name) {
            return Some(index);
        }
        index += 1;
    }
    None
}

/// Decide a report against a request.
///
/// # Errors
///
/// Returns the [`RequestRefusal`] from [`validate_request`] when the request is
/// not admissible. A report that does not answer an admissible request is
/// [`Settlement::ToolFailure`], not an error.
pub fn settle(
    request: &AuditRequest,
    report: &AuditReport,
    pin: &[u8],
) -> Result<Settlement, RequestRefusal> {
    validate_request(request)?;
    if let Some(breach) = protocol_breach(request, report, pin) {
        return Ok(Settlement::ToolFailure(breach));
    }
    match &report.replay {
        Replay::Rejected => Ok(Settlement::Rejected(vec![Finding {
            root: None,
            item: None,
            code: FindingCode::KernelRejected,
        }])),
        Replay::Accepted(_) => {
            let findings = root_findings(request, report);
            if findings.is_empty() {
                Ok(Settlement::Accepted)
            } else {
                Ok(Settlement::Rejected(findings))
            }
        }
    }
}

/// The first way the report fails to answer the request, if any.
fn protocol_breach(
    request: &AuditRequest,
    report: &AuditReport,
    pin: &[u8],
) -> Option<ProtocolBreach> {
    if report.githash.as_slice() != pin {
        return Some(ProtocolBreach::ToolchainMismatch);
    }
    if report.module != request.module {
        return Some(ProtocolBreach::ModuleMismatch);
    }
    replay_breach(request, report)
}

/// The ways a replay can fail to answer the request.
fn replay_breach(request: &AuditRequest, report: &AuditReport) -> Option<ProtocolBreach> {
    match &report.replay {
        Replay::Rejected => rejected_breach(report),
        Replay::Accepted(modules) => accepted_breach(request, report, modules),
    }
}

/// A rejected replay must report no roots, because nothing was observed.
const fn rejected_breach(report: &AuditReport) -> Option<ProtocolBreach> {
    if report.roots.is_empty() {
        None
    } else {
        Some(ProtocolBreach::RejectedReplayWithRoots)
    }
}

/// An accepted replay must cover the job module and answer every root in
/// order.
fn accepted_breach(
    request: &AuditRequest,
    report: &AuditReport,
    modules: &[Key],
) -> Option<ProtocolBreach> {
    if !contains(modules, &request.module) {
        return Some(ProtocolBreach::CoverageMismatch);
    }
    if report.roots.len() != request.roots.len() {
        return Some(ProtocolBreach::CoverageMismatch);
    }
    if roots_differ(request, report) {
        return Some(ProtocolBreach::CoverageMismatch);
    }
    None
}

/// Whether the report's roots are not the request's roots in the same order.
fn roots_differ(request: &AuditRequest, report: &AuditReport) -> bool {
    let mut index = 0;
    while index < report.roots.len() {
        let Some(observed) = report.roots.get(index) else {
            break;
        };
        let Some(requested) = request.roots.get(index) else {
            break;
        };
        if observed.name != requested.name {
            return true;
        }
        index += 1;
    }
    false
}

/// Every finding about every root, in root order and then axiom order.
fn root_findings(request: &AuditRequest, report: &AuditReport) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut index = 0;
    while index < report.roots.len() {
        let Some(observed) = report.roots.get(index) else {
            break;
        };
        let Some(requested) = request.roots.get(index) else {
            break;
        };
        let pair = RootPair { requested, observed };
        collect_root_findings(request, index, pair, &mut findings);
        index += 1;
    }
    findings
}

/// One requested root beside the observation that answered it.
#[derive(Clone, Copy)]
struct RootPair<'a> {
    /// What the request asked for.
    requested: &'a RequestRoot,
    /// What the worker observed.
    observed: &'a RootObs,
}

/// Append the findings about one root.
fn collect_root_findings(
    request: &AuditRequest,
    index: usize,
    pair: RootPair<'_>,
    findings: &mut Vec<Finding>,
) {
    let Some(declaration) = &pair.observed.decl else {
        findings.push(Finding {
            root: Some(index),
            item: None,
            code: FindingCode::RootAbsent,
        });
        return;
    };
    if declaration.kind != DeclKind::Theorem {
        findings.push(Finding {
            root: Some(index),
            item: None,
            code: FindingCode::WrongKind,
        });
    }
    if declaration.statement.as_ref() != Some(&pair.requested.expected) {
        findings.push(Finding {
            root: Some(index),
            item: None,
            code: FindingCode::StatementMismatch,
        });
    }
    collect_axiom_findings(request, index, declaration, findings);
}

/// Append the findings about one declaration's axioms.
fn collect_axiom_findings(
    request: &AuditRequest,
    index: usize,
    declaration: &Decl,
    findings: &mut Vec<Finding>,
) {
    let mut item = 0;
    while item < declaration.axioms.len() {
        let Some(axiom) = declaration.axioms.get(item) else {
            break;
        };
        if let Some(code) = axiom_code(request, axiom) {
            findings.push(Finding {
                root: Some(index),
                item: Some(item),
                code,
            });
        }
        item += 1;
    }
}

/// Why an axiom is not approved, or `None` when it is.
fn axiom_code(request: &AuditRequest, axiom: &Axiom) -> Option<FindingCode> {
    if approved(request, &axiom.name) {
        return None;
    }
    if axiom.name == K_SORRY.as_slice() {
        Some(FindingCode::SorryAxiom)
    } else if axiom.native {
        Some(FindingCode::NativeAxiom)
    } else {
        Some(FindingCode::UndeclaredAxiom)
    }
}

}
