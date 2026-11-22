//! The reviewed argument vector, its property checker, and its validators.
//!
//! The canonical vector is snapshotted so a change to what the boundary means is
//! reviewed rather than discovered in a probe. [`isolation_faults`] is tested in
//! both directions: the canonical vector has no fault, and each single-property
//! mutation is named, so a checker that stopped noticing one rule fails.

use std::path::{Path, PathBuf};

use rstest::rstest;

use super::{
    LaunchSpec,
    ReadOnlyBind,
    bubblewrap_argv,
    bubblewrap_scope_argv,
    covers,
    isolation_faults,
    unconfined_scope_argv,
};
use crate::{
    boundary::platform::Programs,
    scratch::{check, check_eq},
};

/// The canonical job: a probe child bound at `/job/bin/probe`.
pub(super) fn canonical_spec() -> LaunchSpec {
    LaunchSpec {
        host_program: PathBuf::from("/opt/polean/bin/probe"),
        sandbox_program: "/job/bin/probe".to_owned(),
        args: [
            "--exact",
            "--ignored",
            "toolchain::boundary::hostile::entry",
            "polean-hostile=canary:run-1",
            "--test-threads=1",
            "--nocapture",
            "-q",
        ]
        .map(str::to_owned)
        .to_vec(),
        binds: vec![
            ReadOnlyBind {
                source: PathBuf::from("/opt/polean/bin/probe"),
                target: "/job/bin/probe".to_owned(),
            },
            ReadOnlyBind {
                source: PathBuf::from("/var/tmp/polean-job/inputs"),
                target: "/job/in".to_owned(),
            },
        ],
        scratch_bytes: 8 * 1024 * 1024,
        env: vec![("LEAN_SYSROOT".to_owned(), "/job/lean".to_owned())],
        unit: "polean-probe-run-1".to_owned(),
        memory_bytes: 64 * 1024 * 1024,
        swap_bytes: 0,
        tasks_max: 32,
        runtime_secs: 60,
    }
}

/// The canonical vector is reviewed as a snapshot, one argument per line.
#[test]
fn the_canonical_vector_is_snapshotted() {
    let argv = bubblewrap_argv(&canonical_spec());
    let rendered = argv.join("\n");
    insta::assert_snapshot!("canonical_bubblewrap_argv", rendered);
}

/// The canonical vector breaks none of the properties.
#[test]
fn the_canonical_vector_keeps_every_property() -> Result<(), String> {
    let spec = canonical_spec();
    spec.validate()?;
    let faults = isolation_faults(&bubblewrap_argv(&spec), Path::new("/home/probe"));
    check_eq!(
        faults,
        Vec::<String>::new(),
        "the canonical vector must be faultless"
    );
    Ok(())
}

/// The selected vector wraps the bubblewrap vector in the scope.
#[test]
fn the_selected_vector_wraps_bubblewrap_in_the_scope() -> Result<(), String> {
    let spec = canonical_spec();
    let programs = Programs::standard();
    let argv = bubblewrap_scope_argv(&spec, &programs);
    for property in [
        "--user",
        "--scope",
        "--unit=polean-probe-run-1",
        "MemoryMax=67108864",
        "MemorySwapMax=0",
        "TasksMax=32",
        "OOMPolicy=kill",
        "KillSignal=SIGKILL",
        "RuntimeMaxSec=60",
    ] {
        check!(
            argv.iter().any(|argument| argument.as_str() == property),
            "the scope vector must carry {property}"
        );
    }
    let separator = argv
        .iter()
        .position(|argument| argument.as_str() == "--")
        .ok_or_else(|| "the scope vector has no separator".to_owned())?;
    check_eq!(
        argv.get(separator + 1).map(String::as_str),
        Some("/usr/bin/bwrap"),
        "the scope runs bubblewrap after the separator"
    );
    let inner = bubblewrap_argv(&spec);
    check_eq!(
        argv.get(separator + 2..).map(<[String]>::to_vec),
        Some(inner),
        "the bubblewrap vector follows unchanged"
    );
    Ok(())
}

