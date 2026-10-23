//! Corpus-level tests for the tested-example contract.
//!
//! These tests load the contracted documents as a whole and pin rules that no
//! single-document parse can see: that every listed document contributes its
//! examples, and that an identifier is unique across the whole corpus rather
//! than within one document.

use std::collections::BTreeSet;

use crate::documentation_examples::{
    DOCUMENT_PATHS,
    TestedExample,
    load_document,
    load_documented_examples,
    parse_document,
    read_repository_file,
    reject_duplicate_ids,
    scope::{
        EXECUTION_SCOPE,
        ExecutionScope,
        declared_make_targets,
        execution_scope,
        invoked_make_targets,
    },
};

/// Every contracted document is actually loaded, not just the first one.
///
/// Asserting only that the corpus is non-empty and unique would pass for a
/// loader that silently skipped `docs/users-guide.md`, so this pins the count
/// per document and the total. Deriving the expected count from
/// [`DOCUMENT_PATHS`] catches a document that loads nothing, but not one that
/// is dropped from the list altogether, so the document set is pinned
/// literally as well. Adding or removing a contracted document is therefore a
/// deliberate edit to both the list and this expectation, which is the point:
/// narrowing the contract silently is what must not happen. The set is
/// compared without regard to order, which the contract does not fix.
#[test]
fn every_contracted_document_contributes_its_marked_examples() {
    let mut contracted: Vec<&str> = DOCUMENT_PATHS.to_vec();
    contracted.sort_unstable();
    assert_eq!(
        contracted,
        ["README.md", "docs/users-guide.md"],
        "the set of contracted documents changed; update this expectation deliberately if that is \
         intended",
    );

    let examples = load_documented_examples().expect("contracted documents should satisfy");
    let total: usize = DOCUMENT_PATHS
        .iter()
        .map(|path| {
            let loaded = load_document(path).expect("contracted document should load");
            assert!(
                !loaded.is_empty(),
                "{path} should contribute at least one marked example",
            );
            loaded.len()
        })
        .sum();
    assert_eq!(
        examples.len(),
        total,
        "the corpus should contain every contracted document's examples",
    );
}

/// The corpus carries exactly the identifiers the contracted documents pin.
///
/// The test above compares two counts that both come from this loader, so
/// deleting a marker lowers the per-document count and the corpus count
/// together and the comparison still holds. This test closes that fail-open
/// direction by naming the identifiers literally: an example dropped from
/// either shipped document fails here even though the loader stays
/// self-consistent.
///
/// The expectation is a literal set rather than a count, so adding an example
/// to a contracted document is a deliberate edit here as well as in the
/// document. That is the point: the contract should not narrow or widen
/// without someone saying so.
#[test]
fn the_corpus_carries_exactly_the_expected_identifiers() {
    let examples = load_documented_examples().expect("contracted documents should satisfy");
    let ids: BTreeSet<&str> = examples.iter().map(|example| example.id.as_str()).collect();
    let expected: BTreeSet<&str> = [
        "guide-design-check",
        "guide-proposed-commands",
        "readme-design-check",
        "readme-design-properties",
        "readme-rust-gates",
    ]
    .into_iter()
    .collect();
    assert_eq!(
        ids, expected,
        "the contracted documents' marker identifiers changed; update this expectation \
         deliberately if that is intended",
    );
}

/// A repeated identifier is rejected when the examples come from two documents.
///
/// [`parse_document`] reads one document at a time, so it cannot see this
/// collision; the corpus-level check is the only thing that catches it.
#[test]
fn a_repeated_identifier_across_documents_is_rejected() {
    let document = "\
<!-- tested-example: shared -->
```bash
make lint
```
";
    let first = parse_document("one.md", document).expect("first document should parse");
    let second = parse_document("two.md", document).expect("second document should parse");
    let corpus: Vec<TestedExample> = first.into_iter().chain(second).collect();
    assert_eq!(
        corpus.len(),
        2,
        "the fixture should supply two examples, so the check below is exercised",
    );
    let error = reject_duplicate_ids(&corpus)
        .expect_err("an identifier repeated across documents should be rejected");
    assert!(
        error
            .to_string()
            .contains("duplicate tested-example identifier"),
        "expected a duplicate-identifier rejection in '{error}'",
    );
}

/// Every contracted example records how far the repository can execute it.
///
/// This replaces an earlier check that the corpus was merely non-empty, which
/// every other test here already implied. The style guide asks for each fenced
/// example to be executable and exercised, so the interesting question is not
/// whether examples exist but which of them the repository actually runs. The
/// two sets are compared rather than asserted non-empty: an example added to a
/// document without a recorded scope fails, and a scope left behind after its
/// example is removed fails too.
#[test]
fn every_contracted_example_records_its_execution_scope() {
    let examples = load_documented_examples().expect("contracted documents should satisfy");
    let declared: BTreeSet<&str> = examples.iter().map(|example| example.id.as_str()).collect();
    let classified: BTreeSet<&str> = EXECUTION_SCOPE.iter().map(|(id, _)| *id).collect();
    assert_eq!(
        declared, classified,
        "every contracted example needs a recorded execution scope, and every recorded scope \
         needs a contracted example",
    );
}

