//! The fail-closed submission gate, with an observable spawn counter.
//!
//! AC6 requires that a missing control starts no job. These cases pass a
//! counter as the spawn action: it must stay at zero for every non-ready
//! verdict and must increment exactly once for a ready one. A gate that
//! checked the verdict and then called the spawn action anyway would fail
//! here, without starting a real job.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::super::{
    outcome::{Availability, ControlKind, MissingControl, Verdict},
    platform::PlatformReport,
    submit::{Submitted, submit},
};
use crate::scratch::{check, check_eq};

/// A report with every control available.
fn ready() -> PlatformReport {
    PlatformReport {
        controls: ControlKind::ALL
            .iter()
            .map(|kind| (*kind, Availability::Available))
            .collect(),
    }
}

/// A report whose programs control names a missing bubblewrap.
fn missing_bubblewrap() -> PlatformReport {
    PlatformReport {
        controls: ControlKind::ALL
            .iter()
            .map(|kind| {
                let availability = if *kind == ControlKind::Programs {
                    Availability::Missing("bwrap is missing".to_owned())
                } else {
                    Availability::Available
                };
                (*kind, availability)
            })
            .collect(),
    }
}

/// A report whose control list is malformed.
fn malformed() -> PlatformReport {
    PlatformReport {
        controls: vec![
            (ControlKind::Programs, Availability::Available),
            (ControlKind::Programs, Availability::Available),
        ],
    }
}

/// A ready platform runs the spawn action exactly once.
#[test]
fn a_ready_platform_spawns_once() -> Result<(), String> {
    let spawns = AtomicUsize::new(0);
    let submitted = submit(&ready(), || {
        spawns.fetch_add(1, Ordering::SeqCst);
        Ok(())
    })?;
    check!(
        matches!(submitted, Submitted::Ran(())),
        "a ready platform must run the job"
    );
    check_eq!(
        spawns.load(Ordering::SeqCst),
        1,
        "a ready platform must call the spawn action once"
    );
    Ok(())
}

/// Attempt a submission that must be refused, returning its missing controls.
fn refused(report: &PlatformReport) -> Result<Vec<MissingControl>, String> {
    let spawns = AtomicUsize::new(0);
    let submitted = submit(report, || {
        spawns.fetch_add(1, Ordering::SeqCst);
        Ok(())
    })?;
    check!(
        submitted.is_refused(),
        "a non-ready platform must refuse the submission"
    );
    check_eq!(
        spawns.load(Ordering::SeqCst),
        0,
        "a non-ready platform must not call the spawn action"
    );
    let Submitted::Refused(missing) = submitted else {
        return Err("the submission must be refused".to_owned());
    };
    Ok(missing)
}

/// A missing program refuses without running the spawn action.
#[test]
fn a_missing_program_starts_nothing() -> Result<(), String> {
    let missing = refused(&missing_bubblewrap())?;
    check!(
        missing.iter().any(|control| {
            control.kind == ControlKind::Programs && control.reason.contains("bwrap")
        }),
        "the refusal must name bubblewrap, got {missing:?}"
    );
    Ok(())
}

/// A malformed verdict refuses and accounts for every control.
#[test]
fn a_malformed_verdict_starts_nothing() -> Result<(), String> {
    let missing = refused(&malformed())?;
    check_eq!(
        missing.len(),
        ControlKind::ALL.len(),
        "a malformed verdict must name every control"
    );
    Ok(())
}

/// The verdict helpers agree with the submitted state.
#[test]
fn the_refusal_matches_the_verdict() -> Result<(), String> {
    check!(
        missing_bubblewrap().verdict() != Verdict::Ready,
        "the fixture must not be ready"
    );
    check!(
        matches!(malformed().verdict(), Verdict::Malformed(_)),
        "the fixture must be malformed"
    );
    Ok(())
}
