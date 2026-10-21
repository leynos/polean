//! Provisioned OPA and Lean smoke observations.
//!
//! Every test here is `#[ignore]`d because it needs the digest-checked
//! artefacts that `make tools` installs. `make toolchain-smoke` provisions
//! them and then runs this module by name.
//!
//! A missing tool fails with a remediation message rather than being skipped:
//! a silently skipped provisioned test would look like a pass.
//!
//! These are smoke observations. OPA agreeing with Regorus on four requests is
//! not conformance evidence, and a compiled Lean smoke library is not a
//! theorem about Polean.

use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

use rstest::rstest;

use crate::{
    engine::ENTRYPOINT,
    json::parse_json,
    support::{
        CROSS_TENANT_ADMIN_REQUEST,
        OWNER_READ_REQUEST,
        OWNER_UNLOCKED_REQUEST,
        SAME_TENANT_ADMIN_LOCKED_REQUEST,
        baseline_source_path,
        lean_root,
        opa_path,
        pins_path,
        repository_path,
    },
};

/// One pinned tool, as the pin file records it.
struct Pin {
    /// The digest that names the installed directory.
    sha256: String,
    /// The upstream version, which the binary must report.
    version: String,
    /// The complete identity line Lean reports, checked verbatim.
    identity: Option<String>,
}

/// Read one tool's digest and version from the committed pin file.
///
/// # Errors
///
/// Returns a diagnostic when the pin file cannot be read or the entry is not
/// the expected shape.
fn read_pin(tool: &str) -> Result<Pin, String> {
    let path = pins_path();
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}; run `make tools`", path.display()))?;
    let document = parse_json(&text, "toolchain-pins.json")?;
    let entry = document
        .get(tool)
        .ok_or_else(|| format!("toolchain-pins.json has no `{tool}` entry"))?;
    let sha256 = entry
        .get("sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("toolchain-pins.json `{tool}.sha256` is not a string"))?;
    let version = entry
        .get("version")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("toolchain-pins.json `{tool}.version` is not a string"))?;
    let identity = entry
        .get("identity")
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("toolchain-pins.json `{tool}.identity` is not a string"))
        })
        .transpose()?;
    Ok(Pin {
        sha256: sha256.to_owned(),
        version: version.to_owned(),
        identity,
    })
}

/// Require an executable to exist, with a remediation message when it does not.
///
/// # Errors
///
/// Returns a diagnostic naming the path and `make tools`.
fn require_executable(path: &Path) -> Result<(), String> {
    if path.is_file() {
        return Ok(());
    }
    Err(format!(
        "{} is missing; run `make tools` to install the pinned tools",
        path.display()
    ))
}

/// Run a pinned executable with an empty environment and captured output.
///
/// `env_clear` keeps the host's environment out of the observation: the
/// result must not depend on `PATH`, `HOME`, or a toolchain variable. Enforced
/// isolation is 1.2.4's work; this only makes the smoke run reproducible.
///
/// # Errors
///
/// Returns a diagnostic when the process cannot start, or when it exits
/// non-zero.
fn run(
    program: &Path,
    args: &[&str],
    stdin_text: Option<&str>,
    cwd: &Path,
) -> Result<String, String> {
    let mut command = Command::new(program);
    command.args(args).current_dir(cwd).env_clear();
    let output = if stdin_text.is_some() {
        let mut child = command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|error| format!("cannot start {}: {error}", program.display()))?;
        if let (Some(mut handle), Some(text)) = (child.stdin.take(), stdin_text) {
            use std::io::Write as _;
            handle
                .write_all(text.as_bytes())
                .map_err(|error| format!("cannot write to {}: {error}", program.display()))?;
        }
        child
            .wait_with_output()
            .map_err(|error| format!("cannot wait for {}: {error}", program.display()))?
    } else {
        command
            .output()
            .map_err(|error| format!("cannot run {}: {error}", program.display()))?
    };
    describe(program, args, &output)
}

