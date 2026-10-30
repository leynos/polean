//! The O2 sweep: every mutant of every parsed fixture is rejected.
//!
//! The test is data-driven over the manifest, so a fixture that stops parsing
//! silently leaves the sweep, and a class that loses its last mutant fails the
//! non-vacuity assertion rather than passing with nothing to check. The counts
//! are a snapshot so a change in the shape of the sweep is reviewed rather than
//! printed.

use std::collections::BTreeMap;

use crate::{
    admission::{self, Decision, Frontend, Rejection},
    ast_mutations::{self, Class, Mutant},
    corpus::{self, Entry, Expected, Profile, Stage},
};

/// A reviewable failure message for one mutant.
fn describe(entry: &str, label: &str, expected: corpus::ReasonCode, actual: &str) -> String {
    format!(
        "{entry} {label}: expected decode/{} but {actual}",
        expected.as_wire()
    )
}

/// The manifest entries the sweep and the control test both read.
fn manifest() -> Vec<Entry> {
    match corpus::entries() {
        Ok(entries) => entries,
        Err(error) => panic!("examples/admission/manifest.json must parse: {error}"),
    }
}

/// The profile both tests check against.
fn profile() -> Profile {
    match corpus::profile_contract() {
        Ok(profile) => profile,
        Err(error) => panic!("the profile must parse: {error}"),
    }
}

/// The source of one entry, or a test failure.
fn source(entry: &Entry) -> String {
    match corpus::source(entry) {
        Ok(source) => source,
        Err(error) => panic!("{error}"),
    }
}

/// The running totals of one sweep.
struct Sweep<'a> {
    /// The profile every mutant is admitted against.
    profile: &'a Profile,
    /// Mutants counted per class.
    counts: BTreeMap<&'static str, usize>,
    /// Fixtures the frontend parsed.
    fixtures: usize,
    /// Mutants the sweep applied.
    tested: usize,
    /// One message per mutant that was not rejected as expected.
    failures: Vec<String>,
}

impl<'a> Sweep<'a> {
    /// An empty sweep, with every class counted at zero.
    fn new(profile: &'a Profile) -> Self {
        Self {
            profile,
            counts: Class::ALL.iter().map(|class| (class.as_str(), 0)).collect(),
            fixtures: 0,
            tested: 0,
            failures: Vec::new(),
        }
    }

    /// Sweep every mutant of one manifest entry.
    fn fixture(&mut self, entry: &Entry) {
        let source = source(entry);
        let classification = admission::classify(self.profile, &source);
        let Frontend::Parsed(ast_json) = &classification.frontend else {
            return;
        };
        self.fixtures += 1;
        let mutants = match ast_mutations::mutants(&source, ast_json) {
            Ok(mutants) => mutants,
            Err(error) => panic!("{}: {error}", entry.label()),
        };
        for mutant in &mutants {
            self.mutant(entry, &source, mutant);
        }
    }

    /// Count one mutant and record it unless it was rejected as expected.
    fn mutant(&mut self, entry: &Entry, source: &str, mutant: &Mutant) {
        self.tested += 1;
        match self.counts.get_mut(mutant.class.as_str()) {
            Some(count) => *count += 1,
            None => panic!("{} is not a counted class", mutant.class.as_str()),
        }
        if let Some(actual) = unexpected(self.profile, source, mutant) {
            self.failures.push(describe(
                &entry.label(),
                &mutant.label,
                mutant.expected,
                &actual,
            ));
        }
    }

    /// Assert the sweep did work, covered every class, and found no failure.
    fn finish(self) {
        assert!(
            self.fixtures > 0,
            "the corpus must contain a parsed fixture"
        );
        assert!(self.tested > 0, "the sweep must produce mutants");
        let empty: Vec<&str> = self
            .counts
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(name, _)| *name)
            .collect();
        assert!(
            empty.is_empty(),
            "these classes produced no mutant: {empty:?}"
        );
        if !self.failures.is_empty() {
            let shown = self
                .failures
                .iter()
                .take(20)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");
            panic!(
                "{} of {} mutants were not rejected as expected; first failures:\n{shown}",
                self.failures.len(),
                self.tested
            );
        }
        insta::with_settings!(
            {prepend_module_to_snapshot => false, snapshot_path => "../snapshots"},
            {
                insta::assert_snapshot!("admission__ast_mutations__counts", self.rendered());
            }
        );
    }

    /// Render the counts snapshot.
    fn rendered(&self) -> String {
        let mut rendered = format!("fixtures  parsed={}\n", self.fixtures);
        for class in Class::ALL {
            let count = self.counts.get(class.as_str()).copied().unwrap_or(0);
            let line = format!("{:<18} mutants={count}\n", class.as_str());
            rendered.push_str(&line);
        }
        let total = format!("total     mutants={}", self.tested);
        rendered.push_str(&total);
        rendered
    }
}

/// The description of a mutant that was not rejected as expected, if any.
fn unexpected(profile: &Profile, source: &str, mutant: &Mutant) -> Option<String> {
    match admission::admit(profile, source, &mutant.ast_json) {
        Decision::Rejected(Rejection {
            stage: Stage::Decode,
            code,
            ..
        }) if code == mutant.expected => None,
        Decision::Rejected(rejection) => Some(rejection.describe()),
        Decision::Admitted(ir) => Some(format!("the walk admitted {ir:?}")),
    }
}

#[test]
fn every_mutant_is_rejected_at_the_decode_stage() {
    let profile = profile();
    let mut sweep = Sweep::new(&profile);
    for entry in &manifest() {
        sweep.fixture(entry);
    }
    sweep.finish();
}

#[test]
fn the_unmutated_ast_of_every_admitted_fixture_is_admitted() {
    let profile = profile();
    let mut checked = 0_usize;
    for entry in &manifest() {
        let Expected::Admitted { .. } = &entry.expected else {
            continue;
        };
        let source = source(entry);
        let classification = admission::classify(&profile, &source);
        match admission::compare(&entry.expected, &classification) {
            admission::Comparison::Match => checked += 1,
            admission::Comparison::Mismatch(detail) => {
                panic!(
                    "{} is not admitted before mutation: {detail}",
                    entry.label()
                )
            }
            admission::Comparison::Gap => {
                panic!("{} is expected admitted but was only a gap", entry.label())
            }
        }
    }
    assert!(checked > 0, "the corpus must contain an admitted fixture");
}
