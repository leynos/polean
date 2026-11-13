//! Lemmas connecting the settlement specification to the executable code.
//!
//! The executable code decides acceptance by asking whether the findings
//! sequence is empty. The specification decides it by asking whether every
//! root is clean. These lemmas show the two questions have the same answer.

use vstd::prelude::*;

verus! {

use super::*;

/// A concatenation is empty exactly when both parts are.
pub proof fn seq_add_empty(left: Seq<Finding>, right: Seq<Finding>)
    ensures (left + right == Seq::empty()) == (left == Seq::empty() && right == Seq::empty()),
{
    assert((left + right).len() == left.len() + right.len());
}

/// An axiom is approved exactly when it produces no finding.
pub proof fn axiom_code_none_iff_approved(request: &AuditRequest, axiom: &Axiom)
    ensures axiom_code_spec(request, axiom).is_none() == approved_spec(request, axiom.name@),
{
}

/// A prefix of axiom findings is empty exactly when every axiom in it is
/// approved.
pub proof fn axiom_findings_empty_iff(
    request: &AuditRequest,
    index: int,
    axioms: Seq<Axiom>,
    upto: nat,
)
    ensures
        (axiom_findings_spec(request, index, axioms, upto) == Seq::empty())
            == (forall|j: int| 0 <= j < upto ==> axiom_code_spec(request, &axioms[j]).is_none()),
    decreases upto,
{
    if upto > 0 {
        axiom_findings_empty_iff(request, index, axioms, (upto - 1) as nat);
        axiom_code_none_iff_approved(request, &axioms[upto - 1]);
        seq_add_empty(
            axiom_findings_spec(request, index, axioms, (upto - 1) as nat),
            match axiom_code_spec(request, &axioms[upto - 1]) {
                Some(code) => seq![Finding {
                    root: Some(index as usize),
                    item: Some((upto - 1) as usize),
                    code,
                }],
                None => Seq::empty(),
            },
        );
    }
}

/// One root produces no finding exactly when it is clean.
pub proof fn root_findings_empty_iff(request: &AuditRequest, index: int, observed: &RootObs)
    ensures
        (root_findings_spec(request, index, observed) == Seq::empty())
            == root_clean_spec(request, index, observed),
{
    match observed.decl {
        None => {},
        Some(declaration) => {
            axiom_findings_empty_iff(
                request,
                index,
                declaration.axioms@,
                declaration.axioms@.len(),
            );
            seq_add_empty(
                (if !is_theorem_spec(declaration.kind) {
                    seq![Finding {
                        root: Some(index as usize),
                        item: None,
                        code: FindingCode::WrongKind,
                    }]
                } else {
                    Seq::empty()
                }),
                (if !statement_is_spec(&declaration, request.roots@[index].expected@) {
                    seq![Finding {
                        root: Some(index as usize),
                        item: None,
                        code: FindingCode::StatementMismatch,
                    }]
                } else {
                    Seq::empty()
                }),
            );
        },
    }
}

/// A prefix of root findings is empty exactly when every root in it is clean.
pub proof fn roots_findings_empty_iff(
    request: &AuditRequest,
    report: &AuditReport,
    upto: nat,
)
    ensures
        (roots_findings_spec(request, report, upto) == Seq::empty())
            == (forall|i: int| 0 <= i < upto ==> root_clean_spec(request, i, &report.roots@[i])),
    decreases upto,
{
    if upto > 0 {
        roots_findings_empty_iff(request, report, (upto - 1) as nat);
        root_findings_empty_iff(request, upto - 1, &report.roots@[upto - 1]);
        seq_add_empty(
            roots_findings_spec(request, report, (upto - 1) as nat),
            root_findings_spec(request, upto - 1, &report.roots@[upto - 1]),
        );
    }
}

/// An accepted report's axioms are all pinned foundations, and none is
/// `sorryAx`.
pub proof fn accepted_axioms_closed_world(
    request: &AuditRequest,
    report: &AuditReport,
    pin: Seq<u8>,
)
    requires acceptable_spec(request, report, pin),
    ensures
        forall|i: int, j: int|
            0 <= i < report.roots@.len()
                && report.roots@[i].decl.is_some()
                && 0 <= j < report.roots@[i].decl->Some_0.axioms@.len()
                ==> pinned_spec(report.roots@[i].decl->Some_0.axioms@[j].name@)
                    && report.roots@[i].decl->Some_0.axioms@[j].name@ != K_SORRY@,
{
    assert(valid_request_spec(request));
    assert(foundations_pinned_spec(request));
    assert(forall|i: int|
        0 <= i < report.roots@.len() ==> root_clean_spec(request, i, &report.roots@[i]));
    assert forall|i: int, j: int|
        0 <= i < report.roots@.len()
            && report.roots@[i].decl.is_some()
            && 0 <= j < report.roots@[i].decl->Some_0.axioms@.len()
        implies pinned_spec(report.roots@[i].decl->Some_0.axioms@[j].name@)
            && report.roots@[i].decl->Some_0.axioms@[j].name@ != K_SORRY@
    by {
        let name = report.roots@[i].decl->Some_0.axioms@[j].name@;
        assert(approved_spec(request, name));
        assert(contains_spec(request.foundations@, name));
        let k = choose|k: int| 0 <= k < request.foundations@.len() && request.foundations@[k]@ == name;
        assert(pinned_spec(request.foundations@[k]@));
        assert(pinned_spec(name));
        assert(name != K_SORRY@);
    }
}

}
