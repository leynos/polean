//! Settlement: the only authority that accepts an audit report.
//!
//! [`decision`] holds the pure function and its value types. It is a separate
//! module so `verus/audit_settlement.rs` can pull exactly that directory in
//! through `#[path]` and verify the same source Cargo compiles, without the
//! test fixtures, the wire adapter, or the feature steps in the way.

pub mod decision;
