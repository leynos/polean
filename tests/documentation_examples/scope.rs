//! Recorded execution scope for each contracted example.
//!
//! The style guide asks for every fenced example in a user-facing document to
//! be executable and exercised by the test suite. Polean cannot meet that for
//! every fence it ships, so this module records what happens instead of leaving
//! the gap implicit: [`EXECUTION_SCOPE`] classifies each contracted example,
//! and the corpus tests hold the classification to the repository.
//!
//! This lives beside the loader rather than inside it because it is a claim
//! about the repository's fences, not part of the loader's parsing contract.
//! The loader decides whether a fence is contracted; this module records how
//! far each contracted fence is exercised.

use std::collections::BTreeSet;

/// How a contracted example relates to the commands the repository runs.
///
/// [`EXECUTION_SCOPE`] classifies every contracted identifier, and the corpus
/// tests fail on an identifier missing from the table or on an entry no
/// document declares. A new fence therefore cannot inherit an exemption by
/// silence, which is the failure mode a comment alone would not catch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionScope {
    /// The fence's `make` invocations name targets this repository already runs.
    ///
    /// The corpus tests hold each `Gate` fence to the `Makefile`, so a
    /// documented gate command that names a target no longer declared fails the
    /// build. What that establishes is narrow: the targets exist and the
    /// workflows run them, not that the loader ran the fence. Environment
    /// preparation around the calls, such as creating a virtual environment, is
    /// performed by the workflows' own steps and is outside the check. The
    /// README's `make test` fence also re-enters the gate that loads it, which
    /// is the second reason the loader classifies rather than executes.
    Gate,
    /// The fence documents a command line that has no implementation to run.
    ///
    /// `polean` is a proposed interface; no binary target exists for it. The
    /// corpus tests hold this to `Cargo.toml` and start failing once such a
    /// binary is declared, so the deferral cannot outlive its cause.
    DeferredUnimplemented,
}

/// Execution scope of every contracted example, keyed by marker identifier.
///
/// Deliberately exhaustive and literal, in the same spirit as the identifier
/// set the corpus tests pin: widening the exception should be a deliberate edit
/// to this table rather than a side effect elsewhere.
pub const EXECUTION_SCOPE: &[(&str, ExecutionScope)] = &[
    ("readme-design-check", ExecutionScope::Gate),
    ("readme-design-properties", ExecutionScope::Gate),
    ("readme-rust-gates", ExecutionScope::Gate),
    (
        "guide-proposed-commands",
        ExecutionScope::DeferredUnimplemented,
    ),
    ("guide-design-check", ExecutionScope::Gate),
];

/// Report the recorded execution scope of a marker identifier.
#[must_use]
pub fn execution_scope(identifier: &str) -> Option<ExecutionScope> {
    EXECUTION_SCOPE
        .iter()
        .find(|(id, _)| *id == identifier)
        .map(|(_, scope)| *scope)
}

/// Extract the target names a `Makefile` declares.
///
/// Only a target definition begins at column zero; recipes are indented, and
/// `NAME := value` assignments and comments are excluded by the name checks.
/// The returned set holds target *names*, so a rule declaring several targets
/// on one line contributes each one.
#[must_use]
pub fn declared_make_targets(makefile: &str) -> BTreeSet<String> {
    makefile
        .lines()
        .filter(|line| !line.starts_with(['\t', ' ', '#']))
        .filter_map(|line| line.split_once(':'))
        .map(|(name, _)| name)
        .filter(|name| {
            !name.is_empty()
                && !name.starts_with('.')
                && !name.contains(char::is_whitespace)
                && name
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
        })
        .map(str::to_owned)
        .collect()
}

/// Extract the targets a fenced example invokes with `make`.
///
/// Only a line whose first word is `make` contributes; the target is the next
/// word. Lines that prepare the environment are ignored, which is the
/// limitation [`ExecutionScope::Gate`] records.
#[must_use]
pub fn invoked_make_targets(body: &str) -> Vec<String> {
    body.lines()
        .filter_map(|line| line.trim().strip_prefix("make "))
        .filter_map(|rest| rest.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}
