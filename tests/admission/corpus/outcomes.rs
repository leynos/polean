//! O3: every fixture yields the outcome its manifest entry states.
//!
//! The manifest is the corpus's own contract. This module compares each
//! entry's declared outcome with what admission decides and reports every
//! disagreement together. A fixture admission admits is then checked twice
//! more: the witness confirms the recognizer's offsets independently (O4), and
//! the pinned lexer confirms the canonical tokens (O7).

use crate::{
    admission::{self, Classification, Comparison, Decision, Rejection},
    corpus::{self, Entry, Profile},
    recognizer,
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

/// One pass over the corpus, accumulating faults rather than stopping at the
/// first, so a reviewer sees every disagreement at once.
struct Checker<'a> {
    /// The profile every fixture is admitted against.
    profile: &'a Profile,
    /// The faults found so far.
    failures: Vec<String>,
}

impl Checker<'_> {
    /// Check one fixture against its manifest entry.
    fn fixture(&mut self, entry: &Entry) {
        let source = match corpus::source(entry) {
            Ok(source) => source,
            Err(error) => panic!("{error}"),
        };
        let classification = admission::classify(self.profile, &source);
        if let Some(Decision::Rejected(rejection)) = &classification.decision
            && let Some(problem) = span_problem(&entry.label(), rejection, source.len())
        {
            self.failures.push(problem);
        }
        match admission::compare(&entry.expected, &classification) {
            Comparison::Match => self.admitted(entry, &source, &classification),
            Comparison::Mismatch(detail) => {
                self.failures.push(format!("{}: {detail}", entry.label()));
            }
        }
    }

    /// Re-check an admitted source with the witness and the pinned lexer.
    ///
    /// A manifest match that admitted the source must come with recognizer
    /// offsets; one without them is itself a fault, because it means admission
    /// accepted a source whose canonical spelling was never checked.
    fn admitted(&mut self, entry: &Entry, source: &str, classification: &Classification) {
        let Some(Decision::Admitted(ir)) = &classification.decision else {
            return;
        };
        let Some(Ok(offsets)) = &classification.recognizer else {
            self.failures.push(format!(
                "{}: admission admitted the source without a recognizer witness",
                entry.label()
            ));
            return;
        };
        let tokens = match recognizer::canonical_tokens(self.profile, ir) {
            Ok(tokens) => tokens,
            Err(detail) => {
                self.failures.push(format!("{}: {detail}", entry.label()));
                return;
            }
        };
        if let Err(detail) = recognizer::confirm(
            source.as_bytes(),
            &tokens.bytes(),
            &tokens.needs_newline,
            offsets,
        ) {
            self.failures.push(format!(
                "{}: the witness refused the admission: {detail}",
                entry.label()
            ));
        }
        if let Err(detail) = recognizer::cross_check(source, &tokens.text) {
            self.failures.push(format!(
                "{}: the pinned lexer disagrees with the canonical tokens: {detail}",
                entry.label()
            ));
        }
    }
}

#[test]
fn every_fixture_yields_its_manifest_outcome() {
    let entries = manifest();
    let profile = profile();
    let mut checker = Checker {
        profile: &profile,
        failures: Vec::new(),
    };
    for entry in &entries {
        checker.fixture(entry);
    }
    if !checker.failures.is_empty() {
        let shown = checker
            .failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        panic!(
            "{} faults while checking {} fixtures; first faults:\n{shown}",
            checker.failures.len(),
            entries.len()
        );
    }
}
