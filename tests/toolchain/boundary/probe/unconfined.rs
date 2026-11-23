//! The negative controls: the scope without bubblewrap.
//!
//! Each probe runs the same hostile behaviour the selected boundary will run,
//! with the isolating layer removed. The control must show the hostile
//! behaviour succeeding; otherwise the selected boundary's failure would prove
//! nothing, because the probe would not notice the property failing at all.
//!
//! The escapee cell is the reason the cgroup, not the process group, is the
//! authority: a process that leaves the group outlives a group kill, and only
//! `cgroup.kill` removes it. Under bubblewrap, EP-M3 shows the PID namespace
//! removing it even after a group kill alone.

use std::time::Duration;

use super::{
    super::{
        hostile::Behaviour,
        outcome::{Bound, ExitForm, Outcome},
        probe::{Options, Probe, attempt, attempt_containing, started, wait_until_gone},
        supervisor::{Boundary, require_outcome},
    },
    listeners,
};
use crate::{
    ambient,
    scratch::{check, check_eq},
};

/// The negative control for offline execution and inherited descriptors.
#[test]
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
pub(super) fn host_listeners_are_reachable_without_the_boundary() -> Result<(), String> {
    let probe = Probe::new("boundary-unconfined-offline")?;
    let (endpoints, watchers) = listeners::listen(probe.nonce())?;
    let options = Options::new(Boundary::Unconfined)
        .deadline(Duration::from_secs(10))
        .targets(endpoints.targets());
    let run = probe.run(Behaviour::Offline, &options)?;
    let stdout = started(&run, &probe)?;
    for name in ["tcp", "abstract", "socket-file"] {
        check_eq!(
            attempt(&stdout, name)?,
            "connected",
            "the negative control must reach the {name} listener"
        );
    }
    watchers.require_connected()?;
    check_eq!(
        run.fds,
        Some(vec![0, 1, 2]),
        "the supervisor must observe exactly the standard streams"
    );
    require_outcome(&run, &Outcome::Completed)
}

/// The negative control for the memory limit.
#[test]
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
pub(super) fn the_allocation_completes_without_a_memory_limit() -> Result<(), String> {
    let probe = Probe::new("boundary-unconfined-memory")?;
    let options = Options::new(Boundary::Unconfined)
        .memory(1024 * 1024 * 1024)
        .deadline(Duration::from_secs(30))
        .target("polean-target=memory:allocate");
    let run = probe.run(Behaviour::Memory, &options)?;
    let stdout = started(&run, &probe)?;
    let line = attempt_containing(&stdout, "memory", "allocated")?;
    check!(
        line.contains("allocated"),
        "the allocation must complete under the lifted limit, got {line:?}"
    );
    require_outcome(&run, &Outcome::Completed)
}

