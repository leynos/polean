//! The offline behaviour: try to reach host listeners and list descriptors.
//!
//! Three listeners cover the three ways a job could reach the host: a TCP
//! address on the host's loopback interface, an abstract Unix socket, and a Unix
//! socket file the probe created. A network namespace isolates the first two;
//! only the allowlisted filesystem view hides the third, which is why the probe
//! tests all three.
//!
//! Every target is supplied by the probe as an argument. The child never guesses
//! a path, so an accidental run cannot connect to a real host service.

use std::{
    io,
    net::TcpStream,
    os::{
        linux::net::SocketAddrExt,
        unix::net::{SocketAddr, UnixStream},
    },
    path::Path,
    time::Duration,
};

use super::{Targets, attempt, canary};

/// How long one connection attempt may take.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// Try every configured target and report each outcome.
///
/// # Errors
///
/// Returns only a diagnostic the behaviour itself cannot report; every attempt's
/// failure is reported as an observation instead.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every behaviour shares the dispatch signature so a failure is reported the same way"
)]
pub fn run(targets: &Targets) -> Result<(), String> {
    if let Some(address) = &targets.tcp {
        report_result("tcp", tcp(address));
    }
    if let Some(name) = &targets.abstract_name {
        report_result("abstract", abstract_socket(name));
    }
    if let Some(path) = &targets.socket {
        report_result("socket-file", unix(path));
    }
    match canary::descriptors() {
        Ok(fds) => {
            let listing: Vec<String> = fds.iter().map(ToString::to_string).collect();
            attempt("descriptors", &listing.join(","));
        }
        Err(reason) => attempt("descriptors", &format!("error: {reason}")),
    }
    Ok(())
}

/// Report a connection attempt: `connected` or the error it produced.
fn report_result(name: &str, result: Result<(), String>) {
    match result {
        Ok(()) => attempt(name, "connected"),
        Err(reason) => attempt(name, &format!("error: {reason}")),
    }
}

/// Reach the host's loopback TCP listener at `address`.
fn tcp(address: &str) -> Result<(), String> {
    let target = address
        .parse()
        .map_err(|error| format!("the address is invalid: {error}"))?;
    TcpStream::connect_timeout(&target, CONNECT_TIMEOUT)
        .map(drop)
        .map_err(|error| error.to_string())
}

/// Reach the abstract Unix socket called `name`.
fn abstract_socket(name: &str) -> Result<(), String> {
    let address = SocketAddr::from_abstract_name(name.as_bytes())
        .map_err(|error| format!("the abstract name is invalid: {error}"))?;
    UnixStream::connect_addr(&address)
        .map(drop)
        .map_err(|error| error.to_string())
}

/// Reach the Unix socket file at `path`.
fn unix(path: &Path) -> Result<(), String> {
    UnixStream::connect(path)
        .map(drop)
        .map_err(|error| io_error_text(&error))
}

/// Render an I/O error with its kind, which is what the probe matches.
///
/// The error's `Display` text for a Unix socket failure includes the path on
/// some platforms and not others, so the kind is appended explicitly.
fn io_error_text(error: &io::Error) -> String { format!("{error} ({:?})", error.kind()) }
