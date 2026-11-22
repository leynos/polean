//! The per-job helpers: limits, waiting, abandonment, and systemd's result.
//!
//! These are the parts of one run that are not the orchestration itself: the
//! limit read-back for the canary, the wait-with-abandon path, the diagnostic
//! recorder, and the final `Result=` read. They live here so the orchestration
//! in [`super`] reads as its documented sequence.

use std::{path::Path, process::Child};

use super::{
    super::{
        platform::Programs,
        scope::{LimitReadback, Scope},
    },
    Clock,
    Inputs,
    Readers,
    Supervisor,
    Waiting,
    drain_readers,
    tear_down,
    wait,
};

/// Read the scope's limit files when the job asks for them.
///
/// A read failure is a diagnostic, not an error: the job still has to be torn
/// down, and a missing read-back makes the canary control fail rather than the
/// run vanish.
pub(super) fn read_limits(
    cgroup: Option<&Path>,
    requested: bool,
    diagnostics: &mut Vec<String>,
) -> Option<LimitReadback> {
    if !requested {
        return None;
    }
    let Some(observed) = cgroup else {
        diagnostics.push("the scope's limits were requested but no cgroup was observed".to_owned());
        return None;
    };
    match Scope::read_limits(observed) {
        Ok(limits) => Some(limits),
        Err(reason) => {
            diagnostics.push(reason);
            None
        }
    }
}

/// Tear a job down after a wait failure and return the original diagnostic.
///
/// A `wait` error means the supervisor could not observe the job; the job and
/// its readers must still be released before the error is returned, or the
/// process group and its pipes outlive the run.
pub(super) fn abandon(
    target: &mut Target<'_>,
    readers: &Readers,
    programs: &Programs,
    reason: &str,
) -> String {
    let (report, _status, errors) = tear_down(Inputs {
        child: target.child,
        pgid: target.pgid,
        cgroup: target.cgroup,
        kill: programs.kill.as_path(),
        running: true,
        status: None,
    });
    let mut drain = Vec::new();
    drain_readers(readers, &mut drain);
    format!(
        "{reason}; teardown diagnostics: {errors:?} {drain:?}; survivors {:?}, cgroup empty {}",
        report.survivors, report.cgroup_empty
    )
}

/// The process tree a wait or teardown operates on.
pub(super) struct Target<'a> {
    /// The direct child.
    pub(super) child: &'a mut Child,
    /// The direct child's process-group id.
    pub(super) pgid: u32,
    /// The job's cgroup, when the scope was observed.
    pub(super) cgroup: Option<&'a Path>,
    /// The job's deadline and start.
    pub(super) clock: Clock,
}

impl Supervisor<'_> {
    /// Wait for the job, tearing it down if the wait itself fails.
    pub(super) fn wait_for_job(
        &self,
        target: &mut Target<'_>,
        readers: &Readers,
        snapshot: &mut impl FnMut() -> Option<Vec<i32>>,
    ) -> Result<Waiting, String> {
        let waited = wait(target.child, &readers.trips, target.clock, snapshot);
        match waited {
            Ok(waiting) => Ok(waiting),
            Err(reason) => Err(abandon(target, readers, self.programs, &reason)),
        }
    }
}

/// Record a cleanup error without abandoning the remaining steps.
pub(super) fn note(result: Result<(), String>, diagnostics: &mut Vec<String>) {
    if let Err(reason) = result {
        diagnostics.push(reason);
    }
}

/// Read systemd's final result, recording a query failure.
pub(super) fn read_result(scope: &Scope<'_>, diagnostics: &mut Vec<String>) -> Option<String> {
    match scope.wait_for_result() {
        Ok(result) => result,
        Err(reason) => {
            diagnostics.push(reason);
            None
        }
    }
}

/// Clear the failed state of the exact unit this run created.
///
/// This is a cleanup step, not part of reading the result: a failed unit stays
/// in systemd's failed list, and the next run uses a fresh nonce, but leaving
/// failures behind would accumulate state on the host. Only the exact unit name
/// is named, never a pattern.
pub(super) fn reset_failed_unit(scope: &Scope<'_>, result: Option<&str>) {
    if !matches!(result, None | Some("success")) {
        scope.reset_failed();
    }
}
