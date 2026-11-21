//! The launch vectors: one systemd scope, optionally around bubblewrap.
//!
//! Two argument vectors are built here, and nothing is executed. The supervisor
//! spawns them; this module decides what "the boundary" means as data, so the
//! properties can be reviewed, snapshotted, and mutated in tests.
//!
//! # The selected chain
//!
//! ```plaintext
//! systemd-run --user --scope --quiet --unit=<unit>
//!   -p MemoryMax=<bytes> -p MemorySwapMax=<bytes> -p TasksMax=<n>
//!   -p OOMPolicy=kill -p KillSignal=SIGKILL -p RuntimeMaxSec=<seconds>
//!   -- bwrap <isolating options> -- <program> <arguments>
//! ```
//!
//! `systemd-run --scope` runs the command itself, so the supervisor's direct
//! child is the process it spawned. The scope supplies the limits, a whole-group
//! out-of-memory kill, and an authoritative whole-tree kill. Bubblewrap supplies
//! the namespaces and an allowlisted filesystem view: `/usr`, the bound program
//! and inputs, and a private size-limited scratch area.
//!
//! # Properties the vector must keep
//!
//! [`isolation_faults`] states them and [`bubblewrap_argv`] is reviewed against
//! it by the tests. Every namespace option is present; `--cap-drop ALL` and
//! `--clearenv` are present; `--size` immediately precedes `--tmpfs`; the root
//! and `/dev` are remounted read-only, with `--remount-ro /` last before the
//! command separator; no bind is `/`, `/run`, `/tmp`, `/home`, the user's home,
//! or an ancestor of it; and no writable `--bind` or `--dev-bind` appears.
//!
//! `/dev` is remounted read-only in addition to `/` because bubblewrap's
//! `--dev` mounts a writable tmpfs that includes `/dev/shm`. Without the
//! remount, a job could write there: not to the host's `/dev/shm`, but to an
//! area outside the `--size` bound on which the scratch rule depends.

use std::path::PathBuf;

use super::platform::Programs;

mod bind;
pub use self::bind::*;

#[cfg(test)]
mod tests;

/// One read-only host path and where it appears inside the boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadOnlyBind {
    /// The absolute host path, a file or a directory.
    pub source: PathBuf,
    /// The absolute path it is mounted at inside the boundary.
    pub target: String,
}

/// A job's launch inputs, independent of the boundary that runs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchSpec {
    /// The absolute host path of the program, which `Unconfined` runs directly.
    pub host_program: PathBuf,
    /// The path the program is run as inside the boundary.
    pub sandbox_program: String,
    /// The program's arguments, passed as an array and never through a shell.
    pub args: Vec<String>,
    /// Read-only host paths, including the program and the input snapshot.
    pub binds: Vec<ReadOnlyBind>,
    /// The private scratch area's size in bytes (`--size`).
    pub scratch_bytes: u64,
    /// The environment entries the child gets, and nothing else.
    pub env: Vec<(String, String)>,
    /// The transient scope's unit name, without the `.scope` suffix.
    pub unit: String,
    /// `MemoryMax=`, in bytes.
    pub memory_bytes: u64,
    /// `MemorySwapMax=`, in bytes; the probe always requests zero.
    pub swap_bytes: u64,
    /// `TasksMax=`.
    pub tasks_max: u64,
    /// `RuntimeMaxSec=`, the scope's backstop deadline in seconds.
    pub runtime_secs: u64,
}

impl LaunchSpec {
    /// Reject a specification the launch vector could not safely express.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic naming the first fault. The scratch area must be
    /// non-empty and no larger than the memory limit, the unit name must be a
    /// bare systemd name, every bind target must be absolute and outside the
    /// reserved paths, and the program must be covered by a bind.
    pub fn validate(&self) -> Result<(), String> {
        if self.scratch_bytes == 0 {
            return Err("the scratch area must be non-empty".to_owned());
        }
        if self.scratch_bytes > self.memory_bytes {
            return Err(format!(
                "the scratch area ({} bytes) must not exceed the memory limit ({} bytes); tmpfs \
                 pages are charged to the job's cgroup",
                self.scratch_bytes, self.memory_bytes
            ));
        }
        validate_unit(&self.unit)?;
        validate_env(&self.env)?;
        validate_binds(&self.binds)?;
        if !covers(&self.binds, &self.sandbox_program) {
            return Err(format!(
                "no bind provides the program path {}",
                self.sandbox_program
            ));
        }
        Ok(())
    }
}

