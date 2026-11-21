//! The escapee behaviour: leave the process group and outlive a group kill.
//!
//! The child starts `/usr/bin/sleep` in a new process group, then sleeps itself.
//! A process-group kill reaches the child but not the grandchild, which is the
//! observation D6 exists for: the process group is killed, but the cgroup is the
//! authority. Under bubblewrap the PID namespace removes the grandchild anyway,
//! because the namespace's init dies with the group; that cell is EP-M3's.
//!
//! The sleep is finite so a failed probe leaves nothing behind: the supervisor's
//! cgroup kill removes the grandchild even when everything else goes wrong.

use std::{os::unix::process::CommandExt, process::Command, thread, time::Duration};

use super::attempt;

/// The grandchild's program and its lifetime.
const SLEEP: &str = "/usr/bin/sleep";
/// How long the grandchild sleeps.
const GRANDCHILD_SECONDS: &str = "60";
/// How long the parent sleeps after starting it.
const PARENT_SLEEP: Duration = Duration::from_secs(60);

/// Start a grandchild in its own process group and sleep.
///
/// # Errors
///
/// Returns a diagnostic when the grandchild cannot be started; the failure is
/// reported as an observation and the behaviour ends.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every behaviour shares the dispatch signature so a failure is reported the same way"
)]
pub fn run() -> Result<(), String> {
    match Command::new(SLEEP)
        .arg(GRANDCHILD_SECONDS)
        .process_group(0)
        .spawn()
    {
        Ok(grandchild) => {
            attempt("escapee", &format!("started {}", grandchild.id()));
            thread::sleep(PARENT_SLEEP);
        }
        Err(error) => attempt("escapee", &format!("error: {error}")),
    }
    Ok(())
}
