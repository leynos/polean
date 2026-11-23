//! The fail-closed platform check: five controls, each observed, none assumed.
//!
//! Presence is not enforcement. `systemd-run` accepts limit properties it cannot
//! apply, and a `bwrap` set-up failure looks like an ordinary child failure, so a
//! check that only looked for programs on `PATH` could pass on a host where no
//! isolation exists. Each control therefore reports an observation:
//!
//! 1. the launcher programs exist, are executable, and bubblewrap is at least 0.10.0;
//! 2. the systemd user manager answers through a cleared environment, which is the environment the
//!    supervisor will use;
//! 3. the user manager's cgroup delegates the `memory` and `pids` controllers;
//! 4. the canary, run through the full launch chain, observed the promised isolation from inside;
//!    and
//! 5. while the canary's scope was active, its `memory.max`, `memory.swap.max`, and `pids.max` read
//!    back the requested values.
//!
//! The canary is supplied to [`check`] rather than run here, so the decision is
//! a pure composition of observations. [`CanaryReport`] is what the canary
//! reports; `tests/toolchain/boundary/canary.rs` runs it and parses its output.
//!
//! Missing paths are injectable through [`Programs`], and the canary result is
//! supplied by the caller, so the "starts no job" properties run under
//! `make test` on any host.

use std::path::{Path, PathBuf};

use cap_std::fs::PermissionsExt;

use super::{
    outcome::{Availability, ControlKind, MissingControl, Verdict},
    process,
};
use crate::{ambient, support::path_exists};

mod canary;
mod manager;
pub use self::{canary::*, manager::*};

/// The absolute paths of every program the supervisor launches.
///
/// Nothing is resolved through `PATH`: a cleared environment has no search path,
/// and a launch that named a program by bare name would fail or, worse, find a
/// different program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Programs {
    /// The systemd transient-scope launcher.
    pub systemd_run: PathBuf,
    /// The systemd user-manager client.
    pub systemctl: PathBuf,
    /// The process-group signaller.
    pub kill: PathBuf,
    /// The namespace and filesystem sandbox.
    pub bwrap: PathBuf,
}

impl Programs {
    /// The distribution paths the certified host provides.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            systemd_run: PathBuf::from("/usr/bin/systemd-run"),
            systemctl: PathBuf::from("/usr/bin/systemctl"),
            kill: PathBuf::from("/usr/bin/kill"),
            bwrap: PathBuf::from("/usr/bin/bwrap"),
        }
    }

    /// Every program, in a fixed order, with its role.
    #[must_use]
    pub fn listed(&self) -> [(&'static str, &Path); 4] {
        [
            ("systemd-run", self.systemd_run.as_path()),
            ("systemctl", self.systemctl.as_path()),
            ("kill", self.kill.as_path()),
            ("bwrap", self.bwrap.as_path()),
        ]
    }
}

/// The minimum bubblewrap release whose option set the launch vector uses.
pub const MINIMUM_BWRAP: (u32, u32, u32) = (0, 10, 0);

/// The limits the probe asks systemd to apply, and reads back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestedLimits {
    /// `MemoryMax=`, in bytes.
    pub memory_bytes: u64,
    /// `MemorySwapMax=`, in bytes; the probe always requests zero.
    pub swap_bytes: u64,
    /// `TasksMax=`.
    pub tasks_max: u64,
}

impl Default for RequestedLimits {
    fn default() -> Self {
        Self {
            memory_bytes: 64 * 1024 * 1024,
            swap_bytes: 0,
            tasks_max: 32,
        }
    }
}

impl RequestedLimits {
    /// The values systemd must report back, in `memory.max`, `memory.swap.max`,
    /// and `pids.max` order.
    #[must_use]
    pub fn expected_readback(&self) -> [String; 3] {
        [
            self.memory_bytes.to_string(),
            self.swap_bytes.to_string(),
            self.tasks_max.to_string(),
        ]
    }
}

/// What the canary observed from inside the boundary.
///
/// A field is `None` when the canary never reported it, which is a missing
/// control rather than an assumed value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CanaryReport {
    /// Whether the canary printed its start line with the run's nonce.
    pub started: bool,
    /// The `NoNewPrivs` value from `/proc/self/status`.
    pub no_new_privs: Option<String>,
    /// The `CapEff` value from `/proc/self/status`.
    pub cap_eff: Option<String>,
    /// The network interfaces the canary could see.
    pub interfaces: Vec<String>,
    /// The error an outbound TCP connection produced.
    pub outbound_error: Option<String>,
    /// The inherited descriptors the canary could see.
    pub descriptors: Option<Vec<i32>>,
    /// Whether the job's network namespace differed from the supervisor's.
    pub separate_netns: Option<bool>,
    /// `memory.max` read from the active scope.
    pub memory_max: Option<String>,
    /// `memory.swap.max` read from the active scope.
    pub memory_swap_max: Option<String>,
    /// `pids.max` read from the active scope.
    pub pids_max: Option<String>,
}

