//! The feasibility scenarios, bound to
//! `tests/admission/features/admission_feasibility.feature`.
//!
//! The feature file is the readable specification: the committed baseline is
//! admitted with its committed rule list, and two variants the walk admits are
//! refused by the recognizer for faults only the source bytes show. Every
//! variant is derived from the baseline at test time, so the scenarios cannot
//! drift into describing a different module.
//!
//! The state is a `Slot`-bearing struct rather than a `&mut` world fixture,
//! following `tests/toolchain/bdd.rs`: every step borrows the state immutably
//! and fills one slot.

/// The per-scenario state fixture.
///
/// `rstest`'s fixture expansion wraps a single-expression body in braces, which
/// `unused_braces` rejects. The expectation is scoped to this module so it
/// covers only the generation it is about.
mod fixture {
    #![expect(
        unused_braces,
        reason = "rstest fixture expansion wraps a single-expression body in braces"
    )]

    use rstest::fixture;

    use super::AdmissionState;

    /// The per-scenario state, built empty for each generated test.
    #[fixture]
    pub fn state() -> AdmissionState { AdmissionState::default() }
}

pub use fixture::state;
use rstest_bdd::Slot;
use rstest_bdd_macros::{ScenarioState, given, scenario, then, when};

use crate::{
    admission::{self, Classification, Decision},
    corpus::{self, Entry, Profile, ReasonCode, Stage},
};

/// Cargo recompiles this target when the specification changes.
///
/// `#[scenario]` reads the feature file itself, which Cargo cannot fingerprint,
/// so without this reference an edited feature file can leave a previously
/// expanded binary in place.
const _: [&str; 1] = [include_str!("features/admission_feasibility.feature")];

/// The baseline source and its committed rule list.
const BASELINE: &str = "examples/tenant-write/policy.rego";
const BASELINE_IR: &str = "examples/tenant-write/policy.json";

/// The comparison the parenthesised scenario wraps.
const COMPARISON: &str = "input.subject.tenant == input.resource.tenant";

/// The state one scenario accumulates.
#[derive(Default, ScenarioState)]
struct AdmissionState {
    /// The profile every source is admitted against.
    profile: Slot<Profile>,
    /// The source the scenario classifies.
    source: Slot<String>,
    /// What admission decided about that source.
    classification: Slot<Classification>,
}

/// Load the profile contract the scenarios admit against.
#[given("the polean.tenant-write.v0 profile contract")]
fn load_profile(state: &AdmissionState) -> Result<(), String> {
    state.profile.set(corpus::profile_contract()?);
    Ok(())
}

/// The manifest entry at `path`, or a failure naming the absence.
fn find_entry(path: &str) -> Result<Entry, String> {
    corpus::entries()?
        .into_iter()
        .find(|entry| entry.label() == path)
        .ok_or_else(|| format!("the manifest has no fixture at `{path}`"))
}

/// The committed baseline source, read through the manifest.
fn baseline() -> Result<String, String> { corpus::source(&find_entry(BASELINE)?) }

/// Record the committed baseline as the source to classify.
#[given("the baseline tenant-write source")]
fn baseline_source(state: &AdmissionState) -> Result<(), String> {
    state.source.set(baseline()?);
    Ok(())
}

/// Record the baseline preceded by a comment block whose body spells
/// `METADATA`.
#[given("the baseline source preceded by a METADATA comment block")]
fn metadata_source(state: &AdmissionState) -> Result<(), String> {
    state
        .source
        .set(format!("# METADATA\n# title: admission\n{}", baseline()?));
    Ok(())
}

/// Record the baseline with the first comparison wrapped in parentheses.
#[given("the baseline source with one comparison wrapped in parentheses")]
fn parenthesised_source(state: &AdmissionState) -> Result<(), String> {
    let source = baseline()?;
    let wrapped = format!("({COMPARISON})");
    let derived = source.replacen(COMPARISON, &wrapped, 1);
    if derived == source {
        return Err(format!("the baseline must contain `{COMPARISON}`"));
    }
    state.source.set(derived);
    Ok(())
}

