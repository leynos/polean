//! The selected boundary: the systemd scope around bubblewrap.
//!
//! Each probe runs one hostile behaviour the negative controls showed succeeding
//! without the boundary, and requires the boundary to stop it. The canary probe
//! checks the platform verdict end to end: the isolation it reports, the limits
//! systemd actually applied, and the descriptors the job really held.
//!
//! A probe passes only on positive evidence from the child or the supervisor.
//! Silence is a failure, a missing read-back is a failure, and a cleanup
//! diagnostic makes the outcome a containment failure.

use std::time::Duration;

use super::{
    super::{
        hostile::Behaviour,
        outcome::{Bound, Outcome, Verdict},
        platform::{Check, RequestedLimits, check as platform_check},
        supervisor::{Boundary, Run, require_outcome},
    },
    Options,
    Probe,
    attempt,
    listeners,
    started,
};
use crate::{
    ambient,
    scratch::{check, check_eq},
};

/// The default probe limits: 64 MiB, 8 MiB scratch, 32 tasks.
fn limits() -> RequestedLimits { RequestedLimits::default() }

/// Host listeners are unreachable and no descriptor beyond the standard
/// streams crosses the boundary.
#[test]
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
fn host_listeners_are_unreachable_inside_the_boundary() -> Result<(), String> {
    host_listeners_are_unreachable()
}

/// The offline property, for the feature scenario.
pub(super) fn host_listeners_are_unreachable() -> Result<(), String> {
    let probe = Probe::new("boundary-bubblewrap-offline")?;
    let (endpoints, watchers) = listeners::listen(probe.nonce())?;
    let options = Options::new(Boundary::Bubblewrap)
        .deadline(Duration::from_secs(10))
        .targets(endpoints.targets())
        .target(format!("polean-target=marker:{}", probe.nonce()));
    let run = probe.run(Behaviour::Offline, &options)?;
    let stdout = started(&run, &probe)?;
    let tcp = attempt(&stdout, "tcp")?;
    // A loopback address has a route inside an empty network namespace, so the
    // connection is refused rather than reporting that the network is
    // unreachable; the off-host canary observes `Network is unreachable`.
    check!(
        tcp.contains("Connection refused") || tcp.contains("Network is unreachable"),
        "the TCP attempt must be refused or unreachable, got {tcp:?}"
    );
    for name in ["abstract", "socket-file"] {
        let line = attempt(&stdout, name)?;
        check!(
            line.contains("Connection refused") || line.contains("No such file"),
            "the {name} attempt must be refused or absent, got {line:?}"
        );
    }
    watchers.require_silent()?;
    check_eq!(
        run.fds,
        Some(vec![0, 1, 2]),
        "the job must hold only its standard streams"
    );
    require_outcome(&run, &Outcome::Completed)
}

/// An allocation above the memory limit is killed as a group.
#[test]
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
fn the_memory_limit_kills_the_allocation() -> Result<(), String> {
    memory_limit_kills_the_allocation()
}

/// The allocation property, for the feature scenario.
pub(super) fn memory_limit_kills_the_allocation() -> Result<(), String> {
    let probe = Probe::new("boundary-bubblewrap-memory")?;
    let options = Options::new(Boundary::Bubblewrap)
        .memory(limits().memory_bytes)
        .deadline(Duration::from_secs(20))
        .target("polean-target=memory:allocate");
    let run = probe.run(Behaviour::Memory, &options)?;
    drop(started(&run, &probe)?);
    require_memory_kill(&run)
}

/// A scratch fill larger than the memory limit is killed by the memory limit,
/// not by the tmpfs bound: tmpfs pages are charged to the job's cgroup.
#[test]
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
fn the_scratch_fill_is_killed_by_the_memory_limit() -> Result<(), String> {
    let probe = Probe::new("boundary-bubblewrap-scratch-oom")?;
    let options = Options::new(Boundary::Bubblewrap)
        .memory(limits().memory_bytes)
        .scratch(128 * 1024 * 1024)
        .deadline(Duration::from_secs(20))
        .target("polean-target=memory:fill");
    let run = probe.run(Behaviour::Memory, &options)?;
    drop(started(&run, &probe)?);
    require_memory_kill(&run)
}

/// Require the out-of-memory kill systemd recorded, with an empty group.
fn require_memory_kill(run: &Run) -> Result<(), String> {
    require_outcome(run, &Outcome::BoundsExhausted(vec![Bound::Memory]))?;
    check_eq!(
        run.result.as_deref(),
        Some("oom-kill"),
        "the bound must be read from systemd's record, not the exit status"
    );
    check!(run.cgroup_empty, "the scope must be empty afterwards");
    Ok(())
}

/// A flood past the output cap stops the job and keeps exactly the cap.
#[test]
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
fn the_output_cap_stops_the_flood() -> Result<(), String> { output_cap_stops_the_flood() }

