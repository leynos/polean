//! Teardown when discovery failed: the unit is the only whole-tree handle.
//!
//! With no cgroup, a failed process-group kill must not end cleanup before the
//! unit kill. The cases use scratch `kill` and `systemctl` scripts, so they
//! need no systemd; the child is `/usr/bin/true`, not a supervised job.

use std::{
    path::Path,
    process::{Command, Stdio},
};

use super::super::{
    platform::Programs,
    scope::Scope,
    supervisor::{TestInputs, tear_down_for_test},
};
use crate::{
    ambient,
    scratch::{Scratch, check},
};

/// Write an executable scratch script.
fn script(scratch: &Scratch, name: &str, body: &str) -> Result<std::path::PathBuf, String> {
    let path = scratch.path().join(name);
    ambient::write(&path, body.as_bytes())
        .map_err(|error| format!("cannot write {name}: {error}"))?;
    ambient::make_executable(&path)
        .map_err(|error| format!("cannot make {name} executable: {error}"))?;
    Ok(path)
}

/// A failed group kill is reported, and the unit kill still runs.
#[test]
fn a_failed_group_kill_still_kills_the_unit() -> Result<(), String> {
    let mut scratch = Scratch::try_new("boundary-teardown-unit")
        .map_err(|error| format!("cannot create scratch: {error}"))?;
    let kill = script(&scratch, "kill", "#!/bin/sh\nexit 1\n")?;
    let record = scratch.path().join("systemctl-calls");
    let recorder = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\nexit 0\n",
        record.display()
    );
    let systemctl = script(&scratch, "systemctl", &recorder)?;
    let programs = Programs {
        systemctl,
        ..Programs::standard()
    };
    let scope = Scope {
        programs: &programs,
        runtime_dir: Path::new("/nonexistent/runtime"),
        unit: "polean-teardown-test".to_owned(),
    };
    let mut child = Command::new("/usr/bin/true")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("cannot start the child: {error}"))?;
    let pgid = child.id();
    let (report, _status, errors) = tear_down_for_test(TestInputs {
        child: &mut child,
        pgid,
        cgroup: None,
        kill: &kill,
        running: true,
        status: None,
        scope: &scope,
        require_scope: true,
    });
    let calls = ambient::read_to_string(&record).unwrap_or_default();
    let outcome = scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch: {error}"));
    outcome?;
    check!(
        errors.iter().any(|error| error.contains("kill")),
        "the failed group kill must be reported, got {errors:?}"
    );
    check!(
        calls.contains("kill") && calls.contains("polean-teardown-test.scope"),
        "the unit kill must run when no cgroup was observed, got {calls:?}"
    );
    check!(
        !report.cgroup_empty,
        "an unobserved cgroup is not evidence of emptiness"
    );
    check!(
        errors.iter().any(|error| error.contains("unconfirmed")),
        "a scope that never appeared must be reported as unconfirmed, got {errors:?}"
    );
    Ok(())
}

/// A successful result with a cgroup observed does not need the unit kill.
#[test]
fn a_known_cgroup_uses_the_cgroup_kill() -> Result<(), String> {
    let mut scratch = Scratch::try_new("boundary-teardown-known")
        .map_err(|error| format!("cannot create scratch: {error}"))?;
    let record = scratch.path().join("systemctl-calls");
    let recorder = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\nexit 0\n",
        record.display()
    );
    let systemctl = script(&scratch, "systemctl", &recorder)?;
    let programs = Programs {
        systemctl,
        ..Programs::standard()
    };
    let scope = Scope {
        programs: &programs,
        runtime_dir: Path::new("/nonexistent/runtime"),
        unit: "polean-teardown-known".to_owned(),
    };
    let cgroup = scratch.path().join("cgroup");
    ambient::create_dir(&cgroup)
        .map_err(|error| format!("cannot create the cgroup dir: {error}"))?;
    let kill = script(&scratch, "kill", "#!/bin/sh\nexit 0\n")?;
    let mut child = Command::new("/usr/bin/true")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("cannot start the child: {error}"))?;
    let pgid = child.id();
    let (_report, _status, _errors) = tear_down_for_test(TestInputs {
        child: &mut child,
        pgid,
        cgroup: Some(&cgroup),
        kill: &kill,
        running: true,
        status: None,
        scope: &scope,
        require_scope: true,
    });
    let calls = ambient::read_to_string(&record).unwrap_or_default();
    let outcome = scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch: {error}"));
    outcome?;
    check!(
        !calls.contains("--user kill"),
        "a known cgroup must not fall back to the unit kill, got {calls:?}"
    );
    Ok(())
}
