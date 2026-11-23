//! The execution boundary, bound to `tests/features/execution_boundary.feature`.
//!
//! The feature file is the readable specification. Each scenario delegates to
//! the probe helper that performs the behaviour and asserts every one of its
//! properties, so the bindings add no second, weaker check: a scenario passes
//! only when the same evidence `make boundary-probe` requires passes.
//!
//! The last scenario starts no job. It injects a missing bubblewrap path into
//! the platform check and requires an unavailable verdict naming it, which is
//! the fail-closed decision the supervisor acts on before any job.
//!
//! The state is a `Slot`-bearing struct rather than a `&mut` world fixture, for
//! the same reason as [`crate::bdd`]: `rstest-bdd` 0.6.0 documents a rustc
//! internal compiler error for the macro-driven mutable world on the pinned
//! nightly.

mod fixture {
    //! The per-scenario state fixture.

    #![expect(
        unused_braces,
        reason = "rstest fixture expansion wraps a single-expression body in braces"
    )]

    use rstest::fixture;

    use super::BoundaryState;

    /// The per-scenario state, built empty for each generated test.
    #[fixture]
    pub fn state() -> BoundaryState { BoundaryState::default() }
}

pub use fixture::state;
use rstest_bdd::Slot;
use rstest_bdd_macros::{ScenarioState, given, scenario, then, when};

use super::{
    super::{
        outcome::{ControlKind, Verdict},
        platform::{Check, PlatformReport, Programs, RequestedLimits, check},
        submit::Submitted,
        supervisor::Run,
    },
    Options,
    bubblewrap,
};
use crate::boundary::supervisor::Boundary;

/// Cargo recompiles this target when the specification changes.
const _: [&str; 1] = [include_str!("../../../features/execution_boundary.feature")];

/// The state one scenario accumulates.
#[derive(Default, ScenarioState)]
struct BoundaryState {
    /// The platform check's report.
    platform: Slot<PlatformReport>,
    /// The outcome of the attempted job submission.
    submission: Slot<Submitted<Run>>,
    /// The probe helper's outcome.
    behaviour: Slot<Result<(), String>>,
}

/// The platform check reports every control available on the probe host.
#[given("the platform check reports every control available")]
fn platform_reports_ready(state: &BoundaryState) -> Result<(), String> {
    let report = platform_report()?;
    if report.verdict() != Verdict::Ready {
        return Err(format!(
            "the platform check must report every control available, got {:?}",
            report.verdict()
        ));
    }
    state.platform.set(report);
    Ok(())
}

/// Run the canary and decide the platform report.
fn platform_report() -> Result<PlatformReport, String> {
    let probe = super::Probe::new("boundary-bdd-canary")?;
    let limits = RequestedLimits::default();
    let (run, report) = probe.canary(&limits)?;
    super::super::supervisor::require_outcome(&run, &super::super::outcome::Outcome::Completed)?;
    Ok(check(&Check {
        programs: probe.programs(),
        runtime_dir: probe.runtime_dir(),
        canary: Ok(&report),
        limits: &limits,
    }))
}

/// The report recorded by the readiness given step.
fn report(state: &BoundaryState) -> Result<PlatformReport, String> {
    state
        .platform
        .with_ref(Clone::clone)
        .ok_or_else(|| "the platform report was not recorded".to_owned())
}

/// The probe creates its host listeners as part of the offline behaviour.
#[given("the probe listens on a loopback port, an abstract socket, and a socket file")]
fn probe_listens(_state: &BoundaryState) {}

/// The hostile job tries every listener.
#[when("a hostile job tries to connect to each of them")]
fn hostile_job_connects(state: &BoundaryState) -> Result<(), String> {
    state
        .behaviour
        .set(bubblewrap::host_listeners_are_unreachable(&report(state)?));
    Ok(())
}

/// Every connection attempt failed.
#[then("every attempt fails inside the job with the expected error")]
fn attempts_fail(state: &BoundaryState) -> Result<(), String> { behaviour(state) }

/// No host listener recorded a connection.
#[then("no listener records a connection")]
fn no_listener_records(state: &BoundaryState) -> Result<(), String> { behaviour(state) }

/// The job held only its standard descriptors.
#[then("the job holds only its standard descriptors")]
fn only_standard_descriptors(state: &BoundaryState) -> Result<(), String> { behaviour(state) }

/// A job allocates above its memory limit.
#[when("a job allocates 256 MiB under a 64 MiB limit")]
fn job_allocates(state: &BoundaryState) -> Result<(), String> {
    state
        .behaviour
        .set(bubblewrap::memory_limit_kills_the_allocation(&report(
            state,
        )?));
    Ok(())
}

/// The outcome names the memory limit.
#[then("the outcome names the memory limit")]
fn outcome_names_memory(state: &BoundaryState) -> Result<(), String> { behaviour(state) }

/// The job's control group is empty.
#[then("the job's control group is empty")]
fn control_group_is_empty(state: &BoundaryState) -> Result<(), String> { behaviour(state) }

/// A job floods standard output.
#[when("a job writes 2 MiB to standard output under a 1 MiB cap")]
fn job_floods(state: &BoundaryState) -> Result<(), String> {
    state
        .behaviour
        .set(bubblewrap::output_cap_stops_the_flood(&report(state)?));
    Ok(())
}

/// The outcome names the output limit.
#[then("the outcome names the output limit")]
fn outcome_names_output(state: &BoundaryState) -> Result<(), String> { behaviour(state) }

/// Exactly the first cap bytes are kept.
#[then("exactly the first 1 MiB of the output is kept")]
fn first_cap_bytes_kept(state: &BoundaryState) -> Result<(), String> { behaviour(state) }

