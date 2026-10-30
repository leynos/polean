//! Build the O2 mutants from one fixture's AST JSON.
//!
//! The generator walks the JSON value the pinned frontend produced. It decides
//! where a mutation applies from the value's own shape and position: an object
//! with one object-valued key is an externally tagged variant, an array whose
//! first element is a span-shaped object is a tuple, and an object of four
//! numbers is a span. It never reads the decoder's field or variant tables, so
//! a decoder that stops requiring a key is caught by a sweep that still tries
//! to delete it.
//!
//! [`shapes`] finds the mutation sites; this module names the classes, records
//! the changes, and applies one change at a time to a copy of the tree.

mod shapes;

use serde_json::{Map, Value as Json};
use shapes::{envelope, visit};

use crate::corpus::ReasonCode;

/// The tag a mutated variant is renamed to.
const MUTANT_TAG: &str = "Mutant";

/// The key every object gains in the unknown-key class.
const MUTANT_KEY: &str = "mutant";

/// The spelling a mutated unit-enum position is replaced with.
const MUTANT_OP: &str = "Mutant";

/// The string a mutated string or number value is replaced with.
const MUTANT_TEXT: &str = "mutant";

/// One class of structural mutation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Class {
    /// A tag outside the variant table.
    UnknownTag,
    /// A unit-enum spelling outside the operator table.
    UnknownOperator,
    /// A key the object's shape does not have.
    UnknownKey,
    /// A key the frontend always emits, deleted.
    MissingKey,
    /// A value of another JSON kind.
    WrongType,
    /// A tuple array of the wrong length.
    WrongArity,
    /// A span that is not a range inside the source.
    Span,
    /// An optional key present with an empty or null value.
    OptionalPresent,
    /// A number of modules other than one.
    ModuleCount,
    /// An echoed source that is not the submitted text.
    SourceMismatch,
}

impl Class {
    /// Every class, in declaration order.
    pub const ALL: [Self; 10] = [
        Self::UnknownTag,
        Self::UnknownOperator,
        Self::UnknownKey,
        Self::MissingKey,
        Self::WrongType,
        Self::WrongArity,
        Self::Span,
        Self::OptionalPresent,
        Self::ModuleCount,
        Self::SourceMismatch,
    ];

    /// The class's name in the counts snapshot and in failure messages.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnknownTag => "unknown-tag",
            Self::UnknownOperator => "unknown-operator",
            Self::UnknownKey => "unknown-key",
            Self::MissingKey => "missing-key",
            Self::WrongType => "wrong-type",
            Self::WrongArity => "wrong-arity",
            Self::Span => "span",
            Self::OptionalPresent => "optional-present",
            Self::ModuleCount => "module-count",
            Self::SourceMismatch => "source-mismatch",
        }
    }
}

/// One mutated AST JSON and the rejection it must provoke.
#[derive(Clone, Debug)]
pub struct Mutant {
    /// The class the mutation belongs to.
    pub class: Class,
    /// The path and change, for a failure message.
    pub label: String,
    /// The reason code the decoder must report.
    pub expected: ReasonCode,
    /// The mutated AST JSON.
    pub ast_json: String,
}

/// What a node is, for the checks that depend on position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Place {
    /// The module object.
    Module,
    /// An import declaration.
    Import,
    /// A statement.
    Statement,
    /// Any other node.
    Other,
}

/// One step of a JSON path.
#[derive(Clone, Debug)]
enum Step {
    /// A member of an object.
    Key(String),
    /// An element of an array.
    Index(usize),
}

