//! The probe harness: build one job, run it, and name its evidence.
//!
//! A probe is a test under `make boundary-probe` that runs a hostile child
//! through [`Supervisor`] and asserts on what the child and the supervisor
//! observed. This module holds the parts every probe shares: the scratch
//! directory that owns the run's paths, the nonce that names its unit and every
//! report line, the re-executed test binary, and the launch specification.
//!
//! The child is the test binary itself. It is bound as one file at
//! [`PROBE_BINARY`] and selected by libtest's exact filter, so no repository
//! binary target is needed, which roadmap 2.1.4 owns.
//!
//! The probes run with one test thread (`make boundary-probe` passes
//! `--test-threads=1`), because each job's unit name and cgroup are per-run
//! state and the host is shared with other agents.

use std::{
    io,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use cap_std::fs::MetadataExt;

mod bdd;
mod bubblewrap;
mod listeners;
mod pins;
mod run;
mod tools;
mod unconfined;

use super::{
    canary,
    hostile::{self, Behaviour},
    launch::{LaunchSpec, ReadOnlyBind},
    outcome::Verdict,
    platform::{CanaryReport, PlatformReport, Programs, RequestedLimits},
    supervisor::{Boundary, Run},
};
use crate::{ambient, scratch::Scratch};

/// The path the test binary is bound at inside the boundary.
pub const PROBE_BINARY: &str = "/job/bin/probe";

/// The path the input snapshot is bound at inside the boundary.
pub const PROBE_INPUTS: &str = "/job/in";
mod options;
pub use self::options::{Options, ToolJob};

/// One probe run's owned paths and identity.
pub struct Probe {
    /// The scratch directory, cleaned when the probe is dropped.
    scratch: Scratch,
    /// The run's nonce.
    nonce: String,
    /// The launcher programs.
    programs: Programs,
    /// The directory the user manager is reached through.
    runtime_dir: PathBuf,
    /// The re-executed test binary.
    binary: PathBuf,
    /// The empty input snapshot directory.
    inputs: PathBuf,
}

impl Probe {
    /// Create a probe with its own scratch directory and nonce.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the scratch directory cannot be created, the
    /// test binary's path cannot be found, or the user ID cannot be read.
    pub fn new(label: &str) -> Result<Self, String> {
        let scratch =
            Scratch::try_new(label).map_err(|error| format!("cannot create scratch: {error}"))?;
        let inputs = scratch.path().join("inputs");
        ambient::create_dir(&inputs).map_err(|error| {
            format!(
                "cannot create the input snapshot {}: {error}",
                inputs.display()
            )
        })?;
        let nonce = scratch
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .ok_or_else(|| "the scratch directory has no usable name".to_owned())?;
        let binary = std::env::current_exe()
            .map_err(|error| format!("cannot find the test binary: {error}"))?;
        let uid = ambient::metadata(scratch.path())
            .map_err(|error| format!("cannot inspect the scratch directory: {error}"))?
            .uid();
        Ok(Self {
            scratch,
            nonce,
            programs: Programs::standard(),
            runtime_dir: PathBuf::from(format!("/run/user/{uid}")),
            binary,
            inputs,
        })
    }

    /// The run's nonce.
    #[must_use]
    pub fn nonce(&self) -> &str { &self.nonce }

    /// The run's scratch directory.
    #[must_use]
    pub fn scratch_path(&self) -> &Path { self.scratch.path() }

    /// The input snapshot directory, which the probe writes before a job.
    #[must_use]
    pub fn inputs(&self) -> &Path { &self.inputs }

    /// The standard programs.
    #[must_use]
    pub fn programs(&self) -> &Programs { &self.programs }

    /// The runtime directory the scope is reached through.
    #[must_use]
    pub fn runtime_dir(&self) -> &Path { &self.runtime_dir }

    /// Build the launch specification for one behaviour.
    #[must_use]
    pub fn spec(&self, behaviour: Behaviour, options: &Options) -> LaunchSpec {
        LaunchSpec {
            host_program: self.binary.clone(),
            sandbox_program: PROBE_BINARY.to_owned(),
            args: self.arguments(behaviour, &options.targets),
            binds: vec![
                ReadOnlyBind {
                    source: self.binary.clone(),
                    target: PROBE_BINARY.to_owned(),
                },
                ReadOnlyBind {
                    source: self.inputs.clone(),
                    target: PROBE_INPUTS.to_owned(),
                },
            ],
            scratch_bytes: options.scratch_bytes,
            env: Vec::new(),
            unit: format!("polean-probe-{}", self.nonce),
            memory_bytes: options.memory_bytes,
            swap_bytes: 0,
            tasks_max: 32,
            runtime_secs: options.deadline.as_secs() + 5,
        }
    }

    /// Run the canary under the selected boundary and parse its report.
    ///
    /// The supervisor reads the active scope's limits, so the report carries
    /// both what the child observed and what systemd actually applied. The
    /// descriptor set is the supervisor's observation of the job's own table.
    ///
    /// # Errors
    ///
    /// Returns the supervisor's diagnostic when the job could not be observed.
    pub fn canary(&self, limits: &RequestedLimits) -> Result<(Run, CanaryReport), String> {
        // The canary's outbound attempt targets this listener, so the probe
        // owns every endpoint it can reach: a failed isolation can only reach
        // the probe itself.
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("cannot bind the canary listener: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("cannot make the canary listener non-blocking: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("cannot read the canary listener address: {error}"))?;
        let options = Options::new(Boundary::Bubblewrap)
            .memory(limits.memory_bytes)
            .deadline(Duration::from_secs(10))
            .read_limits()
            .target(format!("polean-target=tcp:{address}"))
            .target(format!("polean-target=marker:{}", self.nonce));
        let run = self.run(Behaviour::Canary, &options)?;
        // A connection that arrived here is a failed isolation observation; the
        // canary reports it as `connected`, and this listener corroborates it.
        if listener.accept().is_ok() {
            drop(listener);
            return Err("the canary reached the probe's listener".to_owned());
        }
        drop(listener);
        let readback = run.limits.clone().unwrap_or_default();
        let mut report = canary::parse(
            &String::from_utf8_lossy(&run.stdout),
            &self.nonce,
            &readback,
        );
        report.descriptors = run.fds.clone();
        report.separate_netns = match (&run.netns, netns_identity()) {
            (Some(job), Some(supervisor)) => Some(job != &supervisor),
            _ => None,
        };
        Ok((run, report))
    }

    /// The libtest and hostile arguments the child receives.
    fn arguments(&self, behaviour: Behaviour, targets: &[String]) -> Vec<String> {
        let mut args = vec![
            "--exact".to_owned(),
            "--ignored".to_owned(),
            "--test-threads=1".to_owned(),
            "--nocapture".to_owned(),
            "-q".to_owned(),
            hostile::entry_name(),
            format!("polean-hostile={}:{}", behaviour.as_str(), self.nonce),
        ];
        args.extend(targets.iter().cloned());
        args
    }
}

/// The supervisor's own network-namespace identity.
fn netns_identity() -> Option<std::path::PathBuf> {
    ambient::link_target(Path::new("/proc/self/ns/net")).ok()
}

/// Run a canary on its own probe and require a ready platform report.
///
/// One canary per probe test costs about a second, and it means a selected
/// job is submitted only after this host's controls were observed, not after a
/// check some other test ran.
///
/// # Errors
///
/// Returns a diagnostic when the canary cannot run or a control is missing.
pub(super) fn ready_report(label: &str) -> Result<PlatformReport, String> {
    let probe = Probe::new(label)?;
    let limits = RequestedLimits::default();
    let observation = super::platform::observe(probe.programs(), probe.runtime_dir());
    let report =
        super::platform::readiness(&observation, &limits, || match probe.canary(&limits) {
            Ok((run, report)) => {
                super::supervisor::require_outcome(&run, &super::outcome::Outcome::Completed)?;
                Ok(report)
            }
            Err(reason) => Err(reason),
        });
    if report.verdict() != Verdict::Ready {
        return Err(format!("the platform is not ready: {:?}", report.verdict()));
    }
    Ok(report)
}

/// Require the child's start line, then return its output as text.
///
/// # Errors
///
/// Returns a diagnostic when the start line is absent, so a child that never ran
/// cannot pass.
pub fn started(run: &Run, probe: &Probe) -> Result<String, String> {
    let stdout = String::from_utf8_lossy(&run.stdout).into_owned();
    let expected = super::canary::start_line(probe.nonce());
    if !stdout.lines().any(|line| line == expected) {
        return Err(format!(
            "the child never reported {:?}; stdout was {stdout:?}, stderr {:?}, outcome {:?}",
            expected,
            String::from_utf8_lossy(&run.stderr),
            run.outcome
        ));
    }
    Ok(stdout)
}

/// Require one attempt line and return its observation.
///
/// # Errors
///
/// Returns a diagnostic quoting the output when the line is absent.
pub fn attempt<'a>(stdout: &'a str, name: &str) -> Result<&'a str, String> {
    let prefix = format!("polean-hostile attempt {name} ");
    let line = stdout
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(str::trim)
        .ok_or_else(|| {
            format!("the child never reported an {name} attempt; stdout was {stdout:?}")
        })?;
    Ok(line)
}

