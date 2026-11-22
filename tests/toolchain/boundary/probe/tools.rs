//! The real pinned tools inside the selected boundary.
//!
//! These are the end-to-end observations that the boundary does not break the
//! tools it exists to contain. Pinned OPA evaluates the baseline policy inside
//! and outside the boundary and must decide the same Booleans; an OPA query that
//! calls `http.send` must reach nothing; and the pinned Lean must check the
//! repository's smoke module inside the 2 GiB and 60 s defaults.
//!
//! The inputs are copied from trusted repository files. A tool that is not
//! provisioned fails with the remediation the smoke suite gives, because a
//! skipped observation would read as a pass.

use std::{path::Path, process::Command, time::Duration};

use serde_json::Value as Json;

use super::{
    super::{
        launch::ReadOnlyBind,
        outcome::{ExitForm, Outcome},
        supervisor::{Boundary, Run, require_outcome},
    },
    Options,
    Probe,
    ToolJob,
    listeners,
    pins::{pinned_lean, pinned_opa},
};
use crate::{
    ambient,
    json::parse_json,
    scratch::{check, check_eq},
    support::{CROSS_TENANT_ADMIN_REQUEST, baseline_source_path, repository_path},
};

/// The OPA entry point the baseline policy exports.
const ENTRY: &str = "data.authz.allow";

/// The fixed path OPA is bound at inside the boundary.
const OPA: &str = "/job/tool/opa";

/// The fixed path Lean is bound at inside the boundary.
const LEAN: &str = "/job/lean/bin/lean";

/// The fixed Lean root inside the boundary.
const LEAN_SYSROOT: &str = "/job/lean";

/// The OPA query that must not reach a host listener.
const HTTP_POLICY: &str = r#"package probe.http

import rego.v1

allow if {
	response := http.send({"method": "get", "url": "http://127.0.0.1:PORT/"})
	response.status_code == 200
}
"#;

/// Copy a repository file into the input snapshot and return its in-boundary
/// path.
///
/// # Errors
///
/// Returns a diagnostic when the source cannot be read or the copy written.
fn copy_input(probe: &Probe, source: &Path, name: &str) -> Result<String, String> {
    let bytes = ambient::read(source)
        .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
    write_input(probe, name, &bytes)
}

/// Write bytes into the input snapshot and return their in-boundary path.
///
/// # Errors
///
/// Returns a diagnostic when the file cannot be written.
fn write_input(probe: &Probe, name: &str, bytes: &[u8]) -> Result<String, String> {
    let target = probe.inputs().join(name);
    ambient::write(&target, bytes)
        .map_err(|error| format!("cannot write {}: {error}", target.display()))?;
    Ok(format!("/job/in/{name}"))
}

/// The Boolean OPA decided.
///
/// # Errors
///
/// Returns a diagnostic when the output is not exactly one Boolean result.
fn decided(stdout: &str, entry: &str) -> Result<bool, String> {
    let document = parse_json(stdout, "OPA stdout")?;
    if document.get("errors").is_some() {
        return Err(format!("OPA reported errors for {entry}: {stdout}"));
    }
    let results = document
        .get("result")
        .and_then(Json::as_array)
        .ok_or_else(|| format!("OPA printed no result array for {entry}: {stdout}"))?;
    if results.len() != 1 {
        return Err(format!(
            "OPA must print exactly one result, printed {} for {entry}: {stdout}",
            results.len()
        ));
    }
    let expressions = results
        .first()
        .and_then(|result| result.get("expressions"))
        .and_then(Json::as_array)
        .ok_or_else(|| format!("OPA printed no expressions for {entry}: {stdout}"))?;
    if expressions.len() != 1 {
        return Err(format!(
            "OPA must print exactly one expression, printed {} for {entry}: {stdout}",
            expressions.len()
        ));
    }
    expressions
        .first()
        .and_then(|expression| expression.get("value"))
        .and_then(Json::as_bool)
        .ok_or_else(|| format!("OPA printed no Boolean value for {entry}: {stdout}"))
}

