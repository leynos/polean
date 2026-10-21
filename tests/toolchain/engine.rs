//! The Regorus runtime configuration `contracts/profile.json` requires.
//!
//! The structural observations and the BDD decisions both build their engine
//! here, so one of them cannot observe a differently configured Regorus from
//! the other. The options are written out in full rather than left to defaults
//! so that a change of default cannot silently change what either means.

use regorus::{Engine, Value};

/// The entry point named by `contracts/profile.json`.
pub const ENTRYPOINT: &str = "data.authz.allow";

/// Source name recorded for the baseline in the AST and in diagnostics.
pub const POLICY_FILE: &str = "policy.rego";

/// A Regorus engine configured as `contracts/profile.json` requires.
///
/// # Errors
///
/// Returns a diagnostic when the source does not parse under those options.
pub fn engine_for(source: &str) -> Result<Engine, String> {
    let mut engine = Engine::new();
    engine.set_rego_v0(false);
    engine.set_strict_builtin_errors(true);
    engine
        .add_policy(POLICY_FILE.to_owned(), source.to_owned())
        .map_err(|error| format!("add_policy rejected the source: {error}"))?;
    Ok(engine)
}

/// Set `request` as `input` and evaluate the profile entry point.
///
/// # Errors
///
/// Returns a diagnostic when the request is not valid JSON or the evaluation
/// fails.
pub fn evaluate(engine: &mut Engine, request: &str) -> Result<Value, String> {
    engine
        .set_input_json(request)
        .map_err(|error| format!("set_input_json rejected the request: {error}"))?;
    engine
        .eval_rule(ENTRYPOINT.to_owned())
        .map_err(|error| format!("eval_rule failed: {error}"))
}

/// Evaluate the profile entry point on `source` and `request`.
///
/// Parsing and evaluation are combined because O3 only needs to know whether a
/// decision was produced; the exact failure point is recorded separately.
///
/// # Errors
///
/// Returns the parse error or the evaluation error.
pub fn decide(source: &str, request: &str) -> Result<Value, String> {
    let mut engine = engine_for(source)?;
    evaluate(&mut engine, request)
}
