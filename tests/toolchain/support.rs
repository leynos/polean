//! Shared inputs and paths for the toolchain observations.
//!
//! The baseline and the deny-all mutation are read from the repository's own
//! example fixtures, so the tests observe the same bytes that the design
//! harness validates. The three further requests are written out here in
//! full: they are smoke inputs, not profile fixtures, and adding them to
//! `examples/` would widen the design pack the Python harness owns.
//!
//! The path helpers build from `CARGO_MANIFEST_DIR` rather than the process
//! working directory, so the provisioned smoke tests work from any directory.

use std::path::{Path, PathBuf};

/// The repository root, as recorded by Cargo at compile time.
const REPOSITORY_ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// The path to the committed pin file.
#[must_use]
pub fn pins_path() -> PathBuf { repository_path("toolchain-pins.json") }

/// The digest-named OPA binary, from the pin file's digest.
#[must_use]
pub fn opa_path(sha256: &str) -> PathBuf {
    repository_path(".polean/tools/opa")
        .join(sha256)
        .join("opa")
}

/// The digest-named Lean root, from the pin file's digest.
#[must_use]
pub fn lean_root(sha256: &str) -> PathBuf { repository_path(".polean/tools/lean").join(sha256) }

/// A path under the repository root, joined as a relative POSIX path.
#[must_use]
pub fn repository_path(relative: &str) -> PathBuf {
    let mut path = Path::new(REPOSITORY_ROOT).to_path_buf();
    for component in relative.split('/') {
        path.push(component);
    }
    path
}

/// Baseline source as a repository path, for tools that read the file.
#[must_use]
pub fn baseline_source_path() -> PathBuf { repository_path("examples/tenant-write/policy.rego") }

/// Baseline policy source, the admitted reference module.
pub const BASELINE_SOURCE: &str = include_str!("../../examples/tenant-write/policy.rego");

/// Deny-all mutation source, which keeps no non-default rule.
pub const DENY_ALL_SOURCE: &str =
    include_str!("../../examples/tenant-write/mutations/deny-all/policy.rego");

/// Owner-unlocked write fixture request: `T W ¬A O ¬L`. Expected admission.
pub const OWNER_UNLOCKED_REQUEST: &str = include_str!("../../examples/tenant-write/request.json");

/// Cross-tenant administrator write: `¬T W A ¬O ¬L`. Expected denial.
///
/// This is the request on which the E0 mutation diverges from the baseline.
pub const CROSS_TENANT_ADMIN_REQUEST: &str = r#"{
  "subject": {
    "tenant": "tenant-b",
    "id": "subject-b",
    "role": "admin"
  },
  "resource": {
    "tenant": "tenant-a",
    "owner": "subject-a",
    "locked": false
  },
  "action": "write"
}"#;

/// Same-tenant administrator write on a locked resource: `T W A ¬O L`.
/// Expected admission.
pub const SAME_TENANT_ADMIN_LOCKED_REQUEST: &str = r#"{
  "subject": {
    "tenant": "tenant-a",
    "id": "subject-a",
    "role": "admin"
  },
  "resource": {
    "tenant": "tenant-a",
    "owner": "subject-b",
    "locked": true
  },
  "action": "write"
}"#;

/// Owner read: `T ¬W ¬A O ¬L`. Expected denial.
pub const OWNER_READ_REQUEST: &str = r#"{
  "subject": {
    "tenant": "tenant-a",
    "id": "subject-a",
    "role": "member"
  },
  "resource": {
    "tenant": "tenant-a",
    "owner": "subject-a",
    "locked": false
  },
  "action": "read"
}"#;
