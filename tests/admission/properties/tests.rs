//! The O5 property, the pinned boundary shapes, and the comment-insertion
//! property.

use std::collections::BTreeMap;

use proptest::{
    collection::vec as strategy_vec,
    sample::select,
    strategy::{Strategy, ValueTree},
    test_runner::TestRunner,
};
use rstest::rstest;

use super::case::{self, Case, Gap};
use crate::{
    admission::{self, Decision},
    corpus::{Profile, ReasonCode, Stage},
    recognizer,
};

/// How many generated cases the property checks.
const CASES: usize = 64;

/// The least number of cases that must use each trivia class.
const FLOOR: usize = 4;

/// How many comment insertions the second property checks.
const INSERTIONS: usize = 32;

/// The profile contract, or a panic naming the read failure.
fn profile() -> Profile {
    match crate::corpus::profile_contract() {
        Ok(profile) => profile,
        Err(error) => panic!("the profile must parse: {error}"),
    }
}

#[test]
fn every_generated_canonical_source_is_admitted() {
    let profile = profile();
    let strategy = case::strategy(&profile);
    let mut runner = TestRunner::deterministic();
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for _ in 0..CASES {
        let generated = match strategy.new_tree(&mut runner) {
            Ok(tree) => tree.current(),
            Err(error) => panic!("proptest could not build a case: {error}"),
        };
        for gap in &generated.gaps {
            *counts.entry(gap.name()).or_default() += 1;
        }
        check_case(&profile, &generated);
    }
    for gap in Gap::ALL {
        let seen = counts.get(gap.name()).copied().unwrap_or_default();
        assert!(
            seen >= FLOOR,
            "the {} trivia class appeared {seen} times, below the floor of {FLOOR}",
            gap.name()
        );
    }
}

/// Require one generated source to be admitted as exactly its rule list, and
/// to satisfy the witness and the lexer cross-check.
fn check_case(profile: &Profile, generated: &Case) {
    let source = case::source(profile, generated);
    let classification = admission::classify(profile, &source);
    match &classification.decision {
        Some(Decision::Admitted(actual)) => assert_eq!(
            actual, &generated.ir,
            "the generated source must be admitted as its own rule list"
        ),
        Some(Decision::Rejected(rejection)) => {
            panic!(
                "the generated source was rejected: {}",
                rejection.describe()
            )
        }
        None => panic!("the generated source never reached admission"),
    }
    let tokens = match recognizer::canonical_tokens(profile, &generated.ir) {
        Ok(tokens) => tokens,
        Err(error) => panic!("the rule list must render: {error}"),
    };
    let Some(Ok(offsets)) = &classification.recognizer else {
        panic!("an admitted source must carry a recognizer witness");
    };
    if let Err(detail) = recognizer::confirm(
        source.as_bytes(),
        &tokens.bytes(),
        &tokens.needs_newline,
        offsets,
    ) {
        panic!("the witness refused a generated admission: {detail}");
    }
    if let Err(detail) = recognizer::cross_check(&source, &tokens.text) {
        panic!("the pinned lexer disagrees with a generated source: {detail}");
    }
}

#[test]
fn inserting_text_into_a_comment_does_not_change_the_outcome() {
    let profile = profile();
    let strategy = strategy_vec(select(vec!['a', 'b', 'c', '1', ' ']), 0..12);
    let mut runner = TestRunner::deterministic();
    for _ in 0..INSERTIONS {
        let inserted: String = match strategy.new_tree(&mut runner) {
            Ok(tree) => tree.current().into_iter().collect(),
            Err(error) => panic!("proptest could not build an insertion: {error}"),
        };
        let generated = Case::rectangular(&profile, 1, 1);
        let source = case::source(&profile, &generated);
        assert!(
            source.contains(" # generated comment\n"),
            "the pinned case must contain the comment the property edits"
        );
        let edited = source.replacen(
            " # generated comment\n",
            &format!(" # inserted{inserted}\n"),
            1,
        );
        match &admission::classify(&profile, &edited).decision {
            Some(Decision::Admitted(actual)) => assert_eq!(
                actual, &generated.ir,
                "inserting `{inserted}` into a comment must not change the rule list"
            ),
            Some(Decision::Rejected(rejection)) => panic!(
                "inserting `{inserted}` into a comment rejected the source: {}",
                rejection.describe()
            ),
            None => panic!("inserting `{inserted}` into a comment never reached admission"),
        }
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
