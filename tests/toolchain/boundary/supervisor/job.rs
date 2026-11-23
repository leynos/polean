//! One job's request and its finished record.
//!
//! These are the values the supervisor and its probes exchange. They live here
//! so the supervisor module keeps its orchestration within the size rule.

use std::time::Duration;

use super::{
    super::{
        launch::LaunchSpec,
        outcome::{Bounds, ExitForm, Outcome},
        scope::LimitReadback,
    },
    Boundary,
};

/// One job for the supervisor to run.
pub struct Job {
    /// The launch specification, validated before anything starts.
    pub spec: LaunchSpec,
    /// Which launch chain to use.
    pub boundary: Boundary,
    /// The cap on standard output, which carries the structured result.
    pub stdout_cap: usize,
    /// The cap on standard error, which carries diagnostics.
    pub stderr_cap: usize,
    /// The single deadline for the whole job.
    pub deadline: Duration,
    /// Whether to read the scope's limit files while it is active.
    pub read_limits: bool,
    /// A cancellation request the wait loop observes, when a caller supplies
    /// one. The request stops the job and is recorded as `Bound::Cancelled`.
    pub cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// Whether the scope join is required evidence for this job.
    ///
    /// Hostile jobs require it: the probe's claim is that the job ran inside
    /// the scope. A pinned tool can finish in milliseconds, before the first
    /// successful poll, and systemd garbage-collects a successful scope at
    /// once; the tool probes therefore rely on the unit's recorded success and
    /// on the canary's proof of the same launch chain, and this flag records
    /// that choice rather than hiding it.
    pub require_scope: bool,
}

/// One finished job.
#[derive(Debug)]
pub struct Run {
    /// The probe's outcome category.
    pub outcome: Outcome,
    /// The kept prefix of standard output.
    pub stdout: Vec<u8>,
    /// The kept prefix of standard error.
    pub stderr: Vec<u8>,
    /// How the direct child ended.
    pub exit: ExitForm,
    /// Every bound that fired.
    pub bounds: Bounds,
    /// Processes that outlived the process-group kill.
    pub survivors: Vec<u32>,
    /// Whether the job's cgroup was empty after teardown.
    pub cgroup_empty: bool,
    /// The processes still in the cgroup when the emptiness bound expired.
    pub residual: Vec<u32>,
    /// The last `cgroup.events` text seen, for a containment diagnostic.
    pub events: Option<String>,
    /// systemd's `Result=` for the scope, when it recorded one.
    pub result: Option<String>,
    /// The job's open descriptors while it was alive, when they were observed.
    pub fds: Option<Vec<i32>>,
    /// The scope's limit files, when the job asked for them.
    pub limits: Option<LimitReadback>,
    /// Observations that make the run fail closed, in the order they occurred.
    pub diagnostics: Vec<String>,
    /// Observations recorded without making the run fail closed.
    pub notes: Vec<String>,
}
