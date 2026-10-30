//! The 1.2.2 fixture corpus and the manifest that describes it.
//!
//! Every fixture is the same baseline with one deliberate difference, named on
//! its first line. The manifest lists each fixture with the AST node kinds it
//! is intended to add and the outcome it must yield. Nothing here decides
//! admission: EP-M1 records the corpus and observes the frontend, and the
//! classifier arrives with EP-M2 and EP-M3.
//!
//! The manifest is included with [`include_str!`] so that Cargo tracks it. A
//! committed `.rego` file that the manifest does not list, or a manifest entry
//! with no file, fails the bijection test.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use serde_json::Value as Json;

mod generated;
mod manifest;
mod profile;

pub use manifest::{Entry, Expected, Origin, ReasonCode, Stage};
pub use profile::Profile;

/// The corpus manifest, as committed.
pub const MANIFEST: &str = include_str!("../../../examples/admission/manifest.json");

/// Every generated fixture name, as the manifest lists them.
pub const GENERATED_NAMES: [&str; 5] = generated::NAMES;

/// The profile contract, as committed.
const PROFILE: &str = include_str!("../../../contracts/profile.json");

/// Parse the committed manifest.
pub fn entries() -> Result<Vec<Entry>, String> { manifest::parse(MANIFEST) }

/// The source bytes of one entry.
pub fn source(entry: &Entry) -> Result<String, String> {
    match &entry.origin {
        Origin::File(path) => std::fs::read_to_string(repository_path(path))
            .map_err(|error| format!("cannot read `{path}`: {error}")),
        Origin::Generated(name) => {
            generated::source(name).ok_or_else(|| format!("no generated fixture is named `{name}`"))
        }
    }
}

/// Resolve a repository-relative path.
#[must_use]
pub fn repository_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// Read a hand-written IR file as its rule list.
pub fn ir_rules(relative: &str) -> Result<Vec<Vec<String>>, String> {
    let text = std::fs::read_to_string(repository_path(relative))
        .map_err(|error| format!("cannot read `{relative}`: {error}"))?;
    let value: Json = serde_json::from_str(&text)
        .map_err(|error| format!("`{relative}` is not JSON: {error}"))?;
    let rules = value
        .get("rules")
        .and_then(Json::as_array)
        .ok_or_else(|| format!("`{relative}` has no `rules` array"))?;
    rules
        .iter()
        .enumerate()
        .map(|(index, rule)| {
            rule.as_array()
                .ok_or_else(|| format!("`{relative}` rule {index} is not an array"))?
                .iter()
                .map(|atom| {
                    atom.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| format!("`{relative}` rule {index} has a non-string atom"))
                })
                .collect()
        })
        .collect()
}

/// The profile the admission spike enforces.
///
/// # Errors
///
/// Returns a description of the first missing, mistyped, or contradictory key
/// in the committed contract.
pub fn profile_contract() -> Result<Profile, String> { profile::profile(PROFILE) }

/// The atom names registered by `contracts/profile.json`.
///
/// # Errors
///
/// Returns a description of the first fault in the committed contract.
pub fn profile_atoms() -> Result<BTreeSet<String>, String> { Ok(profile_contract()?.atom_names()) }

mod outcomes;
#[cfg(test)]
mod tests;
