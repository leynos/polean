//! Loads fenced examples from Polean's user-facing Markdown documents.
//!
//! Polean adopts the tested-example contract recorded in
//! `docs/documentation-style-guide.md`: an executable fenced example in a
//! published, user-facing document is preceded immediately by a
//! `<!-- tested-example: <identifier> -->` marker, so a shared loader reads the
//! example from the shipped document instead of a copied fixture.
//!
//! This module is Polean's loader, ported from the reference implementations in
//! Netsuke (`tests/documentation_examples/mod.rs` at commit `90ca9af`) and the
//! shared library loader
//! (`leynos/agent-helper-scripts`, `tests/tested_example_test_support.py`). It
//! reads the documents listed in [`DOCUMENT_PATHS`] and reports each contract
//! violation by document name and one-based line number.
//!
//! Scope: the loader enforces the marker contract. It does not execute the
//! examples it returns. Polean's `polean` commands are proposals with no
//! implementation, and its design-check fences install dependencies or re-enter
//! `make test`, so execution is deferred. A passing run is evidence that
//! markers are present, unique, and terminated; it is not evidence that the
//! documented commands succeed.
//!
//! Deliberate differences from the Netsuke port:
//!
//! - Markers must be strictly adjacent to their fence. Netsuke tolerates blank lines between the
//!   two; the guide says "immediately", and the shared loader that defines the contract enforces
//!   strict adjacency.
//! - Fence runs are honoured properly. Netsuke strips a bare three backticks, so a four-backtick
//!   fence keeps a stray backtick in its language tag and closes on a shorter run; this loader
//!   captures the whole run and closes only on a run at least as long, as `CommonMark` requires.
//! - Both `CommonMark` fence characters are recognized. The guide's rule is that *every* fenced
//!   example carries a marker, so recognizing only backticks would leave a tilde fence outside the
//!   contract. A fence closes only on its own character, so a backtick run inside a tilde fence is
//!   body content.
//! - A fence whose language is in [`NON_EXECUTABLE_LANGUAGES`] is exempt, matching the shared
//!   loader, which carves out Mermaid diagrams for the separate diagram gate.
//! - Indented fences are recognized. A fence inside a list item is indented relative to column
//!   zero, so a column-zero-only match would let it escape the contract; a fence and its marker may
//!   each carry up to [`fence::MAXIMUM_INDENT`] leading spaces, and that indentation is stripped
//!   from the returned body.
//!
//! Known limitation: a fenced block nested in a list or block quote is indented relative to its
//! container, and a line-based loader cannot measure that. Only indentation from column zero is
//! modelled, so an example indented further than [`fence::MAXIMUM_INDENT`] escapes the contract
//! instead of failing it. Reading only the two top-level documents keeps that gap narrow, but it is
//! a gap: it is recorded here rather than papered over with a tolerance wide enough to swallow an
//! indented code block, which is prose that shows a fence rather than a fence.

mod fence;

pub mod sweep;

use std::{collections::HashSet, fmt, fs, iter::Enumerate, path::Path, str::Lines};

use fence::{Fence, is_closing_fence, opening_fence, strip_indent, strip_indent_of};

/// User-facing documents covered by the tested-example contract.
pub const DOCUMENT_PATHS: &[&str] = &["README.md", "docs/users-guide.md"];

/// Fence languages the tested-example contract does not cover.
///
/// Mermaid fences are diagrams, checked by the repository's separate diagram
/// gate, so they neither require nor reject a marker. Every other language,
/// including a missing tag, is treated as executable: the guide requires a
/// language on every fence, so an unlabelled fence is more likely to be an
/// overlooked example than a deliberate exemption.
pub const NON_EXECUTABLE_LANGUAGES: &[&str] = &["mermaid"];

/// Opening text of a tested-example marker comment.
const MARKER_PREFIX: &str = "<!-- tested-example: ";

/// Closing text of a tested-example marker comment.
const MARKER_SUFFIX: &str = " -->";

/// A marker comment carrying no identifier, rejected with its own message.
const EMPTY_MARKER: &str = "<!-- tested-example: -->";

/// A violation of the tested-example contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestedExampleError {
    /// Document name and, when known, the one-based line number.
    location: String,
    /// What the contract requires at that location.
    message: String,
}

impl TestedExampleError {
    /// Record a contract violation against a document location.
    fn at(location: &str, message: &str) -> Self {
        Self {
            location: location.to_owned(),
            message: message.to_owned(),
        }
    }
}

impl fmt::Display for TestedExampleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.location, self.message)
    }
}

impl std::error::Error for TestedExampleError {}

/// One marked fenced example read from a user-facing document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestedExample {
    /// Stable identifier declared by the `tested-example` marker.
    pub id: String,
    /// Markdown fence language.
    pub language: String,
    /// Exact text inside the fence, including a trailing newline.
    pub body: String,
}

/// A position in the document currently being parsed.
#[derive(Clone, Copy)]
struct Cursor<'a> {
    /// Document name, used verbatim in error messages.
    source: &'a str,
    /// One-based line number.
    line: usize,
}

impl Cursor<'_> {
    /// Build the error for a violation at this position.
    fn error(&self, message: &str) -> TestedExampleError {
        TestedExampleError::at(&format!("{}:{}", self.source, self.line), message)
    }
}

/// Load every marked example from the contracted documents.
///
/// # Errors
///
/// Returns an error when a document cannot be read, a marker is malformed, a
/// fence is unmarked or unterminated, or an identifier is duplicated.
pub fn load_documented_examples() -> Result<Vec<TestedExample>, TestedExampleError> {
    let mut examples = Vec::new();
    for path in DOCUMENT_PATHS {
        examples.extend(load_document(path)?);
    }
    reject_duplicate_ids(&examples)?;
    Ok(examples)
}

