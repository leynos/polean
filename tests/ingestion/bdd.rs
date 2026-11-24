//! Ingestion and canonical-form scenarios, bound to
//! `tests/ingestion/features/policy_ingestion.feature`.
//!
//! The feature file is the readable specification of the externally observable
//! workflow. There is no command-line interface before roadmap 2.1.4, so these
//! scenarios are the workflow: each one names the library call it makes, and
//! nothing here claims that a source belongs to the profile.
//!
//! The state is a `Slot`-bearing struct rather than a `&mut` world fixture,
//! following `tests/admission/bdd.rs`: every step borrows the state immutably
//! and fills one slot.

mod fixture {
    //! The per-scenario state fixture generated for each BDD scenario.
    //!
    //! `rstest`'s fixture expansion wraps a single-expression body in braces,
    //! which `unused_braces` rejects. The expectation is scoped to this module
    //! so it covers only the generation it is about.

    #![expect(
        unused_braces,
        reason = "rstest fixture expansion wraps a single-expression body in braces"
    )]

    use rstest::fixture;

    use super::IngestionState;

    /// The per-scenario state, built empty for each generated test.
    #[fixture]
    pub fn state() -> IngestionState { IngestionState::default() }
}

pub use fixture::state;
use polean::source::{PolicySource, SourceError};
use rstest_bdd::Slot;
use rstest_bdd_macros::{ScenarioState, given, scenario, then, when};

use crate::support::Counted;

/// Cargo recompiles this target when the specification changes.
///
/// `#[scenario]` reads the feature file itself, which Cargo cannot fingerprint,
/// so without this reference an edited feature file can leave a previously
/// expanded binary in place.
const _: [&str; 1] = [include_str!("features/policy_ingestion.feature")];

/// The size cap the feature names.
const CAP: usize = 65_536;

/// The state one scenario accumulates.
#[derive(Default, ScenarioState)]
struct IngestionState {
    /// The bytes the scenario ingests, when it starts from bytes.
    bytes: Slot<Vec<u8>>,
    /// The bytes actually handed to the reader, for the consumption scenario.
    delivered: Slot<std::rc::Rc<std::cell::Cell<usize>>>,
    /// What ingestion returned.
    outcome: Slot<Result<PolicySource, SourceError>>,
}

/// A source of exactly `length` valid UTF-8 bytes.
fn source_of_length(length: usize) -> Vec<u8> {
    let mut bytes = vec![b'a'; length];
    if let Some(last) = bytes.last_mut() {
        *last = b'\n';
    }
    bytes
}

/// A valid source of exactly `length` bytes.
#[given("a valid UTF-8 policy source of exactly 65536 bytes")]
fn source_at_the_cap(state: &IngestionState) { state.bytes.set(source_of_length(CAP)); }

/// A valid source of exactly one byte over the cap.
#[given("a valid UTF-8 policy source of exactly 65537 bytes")]
fn source_over_the_cap(state: &IngestionState) { state.bytes.set(source_of_length(CAP + 1)); }

/// Ingest the recorded bytes from a stream.
#[when("the source is ingested from a stream")]
fn ingest_from_a_stream(state: &IngestionState) -> Result<(), String> {
    let bytes = recorded_bytes(state)?;
    state.outcome.set(PolicySource::read_from(bytes.as_slice()));
    Ok(())
}

/// Ingest the recorded bytes directly.
#[when("the source is ingested from bytes")]
fn ingest_from_bytes(state: &IngestionState) -> Result<(), String> {
    let bytes = recorded_bytes(state)?;
    state.outcome.set(PolicySource::from_bytes(bytes));
    Ok(())
}

/// The recorded bytes, or a failure naming the absence.
fn recorded_bytes(state: &IngestionState) -> Result<Vec<u8>, String> {
    state
        .bytes
        .with_ref(Clone::clone)
        .ok_or_else(|| "no source bytes were recorded".to_owned())
}

/// Ingest an endless stream through a reader that counts what it hands out.
#[when("the source is ingested from the stream")]
fn ingest_an_endless_stream(state: &IngestionState) {
    let (reader, given) = Counted::new(std::io::repeat(b'a'));
    state.outcome.set(PolicySource::read_from(reader));
    state.delivered.set(given);
}

/// The stream the scenario reads; the `when` step supplies its contents.
#[given("an unbounded stream of policy bytes")]
fn unbounded_stream(state: &IngestionState) { let _ = state; }

/// Ingestion succeeded.
#[then("ingestion succeeds")]
fn ingestion_succeeds(state: &IngestionState) -> Result<(), String> {
    with_outcome(state, |outcome| match outcome {
        Ok(_) => Ok(()),
        Err(error) => Err(format!("ingestion failed: {error}")),
    })?
}

/// The recorded outcome, or a failure naming the absence.
///
/// `SourceError` holds an `io::Error`, so it cannot derive `Clone`; each
/// caller borrows the recorded value instead.
fn with_outcome<T>(
    state: &IngestionState,
    inspect: impl FnOnce(&Result<PolicySource, SourceError>) -> T,
) -> Result<T, String> {
    state
        .outcome
        .with_ref(inspect)
        .ok_or_else(|| "ingestion never ran".to_owned())
}