/// One single-position change.
enum Change {
    /// Insert a member into an object.
    Insert(&'static str, Json),
    /// Remove a member from an object.
    Remove(String),
    /// Rename a member of an object, keeping its value.
    Rename(String, &'static str),
    /// Replace the node itself.
    Replace(Json),
    /// Append an element to an array.
    Push(Json),
}

/// A mutation before it is applied to the tree.
struct Spec {
    /// The class the mutation belongs to.
    class: Class,
    /// The path and change, for a failure message.
    label: String,
    /// The path of the node the change applies to.
    path: Vec<Step>,
    /// The change itself.
    change: Change,
    /// The reason code the decoder must report.
    expected: ReasonCode,
}

/// What a mutation is, and how a failure should describe it.
#[derive(Clone, Copy)]
struct Expect<'a> {
    /// The class the mutation belongs to.
    class: Class,
    /// The reason code the decoder must report.
    code: ReasonCode,
    /// What the mutation does, for a failure message.
    what: &'a str,
}

impl<'a> Expect<'a> {
    /// A mutation of class `class` that must be rejected with `code`.
    const fn new(class: Class, code: ReasonCode, what: &'a str) -> Self {
        Self { class, code, what }
    }

    /// A mutation that adds an optional member with an empty value.
    const fn present(code: ReasonCode, what: &'a str) -> Self {
        Self::new(Class::OptionalPresent, code, what)
    }
}

/// A growing list of mutation specifications.
struct Collector {
    /// The specifications recorded so far.
    specs: Vec<Spec>,
}

impl Collector {
    /// An empty collector.
    const fn new() -> Self { Self { specs: Vec::new() } }

    /// Record one mutation at `path`.
    fn push(&mut self, path: &[Step], change: Change, expect: Expect<'_>) {
        self.specs.push(Spec {
            class: expect.class,
            label: format!(
                "{} {}: {}",
                expect.class.as_str(),
                path_label(path),
                expect.what
            ),
            path: path.to_vec(),
            change,
            expected: expect.code,
        });
    }

