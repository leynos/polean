//! The test-only execution boundary for roadmap 1.2.4.
//!
//! Technical design §11 requires every external program Polean runs to work
//! offline, inside memory and output limits, in private storage, with
//! cancellation killing and reaping every process the job started. This tree is
//! the bounded probe that decides whether that is enforceable, and it is
//! test-only: nothing under `src/` is involved, and roadmap 4.3.1 promotes the
//! supervisor into the product.
//!
//! The pieces are:
//!
//! - [`capture`]: the bounded stream reader both pipes go through, and the one limit the supervisor
//!   enforces itself. Verus proves it over every finite chunk sequence
//!   (`verus/boundary_capture.rs`).
//! - [`outcome`]: the probe's outcome categories and the fail-closed platform verdict, each decided
//!   by enumerating a finite domain.
//! - [`platform`]: the five enforcement controls, each an observation rather than a presence check.
//! - [`launch`]: the systemd-scope and bubblewrap argument vectors, as data, with the properties
//!   they must keep.
//! - [`hostile`]: the child entry point the probes re-execute, selected by an argument token so an
//!   accidental `--ignored` run is inert.
//!
//! EP-M1 establishes these pure pieces. The supervisor, the scopes, and the
//! probes that run hostile children arrive in EP-M2 and EP-M3, so nothing here
//! starts a process except the platform check's own `--version` and
//! `is-system-running` observations.
//!
//! A passing probe will say nothing about admission, the model proof,
//! translation, or runtime correspondence, the four claims in `AGENTS.md`. It
//! concerns only the environment in which those claims are later checked.

pub(crate) mod canary;
pub(crate) mod capture;
pub(crate) mod hostile;
pub(crate) mod launch;
pub(crate) mod outcome;
pub(crate) mod platform;
pub(crate) mod probe;
pub(crate) mod process;
pub(crate) mod scope;
pub(crate) mod supervisor;

#[cfg(test)]
mod tests;
