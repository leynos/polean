//! The cgroup files and the process-group signaller.
//!
//! Everything here observes or kills a scope's processes through the files the
//! kernel and systemd expose. Each wait is bounded, and a bound reached is
//! reported with the evidence it observed rather than retried.
//!
//! The process-group kill is a first step, not the authority: a process can
//! leave its group, which is why `cgroup.kill` follows and why the survivors of
//! the group kill are recorded.

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use super::POLL;
use crate::ambient;

/// `ENODEV`: the cgroup was removed while its `cgroup.kill` file was open.
const ENODEV: i32 = 19;

/// Whether `pid`'s cgroup names `unit`.
///
/// # Errors
///
/// Returns a diagnostic when `/proc/<pid>/cgroup` cannot be read.
pub fn joined(pid: u32, unit: &str) -> Result<bool, String> {
    let path = PathBuf::from("/proc").join(pid.to_string()).join("cgroup");
    let text = ambient::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let suffix = format!("/{unit}");
    Ok(text.lines().any(|line| {
        line.rsplit_once("::")
            .is_some_and(|(_hierarchy, cgroup_path)| cgroup_path.ends_with(&suffix))
    }))
}

/// The host pid of `program` in the cgroup, if one is running.
///
/// The process's `argv[0]` is compared by file name against the program the
/// supervisor launched: `systemd-run --scope` keeps the job's own argument
/// vector, whether it runs bare or inside bubblewrap, while bubblewrap's reaper
/// carries bubblewrap's own. A cgroup with no matching process means the job has
/// already exited.
///
/// # Errors
///
/// Returns a diagnostic when the cgroup cannot be listed; an unreadable
/// `/proc/<pid>/cmdline` is skipped because that process has gone.
pub fn job_pid(cgroup: &Path, program: &Path) -> Result<Option<u32>, String> {
    let Some(name) = program.file_name() else {
        return Ok(None);
    };
    for pid in cgroup_procs(cgroup)?.unwrap_or_default() {
        let path = PathBuf::from("/proc").join(pid.to_string()).join("cmdline");
        let Ok(text) = ambient::read_to_string(&path) else {
            continue;
        };
        let argument_zero = text.split('\0').next().unwrap_or_default();
        if Path::new(argument_zero).file_name() == Some(name) {
            return Ok(Some(pid));
        }
    }
    Ok(None)
}

/// Read one file from a cgroup, or `None` when the cgroup has gone.
///
/// # Errors
///
/// Returns a diagnostic for any failure other than the file being absent.
pub fn read_cgroup(cgroup: &Path, name: &str) -> Result<Option<String>, String> {
    let path = cgroup.join(name);
    match ambient::read_to_string(&path) {
        Ok(text) => Ok(Some(text.trim().to_owned())),
        // A cgroup that has gone is reported as absent, whether the directory
        // was already removed (`ENOENT`) or it was removed while the file was
        // open (`ENODEV`).
        Err(error)
            if matches!(error.kind(), ErrorKind::NotFound)
                || error.raw_os_error() == Some(ENODEV) =>
        {
            Ok(None)
        }
        Err(error) => Err(format!("cannot read {}: {error}", path.display())),
    }
}

/// The host process IDs in the cgroup, or `None` when it has gone.
///
/// # Errors
///
/// Returns a diagnostic for an unreadable file or a line that is not a process
/// id. The second is a kernel or format change, not a transient condition, so it
/// fails closed rather than silently reporting an empty group.
pub fn cgroup_procs(cgroup: &Path) -> Result<Option<Vec<u32>>, String> {
    let Some(text) = read_cgroup(cgroup, "cgroup.procs")? else {
        return Ok(None);
    };
    let mut procs = Vec::new();
    for record in text.lines() {
        let entry = record.trim();
        if entry.is_empty() {
            continue;
        }
        let pid = entry
            .parse::<u32>()
            .map_err(|error| format!("cgroup.procs entry {entry:?} is not a pid: {error}"))?;
        procs.push(pid);
    }
    Ok(Some(procs))
}

/// The `populated` count from a `cgroup.events` document.
#[must_use]
pub fn populated(events: &str) -> Option<u32> {
    events.lines().find_map(|line| {
        line.strip_prefix("populated ")
            .and_then(|value| value.trim().parse().ok())
    })
}

