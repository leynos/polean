//! Protocol conformance: the rules that decide whether a report answers a
//! request at all.
//!
//! A breach takes precedence over findings, so these rules run before the
//! settlement examines the roots. Each returns the first breach it meets.

use vstd::prelude::*;

verus! {

use super::*;

/// The first way the report fails to answer the request, if any.
pub(super) fn protocol_breach(
    request: &AuditRequest,
    report: &AuditReport,
    pin: &[u8],
) -> (result: Option<ProtocolBreach>)
    ensures result.is_none() == protocol_ok_spec(request, report, pin@),
{
    if report.githash.as_slice() == pin {
        assert(report.githash@ == pin@);
    } else {
        assert(report.githash@ != pin@);
        assert(!protocol_ok_spec(request, report, pin@));
        return Some(ProtocolBreach::ToolchainMismatch);
    }
    if report.module == request.module {
        assert(report.module@ == request.module@);
    } else {
        assert(report.module@ != request.module@);
        assert(!protocol_ok_spec(request, report, pin@));
        return Some(ProtocolBreach::ModuleMismatch);
    }
    replay_breach(request, report)
}

/// The ways a replay can fail to answer the request.
fn replay_breach(
    request: &AuditRequest,
    report: &AuditReport,
) -> (result: Option<ProtocolBreach>)
    ensures result.is_none() == replay_ok_spec(request, report),
{
    match &report.replay {
        Replay::Rejected => rejected_breach(report),
        Replay::Accepted(modules) => accepted_breach(request, report, modules.as_slice()),
    }
}

/// A rejected replay must report no roots, because nothing was observed.
const fn rejected_breach(report: &AuditReport) -> (result: Option<ProtocolBreach>)
    ensures result.is_none() == (report.roots@.len() == 0),
{
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
) -> (result: Option<ProtocolBreach>)
    ensures result.is_none() == (covers_spec(modules@, request.module@)
        && roots_match_spec(request, report)),
{
    if !contains(modules, request.module.as_slice()) {
        assert(!covers_spec(modules@, request.module@));
        assert(!(covers_spec(modules@, request.module@)
            && roots_match_spec(request, report)));
        return Some(ProtocolBreach::CoverageMismatch);
    }
    assert(covers_spec(modules@, request.module@));
    if report.roots.len() != request.roots.len() {
        assert(report.roots@.len() != request.roots@.len());
        assert(!roots_match_spec(request, report));
        assert(!(covers_spec(modules@, request.module@)
            && roots_match_spec(request, report)));
        return Some(ProtocolBreach::CoverageMismatch);
    }
    assert(report.roots@.len() == request.roots@.len());
    if roots_differ(request, report) {
        assert(!roots_names_match_spec(request, report));
        assert(!roots_match_spec(request, report));
        assert(!(covers_spec(modules@, request.module@)
            && roots_match_spec(request, report)));
        return Some(ProtocolBreach::CoverageMismatch);
    }
    assert(roots_names_match_spec(request, report));
    assert(roots_match_spec(request, report));
    None
}

/// Whether the report's roots are not the request's roots in the same order.
#[expect(clippy::indexing_slicing, reason = "the loop guard bounds the index")]
fn roots_differ(request: &AuditRequest, report: &AuditReport) -> (result: bool)
    requires report.roots@.len() == request.roots@.len(),
    ensures result == !roots_names_match_spec(request, report),
{
    let mut index = 0;
    while index < report.roots.len()
        invariant
            index <= report.roots@.len(),
            report.roots@.len() == request.roots@.len(),
            forall|j: int| #![trigger report.roots@[j].name@]
                0 <= j < index ==> report.roots@[j].name@ == request.roots@[j].name@,
        decreases report.roots.len() - index,
    {
        let observed = &report.roots[index];
        let requested = &request.roots[index];
        if observed.name != requested.name {
            assert(report.roots@[index as int].name@ != request.roots@[index as int].name@);
            return true;
        }
        index += 1;
    }
    false
}

}
