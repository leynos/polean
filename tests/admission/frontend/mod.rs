//! The pinned Regorus frontend, configured exactly as admission requires.
//!
//! One function builds the engine so that every observation, every decoder test,
//! and every mutation is made against the same configuration: a fresh engine per
//! source, with Rego v0 disabled because the profile is Rego v1. The frontend is
//! trusted input (axiom X1 in the plan); this module only fixes its settings and
//! normalises its diagnostics to strings.

use regorus::Engine;

/// The policy name every source is registered under.
///
/// Regorus reports it in parse diagnostics, and the decoder sees it echoed in
/// the AST JSON's `source.file`, so it is named once here.
pub const FILE: &str = "fixture.rego";

/// Parse one source with the pinned frontend.
///
/// # Errors
///
/// Returns the frontend's diagnostic when the source does not parse. The
/// message is Regorus's own text; [`crate::admission`] classifies it.
pub fn parse(source: &str) -> Result<Engine, String> {
    let mut engine = Engine::new();
    engine.set_rego_v0(false);
    engine
        .add_policy(FILE.to_owned(), source.to_owned())
        .map_err(|error| error.to_string())?;
    Ok(engine)
}
