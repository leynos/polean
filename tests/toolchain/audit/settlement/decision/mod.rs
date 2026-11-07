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
//!
//! The rules are split by question: [`scan`] answers byte-level predicates,
//! [`request`] decides admissibility, [`replay`] decides whether a report
//! answers the request, and [`findings`] builds the exact rejection. This file
//! holds only the decision that combines them.

use vstd::prelude::*;

mod types;

mod scan;

mod request;

mod replay;

mod findings;

// The specification, the lemmas, and the witnesses exist only for the
// verifier. Under plain `rustc` the `verus!` macro erases every item in them,
// so compiling the modules would leave their imports unused and `-D warnings`
// would fail the build. They are gated together for that reason.
#[cfg(verus_keep_ghost)]
#[path = "../../../../../verus/audit_settlement_spec.rs"]
mod spec;

#[cfg(verus_keep_ghost)]
#[path = "../../../../../verus/audit_settlement_lemmas.rs"]
mod lemmas;

#[cfg(verus_keep_ghost)]
#[path = "../../../../../verus/audit_settlement_witnesses.rs"]
mod witnesses;

#[cfg(verus_keep_ghost)]
pub use self::lemmas::*;
#[cfg(verus_keep_ghost)]
pub use self::spec::*;
#[cfg(verus_keep_ghost)]
pub use self::witnesses::*;
use self::{findings::*, replay::*, scan::*};
pub use self::{request::*, types::*};

verus! {

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
) -> (result: Result<Settlement, RequestRefusal>)
    ensures
        result.is_err() == !valid_request_spec(request),
        is_accepted(result) == acceptable_spec(request, report, pin@),
        is_tool_failure(result) == (valid_request_spec(request) && !protocol_ok_spec(request, report, pin@)),
        (valid_request_spec(request) && protocol_ok_spec(request, report, pin@)
            && !acceptable_spec(request, report, pin@))
            ==> is_rejected(result, expected_findings_spec(request, report)),
{
    validate_request(request)?;
    if let Some(breach) = protocol_breach(request, report, pin) {
        return Ok(Settlement::ToolFailure(breach));
    }
    match &report.replay {
        Replay::Rejected => {
            let findings = vec![Finding {
                root: None,
                item: None,
                code: FindingCode::KernelRejected,
            }];
            assert(findings@ == expected_findings_spec(request, report));
            Ok(Settlement::Rejected(findings))
        },
        Replay::Accepted(_) => {
            let findings = root_findings(request, report);
            proof { roots_findings_empty_iff(request, report, report.roots@.len()); }
            if findings.is_empty() {
                assert(acceptable_spec(request, report, pin@));
                Ok(Settlement::Accepted)
            } else {
                assert(!acceptable_spec(request, report, pin@));
                assert(findings@ == expected_findings_spec(request, report));
                Ok(Settlement::Rejected(findings))
            }
        }
    }
}

}
