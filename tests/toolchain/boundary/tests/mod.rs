//! Unit tests for the boundary's pure kernels, decisions, and contracts.
//!
//! The two decisions are enumerated completely: five bound flags, four exit
//! forms, two control-group states, and two platform states for `classify`; and
//! five controls with two states each for `verdict`. Enumerating a finite domain
//! is complete, so a named case cannot hide behind a sampling argument.
//!
//! The platform tests inject an `Observation` and a canary report, so they start
//! no process and hold on any host. The host observation lives in
//! `platform::observe` and is exercised by the boundary probe, not here.

mod canary;
mod hostile;
mod outcome;
mod platform;
mod probe;
mod reader;
mod readiness;
mod reset;
mod submit;
mod support;
mod wait;