    /// The recorded specifications, in the order they were found.
    fn into_specs(self) -> Vec<Spec> { self.specs }
}

/// Build every mutant of one fixture's AST JSON.
///
/// # Errors
///
/// Returns a description when the AST is not the JSON of a single module that
/// echoes `source`, because a mutation of anything else would not be the
/// mutation the sweep claims.
pub fn mutants(source: &str, ast_json: &str) -> Result<Vec<Mutant>, String> {
    let root: Json = serde_json::from_str(ast_json)
        .map_err(|error| format!("the fixture's AST is not JSON: {error}"))?;
    let mut collector = Collector::new();
    envelope(&root, source, &mut collector)?;
    visit(&root, &mut Vec::new(), source, &mut collector);
    collector
        .into_specs()
        .into_iter()
        .map(|spec| {
            let mutated = apply(&root, &spec.path, &spec.change)
                .ok_or_else(|| format!("the mutation {} does not apply to the AST", spec.label))?;
            let text = serde_json::to_string(&mutated)
                .map_err(|error| format!("a mutated AST is not JSON: {error}"))?;
            Ok(Mutant {
                class: spec.class,
                label: spec.label,
                expected: spec.expected,
                ast_json: text,
            })
        })
        .collect()
}

/// The tag of an externally tagged object, if this is one.
fn tag_of(map: &Map<String, Json>) -> Option<&str> {
    if map.len() != 1 {
        return None;
    }
    map.iter()
        .next()
        .filter(|(_, value)| value.is_object())
        .map(|(key, _)| key.as_str())
}

/// What a node at `path` is.
fn place_of(path: &[Step]) -> Place {
    match path.last() {
        Some(Step::Key(key)) if key == "ast" => Place::Module,
        Some(Step::Index(_)) => match path.iter().rev().nth(1) {
            Some(Step::Key(key)) if key == "imports" => Place::Import,
            Some(Step::Key(key)) if key == "stmts" => Place::Statement,
            _ => Place::Other,
        },
        _ => Place::Other,
    }
}

/// Whether `key` is one the frontend may omit at `place`.
fn is_optional(place: Place, key: &str) -> bool {
    match place {
        Place::Module => key == "target",
        Place::Import => key == "as",
        Place::Statement => key == "with_mods",
        Place::Other => false,
    }
}

/// A value of another JSON kind.
fn substitute(value: &Json) -> Json {
    match value {
        Json::Null => Json::Bool(true),
        Json::Bool(_) | Json::Number(_) => Json::from(MUTANT_TEXT),
        Json::String(_) => Json::from(0),
        Json::Array(_) => Json::Object(Map::new()),
        Json::Object(_) => Json::Array(Vec::new()),
    }
}

/// Whether an object has the four numbers of a span.
fn is_span(map: &Map<String, Json>) -> bool { map.len() == 4 && map.values().all(Json::is_number) }

/// Whether a value is a span-shaped object.
fn span_like(value: &Json) -> bool { value.as_object().is_some_and(is_span) }

/// A member's non-negative 32-bit number.
fn number(map: &Map<String, Json>, key: &str) -> Option<u32> {
    u32::try_from(map.get(key)?.as_u64()?).ok()
}

/// The first offset inside a multi-byte character.
fn off_boundary(source: &str) -> Option<u32> {
    source
        .char_indices()
        .find(|(_, character)| character.len_utf8() > 1)
        .and_then(|(offset, _)| u32::try_from(offset + 1).ok())
}

/// Render a path for a failure message.
fn path_label(path: &[Step]) -> String {
    let mut rendered = String::from("$");
    for step in path {
        match step {
            Step::Key(key) => {
                rendered.push('.');
                rendered.push_str(key);
            }
            Step::Index(index) => {
                rendered.push('[');
                rendered.push_str(&index.to_string());
                rendered.push(']');
            }
        }
    }
    rendered
}

/// Apply one change to a copy of the tree.
fn apply(root: &Json, path: &[Step], change: &Change) -> Option<Json> {
    let mut tree = root.clone();
    match change {
        Change::Replace(replacement) => replace(&mut tree, path, replacement)?,
        Change::Insert(name, value) => insert(&mut tree, path, name, value)?,
        Change::Remove(name) => remove(&mut tree, path, name)?,
        Change::Rename(from, to) => rename(&mut tree, path, from, to)?,
        Change::Push(value) => push_element(&mut tree, path, value)?,
    }
    Some(tree)
}

/// Replace the node a change names, which must be a member of a tuple or an
/// object.
fn replace(tree: &mut Json, path: &[Step], replacement: &Json) -> Option<()> {
    let (last, parent) = path.split_last()?;
    let node = at(tree, parent)?;
    match last {
        Step::Key(key) => {
            *node.as_object_mut()?.get_mut(key)? = replacement.clone();
        }
        Step::Index(index) => {
            *node.as_array_mut()?.get_mut(*index)? = replacement.clone();
        }
    }
    Some(())
}

/// Add a member to the object a change names.
fn insert(tree: &mut Json, path: &[Step], name: &str, value: &Json) -> Option<()> {
    at(tree, path)?
        .as_object_mut()?
        .insert(name.to_owned(), value.clone());
    Some(())
}

/// Remove a member from the object a change names.
fn remove(tree: &mut Json, path: &[Step], name: &str) -> Option<()> {
    at(tree, path)?.as_object_mut()?.remove(name)?;
    Some(())
}

/// Rename a member of the object a change names, keeping its value.
fn rename(tree: &mut Json, path: &[Step], from: &str, to: &'static str) -> Option<()> {
    let object = at(tree, path)?.as_object_mut()?;
    let value = object.remove(from)?;
    object.insert(to.to_owned(), value);
    Some(())
}

/// Append an element to the array a change names.
fn push_element(tree: &mut Json, path: &[Step], value: &Json) -> Option<()> {
    at(tree, path)?.as_array_mut()?.push(value.clone());
    Some(())
}

/// The node at `path`.
fn at<'a>(mut node: &'a mut Json, path: &[Step]) -> Option<&'a mut Json> {
    for step in path {
        node = match step {
            Step::Key(key) => node.get_mut(key)?,
            Step::Index(index) => node.get_mut(*index)?,
        };
    }
    Some(node)
}
