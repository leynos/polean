//! The storage target's fixed-path switch, without starting a job.
//!
//! The unconfined storage control passes `fixed:no`, so the fixed sandbox
//! paths are refused for it. These cases show the token parses both ways and
//! default to on, and that a probe-owned host path is still accepted.

use super::super::hostile::{TARGET_PREFIX, targets};
use crate::scratch::check;

/// The fixed paths are attempted unless the caller turns them off.
#[test]
fn fixed_paths_default_to_on_and_parse_both_ways() -> Result<(), String> {
    let default = targets(&[])?;
    check!(
        default.fixed_paths,
        "the fixed sandbox paths must be attempted by default"
    );
    let off = targets(&[format!("{TARGET_PREFIX}fixed:no")])?;
    check!(!off.fixed_paths, "`fixed:no` must disable the fixed paths");
    let on = targets(&[format!("{TARGET_PREFIX}fixed:yes")])?;
    check!(on.fixed_paths, "`fixed:yes` must enable the fixed paths");
    Ok(())
}

/// A probe-owned host path is parsed alongside the switch.
#[test]
fn a_host_path_is_independent_of_the_fixed_switch() -> Result<(), String> {
    let parsed = targets(&[
        format!("{TARGET_PREFIX}host:/tmp/polean-owned/marker"),
        format!("{TARGET_PREFIX}fixed:no"),
    ])?;
    check!(!parsed.fixed_paths, "the switch must be recorded");
    check!(
        parsed
            .host
            .as_deref()
            .is_some_and(|path| path.ends_with("marker")),
        "the host path must be recorded independently"
    );
    Ok(())
}

/// An empty target list is the default target set.
#[test]
fn an_empty_list_is_default() -> Result<(), String> {
    check!(
        targets(&[])?.tcp.is_none(),
        "no target tokens must leave the TCP field empty"
    );
    Ok(())
}
