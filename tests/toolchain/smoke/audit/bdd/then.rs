//! The `Then` steps of the audit feature scenarios.
//!
//! Each step asserts one reading of the state the scenario accumulated. The
//! steps register with `rstest-bdd`'s crate-wide step inventory, so they may
//! live in a child module of the scenario module; keeping them here holds
//! `bdd` inside the repository's module-size bound.
//!
//! A passing step is evidence about this build of the worker, the decoder, and
//! the settlement function over these fixtures. It is not evidence that the
//! worker is truthful, and it is not evidence that the source policy belongs
//! to an admitted profile.

use rstest_bdd_macros::then;

use super::{
    AuditState,
    Outcome,
    support::{finding_code, protocol_breach, with_settlement},
};
use crate::audit::{
    settlement::decision::{FindingCode, Settlement},
    wire::DecodeError,
};

/// Require acceptance.
#[then("the settlement accepts")]
fn settlement_accepts(state: &AuditState) -> Result<(), String> {
    with_settlement(state, |outcome| match outcome {
        Ok(Settlement::Accepted) => Ok(()),
        other => Err(format!("expected acceptance, got {other:?}")),
    })?
}

/// Require exactly one finding with the named code.
#[then("the settlement rejects with the finding {code}")]
fn settlement_rejects_with_finding(state: &AuditState, code: String) -> Result<(), String> {
    let expected = finding_code(&code)?;
    with_settlement(state, |outcome| match outcome {
        Ok(Settlement::Rejected(findings)) => {
            let Some(finding) = findings.first() else {
                return Err("the rejection carried no findings".to_owned());
            };
            if findings.len() != 1 {
                return Err(format!("expected one finding, got {findings:?}"));
            }
            if finding.code != expected {
                return Err(format!(
                    "expected the finding {expected:?}, got {:?}",
                    finding.code
                ));
            }
            Ok(())
        }
        other => Err(format!("expected a rejection, got {other:?}")),
    })?
}

/// Require exactly two findings, in the named order.
#[then("the settlement rejects with the findings {first} and {second}")]
fn settlement_rejects_with_findings(
    state: &AuditState,
    first: String,
    second: String,
) -> Result<(), String> {
    let expected = [finding_code(&first)?, finding_code(&second)?];
    with_settlement(state, |outcome| match outcome {
        Ok(Settlement::Rejected(findings)) => {
            let observed: Vec<FindingCode> = findings.iter().map(|finding| finding.code).collect();
            if observed != expected {
                return Err(format!(
                    "expected the findings {expected:?}, got {observed:?}"
                ));
            }
            Ok(())
        }
        other => Err(format!("expected a rejection, got {other:?}")),
    })?
}

/// Require the named protocol breach.
#[then("the settlement reports the tool failure {breach}")]
fn settlement_reports_tool_failure(state: &AuditState, breach: String) -> Result<(), String> {
    let expected = protocol_breach(&breach)?;
    with_settlement(state, |outcome| match outcome {
        Ok(Settlement::ToolFailure(observed)) if *observed == expected => Ok(()),
        other => Err(format!(
            "expected the tool failure {expected:?}, got {other:?}"
        )),
    })?
}

/// Require the adapter to refuse the document as malformed because of `field`.
///
/// The cause is asserted, not just the failure: the mutation under test is an
/// unknown field, and a decoder that refused the document for another reason
/// would still satisfy a bare `Malformed` check.
#[then("the adapter refuses it as malformed, naming {field}")]
fn adapter_refuses_as_malformed(state: &AuditState, field: String) -> Result<(), String> {
    state
        .decoded
        .with_ref(|decoded| match decoded {
            Err(DecodeError::Malformed(message)) if message.contains(&field) => Ok(()),
            other => Err(format!(
                "expected a malformed refusal naming {field}, got {other:?}"
            )),
        })
        .ok_or_else(|| "the adapter did not run".to_owned())?
}

/// Require the corpus boundary to report the named tool failure.
#[then("the classification reports the tool failure {reason}")]
fn classification_reports_tool_failure(state: &AuditState, reason: String) -> Result<(), String> {
    state
        .outcome
        .with_ref(|outcome| match outcome {
            Outcome::ToolFailure(observed) if *observed == reason => Ok(()),
            Outcome::ToolFailure(observed) => Err(format!(
                "expected the tool failure {reason}, got {observed}"
            )),
            Outcome::Accepted => Err(format!(
                "expected the tool failure {reason}, got acceptance"
            )),
            Outcome::Rejected(findings) => Err(format!(
                "expected the tool failure {reason}, got a rejection with {} finding(s)",
                findings.len()
            )),
        })
        .ok_or_else(|| "the corpus did not classify a document".to_owned())?
}

/// Require that settlement was never reached.
///
/// A malformed report must be refused by the adapter, and the corpus boundary
/// must classify it as a tool failure before settlement is asked to decide it.
/// The scenario's settlement slot covers the first claim; the counter the
/// boundary's callback records covers the second. This asserts the boundary
/// rather than relying on the scenario's step order.
#[then("the settlement does not run")]
fn settlement_does_not_run(state: &AuditState) -> Result<(), String> {
    if state.settlement.with_ref(|_| ()).is_some() {
        return Err("settlement ran on a report the adapter refused".to_owned());
    }
    if let Some(calls) = state.settlement_calls.with_ref(|calls| *calls)
        && calls != 0
    {
        return Err(format!(
            "the classification boundary reached settlement {calls} time(s) for a report the \
             adapter refused"
        ));
    }
    Ok(())
}
