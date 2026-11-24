//! The platform controls, driven through injected paths and reports.
//!
//! These tests start no job. The parsers are exercised on synthetic texts, the
//! programs check on scratch paths, and the canary entries from reports the
//! canary could have produced; no boundary process runs.

use std::path::Path;

use rstest::rstest;

use super::super::{
    outcome::{Availability, Bound, ControlKind, Verdict},
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
        separate_netns: Some(true),
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

/// A report whose job shares the supervisor's network namespace.
fn shared_netns() -> CanaryReport {
    CanaryReport {
        separate_netns: Some(false),
        ..good_report()
    }
}

/// A report whose network namespace was not observed.
fn unknown_netns() -> CanaryReport {
    CanaryReport {
        separate_netns: None,
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
#[case::connected(connected, "not refusal or unreachability")]
#[case::shared_netns(shared_netns, "network namespace")]
#[case::unknown_netns(unknown_netns, "network namespace")]
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
        delegation: Ok(Availability::Available),
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

/// Control 3 is decided from an injected observation, not a host read.
#[test]
fn delegation_is_injected_into_the_platform_decision() -> Result<(), String> {
    let absent = Path::new("/nonexistent/absent");
    let programs = Programs {
        systemd_run: absent.to_path_buf(),
        systemctl: absent.to_path_buf(),
        kill: absent.to_path_buf(),
        bwrap: absent.to_path_buf(),
    };
    let limits = RequestedLimits::default();
    let report = platform_check(&Check {
        programs: &programs,
        runtime_dir: Path::new("/nonexistent/runtime"),
        canary: Err("not run".to_owned()),
        delegation: Err("the injected delegation failure".to_owned()),
        limits: &limits,
    });
    let delegation = report
        .controls
        .iter()
        .find(|(kind, _)| *kind == ControlKind::Delegation)
        .map(|(_, availability)| availability.clone());
    match delegation {
        Some(Availability::Missing(reason)) if reason.contains("injected delegation failure") => {
            Ok(())
        }
        other => Err(format!(
            "the injected delegation observation must be reported, got {other:?}"
        )),
    }
}
