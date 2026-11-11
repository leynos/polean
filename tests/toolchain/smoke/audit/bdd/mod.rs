//! The audit workflow, bound to `tests/features/axiom_audit.feature`.
//!
//! The feature file is the readable specification: one scenario per distinct
//! settlement outcome, each naming a representative corpus fixture. Each
//! scenario runs the real `polean-audit` worker over a corpus fixture, decodes
//! its report with the strict wire adapter, and settles the decoded report
//! against the request the corpus asks. The scenario therefore exercises the
//! boundary it describes rather than a hand-built pair at the settlement seam.
//!
//! The malformed-report scenario starts from the worker's own report for
//! `AcceptedGround` and adds a field the contract does not name before
//! decoding, so its refusal is exercised at the process-output boundary and
//! settlement is then asserted not to have run.
//!
//! A scenario that passes is evidence about this build of the worker, the
//! decoder, and the settlement function over these fixtures. It is not evidence
//! that the worker is truthful, and it is not evidence that the source policy
//! belongs to an admitted profile.
//!
//! The direct fixture-pair checks stay in `tests/toolchain/audit/tests.rs` as
//! unit tests, where a settlement outcome can be isolated from the worker.
//!
//! The state is a `Slot`-bearing struct rather than a `&mut` world fixture, for
//! the same reason as [`crate::bdd`]: `rstest-bdd` 0.6.0 documents a rustc
//! internal compiler error for the macro-driven mutable world on the pinned
//! nightly.

mod support;

use std::sync::OnceLock;

use rstest_bdd::Slot;
use rstest_bdd_macros::{ScenarioState, given, scenario, then, when};
pub use support::{finding_code, protocol_breach, state, with_settlement};

use super::{requests, runner::Corpus};
use crate::audit::{
    settlement::decision::{
        AuditReport,
        AuditRequest,
        FindingCode,
        RequestRefusal,
        Settlement,
        settle,
    },
    wire::{DecodeError, PIN, decode_report},
};

/// Cargo recompiles this target when the specification changes.
///
/// `#[scenario]` reads the feature file itself, which Cargo cannot fingerprint,
/// so without this reference an edited feature file can leave a previously
/// expanded binary in place.
const _: [&str; 1] = [include_str!("../../../../features/axiom_audit.feature")];

/// The fixtures the feature scenarios name, dependencies before dependants.
const SOURCES: &[&str] = &[
    "AuditFixture.Goal",
    "AuditFixture.AcceptedGround",
    "AuditFixture.AcceptedFoundations",
    "AuditFixture.Sorry",
    "AuditFixture.Native",
    "AuditFixture.Undeclared",
    "AuditFixture.WrongStatement",
    "AuditFixture.AxiomRoot",
    "AuditFixture.RootAbsent",
    "AuditFixture.ForgeInit",
];

/// The corpus, built once per process and shared by every scenario.
///
/// Each scenario is a separate test, so without the cache every one of them
/// would rebuild the worker and recompile the fixtures. The corpus lives for
/// the rest of the process, which is the whole test binary.
fn corpus() -> Result<&'static Corpus, String> {
    static CORPUS: OnceLock<Result<Corpus, String>> = OnceLock::new();
    CORPUS
        .get_or_init(|| Corpus::prepare(SOURCES))
        .as_ref()
        .map_err(String::clone)
}

/// The state one scenario accumulates.
///
/// Visible in the parent module because the `support` child re-exports the
/// fixture that builds it.
#[derive(Default, ScenarioState)]
pub(super) struct AuditState {
    /// The request the corpus asks for the selected fixture.
    request: Slot<AuditRequest>,
    /// The report the worker produced, as the adapter decoded it.
    report: Slot<AuditReport>,
    /// The outcome of the most recent settlement.
    settlement: Slot<Result<Settlement, RequestRefusal>>,
    /// A document the adapter is asked to decode.
    document: Slot<String>,
    /// The result of the most recent decode.
    decoded: Slot<Result<AuditReport, DecodeError>>,
}

