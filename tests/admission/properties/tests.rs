//! The O5 property, the pinned boundary shapes, and the comment-insertion
//! property.

use std::{cell::RefCell, collections::BTreeMap};

use proptest::{
    collection::vec as strategy_vec,
    prop_assert_eq,
    sample::select,
    test_runner::{Config, RngAlgorithm, TestCaseError, TestCaseResult, TestRng, TestRunner},
};
use rstest::rstest;

use super::case::{self, Case, Gap};
use crate::{
    admission::{self, Decision},
    corpus::{Profile, ReasonCode, Stage},
    recognizer,
};

/// How many generated cases the property checks.
const CASES: u32 = 64;

/// The least number of cases that must use each trivia class.
const FLOOR: usize = 4;

/// How many comment insertions the second property checks.
const INSERTIONS: u32 = 32;

/// The profile contract, or a panic naming the read failure.
fn profile() -> Profile {
    match crate::corpus::profile_contract() {
        Ok(profile) => profile,
        Err(error) => panic!("the profile must parse: {error}"),
    }
}

/// A shrinking runner with a fixed seed and no on-disk failure persistence.
///
/// The deterministic seed keeps the run reproducible; `run` still shrinks a
/// failing case and reports the smallest one it finds, and a failing run is
/// raised as a panic naming that case.
fn runner(cases: u32) -> TestRunner {
    let config = Config {
        cases,
        failure_persistence: None,
        ..Config::default()
    };
    TestRunner::new_with_rng(config, TestRng::deterministic_rng(RngAlgorithm::ChaCha))
}

#[test]
fn every_generated_canonical_source_is_admitted() {
    let profile = profile();
    let strategy = case::strategy(&profile);
    let counts: RefCell<BTreeMap<&'static str, usize>> = RefCell::new(BTreeMap::new());
    let outcome = runner(CASES).run(&strategy, |generated| {
        let (_, emitted) = case::render(&profile, &generated);
        for gap in &emitted {
            *counts.borrow_mut().entry(gap.name()).or_default() += 1;
        }
        check_case(&profile, &generated)
    });
    if let Err(error) = outcome {
        panic!("the generated-source property failed: {error}");
    }
    let totals = counts.into_inner();
    for gap in Gap::ALL {
        let seen = totals.get(gap.name()).copied().unwrap_or_default();
        assert!(
            seen >= FLOOR,
            "the {} trivia class appeared {seen} times, below the floor of {FLOOR}",
            gap.name()
        );
    }
}

/// Require one generated source to be admitted as exactly its rule list, and
/// to satisfy the witness and the lexer cross-check.
fn check_case(profile: &Profile, generated: &Case) -> TestCaseResult {
    let source = case::source(profile, generated);
    let classification = admission::classify(profile, &source);
    match &classification.decision {
        Some(Decision::Admitted(actual)) => prop_assert_eq!(
            actual,
            &generated.ir,
            "the generated source must be admitted as its own rule list"
        ),
        Some(Decision::Rejected(rejection)) => {
            return Err(TestCaseError::fail(format!(
                "the generated source was rejected: {}",
                rejection.describe()
            )));
        }
        None => {
            return Err(TestCaseError::fail(
                "the generated source never reached admission",
            ));
        }
    }
    let tokens = match recognizer::canonical_tokens(profile, &generated.ir) {
        Ok(tokens) => tokens,
        Err(error) => {
            return Err(TestCaseError::fail(format!(
                "the rule list must render: {error}"
            )));
        }
    };
    let Some(Ok(offsets)) = &classification.recognizer else {
        return Err(TestCaseError::fail(
            "an admitted source must carry a recognizer witness",
        ));
    };
    if let Err(detail) = recognizer::confirm(
        source.as_bytes(),
        &tokens.bytes(),
        &tokens.needs_newline,
        offsets,
    ) {
        return Err(TestCaseError::fail(format!(
            "the witness refused a generated admission: {detail}"
        )));
    }
    if let Err(detail) = recognizer::cross_check(&source, &tokens.text) {
        return Err(TestCaseError::fail(format!(
            "the pinned lexer disagrees with a generated source: {detail}"
        )));
    }
    Ok(())
}

