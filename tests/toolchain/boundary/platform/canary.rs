//! The canary's two controls, decided from what the canary reported.
//!
//! The canary is a job run through the full launch chain that reports the
//! security state it observes from inside. Its report is the only evidence that
//! the mechanisms took effect rather than merely accepting their arguments: a
//! `bwrap` set-up failure would otherwise look like an ordinary child failure,
//! and systemd accepts limit properties it cannot apply.
//!
//! Every required field is `Option`al in [`CanaryReport`], so a field the canary
//! never reported is a missing control rather than an assumed value.

use super::{super::outcome::Availability, CanaryReport, RequestedLimits};

/// Controls 4 and 5, from the canary's report and the requested limits.
#[must_use]
pub fn evaluate_canary(
    report: &CanaryReport,
    limits: &RequestedLimits,
) -> (Availability, Availability) {
    (canary_isolation(report), canary_limits(report, limits))
}

/// Control 4: the canary saw the isolation the boundary promises.
#[must_use]
pub fn canary_isolation(report: &CanaryReport) -> Availability {
    if !report.started {
        return Availability::Missing("the canary did not print its start line".to_owned());
    }
    if report.no_new_privs.as_deref() != Some("1") {
        return Availability::Missing(format!(
            "the canary reported NoNewPrivs {:?}, not 1",
            report.no_new_privs
        ));
    }
    let Some(caps) = report.cap_eff.as_deref() else {
        return Availability::Missing("the canary reported no CapEff".to_owned());
    };
    match u64::from_str_radix(caps, 16) {
        Ok(0) => (),
        Ok(_) => {
            return Availability::Missing(format!(
                "the canary retained capabilities: CapEff {caps}"
            ));
        }
        Err(error) => {
            return Availability::Missing(format!("CapEff {caps:?} is not hexadecimal: {error}"));
        }
    }
    if report.interfaces.as_slice() != ["lo"] {
        return Availability::Missing(format!(
            "the canary saw interfaces {:?}, not only loopback",
            report.interfaces
        ));
    }
    match report.outbound_error.as_deref() {
        Some(error) if error.contains("Network is unreachable") => (),
        other => {
            return Availability::Missing(format!(
                "an outbound connection reported {other:?}, not `Network is unreachable`"
            ));
        }
    }
    match &report.descriptors {
        Some(descriptors) if descriptors.as_slice() == [0, 1, 2] => Availability::Available,
        Some(descriptors) => Availability::Missing(format!(
            "the canary inherited descriptors {descriptors:?}, not only 0, 1, and 2"
        )),
        None => Availability::Missing("the canary reported no descriptor listing".to_owned()),
    }
}

/// Control 5: the active scope read back the limits the probe requested.
#[must_use]
pub fn canary_limits(report: &CanaryReport, limits: &RequestedLimits) -> Availability {
    let [memory, swap, tasks] = limits.expected_readback();
    let observed = [
        ("memory.max", report.memory_max.as_deref()),
        ("memory.swap.max", report.memory_swap_max.as_deref()),
        ("pids.max", report.pids_max.as_deref()),
    ];
    let expected = [memory.as_str(), swap.as_str(), tasks.as_str()];
    for ((name, seen), want) in observed.into_iter().zip(expected) {
        match seen {
            Some(value) if value == want => (),
            other => {
                return Availability::Missing(format!(
                    "{name} read back {other:?}, not the requested {want:?}"
                ));
            }
        }
    }
    Availability::Available
}