/// The negative-control vector runs the host program directly.
#[test]
fn the_unconfined_vector_runs_the_host_program() -> Result<(), String> {
    let argv = unconfined_scope_argv(&canonical_spec());
    check!(
        argv.iter()
            .any(|argument| argument.as_str() == "/opt/polean/bin/probe"),
        "the host program must appear in the vector"
    );
    check!(
        !argv
            .iter()
            .any(|argument| argument.as_str() == "--unshare-net"),
        "the negative control must not isolate the network"
    );
    check_eq!(
        argv.last().map(String::as_str),
        Some("-q"),
        "the program arguments must be last"
    );
    Ok(())
}

/// Remove the network namespace option.
fn drop_network_namespace(argv: &mut Vec<String>) {
    argv.retain(|argument| argument.as_str() != "--unshare-net");
}

/// Remove the root remount, which must sit last before the separator.
fn drop_root_remount(argv: &mut Vec<String>) {
    if let Some(position) = argv
        .iter()
        .rposition(|argument| argument.as_str() == "--remount-ro")
    {
        argv.drain(position..position + 2);
    }
}

/// Move `--size` away from `--tmpfs`.
fn move_size(argv: &mut Vec<String>) {
    if let Some(position) = argv
        .iter()
        .position(|argument| argument.as_str() == "--size")
    {
        // Keep `--size` in the launcher prefix, but move it past `--tmpfs` so
        // it no longer immediately precedes the option it sizes.
        let pair: Vec<String> = argv.drain(position..position + 2).collect();
        if let Some(tmpfs) = argv
            .iter()
            .position(|argument| argument.as_str() == "--tmpfs")
        {
            argv.splice(tmpfs + 2..tmpfs + 2, pair);
        }
    }
}

/// Bind the host's `/home`.
fn bind_host_home(argv: &mut Vec<String>) {
    insert_before_separator(argv, &["--ro-bind", "/home", "/job/home"]);
}

/// Bind the host's `/tmp` with a trailing separator.
fn bind_host_tmp_slash(argv: &mut Vec<String>) {
    insert_before_separator(argv, &["--ro-bind", "/tmp/", "/job/host-tmp"]);
}

/// Add a second scratch mount that replaces the sized one.
fn add_second_scratch(argv: &mut Vec<String>) {
    if let Some(chdir) = argv
        .iter()
        .position(|argument| argument.as_str() == "--chdir")
    {
        argv.splice(chdir..chdir, ["--tmpfs".to_owned(), "/job/work".to_owned()]);
    }
}

/// Bind an ancestor of the user's home, which is `/home/probe/work` here.
fn bind_home_ancestor(argv: &mut Vec<String>) {
    insert_before_separator(argv, &["--ro-bind", "/home/probe", "/job/probe"]);
}

/// Add a writable bind.
fn add_writable_bind(argv: &mut Vec<String>) {
    insert_before_separator(argv, &["--bind", "/opt/polean", "/job/opt"]);
}

/// Remove the new terminal session.
fn drop_new_session(argv: &mut Vec<String>) {
    argv.retain(|argument| argument.as_str() != "--new-session");
}

/// Remove only the `/dev` remount, leaving the root remount in place.
fn drop_device_remount(argv: &mut Vec<String>) {
    if let Some(position) = argv
        .iter()
        .position(|argument| argument.as_str() == "--dev")
    {
        // `--dev /dev` is followed by `--remount-ro /dev`.
        argv.drain(position + 2..position + 4);
    }
}

/// Insert an option triple immediately before the command separator.
fn insert_before_separator(argv: &mut Vec<String>, values: &[&str]) {
    if let Some(position) = argv.iter().position(|argument| argument.as_str() == "--") {
        let owned: Vec<String> = values.iter().map(|value| (*value).to_owned()).collect();
        argv.splice(position..position, owned);
    }
}

