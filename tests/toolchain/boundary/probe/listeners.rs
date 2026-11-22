//! The host listeners the offline probes try to reach.
//!
//! Three endpoints cover the three ways a job could reach the host: a TCP
//! address on the host's loopback interface, an abstract Unix socket whose name
//! lives in the host's network namespace, and a Unix socket file on the host
//! filesystem. A network namespace isolates the first two; only an allowlisted
//! filesystem view hides the third.
//!
//! The listeners only ever report whether a connection arrived, or whether the
//! listener itself failed to observe. They stay active from before the job
//! starts until the job has finished, so a slow launch cannot turn "the job
//! reached a closed listener" into a vacuous pass. Under the selected boundary
//! nothing may connect; under the negative control all three must. The socket
//! file's path must stay short enough for `SUN_LEN`, which the scratch
//! directory's absolute path can exceed, so it lives under `/tmp` with a
//! per-run name and is removed when the watchers finish.

use std::{
    io,
    net::TcpListener,
    os::{
        linux::net::SocketAddrExt,
        unix::net::{SocketAddr, UnixListener},
    },
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::ambient;

/// The longest a listener waits before reporting that it observed nothing.
///
/// This is an overall bound on the probe, not the job's lifetime: the watchers
/// stop when the probe has finished with them. It is longer than every job
/// deadline the offline probes use, so an expired watcher never stands in for
/// isolation.
const WATCH_BOUND: Duration = Duration::from_secs(30);

/// How often a listener polls for a connection or a stop request.
const POLL: Duration = Duration::from_millis(20);

/// The three host endpoints, as target-token values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoints {
    /// The loopback TCP address.
    pub tcp: String,
    /// The abstract Unix socket name.
    pub abstract_name: String,
    /// The Unix socket file's path.
    pub socket: PathBuf,
}

impl Endpoints {
    /// One `polean-target=` token per endpoint.
    #[must_use]
    pub fn targets(&self) -> Vec<String> {
        vec![
            format!("polean-target=tcp:{}", self.tcp),
            format!("polean-target=abstract:{}", self.abstract_name),
            format!("polean-target=socket:{}", self.socket.display()),
        ]
    }
}

/// One listener's observation.
type Observed = Result<bool, String>;

/// The listeners' watcher threads.
pub struct Watchers {
    /// The loopback TCP watcher.
    tcp: JoinHandle<Observed>,
    /// The abstract socket watcher.
    abstract_socket: JoinHandle<Observed>,
    /// The socket-file watcher.
    socket_file: JoinHandle<Observed>,
    /// The socket file's path, removed when the watchers finish.
    socket_path: PathBuf,
    /// Set once the probe has finished with the listeners.
    stop: Arc<AtomicBool>,
}

/// Start all three listeners with a per-run abstract name and socket path.
///
/// # Errors
///
/// Returns a diagnostic when an endpoint cannot be bound.
pub fn listen(nonce: &str) -> Result<(Endpoints, Watchers), String> {
    let tcp = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("cannot listen on loopback: {error}"))?;
    let tcp_address = tcp
        .local_addr()
        .map_err(|error| format!("cannot read the loopback port: {error}"))?;
    tcp.set_nonblocking(true)
        .map_err(|error| format!("cannot make the TCP listener non-blocking: {error}"))?;
    let stop = Arc::new(AtomicBool::new(false));
    let tcp_watch = watch(stop.clone(), move || tcp.accept().map(drop));
    let abstract_name = format!("polean-probe-{nonce}");
    let abstract_address = SocketAddr::from_abstract_name(abstract_name.as_bytes())
        .map_err(|error| format!("cannot name the abstract socket: {error}"))?;
    let abstract_listener = UnixListener::bind_addr(&abstract_address)
        .map_err(|error| format!("cannot bind the abstract socket: {error}"))?;
    abstract_listener
        .set_nonblocking(true)
        .map_err(|error| format!("cannot make the abstract listener non-blocking: {error}"))?;
    let abstract_watch = watch(stop.clone(), move || abstract_listener.accept().map(drop));
    let socket_path = PathBuf::from(format!("/tmp/polean-probe-{nonce}.sock"));
    let socket_listener = UnixListener::bind(&socket_path)
        .map_err(|error| format!("cannot bind {}: {error}", socket_path.display()))?;
    socket_listener
        .set_nonblocking(true)
        .map_err(|error| format!("cannot make the socket listener non-blocking: {error}"))?;
    let socket_watch = watch(stop.clone(), move || socket_listener.accept().map(drop));
    Ok((
        Endpoints {
            tcp: tcp_address.to_string(),
            abstract_name,
            socket: socket_path.clone(),
        },
        Watchers {
            tcp: tcp_watch,
            abstract_socket: abstract_watch,
            socket_file: socket_watch,
            socket_path,
            stop,
        },
    ))
}

impl Watchers {
    /// Require every listener to have recorded a connection.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic naming the listener that stayed silent or failed.
    pub fn require_connected(self) -> Result<(), String> { self.finish(true) }

    /// Require no listener to have recorded a connection.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic naming the listener that recorded one or failed.
    pub fn require_silent(self) -> Result<(), String> { self.finish(false) }

    /// Stop the watchers, join them, and check the expected connection state.
    fn finish(self, expected: bool) -> Result<(), String> {
        self.stop.store(true, Ordering::Relaxed);
        let outcome = [
            self.tcp.join(),
            self.abstract_socket.join(),
            self.socket_file.join(),
        ];
        drop(ambient::remove_file(&self.socket_path));
        for (handle, name) in outcome.into_iter().zip(["TCP", "abstract", "socket-file"]) {
            let Ok(observed) = handle else {
                return Err(format!("the {name} listener thread panicked"));
            };
            let connected = observed?;
            if connected != expected {
                return Err(mismatch(expected, name));
            }
        }
        Ok(())
    }
}

/// The diagnostic for a listener whose connection state is not the expected one.
fn mismatch(expected: bool, name: &str) -> String {
    if expected {
        format!("the {name} listener recorded no connection")
    } else {
        format!("the {name} listener recorded a connection")
    }
}

/// Watch until a connection arrives, the probe stops the watcher, or it fails.
fn watch(
    stop: Arc<AtomicBool>,
    accept: impl Fn() -> io::Result<()> + Send + 'static,
) -> JoinHandle<Observed> {
    thread::spawn(move || {
        let started = Instant::now();
        while !stop.load(Ordering::Relaxed) && started.elapsed() < WATCH_BOUND {
            match accept() {
                Ok(()) => return Ok(true),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(POLL);
                }
                Err(error) => return Err(format!("the listener failed: {error}")),
            }
        }
        Ok(false)
    })
}