/// The negative control for the output cap.
#[test]
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
pub(super) fn the_flood_is_kept_whole_without_the_cap() -> Result<(), String> {
    let probe = Probe::new("boundary-unconfined-output")?;
    let options = Options::new(Boundary::Unconfined)
        .stdout_cap(4 * 1024 * 1024)
        .deadline(Duration::from_secs(15));
    let run = probe.run(Behaviour::Output, &options)?;
    let stdout = started(&run, &probe)?;
    let line = attempt(&stdout, "stdout")?;
    check!(
        line.starts_with("wrote 2097152"),
        "the flood must be written whole under the lifted cap, got {line:?}"
    );
    // The marker alone would not show that the supervisor kept the bytes, so
    // compare the retained stream: after the start line come exactly the flood
    // bytes, then the attempt line.
    let start = format!("{}\n", super::canary::start_line(probe.nonce()));
    let offset = run
        .stdout
        .windows(start.len())
        .position(|window| window == start.as_bytes())
        .ok_or_else(|| "the kept output holds no start line".to_owned())?;
    let preamble = b"\nrunning 1 test\n";
    check!(
        run.stdout.starts_with(preamble),
        "the retained stream must begin with libtest's preamble"
    );
    check_eq!(
        offset,
        preamble.len(),
        "the start line must immediately follow the preamble"
    );
    let flood = run
        .stdout
        .get(offset + start.len()..offset + start.len() + super::hostile::output::FLOOD_BYTES)
        .ok_or_else(|| "the kept output is shorter than the flood".to_owned())?;
    check!(
        flood
            .iter()
            .all(|byte| *byte == super::hostile::output::FLOOD_BYTE),
        "the supervisor must keep the whole flood unchanged"
    );
    // After the flood comes the child's attempt line and libtest's own
    // summary, so the complete retained stream is checked, not just its head.
    let expect_tail = format!(
        "\npolean-hostile attempt stdout wrote {}",
        super::hostile::output::FLOOD_BYTES
    );
    let tail = run
        .stdout
        .get(offset + start.len() + super::hostile::output::FLOOD_BYTES..)
        .ok_or_else(|| "the kept output ends inside the flood".to_owned())?;
    let after_flood = String::from_utf8_lossy(tail);
    check!(
        after_flood.starts_with(&expect_tail),
        "the attempt line must follow the flood, got {after_flood:?}"
    );
    check!(
        after_flood.contains("test result: ok"),
        "libtest's summary must be retained after the attempt line, got {after_flood:?}"
    );
    check!(
        !run.bounds.contains(Bound::Stdout),
        "the lifted cap must not trip"
    );
    require_outcome(&run, &Outcome::Completed)
}

/// The negative control for private storage.
#[test]
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
pub(super) fn the_job_writes_into_a_host_directory_without_the_boundary() -> Result<(), String> {
    let probe = Probe::new("boundary-unconfined-storage")?;
    let host = probe.scratch_path().join("hostdir");
    ambient::create_dir(&host)
        .map_err(|error| format!("cannot create {}: {error}", host.display()))?;
    let device_marker = format!("/dev/shm/polean-{}", probe.nonce());
    let options = Options::new(Boundary::Unconfined)
        .deadline(Duration::from_secs(10))
        .target(format!(
            "polean-target=host:{}",
            host.join("marker").display()
        ))
        .target(format!("polean-target=marker:{}", probe.nonce()));
    let run = probe.run(Behaviour::Storage, &options)?;
    let stdout = started(&run, &probe)?;
    let outcome = attempt(&stdout, "host");
    // Remove the device marker the negative control necessarily created, so the
    // shared host is left as it was found.
    drop(ambient::remove_file(std::path::Path::new(&device_marker)));
    check_eq!(
        outcome?,
        "wrote",
        "the negative control must write into the probe's host directory"
    );
    check!(
        ambient::metadata(&host.join("marker")).is_ok(),
        "the marker must be present in the host directory"
    );
    require_outcome(&run, &Outcome::Completed)
}

/// A grandchild that left the process group outlives a group kill.
#[test]
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
pub(super) fn an_escapee_outlives_the_process_group_kill() -> Result<(), String> {
    let probe = Probe::new("boundary-unconfined-escapee")?;
    let options = Options::new(Boundary::Unconfined).deadline(Duration::from_millis(1500));
    let run = probe.run(Behaviour::Escapee, &options)?;
    let stdout = started(&run, &probe)?;
    let line = attempt(&stdout, "escapee")?;
    let pid = line
        .strip_prefix("started ")
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| format!("the escapee's pid is missing from {line:?}"))?;
    require_outcome(&run, &Outcome::ContainmentFailure(vec![Bound::Deadline]))?;
    check!(
        run.survivors.contains(&pid),
        "the grandchild must outlive the process-group kill; survivors were {:?}",
        run.survivors
    );
    check_eq!(
        run.exit,
        ExitForm::Signalled(9),
        "the direct child is killed at the deadline"
    );
    wait_until_gone(pid)
}
