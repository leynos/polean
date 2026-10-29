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
//! Regorus itself does with every fixture. Classification arrives in EP-M2 and
//! EP-M3.

mod corpus;
mod inventory;
mod observations;
