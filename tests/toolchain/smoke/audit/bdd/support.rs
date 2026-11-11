//! Shared state, fixture, and vocabulary for the audit feature scenarios.
//!
//! The scenario module keeps the step definitions and the generated tests; this
//! child module holds what they share without being a step itself: the
//! per-scenario fixture, the accessor that requires settlement to have run, and
//! the parsers for the finding and protocol-breach names a scenario spells.
//! Splitting them keeps `bdd` inside the repository's module-size bound without
//! moving any step out of the module the `#[scenario]` expansion expects.

mod fixture {
    //! The per-scenario state fixture.
    //!
    //! `rstest`'s fixture expansion wraps a single-expression body in braces,
    //! which `unused_braces` rejects. The expectation is scoped to this module
    //! so it covers only the generation it is about.

    #![expect(
        unused_braces,
        reason = "rstest fixture expansion wraps a single-expression body in braces"
    )]

    use rstest::fixture;

    use super::super::AuditState;

    /// The per-scenario state, built empty for each generated test.
    #[fixture]
    pub fn state() -> AuditState { AuditState::default() }
}

pub use fixture::state;

use super::AuditState;
use crate::audit::settlement::decision::{FindingCode, ProtocolBreach, RequestRefusal, Settlement};

/// Borrow the settlement outcome, or report that settlement never ran.
pub fn with_settlement<T>(
    state: &AuditState,
    inspect: impl FnOnce(&Result<Settlement, RequestRefusal>) -> T,
) -> Result<T, String> {
    state
        .settlement
        .with_ref(inspect)
        .ok_or_else(|| "the settlement did not run".to_owned())
}

/// Parse a finding code named by a scenario.
pub fn finding_code(name: &str) -> Result<FindingCode, String> {
    match name {
        "KernelRejected" => Ok(FindingCode::KernelRejected),
        "RootAbsent" => Ok(FindingCode::RootAbsent),
        "WrongKind" => Ok(FindingCode::WrongKind),
        "StatementMismatch" => Ok(FindingCode::StatementMismatch),
        "SorryAxiom" => Ok(FindingCode::SorryAxiom),
        "NativeAxiom" => Ok(FindingCode::NativeAxiom),
        "UndeclaredAxiom" => Ok(FindingCode::UndeclaredAxiom),
        other => Err(format!("unknown finding code {other}")),
    }
}

/// Parse a protocol breach named by a scenario.
pub fn protocol_breach(name: &str) -> Result<ProtocolBreach, String> {
    match name {
        "ToolchainMismatch" => Ok(ProtocolBreach::ToolchainMismatch),
        "ModuleMismatch" => Ok(ProtocolBreach::ModuleMismatch),
        "CoverageMismatch" => Ok(ProtocolBreach::CoverageMismatch),
        "RejectedReplayWithRoots" => Ok(ProtocolBreach::RejectedReplayWithRoots),
        other => Err(format!("unknown protocol breach {other}")),
    }
}
