//! The transient systemd user scope: limits, whole-tree kill, and its record.
//!
//! `systemd-run --scope` runs the command itself, so the supervisor's direct
//! child is the job. The scope gives that job a cgroup with `MemoryMax=`,
//! `MemorySwapMax=`, `TasksMax=`, a whole-group out-of-memory kill, a backstop
//! deadline, and `cgroup.kill` as the authoritative whole-tree kill.
//!
//! # Timing
//!
//! `systemd-run` joins its scope a moment after it starts, and systemd records
//! `Result=` only after the processes die. Every wait here is bounded, and a
//! bound reached is reported rather than retried forever: the supervisor turns a
//! scope that never appeared into a containment failure, because a job that was
//! never confirmed inside its cgroup cannot be confirmed cleaned up either.
//!
//! # What is read, and why
//!
//! `systemctl --user show -P <property>` is the only source for the scope's
//! cgroup path and its `Result=`. The limit files under `/sys/fs/cgroup` are the
//! only evidence that systemd applied the properties it accepted, which is what
//! the canary control reads back.

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

use super::platform::Programs;
use crate::ambient;

/// How often a scope's state is polled.
pub const POLL: Duration = Duration::from_millis(20);

/// How long a scope may take to appear with the job inside it.
pub const JOIN_BOUND: Duration = Duration::from_secs(5);

/// How long a killed job's cgroup may take to report itself empty.
pub const EMPTY_BOUND: Duration = Duration::from_secs(5);

/// How long the cgroup is watched for survivors of a process-group kill.
pub const SETTLE_BOUND: Duration = Duration::from_millis(500);

/// How long systemd may take to record the scope's `Result=`.
pub const RESULT_BOUND: Duration = Duration::from_secs(5);

/// The three limit values a canary's scope reports while it is active.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LimitReadback {
    /// `memory.max`, verbatim.
    pub memory: Option<String>,
    /// `memory.swap.max`, verbatim.
    pub swap: Option<String>,
    /// `pids.max`, verbatim.
    pub pids: Option<String>,
}

/// One transient scope the supervisor created.
pub struct Scope<'a> {
    /// The launcher programs, so `systemctl` is never resolved through `PATH`.
    pub programs: &'a Programs,
    /// The directory the user manager is reached through.
    pub runtime_dir: &'a Path,
    /// The bare unit name, without `.scope`.
    pub unit: String,
}

impl Scope<'_> {
    /// The unit name with its type suffix.
    #[must_use]
    pub fn systemd_unit(&self) -> String { format!("{}.scope", self.unit) }

    /// One property of the unit, or `None` when systemd does not know the unit.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when `systemctl` cannot be run.
    pub fn show(&self, property: &str) -> Result<Option<String>, String> {
        let output = Command::new(self.programs.systemctl.as_path())
            .args(["--user", "show", "-P", property, &self.systemd_unit()])
            .env_clear()
            .env("XDG_RUNTIME_DIR", self.runtime_dir)
            .output()
            .map_err(|error| format!("cannot run systemctl: {error}"))?;
        if !output.status.success() {
            return Ok(None);
        }
        let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        Ok(if value.is_empty() { None } else { Some(value) })
    }

    /// The scope's cgroup, as an absolute path under `/sys/fs/cgroup`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when `systemctl` cannot be run.
    pub fn control_group(&self) -> Result<Option<PathBuf>, String> {
        let Some(relative) = self.show("ControlGroup")? else {
            return Ok(None);
        };
        if relative.is_empty() {
            return Ok(None);
        }
        let mut path = PathBuf::from("/sys/fs/cgroup");
        for component in relative.split('/').filter(|part| !part.is_empty()) {
            path.push(component);
        }
        Ok(Some(path))
    }

    /// Wait until the scope exists and `pid` is inside it.
    ///
    /// Both conditions are required: systemd may create the cgroup before
    /// `systemd-run` moves into it, and a job that never joined would not be
    /// bounded by the limits the probe claims to have applied.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the scope does not appear within
    /// [`JOIN_BOUND`], or when `/proc` cannot be read.
    pub fn wait_until_joined(&self, pid: u32) -> Result<PathBuf, String> {
        let started = Instant::now();
        loop {
            if let Some(cgroup) = self.active_with(pid)? {
                return Ok(cgroup);
            }
            if started.elapsed() >= JOIN_BOUND {
                return Err(format!(
                    "the scope {} did not become active with the job inside it within {:?}",
                    self.systemd_unit(),
                    JOIN_BOUND
                ));
            }
            thread::sleep(POLL);
        }
    }

    /// The scope's cgroup, but only once `pid` is inside it.
    fn active_with(&self, pid: u32) -> Result<Option<PathBuf>, String> {
        let Some(cgroup) = self.control_group()? else {
            return Ok(None);
        };
        if joined(pid, &self.systemd_unit())? {
            Ok(Some(cgroup))
        } else {
            Ok(None)
        }
    }

    /// Wait until systemd records the scope's final state, then read `Result=`.
    ///
    /// A scope that succeeded is garbage-collected, so `None` means either "no
    /// result was recorded" or "the unit has gone"; both are the same for the
    /// only decision this feeds, the out-of-memory bound.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when `systemctl` cannot be run.
    pub fn wait_for_result(&self) -> Result<Option<String>, String> {
        let started = Instant::now();
        loop {
            let active = self.show("ActiveState")?;
            if matches!(active.as_deref(), None | Some("failed" | "inactive")) {
                return self.show("Result");
            }
            if started.elapsed() >= RESULT_BOUND {
                return self.show("Result");
            }
            thread::sleep(POLL);
        }
    }

    /// Reset this exact unit if systemd kept it as failed.
    ///
    /// A successful scope is garbage-collected and needs no reset. The reset is
    /// best effort because a unit that has already gone needs nothing, and a
    /// reset failure leaves a failed unit that the next run's per-run nonce
    /// cannot collide with. Only the exact unit name this run created is named,
    /// never a pattern.
    pub fn reset_failed(&self) {
        if let Ok(output) = Command::new(self.programs.systemctl.as_path())
            .args(["--user", "reset-failed", &self.systemd_unit()])
            .env_clear()
            .env("XDG_RUNTIME_DIR", self.runtime_dir)
            .output()
        {
            drop(output);
        }
    }
}

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

/// Read one file from a cgroup, or `None` when the cgroup has gone.
///
/// # Errors
///
/// Returns a diagnostic for any failure other than the file being absent.
pub fn read_cgroup(cgroup: &Path, name: &str) -> Result<Option<String>, String> {
    let path = cgroup.join(name);
    match ambient::read_to_string(&path) {
        Ok(text) => Ok(Some(text.trim().to_owned())),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
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
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
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
    Command::new(kill)
        .args(["-s", "KILL", "--", &target])
        .env_clear()
        .output()
        .map(drop)
        .map_err(|error| format!("cannot run {}: {error}", kill.display()))
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