/// Run OPA on the host, outside any boundary.
///
/// # Errors
///
/// Returns a diagnostic when OPA cannot be run or exits non-zero.
fn opa_outside(opa: &Path, policy: &Path, request: &Path, entry: &str) -> Result<bool, String> {
    let output = Command::new(opa)
        .args([
            "eval",
            "--format",
            "json",
            "--strict-builtin-errors",
            "-d",
            &policy.display().to_string(),
            "--input",
            &request.display().to_string(),
            entry,
        ])
        .env_clear()
        .current_dir(repository_path("."))
        .output()
        .map_err(|error| format!("cannot run {}: {error}", opa.display()))?;
    if !output.status.success() {
        return Err(format!(
            "OPA {entry} failed outside the boundary with {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    decided(&String::from_utf8_lossy(&output.stdout), entry)
}

/// One OPA query: where its policy and request are bound, and its entry point.
struct Query<'a> {
    /// The policy's in-boundary path.
    policy: &'a str,
    /// The request's in-boundary path.
    request: &'a str,
    /// The query entry point.
    entry: &'a str,
}

/// Run one OPA query inside the boundary.
///
/// # Errors
///
/// Returns a diagnostic when the job could not be observed.
fn opa_inside(probe: &Probe, opa: &Path, query: &Query<'_>) -> Result<Run, String> {
    let tool = ToolJob {
        host_program: opa,
        sandbox_program: OPA,
        args: vec![
            "eval".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--strict-builtin-errors".to_owned(),
            "-d".to_owned(),
            query.policy.to_owned(),
            "--input".to_owned(),
            query.request.to_owned(),
            query.entry.to_owned(),
        ],
        binds: vec![ReadOnlyBind {
            source: opa.to_path_buf(),
            target: OPA.to_owned(),
        }],
        env: Vec::new(),
    };
    let options = Options::new(Boundary::Bubblewrap)
        .memory(256 * 1024 * 1024)
        .deadline(Duration::from_secs(30));
    probe.run_tool(&tool, &options)
}

/// Pinned OPA decides the same Booleans inside and outside the boundary.
#[test]
#[ignore = "run by make boundary-probe; needs make tools and starts supervised jobs"]
fn opa_decides_the_same_inside_and_outside() -> Result<(), String> {
    let opa = pinned_opa()?;
    let probe = Probe::new("boundary-tools-opa")?;
    let source = baseline_source_path();
    let inside_policy = copy_input(&probe, &source, "policy.rego")?;
    let cross_tenant = probe.scratch_path().join("cross-tenant.json");
    ambient::write(&cross_tenant, CROSS_TENANT_ADMIN_REQUEST.as_bytes())
        .map_err(|error| format!("cannot write the cross-tenant request: {error}"))?;
    for (outside_request, inside_request, expected) in [
        (
            repository_path("examples/tenant-write/request.json"),
            copy_input(
                &probe,
                &repository_path("examples/tenant-write/request.json"),
                "request.json",
            )?,
            true,
        ),
        (
            cross_tenant.clone(),
            write_input(
                &probe,
                "cross-tenant.json",
                CROSS_TENANT_ADMIN_REQUEST.as_bytes(),
            )?,
            false,
        ),
    ] {
        let outside = opa_outside(&opa, &source, &outside_request, ENTRY)?;
        check_eq!(
            outside,
            expected,
            "OPA outside the boundary must decide the expected Boolean"
        );
        let run = opa_inside(
            &probe,
            &opa,
            &Query {
                policy: &inside_policy,
                request: &inside_request,
                entry: ENTRY,
            },
        )?;
        require_outcome(&run, &Outcome::Completed)?;
        let inside = decided(&String::from_utf8_lossy(&run.stdout), ENTRY)?;
        check_eq!(
            inside,
            outside,
            "OPA inside the boundary must decide what it decided outside"
        );
    }
    Ok(())
}

/// An OPA query calling `http.send` reaches no host listener.
#[test]
#[ignore = "run by make boundary-probe; needs make tools and starts supervised jobs"]
fn opa_cannot_reach_a_host_listener() -> Result<(), String> {
    let opa = pinned_opa()?;
    let probe = Probe::new("boundary-tools-http")?;
    let (endpoints, watchers) = listeners::listen(probe.nonce())?;
    let port = endpoints
        .tcp
        .rsplit_once(':')
        .map(|(_host, port)| port.to_owned())
        .ok_or_else(|| format!("the listener address {:?} has no port", endpoints.tcp))?;
    let source = HTTP_POLICY.replace("PORT", &port);
    let policy = write_input(&probe, "http.rego", source.as_bytes())?;
    let request = write_input(&probe, "request.json", b"{}")?;
    let run = opa_inside(
        &probe,
        &opa,
        &Query {
            policy: &policy,
            request: &request,
            entry: "data.probe.http.allow",
        },
    )?;
    watchers.require_silent()?;
    check!(
        run.bounds.is_empty(),
        "the http.send failure must not be a bound: {:?}",
        run.bounds
    );
    check!(
        matches!(run.exit, ExitForm::Exited(code) if code != 0),
        "OPA must exit with an error after the http.send failure, got {:?} and stdout {:?}",
        run.exit,
        String::from_utf8_lossy(&run.stdout)
    );
    check_eq!(
        run.outcome,
        Outcome::ChildFailed(run.exit),
        "the outcome must be the child's own failure, not containment or a bound"
    );
    let detail = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    check!(
        detail.contains("http.send"),
        "the diagnostic must name the failed http.send, got {detail:?}"
    );
    Ok(())
}

/// Pinned Lean checks the smoke module inside the 2 GiB and 60 s defaults.
#[test]
#[ignore = "run by make boundary-probe; needs make tools and starts supervised jobs"]
fn lean_checks_the_smoke_module() -> Result<(), String> {
    let root = pinned_lean()?;
    let probe = Probe::new("boundary-tools-lean")?;
    let smoke = repository_path("lean/PoleanSmoke.lean");
    let smoke_text = ambient::read_to_string(&smoke)
        .map_err(|error| format!("cannot read {}: {error}", smoke.display()))?;
    let target = probe.inputs().join("PoleanSmoke.lean");
    ambient::write(&target, smoke_text.as_bytes())
        .map_err(|error| format!("cannot write the smoke module: {error}"))?;
    let tool = ToolJob {
        host_program: &root.join("bin").join("lean"),
        sandbox_program: LEAN,
        args: vec!["/job/in/PoleanSmoke.lean".to_owned()],
        binds: vec![ReadOnlyBind {
            source: root.clone(),
            target: LEAN_SYSROOT.to_owned(),
        }],
        env: vec![("LEAN_SYSROOT".to_owned(), LEAN_SYSROOT.to_owned())],
    };
    let options = Options::new(Boundary::Bubblewrap)
        .memory(2_147_483_648)
        .scratch(64 * 1024 * 1024)
        .deadline(Duration::from_secs(60));
    let run = probe.run_tool(&tool, &options)?;
    let stderr = String::from_utf8_lossy(&run.stderr);
    check!(
        !stderr.contains("error"),
        "Lean must check the smoke module without errors; stderr was {stderr:?}"
    );
    require_outcome(&run, &Outcome::Completed)
}
