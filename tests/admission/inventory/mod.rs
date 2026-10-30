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

use crate::frontend;

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
/// The observation parses, serializes, and then walks the typed AST. Parsing is
/// attempted before serialization because a source that does not parse has no
/// serializable AST to report on.
pub fn observe(source: &str) -> Observation {
    let mut engine = match frontend::parse(source) {
        Ok(engine) => engine,
        Err(message) => return Observation::Rejected(message),
    };
    if let Err(error) = engine.get_ast_as_json() {
        return Observation::SerializeFailed(error.to_string());
    }
    Observation::Parsed(count(&mut engine))
}

/// Count the node kinds in every module the engine holds.
///
/// This is the reference side of O1: the typed walk's multiset of kinds, which
/// the JSON decoder's multiset must equal for every fixture.
#[must_use]
pub fn count(engine: &mut Engine) -> KindCounts {
    let mut counts = KindCounts::new();
    for module in engine.get_modules() {
        walk::visit_module(module, &mut counts);
    }
    counts
}

#[cfg(test)]
mod tests;