/// Kill every process in the cgroup.
///
/// A cgroup that has gone is already empty, so its absence is success.
///
/// # Errors
///
/// Returns a diagnostic when the kill request cannot be written for any other
/// reason.
pub fn kill_cgroup(cgroup: &Path) -> Result<(), String> {
    let path = cgroup.join("cgroup.kill");
    match ambient::write(&path, b"1") {
        Ok(()) => Ok(()),
        // A cgroup that has gone is already empty. `ENOENT` is the directory
        // being removed before the open, and `ENODEV` is the kernel refusing a
        // write to a cgroup that was removed while the file was open.
        Err(error)
            if matches!(error.kind(), ErrorKind::NotFound)
                || error.raw_os_error() == Some(ENODEV) =>
        {
            Ok(())
        }
        Err(error) => Err(format!("cannot write {}: {error}", path.display())),
    }
}

/// Send `SIGKILL` to the process group `pgid`.
///
/// A non-zero exit is treated as benign: the only failure that matters is that
/// the group is not running, and the cgroup kill follows as the authority. A
/// spawn failure is reported because it means the signaller itself is missing.
///
/// # Errors
///
/// Returns a diagnostic when `/usr/bin/kill` cannot be run.
pub fn kill_process_group(kill: &Path, pgid: u32) -> Result<(), String> {
    let target = format!("-{pgid}");
    // A stalled signaller must not hold teardown past the authoritative cgroup
    // kill, so it runs under the control bound and a timeout is reported.
    let output = super::process::run(
        kill,
        &["-s", "KILL", "--", &target],
        &[],
        super::process::CONTROL_BOUND,
    )?;
    if output.success() {
        return Ok(());
    }
    let detail = output.stderr_text();
    if detail.contains("No such process") {
        // The process already exited; there is nothing left to signal.
        return Ok(());
    }
    Err(format!("kill {target} reported: {detail}"))
}

/// What one emptiness wait observed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Emptiness {
    /// Whether the cgroup was empty or gone.
    pub empty: bool,
    /// The processes still present when the bound expired.
    pub residual: Vec<u32>,
    /// The last `cgroup.events` text seen.
    pub events: Option<String>,
}

/// Wait until the cgroup reports itself empty or has gone.
///
/// The returned snapshot names what was still present when the bound expired,
/// so a containment failure carries evidence rather than a bare verdict.
///
/// # Errors
///
/// Returns a diagnostic when a cgroup file cannot be read.
pub fn wait_until_empty(cgroup: &Path, bound: Duration) -> Result<Emptiness, String> {
    let started = Instant::now();
    loop {
        let events = read_cgroup(cgroup, "cgroup.events")?;
        if let Some(empty) = emptiness_of(events.as_deref()) {
            return Ok(empty);
        }
        if started.elapsed() >= bound {
            return Ok(Emptiness {
                empty: false,
                residual: cgroup_procs(cgroup)?.unwrap_or_default(),
                events,
            });
        }
        thread::sleep(POLL);
    }
}

/// The empty snapshot, when the events document already proves emptiness.
///
/// A cgroup that has gone is empty by definition; one whose `populated` count
/// is zero is empty. Anything else needs another poll.
fn emptiness_of(events: Option<&str>) -> Option<Emptiness> {
    match events {
        None => Some(Emptiness {
            empty: true,
            residual: Vec::new(),
            events: None,
        }),
        Some(text) if populated(text) == Some(0) => Some(Emptiness {
            empty: true,
            residual: Vec::new(),
            events: Some(text.to_owned()),
        }),
        Some(_text) => None,
    }
}

/// Watch the cgroup for a settle window and report who is still in it.
///
/// This is the observation that separates "the process-group kill reached every
/// process" from "the cgroup kill was needed". An empty result is the former.
///
/// # Errors
///
/// Returns a diagnostic when a cgroup file cannot be read.
pub fn survivors_after_settle(cgroup: &Path, bound: Duration) -> Result<Vec<u32>, String> {
    let started = Instant::now();
    loop {
        let procs = cgroup_procs(cgroup)?.unwrap_or_default();
        if procs.is_empty() || started.elapsed() >= bound {
            return Ok(procs);
        }
        thread::sleep(POLL);
    }
}
