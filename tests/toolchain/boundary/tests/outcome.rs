//! The outcome classification and the fail-closed verdict, enumerated.
//!
//! `classify` is a finite product of observations and `verdict` a finite product
//! of control availabilities, so both are decided exhaustively here rather than
//! sampled.

use super::{
    super::outcome::{
        Availability,
        Bound,
        Bounds,
        ControlKind,
        ExitForm,
        Observation,
        Outcome,
        Verdict,
        classify,
        verdict,
    },
    support::{Case, bound_sets, completion_cases, exit_forms, missing_programs},
};
use crate::scratch::{check, check_eq};

/// Check the completion rule for one combination of observations.
fn check_completion_rule(case: &Case) -> Result<(), String> {
    let observation = Observation {
        bounds: case.bounds,
        exit: case.exit,
        cgroup_empty: case.cgroup_empty,
        survivors: case.survivors,
        unavailable: case.unavailable.clone(),
    };
    let completed = matches!(classify(&observation), Outcome::Completed);
    let may_complete = case.bounds.is_empty()
        && case.cgroup_empty
        && !case.survivors
        && case.exit == ExitForm::Exited(0)
        && case.unavailable.is_empty();
    check_eq!(
        completed,
        may_complete,
        "classify({observation:?}) must complete only when every condition holds"
    );
    Ok(())
}
/// Classify never reports completion unless every condition holds.
#[test]
fn classify_never_completes_under_failure() -> Result<(), String> {
    for case in completion_cases() {
        check_completion_rule(&case)?;
    }
    Ok(())
}

/// Check one containment observation.
fn check_containment(
    bounds: Bounds,
    exit: ExitForm,
    cgroup_empty: bool,
    survivors: bool,
) -> Result<(), String> {
    let observation = Observation {
        bounds,
        exit,
        cgroup_empty,
        survivors,
        unavailable: Vec::new(),
    };
    match classify(&observation) {
        Outcome::ContainmentFailure(listed) => check_eq!(
            listed,
            bounds.listed(),
            "the containment failure must list every fired bound"
        ),
        other => {
            return Err(format!(
                "a surviving process must outrank {observation:?}, got {other:?}"
            ));
        }
    }
    Ok(())
}

/// A surviving process makes the outcome a containment failure, whatever else
/// happened, and the bounds are still listed.
#[test]
fn containment_failure_outranks_every_other_observation() -> Result<(), String> {
    for bounds in bound_sets() {
        for exit in exit_forms() {
            for (cgroup_empty, survivors) in [(false, false), (true, true)] {
                check_containment(bounds, exit, cgroup_empty, survivors)?;
            }
        }
    }
    Ok(())
}

/// Every bound that fired is listed once, in the fixed order.
#[test]
fn every_fired_bound_is_listed() -> Result<(), String> {
    for bounds in bound_sets().into_iter().filter(|bounds| !bounds.is_empty()) {
        let observation = Observation {
            bounds,
            exit: ExitForm::Signalled(9),
            cgroup_empty: true,
            survivors: false,
            unavailable: Vec::new(),
        };
        match classify(&observation) {
            Outcome::BoundsExhausted(listed) => {
                check_eq!(listed, bounds.listed(), "the outcome must list every bound");
                check_eq!(
                    listed.len(),
                    bounds.count(),
                    "no bound may be listed twice or omitted"
                );
            }
            other => {
                return Err(format!(
                    "a fired bound must be reported for {observation:?}, got {other:?}"
                ));
            }
        }
    }
    Ok(())
}

/// A memory kill and a deadline together are both named, in order.
#[test]
fn a_memory_kill_and_a_deadline_are_both_named() -> Result<(), String> {
    let mut bounds = Bounds::none();
    bounds.insert(Bound::Deadline);
    bounds.insert(Bound::Memory);
    let observation = Observation {
        bounds,
        exit: ExitForm::Signalled(9),
        cgroup_empty: true,
        survivors: false,
        unavailable: Vec::new(),
    };
    check_eq!(
        classify(&observation),
        Outcome::BoundsExhausted(vec![Bound::Memory, Bound::Deadline]),
        "both bounds are named, in Bound order"
    );
    Ok(())
}

