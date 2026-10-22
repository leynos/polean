//! Enforce the tested-example contract over Polean's user-facing documents.
//!
//! `docs/documentation-style-guide.md` requires every executable fenced example
//! in a published, user-facing document to carry a `<!-- tested-example: <id>
//! -->` marker, and to fail the build on an unmarked fence, an unterminated
//! fence, a missing identifier, or a duplicate identifier. These tests apply
//! that contract to the documents listed in
//! [`documentation_examples::DOCUMENT_PATHS`] and pin the loader's failure
//! behaviour, so a malformed document is rejected rather than silently
//! skipped.
//!
//! The loader does not execute the examples it returns; see the module
//! documentation for why execution is deferred.

mod documentation_examples;

use std::collections::BTreeSet;

use documentation_examples::{TestedExample, load_documented_examples, parse_document};

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

/// An identifier may be reused in a different document.
#[test]
fn identifiers_are_scoped_to_a_document() {
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
        "the same identifier should parse in both documents"
    );
}

/// A mermaid fence is exempt, so it needs neither a marker nor a body.
#[test]
fn mermaid_fences_are_exempt() {
    let document = "\
```mermaid
flowchart TD
    A --> B
```
";
    let examples = parse_document("fixture.md", document).expect("mermaid should be exempt");
    assert!(examples.is_empty(), "mermaid should not yield examples");
}

/// A marker and fence indented inside a list item are still contracted.
#[test]
fn indented_examples_are_read_with_their_indentation_stripped() {
    let document = "\
1. Install the tool:

   <!-- tested-example: sample -->
   ```bash
   make design-check
   ```
";
    let examples = parse_document("fixture.md", document).expect("indented example should parse");
    assert_eq!(
        examples,
        vec![TestedExample {
            id: "sample".to_owned(),
            language: "bash".to_owned(),
            body: "make design-check\n".to_owned(),
        }],
        "the fence's own indentation should be stripped from the body",
    );
}

/// An unmarked indented fence is rejected rather than skipped.
#[test]
fn an_unmarked_indented_fence_is_rejected() {
    let document = "1. Run:\n\n   ```bash\n   make lint\n   ```\n";
    let error =
        parse_document("fixture.md", document).expect_err("unmarked fence should be rejected");
    assert!(
        error
            .to_string()
            .contains("missing a tested-example marker"),
        "expected an unmarked-fence rejection in '{error}'",
    );
}

/// An indented code block shows a fence rather than being one.
#[test]
fn a_fence_indented_four_spaces_is_not_a_fence() {
    let document = "    ```bash\n    make lint\n    ```\n";
    let examples = parse_document("fixture.md", document).expect("an indented code block is prose");
    assert!(examples.is_empty(), "an indented code block is not a fence");
}

/// An unmarked tilde fence is rejected rather than silently skipped.
///
/// A tilde fence is as valid as a backtick fence, so a loader that recognised
/// only backticks would let it escape the marker contract.
#[test]
fn an_unmarked_tilde_fence_is_rejected() {
    let document = "~~~bash\nmake lint\n~~~\n";
    let error = parse_document("fixture.md", document)
        .expect_err("unmarked tilde fence should be rejected");
    assert!(
        error
            .to_string()
            .contains("fence is missing a tested-example marker"),
        "expected an unmarked-fence rejection in '{error}'",
    );
}

/// A marked tilde fence is read back with its language and body.
#[test]
fn a_marked_tilde_fence_is_read_back() {
    let document = "\
<!-- tested-example: tilde-sample -->
~~~bash
make design-check
~~~
";
    let examples = parse_document("fixture.md", document).expect("tilde example should parse");
    assert_eq!(
        examples.first().map(|example| example.language.as_str()),
        Some("bash"),
        "the tilde fence's language should be read",
    );
    assert_eq!(
        examples.first().map(|example| example.body.as_str()),
        Some("make design-check\n"),
        "the tilde fence's body should be read",
    );
}

/// A fence does not close on a run of the other fence character.
///
/// Each direction is a separate case: a backtick run inside a tilde fence, and
/// a tilde run inside a backtick fence. Both are body content, because a closer
/// must repeat the opening fence's own character.
#[test]
fn fence_characters_do_not_close_each_other() {
    let cases = [
        (
            "\
<!-- tested-example: tildes-outside -->
~~~markdown
```bash
make lint
```
~~~
",
            "```bash\nmake lint\n```\n",
        ),
        (
            "\
<!-- tested-example: backticks-outside -->
```markdown
~~~bash
make lint
~~~
```
",
            "~~~bash\nmake lint\n~~~\n",
        ),
    ];

    for (document, expected) in cases {
        let examples = parse_document("fixture.md", document)
            .expect("a run of the other fence character is body content");
        assert_eq!(examples.len(), 1, "the example should be read as one fence");
        assert_eq!(
            examples.first().map(|example| example.body.as_str()),
            Some(expected),
            "the inner run should stay in the body: {examples:?}",
        );
    }
}

/// A closing run at least as long as the opener closes the fence.
///
/// The opener is four backticks and the closer five, which is the case the
/// loader's own name promises: a *longer* run. The three-backtick inner run is
/// shorter than the opener, so it is body content rather than a closer.
#[test]
fn a_longer_closing_fence_closes_an_example() {
    let document = "\
<!-- tested-example: nested -->
````markdown
```mermaid
flowchart TD
```
`````
";
    let examples = parse_document("fixture.md", document).expect("nested fence should parse");
    assert_eq!(examples.len(), 1, "the example should be read as one fence");
    assert_eq!(
        examples.first().map(|example| example.language.as_str()),
        Some("markdown"),
        "the outer language should win",
    );
    assert_eq!(
        examples.first().map(|example| example.body.as_str()),
        Some("```mermaid\nflowchart TD\n```\n"),
        "the shorter inner run should stay in the body",
    );
}

