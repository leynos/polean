//! Ingestion and IR behaviour over the public library API.
//!
//! This binary is where the externally observable workflow is described: the
//! behaviour-driven scenarios in `features/`, the properties that quantify over
//! generated sources and policies, the contract-drift checks, and the tests
//! over the committed fixtures. It builds one binary so that a module used by a
//! single target does not become `dead_code` under the denied-`allow` lint
//! policy, and so `cap-std` is linked once.
//!
//! Nothing here is an end-to-end test of a checker. There is no command-line
//! interface before roadmap 2.1.4, so the scenarios stand in for one, and every
//! claim they establish is about the library functions they call.

mod bdd;
mod contracts;
mod properties;
mod support;