/// Run the worker over the named fixture and decode its report.
#[given("the audit fixture {fixture}")]
fn audit_fixture(state: &AuditState, fixture: String) -> Result<(), String> {
    let entry = requests::entry(&fixture)?;
    let output = corpus()?.worker(&entry.module, &entry.roots)?;
    if !output.status.success() {
        return Err(format!(
            "{fixture}: the worker failed with {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let report = decode_report(&output.stdout)
        .map_err(|error| format!("{fixture}: the worker's report did not decode: {error:?}"))?;
    state.request.set(entry.request);
    state.report.set(report);
    Ok(())
}

/// Make the decoded report name a different toolchain.
#[given("the report names another toolchain")]
fn report_names_another_toolchain(state: &AuditState) -> Result<(), String> {
    state
        .report
        .with_mut(|report| report.githash = b"0000000000000000000000000000000000000000".to_vec())
        .ok_or_else(|| "no fixture was run before the mutation".to_owned())
}

/// Select the worker's report for a valid fixture with an unknown field added.
///
/// The document starts as the real worker's output, so the scenario exercises
/// the process-output boundary: the adapter must refuse the worker's own report
/// once a field the contract does not name is added, before settlement sees it.
#[given("a report with an unknown field")]
fn report_with_unknown_field(state: &AuditState) -> Result<(), String> {
    let entry = requests::entry("AcceptedGround")?;
    let output = corpus()?.worker(&entry.module, &entry.roots)?;
    if !output.status.success() {
        return Err(format!(
            "AcceptedGround: the worker failed with {:?}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let report = std::str::from_utf8(&output.stdout)
        .map_err(|error| format!("the worker's report is not UTF-8: {error}"))?;
    state.document.set(with_unknown_field(report)?);
    Ok(())
}

/// Add a field the report contract does not name to a compact JSON object.
fn with_unknown_field(report: &str) -> Result<String, String> {
    let body = report.trim_end();
    let Some(prefix) = body.strip_suffix('}') else {
        return Err(format!("the worker's report is not a JSON object: {body}"));
    };
    Ok(format!("{prefix},\"extra\":0}}"))
}

/// Decide the decoded report against the corpus request.
#[when("the settlement decides the report")]
fn settlement_decides(state: &AuditState) -> Result<(), String> {
    let outcome = state
        .request
        .with_ref(|request| {
            state
                .report
                .with_ref(|report| settle(request, report, PIN.as_bytes()))
        })
        .flatten()
        .ok_or_else(|| "no fixture was run before settlement".to_owned())?;
    state.settlement.set(outcome);
    Ok(())
}

/// Decode the selected document.
#[when("the adapter decodes the report")]
fn adapter_decodes(state: &AuditState) -> Result<(), String> {
    let document = state
        .document
        .with_ref(String::clone)
        .ok_or_else(|| "no document was selected before decoding".to_owned())?;
    state.decoded.set(decode_report(document.as_bytes()));
    Ok(())
}

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

/// Require that no settlement outcome was recorded.
///
/// A malformed report must be refused by the adapter, so settlement is never
/// asked to decide it. This asserts the boundary rather than relying on the
/// scenario's step order.
#[then("the settlement does not run")]
fn settlement_does_not_run(state: &AuditState) -> Result<(), String> {
    if state.settlement.with_ref(|_| ()).is_some() {
        return Err("settlement ran on a report the adapter refused".to_owned());
    }
    Ok(())
}

/// A theorem of the expected proposition is accepted.
#[ignore = "needs make tools; run make toolchain-smoke"]
#[scenario(
    path = "tests/features/axiom_audit.feature",
    name = "A theorem of the expected proposition is accepted"
)]
fn accepted_ground_is_accepted(#[from(state)] _state: AuditState) {}

/// A theorem depending only on approved foundations is accepted.
#[ignore = "needs make tools; run make toolchain-smoke"]
#[scenario(
    path = "tests/features/axiom_audit.feature",
    name = "A theorem depending only on approved foundations is accepted"
)]
fn accepted_foundations_is_accepted(#[from(state)] _state: AuditState) {}

/// A theorem depending on `sorry` is rejected.
#[ignore = "needs make tools; run make toolchain-smoke"]
#[scenario(
    path = "tests/features/axiom_audit.feature",
    name = "A theorem depending on sorry is rejected"
)]
fn sorry_is_rejected(#[from(state)] _state: AuditState) {}

/// A theorem depending on a native axiom is rejected.
#[ignore = "needs make tools; run make toolchain-smoke"]
#[scenario(
    path = "tests/features/axiom_audit.feature",
    name = "A theorem depending on a native axiom is rejected"
)]
fn native_is_rejected(#[from(state)] _state: AuditState) {}

/// A theorem depending on an undeclared axiom is rejected.
#[ignore = "needs make tools; run make toolchain-smoke"]
#[scenario(
    path = "tests/features/axiom_audit.feature",
    name = "A theorem depending on an undeclared axiom is rejected"
)]
fn undeclared_is_rejected(#[from(state)] _state: AuditState) {}

/// A theorem of another proposition is rejected.
#[ignore = "needs make tools; run make toolchain-smoke"]
#[scenario(
    path = "tests/features/axiom_audit.feature",
    name = "A theorem of another proposition is rejected"
)]
fn wrong_statement_is_rejected(#[from(state)] _state: AuditState) {}

/// An axiom root is rejected for its kind and its dependency.
#[ignore = "needs make tools; run make toolchain-smoke"]
#[scenario(
    path = "tests/features/axiom_audit.feature",
    name = "An axiom root is rejected for its kind and its dependency"
)]
fn axiom_root_is_rejected(#[from(state)] _state: AuditState) {}

/// A root the environment does not hold is rejected.
#[ignore = "needs make tools; run make toolchain-smoke"]
#[scenario(
    path = "tests/features/axiom_audit.feature",
    name = "A root the environment does not hold is rejected"
)]
fn root_absent_is_rejected(#[from(state)] _state: AuditState) {}

/// A rejected replay is a completed audit that rejects.
#[ignore = "needs make tools; run make toolchain-smoke"]
#[scenario(
    path = "tests/features/axiom_audit.feature",
    name = "A rejected replay is a completed audit that rejects"
)]
fn rejected_replay_is_rejected(#[from(state)] _state: AuditState) {}

/// A report naming another toolchain is a tool failure.
#[ignore = "needs make tools; run make toolchain-smoke"]
#[scenario(
    path = "tests/features/axiom_audit.feature",
    name = "A report naming another toolchain is a tool failure"
)]
fn toolchain_mismatch_is_a_tool_failure(#[from(state)] _state: AuditState) {}

/// A malformed report is refused before settlement.
#[ignore = "needs make tools; run make toolchain-smoke"]
#[scenario(
    path = "tests/features/axiom_audit.feature",
    name = "A malformed report is refused before settlement"
)]
fn malformed_report_is_refused(#[from(state)] _state: AuditState) {}
