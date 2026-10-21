//! O4 Regorus decisions, bound to `tests/features/regorus_baseline.feature`.
//!
//! The feature file is the readable specification: four requests, each with a
//! decision the admitted model predicts by hand from
//! `T ∧ W ∧ (A ∨ (O ∧ ¬L))`. The steps here load the baseline source into a
//! freshly configured engine and evaluate `data.authz.allow` on each request.
//!
//! A decision that agrees on four requests is evidence about this build of
//! Regorus. It is not admission of the source, and it is not correspondence
//! between the source and the model.
//!
//! The state is a `Slot`-bearing struct rather than a `&mut` world fixture:
//! `rstest-bdd` 0.6.0 documents a rustc internal compiler error for the
//! macro-driven mutable world on the pinned nightly, so every step borrows the
//! state immutably and fills one slot.

/// The per-scenario state fixture.
///
/// `rstest`'s fixture expansion wraps a single-expression body in braces, which
/// `unused_braces` rejects. Upstream attach the matching expectation from a
/// helper macro that is not published for downstream users, and an expectation
/// written on the fixture item itself is reported as unfulfilled because the
/// lint arises in a separate generated item. The expectation is scoped to this
/// module so it covers only the generation it is about, and still warns if a
/// future release stops emitting the braces.
mod fixture {
    #![expect(
        unused_braces,
        reason = "rstest fixture expansion wraps a single-expression body in braces"
    )]

    use rstest::fixture;

    use super::RegorusState;

    /// The per-scenario state, built empty for each generated test.
    #[fixture]
    pub fn state() -> RegorusState { RegorusState::default() }
}

pub use fixture::state;
use regorus::{Engine, Value};
use rstest_bdd::Slot;
use rstest_bdd_macros::{ScenarioState, given, scenario, then, when};

use crate::{
    engine::engine_for,
    support::{
        BASELINE_SOURCE,
        CROSS_TENANT_ADMIN_REQUEST,
        OWNER_READ_REQUEST,
        OWNER_UNLOCKED_REQUEST,
        SAME_TENANT_ADMIN_LOCKED_REQUEST,
    },
};

/// Cargo recompiles this target when the specification changes.
///
/// `#[scenario]` reads the feature file itself, which Cargo cannot fingerprint,
/// so without these references an edited feature file can leave a previously
/// expanded binary in place and scenarios that no longer parse would pass.
const _: [&str; 1] = [include_str!("../features/regorus_baseline.feature")];

/// The state one scenario accumulates.
///
/// Each field is a `Slot`, so a step can fill it through an immutable borrow.
#[derive(Default, ScenarioState)]
struct RegorusState {
    /// The engine, configured as the profile requires and loaded with the
    /// baseline source.
    engine: Slot<Engine>,
    /// The request the next evaluation will read as `input`.
    request: Slot<String>,
    /// The decision produced by the most recent evaluation.
    decision: Slot<Value>,
}

/// Load the baseline source with the profile's runtime options.
#[given(
    "the baseline tenant-write source loaded into Regorus with Rego v1 and strict built-in errors"
)]
fn load_baseline(state: &RegorusState) -> Result<(), String> {
    state.engine.set(engine_for(BASELINE_SOURCE)?);
    Ok(())
}

/// Select the request the next evaluation reads as `input`.
fn select_request(state: &RegorusState, request: &str) { state.request.set(request.to_owned()); }

/// The owner-unlocked write request: `T W ¬A O ¬L`.
#[given("the owner-unlocked write request")]
fn owner_unlocked_write_request(state: &RegorusState) {
    select_request(state, OWNER_UNLOCKED_REQUEST);
}

/// The cross-tenant administrator write request: `¬T W A ¬O ¬L`.
#[given("the cross-tenant administrator write request")]
fn cross_tenant_administrator_write_request(state: &RegorusState) {
    select_request(state, CROSS_TENANT_ADMIN_REQUEST);
}

/// The same-tenant administrator locked write request: `T W A ¬O L`.
#[given("the same-tenant administrator locked write request")]
fn same_tenant_administrator_locked_write_request(state: &RegorusState) {
    select_request(state, SAME_TENANT_ADMIN_LOCKED_REQUEST);
}

/// The owner read request: `T ¬W ¬A O ¬L`.
#[given("the owner read request")]
fn owner_read_request(state: &RegorusState) { select_request(state, OWNER_READ_REQUEST); }

/// Set the selected request as input and evaluate the profile entry point.
#[when("Regorus evaluates data.authz.allow")]
fn evaluate_entrypoint(state: &RegorusState) -> Result<(), String> {
    let request = state
        .request
        .with_ref(String::clone)
        .ok_or_else(|| "no request was selected before evaluation".to_owned())?;
    let outcome = state
        .engine
        .with_mut(|engine| crate::engine::evaluate(engine, &request))
        .ok_or_else(|| "the engine was not loaded before evaluation".to_owned())?;
    state.decision.set(outcome?);
    Ok(())
}

/// Require the decision to be exactly the expected Boolean.
///
/// `Value` has more shapes than a Boolean, so an undefined result or a string
/// `"true"` fails rather than being coerced.
fn expect_decision(state: &RegorusState, expected: bool) -> Result<(), String> {
    let decision = state
        .decision
        .with_ref(Value::clone)
        .ok_or_else(|| "no decision was produced before the assertion".to_owned())?;
    if decision != Value::Bool(expected) {
        return Err(format!(
            "the decision must be exactly {expected}, got {decision:?}"
        ));
    }
    Ok(())
}

/// The decision is exactly `true`.
#[then("the decision is exactly true")]
fn decision_is_true(state: &RegorusState) -> Result<(), String> { expect_decision(state, true) }

/// The decision is exactly `false`.
#[then("the decision is exactly false")]
fn decision_is_false(state: &RegorusState) -> Result<(), String> { expect_decision(state, false) }

/// An owner may write an unlocked resource in their tenant.
#[scenario(
    path = "tests/features/regorus_baseline.feature",
    name = "An owner may write an unlocked resource in their tenant"
)]
fn owner_may_write_unlocked_resource(#[from(state)] _state: RegorusState) {}

/// A cross-tenant administrator write is denied.
#[scenario(
    path = "tests/features/regorus_baseline.feature",
    name = "A cross-tenant administrator write is denied"
)]
fn cross_tenant_administrator_write_is_denied(#[from(state)] _state: RegorusState) {}

/// An administrator may write a locked resource in their tenant.
#[scenario(
    path = "tests/features/regorus_baseline.feature",
    name = "An administrator may write a locked resource in their tenant"
)]
fn same_tenant_administrator_may_write_locked_resource(#[from(state)] _state: RegorusState) {}

/// An owner read is denied.
#[scenario(
    path = "tests/features/regorus_baseline.feature",
    name = "An owner read is denied"
)]
fn owner_read_is_denied(#[from(state)] _state: RegorusState) {}
