//! The document types the contracts decode into.
//!
//! These are the shapes `serde_json` sees, and every one of them is stricter
//! than the JSON Schema: `deny_unknown_fields` rejects an unknown or duplicate
//! key, a field with no `Option` and no default is required, and the
//! presence-checking [`present`] deserializer separates a missing nullable key
//! from an explicit `null`.
//!
//! The types are deliberately inert. They hold no behaviour beyond the derive,
//! so the conversion in [`super::decode`] is the single place a document
//! becomes a settlement value, and a change to the contract cannot quietly
//! change the meaning of a decoded report.

use serde::{Deserialize, Deserializer};

/// Accept a present value, including `null`, and record that it was present.
///
/// A derived `Option` field cannot tell a missing key from an explicit `null`,
/// because both become `None`. The field holds `Option<Option<T>>`: a missing
/// key keeps the outer `None`, while `null` and a value become `Some(None)` and
/// `Some(Some(_))`. The conversion then refuses a missing key.
pub(super) fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// One name component as the contract spells it.
#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum ComponentDocument {
    /// A textual component.
    Text(String),
    /// A numeric component, as a private name uses.
    Number(u64),
}

/// A ground statement tree as the contract spells it.
#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum TreeDocument {
    /// A constant applied to universe levels.
    Const(ConstTreeDocument),
    /// One tree applied to another.
    App(AppTreeDocument),
}

/// The constant form of a ground tree.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConstTreeDocument {
    /// The constant's name.
    #[serde(rename = "const")]
    pub(super) name: Vec<ComponentDocument>,
    /// The universe levels the constant is instantiated at.
    pub(super) levels: Vec<u32>,
}

/// The application form of a ground tree.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AppTreeDocument {
    /// The operator and operand, in that order.
    pub(super) app: (Box<TreeDocument>, Box<TreeDocument>),
}

/// The request schema identifier, which admits exactly one value.
#[derive(Deserialize)]
pub(super) enum RequestSchemaVersion {
    /// The only request version this decoder reads.
    #[serde(rename = "polean.audit-request.v0")]
    V0,
}

/// The report schema identifier, which admits exactly one value.
#[derive(Deserialize)]
pub(super) enum ReportSchemaVersion {
    /// The only report version this decoder reads.
    #[serde(rename = "polean.audit-report.v0")]
    V0,
}

/// The request contract's top level.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RequestDocument {
    /// The schema identifier. Present, typed, and otherwise discarded.
    #[serde(rename = "schema_version")]
    pub(super) _schema_version: RequestSchemaVersion,
    /// The module the worker compiles and replays.
    pub(super) module: Vec<ComponentDocument>,
    /// The roots to audit, in the order settlement expects.
    pub(super) roots: Vec<RequestRootDocument>,
    /// The transitive axioms the request declares approved.
    pub(super) foundations: Vec<Vec<ComponentDocument>>,
}

/// One requested root.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RequestRootDocument {
    /// The root's name.
    pub(super) name: Vec<ComponentDocument>,
    /// The proposition the root must prove.
    pub(super) expected: TreeDocument,
}

/// The report contract's top level.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReportDocument {
    /// The schema identifier. Present, typed, and otherwise discarded.
    #[serde(rename = "schema_version")]
    pub(super) _schema_version: ReportSchemaVersion,
    /// The Lean build's commit.
    pub(super) githash: String,
    /// The module the worker was asked to audit.
    pub(super) module: Vec<ComponentDocument>,
    /// What the kernel replay did.
    pub(super) replay: ReplayDocument,
    /// One observation per requested root, in request order.
    pub(super) roots: Vec<RootDocument>,
}

/// One observed root.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RootDocument {
    /// The root's name.
    pub(super) name: Vec<ComponentDocument>,
    /// The declaration, or `null` when the replayed environment has none.
    ///
    /// The outer `Option` records that the key was present. Collapsing it to
    /// one `Option` would make a missing key indistinguishable from `null`,
    /// and a derived default would then silently accept a truncated report.
    #[expect(
        clippy::option_option,
        reason = "the outer Option records presence, which one Option cannot distinguish from null"
    )]
    #[serde(default, deserialize_with = "present")]
    pub(super) declaration: Option<Option<DeclarationDocument>>,
}

/// One observed declaration.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeclarationDocument {
    /// The declaration's kind.
    pub(super) kind: KindDocument,
    /// The declaration's type, or `null` when out of grammar or over bound.
    ///
    /// The outer `Option` records presence for the same reason as
    /// [`RootDocument::declaration`].
    #[expect(
        clippy::option_option,
        reason = "the outer Option records presence, which one Option cannot distinguish from null"
    )]
    #[serde(default, deserialize_with = "present")]
    pub(super) statement: Option<Option<TreeDocument>>,
    /// Every axiom reachable from the declaration.
    pub(super) axioms: Vec<Vec<ComponentDocument>>,
}

/// The declaration kinds the report contract admits.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum KindDocument {
    /// A theorem.
    Theorem,
    /// An axiom.
    Axiom,
    /// A definition.
    Definition,
    /// An opaque definition.
    Opaque,
    /// A quotient.
    Quotient,
    /// An inductive type's constructor.
    Constructor,
    /// An inductive type's recursor.
    Recursor,
    /// An inductive type.
    Inductive,
}

/// What the kernel replay did, as the contract spells it.
#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum ReplayDocument {
    /// The kernel accepted every replayed declaration.
    Accepted(AcceptedReplayDocument),
    /// The kernel rejected the replay.
    Rejected(RejectedReplayDocument),
}

/// An accepted replay.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AcceptedReplayDocument {
    /// The status literal, checked during conversion.
    pub(super) status: String,
    /// The modules replayed.
    pub(super) modules: Vec<Vec<ComponentDocument>>,
}

/// A rejected replay.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RejectedReplayDocument {
    /// The status literal, checked during conversion.
    pub(super) status: String,
    /// Diagnostic text, bounded, never parsed, and discarded.
    pub(super) message: String,
}
