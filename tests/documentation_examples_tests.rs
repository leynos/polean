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
//!
//! The tests are grouped by what they exercise. Only this root declares the
//! modules: a `#[path]` attribute names each child's file, because a child
//! placed in a subdirectory whose name matches a sibling module would otherwise
//! be looked up beside the root instead.
//!
//! - [`corpus`] checks rules that span the contracted documents.
//! - [`fence_and_parser`] checks how one document is read and rejected.
//! - [`sweeps`] carries the bounded shape sweeps.

mod documentation_examples;

#[path = "documentation_examples_tests/corpus.rs"]
mod corpus;
#[path = "documentation_examples_tests/fence_and_parser.rs"]
mod fence_and_parser;
#[path = "documentation_examples_tests/sweeps.rs"]
mod sweeps;
