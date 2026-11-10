//! The provisioned audit corpus: the worker, the fixtures, and settlement.
//!
//! `corpus` runs the real `polean-audit` worker over twenty-two committed Lean
//! fixtures and requires each settlement to equal the hand-written manifest.
//! `sentinel_fires_when_initializers_run` is the positive control for the
//! no-execution claim: importing a module must not run its initializer, while
//! `lean --run` on the same module must.
//!
//! `requests` holds the hand-written request for each fixture, including the
//! expected statement trees. `runner` builds the worker and compiles the
//! fixtures in a scratch tree. `manifest` reads the committed expectations and
//! compares them with a run. `render` builds and runs the Lean-side tests of the
//! renderer's bounds and the diagnostic truncation, which the corpus cannot
//! reach.
//!
//! These tests are `#[ignore]`d because they need `make tools`. The one
//! exception, `corpus::every_manifest_entry_has_a_request`, reads files only
//! and runs under `make test`.

mod corpus;
mod manifest;
mod render;
mod requests;
mod runner;