/// Classify the recorded source with the pinned frontend and admission.
#[when("the admission spike classifies it")]
fn classify(state: &AdmissionState) -> Result<(), String> {
    let profile = state
        .profile
        .with_ref(Profile::clone)
        .ok_or_else(|| "the profile was not loaded before classification".to_owned())?;
    let source = state
        .source
        .with_ref(String::clone)
        .ok_or_else(|| "no source was recorded before classification".to_owned())?;
    state
        .classification
        .set(admission::classify(&profile, &source));
    Ok(())
}

/// The classification recorded by the `When` step.
fn recorded(state: &AdmissionState) -> Result<Classification, String> {
    state
        .classification
        .with_ref(Classification::clone)
        .ok_or_else(|| "no source was classified before the assertion".to_owned())
}

/// Admission admitted exactly the baseline's committed rule list.
#[then("it is admitted with the IR in examples/tenant-write/policy.json")]
fn admitted_with_committed_ir(state: &AdmissionState) -> Result<(), String> {
    let expected = corpus::ir_rules(BASELINE_IR)?;
    match recorded(state)?.decision {
        Some(Decision::Admitted(actual)) if actual == expected => Ok(()),
        Some(Decision::Admitted(actual)) => Err(format!(
            "admission admitted {actual:?}, expected the committed IR {expected:?}"
        )),
        Some(Decision::Rejected(rejection)) => Err(format!(
            "admission rejected the source: {}",
            rejection.describe()
        )),
        None => Err("admission never ran".to_owned()),
    }
}

/// The AST walk admitted the module, whatever the recognizer then decided.
#[then("the AST walk alone admits it")]
fn walk_admits_the_module(state: &AdmissionState) -> Result<(), String> {
    let classification = recorded(state)?;
    match &classification.decision {
        Some(Decision::Admitted(_)) => Ok(()),
        Some(Decision::Rejected(_)) if classification.recognizer.is_some() => Ok(()),
        Some(Decision::Rejected(rejection)) => Err(format!(
            "the walk rejected the source: {}",
            rejection.describe()
        )),
        None => Err("the source never reached the walk".to_owned()),
    }
}

/// Admission refused the source at the recognizer stage with `code`.
fn expect_refusal(state: &AdmissionState, code: ReasonCode) -> Result<(), String> {
    match recorded(state)?.decision {
        Some(Decision::Rejected(rejection))
            if rejection.stage == Stage::Recognizer && rejection.code == code =>
        {
            Ok(())
        }
        Some(Decision::Rejected(rejection)) => Err(format!(
            "admission reported {}, expected recognizer.{}",
            rejection.describe(),
            code.as_wire()
        )),
        Some(Decision::Admitted(ir)) => Err(format!("admission admitted {ir:?}")),
        None => Err("admission never ran".to_owned()),
    }
}

/// The refusal is `recognizer.trivia.metadata`.
#[then("it is rejected by the recognizer with code trivia.metadata")]
fn refused_as_metadata(state: &AdmissionState) -> Result<(), String> {
    expect_refusal(state, ReasonCode::TriviaMetadata)
}

/// The refusal is `recognizer.token.mismatch`.
#[then("it is rejected by the recognizer with code token.mismatch")]
fn refused_as_mismatch(state: &AdmissionState) -> Result<(), String> {
    expect_refusal(state, ReasonCode::TokenMismatch)
}

/// The baseline is admitted with its committed rule list.
#[scenario(
    path = "tests/admission/features/admission_feasibility.feature",
    name = "The baseline is admitted with its committed IR"
)]
fn baseline_is_admitted(#[from(state)] _state: AdmissionState) {}

/// A metadata comment is refused after the walk admits the module.
#[scenario(
    path = "tests/admission/features/admission_feasibility.feature",
    name = "A metadata annotation is rejected although Regorus discards it"
)]
fn metadata_annotation_is_rejected(#[from(state)] _state: AdmissionState) {}

/// A redundant parenthesis is refused at the recognizer stage.
#[scenario(
    path = "tests/admission/features/admission_feasibility.feature",
    name = "A parenthesised comparison is caught by the bounded source check"
)]
fn parenthesised_comparison_is_refused(#[from(state)] _state: AdmissionState) {}
