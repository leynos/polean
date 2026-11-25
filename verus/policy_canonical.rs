//! The Verus proof entry point for the canonical policy encoding.
//!
//! The verified source is the production kernel, `src/ir/model/`, reached
//! through `#[path]`. There is no second copy of the encoder, so a change to
//! the executable code either keeps the proof true or breaks it. `Cargo`
//! compiles the same file as library code, so the shipped encoder and the
//! verified encoder cannot drift.

#[path = "../src/ir/model/mod.rs"]
mod model;

fn main() {}
