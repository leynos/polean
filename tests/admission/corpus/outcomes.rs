//! O3: every fixture yields the outcome its manifest entry states.
//!
//! The manifest is the corpus's own contract. This module compares each
//! entry's declared outcome with what admission decides and reports every
//! disagreement together. A fixture admission admits is then checked twice
//! more: the witness confirms the recognizer's offsets independently (O4), and
//! the pinned lexer confirms the canonical tokens (O7).

use crate::{
    admission::{self, Classification, Comparison, Decision, Frontend, Rejection},
    corpus::{self, Entry, Expected, Origin, Profile, ReasonCode, Stage},
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

/// Check every manifest entry against `profile`, returning the faults found.
fn check_corpus(profile: &Profile) -> Vec<String> {
    let entries = manifest();
    let mut checker = Checker {
        profile,
        failures: Vec::new(),
    };
    for entry in &entries {
        checker.fixture(entry);
    }
    checker.failures
}

#[test]
fn every_fixture_yields_its_manifest_outcome() {
    let failures = check_corpus(&profile());
    if !failures.is_empty() {
        let shown = failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        panic!(
            "{} faults while checking the corpus; first faults:\n{shown}",
            failures.len()
        );
    }
}

/// The `nul_in_comment` fixture splits the frontend from the recognizer.
///
/// Regorus reads the NUL inside the trailing comment as the end of its input,
/// so the frontend parses the prefix, the walk admits the baseline rule list,
/// and the rule written after the NUL never reaches either. The recognizer
/// reads the submitted bytes instead, so it must refuse at the NUL rather than
/// accept the module the AST describes.
#[test]
fn a_nul_in_a_trailing_comment_is_hidden_from_the_frontend() {
    let profile = profile();
    let entry = Entry {
        origin: Origin::Generated("nul_in_comment".to_owned()),
        covers: Vec::new(),
        expected: Expected::Rejected {
            stage: Stage::Recognizer,
            code: ReasonCode::TriviaByte,
        },
    };
    let source = corpus::source(&entry).unwrap_or_else(|error| panic!("{error}"));
    let Some(nul) = source.find('\u{0}') else {
        panic!("the fixture must contain a NUL");
    };
    assert!(
        source
            .get(..nul)
            .is_some_and(|prefix| prefix.contains("package authz")),
        "the admissible prefix must come before the NUL"
    );
    assert!(
        source
            .get(nul..)
            .is_some_and(|tail| tail.contains("input.subject.role")),
        "the hidden trailing syntax must come after the NUL"
    );
    let classification = admission::classify(&profile, &source);
    assert!(
        matches!(classification.frontend, Frontend::Parsed(_)),
        "the frontend must parse the prefix and hide the trailing rule"
    );
    assert_eq!(
        classification.recognizer,
        Some(Err(nul)),
        "the recognizer must read the raw bytes and stop at the NUL"
    );
    assert_refused_at_nul(&classification, nul);
}

/// Assert the classification refused the NUL at `nul` as `trivia.byte`.
fn assert_refused_at_nul(classification: &Classification, nul: usize) {
    match &classification.decision {
        Some(Decision::Rejected(rejection)) => {
            assert_eq!(
                rejection.stage,
                Stage::Recognizer,
                "the NUL is the recognizer's refusal, not the frontend's"
            );
            assert_eq!(
                rejection.code,
                ReasonCode::TriviaByte,
                "a NUL outside a comment body is trivia.byte"
            );
            assert_eq!(
                rejection.span,
                Some((nul, nul + 1)),
                "the refusal must point at the NUL itself"
            );
        }
        other => panic!("the recognizer must refuse the submitted bytes, got {other:?}"),
    }
}

#[test]
fn a_swapped_renderer_is_refused_by_the_corpus() {
    let faulty = profile().with_swapped_lock_atoms();
    let failures = check_corpus(&faulty);
    let lock_fixture_failed = failures.iter().any(|failure| {
        [
            "admitted/locked-owner-rule.rego",
            "atom/locked.rego",
            "atom/unlocked.rego",
        ]
        .iter()
        .any(|fixture| failure.contains(fixture))
    });
    assert!(
        lock_fixture_failed,
        "a swapped renderer must make the lock-atom fixtures disagree with the manifest; faults: \
         {failures:?}"
    );
}
