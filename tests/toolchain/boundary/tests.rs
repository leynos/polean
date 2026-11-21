//! Unit tests for the outcome decisions, the platform check, and the hostile
//! entry contract.
//!
//! The two decisions are enumerated completely: five bound flags, four exit
//! forms, two control-group states, and two platform states for `classify`; and
//! five controls with two states each for `verdict`. Enumerating a finite domain
//! is complete, so a named case cannot hide behind a sampling argument.
//!
//! The platform tests inject missing program paths and canary reports, so they
//! start no job and hold on any host. The one exception is `check`, which also
//! runs the real `systemctl --user is-system-running` when a test calls it;
//! that test asserts only on the controls it injected.

use std::path::Path;

use rstest::rstest;

use super::{
    hostile::{self, Behaviour, Request, TOKEN_PREFIX},
    outcome::{
        Availability,
        Bound,
        Bounds,
        ControlKind,
        ExitForm,
        MissingControl,
        Observation,
        Outcome,
        Verdict,
        classify,
        verdict,
    },
    platform::{
        CanaryReport,
        Check,
        Programs,
        RequestedLimits,
        canary_isolation,
        canary_limits,
        check as platform_check,
        check_programs,
        delegation_from,
        executable,
        manager_state,
        parse_bwrap_version,
        user_manager_cgroup,
    },
};
use crate::scratch::{Scratch, check, check_eq};

