//! Witnesses showing every settlement outcome is reachable.
//!
//! Each witness runs the real `settle` on one concrete request and report, so
//! the acceptance, rejection, and protocol-failure outcomes are reachable as
//! abstract values and the proof's preconditions are satisfiable. The keys are
//! arbitrary byte strings rather than keys produced by `name_key`, so the
//! witnesses do not show reachability through the wire decoder or non-vacuity
//! of every obligation; the A1 key properties cover the encoding's link to real
//! names and statements.

use vstd::prelude::*;

verus! {

use super::*;

/// Copy a byte slice into an owned key.
#[expect(clippy::indexing_slicing, reason = "the loop invariant bounds the index")]
fn key(bytes: &[u8]) -> (key: Vec<u8>)
    ensures key@ == bytes@,
{
    let mut key: Vec<u8> = Vec::new();
    let mut index = 0;
    while index < bytes.len()
        invariant
            index <= bytes.len(),
            key@ == bytes@.subrange(0, index as int),
        decreases bytes.len() - index,
    {
        key.push(bytes[index]);
        index += 1;
    }
    key
}

/// A request whose single root must prove `3`.
pub fn sample_request() -> (request: AuditRequest)
    ensures
        request.module@ == seq![1u8],
        request.foundations@.len() == 1,
        request.foundations@[0]@ == K_PROPEXT@,
        request.roots@.len() == 1,
        request.roots@[0].name@ == seq![2u8],
        request.roots@[0].expected@ == seq![3u8],
{
    AuditRequest {
        module: vec![1u8],
        foundations: vec![key(&K_PROPEXT)],
        roots: vec![RequestRoot { name: vec![2u8], expected: vec![3u8] }],
    }
}

/// A report whose single root is a clean theorem depending only on `propext`.
pub fn sample_report() -> (report: AuditReport)
    ensures
        report.githash@ == seq![9u8],
        report.module@ == seq![1u8],
        match report.replay {
            Replay::Accepted(modules) => modules@.len() == 1 && modules@[0]@ == seq![1u8],
            Replay::Rejected => false,
        },
        report.roots@.len() == 1,
        report.roots@[0].name@ == seq![2u8],
        report.roots@[0].decl.is_some(),
        report.roots@[0].decl->Some_0.kind == DeclKind::Theorem,
        match report.roots@[0].decl->Some_0.statement {
            Some(statement) => statement@ == seq![3u8],
            None => false,
        },
        report.roots@[0].decl->Some_0.axioms@.len() == 1,
        report.roots@[0].decl->Some_0.axioms@[0].name@ == K_PROPEXT@,
{
    AuditReport {
        githash: vec![9u8],
        module: vec![1u8],
        replay: Replay::Accepted(vec![vec![1u8]]),
        roots: vec![RootObs {
            name: vec![2u8],
            decl: Some(Decl {
                kind: DeclKind::Theorem,
                statement: Some(vec![3u8]),
                axioms: vec![Axiom { name: key(&K_PROPEXT), native: false }],
            }),
        }],
    }
}

/// The sample report with its one axiom replaced by `sorryAx`.
pub fn sorry_report() -> (report: AuditReport)
    ensures
        report.githash@ == seq![9u8],
        report.module@ == seq![1u8],
        match report.replay {
            Replay::Accepted(modules) => modules@.len() == 1 && modules@[0]@ == seq![1u8],
            Replay::Rejected => false,
        },
        report.roots@.len() == 1,
        report.roots@[0].name@ == seq![2u8],
        report.roots@[0].decl.is_some(),
        report.roots@[0].decl->Some_0.kind == DeclKind::Theorem,
        match report.roots@[0].decl->Some_0.statement {
            Some(statement) => statement@ == seq![3u8],
            None => false,
        },
        report.roots@[0].decl->Some_0.axioms@.len() == 1,
        report.roots@[0].decl->Some_0.axioms@[0].name@ == K_SORRY@,
{
    AuditReport {
        githash: vec![9u8],
        module: vec![1u8],
        replay: Replay::Accepted(vec![vec![1u8]]),
        roots: vec![RootObs {
            name: vec![2u8],
            decl: Some(Decl {
                kind: DeclKind::Theorem,
                statement: Some(vec![3u8]),
                axioms: vec![Axiom { name: key(&K_SORRY), native: false }],
            }),
        }],
    }
}

/// A request that names no roots.
pub fn empty_request() -> (request: AuditRequest)
    ensures request.roots@.len() == 0,
{
    AuditRequest {
        module: vec![1u8],
        foundations: vec![key(&K_PROPEXT)],
        roots: Vec::new(),
    }
}

/// The single finding a `sorryAx` axiom produces.
pub open spec fn sorry_findings() -> Seq<Finding> {
    Seq::empty().push(Finding {
        root: Some(0usize),
        item: Some(0usize),
        code: FindingCode::SorryAxiom,
    })
}

/// `Ok(Accepted)` is reachable.
pub fn witness_accepted() -> (result: Result<Settlement, RequestRefusal>)
    ensures is_accepted(result),
{
    let request = sample_request();
    let report = sample_report();
    let pin = vec![9u8];
    let result = settle(&request, &report, &pin);
    assert(valid_request_spec(&request));
    assert(report.githash@ == pin@);
    assert(report.module@ == request.module@);
    assert(replay_ok_spec(&request, &report));
    assert(protocol_ok_spec(&request, &report, pin@));
    assert(forall|i: int| 0 <= i < report.roots@.len() ==> root_clean_spec(&request, i, &report.roots@[i]));
    assert(acceptable_spec(&request, &report, pin@));
    result
}

/// `Ok(Rejected)` is reachable, with the exact finding for `sorryAx`.
pub fn witness_rejected() -> (result: Result<Settlement, RequestRefusal>)
    ensures is_rejected(result, sorry_findings()),
{
    let request = sample_request();
    let report = sorry_report();
    let pin = vec![9u8];
    let result = settle(&request, &report, &pin);
    assert(valid_request_spec(&request));
    assert(report.githash@ == pin@);
    assert(report.module@ == request.module@);
    assert(replay_ok_spec(&request, &report));
    assert(protocol_ok_spec(&request, &report, pin@));
    assert(!acceptable_spec(&request, &report, pin@));
    assert(!approved_spec(&request, K_SORRY@));
    assert(axiom_code_spec(&request, &report.roots@[0].decl->Some_0.axioms@[0])
        == Some(FindingCode::SorryAxiom));
    assert(axiom_findings_spec(&request, 0, report.roots@[0].decl->Some_0.axioms@, 0) == Seq::empty());
    assert(axiom_findings_spec(&request, 0, report.roots@[0].decl->Some_0.axioms@, 1)
        == Seq::empty() + sorry_findings());
    assert(Seq::empty() + sorry_findings() == sorry_findings());
    assert(root_findings_spec(&request, 0, &report.roots@[0]) == sorry_findings());
    assert(roots_findings_spec(&request, &report, 0) == Seq::empty());
    assert(roots_findings_spec(&request, &report, 1) == Seq::empty() + sorry_findings());
    assert(roots_findings_spec(&request, &report, 1) == sorry_findings());
    assert(expected_findings_spec(&request, &report) == sorry_findings());
    result
}

/// `Ok(ToolFailure)` is reachable when the report names another toolchain.
pub fn witness_tool_failure() -> (result: Result<Settlement, RequestRefusal>)
    ensures is_tool_failure(result),
{
    let request = sample_request();
    let report = sample_report();
    let pin = vec![8u8];
    let result = settle(&request, &report, &pin);
    assert(valid_request_spec(&request));
    assert(!protocol_ok_spec(&request, &report, pin@));
    result
}

/// `Err` is reachable when the request names no roots.
pub fn witness_refused() -> (result: Result<Settlement, RequestRefusal>)
    ensures result.is_err(),
{
    let request = empty_request();
    let report = sample_report();
    let pin = vec![9u8];
    let result = settle(&request, &report, &pin);
    assert(!valid_request_spec(&request));
    result
}

}
