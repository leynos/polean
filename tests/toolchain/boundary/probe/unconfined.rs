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

use std::{
    io,
    net::TcpListener,
    os::{
        linux::net::SocketAddrExt,
        unix::net::{SocketAddr, UnixListener},
    },
    path::PathBuf,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use super::super::{
    hostile::Behaviour,
    outcome::{Bound, ExitForm, Outcome},
    probe::{Options, Probe, attempt, started},
    supervisor::require_outcome,
};
use crate::{
    ambient,
    scratch::{check, check_eq},
};

/// How long a listener waits for the child's connection.
const WATCH_BOUND: Duration = Duration::from_secs(10);

/// How often a listener polls for a connection.
const WATCH_POLL: Duration = Duration::from_millis(20);

/// How long a survivor's `/proc` entry may take to disappear.
const GONE_BOUND: Duration = Duration::from_secs(2);

/// Accept one connection and report whether one arrived.
fn watch(accept: impl Fn() -> io::Result<()> + Send + 'static) -> JoinHandle<bool> {
    thread::spawn(move || {
        let started = Instant::now();
        while started.elapsed() < WATCH_BOUND {
            match accept() {
                Ok(()) => return true,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(WATCH_POLL);
                }
                Err(_) => return false,
            }
        }
        false
    })
}

/// Require one listener to have recorded a connection.
fn accepted(handle: JoinHandle<bool>, name: &str) -> Result<(), String> {
    match handle.join() {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!("the {name} listener recorded no connection")),
        Err(_) => Err(format!("the {name} listener thread panicked")),
    }
}

/// The negative control for offline execution and inherited descriptors.
#[test]
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
fn host_listeners_are_reachable_without_the_boundary() -> Result<(), String> {
    let probe = Probe::new("boundary-unconfined-offline")?;
    let tcp = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("cannot listen on loopback: {error}"))?;
    let address = tcp
        .local_addr()
        .map_err(|error| format!("cannot read the loopback port: {error}"))?;
    tcp.set_nonblocking(true)
        .map_err(|error| format!("cannot make the TCP listener non-blocking: {error}"))?;
    let tcp_watch = watch(move || tcp.accept().map(drop));
    let abstract_name = format!("polean-probe-{}", probe.nonce());
    let abstract_address = SocketAddr::from_abstract_name(abstract_name.as_bytes())
        .map_err(|error| format!("cannot name the abstract socket: {error}"))?;
    let abstract_listener = UnixListener::bind_addr(&abstract_address)
        .map_err(|error| format!("cannot bind the abstract socket: {error}"))?;
    abstract_listener
        .set_nonblocking(true)
        .map_err(|error| format!("cannot make the abstract listener non-blocking: {error}"))?;
    let abstract_watch = watch(move || abstract_listener.accept().map(drop));
    // A Unix socket path is limited to `SUN_LEN`, which the scratch directory's
    // absolute path can exceed; the probe owns this unique name and removes it.
    let socket_path = PathBuf::from(format!("/tmp/polean-probe-{}.sock", probe.nonce()));
    let socket_listener = UnixListener::bind(&socket_path)
        .map_err(|error| format!("cannot bind {}: {error}", socket_path.display()))?;
    socket_listener
        .set_nonblocking(true)
        .map_err(|error| format!("cannot make the socket listener non-blocking: {error}"))?;
    let socket_watch = watch(move || socket_listener.accept().map(drop));
    let options = Options::new(super::super::supervisor::Boundary::Unconfined)
        .deadline(Duration::from_secs(10))
        .target(format!("polean-target=tcp:{address}"))
        .target(format!("polean-target=abstract:{abstract_name}"))
        .target(format!("polean-target=socket:{}", socket_path.display()));
    let run = probe.run(Behaviour::Offline, &options)?;
    drop(ambient::remove_file(&socket_path));
    let stdout = started(&run, &probe)?;
    for name in ["tcp", "abstract", "socket-file"] {
        check_eq!(
            attempt(&stdout, name)?,
            "connected",
            "the negative control must reach the {name} listener"
        );
    }
    accepted(tcp_watch, "TCP")?;
    accepted(abstract_watch, "abstract")?;
    accepted(socket_watch, "socket-file")?;
    let descriptors = attempt(&stdout, "descriptors")?;
    check!(
        descriptors.contains("0,1,2") || descriptors == "0,1,2",
        "the child must at least inherit its standard streams, got {descriptors:?}"
    );
    require_outcome(&run, &Outcome::Completed)
}

/// The negative control for the memory limit.
#[test]
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
fn the_allocation_completes_without_a_memory_limit() -> Result<(), String> {
    let probe = Probe::new("boundary-unconfined-memory")?;
    let options = Options::new(super::super::supervisor::Boundary::Unconfined)
        .memory(1024 * 1024 * 1024)
        .deadline(Duration::from_secs(30))
        .target("polean-target=memory:allocate");
    let run = probe.run(Behaviour::Memory, &options)?;
    let stdout = started(&run, &probe)?;
    let line = attempt(&stdout, "memory")?;
    check!(
        line.contains("allocated"),
        "the allocation must complete under the lifted limit, got {line:?}"
    );
    require_outcome(&run, &Outcome::Completed)
}

/// The negative control for the output cap.
#[test]
#[ignore = "run by make boundary-probe; starts supervised hostile jobs"]
fn the_flood_is_kept_whole_without_the_cap() -> Result<(), String> {
    let probe = Probe::new("boundary-unconfined-output")?;
    let options = Options::new(super::super::supervisor::Boundary::Unconfined)
        .stdout_cap(4 * 1024 * 1024)
        .deadline(Duration::from_secs(15));
    let run = probe.run(Behaviour::Output, &options)?;
    let stdout = started(&run, &probe)?;
    let line = attempt(&stdout, "stdout")?;
    check!(
        line.starts_with("wrote 2097152"),
        "the flood must be written whole under the lifted cap, got {line:?}"
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
fn the_job_writes_into_a_host_directory_without_the_boundary() -> Result<(), String> {
    let probe = Probe::new("boundary-unconfined-storage")?;
    let host = probe.scratch_path().join("hostdir");
    ambient::create_dir(&host)
        .map_err(|error| format!("cannot create {}: {error}", host.display()))?;
    let device_marker = format!("/dev/shm/polean-{}", probe.nonce());
    let options = Options::new(super::super::supervisor::Boundary::Unconfined)
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
fn an_escapee_outlives_the_process_group_kill() -> Result<(), String> {
    let probe = Probe::new("boundary-unconfined-escapee")?;
    let options = Options::new(super::super::supervisor::Boundary::Unconfined)
        .deadline(Duration::from_millis(1500));
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
    wait_gone(pid)
}

/// Wait until a process's `/proc` entry has gone.
fn wait_gone(pid: u32) -> Result<(), String> {
    let path = PathBuf::from(format!("/proc/{pid}"));
    let started = Instant::now();
    loop {
        match ambient::metadata(&path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(format!("cannot inspect {}: {error}", path.display())),
            Ok(_metadata) => {
                if started.elapsed() >= GONE_BOUND {
                    return Err(format!(
                        "the survivor {pid} is still present after {GONE_BOUND:?}"
                    ));
                }
                thread::sleep(WATCH_POLL);
            }
        }
    }
}
