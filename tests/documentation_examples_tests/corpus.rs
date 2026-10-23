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
    reject_duplicate_ids,
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

/// The contracted documents satisfy the marker contract.
#[test]
fn contracted_documents_satisfy_the_marker_contract() {
    let examples = load_documented_examples().expect("contracted documents should satisfy");
    assert!(
        !examples.is_empty(),
        "the contracted documents should declare at least one marked example",
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
