//! The settlement specification: what accepting a report means.
//!
//! These predicates are the model the proof reasons about. They name no
//! executable function, so a defect that makes the executable code agree with
//! itself cannot make the model agree with it. `verus/audit_settlement.rs`
//! verifies the executable settlement against them.

use vstd::prelude::*;

verus! {

use super::types::*;

/// Whether a declaration kind is the kind a root must have.
pub open spec fn is_theorem_spec(kind: DeclKind) -> bool {
    match kind {
        DeclKind::Theorem => true,
        DeclKind::Axiom => false,
        DeclKind::Definition => false,
        DeclKind::Opaque => false,
        DeclKind::Quotient => false,
        DeclKind::Constructor => false,
        DeclKind::Recursor => false,
        DeclKind::Inductive => false,
    }
}

/// Whether settlement accepted the report.
pub open spec fn is_accepted(result: Result<Settlement, RequestRefusal>) -> bool {
    match result {
        Ok(Settlement::Accepted) => true,
        _ => false,
    }
}

/// Whether settlement reported that the report does not answer the request.
pub open spec fn is_tool_failure(result: Result<Settlement, RequestRefusal>) -> bool {
    match result {
        Ok(Settlement::ToolFailure(_)) => true,
        _ => false,
    }
}

/// Whether settlement rejected the report with exactly these findings.
pub open spec fn is_rejected(
    result: Result<Settlement, RequestRefusal>,
    findings: Seq<Finding>,
) -> bool {
    match result {
        Ok(Settlement::Rejected(actual)) => actual@ == findings,
        _ => false,
    }
}

/// Whether `name` is one of the three pinned foundations.
pub open spec fn pinned_spec(name: Seq<u8>) -> bool {
    name == K_PROPEXT@ || name == K_QUOT@ || name == K_CHOICE@
}

/// Whether `name` appears in `keys`.
pub open spec fn contains_spec(keys: Seq<Key>, name: Seq<u8>) -> bool {
    exists|i: int| 0 <= i < keys.len() && keys[i]@ == name
}

/// Whether the request declares `name` an approved foundation.
pub open spec fn approved_spec(q: &AuditRequest, name: Seq<u8>) -> bool {
    contains_spec(q.foundations@, name)
}

/// Whether every root name in the request is distinct.
pub open spec fn roots_distinct_spec(q: &AuditRequest) -> bool {
    forall|i: int, j: int|
        0 <= i < j < q.roots@.len() ==> q.roots@[i].name@ != q.roots@[j].name@
}

/// Whether every declared foundation is pinned.
pub open spec fn foundations_pinned_spec(q: &AuditRequest) -> bool {
    forall|i: int| 0 <= i < q.foundations@.len() ==> pinned_spec(q.foundations@[i]@)
}

/// Whether the request is admissible.
pub open spec fn valid_request_spec(q: &AuditRequest) -> bool {
    q.roots@.len() > 0 && roots_distinct_spec(q) && foundations_pinned_spec(q)
}

/// Whether `module` is among the replayed modules.
pub open spec fn covers_spec(modules: Seq<Key>, module: Seq<u8>) -> bool {
    contains_spec(modules, module)
}

/// Whether the report's root names are the request's root names, in order.
pub open spec fn roots_names_match_spec(q: &AuditRequest, p: &AuditReport) -> bool {
    forall|i: int| #![trigger p.roots@[i].name@]
        0 <= i < q.roots@.len() ==> p.roots@[i].name@ == q.roots@[i].name@
}

/// Whether the report's roots are the request's roots, in order.
pub open spec fn roots_match_spec(q: &AuditRequest, p: &AuditReport) -> bool {
    q.roots@.len() == p.roots@.len() && roots_names_match_spec(q, p)
}

/// Whether the replay answers the request.
pub open spec fn replay_ok_spec(q: &AuditRequest, p: &AuditReport) -> bool {
    match p.replay {
        Replay::Rejected => p.roots@.len() == 0,
        Replay::Accepted(modules) => {
            covers_spec(modules@, q.module@) && roots_match_spec(q, p)
        },
    }
}

/// Whether the report answers the request.
pub open spec fn protocol_ok_spec(q: &AuditRequest, p: &AuditReport, pin: Seq<u8>) -> bool {
    p.githash@ == pin && p.module@ == q.module@ && replay_ok_spec(q, p)
}

/// Whether the declaration's type is exactly `expected`.
pub open spec fn statement_is_spec(d: &Decl, expected: Seq<u8>) -> bool {
    match d.statement {
        Some(statement) => statement@ == expected,
        None => false,
    }
}

/// Whether one observed root is clean.
pub open spec fn root_clean_spec(q: &AuditRequest, i: int, o: &RootObs) -> bool {
    match o.decl {
        Some(d) => is_theorem_spec(d.kind)
            && statement_is_spec(&d, q.roots@[i].expected@)
            && forall|j: int| 0 <= j < d.axioms.len() ==> approved_spec(q, d.axioms@[j].name@),
        None => false,
    }
}

/// Whether the report proves the request.
pub open spec fn acceptable_spec(q: &AuditRequest, p: &AuditReport, pin: Seq<u8>) -> bool {
    valid_request_spec(q) && protocol_ok_spec(q, p, pin)
        && match p.replay {
            Replay::Rejected => false,
            Replay::Accepted(_) => {
                forall|i: int| 0 <= i < p.roots@.len() ==> root_clean_spec(q, i, &p.roots@[i])
            },
        }
}

/// Why an axiom is not approved, or `None` when it is.
pub open spec fn axiom_code_spec(q: &AuditRequest, a: &Axiom) -> Option<FindingCode> {
    if approved_spec(q, a.name@) {
        None
    } else if a.name@ == K_SORRY@ {
        Some(FindingCode::SorryAxiom)
    } else if a.native {
        Some(FindingCode::NativeAxiom)
    } else {
        Some(FindingCode::UndeclaredAxiom)
    }
}

/// The findings about one declaration's axioms.
pub open spec fn axiom_findings_spec(q: &AuditRequest, i: int, axioms: Seq<Axiom>, upto: nat) -> Seq<Finding>
    decreases upto
{
    if upto <= 0 {
        Seq::empty()
    } else {
        axiom_findings_spec(q, i, axioms, (upto - 1) as nat)
            + match axiom_code_spec(q, &axioms[upto - 1]) {
                Some(code) => seq![Finding { root: Some(i as usize), item: Some((upto - 1) as usize), code }],
                None => Seq::empty(),
            }
    }
}

/// The findings about one root.
pub open spec fn root_findings_spec(q: &AuditRequest, i: int, o: &RootObs) -> Seq<Finding> {
    match o.decl {
        None => seq![Finding { root: Some(i as usize), item: None, code: FindingCode::RootAbsent }],
        Some(d) => {
            (if !is_theorem_spec(d.kind) {
                seq![Finding { root: Some(i as usize), item: None, code: FindingCode::WrongKind }]
            } else { Seq::empty() })
            + (if !statement_is_spec(&d, q.roots@[i].expected@) {
                seq![Finding { root: Some(i as usize), item: None, code: FindingCode::StatementMismatch }]
            } else { Seq::empty() })
            + axiom_findings_spec(q, i, d.axioms@, d.axioms@.len())
        },
    }
}

/// The findings about the first `upto` roots.
pub open spec fn roots_findings_spec(q: &AuditRequest, p: &AuditReport, upto: nat) -> Seq<Finding>
    decreases upto
{
    if upto <= 0 {
        Seq::empty()
    } else {
        roots_findings_spec(q, p, (upto - 1) as nat) + root_findings_spec(q, upto - 1, &p.roots@[upto - 1])
    }
}

/// The findings a settlement must report.
pub open spec fn expected_findings_spec(q: &AuditRequest, p: &AuditReport) -> Seq<Finding> {
    match p.replay {
        Replay::Rejected => seq![Finding {
            root: None,
            item: None,
            code: FindingCode::KernelRejected,
        }],
        Replay::Accepted(_) => roots_findings_spec(q, p, p.roots@.len()),
    }
}

}