/// A job has started a grandchild in its own process group.
#[given("a job has started a grandchild in a new process group")]
fn job_has_escapee(_state: &BoundaryState) {}

/// The job's deadline expires.
#[when("the job's deadline expires")]
fn deadline_expires(state: &BoundaryState) -> Result<(), String> {
    state
        .behaviour
        .set(bubblewrap::deadline_reaches_the_escapee(&report(state)?));
    Ok(())
}

/// The outcome names the deadline.
#[then("the outcome names the deadline")]
fn outcome_names_deadline(state: &BoundaryState) -> Result<(), String> { behaviour(state) }

/// A job writes to every forbidden path.
#[when("a job writes to its scratch area, the root, its inputs, and a host path")]
fn job_writes_everywhere(state: &BoundaryState) -> Result<(), String> {
    state
        .behaviour
        .set(bubblewrap::job_writes_only_to_scratch(&report(state)?));
    Ok(())
}

/// Only the scratch write succeeded.
#[then("only the scratch write succeeds")]
fn only_scratch_succeeds(state: &BoundaryState) -> Result<(), String> { behaviour(state) }

/// No file from the job remains on the host.
#[then("no file from the job remains on the host")]
fn no_file_remains(state: &BoundaryState) -> Result<(), String> { behaviour(state) }

/// Bubblewrap is not at its configured path.
#[given("bubblewrap is not at its configured path")]
fn bubblewrap_missing(state: &BoundaryState) -> Result<(), String> {
    state.platform.set(missing_bubblewrap_report()?);
    Ok(())
}

/// The check with an injected missing bubblewrap path reports it missing.
fn missing_bubblewrap_report() -> Result<PlatformReport, String> {
    let probe = super::Probe::new("boundary-bdd-missing")?;
    let absent = probe.scratch_path().join("absent-bwrap");
    let programs = Programs {
        bwrap: absent,
        ..Programs::standard()
    };
    let limits = RequestedLimits::default();
    Ok(check(&Check {
        programs: &programs,
        runtime_dir: probe.runtime_dir(),
        canary: Err("the canary is not run when bubblewrap is absent".to_owned()),
        limits: &limits,
    }))
}

/// A job is submitted through the gated path.
#[when("a job is submitted")]
fn job_submitted(state: &BoundaryState) -> Result<(), String> {
    let report = state
        .platform
        .with_ref(Clone::clone)
        .ok_or_else(|| "the platform report was not recorded".to_owned())?;
    let probe = super::Probe::new("boundary-bdd-submit")?;
    let options = Options::new(Boundary::Bubblewrap);
    state
        .submission
        .set(probe.submit(super::hostile::Behaviour::Canary, &options, &report)?);
    Ok(())
}

/// The submission is refused and names bubblewrap.
#[then("the outcome is unavailable and names bubblewrap")]
fn unavailable_names_bubblewrap(state: &BoundaryState) -> Result<(), String> {
    let refused = state
        .submission
        .with_ref(|submitted| match submitted {
            Submitted::Refused(missing) => Some(missing.clone()),
            Submitted::Ran(_) => None,
        })
        .ok_or_else(|| "the job submission was not recorded".to_owned())?;
    match refused {
        Some(missing)
            if missing.iter().any(|control| {
                control.kind == ControlKind::Programs && control.reason.contains("bwrap")
            }) =>
        {
            Ok(())
        }
        Some(missing) => Err(format!("the refusal must name bubblewrap, got {missing:?}")),
        None => Err("a missing control must refuse the submission".to_owned()),
    }
}

/// No job is started.
#[then("no job is started")]
fn no_job_started(state: &BoundaryState) -> Result<(), String> {
    match state.submission.with_ref(Submitted::is_refused) {
        Some(true) => Ok(()),
        Some(false) => Err("a refused submission must start no job".to_owned()),
        None => Err("the job submission was not recorded".to_owned()),
    }
}

/// Require the recorded behaviour to have succeeded.
fn behaviour(state: &BoundaryState) -> Result<(), String> {
    match state.behaviour.with_ref(Clone::clone) {
        Some(Ok(())) => Ok(()),
        Some(Err(reason)) => Err(reason),
        None => Err("the hostile behaviour was not run".to_owned()),
    }
}

/// A job cannot reach host listeners or inherit host descriptors.
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
#[scenario(
    path = "tests/features/execution_boundary.feature",
    name = "A job cannot reach host listeners or inherit host descriptors"
)]
fn listeners_are_unreachable(#[from(state)] _state: BoundaryState) {}

/// A job that exceeds its memory limit is killed as a group.
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
#[scenario(
    path = "tests/features/execution_boundary.feature",
    name = "A job that exceeds its memory limit is killed as a group"
)]
fn memory_limit_kills(#[from(state)] _state: BoundaryState) {}

/// A job that floods its output is stopped at the cap.
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
#[scenario(
    path = "tests/features/execution_boundary.feature",
    name = "A job that floods its output is stopped at the cap"
)]
fn output_cap_stops(#[from(state)] _state: BoundaryState) {}

/// A deadline reaches a process that left the process group.
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
#[scenario(
    path = "tests/features/execution_boundary.feature",
    name = "A deadline reaches a process that left the process group"
)]
fn deadline_reaches_group_escapee(#[from(state)] _state: BoundaryState) {}

/// A job writes only to its private scratch area.
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
#[scenario(
    path = "tests/features/execution_boundary.feature",
    name = "A job writes only to its private scratch area"
)]
fn writes_only_scratch(#[from(state)] _state: BoundaryState) {}

/// Missing enforcement starts no job.
#[scenario(
    path = "tests/features/execution_boundary.feature",
    name = "Missing enforcement starts no job"
)]
fn missing_enforcement_starts_nothing(#[from(state)] _state: BoundaryState) {}
