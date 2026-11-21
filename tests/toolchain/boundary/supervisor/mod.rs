//! The job supervisor: spawn, bounded capture, and unconditional teardown.
//!
//! One job is one `systemd-run --scope` invocation, optionally around
//! bubblewrap. The supervisor:
//!
//! 1. spawns the launch vector with a cleared environment, a new process group, standard input from
//!    `/dev/null`, and both streams piped;
//! 2. waits, within a bound, until the scope exists with the job inside it;
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
//! The supervisor spawns and reaps on one thread because `--die-with-parent`
//! fires when the spawning thread exits.

use std::{
    io::Read,
    os::unix::process::{CommandExt, ExitStatusExt},
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{Receiver, channel},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use super::{
    capture::Capture,
    launch::{LaunchSpec, bubblewrap_scope_argv, unconfined_scope_argv},
    outcome::{Bound, Bounds, ExitForm, Observation, Outcome, classify},
    platform::Programs,
    scope::Scope,
};
use crate::scratch::check_eq;

mod teardown;

use teardown::{Inputs, tear_down};

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
}

/// Runs jobs through the selected boundary.
pub struct Supervisor<'a> {
    /// The launcher programs.
    pub programs: &'a Programs,
    /// The directory the user manager is reached through.
    pub runtime_dir: &'a Path,
}

impl Supervisor<'_> {
    /// Run one job to completion, teardown included.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the job's specification is invalid, the launch
    /// vector cannot be spawned, the scope cannot be observed, or teardown
    /// cannot be completed. A job that merely fails its bounds or is killed is a
    /// [`Run`], not an error.
    pub fn run(&self, job: &Job) -> Result<Run, String> {
        job.spec.validate()?;
        let started = Instant::now();
        let argv = match job.boundary {
            Boundary::Bubblewrap => bubblewrap_scope_argv(&job.spec, self.programs),
            Boundary::Unconfined => unconfined_scope_argv(&job.spec),
        };
        let mut child = spawn(self.programs, self.runtime_dir, &argv)?;
        let pgid = child.id();
        let scope = Scope {
            programs: self.programs,
            runtime_dir: self.runtime_dir,
            unit: job.spec.unit.clone(),
        };
        let cgroup = scope.wait_until_joined(pgid).ok();
        let readers = readers(&mut child, job)?;
        let (status, mut bounds, running) =
            wait(&mut child, &readers.trips, job.deadline, started)?;
        let (report, reaped) = tear_down(Inputs {
            child: &mut child,
            pgid,
            cgroup: cgroup.as_deref(),
            kill: self.programs.kill.as_path(),
            running,
            status,
        })?;
        let stdout = join(readers.stdout, "standard output")?;
        let stderr = join(readers.stderr, "standard error")?;
        let result = scope.wait_for_result()?;
        if !matches!(result.as_deref(), None | Some("success")) {
            scope.reset_failed();
        }
        if result.as_deref() == Some("oom-kill") {
            bounds.insert(Bound::Memory);
        }
        let exit = exit_form(reaped);
        let outcome = classify(&Observation {
            bounds,
            exit,
            cgroup_empty: report.cgroup_empty,
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
            cgroup_empty: report.cgroup_empty,
            residual: report.residual,
            events: report.events,
            result,
        })
    }
}

/// Poll the child, its stream trips, and the deadline until one stops it.
///
/// Returns the status when the child exited on its own, the bounds that fired,
/// and whether the child was still running when the loop stopped.
fn wait(
    child: &mut Child,
    trips: &Receiver<Bound>,
    deadline: Duration,
    started: Instant,
) -> Result<(Option<ExitStatus>, Bounds, bool), String> {
    let mut bounds = Bounds::none();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok((Some(status), bounds, false)),
            Ok(None) => (),
            Err(error) => return Err(format!("cannot wait for the job: {error}")),
        }
        if let Ok(bound) = trips.try_recv() {
            bounds.insert(bound);
            return Ok((None, bounds, true));
        }
        if started.elapsed() >= deadline {
            bounds.insert(Bound::Deadline);
            return Ok((None, bounds, true));
        }
        thread::sleep(super::scope::POLL);
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

/// The readers and the trip channel for one job.
struct Readers {
    /// The standard-output reader.
    stdout: JoinHandle<Vec<u8>>,
    /// The standard-error reader.
    stderr: JoinHandle<Vec<u8>>,
    /// The first-trip signal from either reader.
    trips: Receiver<Bound>,
}

/// Start one bounded reader per stream.
fn readers(child: &mut Child, job: &Job) -> Result<Readers, String> {
    let (sender, receiver) = channel();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "standard output is not piped".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "standard error is not piped".to_owned())?;
    let stdout_reader = drain(stdout, job.stdout_cap, Bound::Stdout, sender.clone());
    let stderr_reader = drain(stderr, job.stderr_cap, Bound::Stderr, sender);
    Ok(Readers {
        stdout: stdout_reader,
        stderr: stderr_reader,
        trips: receiver,
    })
}

/// Read one stream into a capture until end of file.
///
/// The trip is signalled once. After it, the reader keeps draining into the
/// capture, which keeps nothing more, so the child never blocks on a full pipe
/// before the supervisor's kill lands.
fn drain(
    mut stream: impl Read + Send + 'static,
    cap: usize,
    bound: Bound,
    trips: std::sync::mpsc::Sender<Bound>,
) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut capture = Capture::new(cap);
        let mut signalled = false;
        let mut buffer = [0u8; 16 * 1024];
        loop {
            let read = match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            };
            let chunk = buffer.get(..read).unwrap_or(&[]);
            capture.feed(chunk);
            if !record_trip(&capture, &trips, bound, &mut signalled) {
                break;
            }
        }
        capture.kept().to_vec()
    })
}

/// Signal the first trip, reporting whether reading should continue.
fn record_trip(
    capture: &Capture,
    trips: &std::sync::mpsc::Sender<Bound>,
    bound: Bound,
    signalled: &mut bool,
) -> bool {
    if !capture.tripped() || *signalled {
        return true;
    }
    *signalled = true;
    trips.send(bound).is_ok()
}

/// Join one reader, naming the stream when its thread panicked.
fn join(reader: JoinHandle<Vec<u8>>, name: &str) -> Result<Vec<u8>, String> {
    reader
        .join()
        .map_err(|_panic| format!("the {name} reader panicked"))
}

/// How a direct child ended.
#[must_use]
pub fn exit_form(status: ExitStatus) -> ExitForm {
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
    check_eq!(
        &run.outcome,
        expected,
        "the run reported an unexpected outcome; stdout {:?}, stderr {:?}, exit {:?}, result \
         {:?}, survivors {:?}, cgroup empty {}, residual {:?}, events {:?}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
        run.exit,
        run.result,
        run.survivors,
        run.cgroup_empty,
        run.residual,
        run.events
    );
    Ok(())
}
