//! Verus entry point for the settlement proof.
//!
//! The verified source is the same `settlement/decision/` directory Cargo
//! compiles into the toolchain test binary, reached through `#[path]`. There is
//! no second copy of the decision logic, so a change to the executable code
//! either keeps the proof true or breaks it.

#[path = "../tests/toolchain/audit/settlement/decision/mod.rs"]
mod decision;

fn main() {}
