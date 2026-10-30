//! Find the mutation sites in one AST and record the changes.
//!
//! The shapes here are structural. A tagged object is recognised from having
//! exactly one member whose value is an object; a span from having exactly four
//! number members; a tuple from being an array whose first element is span-like
//! but whose elements are not all span-like. None of these tests reads the
//! decoder's tables, so this file cannot drift with them.

use serde_json::{Map, Value as Json};

use super::{
    Change,
    Class,
    Collector,
    Expect,
    MUTANT_KEY,
    MUTANT_OP,
    MUTANT_TAG,
    Place,
    Step,
    is_optional,
    is_span,
    number,
    off_boundary,
    place_of,
    span_like,
    substitute,
    tag_of,
};
use crate::corpus::ReasonCode;

/// Add the mutations that only make sense at the envelope.
pub(super) fn envelope(root: &Json, source: &str, out: &mut Collector) -> Result<(), String> {
    let Some(entries) = root.as_array() else {
        return Err("the AST is not an array of modules".to_owned());
    };
    let Some(entry) = entries.first() else {
        return Err("the AST holds no module".to_owned());
    };
    out.push(
        &[],
        Change::Push(entry.clone()),
        Expect::new(
            Class::ModuleCount,
            ReasonCode::AstModuleCount,
            "duplicate the module",
        ),
    );
    let echoed = root
        .pointer("/0/source/contents")
        .and_then(Json::as_str)
        .ok_or_else(|| "the AST echoes no source text".to_owned())?;
    if echoed != source {
        return Err("the AST does not echo the source it was produced from".to_owned());
    }
    let contents = [
        Step::Index(0),
        Step::Key("source".to_owned()),
        Step::Key("contents".to_owned()),
    ];
    out.push(
        &contents,
        Change::Replace(Json::from(format!("{source} "))),
        Expect::new(
            Class::SourceMismatch,
            ReasonCode::AstSourceMismatch,
            "alter the echoed source",
        ),
    );
    Ok(())
}

/// Visit one node and everything below it.
pub(super) fn visit(value: &Json, path: &mut Vec<Step>, source: &str, out: &mut Collector) {
    match value {
        Json::Object(map) => {
            object_mutants(map, path, source, out);
            for (key, child) in map {
                path.push(Step::Key(key.clone()));
                visit(child, path, source, out);
                path.pop();
            }
        }
        Json::Array(items) => {
            array_mutants(items, path, out);
            for (index, child) in items.iter().enumerate() {
                path.push(Step::Index(index));
                visit(child, path, source, out);
                path.pop();
            }
        }
        Json::Null | Json::Bool(_) | Json::Number(_) | Json::String(_) => {}
    }
}

/// Add the mutations of one object.
fn object_mutants(map: &Map<String, Json>, path: &[Step], source: &str, out: &mut Collector) {
    let tag = tag_of(map);
    let place = place_of(path);
    if let Some(name) = tag {
        out.push(
            path,
            Change::Rename(name.to_owned(), MUTANT_TAG),
            Expect::new(
                Class::UnknownTag,
                ReasonCode::AstUnknownTag,
                "rename the variant tag",
            ),
        );
        out.push(
            path,
            Change::Remove(name.to_owned()),
            Expect::new(
                Class::UnknownTag,
                ReasonCode::AstUnknownTag,
                "delete the variant tag",
            ),
        );
    }
    out.push(
        path,
        Change::Insert(MUTANT_KEY, Json::Null),
        Expect::new(
            Class::UnknownKey,
            ReasonCode::AstUnknownKey,
            "add an unknown key",
        ),
    );
    if map.get("op").is_some_and(Json::is_string) {
        let mut member = path.to_vec();
        member.push(Step::Key("op".to_owned()));
        out.push(
            &member,
            Change::Replace(Json::from(MUTANT_OP)),
            Expect::new(
                Class::UnknownOperator,
                ReasonCode::AstUnknownTag,
                "replace the operator spelling",
            ),
        );
    }
    if tag.is_none() {
        missing_keys(map, place, path, out);
    }
    wrong_types(map, path, out);
    present_mutants(map, place, path, out);
    span_mutants(map, path, source, out);
}

