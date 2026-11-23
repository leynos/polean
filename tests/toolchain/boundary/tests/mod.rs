//! Unit tests for the boundary's pure kernels, decisions, and contracts.
//!
//! The two decisions are enumerated completely: five bound flags, four exit
//! forms, two control-group states, and two platform states for `classify`; and
//! five controls with two states each for `verdict`. Enumerating a finite domain
//! is complete, so a named case cannot hide behind a sampling argument.
//!
//! The platform tests inject missing program paths and canary reports, so they
//! start no job and hold on any host. The one exception is `check`, which also
//! runs the real `systemctl --user is-system-running` when a test calls it;
//! that test asserts only on the controls it injected.

mod canary;
mod hostile;
mod outcome;
mod platform;
mod probe;
mod reader;
mod submit;
mod support;
mod wait;
