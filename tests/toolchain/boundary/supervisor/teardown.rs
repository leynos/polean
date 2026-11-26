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
    path::{Path, PathBuf},
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
    /// The whole-tree handle used when no cgroup was observed.
    pub late: &'a dyn LateScope,
    /// Whether the job required the scope observation. A tool job may finish
    /// before its scope appears, so it does not wait for one in teardown.
    pub require_scope: bool,
}

/// The whole-tree operations teardown needs when discovery failed.
///
/// Injecting them lets a test simulate a late scope with an exited original
/// pid, which a real systemd manager cannot be made to produce on demand.
pub trait LateScope {
    /// Kill every process of the unit.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the unit kill fails.
    fn kill_unit(&self) -> Result<(), String>;
    /// Wait for the scope's cgroup to exist, bounded by `bound`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the query fails.
    fn wait_for_cgroup(&self, bound: std::time::Duration) -> Result<Option<PathBuf>, String>;
}

impl LateScope for super::Scope<'_> {
    fn kill_unit(&self) -> Result<(), String> { super::Scope::kill_unit(self) }

    fn wait_for_cgroup(&self, bound: std::time::Duration) -> Result<Option<PathBuf>, String> {
        self.wait_until_present_within(bound)
    }
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
    let (survivors, emptiness) = if let Some(cgroup) = inputs.cgroup {
        clean_cgroup(cgroup, &mut errors)
    } else if inputs.require_scope {
        // Without a cgroup the unit is the only whole-tree handle. A scope can
        // still be created after the discovery failure's earlier kill, so wait
        // briefly for it and clean it like any observed cgroup; a scope that
        // never appears is reported, because the fallback direct-child kill
        // cannot be described as a whole-tree kill.
        match kill_late_scope(inputs.late, &mut errors) {
            Ok(Some(cgroup)) => clean_cgroup(&cgroup, &mut errors),
            Ok(None) => (Vec::new(), Emptiness::default()),
            Err(reason) => {
                errors.push(reason);
                (Vec::new(), Emptiness::default())
            }
        }
    } else {
        // A tool job may finish before its scope is created. Its containment
        // evidence is the unit's recorded success and the canary (D21), so the
        // unit kill is attempted without waiting for a scope that may never
        // exist.
        super::note(inputs.late.kill_unit(), &mut errors);
        (Vec::new(), Emptiness::default())
    };
    let status = reap(inputs.status, inputs.running, inputs.child, &mut errors);
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

/// Kill a scope that may have appeared after the discovery failure.
///
/// The wait is bounded by [`super::super::scope::JOIN_BOUND`]: a scope that
/// appears is killed and confirmed empty; one that never appears is reported,
/// so the run cannot claim a whole-tree kill it did not perform.
fn kill_late_scope(
    late: &dyn LateScope,
    errors: &mut Vec<String>,
) -> Result<Option<PathBuf>, String> {
    // Kill the unit first: `systemctl kill` succeeds for a unit that is not
    // loaded, and catches a scope the manager has already created.
    super::note(late.kill_unit(), errors);
    match late.wait_for_cgroup(super::super::scope::JOIN_BOUND) {
        Ok(Some(cgroup)) => Ok(Some(cgroup)),
        Ok(None) => Err(
            "no cgroup was observed and no scope appeared, so the whole-tree kill is unconfirmed"
                .to_owned(),
        ),
        Err(reason) => Err(format!(
            "the scope query failed, so the whole-tree kill is unconfirmed: {reason}"
        )),
    }
}

/// Record who survived, kill the cgroup, and confirm it empty.
fn clean_cgroup(cgroup: &Path, errors: &mut Vec<String>) -> (Vec<u32>, Emptiness) {
    let survivors = survivors(cgroup, errors);
    super::note(kill_cgroup(cgroup), errors);
    let emptiness = emptiness(cgroup, errors);
    (survivors, emptiness)
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
    running: bool,
    child: &mut Child,
    errors: &mut Vec<String>,
) -> Option<ExitStatus> {
    if observed.is_some() {
        return observed;
    }
    // A fallback kill of the direct child guarantees the reap returns, even
    // when the signaller failed and no cgroup kill was possible. It does not
    // replace the group or cgroup kill; it only keeps an unbounded wait from
    // hiding the diagnostics collected so far.
    if running {
        drop(child.kill());
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
