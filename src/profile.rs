//! The profile constants shared by ingestion, the IR, and their tests.
//!
//! Every value here has an authoritative source in the profile contract,
//! `contracts/profile.json`. The tests in `tests/ingestion/contracts.rs` read
//! that file and fail if a constant here drifts from it, so this module is a
//! checked restatement rather than an independent authority.
//!
//! The profile identifier is immutable. A different profile is a different
//! syntactic and semantic contract, not a version bump of this one.

/// The profile identifier this crate implements.
///
/// The value is `contracts/profile.json`'s `id`, and the IR carries it in every
/// canonical encoding.
pub const PROFILE_ID: &str = "polean.tenant-write.v0";

/// The largest policy source accepted, in bytes.
///
/// The bound is inclusive: a source of exactly this many bytes is accepted. It
/// restates `contracts/profile.json`'s `limits.source_bytes`.
pub const SOURCE_BYTES_MAX: usize = 65_536;

/// The largest request document accepted, in bytes.
///
/// The bound is inclusive and restates `contracts/profile.json`'s
/// `limits.input_bytes`. Roadmap 2.1.3 uses it through
/// [`crate::bounded::read_bounded`]; nothing here reads requests yet.
pub const INPUT_BYTES_MAX: usize = 65_536;

/// The most rules a policy may hold.
///
/// Restates `contracts/profile.json`'s `rules.max`. A policy may hold zero
/// rules, which is the valid deny-all form.
pub const RULES_MAX: usize = 16;

/// The fewest atoms a rule body may hold.
///
/// Restates `contracts/profile.json`'s `rules.body_min`. An empty body is not
/// representable in [`crate::ir`].
pub const BODY_ATOMS_MIN: usize = 1;

/// The most atoms a rule body may hold.
///
/// Restates `contracts/profile.json`'s `rules.body_max`.
pub const BODY_ATOMS_MAX: usize = 16;
