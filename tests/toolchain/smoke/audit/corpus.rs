//! The end-to-end audit corpus.
//!
//! Each fixture is compiled with the pinned Lean, audited by the real worker,
//! decoded by the strict wire adapter, and settled against a hand-written
//! request. The outcome must equal the hand-written manifest line.
//!
//! Nothing here proves the worker truthful. A matching corpus shows the
//! worker and settlement agree with the plan on these twenty-three cases; the
//! settlement proof separately shows what acceptance means if the report is
//! true. Neither substitutes for the other.
//!
//! The two canonical snapshots pin the report shape for an accepted
//! foundations case and a rejected native case. The replay `message` is
//! excluded: it quotes the kernel's diagnostic, which is not part of the
//! contract. `githash` is replaced with a fixed marker, so a pin update does
//! not churn the snapshots; settlement still refuses a report whose pin is not
//! [`PIN`].

use std::process::Output;

use super::{
    manifest::{self, Outcome},
    requests::{self, Entry},
    runner::Corpus,
};
use crate::{
    audit::{
        settlement::decision::settle,
        wire::{DecodeError, PIN, decode_report},
    },
    scratch::{check, check_eq},
};

/// The fixtures to compile, dependencies before dependants.
const FIXTURES: &[&str] = &[
    "AuditFixture.Goal",
    "AuditFixture.ImportedBase",
    "AuditFixture.Claim",
    "AuditFixture.SorryBase",
    "AuditFixture.ReachBase",
    "AuditFixture.ForgeBase",
    "AuditFixture.AcceptedGround",
    "AuditFixture.AcceptedFoundations",
    "AuditFixture.AcceptedClaimShape",
    "AuditFixture.MaximalPolicy",
    "AuditFixture.InitSentinel",
    "AuditFixture.MixedRoots",
    "AuditFixture.Sorry",
    "AuditFixture.SorryImported",
    "AuditFixture.Native",
    "AuditFixture.Undeclared",
    "AuditFixture.ReachImported",
    "AuditFixture.NarrowedFoundations",
    "AuditFixture.WrongStatement",
    "AuditFixture.WrongPolicyLiteral",
    "AuditFixture.Unfolded",
    "AuditFixture.AxiomRoot",
    "AuditFixture.DefRoot",
    "AuditFixture.RootAbsent",
    "AuditFixture.ForgeInit",
    "AuditFixture.ForgeImported",
];

/// The fixtures whose canonical report is snapshotted, with the snapshot name.
///
/// One table, because deciding to compare and naming the snapshot have to
/// agree: two copies of the list could drift, leaving a snapshot that is never
/// checked or a fixture that is compared against another's snapshot.
const SNAPSHOTS: &[(&str, &str)] = &[
    ("AcceptedFoundations", "canonical_accepted_foundations"),
    ("Native", "canonical_native"),
];

/// The fixture whose initializer writes a sentinel only when it runs.
///
/// Named once so a rename cannot silently skip both the pre-run and post-run
/// sentinel checks.
const INIT_SENTINEL: &str = "InitSentinel";

/// The fixture whose worker output is truncated mid-document.
const WORKER_GARBLED: &str = "WorkerGarbled";

/// Every manifest entry must have a request, even without provisioned tools.
#[test]
fn every_manifest_entry_has_a_request() -> Result<(), String> {
    let manifest = manifest::load()?;
    for expected in &manifest {
        let entry = requests::entry(&expected.fixture)?;
        check_eq!(
            entry.module,
            expected.module.join("."),
            "{}: the manifest module and the request module disagree",
            expected.fixture
        );
    }
    Ok(())
}

/// Every fixture settles to its manifest line at the real process boundary.
///
/// Every fixture is run even after one diverges, so a single run names the
/// whole blast radius of a worker fault rather than only its first symptom.
#[test]
#[ignore = "needs make tools; run make toolchain-smoke"]
fn corpus_matches_manifest() -> Result<(), String> {
    let manifest = manifest::load()?;
    let corpus = Corpus::prepare(FIXTURES)?;
    let mut failures = Vec::new();
    for expected in &manifest {
        if let Err(error) = settle_one(&corpus, expected) {
            failures.push(error);
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("\n\n"))
    }
}

