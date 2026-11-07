//! Generators, reference encodings, and document helpers for A1 and A2.
//!
//! The properties in [`super::keys`] and [`super::contract`] share these so the
//! reference encodings and the mutated documents are defined once. Every helper
//! here is deterministic: the seeded runner fixes the draw order, and the near
//! edits are total functions over the generated values.

use proptest::{
    arbitrary::any,
    collection::vec as strategy_vec,
    prelude::{Strategy, prop_oneof},
    sample::select,
    test_runner::{Config, FileFailurePersistence, RngAlgorithm, TestRng, TestRunner},
};
use serde_json::{Map, Value};

use crate::audit::wire::{Component, Name, Tree};

/// How many generated cases each property checks.
pub(super) const CASES: u32 = 64;

/// The shared report sample, which the Python harness validates too.
const SAMPLE_REPORT: &str = include_str!("../../../../examples/audit/sample-report.json");

/// The top-level fields the report contract requires.
pub(super) const REQUIRED: [&str; 5] = ["schema_version", "githash", "module", "replay", "roots"];

/// Text components chosen to collide under a careless encoding.
///
/// The empty string, a dotted name, the native prefix, and the pieces of the
/// seeded `["ab", "c"]` / `["a", "bc"]` pair are all present. Independent draws
/// rarely collide, so the near-name property derives its second input by an
/// edit that a length-free encoding cannot see; these values make that edit
/// produce the ambiguous pairs worth testing.
const TRICKY: &[&str] = &[
    "",
    "a",
    "b",
    "ab",
    "c",
    "bc",
    "Quot",
    "sound",
    "Quot.sound",
    "_native",
    "0",
    "9",
    "é",
    "A'",
];

/// A shrinking runner with a fixed seed and no persisted regressions.
///
/// The deterministic seed keeps the run reproducible. Persistence is off
/// because these properties are cheap to re-run and a persisted case would add
/// an untracked file to the worktree.
fn runner(cases: u32) -> TestRunner {
    let config = Config {
        cases,
        failure_persistence: Some(Box::new(FileFailurePersistence::Off)),
        ..Config::default()
    };
    TestRunner::new_with_rng(config, TestRng::deterministic_rng(RngAlgorithm::ChaCha))
}

/// One name component: usually tricky text, sometimes a number.
fn component_strategy() -> impl Strategy<Value = Component> {
    prop_oneof![
        4 => select(TRICKY).prop_map(|text| Component::Text(text.to_owned())),
        1 => any::<u64>().prop_map(Component::Number),
    ]
}

/// A name of one to four components.
pub(super) fn name_strategy() -> impl Strategy<Value = Name> {
    strategy_vec(component_strategy(), 1..5)
}

/// A ground statement tree of bounded size.
pub(super) fn tree_strategy() -> impl Strategy<Value = Tree> {
    let leaf = (name_strategy(), strategy_vec(any::<u32>(), 0..3))
        .prop_map(|(name, levels)| Tree::Const { name, levels });
    leaf.prop_recursive(4, 32, 2, |inner| {
        (inner.clone(), inner).prop_map(|(left, right)| Tree::App(Box::new(left), Box::new(right)))
    })
}

/// A tag-free, length-free name encoding: the reference the key must beat.
#[expect(
    clippy::little_endian_bytes,
    reason = "the seeded naive encoding is compared against the real one, so it must match its \
              byte order"
)]
pub(super) fn naive_name(name: &Name) -> Vec<u8> {
    let mut bytes = Vec::new();
    for component in name {
        match component {
            Component::Text(text) => bytes.extend_from_slice(text.as_bytes()),
            Component::Number(number) => bytes.extend_from_slice(&number.to_le_bytes()),
        }
    }
    bytes
}

/// A tag-free, length-free tree encoding built on [`naive_name`].
#[expect(
    clippy::little_endian_bytes,
    reason = "the seeded naive encoding is compared against the real one, so it must match its \
              byte order"
)]
pub(super) fn naive_tree(tree: &Tree) -> Vec<u8> {
    fn walk(tree: &Tree, bytes: &mut Vec<u8>) {
        match tree {
            Tree::Const { name, levels } => {
                bytes.extend_from_slice(&naive_name(name));
                for level in levels {
                    bytes.extend_from_slice(&level.to_le_bytes());
                }
            }
            Tree::App(left, right) => {
                walk(left, bytes);
                walk(right, bytes);
            }
        }
    }
    let mut bytes = Vec::new();
    walk(tree, &mut bytes);
    bytes
}

