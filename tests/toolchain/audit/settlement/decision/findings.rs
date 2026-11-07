//! Findings: the exact reasons a report fails to prove a request.
//!
//! Every root keeps its own findings, in root order and then axiom order, so a
//! mixed report is not reduced to its first fault. The specification
//! `root_findings_spec` says what the sequence must be; these functions build
//! it.

use vstd::prelude::*;

verus! {

use super::*;

/// Every finding about every root, in root order and then axiom order.
#[expect(clippy::indexing_slicing, reason = "the loop guard bounds the index")]
pub(super) fn root_findings(request: &AuditRequest, report: &AuditReport) -> (findings: Vec<Finding>)
    requires report.roots.len() <= request.roots.len(),
    ensures findings@ == roots_findings_spec(request, report, report.roots@.len()),
{
    let mut findings = Vec::new();
    let mut index = 0;
    while index < report.roots.len()
        invariant
            index <= report.roots.len(),
            report.roots.len() <= request.roots.len(),
            findings@ == roots_findings_spec(request, report, index as nat),
        decreases report.roots.len() - index,
    {
        let observed = &report.roots[index];
        let requested = &request.roots[index];
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

/// One finding about a whole root.
const fn root_finding(index: usize, code: FindingCode) -> (finding: Finding)
    ensures finding == (Finding { root: Some(index), item: None, code }),
{
    Finding {
        root: Some(index),
        item: None,
        code,
    }
}

/// The kind finding, when the declaration is not a theorem.
const fn kind_code(declaration: &Decl) -> (result: Option<FindingCode>)
    ensures result == (if !is_theorem_spec(declaration.kind) {
        Some(FindingCode::WrongKind)
    } else {
        None
    }),
{
    if is_theorem(declaration.kind) {
        assert(is_theorem_spec(declaration.kind));
        None
    } else {
        assert(!is_theorem_spec(declaration.kind));
        Some(FindingCode::WrongKind)
    }
}

/// The statement finding, when the declaration does not state `expected`.
fn statement_code(declaration: &Decl, expected: &[u8]) -> (result: Option<FindingCode>)
    ensures result == (if !statement_is_spec(declaration, expected@) {
        Some(FindingCode::StatementMismatch)
    } else {
        None
    }),
{
    let Some(statement) = &declaration.statement else {
        assert(!statement_is_spec(declaration, expected@));
        return Some(FindingCode::StatementMismatch);
    };
    let have = statement.as_slice();
    let want = expected;
    if have == want {
        assert(have@ == want@);
        assert(statement@ == expected@);
        assert(statement_is_spec(declaration, expected@));
        None
    } else {
        assert(have@ != want@);
        assert(statement@ != expected@);
        assert(!statement_is_spec(declaration, expected@));
        Some(FindingCode::StatementMismatch)
    }
}

/// Append the findings about one root.
fn collect_root_findings(
    request: &AuditRequest,
    index: usize,
    pair: RootPair<'_>,
    findings: &mut Vec<Finding>,
)
    requires pair.requested == request.roots@[index as int],
    ensures final(findings)@ == old(findings)@
        + root_findings_spec(request, index as int, pair.observed),
{
    let ghost initial: Seq<Finding> = findings@;
    let Some(declaration) = &pair.observed.decl else {
        findings.push(root_finding(index, FindingCode::RootAbsent));
        assert(findings@ == initial + root_findings_spec(request, index as int, pair.observed));
        return;
    };
    if let Some(code) = kind_code(declaration) {
        findings.push(root_finding(index, code));
    }
    assert(findings@ == initial + (if !is_theorem_spec(declaration.kind) {
        seq![Finding { root: Some(index), item: None, code: FindingCode::WrongKind }]
    } else {
        Seq::empty()
    }));
    if let Some(code) = statement_code(declaration, pair.requested.expected.as_slice()) {
        findings.push(root_finding(index, code));
    }
    assert(findings@ == initial + (if !is_theorem_spec(declaration.kind) {
        seq![Finding { root: Some(index), item: None, code: FindingCode::WrongKind }]
    } else {
        Seq::empty()
    }) + (if statement_is_spec(declaration, pair.requested.expected@) {
        Seq::empty()
    } else {
        seq![Finding { root: Some(index), item: None, code: FindingCode::StatementMismatch }]
    }));
    collect_axiom_findings(request, index, declaration, findings);
    assert(findings@ == initial + root_findings_spec(request, index as int, pair.observed));
}

/// Append the findings about one declaration's axioms.
#[expect(clippy::indexing_slicing, reason = "the loop guard bounds the index")]
fn collect_axiom_findings(
    request: &AuditRequest,
    index: usize,
    declaration: &Decl,
    findings: &mut Vec<Finding>,
)
    ensures final(findings)@ == old(findings)@ + axiom_findings_spec(
        request,
        index as int,
        declaration.axioms@,
        declaration.axioms@.len(),
    ),
{
    let mut item = 0;
    while item < declaration.axioms.len()
        invariant
            item <= declaration.axioms@.len(),
            findings@ == old(findings)@ + axiom_findings_spec(
                request,
                index as int,
                declaration.axioms@,
                item as nat,
            ),
        decreases declaration.axioms.len() - item,
    {
        let axiom = &declaration.axioms[item];
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
fn axiom_code(request: &AuditRequest, axiom: &Axiom) -> (result: Option<FindingCode>)
    ensures result == axiom_code_spec(request, axiom),
{
    if approved(request, axiom.name.as_slice()) {
        assert(approved_spec(request, axiom.name@));
        assert(axiom_code_spec(request, axiom) == None);
        return None;
    }
    assert(!approved_spec(request, axiom.name@));
    if axiom.name.as_slice() == K_SORRY.as_slice() {
        assert(axiom.name@ == K_SORRY@);
        assert(axiom_code_spec(request, axiom) == Some(FindingCode::SorryAxiom));
        return Some(FindingCode::SorryAxiom);
    }
    assert(axiom.name@ != K_SORRY@);
    if axiom.native {
        assert(axiom_code_spec(request, axiom) == Some(FindingCode::NativeAxiom));
        return Some(FindingCode::NativeAxiom);
    }
    assert(axiom_code_spec(request, axiom) == Some(FindingCode::UndeclaredAxiom));
    Some(FindingCode::UndeclaredAxiom)
}

}
