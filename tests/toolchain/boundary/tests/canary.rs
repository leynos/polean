//! Parsing the canary's report, without running a job.
//!
//! These cases pin the contract between what `hostile::canary` prints and what
//! the platform check decides from. A missing start line must leave the report
//! unstarted, because that is how a child that never ran is distinguished from
//! one whose isolation merely failed.

use super::super::{
    canary::{parse, parse_descriptors, start_line},
    scope::LimitReadback,
};
use crate::scratch::check_eq;

/// A canary's output as the child prints it.
fn output() -> String {
    [
        start_line("run-1"),
        "polean-canary nonewprivs 1".to_owned(),
        "polean-canary capeff 0000000000000000".to_owned(),
        "polean-canary iface lo".to_owned(),
        "polean-canary outbound Network is unreachable (os error 101)".to_owned(),
        "polean-canary fds 0,1,2".to_owned(),
        "polean-hostile done".to_owned(),
    ]
    .join("\n")
}

/// Every canary line becomes the field it names.
#[test]
fn a_report_is_parsed() -> Result<(), String> {
    let limits = LimitReadback {
        memory: Some("67108864".to_owned()),
        swap: Some("0".to_owned()),
        pids: Some("32".to_owned()),
    };
    let report = parse(&output(), "run-1", &limits);
    check_eq!(
        report.started,
        true,
        "the start line marks the canary started"
    );
    check_eq!(
        report.no_new_privs.as_deref(),
        Some("1"),
        "NoNewPrivs is parsed"
    );
    check_eq!(
        report.cap_eff.as_deref(),
        Some("0000000000000000"),
        "CapEff is parsed"
    );
    check_eq!(
        report.interfaces,
        vec!["lo".to_owned()],
        "each interface is collected"
    );
    check_eq!(
        report.descriptors,
        Some(vec![0, 1, 2]),
        "the descriptor list is parsed"
    );
    check_eq!(
        report.memory_max.as_deref(),
        Some("67108864"),
        "the limit read-back comes from the supervisor"
    );
    Ok(())
}

/// A report from another run, or one whose start line is missing, is unstarted.
#[test]
fn a_missing_start_line_leaves_the_canary_unstarted() -> Result<(), String> {
    let limits = LimitReadback::default();
    let report = parse(&output(), "run-2", &limits);
    check_eq!(
        report.started,
        false,
        "a start line carrying another nonce must not count"
    );
    Ok(())
}

/// An empty output parses to the empty report.
#[test]
fn an_empty_output_is_unstarted() -> Result<(), String> {
    let report = parse("", "run-1", &LimitReadback::default());
    check_eq!(report.started, false, "nothing ran");
    check_eq!(report.no_new_privs, None, "no field is assumed");
    Ok(())
}

/// Descriptor listings drop entries that are not numbers.
#[test]
fn descriptors_parse_and_drop_nonsense() -> Result<(), String> {
    check_eq!(
        parse_descriptors("0,1,2"),
        vec![0, 1, 2],
        "a clean listing is parsed"
    );
    check_eq!(
        parse_descriptors("0, ,x,2"),
        vec![0, 2],
        "a malformed entry is dropped rather than accepted"
    );
    check_eq!(
        parse_descriptors(""),
        Vec::<i32>::new(),
        "an empty listing is empty"
    );
    Ok(())
}
