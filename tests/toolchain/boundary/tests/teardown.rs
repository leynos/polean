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
    scratch::{Scratch, check, check_eq},
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

/// A scratch cgroup holding one surviving pid and the given populated state.
fn scratch_cgroup(scratch: &Scratch, populated: &str) -> Result<PathBuf, String> {
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
    ambient::write(
        &cgroup.join("cgroup.events"),
        format!("populated {populated}\n").as_bytes(),
    )
    .map_err(|error| format!("cannot write cgroup.events: {error}"))?;
    Ok(cgroup)
}

/// One teardown's observable report and diagnostics.
struct Observed {
    /// Processes reported as survivors.
    survivors: Vec<u32>,
    /// Whether the cgroup was confirmed empty.
    empty: bool,
    /// Processes still in the cgroup when the bound expired.
    residual: Vec<u32>,
    /// The last `cgroup.events` text.
    events: Option<String>,
    /// Cleanup diagnostics.
    errors: Vec<String>,
}

/// Tear one job down with the given late-scope handle.
fn run_teardown(scratch: &Scratch, late: &dyn TestLateScope) -> Result<Observed, String> {
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
    Ok(Observed {
        survivors: report.survivors,
        empty: report.cgroup_empty,
        residual: report.residual,
        events: report.events,
        errors,
    })
}

/// A late scope with the original child gone is found, killed, and checked
/// for emptiness, while the failed group kill is reported.
#[test]
fn a_late_scope_is_found_and_checked_for_emptiness() -> Result<(), String> {
    let mut scratch = Scratch::try_new("boundary-teardown-late")
        .map_err(|error| format!("cannot create scratch: {error}"))?;
    let cgroup = scratch_cgroup(&scratch, "0")?;
    let kill_file = cgroup.join("cgroup.kill");
    let fake = FakeScope {
        cgroup: Some(cgroup),
        killed: Cell::new(false),
    };
    let result = run_teardown(&scratch, &fake);
    // Read the kill request before the scratch cgroup is removed.
    let killed = ambient::read_to_string(&kill_file);
    let outcome = scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch: {error}"));
    let observed = result?;
    outcome?;
    let Observed {
        survivors,
        empty,
        errors,
        ..
    } = observed;
    check_eq!(
        killed.map_err(|error| format!("cannot read cgroup.kill: {error}"))?,
        "1",
        "teardown must write 1 to cgroup.kill"
    );
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

/// A populated late cgroup reports non-empty with its residual pids.
#[test]
fn a_populated_late_scope_is_not_reported_empty() -> Result<(), String> {
    let mut scratch = Scratch::try_new("boundary-teardown-populated")
        .map_err(|error| format!("cannot create scratch: {error}"))?;
    let cgroup = scratch_cgroup(&scratch, "1")?;
    let fake = FakeScope {
        cgroup: Some(cgroup),
        killed: Cell::new(false),
    };
    let result = run_teardown(&scratch, &fake);
    let outcome = scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch: {error}"));
    let observed = result?;
    outcome?;
    let Observed {
        empty,
        residual,
        events,
        ..
    } = observed;
    check!(!empty, "a populated cgroup must not report emptiness");
    check!(
        residual.contains(&std::process::id()),
        "the residual pids must be reported, got {residual:?}"
    );
    check!(
        events
            .as_deref()
            .is_some_and(|text| text.contains("populated 1")),
        "the events text must be retained, got {events:?}"
    );
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
    let observed = result?;
    outcome?;
    let Observed { empty, errors, .. } = observed;
    check!(fake.killed.get(), "the unit kill must be attempted");
    check!(
        errors.iter().any(|error| error.contains("unconfirmed")),
        "a missing scope must be unconfirmed, got {errors:?}"
    );
    check!(!empty, "an unobserved cgroup is not evidence of emptiness");
    Ok(())
}
