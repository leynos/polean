//! Parsing the canary's report from the child's standard output.
//!
//! The canary is an ordinary hostile job whose behaviour prints one line per
//! observation. This module turns those lines into the [`CanaryReport`] the
//! platform check decides from. The start line must carry the run's nonce, so a
//! report from another run, or from a child that never started, cannot be
//! mistaken for this one.
//!
//! The limit read-back does not come from the canary: the supervisor reads it
//! from the cgroup while the scope is active, because that is the only evidence
//! that systemd applied the requested limits.

use super::{hostile::REPORT_PREFIX, platform::CanaryReport, scope::LimitReadback};

/// The line a canary prints first, before any observation.
#[must_use]
pub fn start_line(nonce: &str) -> String { format!("polean-hostile start {nonce}") }

/// Parse a canary's output.
///
/// A missing field stays `None`, which the platform check treats as a missing
/// control rather than an assumed value.
#[must_use]
pub fn parse(stdout: &str, nonce: &str, limits: &LimitReadback) -> CanaryReport {
    let mut report = CanaryReport {
        memory_max: limits.memory.clone(),
        memory_swap_max: limits.swap.clone(),
        pids_max: limits.pids.clone(),
        ..CanaryReport::default()
    };
    let expected = start_line(nonce);
    for line in stdout.lines() {
        if line == expected {
            report.started = true;
            continue;
        }
        let Some(rest) = line
            .strip_prefix(REPORT_PREFIX)
            .and_then(|rest| rest.strip_prefix(" polean-canary "))
        else {
            continue;
        };
        let Some((key, value)) = rest.split_once(' ') else {
            continue;
        };
        match key {
            "nonewprivs" => report.no_new_privs = Some(value.to_owned()),
            "capeff" => report.cap_eff = Some(value.to_owned()),
            "iface" => report.interfaces.push(value.to_owned()),
            "outbound" => report.outbound_error = Some(value.to_owned()),
            _ => (),
        }
    }
    report
}
