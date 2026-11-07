//! Strict decoding of the audit request and report contracts.
//!
//! The worker is unverified, so the adapter treats its output as untrusted
//! data. Derived `serde` decoding with `deny_unknown_fields` rejects unknown
//! and duplicate fields, and a missing required field is an error rather than
//! a default. A *nullable* field is the one case a derive would otherwise
//! accept silently, because `Option` defaults to `None`; `declaration` and
//! `statement` therefore deserialize with `Option::deserialize` and no
//! `default`, which keeps a missing key malformed while admitting an explicit
//! `null`.
//!
//! Every refusal is a [`DecodeError`]. The smoke runner reports `Malformed` as
//! `malformed_report` and `OutputOverflow` as `output_overflow`, both before
//! settlement runs. Decoding success is not acceptance: it says the bytes have
//! the contract's shape, not that the report is true.
//!
//! The document types live in [`super::document`]; this module holds the byte
//! bounds, the entry points, and the conversion into settlement values.

use std::collections::BTreeSet;

use super::{
    document::{
        AcceptedReplayDocument,
        ComponentDocument,
        DeclarationDocument,
        KindDocument,
        RejectedReplayDocument,
        ReplayDocument,
        ReportDocument,
        RequestDocument,
        RootDocument,
        TreeDocument,
    },
    name::{Component, Name, is_native, name_key},
    tree::{MAX_DEPTH, MAX_NODES, Tree, depth, nodes, tree_key},
};
use crate::audit::settlement::decision::{
    AuditReport,
    AuditRequest,
    Axiom,
    Decl,
    DeclKind,
    Key,
    Replay,
    RequestRoot,
    RootObs,
};

/// The largest report the decoder will parse, in bytes.
pub const MAX_REPORT_BYTES: usize = 1 << 20;

/// The largest request the decoder will parse, in bytes.
pub const MAX_REQUEST_BYTES: usize = 1 << 20;

/// The largest replay diagnostic the decoder will accept, in UTF-8 bytes.
pub const MAX_MESSAGE_BYTES: usize = 4_096;

/// The literal the accepted-replay status must equal.
const ACCEPTED_STATUS: &str = "accepted";

/// The literal the rejected-replay status must equal.
const REJECTED_STATUS: &str = "rejected";

/// Why a document could not be decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The output exceeded the byte bound before it was parsed.
    OutputOverflow,
    /// The document did not satisfy its contract.
    Malformed(String),
}

