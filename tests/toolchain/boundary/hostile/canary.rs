//! The canary: report the security state the child observes from inside.
//!
//! The canary exists because presence is not enforcement. A `bwrap` set-up
//! failure looks like an ordinary child failure, and systemd accepts limit
//! properties it cannot apply, so the platform check needs an observation from
//! inside the boundary. The canary prints one `polean-canary` line per fact, which
//! `report` prefixes like every other report line, and `boundary/canary.rs`
//! parses the prefixed form. Nothing here decides whether the state is
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