/// A fence classified as a gate names only targets the `Makefile` declares.
///
/// The tie this establishes is narrow and worth stating precisely: it checks
/// the `make <target>` invocations a gate fence contains, not the entire fence.
/// Environment preparation around those calls — creating a virtual environment
/// or installing `validation/requirements.txt` — is performed by the workflows
/// through their own steps rather than by the loader, and is outside this check.
/// What the check does pin is the part that can silently rot: a documented gate
/// command that no longer names a real target fails the build here instead of
/// being discovered by a reader whose copy-paste fails.
#[test]
fn gate_fences_invoke_declared_make_targets() {
    let makefile = read_repository_file("Makefile").expect("the Makefile should be readable");
    let declared = declared_make_targets(&makefile);
    assert!(
        declared.contains("test") && declared.contains("design-check"),
        "the Makefile parser should find known targets, or this check is vacuous: {declared:?}",
    );

    let examples = load_documented_examples().expect("contracted documents should satisfy");
    let mut missing: BTreeSet<String> = BTreeSet::new();
    for example in &examples {
        if execution_scope(&example.id) != Some(ExecutionScope::Gate) {
            continue;
        }
        for target in invoked_make_targets(&example.body) {
            if !declared.contains(&target) {
                missing.insert(format!("{} -> make {target}", example.id));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "gate fences name targets the Makefile does not declare: {missing:?}",
    );
}

/// A deferred fence is deferred only while it has nothing to run.
///
/// The recorded reason for exempting `guide-proposed-commands` is that no
/// `polean` binary exists, so the commands it documents cannot be executed.
/// That reason expires the moment a binary target is declared, and this test is
/// what notices: it keeps the exemption honest rather than letting it outlive
/// its cause.
#[test]
fn deferred_fences_have_no_implementation_to_run() {
    let manifest = read_repository_file("Cargo.toml").expect("Cargo.toml should be readable");
    let examples = load_documented_examples().expect("contracted documents should satisfy");
    let deferred: Vec<&str> = examples
        .iter()
        .filter(|example| {
            execution_scope(&example.id) == Some(ExecutionScope::DeferredUnimplemented)
        })
        .map(|example| example.id.as_str())
        .collect();
    assert!(
        !deferred.is_empty(),
        "no example is recorded as deferred, so this check proves nothing; remove it or \
         reclassify an example deliberately",
    );
    assert!(
        !manifest.contains("[[bin]]"),
        "a binary target now exists, so {deferred:?} can be executed and must be reclassified \
         rather than left deferred",
    );
}

/// Every marker identifier is unique across the contracted corpus.
#[test]
fn marker_identifiers_are_unique() {
    let examples = load_documented_examples().expect("contracted documents should satisfy");
    let ids: BTreeSet<&str> = examples.iter().map(|example| example.id.as_str()).collect();
    assert_eq!(
        ids.len(),
        examples.len(),
        "duplicate marker identifiers: {examples:?}",
    );
}

/// Every loaded example carries a non-empty body, language, and identifier.
#[test]
fn loaded_examples_are_well_formed() {
    let examples = load_documented_examples().expect("contracted documents should satisfy");
    for example in &examples {
        assert!(!example.id.is_empty(), "identifier should not be empty");
        assert!(
            !example.language.is_empty(),
            "example '{}' should declare a language",
            example.id,
        );
        assert!(
            example.body.ends_with('\n'),
            "example '{}' should end with a newline",
            example.id,
        );
    }
}

/// A marked fence is read back with its language and exact body.
#[test]
fn a_marked_fence_is_read_back_verbatim() {
    let document = "\
<!-- tested-example: sample -->
```bash
make design-check
```
";
    let examples = parse_document("fixture.md", document).expect("fixture should parse");
    assert_eq!(
        examples,
        vec![TestedExample {
            id: "sample".to_owned(),
            language: "bash".to_owned(),
            body: "make design-check\n".to_owned(),
        }],
        "the loader should return the fenced text unchanged",
    );
}

/// `parse_document` sees one document at a time, so it cannot see a collision.
///
/// This pins that limitation deliberately: an identifier repeated in a
/// *different* document parses cleanly here, which is why
/// [`reject_duplicate_ids`] exists as a separate corpus-level check. Reading
/// this test as licence to reuse an identifier across documents would be
/// wrong; `a_repeated_identifier_across_documents_is_rejected` states the
/// rule that actually governs the corpus.
#[test]
fn parse_document_cannot_detect_a_collision_across_documents() {
    let document = "\
<!-- tested-example: repeated -->
```bash
make lint
```
";
    let first = parse_document("one.md", document).expect("first document should parse");
    let second = parse_document("two.md", document).expect("second document should parse");
    assert_eq!(
        first, second,
        "each document parses independently of the other",
    );
}
