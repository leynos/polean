//! Shared inputs for the toolchain observations.
//!
//! The baseline and the deny-all mutation are read from the repository's own
//! example fixtures, so the tests observe the same bytes that the design
//! harness validates. The three further requests are written out here in
//! full: they are smoke inputs, not profile fixtures, and adding them to
//! `examples/` would widen the design pack the Python harness owns.

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

/// Parse JSON text, naming the source when it is malformed.
///
/// # Errors
///
/// Returns a diagnostic when `text` is not valid JSON.
pub fn parse_json(text: &str, source: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str(text).map_err(|error| format!("{source} is not valid JSON: {error}"))
}