/// Add the missing-key mutations of one object that is not a variant.
fn missing_keys(map: &Map<String, Json>, place: Place, path: &[Step], out: &mut Collector) {
    for key in map.keys() {
        if is_optional(place, key) {
            continue;
        }
        out.push(
            path,
            Change::Remove(key.clone()),
            Expect::new(
                Class::MissingKey,
                ReasonCode::AstMissingKey,
                &format!("delete `{key}`"),
            ),
        );
    }
}

/// Add the wrong-JSON-kind mutations of one object's members.
fn wrong_types(map: &Map<String, Json>, path: &[Step], out: &mut Collector) {
    for (key, child) in map {
        let mut member = path.to_vec();
        member.push(Step::Key(key.clone()));
        out.push(
            &member,
            Change::Replace(substitute(child)),
            Expect::new(
                Class::WrongType,
                ReasonCode::AstType,
                "replace the value's JSON kind",
            ),
        );
    }
}

/// Add the mutations of one array.
fn array_mutants(items: &[Json], path: &[Step], out: &mut Collector) {
    let Some(first) = items.first() else {
        return;
    };
    if !span_like(first) || items.iter().all(span_like) {
        return;
    }
    out.push(
        path,
        Change::Push(Json::Null),
        Expect::new(Class::WrongArity, ReasonCode::AstArity, "extend a tuple"),
    );
}

/// Add the mutations of an optional member that is present but empty.
fn present_mutants(map: &Map<String, Json>, place: Place, path: &[Step], out: &mut Collector) {
    match place {
        Place::Module if !map.contains_key("target") => {
            out.push(
                path,
                Change::Insert("target", Json::Null),
                Expect::present(ReasonCode::AstType, "add a null target"),
            );
            out.push(
                path,
                Change::Insert("target", Json::from("")),
                Expect::present(ReasonCode::AstType, "add a blank target"),
            );
        }
        Place::Import if !map.contains_key("as") => {
            out.push(
                path,
                Change::Insert("as", Json::Null),
                Expect::present(ReasonCode::AstType, "add a null alias"),
            );
            out.push(
                path,
                Change::Insert("as", Json::Object(Map::new())),
                Expect::present(ReasonCode::AstMissingKey, "add an empty alias"),
            );
        }
        Place::Statement if !map.contains_key("with_mods") => {
            out.push(
                path,
                Change::Insert("with_mods", Json::Null),
                Expect::present(ReasonCode::AstType, "add a null with-list"),
            );
            out.push(
                path,
                Change::Insert("with_mods", Json::Array(Vec::new())),
                Expect::present(ReasonCode::AstType, "add an empty with-list"),
            );
        }
        Place::Module | Place::Import | Place::Statement | Place::Other => {}
    }
}

/// Add the span mutations of one span-shaped object.
fn span_mutants(map: &Map<String, Json>, path: &[Step], source: &str, out: &mut Collector) {
    if !is_span(map) {
        return;
    }
    let Some(end) = number(map, "end") else {
        return;
    };
    if let Some(after) = end.checked_add(1) {
        span_push(out, path, &with(map, [("start", after)]), "start after end");
    }
    let beyond_source = u32::try_from(source.len())
        .ok()
        .and_then(|length| length.checked_add(1));
    if let Some(beyond) = beyond_source {
        let span = with(map, [("end", beyond)]);
        span_push(out, path, &span, "end beyond the source");
    }
    if let Some(offset) = off_boundary(source) {
        let inside = with(map, [("start", offset), ("end", offset)]);
        span_push(out, path, &inside, "offset inside a character");
    }
}

/// A copy of a span with the named offsets replaced.
fn with<const N: usize>(map: &Map<String, Json>, changes: [(&str, u32); N]) -> Map<String, Json> {
    let mut span = map.clone();
    for (key, value) in changes {
        span.insert(key.to_owned(), Json::from(value));
    }
    span
}

/// Add one span mutant.
fn span_push(out: &mut Collector, path: &[Step], span: &Map<String, Json>, what: &str) {
    out.push(
        path,
        Change::Replace(Json::Object(span.clone())),
        Expect::new(Class::Span, ReasonCode::AstSpan, what),
    );
}