/// Turn a process outcome into stdout, or a diagnostic naming both streams.
fn describe(program: &Path, args: &[&str], output: &Output) -> Result<String, String> {
    if output.status.success() {
        return String::from_utf8(output.stdout.clone())
            .map_err(|error| format!("{} produced invalid UTF-8: {error}", program.display()));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "{} {} failed with {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        program.display(),
        args.join(" "),
        output.status.code()
    ))
}

/// The pinned OPA binary, with a remediation message when it is missing.
///
/// # Errors
///
/// Returns a diagnostic when the pin file or the installed binary is missing.
fn pinned_opa() -> Result<(PathBuf, Pin), String> {
    let pin = read_pin("opa")?;
    let path = opa_path(&pin.sha256);
    require_executable(&path)?;
    Ok((path, pin))
}

/// O7: the provisioned OPA binary reports the pinned version and Rego version.
#[rstest]
#[ignore = "needs make tools; run make toolchain-smoke"]
fn opa_reports_pinned_version() -> Result<(), String> {
    let (path, pin) = pinned_opa()?;
    let root = repository_path(".");
    let report = run(&path, &["version"], None, &root)?;
    // Compare whole lines. A substring test would accept `Version: 1.21.20`
    // as satisfying a `1.21.2` pin.
    let version_line = format!("Version: {}", pin.version);
    if !report.lines().any(|line| line.trim() == version_line) {
        return Err(format!(
            "OPA must report the line `{version_line}`, got:\n{report}"
        ));
    }
    if !report.lines().any(|line| line.trim() == "Rego Version: v1") {
        return Err(format!(
            "OPA must report the line `Rego Version: v1`, got:\n{report}"
        ));
    }
    Ok(())
}

/// Assert OPA's stdout is exactly one defined Boolean result for `expected`.
///
/// The shape is `{"result":[{"expressions":[{"value":<bool>,...}]}]}`, with
/// one result, one expression, a JSON Boolean, and no `errors` key. An
/// undefined result prints `{}` with exit 0, so the shape check is what
/// separates a decision from a miss.
///
/// # Errors
///
/// Returns a diagnostic naming the received text when the shape differs.
fn expect_opa_boolean(stdout: &str, expected: bool, request: &str) -> Result<(), String> {
    let document = parse_json(stdout, "OPA stdout")?;
    if document.get("errors").is_some() {
        return Err(format!(
            "OPA reported errors for request {request}:\n{stdout}"
        ));
    }
    let results = document
        .get("result")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("OPA printed no result array for request {request}:\n{stdout}"))?;
    if results.len() != 1 {
        return Err(format!(
            "OPA must print exactly one result, printed {} for request {request}:\n{stdout}",
            results.len()
        ));
    }
    let expressions = results
        .first()
        .and_then(|result| result.get("expressions"))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("OPA printed no expressions for request {request}:\n{stdout}"))?;
    if expressions.len() != 1 {
        return Err(format!(
            "OPA must print exactly one expression, printed {} for request {request}:\n{stdout}",
            expressions.len()
        ));
    }
    let value = expressions
        .first()
        .and_then(|expression| expression.get("value"))
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| format!("OPA printed no JSON Boolean for request {request}:\n{stdout}"))?;
    if value == expected {
        return Ok(());
    }
    Err(format!(
        "OPA must decide {expected} for request {request}, decided {value}:\n{stdout}"
    ))
}

