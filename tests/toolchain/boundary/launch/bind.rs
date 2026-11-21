//! The bind, environment, and unit-name rules, and the vector's fault checks.
//!
//! The launch vector is data, so its safety properties are decided by inspection
//! rather than by running it. [`isolation_faults`] walks a built vector and
//! reports every property it breaks; an empty result is the review the tests and
//! the probe rely on.
//!
//! The rules exist because bubblewrap's defaults are permissive: an omitted
//! namespace option, a writable bind, or a bind of a host directory the boundary
//! must hide would each turn the sandbox into a formality. The negative
//! controls in the tests mutate one rule at a time, so a checker that stopped
//! noticing one of them fails.

use std::path::Path;

use super::{ReadOnlyBind, isolating_options};

/// Validate a bare transient unit name.
///
/// The supervisor appends `.scope` when it asks systemd about the unit, so a
/// name that already carries a suffix or a path separator is refused rather than
/// silently producing a different unit.
///
/// # Errors
///
/// Returns a diagnostic when the name is empty, too long, or carries a
/// character systemd would not accept.
pub fn validate_unit(unit: &str) -> Result<(), String> {
    if unit.is_empty() {
        return Err("the unit name is empty".to_owned());
    }
    if unit.len() > 200 {
        return Err("the unit name is longer than 200 bytes".to_owned());
    }
    if unit.contains('/') || unit.contains('.') {
        return Err(format!(
            "the unit name {unit:?} must be a bare name without a suffix or a path separator"
        ));
    }
    if !unit
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        return Err(format!(
            "the unit name {unit:?} must use only ASCII letters, digits, `-`, and `_`"
        ));
    }
    Ok(())
}

/// Validate the environment entries the child will receive.
///
/// # Errors
///
/// Returns a diagnostic for a key that cannot be expressed as `KEY=VALUE`.
pub fn validate_env(env: &[(String, String)]) -> Result<(), String> {
    for (key, value) in env {
        if !valid_env_key(key) {
            return Err(format!("the environment key {key:?} is not a valid name"));
        }
        if value.contains('\0') {
            return Err(format!("the environment value for {key:?} contains a NUL"));
        }
    }
    Ok(())
}

/// Whether `key` can be written as the left side of `KEY=VALUE`.
fn valid_env_key(key: &str) -> bool { !key.is_empty() && !key.contains('=') && !key.contains('\0') }

/// Validate the read-only binds.
///
/// # Errors
///
/// Returns a diagnostic for a relative source or target, a duplicate target, or
/// a target at a path the launch vector itself owns.
pub fn validate_binds(binds: &[ReadOnlyBind]) -> Result<(), String> {
    let reserved = ["/", "/proc", "/dev", "/dev/shm", "/job/work", "/job/bin"];
    for bind in binds {
        if !bind.source.is_absolute() {
            return Err(format!(
                "the bind source {} is not absolute",
                bind.source.display()
            ));
        }
        if !bind.target.starts_with('/') {
            return Err(format!("the bind target {:?} is not absolute", bind.target));
        }
        for path in reserved {
            if bind.target == path {
                return Err(format!(
                    "the bind target {:?} is a path the launch vector owns",
                    bind.target
                ));
            }
        }
    }
    let mut targets: Vec<&str> = binds.iter().map(|bind| bind.target.as_str()).collect();
    targets.sort_unstable();
    let before = targets.len();
    targets.dedup();
    if targets.len() != before {
        return Err("two binds share one target".to_owned());
    }
    Ok(())
}

/// Every property the selected vector must keep, as fault descriptions.
///
/// An empty result means the vector is acceptable. `home` is the user's home
/// directory, used to refuse a bind of it or of any ancestor.
#[must_use]
pub fn isolation_faults(argv: &[String], home: &Path) -> Vec<String> {
    let mut faults = Vec::new();
    for option in isolating_options() {
        if !argv.iter().any(|argument| argument.as_str() == option) {
            faults.push(format!("the vector lacks the isolating option {option}"));
        }
    }
    check_separator(argv, &mut faults);
    check_size(argv, &mut faults);
    check_binds(argv, home, &mut faults);
    check_no_writable_bind(argv, &mut faults);
    check_no_terminal(argv, &mut faults);
    faults
}

/// The command separator must be preceded by `--remount-ro /`.
fn check_separator(argv: &[String], faults: &mut Vec<String>) {
    let Some(separator) = argv.iter().position(|argument| argument.as_str() == "--") else {
        faults.push("the vector has no command separator".to_owned());
        return;
    };
    let before = separator.checked_sub(2);
    match before {
        Some(index)
            if argv
                .get(index)
                .is_some_and(|value| value.as_str() == "--remount-ro")
                && argv
                    .get(index + 1)
                    .is_some_and(|value| value.as_str() == "/") => {}
        _ => faults.push("`--remount-ro /` must be the last option before `--`".to_owned()),
    }
}

/// `--size` must immediately precede `--tmpfs`.
fn check_size(argv: &[String], faults: &mut Vec<String>) {
    let Some(size) = argv
        .iter()
        .position(|argument| argument.as_str() == "--size")
    else {
        faults.push("the vector has no `--size`".to_owned());
        return;
    };
    match (argv.get(size + 1), argv.get(size + 2)) {
        (Some(bytes), Some(tmpfs))
            if tmpfs.as_str() == "--tmpfs" && bytes.parse::<u64>().is_ok() => {}
        _ => faults.push("`--size <bytes>` must immediately precede `--tmpfs`".to_owned()),
    }
}

/// No bind may be a host path the boundary must hide.
fn check_binds(argv: &[String], home: &Path, faults: &mut Vec<String>) {
    let forbidden = ["/", "/run", "/tmp", "/home"];
    for source in bind_sources(argv) {
        if forbidden.contains(&source.as_str()) {
            faults.push(format!("the vector binds the forbidden host path {source}"));
            continue;
        }
        let source_path = Path::new(&source);
        if home.starts_with(source_path) {
            faults.push(format!(
                "the vector binds {source}, which is the user's home or an ancestor of it"
            ));
        }
    }
}

/// No writable bind may appear.
fn check_no_writable_bind(argv: &[String], faults: &mut Vec<String>) {
    for option in ["--bind", "--dev-bind", "--bind-try", "--dev-bind-try"] {
        if argv.iter().any(|argument| argument.as_str() == option) {
            faults.push(format!(
                "the vector contains a writable bind option {option}"
            ));
        }
    }
}

/// Standard input must not be a terminal.
fn check_no_terminal(argv: &[String], faults: &mut Vec<String>) {
    if !argv
        .iter()
        .any(|argument| argument.as_str() == "--new-session")
    {
        faults.push("the vector does not start a new terminal session".to_owned());
    }
}

/// Every `--ro-bind` source in the vector.
#[must_use]
pub fn bind_sources(argv: &[String]) -> Vec<String> {
    let mut sources = Vec::new();
    let mut index = 0;
    while index < argv.len() {
        if argv
            .get(index)
            .is_some_and(|value| value.as_str() == "--ro-bind")
        {
            if let Some(source) = argv.get(index + 1) {
                sources.push(source.clone());
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    sources
}
