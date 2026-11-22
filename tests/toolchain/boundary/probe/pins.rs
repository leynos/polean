//! The pinned tool locations, with the remediation a missing install needs.
//!
//! The pins are the single authority for each tool's identity; this module only
//! turns a tool name into its provisioned path and fails closed when the tree is
//! not installed.

use std::path::{Path, PathBuf};

use serde_json::Value as Json;

use crate::{
    ambient,
    json::parse_json,
    support::{lean_root, opa_path, path_exists, pins_path},
};

/// Read a tool's installed digest from the committed pin file.
///
/// # Errors
///
/// Returns a diagnostic when the pin file or the entry is missing.
pub(super) fn pinned_sha(tool: &str) -> Result<String, String> {
    let text = ambient::read_to_string(&pins_path())
        .map_err(|error| format!("cannot read the pin file: {error}; run `make tools`"))?;
    let document = parse_json(&text, "toolchain-pins.json")?;
    document
        .get(tool)
        .and_then(|entry| entry.get("sha256"))
        .and_then(Json::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("toolchain-pins.json `{tool}.sha256` is missing"))
}

/// Require a provisioned path, with the remediation `make tools` gives.
///
/// # Errors
///
/// Returns a diagnostic naming the path and the target that provisions it.
pub(super) fn require_present(path: &Path) -> Result<(), String> {
    if path_exists(path)? {
        return Ok(());
    }
    Err(format!(
        "{} is missing; run `make tools` to provision the pinned tools",
        path.display()
    ))
}

/// The provisioned OPA binary.
///
/// # Errors
///
/// Returns a diagnostic when the pin or the binary is missing.
pub(super) fn pinned_opa() -> Result<PathBuf, String> {
    let path = opa_path(&pinned_sha("opa")?);
    require_present(&path)?;
    Ok(path)
}

/// The provisioned Lean root.
///
/// # Errors
///
/// Returns a diagnostic when the pin or the root is missing.
pub(super) fn pinned_lean() -> Result<PathBuf, String> {
    let root = lean_root(&pinned_sha("lean")?);
    require_present(&root)?;
    Ok(root)
}
