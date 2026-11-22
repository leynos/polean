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

use super::ReadOnlyBind;

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

/// A launcher option and the values it consumed.
#[derive(Clone, Debug, PartialEq, Eq)]
struct LauncherOption {
    /// The option token itself.
    name: String,
    /// The values the option takes.
    values: Vec<String>,
}

/// How many values an option takes in the launcher prefix.
///
/// Anything unrecognised takes none. Parsing the prefix with arities keeps an
/// option's *value* from being read as an option, and the command arguments
/// after `--` are never searched at all.
fn value_arity(option: &str) -> usize {
    match option {
        "--ro-bind" | "--symlink" | "--setenv" => 2,
        "--size" | "--proc" | "--dev" | "--tmpfs" | "--chdir" | "--remount-ro" | "--cap-drop" => 1,
        _ => 0,
    }
}

/// The launcher options before the command separator.
fn launcher_prefix(argv: &[String]) -> &[String] {
    argv.iter()
        .position(|argument| argument.as_str() == "--")
        .map_or(argv, |separator| argv.get(..separator).unwrap_or(&[]))
}

/// The launcher's options and their values.
fn parse_launcher(argv: &[String]) -> Vec<LauncherOption> {
    let prefix = launcher_prefix(argv);
    let mut options = Vec::new();
    let mut index = 0;
    while index < prefix.len() {
        let Some(name) = prefix.get(index) else {
            break;
        };
        let arity = value_arity(name);
        let values = prefix
            .get(index + 1..index + 1 + arity)
            .map_or_else(Vec::new, <[String]>::to_vec);
        options.push(LauncherOption {
            name: name.clone(),
            values,
        });
        index += 1 + arity;
    }
    options
}

/// Whether the launcher carries `name` with exactly `values`.
fn has_option(options: &[LauncherOption], name: &str, values: &[&str]) -> bool {
    options
        .iter()
        .any(|option| option.name == name && option.values == values)
}

/// Whether the launcher carries `name` at all.
fn has_any(options: &[LauncherOption], name: &str) -> bool {
    options.iter().any(|option| option.name == name)
}

/// Every property the selected vector must keep, as fault descriptions.
///
/// An empty result means the vector is acceptable. `home` is the user's home
/// directory, used to refuse a bind of it or of any ancestor.
#[must_use]
pub fn isolation_faults(argv: &[String], home: &Path) -> Vec<String> {
    let options = parse_launcher(argv);
    let prefix = launcher_prefix(argv);
    let mut faults = Vec::new();
    for option in super::ISOLATING_NAMES {
        if !has_any(&options, option) {
            faults.push(format!("the vector lacks the isolating option {option}"));
        }
    }
    check_cap_drop(&options, &mut faults);
    check_remounts(&options, &mut faults);
    check_separator(argv, &mut faults);
    check_scratch(&options, prefix, &mut faults);
    check_binds(&options, home, &mut faults);
    check_no_writable_bind(&options, &mut faults);
    check_no_terminal(&options, &mut faults);
    faults
}

/// `--cap-drop ALL` must be present: all capabilities dropped.
fn check_cap_drop(options: &[LauncherOption], faults: &mut Vec<String>) {
    if !has_option(options, "--cap-drop", &["ALL"]) {
        faults.push("the vector must drop every capability with `--cap-drop ALL`".to_owned());
    }
}

/// The root and `/dev` must both be remounted read-only.
fn check_remounts(options: &[LauncherOption], faults: &mut Vec<String>) {
    if !has_option(options, "--remount-ro", &["/"]) {
        faults.push("the vector must remount the root read-only".to_owned());
    }
    if !has_option(options, "--remount-ro", &["/dev"]) {
        faults.push(
            "the vector must remount /dev read-only, because `--dev` creates a writable tmpfs \
             that includes /dev/shm"
                .to_owned(),
        );
    }
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

/// `--size` must immediately precede the one scratch `--tmpfs`.
///
/// Bubblewrap applies mounts in order and `--size` applies only to the next
/// tmpfs, so a second `--tmpfs /job/work` would replace the sized scratch area
/// with an unbounded one. Exactly one scratch mount is required, and it must be
/// the one the size argument governs.
fn check_scratch(options: &[LauncherOption], prefix: &[String], faults: &mut Vec<String>) {
    let scratch_mounts = options
        .iter()
        .filter(|option| option.name == "--tmpfs" && option.values == ["/job/work"])
        .count();
    if scratch_mounts != 1 {
        faults.push(format!(
            "the vector must mount /job/work exactly once, found {scratch_mounts}"
        ));
        return;
    }
    let Some(size) = prefix
        .iter()
        .position(|argument| argument.as_str() == "--size")
    else {
        faults.push("the vector has no `--size`".to_owned());
        return;
    };
    match (prefix.get(size + 1), prefix.get(size + 2)) {
        (Some(bytes), Some(tmpfs))
            if tmpfs.as_str() == "--tmpfs" && bytes.parse::<u64>().is_ok() => {}
        _ => faults.push("`--size <bytes>` must immediately precede `--tmpfs`".to_owned()),
    }
}

/// No bind may be a host path the boundary must hide.
fn check_binds(options: &[LauncherOption], home: &Path, faults: &mut Vec<String>) {
    let forbidden = ["/", "/run", "/tmp", "/home"];
    for source in bind_sources(options) {
        // Compare as paths, so `/tmp/` and `/tmp` are the same directory.
        let source_path = Path::new(&source);
        if forbidden
            .iter()
            .any(|reserved| source_path == Path::new(reserved))
        {
            faults.push(format!("the vector binds the forbidden host path {source}"));
            continue;
        }
        if home.starts_with(source_path) {
            faults.push(format!(
                "the vector binds {source}, which is the user's home or an ancestor of it"
            ));
        }
    }
}

/// No writable bind may appear.
fn check_no_writable_bind(options: &[LauncherOption], faults: &mut Vec<String>) {
    for option in ["--bind", "--dev-bind", "--bind-try", "--dev-bind-try"] {
        if has_any(options, option) {
            faults.push(format!(
                "the vector contains a writable bind option {option}"
            ));
        }
    }
}

/// Standard input must not be a terminal.
fn check_no_terminal(options: &[LauncherOption], faults: &mut Vec<String>) {
    if !has_any(options, "--new-session") {
        faults.push("the vector does not start a new terminal session".to_owned());
    }
}

/// Every `--ro-bind` source in the launcher options.
fn bind_sources(options: &[LauncherOption]) -> Vec<String> {
    options
        .iter()
        .filter(|option| option.name == "--ro-bind")
        .filter_map(|option| option.values.first().cloned())
        .collect()
}