/// O4 for OPA: the four requests decide the Booleans the model predicts.
///
/// The expectations come from `T ∧ W ∧ (A ∨ (O ∧ ¬L))`, not from either
/// runtime. The cross-tenant case is the input on which the E0 mutation
/// diverges from the baseline, so both outcomes occur.
#[rstest]
#[case::owner_unlocked_write(OWNER_UNLOCKED_REQUEST, true)]
#[case::cross_tenant_admin_write(CROSS_TENANT_ADMIN_REQUEST, false)]
#[case::same_tenant_admin_locked_write(SAME_TENANT_ADMIN_LOCKED_REQUEST, true)]
#[case::owner_read(OWNER_READ_REQUEST, false)]
#[ignore = "needs make tools; run make toolchain-smoke"]
fn opa_decides_baseline_source(
    #[case] request: &str,
    #[case] expected: bool,
) -> Result<(), String> {
    let (path, _pin) = pinned_opa()?;
    let root = repository_path(".");
    let source = baseline_source_path();
    let source_text = source.display().to_string();
    let stdout = run(
        &path,
        &[
            "eval",
            "--format",
            "json",
            "--strict-builtin-errors",
            "-d",
            &source_text,
            "-I",
            ENTRYPOINT,
        ],
        Some(request),
        &root,
    )?;
    expect_opa_boolean(&stdout, expected, request)
}

/// The pinned Lean toolchain root, with a remediation message when missing.
///
/// # Errors
///
/// Returns a diagnostic when the pin file or the installed root is missing.
fn pinned_lean() -> Result<(PathBuf, Pin), String> {
    let pin = read_pin("lean")?;
    let root = lean_root(&pin.sha256);
    if !root.is_dir() {
        return Err(format!(
            "{} is missing; run `make tools` to install the pinned tools",
            root.display()
        ));
    }
    Ok((root, pin))
}

/// O7: the provisioned Lean toolchain reports the pinned identity.
///
/// O7 requires more than a matching version number. Two builds of the same
/// version can differ in platform, commit, or build type, so a prefix check
/// would accept a different toolchain. The reported line must equal the
/// complete identity the pin records, and `lean/lean-toolchain` must select
/// exactly the release that identity names, so the committed selector and the
/// provisioned binary cannot drift.
#[rstest]
#[ignore = "needs make tools; run make toolchain-smoke"]
fn lean_reports_pinned_version() -> Result<(), String> {
    let (root, pin) = pinned_lean()?;
    let lean = root.join("bin").join("lean");
    require_executable(&lean)?;
    let report = run(&lean, &["--version"], None, &repository_path("."))?;
    let reported = report.trim();
    let identity = pin.identity.as_deref().ok_or_else(|| {
        "toolchain-pins.json `lean.identity` is missing; the complete reported version line is the \
         thing O7 checks"
            .to_owned()
    })?;
    if reported != identity {
        return Err(format!(
            "Lean must report exactly `{identity}`, got:\n{reported}"
        ));
    }
    let selection_path = repository_path("lean/lean-toolchain");
    let selection_text = std::fs::read_to_string(&selection_path)
        .map_err(|error| format!("cannot read {}: {error}", selection_path.display()))?;
    let selected = selection_text.trim();
    let expected = format!("leanprover/lean4:v{}", pin.version);
    if selected != expected {
        return Err(format!(
            "lean/lean-toolchain must select exactly `{expected}`, found {selected:?}"
        ));
    }
    Ok(())
}

/// O6: the pinned Lake builds the repository-owned smoke library.
///
/// The build target is named explicitly, and the `.olean` is then required,
/// so a Lake invocation that builds nothing cannot pass. The axiom
/// expectations live in the Lean source as `#guard_msgs`, so a change in what
/// the kernel records fails this build.
#[rstest]
#[ignore = "needs make tools; run make toolchain-smoke"]
fn lean_builds_smoke_library() -> Result<(), String> {
    let (root, _pin) = pinned_lean()?;
    let lake = root.join("bin").join("lake");
    require_executable(&lake)?;
    let package = repository_path("lean");
    let lake_dir = package.display().to_string();
    run(
        &lake,
        &["--dir", &lake_dir, "build", "PoleanSmoke"],
        None,
        &repository_path("."),
    )?;
    let olean = package.join(".lake/build/lib/lean/PoleanSmoke.olean");
    if !olean.is_file() {
        return Err(format!(
            "the build reported success but {} is absent; the target did not build",
            olean.display()
        ));
    }
    Ok(())
}