/// Whether some bind provides `path`, exactly or as a directory prefix.
#[must_use]
pub fn covers(binds: &[ReadOnlyBind], path: &str) -> bool {
    binds.iter().any(|bind| {
        let target = bind.target.as_str();
        path == target
            || path
                .strip_prefix(target)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

/// The isolating options every selected launch vector must carry.
#[must_use]
pub fn isolating_options() -> Vec<&'static str> {
    vec![
        "--unshare-user",
        "--unshare-net",
        "--unshare-pid",
        "--unshare-ipc",
        "--unshare-uts",
        "--unshare-cgroup",
        "--disable-userns",
        "--die-with-parent",
        "--new-session",
        "--cap-drop",
        "ALL",
        "--clearenv",
    ]
}

/// The systemd scope's property arguments, without the command.
fn scope_prefix(spec: &LaunchSpec) -> Vec<String> {
    ["--user", "--scope", "--quiet"]
        .into_iter()
        .map(str::to_owned)
        .chain(std::iter::once(format!("--unit={}", spec.unit)))
        .chain(properties(spec))
        .chain(std::iter::once("--".to_owned()))
        .collect()
}

/// The scope limits, each as a `-p NAME=VALUE` pair.
fn properties(spec: &LaunchSpec) -> Vec<String> {
    let values = [
        format!("MemoryMax={}", spec.memory_bytes),
        format!("MemorySwapMax={}", spec.swap_bytes),
        format!("TasksMax={}", spec.tasks_max),
        "OOMPolicy=kill".to_owned(),
        "KillSignal=SIGKILL".to_owned(),
        format!("RuntimeMaxSec={}", spec.runtime_secs),
    ];
    let mut argv = Vec::with_capacity(values.len() * 2);
    for value in values {
        argv.push("-p".to_owned());
        argv.push(value);
    }
    argv
}

/// The bubblewrap argument vector, including the command separator.
#[must_use]
pub fn bubblewrap_argv(spec: &LaunchSpec) -> Vec<String> {
    let mut argv: Vec<String> = isolating_options().into_iter().map(str::to_owned).collect();
    argv.extend(strings(&["--ro-bind", "/usr", "/usr"]));
    argv.extend(strings(&["--symlink", "usr/bin", "/bin"]));
    argv.extend(strings(&["--symlink", "usr/lib", "/lib"]));
    argv.extend(strings(&["--symlink", "usr/lib64", "/lib64"]));
    for bind in &spec.binds {
        argv.push("--ro-bind".to_owned());
        argv.push(bind.source.display().to_string());
        argv.push(bind.target.clone());
    }
    argv.extend(strings(&["--proc", "/proc"]));
    argv.extend(strings(&["--dev", "/dev"]));
    argv.extend(strings(&["--remount-ro", "/dev"]));
    argv.push("--size".to_owned());
    argv.push(spec.scratch_bytes.to_string());
    argv.extend(strings(&["--tmpfs", "/job/work"]));
    argv.extend(strings(&["--chdir", "/job/work"]));
    for (key, value) in &spec.env {
        argv.push("--setenv".to_owned());
        argv.push(key.clone());
        argv.push(value.clone());
    }
    argv.extend(strings(&["--remount-ro", "/"]));
    argv.push("--".to_owned());
    argv.push(spec.sandbox_program.clone());
    argv.extend(spec.args.iter().cloned());
    argv
}

/// The full selected vector: the scope around bubblewrap.
#[must_use]
pub fn bubblewrap_scope_argv(spec: &LaunchSpec, programs: &Programs) -> Vec<String> {
    let mut argv = scope_prefix(spec);
    argv.push(programs.bwrap.display().to_string());
    argv.extend(bubblewrap_argv(spec));
    argv
}

/// The negative-control vector: the scope around the bare program.
#[must_use]
pub fn unconfined_scope_argv(spec: &LaunchSpec) -> Vec<String> {
    let mut argv = scope_prefix(spec);
    argv.push(spec.host_program.display().to_string());
    argv.extend(spec.args.iter().cloned());
    argv
}

/// Own the given literals as an argument vector.
fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}
