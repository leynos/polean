//! Teardown when discovery failed: the unit and a possibly late scope.
//!
//! The late-scope operation is injected, so a test can simulate a scope that
//! appears after the discovery failure while the original child has exited,
//! which a real systemd manager cannot be made to produce on demand. The
//! cgroup files are scratch files; nothing here touches a real cgroup.

use std::{
    cell::Cell,
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};

use super::super::supervisor::{TestInputs, TestLateScope, tear_down_for_test};
use crate::{
    ambient,
    scratch::{Scratch, check},
};

/// A whole-tree handle whose scope may or may not appear.
struct FakeScope {
    /// The cgroup the scope reports, if it appears.
    cgroup: Option<PathBuf>,
    /// Whether the unit kill was attempted.
    killed: Cell<bool>,
}

impl TestLateScope for FakeScope {
    fn kill_unit(&self) -> Result<(), String> {
        self.killed.set(true);
        Ok(())
    }

    fn wait_for_cgroup(&self, _bound: Duration) -> Result<Option<PathBuf>, String> {
        Ok(self.cgroup.clone())
    }
}

/// Write an executable scratch script.
fn script(scratch: &Scratch, name: &str, body: &str) -> Result<std::path::PathBuf, String> {
    let path = scratch.path().join(name);
    ambient::write(&path, body.as_bytes())
        .map_err(|error| format!("cannot write {name}: {error}"))?;
    ambient::make_executable(&path)
        .map_err(|error| format!("cannot make {name} executable: {error}"))?;
    Ok(path)
}

/// A scratch cgroup holding one surviving pid and reporting populated 0.
fn scratch_cgroup(scratch: &Scratch) -> Result<PathBuf, String> {
    let cgroup = scratch.path().join("cgroup");
    ambient::create_dir(&cgroup)
        .map_err(|error| format!("cannot create the cgroup dir: {error}"))?;
    // The pid stands for a descendant that outlived the original child.
    let survivor = std::process::id();
    ambient::write(
        &cgroup.join("cgroup.procs"),
        format!("{survivor}\n").as_bytes(),
    )
    .map_err(|error| format!("cannot write cgroup.procs: {error}"))?;
    ambient::write(&cgroup.join("cgroup.events"), b"populated 0\n")
        .map_err(|error| format!("cannot write cgroup.events: {error}"))?;
    Ok(cgroup)
}

/// Tear one job down with the given late-scope handle.
fn run_teardown(
    scratch: &Scratch,
    late: &dyn TestLateScope,
) -> Result<(Vec<u32>, bool, Vec<String>), String> {
    let kill = script(scratch, "kill", "#!/bin/sh\nexit 1\n")?;
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
        late,
        require_scope: true,
    });
    Ok((report.survivors, report.cgroup_empty, errors))
}

/// A late scope with the original child gone is found, killed, and checked
/// for emptiness, while the failed group kill is reported.
#[test]
fn a_late_scope_is_found_and_checked_for_emptiness() -> Result<(), String> {
    let mut scratch = Scratch::try_new("boundary-teardown-late")
        .map_err(|error| format!("cannot create scratch: {error}"))?;
    let cgroup = scratch_cgroup(&scratch)?;
    let fake = FakeScope {
        cgroup: Some(cgroup),
        killed: Cell::new(false),
    };
    let result = run_teardown(&scratch, &fake);
    let outcome = scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch: {error}"));
    let (survivors, empty, errors) = result?;
    outcome?;
    check!(fake.killed.get(), "the unit kill must be attempted");
    check!(
        errors.iter().any(|error| error.contains("kill")),
        "the failed group kill must be reported, got {errors:?}"
    );
    check!(
        !errors.iter().any(|error| error.contains("unconfirmed")),
        "a scope that appeared must not be called unconfirmed, got {errors:?}"
    );
    check!(
        survivors.contains(&std::process::id()),
        "the late cgroup's survivors must be read, got {survivors:?}"
    );
    check!(empty, "the late cgroup must be confirmed empty");
    Ok(())
}

/// A scope that never appears is reported as an unconfirmed whole-tree kill.
#[test]
fn a_missing_late_scope_is_unconfirmed() -> Result<(), String> {
    let mut scratch = Scratch::try_new("boundary-teardown-missing")
        .map_err(|error| format!("cannot create scratch: {error}"))?;
    let fake = FakeScope {
        cgroup: None,
        killed: Cell::new(false),
    };
    let result = run_teardown(&scratch, &fake);
    let outcome = scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch: {error}"));
    let (_survivors, empty, errors) = result?;
    outcome?;
    check!(fake.killed.get(), "the unit kill must be attempted");
    check!(
        errors.iter().any(|error| error.contains("unconfirmed")),
        "a missing scope must be unconfirmed, got {errors:?}"
    );
    check!(!empty, "an unobserved cgroup is not evidence of emptiness");
    Ok(())
}
