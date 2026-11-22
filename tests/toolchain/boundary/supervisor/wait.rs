//! The wait loop: one deadline, the stream trips, and the job's descriptors.
//!
//! The loop polls the child, the trip channel, and the deadline in one place, so
//! the deadline is the only clock. It also snapshots the job's descriptor table
//! as soon as the job process is running, because that observation is only
//! available while the job is alive.
//!
//! # Deadline attribution
//!
//! A poll that begins after the deadline and finds the child still running
//! records `Bound::Deadline` and stops the job. A poll that finds the child has
//! already exited reports the exit, even when the deadline has passed: the
//! observation cannot tell whether the exit preceded expiry, and attributing a
//! deadline to an exit that may have completed normally would misreport the
//! job. The policy is therefore "the deadline kills a job still running at a
//! poll", and an exit observed at a poll is an exit.

use std::{
    path::Path,
    process::{Child, ExitStatus},
    sync::mpsc::Receiver,
    thread,
};

use super::{
    super::{
        outcome::{Bound, Bounds},
        scope::{POLL, job_pid},
    },
    Boundary,
    Clock,
    Job,
};
use crate::ambient;

/// What one wait loop observed.
pub(super) struct Waiting {
    /// The status `try_wait` observed, if the child exited on its own.
    pub(super) status: Option<ExitStatus>,
    /// The bounds that fired.
    pub(super) bounds: Bounds,
    /// Whether the child was still running when the loop stopped.
    pub(super) running: bool,
    /// The job's descriptors, once observed.
    pub(super) fds: Option<Vec<i32>>,
}

/// Poll the child, its stream trips, and the deadline until one stops it.
pub(super) fn wait(
    child: &mut Child,
    trips: &Receiver<Bound>,
    clock: Clock,
    snapshot: &mut impl FnMut() -> Option<Vec<i32>>,
) -> Result<Waiting, String> {
    let mut evidence = Evidence {
        bounds: Bounds::none(),
        fds: None,
    };
    loop {
        if evidence.fds.is_none() {
            evidence.fds = snapshot();
        }
        if let Some(stopped) = stop_reason(child, trips, clock, &mut evidence)? {
            return Ok(stopped);
        }
        thread::sleep(POLL);
    }
}

/// The evidence the wait loop has accumulated.
struct Evidence {
    /// The bounds that fired.
    bounds: Bounds,
    /// The job's descriptors, once observed.
    fds: Option<Vec<i32>>,
}

/// Why the wait loop should stop, if it should, and with what evidence.
///
/// The child's own exit is checked first, and any trip queued just before it is
/// drained into the bounds: a stream that crossed its cap just as the child
/// finished must not report completion.
fn stop_reason(
    child: &mut Child,
    trips: &Receiver<Bound>,
    clock: Clock,
    evidence: &mut Evidence,
) -> Result<Option<Waiting>, String> {
    match child.try_wait() {
        Ok(Some(status)) => {
            while let Ok(bound) = trips.try_recv() {
                evidence.bounds.insert(bound);
            }
            return Ok(Some(Waiting {
                status: Some(status),
                bounds: evidence.bounds,
                running: false,
                fds: evidence.fds.clone(),
            }));
        }
        Ok(None) => (),
        Err(error) => return Err(format!("cannot wait for the job: {error}")),
    }
    if let Ok(bound) = trips.try_recv() {
        evidence.bounds.insert(bound);
        return Ok(Some(Waiting {
            status: None,
            bounds: evidence.bounds,
            running: true,
            fds: evidence.fds.clone(),
        }));
    }
    if clock.expired() {
        evidence.bounds.insert(Bound::Deadline);
        return Ok(Some(Waiting {
            status: None,
            bounds: evidence.bounds,
            running: true,
            fds: evidence.fds.clone(),
        }));
    }
    Ok(None)
}

/// Snapshot the job's descriptors once the job process is running.
///
/// Under `Unconfined` the direct child *is* the job, so its own descriptor
/// table is the answer once its command line matches. Under `Bubblewrap` the
/// job is a different host process, found through the cgroup by its argument
/// vector; the process listing is of that process's descriptor directory, so the
/// supervisor's own transient handles are not part of it.
pub(super) fn snapshot_fds(job: &Job, pgid: u32, cgroup: Option<&Path>) -> Option<Vec<i32>> {
    let process = match job.boundary {
        Boundary::Unconfined => process_matches(pgid, &job.spec.host_program).then_some(pgid),
        Boundary::Bubblewrap => cgroup.and_then(|scope_cgroup| {
            job_pid(scope_cgroup, Path::new(&job.spec.sandbox_program))
                .ok()
                .flatten()
        }),
    };
    process.and_then(|found| ambient::open_fds(found).ok())
}

/// Whether `pid`'s command line names `program`.
fn process_matches(pid: u32, program: &Path) -> bool {
    let path = std::path::PathBuf::from("/proc")
        .join(pid.to_string())
        .join("cmdline");
    let Ok(text) = ambient::read_to_string(&path) else {
        return false;
    };
    let argument_zero = text.split('\0').next().unwrap_or_default();
    match (Path::new(argument_zero).file_name(), program.file_name()) {
        (Some(actual), Some(expected)) => actual == expected,
        _ => false,
    }
}
