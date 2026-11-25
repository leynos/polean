//! Polean: bounded policy-source ingestion and the policy IR.
//!
//! What exists today is the start of the trusted frontend, not the checker:
//!
//! - [`profile`] restates the profile contract's constants;
//! - [`bounded`] reads at most a fixed number of bytes from any [`std::io::Read`];
//! - [`source`] ingests a policy source, checking only the ingestion bounds, so the bytes it hands
//!   on are exactly the bytes read;
//! - [`ir`] holds the closed policy representation and its canonical encoding.
//!
//! The decoders and admission of Rego, the Lean model, the evidence bundle, and
//! the runtime oracles are later items. Nothing here proves that a source belongs to
//! the profile, that a model claim holds, that translation is correct, or that a
//! runtime agrees with the model.

pub mod bounded;
pub mod ir;
pub mod profile;
pub mod source;

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