/// The ingested bytes equal the recorded bytes.
#[then("the ingested bytes equal the original bytes")]
fn bytes_are_preserved(state: &IngestionState) -> Result<(), String> {
    let original = recorded_bytes(state)?;
    with_outcome(state, |outcome| match outcome {
        Ok(source) if source.as_bytes() == original.as_slice() => Ok(()),
        Ok(_) => Err("the ingested bytes differ from the original".to_owned()),
        Err(error) => Err(format!("ingestion failed: {error}")),
    })?
}

/// Ingestion refused the source as too large.
#[then("ingestion fails because the source is too large")]
fn ingestion_reports_too_large(state: &IngestionState) -> Result<(), String> {
    expect_fault(
        state,
        |error| matches!(error, SourceError::TooLarge { .. }),
        "too large",
    )
}

/// Compare the recorded fault against a predicate.
fn expect_fault(
    state: &IngestionState,
    matches: impl Fn(&SourceError) -> bool,
    expected: &str,
) -> Result<(), String> {
    with_outcome(state, |outcome| match outcome {
        Err(error) if matches(error) => Ok(()),
        Err(error) => Err(format!("ingestion reported {error}, expected {expected}")),
        Ok(_) => Err(format!("ingestion succeeded, expected {expected}")),
    })?
}

/// A short source whose byte at `offset` is `byte`.
///
/// The size is irrelevant to the scenario: the offset is what the fault
/// reports, and a short source keeps the case easy to read.
fn source_with_byte_at(offset: usize, byte: u8) -> Vec<u8> {
    let mut bytes = source_of_length(offset.saturating_add(1));
    if let Some(slot) = bytes.get_mut(offset) {
        *slot = byte;
    }
    bytes
}

/// A source whose byte at `offset` starts an invalid sequence.
#[given("a policy source whose byte at offset 10 starts an invalid sequence")]
fn source_with_invalid_utf8(state: &IngestionState) {
    state.bytes.set(source_with_byte_at(10, 0x80));
}

/// A source whose byte at `offset` is NUL.
#[given("a policy source with a NUL byte at offset 20")]
fn source_with_nul(state: &IngestionState) { state.bytes.set(source_with_byte_at(20, 0)); }

/// Ingestion refused the source at `offset` as invalid UTF-8.
#[then("ingestion fails with invalid UTF-8 at offset 10")]
fn ingestion_reports_invalid_utf8(state: &IngestionState) -> Result<(), String> {
    expect_fault(
        state,
        |error| matches!(error, SourceError::InvalidUtf8 { valid_up_to: 10 }),
        "invalid UTF-8 at 10",
    )
}

/// Ingestion refused the source at `offset` as holding NUL.
#[then("ingestion fails with a NUL byte at offset 20")]
fn ingestion_reports_nul(state: &IngestionState) -> Result<(), String> {
    expect_fault(
        state,
        |error| matches!(error, SourceError::Nul { offset: 20 }),
        "a NUL at 20",
    )
}

/// The stream delivered at most one byte past the cap.
#[then("no more than 65537 bytes were read from the stream")]
fn bounded_delivery(state: &IngestionState) -> Result<(), String> {
    let delivered = state
        .delivered
        .with_ref(std::rc::Rc::clone)
        .ok_or_else(|| "no stream was ingested".to_owned())?;
    if delivered.get() <= CAP + 1 {
        Ok(())
    } else {
        Err(format!("the stream delivered {} bytes", delivered.get()))
    }
}

/// A source at the cap is accepted unchanged.
#[scenario(
    path = "tests/ingestion/features/policy_ingestion.feature",
    name = "A source at the size cap is accepted unchanged"
)]
fn source_at_the_cap_is_accepted(#[from(state)] _state: IngestionState) {}

/// A source one byte over the cap is refused.
#[scenario(
    path = "tests/ingestion/features/policy_ingestion.feature",
    name = "A source one byte over the cap is rejected"
)]
fn source_over_the_cap_is_refused(#[from(state)] _state: IngestionState) {}

/// An unbounded stream is refused without being consumed.
#[scenario(
    path = "tests/ingestion/features/policy_ingestion.feature",
    name = "An unbounded stream is rejected without reading on"
)]
fn unbounded_stream_is_refused(#[from(state)] _state: IngestionState) {}

/// Malformed UTF-8 is refused with its offset.
#[scenario(
    path = "tests/ingestion/features/policy_ingestion.feature",
    name = "Malformed UTF-8 is rejected with its offset"
)]
fn malformed_utf8_is_refused(#[from(state)] _state: IngestionState) {}

/// A NUL byte is refused with its offset.
#[scenario(
    path = "tests/ingestion/features/policy_ingestion.feature",
    name = "A NUL byte is rejected with its offset"
)]
fn nul_is_refused(#[from(state)] _state: IngestionState) {}
