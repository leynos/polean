//! The job supervisor: spawn, bounded capture, and unconditional teardown.
//!
//! One job is one `systemd-run --scope` invocation, optionally around
//! bubblewrap. The supervisor:
//!
//! 1. spawns the launch vector with a cleared environment, a new process group, standard input from
//!    `/dev/null`, and both streams piped;
//! 2. waits, within a bound, until the scope exists with the job inside it, and records the job's
//!    open descriptors while it is alive;
//! 3. drains each stream on its own thread through its own [`Capture`], so a flood is bounded while
//!    it happens rather than after;
//! 4. polls the child against one deadline and stops it when a stream trips;
//! 5. tears the job down through [`teardown`], which kills the process group, records any survivor,
//!    writes `cgroup.kill`, reaps, and confirms the cgroup empty.
//!
//! Every job is torn down, not only the killed ones. A successful scope is
//! garbage-collected, and a grandchild that left the process group would
//! otherwise hold a pipe and block the readers forever.
//!
//! # Diagnostics are fail-closed
//!
//! A scope that was never observed, a cleanup step that failed, or a reader
//! that never delivered its bytes means containment cannot be confirmed. Each is
//! recorded as a diagnostic and the outcome is a containment failure, whatever
//! else happened. A job never passes on the strength of an observation the
//! supervisor could not make.
//!
//! # Bounded reader completion
//!
//! Readers deliver their bytes over a channel, which the supervisor waits on
//! with a bound. The ordinary path joins the reader thread. If authoritative
//! cleanup could not close the pipes — a descendant that both left the process
//! group and escaped the cgroup kill — the bounded wait reports a diagnostic
//! instead of hanging forever; that is a last resort after the kill attempts,
//! not a substitute for joining.
//!
//! The supervisor spawns and reaps on one thread because `--die-with-parent`
//! fires when the spawning thread exits.

use std::{
    os::unix::process::{CommandExt, ExitStatusExt},
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

use super::{
    launch::{LaunchSpec, bubblewrap_scope_argv, unconfined_scope_argv},
    outcome::{Bound, Bounds, ExitForm, Observation, Outcome, classify},
    platform::Programs,
    scope::{LimitReadback, Scope},
};

mod reader;
mod run;
mod teardown;
mod wait;

use self::{
    reader::{Readers, drain_readers, finish_readers, readers},
    run::{Target, note, read_limits, read_result},
    teardown::{Inputs, tear_down},
    wait::{Waiting, snapshot_fds, wait},
};

/// Which launch chain a job uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    /// The scope alone: the negative control.
    Unconfined,
    /// The scope around bubblewrap: the selected boundary.
    Bubblewrap,
}

/// One job for the supervisor to run.
pub struct Job {
    /// The launch specification, validated before anything starts.
    pub spec: LaunchSpec,
    /// Which launch chain to use.
    pub boundary: Boundary,
    /// The cap on standard output, which carries the structured result.
    pub stdout_cap: usize,
    /// The cap on standard error, which carries diagnostics.
    pub stderr_cap: usize,
    /// The single deadline for the whole job.
    pub deadline: Duration,
    /// Whether to read the scope's limit files while it is active.
    pub read_limits: bool,
    /// Whether the scope join is required evidence for this job.
    ///
    /// Hostile jobs require it: the probe's claim is that the job ran inside
    /// the scope. A pinned tool can finish in milliseconds, before the first
    /// successful poll, and systemd garbage-collects a successful scope at
    /// once; the tool probes therefore rely on the unit's recorded success and
    /// on the canary's proof of the same launch chain, and this flag records
    /// that choice rather than hiding it.
    pub require_scope: bool,
}

