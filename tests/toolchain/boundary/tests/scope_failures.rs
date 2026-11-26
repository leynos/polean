//! `systemctl show` failures, without systemd.
//!
//! A non-zero exit is an error unless the message names this unit as absent.
//! The cases use a scratch `systemctl` script, so they need no user manager.

use std::path::Path;

use super::super::{
    platform::Programs,
    scope::{Scope, names_absent},
};
use crate::{
    ambient,
    scratch::{Scratch, check},
};

/// Write a `systemctl` script that prints `message` to stderr and exits 1.
fn failing_systemctl(scratch: &Scratch, message: &str) -> Result<std::path::PathBuf, String> {
    let path = scratch.path().join("systemctl");
    let script = format!("#!/bin/sh\nprintf '%s\\n' {message:?} >&2\nexit 1\n");
    ambient::write(&path, script.as_bytes())
        .map_err(|error| format!("cannot write the controlled systemctl: {error}"))?;
    ambient::make_executable(&path)
        .map_err(|error| format!("cannot make it executable: {error}"))?;
    Ok(path)
}

/// A scratch scope using the given `systemctl` path.
fn scope_with<'a>(programs: &'a Programs, unit: &str) -> Scope<'a> {
    Scope {
        programs,
        runtime_dir: Path::new("/nonexistent/runtime"),
        unit: unit.to_owned(),
    }
}

/// A query failure names the property and the unit.
#[test]
fn a_failed_show_names_the_property_and_unit() -> Result<(), String> {
    let mut scratch = Scratch::try_new("boundary-show-failure")
        .map_err(|error| format!("cannot create scratch: {error}"))?;
    let systemctl = failing_systemctl(&scratch, "Failed to connect to bus: no socket")?;
    let programs = Programs {
        systemctl,
        ..Programs::standard()
    };
    let scope = scope_with(&programs, "polean-show-test");
    let outcome = scope.show_within("ControlGroup", std::time::Duration::from_secs(1));
    scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch: {error}"))?;
    match outcome {
        Ok(value) => Err(format!("a query failure must be an error, got {value:?}")),
        Err(reason) => {
            check!(
                reason.contains("ControlGroup"),
                "the error must name the property, got {reason:?}"
            );
            check!(
                reason.contains("polean-show-test.scope"),
                "the error must name the unit, got {reason:?}"
            );
            Ok(())
        }
    }
}

/// An absent unit is `None`, not an error.
#[test]
fn an_absent_unit_is_none() -> Result<(), String> {
    let mut scratch = Scratch::try_new("boundary-show-absent")
        .map_err(|error| format!("cannot create scratch: {error}"))?;
    let systemctl = failing_systemctl(&scratch, "Unit polean-show-test.scope could not be found.")?;
    let programs = Programs {
        systemctl,
        ..Programs::standard()
    };
    let scope = scope_with(&programs, "polean-show-test");
    let outcome = scope.show_within("Result", std::time::Duration::from_secs(1));
    scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch: {error}"))?;
    check!(
        outcome?.is_none(),
        "an absent unit must be reported as None"
    );
    Ok(())
}

/// A failing `Result` query propagates rather than becoming a completion.
///
/// `ActiveState` is answered with `failed`, so the wait reaches the `Result`
/// query; only that query fails.
#[test]
fn a_failed_result_query_is_an_error() -> Result<(), String> {
    let mut scratch = Scratch::try_new("boundary-result-failure")
        .map_err(|error| format!("cannot create scratch: {error}"))?;
    let systemctl = scratch.path().join("systemctl");
    ambient::write(
        &systemctl,
        b"#!/bin/sh\ncase \"$*\" in\n  *ActiveState*) echo failed ;;\n  *) echo 'Failed to connect to bus: no socket' >&2; exit 1 ;;\nesac\n",
    )
    .map_err(|error| format!("cannot write the controlled systemctl: {error}"))?;
    ambient::make_executable(&systemctl)
        .map_err(|error| format!("cannot make it executable: {error}"))?;
    let programs = Programs {
        systemctl,
        ..Programs::standard()
    };
    let scope = scope_with(&programs, "polean-result-test");
    let outcome = scope.wait_for_result();
    scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch: {error}"))?;
    check!(
        outcome.is_err(),
        "a failed Result query must be an error, got {outcome:?}"
    );
    Ok(())
}

/// Only the exact systemd absence message is accepted.
#[test]
fn only_the_exact_absence_message_is_accepted() -> Result<(), String> {
    check!(
        names_absent("Unit polean-x.scope could not be found.", "polean-x.scope"),
        "systemd's absence message must be accepted"
    );
    check!(
        !names_absent(
            "Failed to query polean-x.scope: D-Bus socket could not be found",
            "polean-x.scope"
        ),
        "an unrelated failure mentioning the unit must still be an error"
    );
    check!(
        !names_absent("Unit other.scope could not be found.", "polean-x.scope"),
        "another unit's absence must not be accepted"
    );
    Ok(())
}
