//! The unit-reset path, with an injected failing `systemctl`.
//!
//! `reset_failed_unit` records a reset failure instead of dropping it, and a
//! successful result runs nothing. These tests use an absent or failing
//! `systemctl`, so they need no systemd and no live user manager.

use std::path::Path;

use super::super::{platform::Programs, scope::Scope, supervisor::reset_failed_unit};
use crate::scratch::check;

/// A scope whose `systemctl` is the given path.
fn scope_with<'a>(programs: &'a Programs, unit: &str) -> Scope<'a> {
    Scope {
        programs,
        runtime_dir: Path::new("/nonexistent/runtime"),
        unit: unit.to_owned(),
    }
}

/// A failing result with an absent `systemctl` records the reset failure.
#[test]
fn a_failed_reset_is_recorded() -> Result<(), String> {
    let absent = Path::new("/nonexistent/systemctl");
    let programs = Programs {
        systemctl: absent.to_path_buf(),
        ..Programs::standard()
    };
    let scope = scope_with(&programs, "polean-reset-test");
    let mut diagnostics = Vec::new();
    reset_failed_unit(&scope, Some("failed"), &mut diagnostics);
    check!(
        diagnostics
            .iter()
            .any(|line| line.contains("cannot reset polean-reset-test.scope")),
        "the reset failure must name the unit, got {diagnostics:?}"
    );
    Ok(())
}

/// A successful result records nothing and runs no reset.
#[test]
fn a_successful_result_records_nothing() -> Result<(), String> {
    let absent = Path::new("/nonexistent/systemctl");
    let programs = Programs {
        systemctl: absent.to_path_buf(),
        ..Programs::standard()
    };
    let scope = scope_with(&programs, "polean-reset-test");
    let mut diagnostics = Vec::new();
    reset_failed_unit(&scope, Some("success"), &mut diagnostics);
    check!(
        diagnostics.is_empty(),
        "a successful result must record nothing, got {diagnostics:?}"
    );
    Ok(())
}

/// A failing `reset-failed` exit is reported with its stderr.
#[test]
fn a_non_zero_reset_exit_is_reported() -> Result<(), String> {
    let programs = Programs {
        systemctl: Path::new("/usr/bin/false").to_path_buf(),
        ..Programs::standard()
    };
    let scope = scope_with(&programs, "polean-reset-test");
    match scope.reset_failed() {
        Ok(()) => Err("a non-zero reset-failed must be an error".to_owned()),
        Err(reason) => {
            check!(
                reason.contains("cannot reset polean-reset-test.scope"),
                "the error must name the unit, got {reason:?}"
            );
            Ok(())
        }
    }
}
