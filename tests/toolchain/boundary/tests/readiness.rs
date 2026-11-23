//! The aggregate readiness decision, with an injected canary closure.
//!
//! The presence checks run first, and a missing program must not call the
//! canary closure at all. Once the presence controls hold, a canary failure is
//! folded into the same report, so every path returns one verdict naming every
//! missing control.

use std::path::Path;

use super::super::{
    outcome::{Availability, ControlKind, Verdict},
    platform::{CanaryReport, PlatformReport, Programs, RequestedLimits, complete, readiness},
};
use crate::scratch::{check, check_eq};

/// A canary report showing every required state.
fn good_canary() -> CanaryReport {
    CanaryReport {
        started: true,
        no_new_privs: Some("1".to_owned()),
        cap_eff: Some("0000000000000000".to_owned()),
        interfaces: vec!["lo".to_owned()],
        outbound_error: Some("Connection refused (os error 111)".to_owned()),
        descriptors: Some(vec![0, 1, 2]),
        separate_netns: Some(true),
        memory_max: Some("67108864".to_owned()),
        memory_swap_max: Some("0".to_owned()),
        pids_max: Some("32".to_owned()),
    }
}

/// A presence report with every control available.
fn ready_presence() -> PlatformReport {
    PlatformReport {
        controls: ControlKind::ALL
            .iter()
            .map(|kind| (*kind, Availability::Available))
            .collect(),
    }
}

/// A missing program refuses before the canary closure is called.
#[test]
fn readiness_does_not_run_the_canary_when_a_program_is_missing() -> Result<(), String> {
    let absent = Path::new("/nonexistent/absent");
    let programs = Programs {
        systemd_run: absent.to_path_buf(),
        systemctl: absent.to_path_buf(),
        kill: absent.to_path_buf(),
        bwrap: absent.to_path_buf(),
    };
    let mut called = false;
    let report = readiness(
        &programs,
        Path::new("/nonexistent/runtime"),
        &RequestedLimits::default(),
        || {
            called = true;
            Err("the canary must not run".to_owned())
        },
    );
    check!(!called, "a missing program must not run the canary");
    check!(
        matches!(report.verdict(), Verdict::Unavailable(_)),
        "a missing program must be unavailable, got {:?}",
        report.verdict()
    );
    check_eq!(
        report.controls.len(),
        ControlKind::ALL.len(),
        "the report must account for every control"
    );
    Ok(())
}

/// A failed canary is folded into the same report as the presence controls.
#[test]
fn readiness_folds_a_failed_canary_into_one_verdict() -> Result<(), String> {
    let limits = RequestedLimits::default();
    let mut called = false;
    let report = complete(ready_presence(), &limits, || {
        called = true;
        Err("the canary failed".to_owned())
    });
    check!(called, "a ready presence must run the canary");
    match report.verdict() {
        Verdict::Unavailable(missing) => {
            check!(
                missing.iter().any(|control| {
                    control.kind == ControlKind::CanaryIsolation
                        && control.reason.contains("the canary failed")
                }),
                "the verdict must name the failed canary, got {missing:?}"
            );
        }
        other => {
            return Err(format!(
                "a failed canary must be unavailable, got {other:?}"
            ));
        }
    }
    Ok(())
}

/// A completed canary still decides its two controls.
#[test]
fn readiness_keeps_the_canary_controls_separate() -> Result<(), String> {
    let report = complete(ready_presence(), &RequestedLimits::default(), || {
        Ok(good_canary())
    });
    check_eq!(
        report.verdict(),
        Verdict::Ready,
        "a good canary and a ready presence must be ready"
    );
    Ok(())
}