/// Trailing whitespace after a closing fence does not defeat the closer.
///
/// `CommonMark` permits spaces and tabs after a closing run, and the shared
/// loader accepts them. Without this, the closer is not recognised, the
/// example swallows the rest of the document, and a following example is lost.
#[test]
fn trailing_whitespace_after_a_closing_fence_is_tolerated() {
    let document = "\
<!-- tested-example: first -->
```bash
make lint
```\t
<!-- tested-example: second -->
```bash
make test
```
";
    let examples = parse_document("fixture.md", document)
        .expect("a trailing-space closer should still close the fence");
    assert_eq!(
        examples.len(),
        2,
        "both examples should be read: {examples:?}"
    );
    assert_eq!(
        examples.first().map(|example| example.body.as_str()),
        Some("make lint\n"),
        "the closer's trailing whitespace must not reach the body",
    );
    assert_eq!(
        examples.get(1).map(|example| example.id.as_str()),
        Some("second"),
        "the following example should survive the closer",
    );
}

/// A body line indented less than its fence is de-indented, not truncated.
///
/// The body is returned as executable text, so a line's remaining characters
/// are content. Removing a fixed byte offset would drop them, and would split a
/// multibyte character when the line carries one.
#[test]
fn a_body_line_indented_less_than_its_fence_keeps_its_text() {
    let document = "\
<!-- tested-example: ragged -->
   ```bash
  make lint
   make test
   日本語
   ```
";
    let examples =
        parse_document("fixture.md", document).expect("a ragged body should still parse");
    assert_eq!(
        examples.first().map(|example| example.body.as_str()),
        Some("make lint\nmake test\n日本語\n"),
        "only leading spaces up to the fence's own indentation should be removed",
    );
}

/// Each malformed document is rejected with a message naming the fault.
#[test]
fn malformed_documents_are_rejected() {
    let cases = [
        (
            "<!-- tested-example: -->\n",
            "tested-example identifier must not be empty",
        ),
        (
            "<!-- tested-example:  -->\n",
            "tested-example identifier must not be empty",
        ),
        (
            "```bash\nmake lint\n```\n",
            "fence is missing a tested-example marker",
        ),
        (
            "<!-- tested-example: sample -->\n```bash\nmake lint\n",
            "fence is not terminated",
        ),
        ("```mermaid\nflowchart TD\n", "fence is not terminated"),
        ("<!-- tested-example: sample -->\n", "marker has no fence"),
        (
            "<!-- tested-example: sample -->\ntext\n",
            "expected an opening fence after marker",
        ),
        (
            "<!-- tested-example: sample -->\n```\nmake lint\n```\n",
            "fence should declare a language",
        ),
        (
            "<!-- tested-example: sample -->\n\n```bash\nmake lint\n```\n",
            "expected an opening fence after marker",
        ),
        (
            concat!(
                "<!-- tested-example: repeated -->\n```bash\nmake lint\n```\n",
                "<!-- tested-example: repeated -->\n```bash\nmake test\n```\n",
            ),
            "duplicate tested-example identifier",
        ),
        (
            // A three-backtick line is content inside a four-backtick fence
            // and must not close it, so this example is unterminated.
            "<!-- tested-example: outer -->\n````markdown\ncode\n```\n",
            "fence is not terminated",
        ),
    ];

    for (document, expected) in cases {
        let error = parse_document("fixture.md", document)
            .expect_err("malformed document should be rejected");
        assert!(
            error.to_string().contains(expected),
            "expected '{expected}' in '{error}'",
        );
    }
}

/// Rejection messages name the document and the offending line.
#[test]
fn rejections_name_the_document_and_line() {
    let document = "prose\n\n```bash\nmake lint\n```\n";
    let error = parse_document("guide.md", document).expect_err("fence should be rejected");
    assert!(
        error.to_string().starts_with("guide.md:3 "),
        "expected a guide.md:3 prefix in '{error}'",
    );
}
