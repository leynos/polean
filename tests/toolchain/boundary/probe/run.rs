//! Running one job: the ungated mechanism, the cancellation timer, and the gate.
//!
//! The probe's own methods live here so the module that defines the probe stays
//! within the size rule. [`Probe::submit`] is the gated entry; [`Probe::run`]
//! and [`Probe::run_tool`] are the mechanism used by the canary, the negative
//! controls, and the tests that have already established readiness.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use super::{
    super::{
        hostile::Behaviour,
        launch::{LaunchSpec, ReadOnlyBind},
        platform::PlatformReport,
        submit::{Submitted, submit as gate},
        supervisor::{Job, Run, Supervisor},
    },
    Options,
    PROBE_INPUTS,
    Probe,
    ToolJob,
};

/// Whether a job requires the scope observation and any cancellation request.
struct RunRequest {
    /// Whether the scope join is required evidence.
    require_scope: bool,
    /// The cancellation request, when the caller supplies one.
    cancel: Option<Arc<AtomicBool>>,
}

impl RunRequest {
    /// A hostile job, which requires the scope observation.
    const fn hostile() -> Self {
        Self {
            require_scope: true,
            cancel: None,
        }
    }

    /// A hostile job with a cancellation request.
    const fn cancelled(cancel: Arc<AtomicBool>) -> Self {
        Self {
            require_scope: true,
            cancel: Some(cancel),
        }
    }

    /// A tool job, which may finish before its scope is observed.
    const fn tool() -> Self {
        Self {
            require_scope: false,
            cancel: None,
        }
    }
}

impl Probe {
    /// Submit one hostile behaviour under the selected boundary.
    ///
    /// The job starts only when `report` is ready; a missing control returns
    /// the refusal and starts nothing.
    ///
    /// # Errors
    ///
    /// Returns the supervisor's diagnostic when the platform is ready and the
    /// job could not be observed.
    pub fn submit(
        &self,
        behaviour: Behaviour,
        options: &Options,
        report: &PlatformReport,
    ) -> Result<Submitted<Run>, String> {
        gate(report, || self.run(behaviour, options))
    }

    /// Run one hostile behaviour under the selected boundary.
    ///
    /// This is the ungated mechanism: the canary and the negative controls
    /// must run before a verdict exists or without one.
    ///
    /// # Errors
    ///
    /// Returns the supervisor's diagnostic when the job could not be observed.
    pub(super) fn run(&self, behaviour: Behaviour, options: &Options) -> Result<Run, String> {
        self.execute(
            self.spec(behaviour, options),
            options,
            RunRequest::hostile(),
        )
    }

    /// Run one behaviour and request cancellation after `after`.
    ///
    /// The request is delivered while the job is running, so the property is
    /// the supervisor's explicit-cancellation path, not a pre-spawn refusal.
    ///
    /// # Errors
    ///
    /// Returns the supervisor's diagnostic when the job could not be observed.
    pub(super) fn run_cancelled(
        &self,
        behaviour: Behaviour,
        options: &Options,
        after: Duration,
    ) -> Result<Run, String> {
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancel);
        let (stop, stopped) = mpsc::channel();
        let timer = thread::spawn(move || {
            if stopped.recv_timeout(after).is_err() {
                flag.store(true, Ordering::Relaxed);
            }
        });
        let run = self.execute(
            self.spec(behaviour, options),
            options,
            RunRequest::cancelled(cancel),
        );
        if stop.send(()).is_err() {
            // The timer has already fired and set the flag; nothing to stop.
        }
        timer
            .join()
            .map_err(|_| "the cancellation timer panicked".to_owned())?;
        run
    }

    /// Run one prepared specification through the supervisor.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the specification is invalid or the job could
    /// not be observed.
    fn execute(&self, spec: LaunchSpec, options: &Options, run: RunRequest) -> Result<Run, String> {
        spec.validate()?;
        let supervisor = Supervisor {
            programs: &self.programs,
            runtime_dir: &self.runtime_dir,
        };
        supervisor.run(&Job {
            spec,
            boundary: options.boundary,
            stdout_cap: options.stdout_cap,
            stderr_cap: options.stderr_cap,
            deadline: options.deadline,
            read_limits: options.read_limits,
            require_scope: run.require_scope,
            cancel: run.cancel,
        })
    }

    /// Run a non-hostile program under the selected boundary.
    ///
    /// The tool's own binds are added to the input snapshot bind, and the
    /// supervisor runs it through the same scope and capture every job uses.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the tool's specification is invalid or the job
    /// could not be observed.
    pub fn run_tool(&self, tool: &ToolJob<'_>, options: &Options) -> Result<Run, String> {
        let mut binds = tool.binds.clone();
        binds.push(ReadOnlyBind {
            source: self.inputs.clone(),
            target: PROBE_INPUTS.to_owned(),
        });
        let spec = LaunchSpec {
            host_program: tool.host_program.to_path_buf(),
            sandbox_program: tool.sandbox_program.to_owned(),
            args: tool.args.clone(),
            binds,
            scratch_bytes: options.scratch_bytes,
            env: tool.env.clone(),
            unit: format!("polean-probe-{}", self.nonce),
            memory_bytes: options.memory_bytes,
            swap_bytes: 0,
            tasks_max: 32,
            runtime_secs: options.deadline.as_secs() + 5,
        };
        self.execute(spec, options, RunRequest::tool())
    }
}
