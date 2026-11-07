//! The A1 and A2 properties, and the seeded collisions they must catch.

use proptest::{
    arbitrary::any,
    collection::vec as strategy_vec,
    prelude::{Just, Strategy, prop_oneof},
    prop_assert,
    prop_assert_eq,
    sample::select,
    test_runner::{Config, FileFailurePersistence, RngAlgorithm, TestRng, TestRunner},
};
use serde_json::{Map, Value};

use crate::audit::wire::{Component, DecodeError, Name, Tree, decode_report, name_key, tree_key};

/// How many generated cases each property checks.
const CASES: u32 = 64;

/// The shared report sample, which the Python harness validates too.
const SAMPLE_REPORT: &str = include_str!("../../../../examples/audit/sample-report.json");

/// The top-level fields the report contract requires.
const REQUIRED: [&str; 5] = ["schema_version", "githash", "module", "replay", "roots"];

/// Text components chosen to collide under a careless encoding.
///
/// The empty string, a dotted name, the native prefix, and the pieces of the
/// seeded `["ab", "c"]` / `["a", "bc"]` pair are all present, so a generator
/// that drops a length prefix or a tag reaches a collision quickly.
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
fn name_strategy() -> impl Strategy<Value = Name> { strategy_vec(component_strategy(), 1..5) }

/// A ground statement tree of bounded size.
fn tree_strategy() -> impl Strategy<Value = Tree> {
    let leaf = (name_strategy(), strategy_vec(any::<u32>(), 0..3))
        .prop_map(|(name, levels)| Tree::Const { name, levels });
    leaf.prop_recursive(4, 32, 2, |inner| {
        (inner.clone(), inner).prop_map(|(left, right)| Tree::App(Box::new(left), Box::new(right)))
    })
}

/// The report sample as a mutable JSON object.
fn valid_document() -> Map<String, Value> {
    let Ok(Value::Object(document)) = serde_json::from_str(SAMPLE_REPORT) else {
        panic!("the shared report sample is not a JSON object");
    };
    document
}

/// Serialise a mutated document.
fn encode(document: &Map<String, Value>) -> Vec<u8> {
    match serde_json::to_vec(&Value::Object(document.clone())) {
        Ok(bytes) => bytes,
        Err(error) => panic!("a mutated document must serialise: {error}"),
    }
}

/// The required field at `index`, or a panic naming the defect.
fn required(index: usize) -> &'static str {
    let Some(key) = REQUIRED.get(index) else {
        panic!("the strategy produced an index outside the required fields");
    };
    key
}

/// A report with one required field removed.
fn delete_field(index: usize) -> Vec<u8> {
    let mut document = valid_document();
    document.remove(required(index));
    encode(&document)
}

/// A report with one required field renamed to an unknown name.
fn rename_field(index: usize) -> Vec<u8> {
    let key = required(index);
    let mut document = valid_document();
    let Some(value) = document.remove(key) else {
        panic!("the shared report sample is missing {key}");
    };
    document.insert(format!("{key}_renamed"), value);
    encode(&document)
}

/// A report with one required field replaced by a value of another type.
fn retype_field(index: usize) -> Vec<u8> {
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
fn check<S>(
    strategy: S,
    property: impl Fn(S::Value) -> Result<(), proptest::test_runner::TestCaseError>,
) where
    S: Strategy,
{
    if let Err(error) = runner(CASES).run(&strategy, property) {
        panic!("the audit property failed: {error}");
    }
}

/// Distinct names never share a key.
#[test]
fn name_keys_are_injective() {
    check((name_strategy(), name_strategy()), |(left, right)| {
        prop_assert!(
            name_key(&left) != name_key(&right) || left == right,
            "two distinct names shared a key: {:?} and {:?}",
            left,
            right
        );
        Ok(())
    });
}

/// Distinct statements never share a key.
#[test]
fn tree_keys_are_injective() {
    check((tree_strategy(), tree_strategy()), |(left, right)| {
        prop_assert!(
            tree_key(&left) != tree_key(&right) || left == right,
            "two distinct statements shared a key: {:?} and {:?}",
            left,
            right
        );
        Ok(())
    });
}

/// The seeded pair collides without length prefixes and not with them.
///
/// `["ab", "c"]` and `["a", "bc"]` concatenate to the same bytes, so an
/// encoding that merely joins its components cannot tell them apart. The key
/// must, because settlement compares keys and nothing else.
#[test]
fn length_prefixes_prevent_the_naive_collision() {
    let left = vec![
        Component::Text("ab".to_owned()),
        Component::Text("c".to_owned()),
    ];
    let right = vec![
        Component::Text("a".to_owned()),
        Component::Text("bc".to_owned()),
    ];
    #[expect(
        clippy::little_endian_bytes,
        reason = "the seeded naive encoding is compared against the real one, so it must match \
                  its byte order"
    )]
    let naive = |name: &Name| -> Vec<u8> {
        let mut bytes = Vec::new();
        for component in name {
            match component {
                Component::Text(text) => bytes.extend_from_slice(text.as_bytes()),
                Component::Number(number) => bytes.extend_from_slice(&number.to_le_bytes()),
            }
        }
        bytes
    };
    assert_eq!(
        naive(&left),
        naive(&right),
        "the seeded pair must collide without length prefixes"
    );
    assert_ne!(
        name_key(&left),
        name_key(&right),
        "the length prefix did not separate the seeded pair"
    );
}

/// Removing a required field is refused.
#[test]
fn deleting_a_required_field_is_refused() {
    check(0..REQUIRED.len(), |index| {
        let document = delete_field(index);
        prop_assert!(
            matches!(decode_report(&document), Err(DecodeError::Malformed(_))),
            "a report without {} was not refused",
            required(index)
        );
        Ok(())
    });
}

/// Renaming a required field is refused.
#[test]
fn renaming_a_required_field_is_refused() {
    check(0..REQUIRED.len(), |index| {
        let document = rename_field(index);
        prop_assert!(
            matches!(decode_report(&document), Err(DecodeError::Malformed(_))),
            "a report with {} renamed was not refused",
            required(index)
        );
        Ok(())
    });
}

/// Retyping a required field is refused.
#[test]
fn retyping_a_required_field_is_refused() {
    check(0..REQUIRED.len(), |index| {
        let document = retype_field(index);
        prop_assert!(
            matches!(decode_report(&document), Err(DecodeError::Malformed(_))),
            "a report with {} retyped was not refused",
            required(index)
        );
        Ok(())
    });
}

/// Arbitrary bytes are decided, never fatal.
#[test]
fn arbitrary_bytes_are_decided() {
    check(strategy_vec(any::<u8>(), 0..256), |bytes| {
        drop(decode_report(&bytes));
        Ok(())
    });
}

/// The generator reaches every required field.
///
/// A property over `0..REQUIRED.len()` is only as good as the range it draws
/// from, so the range is pinned to the field list rather than to a literal.
#[test]
fn the_required_field_list_is_the_contract() {
    check(Just(REQUIRED.len()), |count| {
        prop_assert_eq!(count, 5, "the report contract gained or lost a field");
        Ok(())
    });
}
