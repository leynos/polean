//! Direct bound tests for the worker's statement renderer.
//!
//! `corpus` shows what the worker reports for the committed fixtures, but every
//! fixture's statement is a tree the kernel already built, so the corpus cannot
//! show that the renderer bounds its walk over a *shared* expression graph. The
//! Lean side, `lean/PoleanAuditRenderTests.lean`, drives `treeBounded` over
//! spines and self-applying chains; this module builds that executable with the
//! pinned Lake and runs it.
//!
//! The deadline is the point of the test. The guarded walk finishes in tens of
//! milliseconds, but with the node-budget guard bypassed the same input expands
//! to 2^41 - 1 nodes and never returns. Running the executable under a deadline
//! makes that defect a reported failure instead of a gate that hangs, so the
//! test fails if the worker's budget stops bounding its work.

use std::{
    path::Path,
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use super::runner::{built_target, require_success};
use crate::{smoke::pinned_lean, support::repository_path};

/// How long the renderer tests may run.
///
/// The whole executable runs in about forty milliseconds on an idle host, so
/// this bound is generous for a loaded one while still reporting a runaway walk
/// promptly.
const DEADLINE: Duration = Duration::from_secs(10);

/// How often the deadline is checked while the executable runs.
const POLL: Duration = Duration::from_millis(20);

/// The number of checks the Lean executable reports.
///
/// Pinned here so a check that silently disappears from the Lean list fails this
/// test rather than shrinking it.
const EXPECTED_CHECKS: usize = 9;

/// The renderer's own bound tests pass, and the node budget bounds the walk.
///
/// Runs against the provisioned toolchain, so it is `#[ignore]`d like the rest
/// of the smoke suite.
#[test]
#[ignore = "needs make tools; run make toolchain-smoke"]
fn renderer_bounds_its_walk() -> Result<(), String> {
    let repository = repository_path(".");
    let (lean_root, _pin) = pinned_lean()?;
    let lake = lean_root.join("bin").join("lake");
    let package = repository.join("lean");
    let tests = built_target(&lake, &package, &repository, "polean-audit-render-tests")?;
    let output = bounded(&tests, &repository, DEADLINE)?;
    require_success(&tests, &[], &output)?;
    require_summary(&output)
}

/// Require the executable's summary line, so every check is known to have run.
///
/// # Errors
///
/// Returns a diagnostic quoting the output when it is not the expected summary.
fn require_summary(output: &Output) -> Result<(), String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = format!("renderer: {EXPECTED_CHECKS} checks passed\n");
    if stdout == expected {
        return Ok(());
    }
    Err(format!(
        "the renderer tests reported {stdout:?}, not {expected:?}; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// Run `program` with no arguments, failing if it outlives `deadline`.
///
/// A `Command::output` call would wait forever on a runaway walk, so the child
/// is polled and killed at the deadline.
///
/// # Errors
///
/// Returns a diagnostic when the program cannot be started or waited for, or
/// when it is still running at the deadline.
fn bounded(program: &Path, cwd: &Path, deadline: Duration) -> Result<Output, String> {
    let mut child = Command::new(program)
        .current_dir(cwd)
        .env_clear()
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run {}: {error}", program.display()))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("cannot wait for {}: {error}", program.display()));
            }
        }
        if started.elapsed() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{} did not finish within {deadline:?}; the renderer's node budget is not \
                 bounding its walk",
                program.display()
            ));
        }
        thread::sleep(POLL);
    }
    child
        .wait_with_output()
        .map_err(|error| format!("cannot read the output of {}: {error}", program.display()))
}