/// One finished job.
#[derive(Debug)]
pub struct Run {
    /// The probe's outcome category.
    pub outcome: Outcome,
    /// The kept prefix of standard output.
    pub stdout: Vec<u8>,
    /// The kept prefix of standard error.
    pub stderr: Vec<u8>,
    /// How the direct child ended.
    pub exit: ExitForm,
    /// Every bound that fired.
    pub bounds: Bounds,
    /// Processes that outlived the process-group kill.
    pub survivors: Vec<u32>,
    /// Whether the job's cgroup was empty after teardown.
    pub cgroup_empty: bool,
    /// The processes still in the cgroup when the emptiness bound expired.
    pub residual: Vec<u32>,
    /// The last `cgroup.events` text seen, for a containment diagnostic.
    pub events: Option<String>,
    /// systemd's `Result=` for the scope, when it recorded one.
    pub result: Option<String>,
    /// The job's open descriptors while it was alive, when they were observed.
    pub fds: Option<Vec<i32>>,
    /// The scope's limit files, when the job asked for them.
    pub limits: Option<LimitReadback>,
    /// Observations that make the run fail closed, in the order they occurred.
    pub diagnostics: Vec<String>,
    /// Observations recorded without making the run fail closed.
    pub notes: Vec<String>,
}

/// Runs jobs through the selected boundary.
pub struct Supervisor<'a> {
    /// The launcher programs.
    pub programs: &'a Programs,
    /// The directory the user manager is reached through.
    pub runtime_dir: &'a Path,
}

/// One started job's state.
struct Started<'a> {
    /// The direct child.
    child: Child,
    /// The direct child's process-group id.
    pgid: u32,
    /// The job's scope.
    scope: Scope<'a>,
    /// The job's cgroup, when the scope was observed.
    cgroup: Option<std::path::PathBuf>,
    /// Why the scope was not observed, when it was not.
    join_error: Option<String>,
    /// Observations that already make the run fail closed.
    diagnostics: Vec<String>,
}

/// The job's single deadline, and when it started.
#[derive(Clone, Copy)]
struct Clock {
    /// The deadline from spawn.
    deadline: Duration,
    /// When the job was spawned.
    started: Instant,
}

impl Clock {
    /// Whether the deadline has expired.
    fn expired(self) -> bool { self.started.elapsed() >= self.deadline }

    /// The budget left before the deadline, zero once it has passed.
    fn remaining(self) -> Duration { self.deadline.saturating_sub(self.started.elapsed()) }
}

impl Supervisor<'_> {
    /// Run one job to completion, teardown included.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the job's specification is invalid, the launch
    /// vector cannot be spawned, or a stream reader cannot be started. A job
    /// that fails its bounds, escapes containment, or cannot be fully observed
    /// is a [`Run`] with diagnostics, not an error.
    #[expect(
        clippy::too_many_lines,
        reason = "the orchestration is one documented sequence; each step is a named helper"
    )]
    pub fn run(&self, job: &Job) -> Result<Run, String> {
        job.spec.validate()?;
        let clock = Clock {
            deadline: job.deadline,
            started: Instant::now(),
        };
        let Started {
            mut child,
            pgid,
            scope,
            cgroup,
            join_error,
            mut diagnostics,
        } = self.start(job, clock)?;
        let mut notes = Vec::new();
        if let Some(reason) = join_error {
            if job.require_scope {
                diagnostics.push(reason);
            } else {
                // A tool job that finished before its first poll records the
                // unobserved scope without failing closed: its containment
                // evidence is the unit's success and the canary (D21).
                notes.push(format!("the scope was not observed: {reason}"));
            }
        }
        let limits = read_limits(cgroup.as_deref(), job.read_limits, &mut diagnostics);
        let readers = readers(&mut child, job)?;
        let mut snapshot = || snapshot_fds(job, pgid, cgroup.as_deref());
        let waiting = {
            let mut target = Target {
                child: &mut child,
                pgid,
                cgroup: cgroup.as_deref(),
                clock,
            };
            self.wait_for_job(&mut target, &readers, &mut snapshot)?
        };
        let (report, status, errors) = tear_down(Inputs {
            child: &mut child,
            pgid,
            cgroup: cgroup.as_deref(),
            kill: self.programs.kill.as_path(),
            running: waiting.running,
            status: waiting.status,
        });
        diagnostics.extend(errors);
        let (stdout, stderr, mut bounds) =
            finish_readers(readers, waiting.bounds, &mut diagnostics);
        let result = read_result(&scope, &mut diagnostics);
        if result.as_deref() == Some("oom-kill") {
            bounds.insert(Bound::Memory);
        }
        let exit = exit_form(status);
        // An observation the supervisor could not make is never a pass, except
        // for a tool job that never required the scope observation: its
        // containment evidence is the unit's recorded success and the canary.
        let cgroup_empty = if job.require_scope || cgroup.is_some() {
            report.cgroup_empty && diagnostics.is_empty()
        } else {
            diagnostics.is_empty()
        };
        let outcome = classify(&Observation {
            bounds,
            exit,
            cgroup_empty,
            survivors: !report.survivors.is_empty(),
            unavailable: Vec::new(),
        });
        Ok(Run {
            outcome,
            stdout,
            stderr,
            exit,
            bounds,
            survivors: report.survivors,
            cgroup_empty,
            residual: report.residual,
            events: report.events,
            result,
            fds: waiting.fds,
            limits,
            diagnostics,
            notes,
        })
    }

    /// Spawn the job and wait for its scope, recording a failed observation.
    fn start<'a>(&'a self, job: &Job, clock: Clock) -> Result<Started<'a>, String> {
        let argv = match job.boundary {
            Boundary::Bubblewrap => bubblewrap_scope_argv(&job.spec, self.programs),
            Boundary::Unconfined => unconfined_scope_argv(&job.spec),
        };
        let child = spawn(self.programs, self.runtime_dir, &argv)?;
        let pgid = child.id();
        let scope = Scope {
            programs: self.programs,
            runtime_dir: self.runtime_dir,
            unit: job.spec.unit.clone(),
        };
        // Scope discovery shares the job's single deadline: a job whose budget
        // expires while systemd is being queried is torn down rather than given
        // a fresh bound.
        let discovery = clock.remaining().min(crate::boundary::scope::JOIN_BOUND);
        let (cgroup, join_error) = match scope.wait_until_joined_within(pgid, discovery) {
            Ok(cgroup) => (Some(cgroup), None),
            Err(reason) => (None, Some(reason)),
        };
        let mut diagnostics = Vec::new();
        if cgroup.is_none() {
            // The cgroup path is unknown, so the unit is the only handle that
            // can still reach every process of the job, whatever its process
            // group. `kill_unit` treats an already-unloaded unit as success.
            note(scope.kill_unit(), &mut diagnostics);
        }
        Ok(Started {
            child,
            pgid,
            scope,
            cgroup,
            join_error,
            diagnostics,
        })
    }
}