/// Decode a report, or refuse it before settlement can see it.
///
/// # Errors
///
/// Returns [`DecodeError::OutputOverflow`] when `bytes` exceeds
/// [`MAX_REPORT_BYTES`], and [`DecodeError::Malformed`] when the document is
/// not valid JSON, has an unknown or duplicate field, omits a required field,
/// carries a value outside the field's range, or holds a statement over the
/// node or depth bound. An empty `githash` is malformed: the contract requires
/// at least one character, and only a non-empty hash is meaningfully compared
/// with the pinned identity.
pub fn decode_report(bytes: &[u8]) -> Result<AuditReport, DecodeError> {
    if bytes.len() > MAX_REPORT_BYTES {
        return Err(DecodeError::OutputOverflow);
    }
    let document: ReportDocument = serde_json::from_slice(bytes)
        .map_err(|error| DecodeError::Malformed(format!("report is not valid JSON: {error}")))?;
    if document.githash.is_empty() {
        return Err(DecodeError::Malformed(
            "githash must not be empty".to_owned(),
        ));
    }
    Ok(AuditReport {
        githash: document.githash.into_bytes(),
        module: name_key(&convert_name(document.module)?),
        replay: convert_replay(document.replay)?,
        roots: document
            .roots
            .into_iter()
            .map(convert_root)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

/// Decode a request, or refuse it before settlement can see it.
///
/// Request names must be passable as dotted command-line arguments, so every
/// component is non-empty text matching `[A-Za-z_][A-Za-z0-9_']*`, and a name
/// has at least one component. The same grammar applies to the constant names
/// inside a request's `expected` tree, because those names are echoed into the
/// worker's arguments. The report contract is looser because private names
/// contain empty and numeric components.
///
/// # Errors
///
/// Returns [`DecodeError::OutputOverflow`] when `bytes` exceeds
/// [`MAX_REQUEST_BYTES`], and [`DecodeError::Malformed`] for the same classes
/// of fault as [`decode_report`], plus a request name component that is
/// numeric, empty, or outside the argument grammar, and a foundation that is
/// declared more than once.
pub fn decode_request(bytes: &[u8]) -> Result<AuditRequest, DecodeError> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(DecodeError::OutputOverflow);
    }
    let document: RequestDocument = serde_json::from_slice(bytes)
        .map_err(|error| DecodeError::Malformed(format!("request is not valid JSON: {error}")))?;
    let foundations = document
        .foundations
        .into_iter()
        .map(|name| convert_request_name(name).map(|key| name_key(&key)))
        .collect::<Result<Vec<_>, _>>()?;
    unique_foundations(&foundations)?;
    Ok(AuditRequest {
        module: name_key(&convert_request_name(document.module)?),
        foundations,
        roots: document
            .roots
            .into_iter()
            .map(|root| {
                Ok(RequestRoot {
                    name: name_key(&convert_request_name(root.name)?),
                    expected: tree_key(&checked_request_tree(root.expected)?),
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
    })
}

/// Refuse a request that declares the same foundation twice.
///
/// The request schema states `uniqueItems`, and settlement reads the list as
/// the approved set. Enforcing it here keeps a document the schema rejects from
/// reaching settlement, so the decoder and the contract agree about which
/// requests exist.
///
/// # Errors
///
/// Returns [`DecodeError::Malformed`] when two entries encode to the same key.
fn unique_foundations(foundations: &[Key]) -> Result<(), DecodeError> {
    let mut seen = BTreeSet::new();
    for foundation in foundations {
        if !seen.insert(foundation.as_slice()) {
            return Err(DecodeError::Malformed(
                "a foundation is declared more than once".to_owned(),
            ));
        }
    }
    Ok(())
}

/// Convert one contract component.
fn convert_component(component: ComponentDocument) -> Component {
    match component {
        ComponentDocument::Text(text) => Component::Text(text),
        ComponentDocument::Number(number) => Component::Number(number),
    }
}

/// Convert a report name, which may hold empty and numeric components.
fn convert_name(name: Vec<ComponentDocument>) -> Result<Name, DecodeError> {
    if name.is_empty() {
        return Err(DecodeError::Malformed(
            "a name must have at least one component".to_owned(),
        ));
    }
    Ok(name.into_iter().map(convert_component).collect())
}

/// Convert a request name, which must be passable as a dotted argument.
///
/// A request name has at least one component, as the request schema's
/// `minItems: 1` states. An empty name would otherwise encode as the same key
/// as no name at all, so it is refused rather than passed to settlement.
fn convert_request_name(name: Vec<ComponentDocument>) -> Result<Name, DecodeError> {
    if name.is_empty() {
        return Err(DecodeError::Malformed(
            "a request name must have at least one component".to_owned(),
        ));
    }
    name.into_iter()
        .map(|component| match component {
            ComponentDocument::Text(text) if is_request_text(&text) => Ok(Component::Text(text)),
            _ => Err(DecodeError::Malformed(
                "request name component is not a dotted-argument token".to_owned(),
            )),
        })
        .collect()
}

/// Whether `text` is one dotted-argument token.
fn is_request_text(text: &str) -> bool {
    let mut characters = text.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '\''
        })
}

/// Convert a report tree and hold it to the node and depth bounds.
fn checked_tree(tree: TreeDocument) -> Result<Tree, DecodeError> {
    let converted = convert_tree(tree)?;
    check_tree_bounds(&converted)?;
    Ok(converted)
}

/// Convert a request's expected tree and hold it to the node and depth bounds.
///
/// A request's `expected` tree is compared against report trees, but its
/// constant names are also echoed into the worker's arguments, so they follow
/// the request name grammar rather than the report's looser one.
fn checked_request_tree(tree: TreeDocument) -> Result<Tree, DecodeError> {
    let converted = convert_tree_with(tree, convert_request_name)?;
    check_tree_bounds(&converted)?;
    Ok(converted)
}

/// Refuse a tree over the node or depth bound.
fn check_tree_bounds(tree: &Tree) -> Result<(), DecodeError> {
    if nodes(tree) > MAX_NODES {
        return Err(DecodeError::Malformed(format!(
            "statement tree exceeds {MAX_NODES} nodes"
        )));
    }
    if depth(tree) > MAX_DEPTH {
        return Err(DecodeError::Malformed(format!(
            "statement tree exceeds depth {MAX_DEPTH}"
        )));
    }
    Ok(())
}

/// Convert a report tree without re-checking the bound at every level.
fn convert_tree(tree: TreeDocument) -> Result<Tree, DecodeError> {
    convert_tree_with(tree, convert_name)
}

/// Convert a tree, naming its constants with `convert`.
///
/// The bound is checked once, on the whole tree, rather than at every level.
fn convert_tree_with(
    tree: TreeDocument,
    convert: fn(Vec<ComponentDocument>) -> Result<Name, DecodeError>,
) -> Result<Tree, DecodeError> {
    Ok(match tree {
        TreeDocument::Const(node) => Tree::Const {
            name: convert(node.name)?,
            levels: node.levels,
        },
        TreeDocument::App(node) => Tree::App(
            Box::new(convert_tree_with(*node.app.0, convert)?),
            Box::new(convert_tree_with(*node.app.1, convert)?),
        ),
    })
}

/// Convert a declaration.
fn convert_declaration(declaration: DeclarationDocument) -> Result<Decl, DecodeError> {
    Ok(Decl {
        kind: convert_kind(declaration.kind),
        statement: declaration
            .statement
            .map(checked_tree)
            .transpose()?
            .map(|tree| tree_key(&tree)),
        axioms: declaration
            .axioms
            .into_iter()
            .map(|components| {
                let name = convert_name(components)?;
                Ok(Axiom {
                    native: is_native(&name),
                    name: name_key(&name),
                })
            })
            .collect::<Result<Vec<_>, DecodeError>>()?,
    })
}

/// Convert a declaration kind.
const fn convert_kind(kind: KindDocument) -> DeclKind {
    match kind {
        KindDocument::Theorem => DeclKind::Theorem,
        KindDocument::Axiom => DeclKind::Axiom,
        KindDocument::Definition => DeclKind::Definition,
        KindDocument::Opaque => DeclKind::Opaque,
        KindDocument::Quotient => DeclKind::Quotient,
        KindDocument::Constructor => DeclKind::Constructor,
        KindDocument::Recursor => DeclKind::Recursor,
        KindDocument::Inductive => DeclKind::Inductive,
    }
}

/// Convert one observed root.
fn convert_root(root: RootDocument) -> Result<RootObs, DecodeError> {
    Ok(RootObs {
        name: name_key(&convert_name(root.name)?),
        decl: root.declaration.map(convert_declaration).transpose()?,
    })
}

/// Convert what the kernel replay did.
fn convert_replay(replay: ReplayDocument) -> Result<Replay, DecodeError> {
    match replay {
        ReplayDocument::Accepted(accepted) => convert_accepted(accepted).map(Replay::Accepted),
        ReplayDocument::Rejected(rejected) => convert_rejected(&rejected),
    }
}

/// Convert an accepted replay's module list.
fn convert_accepted(accepted: AcceptedReplayDocument) -> Result<Vec<Key>, DecodeError> {
    if accepted.status != ACCEPTED_STATUS {
        return Err(DecodeError::Malformed(format!(
            "unknown replay status {:?}",
            accepted.status
        )));
    }
    accepted
        .modules
        .into_iter()
        .map(|name| convert_name(name).map(|key| name_key(&key)))
        .collect()
}

/// Convert a rejected replay, which carries only a bounded diagnostic.
fn convert_rejected(rejected: &RejectedReplayDocument) -> Result<Replay, DecodeError> {
    if rejected.status != REJECTED_STATUS {
        return Err(DecodeError::Malformed(format!(
            "unknown replay status {:?}",
            rejected.status
        )));
    }
    if rejected.message.len() > MAX_MESSAGE_BYTES {
        return Err(DecodeError::Malformed(format!(
            "replay message exceeds {MAX_MESSAGE_BYTES} bytes"
        )));
    }
    Ok(Replay::Rejected)
}
