//! Toolchain observations for the pinned Rust, Regorus, Lean, and OPA tuple.
//!
//! One integration binary holds every module so that the denied-`allow` lint
//! policy does not turn a module used by only one target into `dead_code`, and
//! so that Regorus is linked once.
//!
//! The Regorus observations run under `make test` on any host. The provisioned
//! OPA and Lean observations live in [`smoke`] behind `#[ignore]`, because they
//! require `make tools`.

mod ambient;
mod bdd;
mod engine;
mod json;
mod regorus;
mod scratch;
mod smoke;
mod support;
