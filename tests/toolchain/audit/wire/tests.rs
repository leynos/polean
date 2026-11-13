//! Key encoding and positive decoding of the shared contract samples.
//!
//! These are the wire claims that are not about refusal: the encoding is
//! injective on the cases that matter, the `native` flag tracks the prefix, the
//! pinned keys are the encodings of real names, and the Python harness's
//! samples decode here. The last is the A3 agreement test: the design harness
//! and the Rust decoder read the same bytes, so a schema and a decoder that
//! drift apart fail on one side or the other.

use rstest::rstest;

use super::{
    Component,
    Name,
    PIN,
    Tree,
    decode_report,
    decode_request,
    is_native,
    name_key,
    tree_key,
};
use crate::audit::settlement::decision::{DeclKind, K_CHOICE, K_PROPEXT, K_QUOT, K_SORRY, Replay};

/// The shared report sample, which the Python harness validates too.
const SAMPLE_REPORT: &str = include_str!("../../../../examples/audit/sample-report.json");

/// The shared request sample, which the Python harness validates too.
const SAMPLE_REQUEST: &str = include_str!("../../../../examples/audit/sample-request.json");

/// Build a name from text components.
fn text(parts: &[&str]) -> Name {
    parts
        .iter()
        .map(|part| Component::Text((*part).to_owned()))
        .collect()
}

/// A constant tree with the given universe levels.
fn constant(parts: &[&str], levels: &[u32]) -> Tree {
    Tree::Const {
        name: text(parts),
        levels: levels.to_vec(),
    }
}

/// An application of one tree to another.
fn app(left: Tree, right: Tree) -> Tree { Tree::App(Box::new(left), Box::new(right)) }

/// Each pinned key is the encoding of the real name it depends on.
///
/// Settlement compares keys against these literals, so a broken encoding would
/// make every axiom look undeclared rather than wrong.
#[rstest]
#[case::propext(&["propext"], &K_PROPEXT)]
#[case::quot_sound(&["Quot", "sound"], &K_QUOT)]
#[case::classical_choice(&["Classical", "choice"], &K_CHOICE)]
#[case::sorry(&["sorryAx"], &K_SORRY)]
fn name_key_reproduces_the_pinned_literal(#[case] parts: &[&str], #[case] expected: &[u8]) {
    assert_eq!(
        name_key(&text(parts)),
        expected.to_vec(),
        "the pinned key of {parts:?} is not the encoding of the real name"
    );
}

/// A dotted text component is not the same name as two components.
#[test]
fn dotted_text_differs_from_two_components() {
    assert_ne!(
        name_key(&text(&["Quot.sound"])),
        name_key(&text(&["Quot", "sound"])),
        "a dotted component collided with two components"
    );
}

/// A numeric component is not the same name as its decimal text.
#[test]
fn numeric_component_differs_from_text() {
    assert_ne!(
        name_key(&[Component::Number(1)]),
        name_key(&text(&["1"])),
        "a numeric component collided with its decimal spelling"
    );
}

/// The native flag fires on any component with the prefix.
#[test]
fn native_flag_tracks_the_prefix() {
    assert!(
        is_native(&text(&["_native", "decide"])),
        "a leading native component was not classified"
    );
    assert!(
        is_native(&text(&["A", "_native", "decide"])),
        "a nested native component was not classified"
    );
    assert!(
        !is_native(&text(&["native", "decide"])),
        "a component without the underscore prefix was classified as native"
    );
    assert!(
        !is_native(&[Component::Number(7)]),
        "a numeric component was classified as native"
    );
}

/// Universe levels are part of a constant tree's identity.
#[test]
fn tree_keys_distinguish_levels() {
    assert_ne!(
        tree_key(&constant(&["AuditFixture", "Goal"], &[])),
        tree_key(&constant(&["AuditFixture", "Goal"], &[0])),
        "a constant with and without a universe level collided"
    );
}

/// Application order is part of a tree's identity.
#[test]
fn tree_keys_distinguish_application_order() {
    let left = constant(&["AuditFixture", "Left"], &[]);
    let right = constant(&["AuditFixture", "Right"], &[]);
    assert_ne!(
        tree_key(&app(left.clone(), right.clone())),
        tree_key(&app(right, left)),
        "a tree collided with its operand swap"
    );
}

/// The shared report sample decodes into the values settlement compares.
#[test]
fn decode_report_accepts_the_shared_sample() {
    let report = match decode_report(SAMPLE_REPORT.as_bytes()) {
        Ok(report) => report,
        Err(error) => panic!("the shared report sample did not decode: {error:?}"),
    };
    assert_eq!(
        report.githash,
        PIN.as_bytes().to_vec(),
        "the sample report's toolchain identity changed"
    );
    assert!(
        matches!(report.replay, Replay::Accepted(_)),
        "the sample report's replay is no longer accepted"
    );
    let Some(root) = report.roots.first() else {
        panic!("the sample report has no roots");
    };
    let Some(decl) = &root.decl else {
        panic!("the sample report's root has no declaration");
    };
    assert_eq!(
        decl.kind,
        DeclKind::Theorem,
        "the sample report's root is no longer a theorem"
    );
    assert_eq!(
        decl.statement,
        Some(tree_key(&constant(&["AuditFixture", "Goal"], &[]))),
        "the sample report's statement is not the fixture goal"
    );
    assert_eq!(
        decl.axioms.len(),
        3,
        "the sample report no longer lists the three foundations"
    );
}

/// The shared request sample decodes into the values settlement compares.
#[test]
fn decode_request_accepts_the_shared_sample() {
    let request = match decode_request(SAMPLE_REQUEST.as_bytes()) {
        Ok(request) => request,
        Err(error) => panic!("the shared request sample did not decode: {error:?}"),
    };
    assert_eq!(
        request.module,
        name_key(&text(&["AuditFixture", "AcceptedFoundations"])),
        "the sample request's module changed"
    );
    assert_eq!(
        request.foundations.len(),
        3,
        "the sample request no longer declares the three foundations"
    );
    assert_eq!(
        request.roots.len(),
        1,
        "the sample request no longer names one root"
    );
}