/// Each single-property mutation is reported.
#[rstest]
#[case::network(drop_network_namespace, "unshare-net")]
#[case::root_remount(drop_root_remount, "last option before")]
#[case::size(move_size, "immediately precede")]
#[case::home(bind_host_home, "forbidden host path /home")]
#[case::tmp_slash(bind_host_tmp_slash, "forbidden host path /tmp/")]
#[case::second_scratch(add_second_scratch, "mount /job/work exactly once")]
#[case::ancestor(bind_home_ancestor, "ancestor of it")]
#[case::writable(add_writable_bind, "writable bind option")]
#[case::terminal(drop_new_session, "terminal session")]
#[case::device_remount(drop_device_remount, "/dev read-only")]
fn a_broken_vector_is_reported(
    #[case] mutate: fn(&mut Vec<String>),
    #[case] expected: &str,
) -> Result<(), String> {
    let mut argv = bubblewrap_argv(&canonical_spec());
    mutate(&mut argv);
    let faults = isolation_faults(&argv, Path::new("/home/probe/work"));
    check!(
        faults.iter().any(|fault| fault.contains(expected)),
        "the fault list {faults:?} must mention {expected:?}"
    );
    Ok(())
}

/// A zero scratch area is refused; one above the memory limit is allowed.
#[test]
fn the_scratch_area_must_be_non_empty_and_may_exceed_the_memory_limit() -> Result<(), String> {
    let mut spec = canonical_spec();
    spec.scratch_bytes = 0;
    check!(
        spec.validate().is_err(),
        "a zero scratch area must be refused"
    );
    spec.scratch_bytes = spec.memory_bytes + 1;
    spec.validate()?;
    Ok(())
}

/// Unit names must be bare systemd names.
#[rstest]
#[case::empty("")]
#[case::suffix("probe.scope")]
#[case::dotted("probe.run")]
#[case::spaced("probe run")]
#[case::separator("probe/run")]
fn a_bad_unit_name_is_refused(#[case] unit: &str) -> Result<(), String> {
    let mut spec = canonical_spec();
    spec.unit = unit.to_owned();
    match spec.validate() {
        Err(reason) => check!(
            reason.contains("unit name"),
            "the reason must name the unit, got {reason:?}"
        ),
        Ok(()) => return Err(format!("the unit name {unit:?} must be refused")),
    }
    Ok(())
}

/// An environment key that cannot be expressed is refused.
#[test]
fn a_bad_environment_key_is_refused() -> Result<(), String> {
    let mut spec = canonical_spec();
    spec.env = vec![("A=B".to_owned(), "value".to_owned())];
    match spec.validate() {
        Err(reason) => check!(
            reason.contains("environment key"),
            "the reason must name the environment key, got {reason:?}"
        ),
        Ok(()) => return Err("a key containing `=` must be refused".to_owned()),
    }
    Ok(())
}

/// Two binds may not share a target, and the program must be covered.
#[test]
fn bind_targets_must_be_distinct_and_cover_the_program() -> Result<(), String> {
    let mut spec = canonical_spec();
    spec.binds.push(ReadOnlyBind {
        source: PathBuf::from("/opt/polean/other"),
        target: "/job/in".to_owned(),
    });
    check!(
        spec.validate().is_err(),
        "a duplicate bind target must be refused"
    );
    let mut uncovered = canonical_spec();
    uncovered.sandbox_program = "/job/bin/absent".to_owned();
    check!(
        uncovered.validate().is_err(),
        "a program no bind provides must be refused"
    );
    Ok(())
}

/// A directory bind covers what is beneath it, and only that.
#[test]
fn a_directory_bind_covers_its_children() -> Result<(), String> {
    let binds = canonical_spec().binds;
    check!(covers(&binds, "/job/in"), "the target itself is covered");
    check!(
        covers(&binds, "/job/in/input.json"),
        "a child of a directory bind is covered"
    );
    check!(
        !covers(&binds, "/job/inputs"),
        "a similarly named sibling is not covered"
    );
    check!(
        !covers(&binds, "/job"),
        "a parent of a bind is not covered by it"
    );
    Ok(())
}
