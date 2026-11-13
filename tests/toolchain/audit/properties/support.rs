//! Generators, reference encodings, key decoders, and document helpers for A1
//! and A2.
//!
//! The properties in [`super::keys`] and [`super::contract`] share these so the
//! reference encodings, the key decoders, and the mutated documents are defined
//! once. Every helper here is deterministic: the seeded runner fixes the draw
//! order, and the near edits are total functions over the generated values.

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

/// The tag that starts a name key, repeated for the test-only decoder.
///
/// The wire module's constants are private, so they are written out again here
/// on purpose. A decoder that read the encoder's own constants could not catch
/// an encoding change that made a key ambiguous; repeating the format means a
/// drift on either side breaks the round trip.
const NAME_TAG: u8 = 0x4e;

/// The tag that starts a textual component.
const TEXT_TAG: u8 = 0x53;

/// The tag that starts a numeric component.
const NUMBER_TAG: u8 = 0x49;

/// The tag that starts a constant node.
const CONST_TAG: u8 = 0x43;

/// The tag that starts an application node.
const APP_TAG: u8 = 0x41;

/// A cursor over one key's bytes.
struct KeyReader<'a> {
    /// The key being read.
    bytes: &'a [u8],
    /// How many bytes have been consumed.
    at: usize,
}

impl<'a> KeyReader<'a> {
    /// Start reading at the key's first byte.
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, at: 0 } }

    /// Consume one byte, or return `None` at the end of the key.
    fn byte(&mut self) -> Option<u8> {
        let byte = *self.bytes.get(self.at)?;
        self.at += 1;
        Some(byte)
    }

    /// Consume `len` bytes, or return `None` if the key is too short.
    fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(len)?;
        let slice = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(slice)
    }

    /// Consume one little-endian `u64`.
    #[expect(
        clippy::little_endian_bytes,
        reason = "the decoder must read the wire format's little-endian integers"
    )]
    fn u64(&mut self) -> Option<u64> { Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?)) }

    /// Consume one universe level, which the wire format widens to a `u64`.
    fn level(&mut self) -> Option<u32> { u32::try_from(self.u64()?).ok() }

    /// How many bytes are left.
    const fn remaining(&self) -> usize { self.bytes.len() - self.at }

    /// Whether every byte has been consumed.
    const fn at_end(&self) -> bool { self.at == self.bytes.len() }
}

/// Decode the name at the cursor, or return `None` if the bytes are not one.
fn decode_name(reader: &mut KeyReader<'_>) -> Option<Name> {
    if reader.byte()? != NAME_TAG {
        return None;
    }
    let count = usize::try_from(reader.u64()?).ok()?;
    // Every component spends at least its tag byte, so a count above the
    // remaining length is malformed rather than merely large.
    if count > reader.remaining() {
        return None;
    }
    let mut name = Vec::with_capacity(count.min(64));
    for _ in 0..count {
        let component = match reader.byte()? {
            TEXT_TAG => {
                let len = usize::try_from(reader.u64()?).ok()?;
                Component::Text(String::from_utf8(reader.take(len)?.to_vec()).ok()?)
            }
            NUMBER_TAG => Component::Number(reader.u64()?),
            _ => return None,
        };
        name.push(component);
    }
    Some(name)
}

/// Bytes spent by one universe level on the wire.
const LEVEL_BYTES: usize = 8;

/// Decode the constant node whose tag the cursor has already consumed.
fn decode_const(reader: &mut KeyReader<'_>) -> Option<Tree> {
    let name = decode_name(reader)?;
    let count = usize::try_from(reader.u64()?).ok()?;
    // Each level spends eight bytes, so a count that cannot fit in the bytes
    // left is malformed rather than merely large.
    let needed = count.checked_mul(LEVEL_BYTES)?;
    if needed > reader.remaining() {
        return None;
    }
    let mut levels = Vec::with_capacity(count.min(64));
    for _ in 0..count {
        levels.push(reader.level()?);
    }
    Some(Tree::Const { name, levels })
}

/// Decode the tree at the cursor, or return `None` if the bytes are not one.
fn decode_tree(reader: &mut KeyReader<'_>) -> Option<Tree> {
    match reader.byte()? {
        CONST_TAG => decode_const(reader),
        APP_TAG => Some(Tree::App(
            Box::new(decode_tree(reader)?),
            Box::new(decode_tree(reader)?),
        )),
        _ => None,
    }
}

/// Decode a name key, or return `None` if the bytes are not exactly one key.
pub(super) fn decode_name_key(bytes: &[u8]) -> Option<Name> {
    let mut reader = KeyReader::new(bytes);
    let name = decode_name(&mut reader)?;
    reader.at_end().then_some(name)
}

/// Decode a tree key, or return `None` if the bytes are not exactly one key.
///
/// The cursor must be spent afterwards, so trailing bytes are a decode failure
/// rather than an ignored suffix. Without that check an encoding that appended
/// a discriminant could still pass a prefix-only round trip.
pub(super) fn decode_tree_key(bytes: &[u8]) -> Option<Tree> {
    let mut reader = KeyReader::new(bytes);
    let tree = decode_tree(&mut reader)?;
    reader.at_end().then_some(tree)
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
