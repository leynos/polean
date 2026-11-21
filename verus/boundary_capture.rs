//! Verus entry point for the capture proof.
//!
//! The verified source is the same `boundary/capture/` directory Cargo compiles
//! into the toolchain test binary, reached through `#[path]`. There is no second
//! copy of the kernel, so a change to the executable code either keeps the proof
//! true or breaks it.

#[path = "../tests/toolchain/boundary/capture/mod.rs"]
mod capture;

fn main() {}