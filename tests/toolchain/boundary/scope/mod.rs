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
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use super::{platform::Programs, process};

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
    /// Returns a diagnostic when `systemctl` cannot be run or fails for a reason
    /// other than the unit being absent.
    pub fn show(&self, property: &str) -> Result<Option<String>, String> {
        self.show_within(property, process::CONTROL_BOUND)
    }

    /// One property of the unit, bounded by `bound`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when `systemctl` cannot be run or does not answer
    /// within `bound`.
    pub fn show_within(&self, property: &str, bound: Duration) -> Result<Option<String>, String> {
        let unit = self.systemd_unit();
        let output = process::run(
            self.programs.systemctl.as_path(),
            &["--user", "show", "-P", property, &unit],
            &[("XDG_RUNTIME_DIR", self.runtime_dir)],
            bound,
        )
        .map_err(|error| format!("cannot query {unit}: {error}"))?;
        if !output.success() {
            // A unit systemd has already garbage-collected is a legitimate
            // absence. Any other non-zero exit is a query failure: treating it
            // as absence would lose the reason and misreport the job.
            return if names_absent(&output.stderr_text(), &unit) {
                Ok(None)
            } else {
                Err(format!(
                    "cannot query {property} of {unit}: systemctl show exited {:?}: {}",
                    output.status.code(),
                    output.stderr_text()
                ))
            };
        }
        let value = output.stdout_text();
        Ok(if value.is_empty() { None } else { Some(value) })
    }

    /// The scope's cgroup, bounded by `bound`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when `systemctl` cannot be run or does not answer
    /// within `bound`.
    pub fn control_group_within(&self, bound: Duration) -> Result<Option<PathBuf>, String> {
        let Some(relative) = self.show_within("ControlGroup", bound)? else {
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

    /// Wait for the scope, bounded by `bound`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the scope does not appear within `bound`, or
    /// when `/proc` cannot be read.
    pub fn wait_until_joined_within(&self, pid: u32, bound: Duration) -> Result<PathBuf, String> {
        let started = Instant::now();
        loop {
            let remaining = bound.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Err(format!(
                    "the scope {} did not become active with the job inside it within {:?}",
                    self.systemd_unit(),
                    bound
                ));
            }
            // Each query gets only what is left of the bound, so successive
            // slow queries cannot push discovery past the job's deadline.
            if let Some(cgroup) = self.active_with(pid, remaining)? {
                return Ok(cgroup);
            }
            thread::sleep(POLL.min(remaining));
        }
    }

    /// Wait for the scope's cgroup to exist, without requiring the original
    /// pid to be in it.
    ///
    /// A late scope may contain only descendants after the direct child has
    /// exited, so membership of the original pid is the wrong test for it.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when a query fails for a reason other than the
    /// scope being absent.
    pub fn wait_until_present_within(&self, bound: Duration) -> Result<Option<PathBuf>, String> {
        let started = Instant::now();
        loop {
            let remaining = bound.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Ok(None);
            }
            if let Some(cgroup) = self.control_group_within(remaining)? {
                return Ok(Some(cgroup));
            }
            thread::sleep(POLL.min(remaining));
        }
    }

    /// The scope's cgroup, but only once `pid` is inside it.
    fn active_with(&self, pid: u32, bound: Duration) -> Result<Option<PathBuf>, String> {
        let Some(cgroup) = self.control_group_within(bound)? else {
            return Ok(None);
        };
        if joined(pid, &self.systemd_unit())? {
            Ok(Some(cgroup))
        } else {
            Ok(None)
        }
    }

    /// The limit files, read while the scope is active.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when a limit file cannot be read.
    pub fn read_limits(cgroup: &Path) -> Result<LimitReadback, String> {
        Ok(LimitReadback {
            memory: read_cgroup(cgroup, "memory.max")?,
            swap: read_cgroup(cgroup, "memory.swap.max")?,
            pids: read_cgroup(cgroup, "pids.max")?,
        })
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
    /// # Errors
    ///
    /// Returns a diagnostic when `systemctl reset-failed` cannot be run or exits
    /// unsuccessfully, so the caller can record it rather than losing it.
    pub fn reset_failed(&self) -> Result<(), String> {
        let output = process::run(
            self.programs.systemctl.as_path(),
            &["--user", "reset-failed", &self.systemd_unit()],
            &[("XDG_RUNTIME_DIR", self.runtime_dir)],
            process::CONTROL_BOUND,
        )
        .map_err(|error| format!("cannot reset {}: {error}", self.systemd_unit()))?;
        if output.success() {
            return Ok(());
        }
        let stderr = output.stderr_text();
        Err(format!(
            "cannot reset {}: systemctl reset-failed exited {:?}: {stderr}",
            self.systemd_unit(),
            output.status.code()
        ))
    }

    /// Kill every process in the scope through systemd.
    ///
    /// This is the bounded whole-unit cleanup for a job whose cgroup path was
    /// never observed: `systemctl kill` reaches every process of the unit
    /// without the supervisor holding the cgroup, so the readers' pipes close
    /// even when the cgroup path is unknown.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the kill cannot be issued or reports failure.
    pub fn kill_unit(&self) -> Result<(), String> {
        let unit = self.systemd_unit();
        let output = process::run(
            self.programs.systemctl.as_path(),
            &[
                "--user",
                "kill",
                "--signal=SIGKILL",
                "--kill-whom=all",
                &unit,
            ],
            &[("XDG_RUNTIME_DIR", self.runtime_dir)],
            process::CONTROL_BOUND,
        )?;
        let detail = output.stderr_text();
        if output.success() || detail.contains("not loaded") {
            // A unit that has gone has no processes left to kill.
            Ok(())
        } else {
            Err(format!("systemctl kill {unit} reported: {detail}"))
        }
    }
}

/// Whether `systemctl`'s stderr is systemd's own "unit is not loaded" message.
///
/// The message must be exactly the one systemd prints for this unit:
/// `Unit <unit> could not be found.` A D-Bus or socket failure that happens to
/// mention both the unit and "could not be found" is still a query failure.
pub(crate) fn names_absent(stderr: &str, unit: &str) -> bool {
    let message = format!("Unit {unit} could not be found");
    stderr.trim().trim_end_matches('.') == message
}

mod cgroup;
pub use self::cgroup::*;
