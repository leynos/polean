//! Direct bound tests for the worker's statement renderer and diagnostics.
//!
//! `corpus` shows what the worker reports for the committed fixtures, but every
//! fixture's statement is a tree the kernel already built, so the corpus cannot
//! show that the renderer bounds its walk over a *shared* expression graph. The
//! Lean side, `lean/PoleanAuditRenderTests.lean`, drives `treeBounded` over
//! spines and self-applying chains, and `truncateUtf8` at and beyond the
//! contract's message bound; this module builds that executable with the pinned
//! Lake and runs it.
//!
//! The deadline is the point of the test. The guarded walk finishes in tens of
//! milliseconds, but with the node-budget guard bypassed the same input expands
//! to 2^41 - 1 nodes and never returns. Running the executable under a deadline
//! makes that defect a reported failure instead of a gate that hangs, so the
//! test fails if the worker's budget stops bounding its work.

use std::{
    io::{self, Read},
    path::Path,
    process::{Child, Command, Output, Stdio},
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
const EXPECTED_CHECKS: usize = 18;

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
/// is polled and killed at the deadline. Both pipes are drained on their own
/// threads while it runs: reading them only after the child exits would let a
/// child that writes more than the pipe buffer block on the write, and the
/// deadline would then report a runaway walk for a merely blocked process.
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
    let stdout = child.stdout.take().map(drain).ok_or_else(|| {
        format!(
            "cannot read the output of {}: stdout is not piped",
            program.display()
        )
    })?;
    let stderr = child.stderr.take().map(drain).ok_or_else(|| {
        format!(
            "cannot read the output of {}: stderr is not piped",
            program.display()
        )
    })?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(error) => {
                stop(&mut child, stdout, stderr);
                return Err(format!("cannot wait for {}: {error}", program.display()));
            }
        }
        if started.elapsed() >= deadline {
            stop(&mut child, stdout, stderr);
            return Err(format!(
                "{} did not finish within {deadline:?}; the renderer's node budget is not \
                 bounding its walk",
                program.display()
            ));
        }
        thread::sleep(POLL);
    }
    let status = child
        .wait()
        .map_err(|error| format!("cannot wait for {}: {error}", program.display()))?;
    Ok(Output {
        status,
        stdout: join(stdout, program)?,
        stderr: join(stderr, program)?,
    })
}

/// Read `pipe` to end on its own thread, while the child still runs.
fn drain<R>(mut pipe: R) -> thread::JoinHandle<io::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer = Vec::new();
        pipe.read_to_end(&mut buffer).map(|_| buffer)
    })
}

/// Kill and reap `child`, then wait for its output readers to see end of file.
fn stop(
    child: &mut Child,
    stdout: thread::JoinHandle<io::Result<Vec<u8>>>,
    stderr: thread::JoinHandle<io::Result<Vec<u8>>>,
) {
    drop(child.kill());
    drop(child.wait());
    drop(stdout.join());
    drop(stderr.join());
}

/// Collect one reader's bytes, naming the program when the read failed.
///
/// # Errors
///
/// Returns a diagnostic when the read failed or its thread panicked.
fn join(
    reader: thread::JoinHandle<io::Result<Vec<u8>>>,
    program: &Path,
) -> Result<Vec<u8>, String> {
    match reader.join() {
        Ok(Ok(bytes)) => Ok(bytes),
        Ok(Err(error)) => Err(format!(
            "cannot read the output of {}: {error}",
            program.display()
        )),
        Err(_) => Err(format!(
            "the output reader for {} panicked",
            program.display()
        )),
    }
}
