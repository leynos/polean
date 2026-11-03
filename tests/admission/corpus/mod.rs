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

use std::collections::BTreeSet;

use cap_std::{ambient_authority, fs::Dir};
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

/// The baseline every generated fixture is derived from.
pub use generated::BASELINE;

/// The profile contract, as committed.
const PROFILE: &str = include_str!("../../../contracts/profile.json");

/// Parse the committed manifest and load every admitted entry's IR rules.
///
/// The IR read happens here, at the boundary that already owns corpus I/O, so
/// [`crate::admission::compare`] stays a pure comparison: a missing or
/// malformed IR file surfaces as a fixture fault from this loader rather than
/// as a comparison mismatch during a test.
///
/// # Errors
///
/// Returns a description of the first manifest or IR failure.
pub fn entries() -> Result<Vec<Entry>, String> {
    let mut entries = manifest::parse(MANIFEST)?;
    for entry in &mut entries {
        if let Expected::Admitted { ir, rules } = &mut entry.expected {
            *rules = ir_rules(ir)?;
        }
    }
    Ok(entries)
}

/// The source bytes of one entry.
pub fn source(entry: &Entry) -> Result<String, String> {
    match &entry.origin {
        Origin::File(path) => read_text(path),
        Origin::Generated(name) => {
            generated::source(name).ok_or_else(|| format!("no generated fixture is named `{name}`"))
        }
    }
}

/// Open the repository root as a capability handle.
///
/// Every corpus read goes through this handle, so the tests never touch an
/// ambient working directory.
///
/// # Errors
///
/// Returns a description of any failure to open the crate directory.
pub fn root() -> Result<Dir, String> {
    Dir::open_ambient_dir(env!("CARGO_MANIFEST_DIR"), ambient_authority())
        .map_err(|error| format!("cannot open the repository root: {error}"))
}

/// Read a repository-relative text file.
///
/// # Errors
///
/// Returns a description of any failure to open or read `relative`.
pub fn read_text(relative: &str) -> Result<String, String> {
    root()?
        .read_to_string(relative)
        .map_err(|error| format!("cannot read `{relative}`: {error}"))
}

/// The `id` the committed profile contract declares.
///
/// # Errors
///
/// Returns a description of a contract that is not JSON or has no string `id`.
fn profile_id() -> Result<String, String> {
    let contract: Json = serde_json::from_str(PROFILE)
        .map_err(|error| format!("contracts/profile.json is not JSON: {error}"))?;
    contract
        .get("id")
        .and_then(Json::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "contracts/profile.json has no string `id`".to_owned())
}

/// Read a hand-written IR file as its rule list.
///
/// The IR is the recognizer's expected input, so it is read closed: the root
/// must be an object holding exactly `profile` and `rules`, the profile must
/// name the contract this harness enforces, and each rule must be an array of
/// atom names. A misspelt or extra key is refused rather than ignored, so a
/// typo cannot leave part of the expected rule list silently unchecked.
///
/// # Errors
///
/// Returns a description of the first malformed field.
pub fn ir_rules(relative: &str) -> Result<Vec<Vec<String>>, String> {
    let text = read_text(relative)?;
    let value: Json = serde_json::from_str(&text)
        .map_err(|error| format!("`{relative}` is not JSON: {error}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| format!("`{relative}` is not a JSON object"))?;
    if let Some(key) = object
        .keys()
        .find(|key| !matches!(key.as_str(), "profile" | "rules"))
    {
        return Err(format!("`{relative}` has the unknown key `{key}`"));
    }
    let registered = profile_id()?;
    match object.get("profile").and_then(Json::as_str) {
        Some(id) if id == registered => {}
        Some(id) => {
            return Err(format!(
                "`{relative}` names profile `{id}`, not `{registered}`"
            ));
        }
        None => return Err(format!("`{relative}` has no string `profile`")),
    }
    let rules = object
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