#[test]
fn a_swapped_renderer_is_refused_for_every_generated_source() {
    let profile = profile();
    let faulty = profile.with_swapped_lock_atoms();
    let strategy = case::strategy(&profile);
    let outcome = runner(CASES).run(&strategy, |generated| {
        let source = case::source(&profile, &generated);
        let names_lock = generated
            .ir
            .iter()
            .flatten()
            .any(|atom| matches!(atom.as_str(), "locked" | "unlocked"));
        match (names_lock, admission::classify(&faulty, &source).decision) {
            (true, Some(Decision::Rejected(_))) | (false, Some(Decision::Admitted(_))) => Ok(()),
            (true, other) => Err(TestCaseError::fail(format!(
                "a swapped renderer must refuse a source naming a lock atom, got {other:?}"
            ))),
            (false, other) => Err(TestCaseError::fail(format!(
                "a swapped renderer must still admit a source naming no lock atom, got {other:?}"
            ))),
        }
    });
    if let Err(error) = outcome {
        panic!("the seeded renderer fault was not caught: {error}");
    }
}

#[test]
fn inserting_text_into_a_comment_does_not_change_the_outcome() {
    let profile = profile();
    let strategy = strategy_vec(select(vec!['a', 'b', 'c', '1', ' ']), 0..12);
    let outcome = runner(INSERTIONS).run(&strategy, |inserted: Vec<char>| {
        let inserted_text: String = inserted.into_iter().collect();
        let generated = Case::rectangular(&profile, 1, 1);
        let source = case::source(&profile, &generated);
        if !source.contains(" # generated comment\n") {
            return Err(TestCaseError::fail(
                "the pinned case must contain the comment the property edits",
            ));
        }
        let edited = source.replacen(
            " # generated comment\n",
            &format!(" # inserted{inserted_text}\n"),
            1,
        );
        match &admission::classify(&profile, &edited).decision {
            Some(Decision::Admitted(actual)) => prop_assert_eq!(
                actual,
                &generated.ir,
                "inserting `{}` into a comment must not change the rule list",
                inserted_text
            ),
            Some(Decision::Rejected(rejection)) => {
                return Err(TestCaseError::fail(format!(
                    "inserting `{inserted_text}` into a comment rejected the source: {}",
                    rejection.describe()
                )));
            }
            None => {
                return Err(TestCaseError::fail(format!(
                    "inserting `{inserted_text}` into a comment never reached admission"
                )));
            }
        }
        Ok(())
    });
    if let Err(error) = outcome {
        panic!("the comment-insertion property failed: {error}");
    }
}

#[rstest]
#[case("the empty rule list", 0, 1, None)]
#[case("the sixteen-by-sixteen maximum", 16, 16, None)]
#[case("seventeen rules", 17, 1, Some(ReasonCode::RuleLimit))]
#[case("seventeen literals", 1, 17, Some(ReasonCode::BodyLimit))]
fn pinned_boundary_shapes(
    #[case] what: &str,
    #[case] rules: usize,
    #[case] atoms: usize,
    #[case] expected: Option<ReasonCode>,
) {
    let profile = profile();
    let generated = Case::rectangular(&profile, rules, atoms);
    let source = case::source(&profile, &generated);
    match (expected, admission::classify(&profile, &source).decision) {
        (None, Some(Decision::Admitted(actual))) => {
            assert_eq!(actual, generated.ir, "{what} must be admitted unchanged");
        }
        (Some(code), Some(Decision::Rejected(rejection))) => {
            assert_eq!(
                rejection.stage,
                Stage::Profile,
                "{what} must be rejected by the profile"
            );
            assert_eq!(rejection.code, code, "{what} must report {code:?}");
        }
        (_, other) => panic!("{what}: unexpected decision {other:?}"),
    }
}
