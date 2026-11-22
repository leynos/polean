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
//! reached a closed listener" into a vacuous pass. A watcher that reaches its
//! own bound is an observation failure, never silence: incomplete observation
//! is not evidence of isolation.
//!
//! Setup failure and an early return from a probe both release the listeners
//! through the same guard, so no watcher outlives the probe that started it.
//! The socket file's path must stay short enough for `SUN_LEN`, which the
//! scratch directory's absolute path can exceed, so it lives under `/tmp` with
//! a per-run name.

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
/// Reaching this bound is an observation failure: the probe cannot say whether
/// a connection would have arrived later, so silence at the bound is never
/// evidence.
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

/// One listener's observation: whether a connection arrived, or why it failed.
type Observed = Result<bool, String>;

/// The listeners' watcher threads.
pub struct Watchers {
    /// The watcher threads and their endpoint names.
    started: Vec<(JoinHandle<Observed>, &'static str)>,
    /// The socket file's path, removed when the watchers finish.
    socket: Option<PathBuf>,
    /// Set once the probe has finished with the listeners.
    stop: Arc<AtomicBool>,
}

/// Start all three listeners with a per-run abstract name and socket path.
///
/// Watcher threads start as each endpoint binds, and every endpoint already
/// started is stopped and joined if a later one fails, so a partial setup
/// leaves nothing running.
///
/// # Errors
///
/// Returns a diagnostic when an endpoint cannot be bound.
pub fn listen(nonce: &str) -> Result<(Endpoints, Watchers), String> {
    let mut guard = Watchers {
        started: Vec::new(),
        socket: None,
        stop: Arc::new(AtomicBool::new(false)),
    };
    let (endpoints, socket) = bind_all(nonce, &mut guard)?;
    guard.socket = Some(socket);
    Ok((endpoints, guard))
}

/// Bind every endpoint, starting each watcher as soon as it is ready.
fn bind_all(nonce: &str, guard: &mut Watchers) -> Result<(Endpoints, PathBuf), String> {
    let tcp = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("cannot listen on loopback: {error}"))?;
    let tcp_address = tcp
        .local_addr()
        .map_err(|error| format!("cannot read the loopback port: {error}"))?;
    tcp.set_nonblocking(true)
        .map_err(|error| format!("cannot make the TCP listener non-blocking: {error}"))?;
    guard.push("TCP", move || tcp.accept().map(drop));
    let abstract_name = format!("polean-probe-{nonce}");
    let abstract_address = SocketAddr::from_abstract_name(abstract_name.as_bytes())
        .map_err(|error| format!("cannot name the abstract socket: {error}"))?;
    let abstract_listener = UnixListener::bind_addr(&abstract_address)
        .map_err(|error| format!("cannot bind the abstract socket: {error}"))?;
    abstract_listener
        .set_nonblocking(true)
        .map_err(|error| format!("cannot make the abstract listener non-blocking: {error}"))?;
    guard.push("abstract", move || abstract_listener.accept().map(drop));
    let socket_path = PathBuf::from(format!("/tmp/polean-probe-{nonce}.sock"));
    let socket_listener = UnixListener::bind(&socket_path)
        .map_err(|error| format!("cannot bind {}: {error}", socket_path.display()))?;
    socket_listener
        .set_nonblocking(true)
        .map_err(|error| format!("cannot make the socket listener non-blocking: {error}"))?;
    guard.socket = Some(socket_path.clone());
    guard.push("socket-file", move || socket_listener.accept().map(drop));
    Ok((
        Endpoints {
            tcp: tcp_address.to_string(),
            abstract_name,
            socket: socket_path.clone(),
        },
        socket_path,
    ))
}

impl Watchers {
    /// Start one watcher for a bound endpoint.
    fn push(&mut self, name: &'static str, accept: impl Fn() -> io::Result<()> + Send + 'static) {
        let stop = self.stop.clone();
        self.started.push((watch(stop, accept), name));
    }

    /// Require every listener to have recorded a connection.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic naming the listener that stayed silent or failed.
    pub fn require_connected(mut self) -> Result<(), String> { self.finish(true) }

    /// Require no listener to have recorded a connection.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic naming the listener that recorded one or failed.
    pub fn require_silent(mut self) -> Result<(), String> { self.finish(false) }

    /// Stop the watchers, join them, and check the expected connection state.
    fn finish(&mut self, expected: bool) -> Result<(), String> {
        self.stop.store(true, Ordering::Relaxed);
        let outcome = std::mem::take(&mut self.started);
        let mut result = Ok(());
        for (handle, name) in outcome {
            if result.is_err() {
                drop(handle.join());
                continue;
            }
            result = check(handle, name, expected);
        }
        result.as_ref()?;
        self.remove_socket()
    }

    /// Remove the socket file, reporting a failure to do so.
    fn remove_socket(&mut self) -> Result<(), String> {
        self.socket.take().map_or(Ok(()), |path| {
            ambient::remove_file(&path)
                .map_err(|error| format!("cannot remove {}: {error}", path.display()))
        })
    }

    /// Stop and join every watcher and remove the socket file, ignoring errors.
    fn shutdown(&mut self) {
        for (handle, _name) in self.started.drain(..) {
            drop(handle.join());
        }
        drop(self.remove_socket());
    }
}

impl Drop for Watchers {
    fn drop(&mut self) { self.shutdown(); }
}

/// Check one watcher's observation.
fn check(handle: JoinHandle<Observed>, name: &str, expected: bool) -> Result<(), String> {
    let Ok(observed) = handle.join() else {
        return Err(format!("the {name} listener thread panicked"));
    };
    let connected = observed?;
    if connected != expected {
        return Err(mismatch(expected, name));
    }
    Ok(())
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
///
/// A stop request still performs one final non-blocking `accept`, so a
/// connection that reached the backlog between polls is reported rather than
/// mistaken for silence.
fn watch(
    stop: Arc<AtomicBool>,
    accept: impl Fn() -> io::Result<()> + Send + 'static,
) -> JoinHandle<Observed> {
    thread::spawn(move || {
        let started = Instant::now();
        loop {
            match accept() {
                Ok(()) => return Ok(true),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => (),
                Err(error) => return Err(format!("the listener failed: {error}")),
            }
            if stop.load(Ordering::Relaxed) {
                // The final accept above found nothing; silence is observed.
                return Ok(false);
            }
            if started.elapsed() >= WATCH_BOUND {
                return Err(format!(
                    "the listener reached its {WATCH_BOUND:?} bound before the probe stopped it"
                ));
            }
            thread::sleep(POLL);
        }
    })
}
