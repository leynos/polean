//! The teardown every job runs, whether it completed, failed, or was killed.
//!
//! The order matters and is fixed:
//!
//! 1. if the direct child has not exited, `SIGKILL` its process group;
//! 2. watch the cgroup for a settle window and record who outlived that kill;
//! 3. write `1` to the cgroup's `cgroup.kill`, the authoritative whole-tree kill;
//! 4. reap the direct child;
//! 5. confirm the cgroup empty, within a bound.
//!
//! A process that outlived the process-group kill is a finding, not a reason to
//! retry: the cgroup kill then removes it, and the caller reports a containment
//! failure. A cgroup that never empties is also reported, not retried.

use std::{
    path::Path,
    process::{Child, ExitStatus},
};

use super::super::scope::{
    EMPTY_BOUND,
    SETTLE_BOUND,
    kill_cgroup,
    kill_process_group,
    survivors_after_settle,
    wait_until_empty,
};

/// What teardown observed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Processes that outlived the process-group kill, as host process IDs.
    pub survivors: Vec<u32>,
    /// Whether the cgroup was empty, or gone, after the cgroup kill.
    pub cgroup_empty: bool,
    /// The processes still in the cgroup when the emptiness bound expired.
    pub residual: Vec<u32>,
    /// The last `cgroup.events` text seen when the emptiness bound expired.
    pub events: Option<String>,
}

/// The inputs to one teardown.
pub struct Inputs<'a> {
    /// The direct child, which teardown reaps.
    pub child: &'a mut Child,
    /// The direct child's process-group id, which equals its pid.
    pub pgid: u32,
    /// The job's cgroup, absent when its scope never became active.
    pub cgroup: Option<&'a Path>,
    /// The process-group signaller.
    pub kill: &'a Path,
    /// Whether the direct child was still running when the wait loop stopped.
    pub running: bool,
    /// The status already observed by `try_wait`, if any.
    pub status: Option<ExitStatus>,
}

/// Tear one job down and report what was observed.
///
/// # Errors
///
/// Returns a diagnostic when the signaller cannot be run, the child cannot be
/// reaped, or a cgroup file cannot be read.
#[expect(
    clippy::needless_pass_by_value,
    reason = "the struct carries the mutable child handle teardown reaps"
)]
pub fn tear_down(inputs: Inputs<'_>) -> Result<(Report, ExitStatus), String> {
    if inputs.running {
        kill_process_group(inputs.kill, inputs.pgid)?;
    }
    let survivors = match inputs.cgroup {
        Some(cgroup) => survivors_after_settle(cgroup, SETTLE_BOUND)?,
        None => Vec::new(),
    };
    if let Some(cgroup) = inputs.cgroup {
        kill_cgroup(cgroup)?;
    }
    let status = match inputs.status {
        Some(status) => status,
        None => inputs
            .child
            .wait()
            .map_err(|error| format!("cannot reap the job: {error}"))?,
    };
    let emptiness = match inputs.cgroup {
        Some(cgroup) => wait_until_empty(cgroup, EMPTY_BOUND)?,
        // A scope that never became active cannot be confirmed empty, so it is
        // reported as not empty rather than assumed clean.
        None => super::super::scope::Emptiness::default(),
    };
    Ok((
        Report {
            survivors,
            cgroup_empty: emptiness.empty,
            residual: emptiness.residual,
            events: emptiness.events,
        },
        status,
    ))
}
