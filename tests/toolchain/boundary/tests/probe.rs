//! The probe harness's contract, without starting a job.
//!
//! These tests build the launch specification a probe would use and parse the
//! target tokens a probe would pass. Nothing is spawned, so they run under
//! `make test` on any host.

use std::time::Duration;

use super::super::{
    hostile::{self, Behaviour, TARGET_PREFIX},
    launch::covers,
    probe::{Options, PROBE_BINARY, PROBE_INPUTS, Probe},
    supervisor::Boundary,
};
use crate::scratch::{check, check_eq};

/// Every target kind is parsed into its field.
#[test]
fn target_tokens_parse_into_their_fields() -> Result<(), String> {
    let args = [
        format!("{TARGET_PREFIX}tcp:127.0.0.1:8080"),
        format!("{TARGET_PREFIX}abstract:name"),
        format!("{TARGET_PREFIX}socket:/tmp/probe.sock"),
        format!("{TARGET_PREFIX}host:/tmp/host"),
        format!("{TARGET_PREFIX}memory:fill"),
        format!("{TARGET_PREFIX}marker:run-1"),
        "--exact".to_owned(),
    ];
    let targets = hostile::targets(&args)?;
    check!(
        targets.tcp.as_deref() == Some("127.0.0.1:8080"),
        "the TCP address must be parsed",
    );
    check!(
        targets.abstract_name.as_deref() == Some("name"),
        "the abstract name must be parsed",
    );
    check!(
        targets
            .socket
            .as_deref()
            .is_some_and(|path| path.ends_with("probe.sock")),
        "the socket path must be parsed",
    );
    check!(
        targets
            .host
            .as_deref()
            .is_some_and(|path| path.ends_with("host")),
        "the host path must be parsed",
    );
    check!(
        targets.memory.as_deref() == Some("fill"),
        "the memory mode must be parsed",
    );
    check!(
        targets.marker.as_deref() == Some("run-1"),
        "the marker nonce must be parsed",
    );
    Ok(())
}

/// A malformed or unknown target is a probe defect.
#[test]
fn a_malformed_target_is_refused() -> Result<(), String> {
    for token in [
        format!("{TARGET_PREFIX}bogus:x"),
        format!("{TARGET_PREFIX}tcp"),
    ] {
        check!(
            hostile::targets(std::slice::from_ref(&token)).is_err(),
            "the malformed target {token:?} must be refused",
        );
    }
    Ok(())
}

/// The defaults are the product's limits, and the builders override exactly.
#[test]
fn the_options_defaults_and_overrides_are_both_observable() -> Result<(), String> {
    for boundary in [Boundary::Unconfined, Boundary::Bubblewrap] {
        let defaults = Options::new(boundary);
        check_eq!(
            defaults.memory_bytes,
            64 * 1024 * 1024,
            "the default memory limit is 64 MiB"
        );
        check_eq!(
            defaults.scratch_bytes,
            8 * 1024 * 1024,
            "the default scratch area is 8 MiB"
        );
        check_eq!(
            defaults.deadline,
            Duration::from_secs(10),
            "the default deadline is 10 s"
        );
        check_eq!(
            defaults.stdout_cap,
            1024 * 1024,
            "the default stdout cap is 1 MiB"
        );
        check_eq!(
            defaults.stderr_cap,
            64 * 1024,
            "the default stderr cap is 64 KiB"
        );
        let overridden = Options::new(boundary)
            .scratch(4 * 1024 * 1024)
            .memory(128 * 1024 * 1024)
            .deadline(Duration::from_secs(3))
            .stdout_cap(2048);
        check_eq!(
            overridden.scratch_bytes,
            4 * 1024 * 1024,
            "the scratch override is applied"
        );
        check_eq!(
            overridden.memory_bytes,
            128 * 1024 * 1024,
            "the memory override is applied"
        );
        check_eq!(
            overridden.deadline,
            Duration::from_secs(3),
            "the deadline override is applied"
        );
        check_eq!(
            overridden.stdout_cap,
            2048,
            "the stdout cap override is applied"
        );
        check_eq!(
            overridden.boundary,
            boundary,
            "the boundary must be preserved"
        );
    }
    Ok(())
}

/// The launch specification covers the child and the input snapshot.
#[test]
fn the_specification_covers_the_child() -> Result<(), String> {
    let probe = Probe::new("boundary-probe-spec")?;
    let options = Options::new(Boundary::Bubblewrap).target("polean-target=memory:allocate");
    let spec = probe.spec(Behaviour::Memory, &options);
    spec.validate()?;
    check!(
        covers(&spec.binds, PROBE_BINARY),
        "the child must be provided by a bind",
    );
    check!(
        covers(&spec.binds, PROBE_INPUTS),
        "the input snapshot must be provided by a bind",
    );
    check!(
        spec.unit.contains(probe.nonce()),
        "the unit name must carry the run's nonce",
    );
    check!(
        spec.args
            .iter()
            .any(|argument| argument == &hostile::entry_name()),
        "the child must be selected by libtest's exact filter",
    );
    check!(
        spec.args
            .iter()
            .any(|argument| argument == &format!("polean-hostile=memory:{}", probe.nonce())),
        "the child must receive the behaviour token with the run's nonce",
    );
    check!(
        spec.host_program.is_absolute(),
        "the host program must be an absolute path",
    );
    Ok(())
}
