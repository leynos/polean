//! The audit contracts, the wire encoding, and the pure settlement function.
//!
//! Three layers meet at one JSON report, and this module keeps them apart:
//!
//! - [`wire`] decodes the request and report contracts strictly and turns each Lean name and
//!   statement into an injective byte key. It is the only layer that interprets JSON.
//! - [`settlement::decision`] decides a report against a request using those keys. It is pure: no
//!   `serde`, no I/O, and no Lean. It is also the source `verus/audit_settlement.rs` verifies, so
//!   it holds no dependency that a proof cannot see.
//! - [`fixtures`] builds the request and report pairs the unit and property tests share, so one
//!   case names the same bytes everywhere. The feature scenarios run the real worker instead and
//!   live in `tests/toolchain/smoke/audit/bdd/`.
//!
//! The split matters for assurance. A proof over settlement says acceptance
//! follows from a truthful report; it says nothing about whether the report is
//! truthful, and nothing about whether this decoder read it faithfully. The
//! strict-decoding tests and the provisioned corpus carry those claims, and
//! neither is a substitute for the other.

mod fixtures;
mod properties;
// The smoke corpus settles real worker output, so it consumes the same wire
// adapter and settlement function the unit tests do.
pub(crate) mod settlement;
mod tests;
pub(crate) mod wire;
