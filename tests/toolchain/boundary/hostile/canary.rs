//! The canary: report the security state the child observes from inside.
//!
//! The canary exists because presence is not enforcement. A `bwrap` set-up
//! failure looks like an ordinary child failure, and systemd accepts limit
//! properties it cannot apply, so the platform check needs an observation from
//! inside the boundary. The canary prints one `polean-canary` line per fact, and
//! `boundary/canary.rs` parses them. Nothing here decides whether the state is
//! acceptable; that is the platform check's job.

use std::{
    net::{SocketAddr, TcpStream},
    path::Path,
    time::Duration,
};

use super::report;
use crate::ambient;

/// The outbound address the canary tries; a literal, so no resolver is needed.
const OUTBOUND: &str = "1.1.1.1:53";

/// How long the outbound attempt may take.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// Report the canary's observations.
///
/// # Errors
///
/// Returns a diagnostic when `/proc` cannot be read.
pub fn run() -> Result<(), String> {
    let status = ambient::read_to_string(Path::new("/proc/self/status"))
        .map_err(|error| format!("cannot read /proc/self/status: {error}"))?;
    field("nonewprivs", status_field(&status, "NoNewPrivs"));
    field("capeff", status_field(&status, "CapEff"));
    for name in interfaces()? {
        report(&format!("polean-canary iface {name}"));
    }
    match outbound() {
        Ok(()) => report("polean-canary outbound connected"),
        Err(reason) => report(&format!("polean-canary outbound {reason}")),
    }
    let fds = descriptors()?;
    let listing: Vec<String> = fds.iter().map(ToString::to_string).collect();
    report(&format!("polean-canary fds {}", listing.join(",")));
    Ok(())
}

/// Report one optional status field.
fn field(name: &str, observed: Option<String>) {
    if let Some(seen) = observed {
        report(&format!("polean-canary {name} {seen}"));
    }
}

/// One field of a `/proc/self/status` document.
#[must_use]
pub fn status_field(status: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    status
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(|value| value.trim().to_owned())
}

/// The network interfaces visible from inside.
///
/// `/proc/net/dev` is read rather than `if_nameindex`, which needs an unsafe
/// foreign-function call this repository forbids.
///
/// # Errors
///
/// Returns a diagnostic when `/proc/net/dev` cannot be read.
pub fn interfaces() -> Result<Vec<String>, String> {
    let text = ambient::read_to_string(Path::new("/proc/net/dev"))
        .map_err(|error| format!("cannot read /proc/net/dev: {error}"))?;
    let names = text
        .lines()
        .skip(2)
        .filter_map(|line| line.split_once(':'))
        .map(|(name, _rest)| name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .collect();
    Ok(names)
}

/// Try one outbound TCP connection; `Ok(())` means it reached the network.
///
/// # Errors
///
/// Returns the connection error's text, which is the observation.
pub fn outbound() -> Result<(), String> {
    let address = OUTBOUND
        .parse::<SocketAddr>()
        .map_err(|error| format!("the outbound address is invalid: {error}"))?;
    match TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) {
        Ok(_stream) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

/// The descriptors the child inherited, as numbers.
///
/// Listing `/proc/self/fd` opens a descriptor for the directory, which appears
/// in that listing and is closed when the listing ends. Two listings therefore
/// differ in exactly those transient descriptors, so their intersection is the
/// set that was open before either began. The link targets are never read:
/// a capability refuses an absolute link target, and `/proc` entries are all
/// absolute.
///
/// # Errors
///
/// Returns a diagnostic when `/proc/self/fd` cannot be listed.
pub fn descriptors() -> Result<Vec<i32>, String> {
    let first = listing()?;
    let second = listing()?;
    let mut fds: Vec<i32> = first.into_iter().filter(|fd| second.contains(fd)).collect();
    fds.sort_unstable();
    Ok(fds)
}

/// One numeric listing of `/proc/self/fd`.
fn listing() -> Result<Vec<i32>, String> {
    let names = ambient::read_dir_names(Path::new("/proc/self/fd"))
        .map_err(|error| format!("cannot list /proc/self/fd: {error}"))?;
    Ok(names.iter().filter_map(|name| name.parse().ok()).collect())
}
