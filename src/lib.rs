//! Polean development scaffold.
//!
//! The source frontend, policy model, Lean worker, evidence verifier, and
//! runtime conformance paths are not implemented yet. See the repository design
//! documents for their proposed contracts.

/// Human-readable status of the current crate.
pub const STATUS: &str = "design scaffold";

/// Return the project name used by the scaffold.
///
/// # Examples
///
/// ```
/// assert_eq!(polean::project_name(), "Polean", "project identity");
/// ```
#[must_use]
pub const fn project_name() -> &'static str { "Polean" }