/// The first position of two adjacent text components, if any.
fn adjacent_text_pair(name: &Name) -> Option<usize> {
    name.windows(2)
        .position(|pair| matches!(pair, [Component::Text(_), Component::Text(_)]))
}

/// The first text component long enough to split, if any.
fn splittable_text(name: &Name) -> Option<usize> {
    name.iter().position(
        |component| matches!(component, Component::Text(text) if text.chars().count() >= 2),
    )
}

/// A different name that [`naive_name`] encodes identically.
///
/// The possible edits are the ones a length-free encoding cannot see: merging
/// two adjacent text components, splitting one at a character boundary, and
/// inserting an empty component. The fallback is always available, so the
/// result is always a distinct name with the same naive bytes.
pub(super) fn near_name(name: &Name) -> Name {
    let mut near = name.clone();
    if let Some(index) = adjacent_text_pair(&near) {
        let first = near.remove(index);
        let second = near.remove(index);
        let (Component::Text(mut left), Component::Text(right)) = (first, second) else {
            panic!("the index names an adjacent text pair");
        };
        left.push_str(&right);
        near.insert(index, Component::Text(left));
    } else if let Some(index) = splittable_text(&near) {
        let Some(Component::Text(text)) = near.get(index).cloned() else {
            panic!("the index names a text component");
        };
        let Some((cut, _)) = text.char_indices().nth(1) else {
            panic!("the index names a text component of at least two scalars");
        };
        let (left, right) = text.split_at(cut);
        near.splice(
            index..=index,
            [
                Component::Text(left.to_owned()),
                Component::Text(right.to_owned()),
            ],
        );
    } else {
        near.insert(0, Component::Text(String::new()));
    }
    near
}

/// The same tree with one constant's name replaced by a [`near_name`].
pub(super) fn near_tree(tree: &Tree) -> Tree {
    match tree {
        Tree::Const { name, levels } => Tree::Const {
            name: near_name(name),
            levels: levels.clone(),
        },
        Tree::App(left, right) => Tree::App(Box::new(near_tree(left)), right.clone()),
    }
}

/// The report sample as a mutable JSON object.
pub(super) fn valid_document() -> Map<String, Value> {
    let Ok(Value::Object(document)) = serde_json::from_str(SAMPLE_REPORT) else {
        panic!("the shared report sample is not a JSON object");
    };
    document
}

/// Serialize a mutated document.
fn encode(document: &Map<String, Value>) -> Vec<u8> {
    match serde_json::to_vec(&Value::Object(document.clone())) {
        Ok(bytes) => bytes,
        Err(error) => panic!("a mutated document must serialize: {error}"),
    }
}

/// The required field at `index`, or a panic naming the defect.
pub(super) fn required(index: usize) -> &'static str {
    let Some(key) = REQUIRED.get(index) else {
        panic!("the strategy produced an index outside the required fields");
    };
    key
}

/// A report with one required field removed.
pub(super) fn delete_field(index: usize) -> Vec<u8> {
    let mut document = valid_document();
    document.remove(required(index));
    encode(&document)
}

/// A report with one required field renamed to an unknown name.
pub(super) fn rename_field(index: usize) -> Vec<u8> {
    let key = required(index);
    let mut document = valid_document();
    let Some(value) = document.remove(key) else {
        panic!("the shared report sample is missing {key}");
    };
    document.insert(format!("{key}_renamed"), value);
    encode(&document)
}

/// A report with one required field replaced by a value of another type.
pub(super) fn retype_field(index: usize) -> Vec<u8> {
    let key = required(index);
    let replacement = match key {
        "githash" => Value::Array(Vec::new()),
        "replay" => Value::from("accepted"),
        "schema_version" | "module" | "roots" => Value::from(5),
        other => panic!("unexpected required field {other}"),
    };
    let mut document = valid_document();
    document.insert(key.to_owned(), replacement);
    encode(&document)
}

/// Run a property and report a failure as a panic naming the shrunk case.
pub(super) fn check<S>(
    strategy: S,
    property: impl Fn(S::Value) -> Result<(), proptest::test_runner::TestCaseError>,
) where
    S: Strategy,
{
    if let Err(error) = runner(CASES).run(&strategy, property) {
        panic!("the audit property failed: {error}");
    }
}