/// Spawn the launch vector as the supervisor's direct child.
///
/// The child gets its own process group so the process-group kill reaches the
/// job and not the test process, and `XDG_RUNTIME_DIR` is the only environment
/// entry, because `systemd-run` needs it and nothing else.
fn spawn(programs: &Programs, runtime_dir: &Path, argv: &[String]) -> Result<Child, String> {
    Command::new(programs.systemd_run.as_path())
        .args(argv)
        .env_clear()
        .env("XDG_RUNTIME_DIR", runtime_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|error| format!("cannot start the job: {error}"))
}

/// How a direct child ended.
#[must_use]
pub fn exit_form(observed: Option<ExitStatus>) -> ExitForm {
    let Some(status) = observed else {
        return ExitForm::Unreported;
    };
    match (status.code(), status.signal()) {
        (Some(code), _) => ExitForm::Exited(code),
        (None, Some(signal)) => ExitForm::Signalled(signal),
        (None, None) => ExitForm::Unreported,
    }
}

/// Require a job's outcome to be exactly the expected one.
///
/// This keeps a probe's body about the property under test rather than about
/// matching enum shapes, and the diagnostic quotes the evidence a failure needs.
///
/// # Errors
///
/// Returns a diagnostic naming the actual outcome when it differs.
pub fn require_outcome(run: &Run, expected: &Outcome) -> Result<(), String> {
    if &run.outcome == expected {
        return Ok(());
    }
    Err(format!(
        "the run reported {:?}, expected {expected:?}; stdout {:?}, stderr {:?}, exit {:?}, \
         result {:?}, survivors {:?}, cgroup empty {}, residual {:?}, events {:?}, fds {:?}, \
         diagnostics {:?}, notes {:?}",
        run.outcome,
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
        run.exit,
        run.result,
        run.survivors,
        run.cgroup_empty,
        run.residual,
        run.events,
        run.fds,
        run.diagnostics,
        run.notes
    ))
}
