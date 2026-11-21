//! Shared enumerations for the boundary's unit tests.
//!
//! Every helper here builds a finite domain the tests quantify over, so the
//! coverage is visible in one place rather than spread across test bodies.

use super::super::outcome::{Bound, Bounds, ControlKind, ExitForm, MissingControl};

/// Every combination of the five bound flags.
pub(super) fn bound_sets() -> Vec<Bounds> {
    let mut sets = Vec::new();
    for mask in 0u8..32 {
        let mut bounds = Bounds::none();
        for bound in Bound::ALL {
            if mask & (1 << bound.index()) != 0 {
                bounds.insert(bound);
            }
        }
        sets.push(bounds);
    }
    sets
}

/// The exit forms a directly spawned process can report.
pub(super) fn exit_forms() -> [ExitForm; 4] {
    [
        ExitForm::Exited(0),
        ExitForm::Exited(1),
        ExitForm::Signalled(9),
        ExitForm::Unreported,
    ]
}

/// One missing control naming the programs check.
pub(super) fn missing_programs() -> MissingControl {
    MissingControl {
        kind: ControlKind::Programs,
        reason: "injected".to_owned(),
    }
}

/// One platform state per caller, to keep the enumeration readable.
pub(super) fn platform_states() -> [Vec<MissingControl>; 2] {
    [Vec::new(), vec![missing_programs()]]
}

/// Every exit form paired with both control-group states.
pub(super) fn exit_states() -> Vec<(ExitForm, bool)> {
    let mut states = Vec::new();
    for exit in exit_forms() {
        for cgroup_empty in [true, false] {
            states.push((exit, cgroup_empty));
        }
    }
    states
}

/// Every combination the completion rule quantifies over.
pub(super) fn completion_cases() -> Vec<(Bounds, ExitForm, bool, Vec<MissingControl>)> {
    let mut cases = Vec::new();
    for bounds in bound_sets() {
        for (exit, cgroup_empty) in exit_states() {
            for unavailable in platform_states() {
                cases.push((bounds, exit, cgroup_empty, unavailable));
            }
        }
    }
    cases
}