/// The inputs to one platform check.
pub struct Check<'a> {
    /// The launcher programs, injectable so a test can make one absent.
    pub programs: &'a Programs,
    /// The directory systemd's user manager is reached through.
    pub runtime_dir: &'a Path,
    /// The canary's report, or the reason it could not run.
    pub canary: Result<&'a CanaryReport, String>,
    /// The limits the canary's scope requested.
    pub limits: &'a RequestedLimits,
}

/// The result of one platform check: one availability per control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformReport {
    /// Every control and whether it held.
    pub controls: Vec<(ControlKind, Availability)>,
}

impl PlatformReport {
    /// The fail-closed verdict.
    #[must_use]
    pub fn verdict(&self) -> Verdict { super::outcome::verdict(&self.controls) }

    /// The missing controls, empty when the platform is ready.
    #[must_use]
    pub fn missing(&self) -> Vec<MissingControl> {
        match self.verdict() {
            Verdict::Ready => Vec::new(),
            Verdict::Unavailable(missing) => missing,
            // A malformed list accounts for no control, so every control is
            // reported as unaccounted for.
            Verdict::Malformed(fault) => ControlKind::ALL
                .iter()
                .map(|kind| MissingControl {
                    kind: *kind,
                    reason: fault.clone(),
                })
                .collect(),
        }
    }
}

/// Run every control that can be observed without starting a job.
///
/// The canary is supplied by the caller: it is a job, so it cannot be part of a
/// check that must start no job. A canary that did not run marks both canary
/// controls missing, naming the reason.
#[must_use]
pub fn check(inputs: &Check<'_>) -> PlatformReport {
    let programs = check_programs(inputs.programs);
    let manager = check_user_manager(inputs.programs.systemctl.as_path(), inputs.runtime_dir);
    let delegation = check_delegation();
    let (isolation, limits) = match &inputs.canary {
        Ok(report) => evaluate_canary(report, inputs.limits),
        Err(reason) => {
            let missing = Availability::Missing(format!("the canary did not run: {reason}"));
            (missing.clone(), missing)
        }
    };
    PlatformReport {
        controls: vec![
            (ControlKind::Programs, programs),
            (ControlKind::UserManager, manager),
            (ControlKind::Delegation, delegation),
            (ControlKind::CanaryIsolation, isolation),
            (ControlKind::CanaryLimits, limits),
        ],
    }
}

/// Control 1: every program exists, is executable, and bubblewrap is current.
#[must_use]
pub fn check_programs(programs: &Programs) -> Availability {
    for (role, path) in programs.listed() {
        match executable(path) {
            Ok(true) => (),
            Ok(false) => {
                return Availability::Missing(format!(
                    "{role} at {} is not executable",
                    path.display()
                ));
            }
            Err(reason) => {
                return Availability::Missing(format!("{role} at {}: {reason}", path.display()));
            }
        }
    }
    check_bwrap_version(programs.bwrap.as_path())
}

/// Whether `path` exists and carries an execute bit.
///
/// # Errors
///
/// Returns the inspection error when the path cannot be examined for any reason
/// other than absence, because "could not look" is not "not installed".
pub fn executable(path: &Path) -> Result<bool, String> {
    if !path_exists(path)? {
        return Ok(false);
    }
    let metadata = ambient::metadata(path)
        .map_err(|error| format!("cannot inspect the permissions: {error}"))?;
    if !metadata.is_file() {
        return Ok(false);
    }
    Ok(metadata.permissions().mode() & 0o111 != 0)
}

/// Control 1, part two: bubblewrap reports at least [`MINIMUM_BWRAP`].
fn check_bwrap_version(bwrap: &Path) -> Availability {
    let output = match process::run(bwrap, &["--version"], &[], process::CONTROL_BOUND) {
        Ok(output) => output,
        Err(error) => {
            return Availability::Missing(format!("cannot run {}: {error}", bwrap.display()));
        }
    };
    if !output.success() {
        return Availability::Missing(format!(
            "{} --version exited {:?}",
            bwrap.display(),
            output.status.code()
        ));
    }
    let text = output.stdout_text();
    match parse_bwrap_version(&text) {
        Some(version) if version >= MINIMUM_BWRAP => Availability::Available,
        Some(version) => Availability::Missing(format!(
            "bubblewrap {}.{}.{} is older than the required {}.{}.{}",
            version.0, version.1, version.2, MINIMUM_BWRAP.0, MINIMUM_BWRAP.1, MINIMUM_BWRAP.2
        )),
        None => Availability::Missing(format!(
            "{} --version did not report `bubblewrap <major>.<minor>.<patch>`: {text:?}",
            bwrap.display()
        )),
    }
}

/// Parse the release from `bubblewrap <major>.<minor>.<patch>`.
///
/// A trailing suffix, such as a distribution's `-5.el10_2`, is ignored: the
/// upstream release is what the option set depends on.
#[must_use]
pub fn parse_bwrap_version(text: &str) -> Option<(u32, u32, u32)> {
    let rest = text.trim().strip_prefix("bubblewrap ")?;
    let release = rest.split(['-', '+', ' ']).next()?;
    let mut parts = release.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}
