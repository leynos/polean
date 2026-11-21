//! The user-manager controls: reachability and delegated controllers.
//!
//! `systemd-run --user` needs `XDG_RUNTIME_DIR` and nothing else from the
//! environment, so the supervisor's cleared environment carries exactly that
//! variable. A manager that is not reachable that way cannot create the scope,
//! and a manager that does not delegate `memory` and `pids` cannot apply the
//! limits; both are reported as missing controls rather than assumed.
//!
//! The parsing halves are pure so that hosts with different layouts can be
//! exercised without a systemd user manager.

use std::{path::Path, process::Command};

use super::super::outcome::Availability;
use crate::ambient;

/// Control 2: the user manager answers through a cleared environment.
///
/// `degraded` is accepted: it means the manager is reachable and some unrelated
/// unit failed. `starting`, `maintenance`, `stopping`, and `offline` are not,
/// because a manager that is not settled cannot be relied on to apply limits.
#[must_use]
pub fn check_user_manager(systemctl: &Path, runtime_dir: &Path) -> Availability {
    let output = match Command::new(systemctl)
        .args(["--user", "is-system-running"])
        .env_clear()
        .env("XDG_RUNTIME_DIR", runtime_dir)
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            return Availability::Missing(format!(
                "cannot reach the user manager through {}: {error}",
                runtime_dir.display()
            ));
        }
    };
    let state = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    manager_state(&state)
}

/// Decide control 2 from the state `systemctl --user is-system-running` prints.
#[must_use]
pub fn manager_state(state: &str) -> Availability {
    match state {
        "running" | "degraded" => Availability::Available,
        other => Availability::Missing(format!(
            "systemctl --user is-system-running reported {other:?}, not running or degraded"
        )),
    }
}

/// Control 3: the user manager delegates `memory` and `pids`.
#[must_use]
pub fn check_delegation() -> Availability {
    let proc_cgroup = match ambient::read_to_string(Path::new("/proc/self/cgroup")) {
        Ok(text) => text,
        Err(error) => {
            return Availability::Missing(format!("cannot read /proc/self/cgroup: {error}"));
        }
    };
    let Some(manager) = user_manager_cgroup(&proc_cgroup) else {
        return Availability::Missing(
            "the caller is not inside a systemd user manager cgroup".to_owned(),
        );
    };
    let path = Path::new("/sys/fs/cgroup")
        .join(manager.trim_start_matches('/'))
        .join("cgroup.subtree_control");
    match ambient::read_to_string(&path) {
        Ok(text) => delegation_from(&text),
        Err(error) => Availability::Missing(format!("cannot read {}: {error}", path.display())),
    }
}

/// The caller's user-manager cgroup, from `/proc/self/cgroup`.
///
/// The line is `0::<path>` on the unified hierarchy. The manager is the
/// `user@<uid>.service` component of that path; a caller outside one has no
/// delegated controllers to use.
#[must_use]
pub fn user_manager_cgroup(proc_cgroup: &str) -> Option<String> {
    for line in proc_cgroup.lines() {
        let Some(path) = line.strip_prefix("0::") else {
            continue;
        };
        let mut prefix = String::new();
        for component in path.split('/').filter(|part| !part.is_empty()) {
            prefix.push('/');
            prefix.push_str(component);
            if component.starts_with("user@") && component.ends_with(".service") {
                return Some(prefix);
            }
        }
    }
    None
}

/// Decide control 3 from the manager's `cgroup.subtree_control` contents.
#[must_use]
pub fn delegation_from(subtree_control: &str) -> Availability {
    let enabled: Vec<&str> = subtree_control.split_ascii_whitespace().collect();
    let absent: Vec<&str> = ["memory", "pids"]
        .into_iter()
        .filter(|controller| !enabled.contains(controller))
        .collect();
    if absent.is_empty() {
        Availability::Available
    } else {
        Availability::Missing(format!(
            "the user manager does not delegate {}; cgroup.subtree_control is {subtree_control:?}",
            absent.join(" and ")
        ))
    }
}