/// Require an attempt line for `name` whose detail contains `needle`.
///
/// A behaviour that reports both a start marker and a result is checked for
/// the specific line the property needs, so a later line cannot stand in for
/// an earlier one.
///
/// # Errors
///
/// Returns a diagnostic when no matching attempt was reported.
pub fn attempt_containing<'a>(
    stdout: &'a str,
    name: &str,
    needle: &str,
) -> Result<&'a str, String> {
    let prefix = format!("polean-hostile attempt {name} ");
    stdout
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix))
        .find(|detail| detail.contains(needle))
        .ok_or_else(|| {
            format!("the child never reported an {name} attempt containing {needle:?};                      stdout was {stdout:?}")
        })
}

/// How long a killed process's `/proc` entry may take to disappear.
const GONE_BOUND: Duration = Duration::from_secs(2);

/// How often a disappearing process is polled.
const GONE_POLL: Duration = Duration::from_millis(20);

/// Wait until a process's `/proc` entry has gone.
///
/// A process the boundary killed must not merely be signalled: a zombie that
/// nobody reaps still holds its entry. The supervisor confirms the cgroup empty
/// before this runs, so the entry disappears when the kernel releases it.
///
/// # Errors
///
/// Returns a diagnostic when the entry is still present at the bound, or when it
/// cannot be inspected for any reason other than absence.
pub fn wait_until_gone(pid: u32) -> Result<(), String> {
    let path = PathBuf::from(format!("/proc/{pid}"));
    let started = Instant::now();
    loop {
        if !present(&path)? {
            return Ok(());
        }
        if started.elapsed() >= GONE_BOUND {
            return Err(format!(
                "the process {pid} is still present after {GONE_BOUND:?}"
            ));
        }
        thread::sleep(GONE_POLL);
    }
}

/// Whether `path` exists, reporting an inspection failure other than absence.
fn present(path: &Path) -> Result<bool, String> {
    match ambient::metadata(path) {
        Ok(_metadata) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("cannot inspect {}: {error}", path.display())),
    }
}