/// The output property, for the feature scenario.
pub(super) fn output_cap_stops_the_flood() -> Result<(), String> {
    let probe = Probe::new("boundary-bubblewrap-output")?;
    let options = Options::new(Boundary::Bubblewrap)
        .deadline(Duration::from_secs(15))
        .stdout_cap(1024 * 1024);
    let run = probe.run(Behaviour::Output, &options)?;
    drop(started(&run, &probe)?);
    require_outcome(&run, &Outcome::BoundsExhausted(vec![Bound::Stdout]))?;
    check_eq!(
        run.stdout.len(),
        1024 * 1024,
        "exactly the first cap bytes must be kept"
    );
    Ok(())
}

/// A deadline reaches a process that left the process group, through the PID
/// namespace, and the cgroup is empty afterwards.
#[test]
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
fn a_deadline_reaches_the_escapee() -> Result<(), String> { deadline_reaches_the_escapee() }

/// The cancellation property, for the feature scenario.
pub(super) fn deadline_reaches_the_escapee() -> Result<(), String> {
    let probe = Probe::new("boundary-bubblewrap-escapee")?;
    let options = Options::new(Boundary::Bubblewrap).deadline(Duration::from_millis(1500));
    let run = probe.run(Behaviour::Escapee, &options)?;
    let stdout = started(&run, &probe)?;
    let _line = attempt(&stdout, "escapee")?;
    require_outcome(&run, &Outcome::BoundsExhausted(vec![Bound::Deadline]))?;
    check!(
        run.survivors.is_empty(),
        "the PID namespace must remove the escapee with the group kill; survivors were {:?}",
        run.survivors
    );
    // The pid the child reports is its namespace-local pid, which names an
    // unrelated host process from outside; the empty cgroup and empty survivor
    // list are the host-side evidence that the escapee is gone.
    check!(run.cgroup_empty, "the job's control group must be empty");
    Ok(())
}

/// A job writes only to its private scratch area; no file it makes reaches the
/// host, and a fill past `--size` fails with `ENOSPC`.
#[test]
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
fn the_job_writes_only_to_its_scratch_area() -> Result<(), String> { job_writes_only_to_scratch() }

/// The storage property, for the feature scenario.
pub(super) fn job_writes_only_to_scratch() -> Result<(), String> {
    let probe = Probe::new("boundary-bubblewrap-storage")?;
    let host = probe.scratch_path().join("hostdir");
    ambient::create_dir(&host)
        .map_err(|error| format!("cannot create {}: {error}", host.display()))?;
    let device_marker = format!("/dev/shm/polean-{}", probe.nonce());
    let options = Options::new(Boundary::Bubblewrap)
        .deadline(Duration::from_secs(20))
        .target(format!(
            "polean-target=host:{}",
            host.join("marker").display()
        ))
        .target(format!("polean-target=marker:{}", probe.nonce()));
    let run = probe.run(Behaviour::Storage, &options)?;
    let stdout = started(&run, &probe)?;
    check_eq!(
        attempt(&stdout, "scratch")?,
        "wrote",
        "the scratch write must succeed"
    );
    for name in ["root", "dev-shm", "inputs", "host"] {
        let line = attempt(&stdout, name)?;
        check!(
            line.starts_with("error"),
            "the {name} write must fail inside the boundary, got {line:?}"
        );
    }
    let fill = attempt(&stdout, "scratch-fill")?;
    check!(
        fill.contains("No space left") || fill.contains("ENOSPC"),
        "filling past --size must fail with ENOSPC, got {fill:?}"
    );
    require_outcome(&run, &Outcome::Completed)?;
    check!(
        ambient::metadata(&host.join("marker")).is_err(),
        "no file from the job may remain on the host"
    );
    check!(
        ambient::metadata(std::path::Path::new(&device_marker)).is_err(),
        "the job's /dev/shm write must not reach the host"
    );
    Ok(())
}

/// The platform check reports every control available on the probe host.
#[test]
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
fn the_platform_is_ready() -> Result<(), String> { platform_is_ready() }

/// The canary and limit read-back, for the feature scenario.
pub(super) fn platform_is_ready() -> Result<(), String> {
    let probe = Probe::new("boundary-bubblewrap-canary")?;
    let limits = limits();
    let (run, report) = probe.canary(&limits)?;
    require_outcome(&run, &Outcome::Completed)?;
    check_eq!(
        report.started,
        true,
        "the canary must report its start line"
    );
    check_eq!(
        report.no_new_privs.as_deref(),
        Some("1"),
        "the canary must run with no new privileges"
    );
    check_eq!(
        report.interfaces,
        vec!["lo".to_owned()],
        "the canary must see only loopback"
    );
    let outbound = report
        .outbound_error
        .as_deref()
        .unwrap_or_default()
        .to_owned();
    check!(
        outbound.contains("Network is unreachable"),
        "the outbound attempt must be unreachable, got {outbound:?}"
    );
    check_eq!(
        report.descriptors,
        Some(vec![0, 1, 2]),
        "the canary must hold only its standard streams"
    );
    let platform = platform_check(&Check {
        programs: probe.programs(),
        runtime_dir: probe.runtime_dir(),
        canary: Ok(&report),
        limits: &limits,
    });
    check_eq!(
        platform.verdict(),
        Verdict::Ready,
        "every enforcement control must be available on the probe host"
    );
    Ok(())
}
