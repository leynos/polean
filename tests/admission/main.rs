//! The 1.2.2 complete-module admission feasibility spike.
//!
//! One integration binary holds every module so that Regorus, a dev-dependency,
//! is linked once and so that modules used by a single test do not become
//! `dead_code` under the denied-`allow` lint policy. Nothing here is shipped:
//! the spike demonstrates whether the pinned frontend can enforce complete,
//! fail-closed admission of `polean.tenant-write.v0`, and 2.1.2 deletes or
//! promotes it.
//!
//! EP-M1 commits the fixture corpus, its manifest, and the observation of what
//! Regorus itself does with every fixture. EP-M2 adds the closed decoder and the
//! profile walk; EP-M3 adds the canonical-token recognizer.

mod admission;
mod ast;
mod ast_mutations;
mod bdd;
mod corpus;
mod frontend;
mod inventory;
mod observations;
mod properties;
mod recognizer;