/// Every combination of the five bound flags.
fn bound_sets() -> Vec<Bounds> {
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
fn exit_forms() -> [ExitForm; 4] {
    [
        ExitForm::Exited(0),
        ExitForm::Exited(1),
        ExitForm::Signalled(9),
        ExitForm::Unreported,
    ]
}

/// One missing control naming the programs check.
fn missing_programs() -> MissingControl {
    MissingControl {
        kind: ControlKind::Programs,
        reason: "injected".to_owned(),
    }
}

/// Check the completion rule for one combination of observations.
fn check_completion_rule(
    bounds: Bounds,
    exit: ExitForm,
    cgroup_empty: bool,
    unavailable: &[MissingControl],
) -> Result<(), String> {
    let observation = Observation {
        bounds,
        exit,
        cgroup_empty,
        unavailable: unavailable.to_vec(),
    };
    let completed = matches!(classify(&observation), Outcome::Completed);
    let may_complete =
        bounds.is_empty() && cgroup_empty && exit == ExitForm::Exited(0) && unavailable.is_empty();
    check_eq!(
        completed,
        may_complete,
        "classify({observation:?}) must complete only when every condition holds"
    );
    Ok(())
}

/// One platform state per caller, to keep the enumeration readable.
fn platform_states() -> [Vec<MissingControl>; 2] { [Vec::new(), vec![missing_programs()]] }

/// Every exit form paired with both control-group states.
fn exit_states() -> Vec<(ExitForm, bool)> {
    let mut states = Vec::new();
    for exit in exit_forms() {
        for cgroup_empty in [true, false] {
            states.push((exit, cgroup_empty));
        }
    }
    states
}

/// Every combination the completion rule quantifies over.
fn completion_cases() -> Vec<(Bounds, ExitForm, bool, Vec<MissingControl>)> {
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

/// Classify never reports completion unless every condition holds.
#[test]
fn classify_never_completes_under_failure() -> Result<(), String> {
    for (bounds, exit, cgroup_empty, unavailable) in completion_cases() {
        check_completion_rule(bounds, exit, cgroup_empty, &unavailable)?;
    }
    Ok(())
}

/// A surviving process makes the outcome a containment failure, whatever else
/// happened, and the bounds are still listed.
#[test]
fn containment_failure_outranks_every_other_observation() -> Result<(), String> {
    for bounds in bound_sets() {
        for exit in exit_forms() {
            let observation = Observation {
                bounds,
                exit,
                cgroup_empty: false,
                unavailable: Vec::new(),
            };
            match classify(&observation) {
                Outcome::ContainmentFailure(listed) => {
                    check_eq!(
                        listed,
                        bounds.listed(),
                        "the containment failure must list every fired bound"
                    );
                }
                other => {
                    return Err(format!(
                        "a surviving process must outrank {observation:?}, got {other:?}"
                    ));
                }
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

/// A distribution-suffixed bubblewrap release parses.
#[rstest]
#[case::plain("bubblewrap 0.10.0", Some((0, 10, 0)))]
#[case::suffixed("bubblewrap 0.10.0-5.el10_2", Some((0, 10, 0)))]
#[case::newer("bubblewrap 1.2.3", Some((1, 2, 3)))]
#[case::missing_prefix("0.10.0", None)]
#[case::short("bubblewrap 0.10", None)]
#[case::too_long("bubblewrap 0.10.0.1", None)]
fn bubblewrap_versions_parse(#[case] text: &str, #[case] expected: Option<(u32, u32, u32)>) {
    assert_eq!(
        parse_bwrap_version(text),
        expected,
        "the release parser must handle {text:?}"
    );
}

/// A settled manager is accepted, and an unsettled one is not.
#[rstest]
#[case::running("running", true)]
#[case::degraded("degraded", true)]
#[case::starting("starting", false)]
#[case::maintenance("maintenance", false)]
#[case::offline("offline", false)]
#[case::unknown("", false)]
fn manager_states_decide_reachability(#[case] state: &str, #[case] available: bool) {
    assert_eq!(
        matches!(manager_state(state), Availability::Available),
        available,
        "the manager state {state:?} must decide as expected"
    );
}

/// The manager cgroup is the `user@<uid>.service` prefix of the caller's path.
#[rstest]
#[case::user_service(
    "0::/user.slice/user-1000.slice/user@1000.service/app.slice/lody.service\n",
    Some("/user.slice/user-1000.slice/user@1000.service")
)]
#[case::system_service("0::/system.slice/sshd.service\n", None)]
#[case::named_hierarchy("1:name=systemd:/user.slice/x\n", None)]
fn the_manager_cgroup_is_derived_from_proc(
    #[case] proc_cgroup: &str,
    #[case] expected: Option<&str>,
) {
    assert_eq!(
        user_manager_cgroup(proc_cgroup).as_deref(),
        expected,
        "the manager prefix must be read from /proc/self/cgroup"
    );
}

/// Both controllers must be delegated.
#[rstest]
#[case::both("cpu memory pids", true)]
#[case::memory_only("cpu memory", false)]
#[case::pids_only("pids", false)]
#[case::none("cpu", false)]
fn delegation_requires_memory_and_pids(#[case] control: &str, #[case] available: bool) {
    assert_eq!(
        matches!(delegation_from(control), Availability::Available),
        available,
        "cgroup.subtree_control {control:?} must decide as expected"
    );
}

/// The canary report the host should produce.
fn good_report() -> CanaryReport {
    CanaryReport {
        started: true,
        no_new_privs: Some("1".to_owned()),
        cap_eff: Some("0000000000000000".to_owned()),
        interfaces: vec!["lo".to_owned()],
        outbound_error: Some("Network is unreachable (os error 101)".to_owned()),
        descriptors: Some(vec![0, 1, 2]),
        memory_max: Some("67108864".to_owned()),
        memory_swap_max: Some("0".to_owned()),
        pids_max: Some("32".to_owned()),
    }
}

/// The expected report satisfies the isolation control.
#[test]
fn the_expected_report_is_isolated() -> Result<(), String> {
    check_eq!(
        canary_isolation(&good_report()),
        Availability::Available,
        "the expected report must satisfy the isolation control"
    );
    Ok(())
}

/// Reports that fail each isolation requirement.
fn not_started() -> CanaryReport {
    CanaryReport {
        started: false,
        ..good_report()
    }
}

/// A report with new privileges enabled.
fn privileged() -> CanaryReport {
    CanaryReport {
        no_new_privs: Some("0".to_owned()),
        ..good_report()
    }
}

/// A report that kept a capability.
fn capable() -> CanaryReport {
    CanaryReport {
        cap_eff: Some("0000000000000001".to_owned()),
        ..good_report()
    }
}

/// A report that could see a host interface.
fn networked() -> CanaryReport {
    CanaryReport {
        interfaces: vec!["lo".to_owned(), "eth0".to_owned()],
        ..good_report()
    }
}

/// A report whose outbound attempt reached something.
fn connected() -> CanaryReport {
    CanaryReport {
        outbound_error: Some("connected".to_owned()),
        ..good_report()
    }
}

/// A report that inherited an extra descriptor.
fn leaked() -> CanaryReport {
    CanaryReport {
        descriptors: Some(vec![0, 1, 2, 7]),
        ..good_report()
    }
}

/// Each broken isolation requirement is reported with its reason.
#[rstest]
#[case::not_started(not_started, "start line")]
#[case::privileged(privileged, "NoNewPrivs")]
#[case::capable(capable, "capabilities")]
#[case::networked(networked, "interfaces")]
#[case::connected(connected, "Network is unreachable")]
#[case::leaked(leaked, "descriptors")]
fn a_broken_isolation_control_names_its_reason(
    #[case] report: fn() -> CanaryReport,
    #[case] expected: &str,
) -> Result<(), String> {
    let availability = canary_isolation(&report());
    match availability {
        Availability::Missing(reason) => check!(
            reason.contains(expected),
            "the reason {reason:?} must mention {expected:?}"
        ),
        Availability::Available => {
            return Err(format!(
                "the broken report must not be accepted: {expected}"
            ));
        }
    }
    Ok(())
}

/// The scope readback must equal the requested limits.
#[test]
fn the_scope_readback_must_match_the_request() -> Result<(), String> {
    let limits = RequestedLimits::default();
    check_eq!(
        canary_limits(&good_report(), &limits),
        Availability::Available,
        "the expected readback must match"
    );
    let wrong = CanaryReport {
        memory_max: Some("134217728".to_owned()),
        ..good_report()
    };
    match canary_limits(&wrong, &limits) {
        Availability::Missing(reason) => check!(
            reason.contains("memory.max"),
            "the reason must name the field, got {reason:?}"
        ),
        Availability::Available => return Err("a different limit must not be accepted".to_owned()),
    }
    Ok(())
}

/// Missing programs and a failed canary are both named, and no job starts.
#[test]
fn the_check_fails_closed_and_names_what_is_missing() -> Result<(), String> {
    let mut scratch = Scratch::try_new("boundary-missing-programs")
        .map_err(|error| format!("cannot create scratch: {error}"))?;
    let absent = scratch.path().join("absent");
    let programs = Programs {
        systemd_run: absent.join("systemd-run"),
        systemctl: absent.join("systemctl"),
        kill: absent.join("kill"),
        bwrap: absent.join("bwrap"),
    };
    let reason = "the canary could not start".to_owned();
    let limits = RequestedLimits::default();
    let report = platform_check(&Check {
        programs: &programs,
        runtime_dir: Path::new("/run/user/0"),
        canary: Err(reason.clone()),
        limits: &limits,
    });
    let missing = report.missing();
    let kinds: Vec<ControlKind> = missing.iter().map(|control| control.kind).collect();
    check!(
        kinds.contains(&ControlKind::Programs),
        "the missing programs must be named, got {kinds:?}"
    );
    check!(
        kinds.contains(&ControlKind::CanaryIsolation) && kinds.contains(&ControlKind::CanaryLimits),
        "a canary that did not run must miss both canary controls, got {kinds:?}"
    );
    check!(
        missing
            .iter()
            .filter(|control| control.kind == ControlKind::CanaryIsolation)
            .all(|control| control.reason.contains(&reason)),
        "the canary reason must be carried through"
    );
    check!(
        !matches!(report.verdict(), Verdict::Ready),
        "a platform with missing controls must not be ready"
    );
    scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch: {error}"))
}

/// The standard programs are the four distribution paths, and every name of
/// every bound is distinct and complete.
#[test]
fn the_standard_programs_and_names_are_complete() -> Result<(), String> {
    let programs = Programs::standard();
    let listed = programs.listed();
    check_eq!(listed.len(), 4, "four programs are required");
    for (role, path) in listed {
        check!(path.is_absolute(), "the {role} path must be absolute");
        check!(
            path.starts_with("/usr/bin"),
            "the {role} path must be a distribution path"
        );
    }
    let mut names: Vec<&str> = Bound::ALL.into_iter().map(Bound::as_str).collect();
    let before = names.len();
    names.sort_unstable();
    names.dedup();
    check_eq!(names.len(), before, "every bound name must be distinct");
    Ok(())
}

/// A file without an execute bit is not an installed program.
#[test]
fn a_non_executable_file_is_not_a_program() -> Result<(), String> {
    let mut scratch = Scratch::try_new("boundary-not-executable")
        .map_err(|error| format!("cannot create scratch: {error}"))?;
    let path = scratch.path().join("not-executable");
    crate::ambient::write(&path, b"")
        .map_err(|error| format!("cannot write the probe file: {error}"))?;
    crate::ambient::set_mode(&path, 0o644)
        .map_err(|error| format!("cannot set the probe mode: {error}"))?;
    check_eq!(
        executable(&path),
        Ok(false),
        "a file without an execute bit is not a program"
    );
    let programs = Programs {
        systemd_run: path.clone(),
        systemctl: path.clone(),
        kill: path.clone(),
        bwrap: path,
    };
    match check_programs(&programs) {
        Availability::Missing(reason) => check!(
            reason.contains("not executable"),
            "the reason must say the program is not executable, got {reason:?}"
        ),
        Availability::Available => {
            return Err("a non-executable program must not be accepted".to_owned());
        }
    }
    scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch: {error}"))
}

/// A directory and an absent path are both "not a program".
#[test]
fn a_directory_or_absent_path_is_not_a_program() -> Result<(), String> {
    let mut scratch = Scratch::try_new("boundary-not-a-file")
        .map_err(|error| format!("cannot create scratch: {error}"))?;
    check_eq!(
        executable(scratch.path()),
        Ok(false),
        "a directory is not an executable program"
    );
    check_eq!(
        executable(&scratch.path().join("absent")),
        Ok(false),
        "an absent path is not an executable program"
    );
    scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch: {error}"))
}

/// The entry point's exact libtest path is pinned.
#[test]
fn the_hostile_entry_name_is_pinned() -> Result<(), String> {
    check_eq!(
        hostile::entry_name(),
        "toolchain::boundary::hostile::entry".to_owned(),
        "a module move must be noticed here rather than in a silent probe"
    );
    Ok(())
}

/// An argument list without a token is inert.
#[test]
fn an_absent_token_is_inert() -> Result<(), String> {
    let args = vec![
        "toolchain".to_owned(),
        "--exact".to_owned(),
        "--test-threads=1".to_owned(),
    ];
    check_eq!(
        hostile::request(&args),
        Ok(None),
        "no token must mean no behaviour"
    );
    Ok(())
}

/// One well-formed token parses.
#[test]
fn a_token_selects_one_behaviour() -> Result<(), String> {
    let args = vec![format!("{TOKEN_PREFIX}offline:run-7")];
    check_eq!(
        hostile::request(&args),
        Ok(Some(Request {
            behaviour: Behaviour::Offline,
            nonce: "run-7".to_owned(),
        })),
        "the token must select the offline behaviour with its nonce"
    );
    Ok(())
}

/// Two tokens are a probe defect rather than a silent choice.
#[test]
fn a_duplicate_token_is_rejected() -> Result<(), String> {
    let args = vec![
        format!("{TOKEN_PREFIX}offline:run-7"),
        format!("{TOKEN_PREFIX}memory:run-8"),
    ];
    match hostile::request(&args) {
        Err(reason) => check!(
            reason.contains("more than one"),
            "the reason must say the token is duplicated, got {reason:?}"
        ),
        other => return Err(format!("two tokens must be refused, got {other:?}")),
    }
    Ok(())
}

/// Malformed tokens are refused.
#[rstest]
#[case::no_nonce("offline")]
#[case::unknown_behaviour("bogus:run-1")]
#[case::empty_nonce("offline:")]
#[case::bad_nonce("offline:bad/nonce")]
#[case::long_nonce(
    "offline:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
)]
fn a_malformed_token_is_rejected(#[case] token: &str) -> Result<(), String> {
    let args = vec![format!("{TOKEN_PREFIX}{token}")];
    match hostile::request(&args) {
        Err(_reason) => Ok(()),
        other => Err(format!(
            "the malformed token {token:?} must be refused, got {other:?}"
        )),
    }
}

/// Every behaviour name appears in the diagnostic list.
#[test]
fn the_behaviour_names_are_complete() -> Result<(), String> {
    let names = hostile::names();
    for behaviour in Behaviour::ALL {
        check!(
            names.contains(behaviour.as_str()),
            "the name list {names:?} must contain {}",
            behaviour.as_str()
        );
        check_eq!(
            Behaviour::parse(behaviour.as_str()),
            Some(behaviour),
            "every behaviour must parse from its own name"
        );
    }
    Ok(())
}