/// Load every marked example from one document.
///
/// # Errors
///
/// Returns an error when the document cannot be read or violates the contract.
pub fn load_document(path: &str) -> Result<Vec<TestedExample>, TestedExampleError> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let contents = fs::read_to_string(root.join(path))
        .map_err(|error| TestedExampleError::at(path, &format!("cannot read document: {error}")))?;
    parse_document(path, &contents)
}

/// Parse one document's contents and return its marked examples.
///
/// # Errors
///
/// Returns an error naming the offending line when a marker is malformed, a
/// fence is unmarked or unterminated, or an identifier is duplicated.
pub fn parse_document(
    source: &str,
    contents: &str,
) -> Result<Vec<TestedExample>, TestedExampleError> {
    let mut parser = Parser {
        source,
        lines: contents.lines().enumerate(),
        ids: HashSet::new(),
        examples: Vec::new(),
    };
    parser.run()?;
    Ok(parser.examples)
}

/// Reject a repeated identifier across the whole contracted corpus.
///
/// This is the only check that can see a collision between documents:
/// [`parse_document`] reads one document at a time, so an identifier reused in
/// a second document passes its own check. The rule the corpus obeys is that
/// every identifier is unique across every contracted document, so such a
/// reuse is rejected here rather than allowed. Public so a test can state that
/// corpus-level rule directly.
///
/// # Errors
///
/// Returns an error naming the first identifier that repeats.
pub fn reject_duplicate_ids(examples: &[TestedExample]) -> Result<(), TestedExampleError> {
    let mut seen = HashSet::new();
    for example in examples {
        if !seen.insert(example.id.as_str()) {
            return Err(TestedExampleError::at(
                "documented examples",
                &format!("duplicate tested-example identifier '{}'", example.id),
            ));
        }
    }
    Ok(())
}

/// Line reader and accumulator for one document.
struct Parser<'a> {
    /// Document name, used verbatim in error messages.
    source: &'a str,
    /// Remaining lines, paired with their zero-based index.
    lines: Enumerate<Lines<'a>>,
    /// Identifiers already seen in this document.
    ids: HashSet<String>,
    /// Examples accepted so far.
    examples: Vec<TestedExample>,
}

impl Parser<'_> {
    /// Consume the document, rejecting every contract violation.
    fn run(&mut self) -> Result<(), TestedExampleError> {
        while let Some((line_index, line)) = self.lines.next() {
            let cursor = Cursor {
                source: self.source,
                line: line_index + 1,
            };
            self.visit(&cursor, line)?;
        }
        Ok(())
    }

    /// Apply the contract to one line.
    fn visit(&mut self, cursor: &Cursor<'_>, line: &str) -> Result<(), TestedExampleError> {
        if let Some(identifier) = marker_identifier(line) {
            return self.read_marked(cursor, identifier);
        }
        if line == EMPTY_MARKER {
            return Err(cursor.error("tested-example identifier must not be empty"));
        }
        match opening_fence(line) {
            None => Ok(()),
            Some(fence) if is_non_executable(fence.language) => self.skip_fence(cursor, fence),
            Some(_) => Err(cursor.error("fence is missing a tested-example marker")),
        }
    }

    /// Read the fence that a marker decorates.
    fn read_marked(
        &mut self,
        cursor: &Cursor<'_>,
        identifier: &str,
    ) -> Result<(), TestedExampleError> {
        if identifier.is_empty() {
            return Err(cursor.error("tested-example identifier must not be empty"));
        }
        if !self.ids.insert(identifier.to_owned()) {
            return Err(cursor.error(&format!(
                "duplicate tested-example identifier '{identifier}'"
            )));
        }
        let (fence_index, line) = self
            .lines
            .next()
            .ok_or_else(|| cursor.error("marker has no fence"))?;
        let fence_cursor = Cursor {
            source: self.source,
            line: fence_index + 1,
        };
        let fence = opening_fence(line)
            .ok_or_else(|| fence_cursor.error("expected an opening fence after marker"))?;
        if fence.language.is_empty() {
            return Err(fence_cursor.error("fence should declare a language"));
        }
        let body = self.read_fence_body(fence_index, fence)?;
        self.examples.push(TestedExample {
            id: identifier.to_owned(),
            language: fence.language.to_owned(),
            body,
        });
        Ok(())
    }

    /// Read a fence body up to its closing fence.
    fn read_fence_body(
        &mut self,
        fence_index: usize,
        fence: Fence<'_>,
    ) -> Result<String, TestedExampleError> {
        let mut body = String::new();
        for (_, line) in self.lines.by_ref() {
            if is_closing_fence(line, fence) {
                return Ok(body);
            }
            body.push_str(strip_indent_of(line, fence.indent));
            body.push('\n');
        }
        Err(TestedExampleError::at(
            &format!("{}:{}", self.source, fence_index + 1),
            "fence is not terminated",
        ))
    }

    /// Consume a non-executable fence, including its closing fence.
    fn skip_fence(
        &mut self,
        cursor: &Cursor<'_>,
        fence: Fence<'_>,
    ) -> Result<(), TestedExampleError> {
        for (_, line) in self.lines.by_ref() {
            if is_closing_fence(line, fence) {
                return Ok(());
            }
        }
        Err(cursor.error("fence is not terminated"))
    }
}

/// Report whether a fence language sits outside the tested-example contract.
fn is_non_executable(language: &str) -> bool { NON_EXECUTABLE_LANGUAGES.contains(&language) }

/// Extract a marker's identifier, if the line is a well-formed marker.
fn marker_identifier(line: &str) -> Option<&str> {
    strip_indent(line)
        .strip_prefix(MARKER_PREFIX)
        .and_then(|value| value.strip_suffix(MARKER_SUFFIX))
}