/// A clean exit with a surviving process is still a containment failure.
#[test]
fn a_clean_exit_with_a_survivor_is_a_containment_failure() -> Result<(), String> {
    let observation = Observation {
        bounds: Bounds::none(),
        exit: ExitForm::Exited(0),
        cgroup_empty: false,
        survivors: false,
        unavailable: Vec::new(),
    };
    check_eq!(
        classify(&observation),
        Outcome::ContainmentFailure(Vec::new()),
        "a survivor outranks a clean exit"
    );
    Ok(())
}

/// A missing control outranks everything, because no job should have started.
#[test]
fn an_unavailable_platform_outranks_every_observation() -> Result<(), String> {
    let mut bounds = Bounds::none();
    bounds.insert(Bound::Cancelled);
    let observation = Observation {
        bounds,
        exit: ExitForm::Signalled(9),
        cgroup_empty: false,
        survivors: true,
        unavailable: vec![missing_programs()],
    };
    check_eq!(
        classify(&observation),
        Outcome::Unavailable(vec![missing_programs()]),
        "a missing control outranks a survivor and a bound"
    );
    Ok(())
}

/// The verdict is ready exactly when all five controls are available.
#[test]
fn the_verdict_is_ready_exactly_when_every_control_holds() -> Result<(), String> {
    for mask in 0u8..32 {
        let controls: Vec<(ControlKind, Availability)> = ControlKind::ALL
            .iter()
            .enumerate()
            .map(|(index, kind)| {
                if mask & (1u8 << index) == 0 {
                    (*kind, Availability::Available)
                } else {
                    (
                        *kind,
                        Availability::Missing(format!("missing {}", kind.as_str())),
                    )
                }
            })
            .collect();
        let decision = verdict(&controls);
        check_eq!(
            matches!(decision, Verdict::Ready),
            mask == 0,
            "only an all-available platform may be ready"
        );
        if let Verdict::Unavailable(missing) = decision {
            let expected: Vec<ControlKind> = ControlKind::ALL
                .iter()
                .copied()
                .enumerate()
                .filter(|(index, _kind)| mask & (1u8 << *index) != 0)
                .map(|(_index, kind)| kind)
                .collect();
            check_eq!(
                missing
                    .iter()
                    .map(|control| control.kind)
                    .collect::<Vec<_>>(),
                expected,
                "every missing control must be named, in order"
            );
        }
    }
    Ok(())
}

/// A control the check never reported is missing, not assumed available.
#[test]
fn the_verdict_fails_closed_when_a_control_is_unreported() -> Result<(), String> {
    let missing = match verdict(&[]) {
        Verdict::Unavailable(missing) => missing,
        Verdict::Ready => return Err("an empty control list must not be ready".to_owned()),
    };
    check_eq!(
        missing.len(),
        ControlKind::ALL.len(),
        "every control is named"
    );
    check!(
        missing
            .iter()
            .all(|control| control.reason.contains("did not report")),
        "each absence must be explained, got {missing:?}"
    );
    Ok(())
}

/// Missing controls are listed in control order whatever the input order.
#[test]
fn the_verdict_orders_missing_controls() -> Result<(), String> {
    let controls = vec![
        (
            ControlKind::CanaryLimits,
            Availability::Missing("limits".to_owned()),
        ),
        (ControlKind::Programs, Availability::Available),
        (
            ControlKind::Delegation,
            Availability::Missing("delegation".to_owned()),
        ),
    ];
    let missing = match verdict(&controls) {
        Verdict::Unavailable(missing) => missing,
        Verdict::Ready => return Err("missing controls must not be ready".to_owned()),
    };
    let kinds: Vec<ControlKind> = missing.iter().map(|control| control.kind).collect();
    check_eq!(
        kinds,
        vec![
            ControlKind::UserManager,
            ControlKind::Delegation,
            ControlKind::CanaryIsolation,
            ControlKind::CanaryLimits,
        ],
        "missing controls are ordered by kind, and an unreported one is named"
    );
    Ok(())
}
