//! The options a probe run and a tool job carry.
//!
//! `Options` is the one place a probe says how large the job is, how long it
//! may run, and what its streams are capped at. `ToolJob` describes a pinned
//! program that is not one of the hostile behaviours.

use std::{path::Path, time::Duration};

use super::super::{launch::ReadOnlyBind, supervisor::Boundary};

/// What one probe run asks for.
pub struct Options {
    /// Which launch chain to use.
    pub boundary: Boundary,
    /// `MemoryMax=`, in bytes.
    pub memory_bytes: u64,
    /// The private scratch area, in bytes.
    pub scratch_bytes: u64,
    /// The job's deadline.
    pub deadline: Duration,
    /// The standard-output cap.
    pub stdout_cap: usize,
    /// The standard-error cap.
    pub stderr_cap: usize,
    /// Whether the supervisor reads the scope's limits while it is active.
    pub read_limits: bool,
    /// The target tokens passed to the child.
    pub targets: Vec<String>,
}

impl Options {
    /// The default for one boundary: 64 MiB, 8 MiB scratch, a 10 s deadline, and
    /// the product's 1 MiB/64 KiB stream caps.
    #[must_use]
    pub fn new(boundary: Boundary) -> Self {
        Self {
            boundary,
            memory_bytes: 64 * 1024 * 1024,
            scratch_bytes: 8 * 1024 * 1024,
            deadline: Duration::from_secs(10),
            stdout_cap: 1024 * 1024,
            stderr_cap: 64 * 1024,
            read_limits: false,
            targets: Vec::new(),
        }
    }

    /// Set the memory limit.
    #[must_use]
    pub fn memory(mut self, bytes: u64) -> Self {
        self.memory_bytes = bytes;
        self
    }

    /// Set the scratch area.
    #[must_use]
    pub fn scratch(mut self, bytes: u64) -> Self {
        self.scratch_bytes = bytes;
        self
    }

    /// Set the deadline.
    #[must_use]
    pub fn deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    /// Set the standard-output cap.
    #[must_use]
    pub fn stdout_cap(mut self, cap: usize) -> Self {
        self.stdout_cap = cap;
        self
    }

    /// Ask the supervisor to read the scope's limits.
    #[must_use]
    pub fn read_limits(mut self) -> Self {
        self.read_limits = true;
        self
    }

    /// Add one target token.
    #[must_use]
    pub fn target(mut self, token: impl Into<String>) -> Self {
        self.targets.push(token.into());
        self
    }

    /// Add every target token.
    #[must_use]
    pub fn targets(mut self, tokens: Vec<String>) -> Self {
        self.targets.extend(tokens);
        self
    }
}

/// A program that is not one of the hostile behaviours.
pub struct ToolJob<'a> {
    /// The absolute host path of the program.
    pub host_program: &'a Path,
    /// Its fixed path inside the boundary.
    pub sandbox_program: &'a str,
    /// Its arguments.
    pub args: Vec<String>,
    /// Extra read-only binds the program needs.
    pub binds: Vec<ReadOnlyBind>,
    /// The environment entries it gets.
    pub env: Vec<(String, String)>,
}
