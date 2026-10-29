//! Polean's own inventory of the pinned Regorus AST.
//!
//! The spike must show that every AST variant the pinned frontend can produce
//! has been considered. Regorus exposes its typed AST only through the
//! doc-hidden [`regorus::unstable`] module, so the spike gives each variant a
//! Polean-owned name and reaches the variants through a walk of the module that
//! `Engine::get_modules` returns.
//!
//! The walk is deliberately exhaustive: no arm has a wildcard, so a Regorus
//! release that adds a variant stops compiling rather than silently widening
//! what the observation does not see. `Expr::OrExpr` is absent from
//! [`NodeKind`] because the pinned feature set (`ast`, `std`) does not enable
//! `rego-extensions`; enabling it would break the exhaustive match, which is
//! the intended alarm.
//!
//! This module only observes. It makes no admission decision: a parsed source
//! with every kind observed is not thereby admitted to the profile.

use std::collections::BTreeMap;

use regorus::Engine;

mod kinds;
mod walk;

pub use kinds::NodeKind;

/// How many times each kind occurred in one module.
pub type KindCounts = BTreeMap<NodeKind, usize>;

/// What the pinned frontend did with one source.
#[derive(Debug)]
pub enum Observation {
    /// Regorus rejected the source.
    Rejected(String),
    /// Regorus parsed the source but could not serialize its AST.
    SerializeFailed(String),
    /// Regorus parsed and serialized the source.
    Parsed(KindCounts),
}

/// Observe one source with the pinned frontend.
///
/// The engine is configured as the profile requires: Rego v1, not v0. The
/// observation parses, serializes, and then walks the typed AST. Parsing is
/// attempted before serialization because a source that does not parse has no
/// serializable AST to report on.
pub fn observe(source: &str) -> Observation {
    let mut engine = Engine::new();
    engine.set_rego_v0(false);
    if let Err(error) = engine.add_policy("fixture.rego".to_owned(), source.to_owned()) {
        return Observation::Rejected(error.to_string());
    }
    if let Err(error) = engine.get_ast_as_json() {
        return Observation::SerializeFailed(error.to_string());
    }
    let mut counts = KindCounts::new();
    for module in engine.get_modules() {
        walk::visit_module(module, &mut counts);
    }
    Observation::Parsed(counts)
}

#[cfg(test)]
mod tests;
