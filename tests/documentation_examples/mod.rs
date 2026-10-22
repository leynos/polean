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
//! - A fence whose language is in [`NON_EXECUTABLE_LANGUAGES`] is exempt, matching the shared
//!   loader, which carves out Mermaid diagrams for the separate diagram gate.
//! - Indented fences are recognised. A fence inside a list item is indented relative to column
//!   zero, so a column-zero-only match would let it escape the contract; a fence and its marker may
//!   each carry up to [`MAXIMUM_INDENT`] leading spaces, and that indentation is stripped from the
//!   returned body.
//!
//! Known limitation: a fenced block nested in a list or block quote is indented relative to its
//! container, and a line-based loader cannot measure that. Only indentation from column zero is
//! modelled, so an example indented further than [`MAXIMUM_INDENT`] escapes the contract instead of
//! failing it. Reading only the two top-level documents keeps that gap narrow, but it is a gap: it
//! is recorded here rather than papered over with a tolerance wide enough to swallow an indented
//! code block, which is prose that shows a fence rather than a fence.

use std::{collections::HashSet, fmt, fs, iter::Enumerate, path::Path, str::Lines};

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

/// Shortest backtick run that opens a fence.
const MINIMUM_FENCE: usize = 3;

/// Most leading spaces a fence, its marker, or a marked body line may carry.
///
/// `CommonMark` treats a backtick run indented further than this as an indented
/// code block, which is prose showing a fence rather than a fence.
const MAXIMUM_INDENT: usize = 3;

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

/// An opening fence: its backtick run length, indentation, and language tag.
#[derive(Clone, Copy)]
struct Fence<'a> {
    /// Number of backticks in the opening run.
    ticks: usize,
    /// Leading spaces before the opening run.
    indent: usize,
    /// Language tag, which may be empty when the author omitted one.
    language: &'a str,
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
fn reject_duplicate_ids(examples: &[TestedExample]) -> Result<(), TestedExampleError> {
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
            Some(fence) if is_non_executable(fence.language) => {
                self.skip_fence(cursor, fence.ticks)
            }
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
        let body = self.read_fence_body(fence_index, fence.ticks, fence.indent)?;
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
        ticks: usize,
        indent: usize,
    ) -> Result<String, TestedExampleError> {
        let mut body = String::new();
        for (_, line) in self.lines.by_ref() {
            if is_closing_fence(line, ticks) {
                return Ok(body);
            }
            body.push_str(strip_indent_of(line, indent));
            body.push('\n');
        }
        Err(TestedExampleError::at(
            &format!("{}:{}", self.source, fence_index + 1),
            "fence is not terminated",
        ))
    }

    /// Consume a non-executable fence, including its closing fence.
    fn skip_fence(&mut self, cursor: &Cursor<'_>, ticks: usize) -> Result<(), TestedExampleError> {
        for (_, line) in self.lines.by_ref() {
            if is_closing_fence(line, ticks) {
                return Ok(());
            }
        }
        Err(cursor.error("fence is not terminated"))
    }
}

/// Parse an opening fence: a backtick run of three or more, then a language.
///
/// An info string containing a backtick is not a valid fence, so such a line
/// reports `None` rather than an odd language tag.
fn opening_fence(line: &str) -> Option<Fence<'_>> {
    let indented = strip_indent(line);
    let run = indented.trim_start_matches('`');
    let ticks = indented.len().checked_sub(run.len())?;
    if ticks < MINIMUM_FENCE {
        return None;
    }
    // `run` begins at the backtick run, so a later backtick is part of the info
    // string, which CommonMark forbids.
    let language = run.trim();
    if language.contains('`') {
        return None;
    }
    Some(Fence {
        ticks,
        indent: line.len() - indented.len(),
        language,
    })
}

/// Report whether a line closes a fence opened with `ticks` backticks.
///
/// A closer is a run of at least as many backticks as the opener, indented by
/// no more than [`MAXIMUM_INDENT`] spaces, with nothing else on the line.
fn is_closing_fence(line: &str, ticks: usize) -> bool {
    let run = strip_indent(line);
    let remaining = run.trim_start_matches('`');
    remaining.is_empty() && run.len() >= ticks
}

/// Report whether a fence language sits outside the tested-example contract.
fn is_non_executable(language: &str) -> bool { NON_EXECUTABLE_LANGUAGES.contains(&language) }

/// Extract a marker's identifier, if the line is a well-formed marker.
fn marker_identifier(line: &str) -> Option<&str> {
    strip_indent(line)
        .strip_prefix(MARKER_PREFIX)
        .and_then(|value| value.strip_suffix(MARKER_SUFFIX))
}

/// Remove up to [`MAXIMUM_INDENT`] leading spaces, leaving further indentation.
fn strip_indent(line: &str) -> &str {
    let spaces = line
        .len()
        .saturating_sub(line.trim_start_matches(' ').len());
    line.get(spaces.min(MAXIMUM_INDENT)..).unwrap_or_default()
}

/// Remove `indent` leading spaces from a marked example's body line.
///
/// The body is returned as Rust source or shell commands for execution, in
/// which leading indentation is significant rather than Markdown layout, so
/// each line is de-indented by the opening fence's own indentation.
fn strip_indent_of(line: &str, indent: usize) -> &str { line.get(indent..).unwrap_or_default() }
