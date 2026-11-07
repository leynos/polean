//! The hand-written corpus manifest, and the comparison against a run.
//!
//! `examples/audit/manifest.json` is JSON Lines with one expectation per
//! fixture. It was written before the worker existed and is never generated
//! from a report, so a matching line is evidence that the worker and the
//! settlement agree with the plan rather than with each other.

use crate::{
    ambient,
    audit::settlement::decision::{Finding, FindingCode, ProtocolBreach, Settlement},
    json::{array, field, parse_json, string},
    support::repository_path,
};

/// One finding the manifest expects.
pub struct ExpectedFinding {
    /// The root the finding is about, or `None` for the whole report.
    pub root: Option<usize>,
    /// The finding code's contractual name.
    pub code: String,
}

/// One manifest line.
pub struct Expectation {
    /// The fixture this line describes.
    pub fixture: String,
    /// The module the line audits, in components.
    pub module: Vec<String>,
    /// `accepted`, `rejected`, or `tool_failure`.
    pub settlement: String,
    /// The findings the settlement must report, in order.
    pub findings: Vec<ExpectedFinding>,
    /// The tool-failure reason, when `settlement` is `tool_failure`.
    pub reason: Option<String>,
}

/// What a corpus run produced for one fixture.
pub enum Outcome {
    /// Settlement accepted the report.
    Accepted,
    /// Settlement rejected the report with these findings.
    Rejected(Vec<(Option<usize>, &'static str)>),
    /// The run failed before or during settlement, for this reason.
    ToolFailure(&'static str),
}

/// Read the manifest.
///
/// # Errors
///
/// Returns a diagnostic when the manifest cannot be read or a line is not the
/// expected shape.
pub fn load() -> Result<Vec<Expectation>, String> {
    let path = repository_path("examples/audit/manifest.json");
    let text = ambient::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
        .map(|(index, line)| parse_line(line, index))
        .collect()
}

/// Parse one JSON Lines entry.
///
/// # Errors
///
/// Returns a diagnostic naming the line when a field is missing or malformed.
fn parse_line(line: &str, index: usize) -> Result<Expectation, String> {
    let context = format!("manifest line {}", index + 1);
    let document = parse_json(line, &context)?;
    let findings = array(&document, "findings", &context)?
        .iter()
        .map(|finding| {
            let root = field(finding, "root", &context)?
                .as_u64()
                .map(|number| {
                    usize::try_from(number)
                        .map_err(|error| format!("{context}: root does not fit usize: {error}"))
                })
                .transpose()?;
            let code = string(finding, "code", &context)?.to_owned();
            Ok(ExpectedFinding { root, code })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let reason = field(&document, "reason", &context)?
        .as_str()
        .map(str::to_owned);
    let module = array(&document, "module", &context)?
        .iter()
        .map(|component| {
            component
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{context}: module component is not a string"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Expectation {
        fixture: string(&document, "fixture", &context)?.to_owned(),
        module,
        settlement: string(&document, "settlement", &context)?.to_owned(),
        findings,
        reason,
    })
}

/// Turn settlement's outcome into the form the manifest records.
#[must_use]
pub fn outcome(settlement: Settlement) -> Outcome {
    match settlement {
        Settlement::Accepted => Outcome::Accepted,
        Settlement::Rejected(findings) => {
            Outcome::Rejected(findings.iter().map(describe).collect())
        }
        Settlement::ToolFailure(breach) => Outcome::ToolFailure(breach_name(breach)),
    }
}

/// One finding as its root and contractual code.
#[must_use]
const fn describe(finding: &Finding) -> (Option<usize>, &'static str) {
    (finding.root, code_name(finding.code))
}

/// A protocol breach as its diagnostic name.
///
/// The manifest has no contract spelling for a breach, because a breach is a
/// failure of the worker rather than of the report's content. Naming it keeps
/// the failure message specific when a corpus run goes wrong.
#[must_use]
const fn breach_name(breach: ProtocolBreach) -> &'static str {
    match breach {
        ProtocolBreach::ToolchainMismatch => "toolchain_mismatch",
        ProtocolBreach::ModuleMismatch => "module_mismatch",
        ProtocolBreach::CoverageMismatch => "coverage_mismatch",
        ProtocolBreach::RejectedReplayWithRoots => "rejected_replay_with_roots",
    }
}

/// The contractual name of a finding code.
#[must_use]
pub const fn code_name(code: FindingCode) -> &'static str {
    match code {
        FindingCode::KernelRejected => "kernel_rejected",
        FindingCode::RootAbsent => "root_absent",
        FindingCode::WrongKind => "wrong_kind",
        FindingCode::StatementMismatch => "statement_mismatch",
        FindingCode::SorryAxiom => "sorry_axiom",
        FindingCode::NativeAxiom => "native_axiom",
        FindingCode::UndeclaredAxiom => "undeclared_axiom",
    }
}

/// Require an outcome to match the manifest line.
///
/// # Errors
///
/// Returns a diagnostic naming the fixture and the first difference.
pub fn check(expected: &Expectation, outcome: &Outcome) -> Result<(), String> {
    let actual = match outcome {
        Outcome::Accepted => "accepted",
        Outcome::Rejected(_) => "rejected",
        Outcome::ToolFailure(_) => "tool_failure",
    };
    if actual != expected.settlement {
        return Err(format!(
            "{}: settlement is {actual}, manifest expects {}",
            expected.fixture, expected.settlement
        ));
    }
    match outcome {
        Outcome::Rejected(findings) => compare_findings(expected, findings),
        Outcome::ToolFailure(reason) => {
            if expected.reason.as_deref() == Some(*reason) {
                return Ok(());
            }
            Err(format!(
                "{}: tool failure reason is {reason}, manifest expects {:?}",
                expected.fixture, expected.reason
            ))
        }
        Outcome::Accepted => Ok(()),
    }
}

/// Require the finding list to match, in order.
///
/// # Errors
///
/// Returns a diagnostic naming the fixture, the position, and the difference.
fn compare_findings(
    expected: &Expectation,
    findings: &[(Option<usize>, &'static str)],
) -> Result<(), String> {
    if findings.len() != expected.findings.len() {
        return Err(format!(
            "{}: {} findings, manifest expects {}",
            expected.fixture,
            findings.len(),
            expected.findings.len()
        ));
    }
    for (index, (wanted, (root, code))) in expected.findings.iter().zip(findings).enumerate() {
        if wanted.root != *root || wanted.code != *code {
            return Err(format!(
                "{}: finding {index} is ({root:?}, {code}), manifest expects ({:?}, {})",
                expected.fixture, wanted.root, wanted.code
            ));
        }
    }
    Ok(())
}
