//! Tests for the Polean-owned inventory table.
//!
//! These check the table itself, not the frontend: that every kind has its own
//! ordinal and name, and that a manifest name resolves back to the same kind.
//! Frontend observations live in [`crate::observations`].

use std::collections::BTreeSet;

use crate::{
    ast,
    corpus,
    frontend,
    inventory::{self, KindCounts, NodeKind},
};

#[test]
fn every_kind_has_a_distinct_ordinal() {
    let ordinals: BTreeSet<usize> = NodeKind::ALL.iter().map(|kind| kind.ordinal()).collect();
    let expected: BTreeSet<usize> = (0..NodeKind::ALL.len()).collect();
    assert_eq!(
        ordinals,
        expected,
        "ordinals must cover 0..{} exactly once",
        NodeKind::ALL.len()
    );
}

#[test]
fn every_kind_has_a_distinct_name() {
    let names: BTreeSet<&str> = NodeKind::ALL.iter().map(|kind| kind.name()).collect();
    assert_eq!(
        names.len(),
        NodeKind::ALL.len(),
        "kind names must be unique; duplicates cannot be written to the manifest"
    );
}

#[test]
fn a_manifest_name_resolves_to_its_kind() {
    for kind in NodeKind::ALL {
        assert_eq!(
            NodeKind::from_name(kind.name()),
            Some(kind),
            "{} must resolve to itself",
            kind.name()
        );
    }
}

#[test]
fn an_unknown_manifest_name_resolves_to_nothing() {
    assert_eq!(NodeKind::from_name("ExprOr"), None);
}

/// The manifest entries the O1 test reads.
fn manifest() -> Vec<corpus::Entry> {
    match corpus::entries() {
        Ok(entries) => entries,
        Err(error) => panic!("examples/admission/manifest.json must parse: {error}"),
    }
}

/// The typed and decoded kind counts of one fixture.
///
/// `Ok(None)` means the pinned frontend declined the source, so there is no
/// tree to compare. `Err` means the decoder refused a tree the frontend
/// produced, which is itself a disagreement.
fn kinds(entry: &corpus::Entry) -> Result<Option<(KindCounts, KindCounts)>, String> {
    let source = corpus::source(entry).map_err(|error| format!("{}: {error}", entry.label()))?;
    let Ok(mut engine) = frontend::parse(&source) else {
        return Ok(None);
    };
    let Ok(ast_json) = engine.get_ast_as_json() else {
        return Ok(None);
    };
    let typed = inventory::count(&mut engine);
    match ast::decode(&source, &ast_json) {
        Ok(module) => Ok(Some((typed, ast::count_module(&module)))),
        Err(fault) => Err(format!(
            "{}: the decoder rejected a parsed fixture: {}",
            entry.label(),
            fault.describe()
        )),
    }
}

/// The kinds whose counts differ between the two walks of one fixture.
fn mismatches(entry: &corpus::Entry, typed: &KindCounts, decoded: &KindCounts) -> Vec<String> {
    let mut found = Vec::new();
    for kind in NodeKind::ALL {
        let walked = typed.get(&kind).copied().unwrap_or(0);
        let read = decoded.get(&kind).copied().unwrap_or(0);
        if walked != read {
            found.push(format!(
                "{}: {} observed {walked} time(s) by the typed walk, {read} by the decoder",
                entry.label(),
                kind.name()
            ));
        }
    }
    found
}

/// O1: the JSON decoder and the typed walk must see the same tree.
///
/// The typed walk is the reference: it visits the frontend's own types with an
/// exhaustive match. If the decoder drops a node, reads a different variant
/// arm, or stops descending into a field, the two multisets part company for
/// the fixture that exercises that shape.
#[test]
fn the_json_decoder_sees_the_same_kinds_as_the_typed_walk() {
    let mut compared = 0_usize;
    let mut failures = Vec::new();
    for entry in &manifest() {
        match kinds(entry) {
            Ok(Some((typed, decoded))) => {
                compared += 1;
                failures.extend(mismatches(entry, &typed, &decoded));
            }
            Ok(None) => {}
            Err(message) => failures.push(message),
        }
    }
    assert!(compared > 0, "the corpus must contain a parsed fixture");
    if !failures.is_empty() {
        let shown = failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        panic!(
            "{} kind-count disagreements; first differences:\n{shown}",
            failures.len()
        );
    }
}