/// Run one manifest line and require the settlement to match it.
///
/// # Errors
///
/// Returns a diagnostic naming the fixture, the difference, and both streams.
fn settle_one(corpus: &Corpus, expected: &manifest::Expectation) -> Result<(), String> {
    let entry = requests::entry(&expected.fixture)?;
    if expected.fixture == INIT_SENTINEL {
        check!(
            !corpus.sentinel_present()?,
            "InitSentinel: the sentinel exists before the audit"
        );
    }
    let output = run(corpus, &entry)?;
    let outcome = classify(&entry, &output)?;
    manifest::check(expected, &outcome).map_err(|error| {
        format!(
            "{error}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })?;
    if expected.fixture == INIT_SENTINEL {
        check!(
            !corpus.sentinel_present()?,
            "InitSentinel: the audit ran an initializer"
        );
    }
    if let Some((_, name)) = SNAPSHOTS
        .iter()
        .find(|(fixture, _)| *fixture == expected.fixture.as_str())
    {
        assert_canonical(name, &output.stdout)?;
    }
    Ok(())
}

/// The positive control: `lean --run` does run the sentinel initializer.
#[test]
#[ignore = "needs make tools; run make toolchain-smoke"]
fn sentinel_fires_when_initializers_run() -> Result<(), String> {
    let corpus = Corpus::prepare(&["AuditFixture.Goal", "AuditFixture.InitSentinel"])?;
    check!(
        !corpus.sentinel_present()?,
        "the sentinel exists before the positive control runs"
    );
    let output = corpus.sentinel_probe()?;
    if !output.status.success() {
        return Err(format!(
            "lean --run on SentinelProbe failed with {:?}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    check!(
        corpus.sentinel_present()?,
        "lean --run did not create the sentinel; the fixture cannot detect execution"
    );
    Ok(())
}

/// Run the entry's program and return its captured output.
///
/// # Errors
///
/// Returns a diagnostic when the process cannot be started.
fn run(corpus: &Corpus, entry: &Entry) -> Result<Output, String> {
    if entry.fixture == WORKER_GARBLED {
        return corpus.garbled();
    }
    corpus.worker(&entry.module, &entry.roots)
}

/// Classify a worker run into the manifest's vocabulary.
///
/// # Errors
///
/// Returns a diagnostic when the request is refused, which would mean the
/// corpus itself is inadmissible.
fn classify(entry: &Entry, output: &Output) -> Result<Outcome, String> {
    if !output.status.success() {
        // W1: everything that goes wrong before the report is printed exits
        // non-zero with standard output empty. Without this check a worker that
        // printed a partial report before failing would still be classified as
        // a failure, hiding that its contract was broken.
        if !output.stdout.is_empty() {
            return Err(format!(
                "{} failed with {:?} but left {} bytes on stdout; a worker failure must leave \
                 standard output empty",
                entry.fixture,
                output.status.code(),
                output.stdout.len()
            ));
        }
        return Ok(Outcome::ToolFailure("worker_failed"));
    }
    classify_document(entry, &output.stdout, || {})
}

/// Classify the standard output a successful worker run left behind.
///
/// This is the process-output classification boundary: a document that does
/// not decode is a tool failure and never reaches settlement. `on_decoded`
/// runs exactly when the document decoded, immediately before settlement is
/// asked to decide it, so a caller can observe whether that step was reached.
///
/// # Errors
///
/// Returns a diagnostic when the request is refused, which would mean the
/// corpus itself is inadmissible.
pub(super) fn classify_document(
    entry: &Entry,
    stdout: &[u8],
    mut on_decoded: impl FnMut(),
) -> Result<Outcome, String> {
    match decode_report(stdout) {
        Err(DecodeError::Malformed(_)) => Ok(Outcome::ToolFailure("malformed_report")),
        Err(DecodeError::OutputOverflow) => Ok(Outcome::ToolFailure("output_overflow")),
        Ok(report) => {
            on_decoded();
            let settled = settle(&entry.request, &report, PIN.as_bytes())
                .map_err(|refusal| format!("{}: request refused: {refusal:?}", entry.fixture))?;
            Ok(manifest::outcome(settled))
        }
    }
}

/// Render the canonical report and compare it with the named snapshot.
///
/// # Errors
///
/// Returns a diagnostic when the output is not JSON or cannot be rendered.
fn assert_canonical(name: &str, bytes: &[u8]) -> Result<(), String> {
    let text = canonical(bytes)?;
    compare(name, &text);
    Ok(())
}

/// Compare the rendered report with the committed snapshot.
///
/// This is a separate function so the assertion lives outside a
/// `Result`-returning function, where the repository's lint policy forbids it.
fn compare(name: &str, text: &str) {
    insta::assert_snapshot!(name, text);
}

/// Pretty-print a report, excluding volatile values.
///
/// `message` quotes the kernel's rejection text. It is bounded, never parsed,
/// and deliberately outside the snapshot so a kernel wording change does not
/// look like a contract change. `githash` is replaced with a fixed marker for
/// the same reason: a toolchain pin update must not churn the snapshot, and
/// settlement already refuses a report whose pin is not [`PIN`].
///
/// # Errors
///
/// Returns a diagnostic when `bytes` is not JSON or cannot be rendered.
fn canonical(bytes: &[u8]) -> Result<String, String> {
    let mut document: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("the worker's report is not valid JSON: {error}"))?;
    if let Some(replay) = document
        .get_mut("replay")
        .and_then(serde_json::Value::as_object_mut)
    {
        replay.remove("message");
    }
    if let Some(githash) = document.get_mut("githash") {
        *githash = serde_json::Value::String("<pinned>".to_owned());
    }
    serde_json::to_string_pretty(&document)
        .map_err(|error| format!("cannot render the canonical report: {error}"))
}
