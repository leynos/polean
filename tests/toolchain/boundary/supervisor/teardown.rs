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
//! Cleanup is best effort *as a whole*: a failure in one step is recorded and
//! the remaining independent steps still run, because abandoning the cgroup kill
//! or the reap because a signaller failed would leave a job alive. The first
//! error is returned after every attempt, and the caller reports it as a
//! containment diagnostic rather than a clean run.
//!
//! A process that outlived the process-group kill is a finding, not a reason to
//! retry: the cgroup kill then removes it. A cgroup that never empties is also
//! reported, not retried.

use std::{
    path::Path,
    process::{Child, ExitStatus},
};

use super::super::scope::{
    EMPTY_BOUND,
    Emptiness,
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
/// Returns the report, the direct child's status when it could be reaped, and
/// every cleanup error in the order they occurred. It never returns early: each
/// independent cleanup step is attempted even when an earlier one failed.
#[expect(
    clippy::needless_pass_by_value,
    reason = "the struct carries the mutable child handle teardown reaps"
)]
pub fn tear_down(inputs: Inputs<'_>) -> (Report, Option<ExitStatus>, Vec<String>) {
    let mut errors = Vec::new();
    if inputs.running {
        super::note(kill_process_group(inputs.kill, inputs.pgid), &mut errors);
    }
    let survivors = inputs
        .cgroup
        .map_or_else(Vec::new, |cgroup| survivors(cgroup, &mut errors));
    if let Some(cgroup) = inputs.cgroup {
        super::note(kill_cgroup(cgroup), &mut errors);
    }
    let status = reap(inputs.status, inputs.child, &mut errors);
    let emptiness = inputs
        .cgroup
        .map_or_else(Emptiness::default, |cgroup| emptiness(cgroup, &mut errors));
    (
        Report {
            survivors,
            cgroup_empty: emptiness.empty,
            residual: emptiness.residual,
            events: emptiness.events,
        },
        status,
        errors,
    )
}

/// Observe who outlived the process-group kill.
fn survivors(cgroup: &Path, errors: &mut Vec<String>) -> Vec<u32> {
    match survivors_after_settle(cgroup, SETTLE_BOUND) {
        Ok(survivors) => survivors,
        Err(reason) => {
            errors.push(reason);
            Vec::new()
        }
    }
}

/// Reap the direct child, recording a failure to do so.
fn reap(
    observed: Option<ExitStatus>,
    child: &mut Child,
    errors: &mut Vec<String>,
) -> Option<ExitStatus> {
    if observed.is_some() {
        return observed;
    }
    match child.wait() {
        Ok(reaped) => Some(reaped),
        Err(error) => {
            errors.push(format!("cannot reap the job: {error}"));
            None
        }
    }
}

/// Confirm the cgroup empty, recording a read failure as not empty.
fn emptiness(cgroup: &Path, errors: &mut Vec<String>) -> Emptiness {
    match wait_until_empty(cgroup, EMPTY_BOUND) {
        Ok(emptiness) => emptiness,
        Err(reason) => {
            errors.push(reason);
            Emptiness::default()
        }
    }
}
