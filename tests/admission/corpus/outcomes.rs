//! O3: every fixture yields the outcome its manifest entry states.
//!
//! The manifest is the corpus's own contract. This module compares each
//! entry's declared outcome with what admission decides, reports every
//! disagreement together, and requires the only remaining gaps to be the
//! recognizer-stage fixtures EP-M3 is due to close.

use std::collections::BTreeSet;

use crate::{
    admission::{self, Comparison, Decision, Rejection},
    corpus::{self, Entry, Expected, Profile, Stage},
};

/// The parsed manifest, or a panic naming the first parse failure.
fn manifest() -> Vec<Entry> {
    match corpus::entries() {
        Ok(entries) => entries,
        Err(error) => panic!("examples/admission/manifest.json must parse: {error}"),
    }
}

/// The profile contract, or a panic naming the parse failure.
fn profile() -> Profile {
    match corpus::profile_contract() {
        Ok(profile) => profile,
        Err(error) => panic!("the profile must parse: {error}"),
    }
}

/// The source of one entry, or a panic naming the read failure.
fn source(entry: &Entry) -> String {
    match corpus::source(entry) {
        Ok(source) => source,
        Err(error) => panic!("{error}"),
    }
}

/// Whether the manifest expects a recognizer-stage rejection.
const fn expects_recognizer(expected: &Expected) -> bool {
    matches!(
        expected,
        Expected::Rejected {
            stage: Stage::Recognizer,
            ..
        }
    )
}

/// The failure message for a rejection span that is not inside the source.
fn span_problem(label: &str, rejection: &Rejection, len: usize) -> Option<String> {
    let (start, end) = rejection.span?;
    if start <= end && end <= len {
        return None;
    }
    Some(format!(
        "{label}: the rejection span [{start}, {end}) is not inside the source"
    ))
}

/// Check one fixture, recording a failure and returning its gap label.
fn check_fixture(
    profile: &Profile,
    entry: &Entry,
    source: &str,
    failures: &mut Vec<String>,
) -> Option<String> {
    let classification = admission::classify(profile, source);
    if let Some(Decision::Rejected(rejection)) = &classification.decision
        && let Some(problem) = span_problem(&entry.label(), rejection, source.len())
    {
        failures.push(problem);
    }
    match admission::compare(&entry.expected, &classification) {
        Comparison::Match => None,
        Comparison::Gap if expects_recognizer(&entry.expected) => Some(entry.label()),
        Comparison::Gap => {
            failures.push(format!(
                "{}: only a recognizer-stage fixture may be a gap",
                entry.label()
            ));
            None
        }
        Comparison::Mismatch(detail) => {
            failures.push(format!("{}: {detail}", entry.label()));
            None
        }
    }
}

#[test]
fn every_fixture_yields_its_manifest_outcome() {
    let entries = manifest();
    let profile = profile();
    let mut failures = Vec::new();
    let mut gaps = BTreeSet::new();
    for entry in &entries {
        if let Some(label) = check_fixture(&profile, entry, &source(entry), &mut failures) {
            gaps.insert(label);
        }
    }
    let expected_gaps: BTreeSet<String> = entries
        .iter()
        .filter(|entry| expects_recognizer(&entry.expected))
        .map(Entry::label)
        .collect();
    if !failures.is_empty() {
        let shown = failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        panic!(
            "{} of {} fixtures did not yield their manifest outcome; first mismatches:\n{shown}",
            failures.len(),
            entries.len()
        );
    }
    assert_eq!(
        gaps, expected_gaps,
        "exactly the recognizer-stage fixtures must remain the only gaps"
    );
}
