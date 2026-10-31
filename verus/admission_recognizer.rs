//! The Verus proof entry point for the admission recognizer kernel.
//!
//! The kernel is written once, in `tests/admission/recognizer/kernel.rs`, and
//! pulled in here with `#[path]`. Cargo compiles the same file as test code, so
//! the executed kernel and the verified kernel cannot drift. The specification
//! is `verus/admission_spec.rs` and the proofs are
//! `verus/admission_lemmas.rs`; both are declared inside the kernel module.
//!
//! Run `make verus` after `make install-verus`. The proof is an on-demand
//! target: it is not part of `make all` or the pull-request gate.

#[path = "../tests/admission/recognizer/kernel.rs"]
mod kernel;

fn main() {}