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
    path::{Path, PathBuf},
    time::Duration,
};

use cap_std::fs::MetadataExt;

mod unconfined;

use super::{
    hostile::{self, Behaviour},
    launch::{LaunchSpec, ReadOnlyBind},
    platform::Programs,
    supervisor::{Boundary, Job, Run, Supervisor},
};
use crate::{ambient, scratch::Scratch};

/// The path the test binary is bound at inside the boundary.
pub const PROBE_BINARY: &str = "/job/bin/probe";

/// The path the input snapshot is bound at inside the boundary.
pub const PROBE_INPUTS: &str = "/job/in";

/// What one probe run asks for.
pub struct Options {
    /// Which launch chain to use.
    pub boundary: Boundary,
    /// `MemoryMax=`, in bytes.
    pub memory_bytes: u64,
    /// The private scratch area, in bytes.
    pub scratch_bytes: u64,
    /// The job's deadline.
    pub deadline: Duration,
    /// The standard-output cap.
    pub stdout_cap: usize,
    /// The standard-error cap.
    pub stderr_cap: usize,
    /// The target tokens passed to the child.
    pub targets: Vec<String>,
}

impl Options {
    /// The default for one boundary: 64 MiB, 8 MiB scratch, a 10 s deadline, and
    /// the product's 1 MiB/64 KiB stream caps.
    #[must_use]
    pub fn new(boundary: Boundary) -> Self {
        Self {
            boundary,
            memory_bytes: 64 * 1024 * 1024,
            scratch_bytes: 8 * 1024 * 1024,
            deadline: Duration::from_secs(10),
            stdout_cap: 1024 * 1024,
            stderr_cap: 64 * 1024,
            targets: Vec::new(),
        }
    }

    /// Set the memory limit.
    #[must_use]
    pub fn memory(mut self, bytes: u64) -> Self {
        self.memory_bytes = bytes;
        self
    }

    /// Set the scratch area.
    #[must_use]
    pub fn scratch(mut self, bytes: u64) -> Self {
        self.scratch_bytes = bytes;
        self
    }

    /// Set the deadline.
    #[must_use]
    pub fn deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    /// Set the standard-output cap.
    #[must_use]
    pub fn stdout_cap(mut self, cap: usize) -> Self {
        self.stdout_cap = cap;
        self
    }

    /// Add one target token.
    #[must_use]
    pub fn target(mut self, token: impl Into<String>) -> Self {
        self.targets.push(token.into());
        self
    }
}

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

    /// Run one hostile behaviour under the selected boundary.
    ///
    /// # Errors
    ///
    /// Returns the supervisor's diagnostic when the job could not be observed.
    pub fn run(&self, behaviour: Behaviour, options: &Options) -> Result<Run, String> {
        let supervisor = Supervisor {
            programs: &self.programs,
            runtime_dir: &self.runtime_dir,
        };
        supervisor.run(&Job {
            spec: self.spec(behaviour, options),
            boundary: options.boundary,
            stdout_cap: options.stdout_cap,
            stderr_cap: options.stderr_cap,
            deadline: options.deadline,
        })
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
