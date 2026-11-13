//! The values settlement compares, and the pinned foundation keys.
//!
//! Every type here is plain data: no I/O, no `serde`, no Lean. Keys are opaque
//! byte sequences built by [`crate::audit::wire`], and the four pinned keys are
//! literal bytes that a wire test reproduces from real names.
//!
//! The module is separate from the rules in [`super`] so that each file stays
//! inside the repository's module-length limit. Splitting does not change the
//! verified text: `verus/audit_settlement.rs` pulls both files in through
//! `#[path]`, exactly as Cargo compiles them.

use vstd::prelude::*;

verus! {

/// A name or statement encoded as an injective byte key.
pub type Key = Vec<u8>;

/// The key of the `propext` foundation.
///
/// The bytes are `name_key(&[Component::Text("propext")])`, written out so the
/// proof can name a concrete sequence. A wire test checks the encoding
/// reproduces them.
pub const K_PROPEXT: [u8; 25] = [
    0x4e, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x53, 0x07, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x70, 0x72, 0x6f, 0x70, 0x65, 0x78,
    0x74,
];

/// The key of the `Quot.sound` foundation.
pub const K_QUOT: [u8; 36] = [
    0x4e, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x53, 0x04, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x51, 0x75, 0x6f, 0x74, 0x53, 0x05,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x73, 0x6f, 0x75, 0x6e, 0x64,
];

/// The key of the `Classical.choice` foundation.
pub const K_CHOICE: [u8; 42] = [
    0x4e, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x53, 0x09, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x43, 0x6c, 0x61, 0x73, 0x73, 0x69,
    0x63, 0x61, 0x6c, 0x53, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x63, 0x68, 0x6f, 0x69, 0x63, 0x65,
];

/// The key of `sorryAx`, the axiom a `sorry` leaves behind.
pub const K_SORRY: [u8; 25] = [
    0x4e, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x53, 0x07, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x73, 0x6f, 0x72, 0x72, 0x79, 0x41,
    0x78,
];

/// One axiom reachable from a declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Axiom {
    /// The axiom's name, as a key.
    pub name: Key,
    /// Whether a component begins with `_native`, which selects a diagnostic.
    pub native: bool,
}

/// The kind of a declaration the worker observed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclKind {
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

/// One observed declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decl {
    /// The declaration's kind.
    pub kind: DeclKind,
    /// The declaration's type, or `None` when out of grammar or over bound.
    pub statement: Option<Key>,
    /// Every axiom reachable from the declaration.
    pub axioms: Vec<Axiom>,
}

/// One observed root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootObs {
    /// The root's name, as a key.
    pub name: Key,
    /// The declaration, or `None` when the environment has no such constant.
    pub decl: Option<Decl>,
}

/// What the kernel replay did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Replay {
    /// The kernel accepted every replayed declaration in these modules.
    Accepted(Vec<Key>),
    /// The kernel rejected the replay.
    Rejected,
}

/// One observation of a Lean job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditReport {
    /// The Lean build's commit, as raw bytes.
    pub githash: Key,
    /// The module the worker was asked to audit.
    pub module: Key,
    /// What the kernel replay did.
    pub replay: Replay,
    /// One observation per requested root, in request order.
    pub roots: Vec<RootObs>,
}

/// One requested root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestRoot {
    /// The root's name, as a key.
    pub name: Key,
    /// The proposition the root must prove, as a key.
    pub expected: Key,
}

/// The question a report must answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditRequest {
    /// The module to compile and replay.
    pub module: Key,
    /// The transitive axioms the request declares approved.
    pub foundations: Vec<Key>,
    /// The roots to audit, in the order the report must follow.
    pub roots: Vec<RequestRoot>,
}

/// Why a request is not admissible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestRefusal {
    /// The request names no roots, so there is nothing to audit.
    NoRoots,
    /// Two roots share a name, so the report could not answer both.
    DuplicateRoot(usize),
    /// A declared foundation is not one of the three pinned foundations.
    UnapprovedFoundation(usize),
}

/// Why a report does not answer its request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolBreach {
    /// The report names a different Lean build.
    ToolchainMismatch,
    /// The report names a different module.
    ModuleMismatch,
    /// The report's roots do not match the request's roots, or the replayed
    /// modules omit the job module.
    CoverageMismatch,
    /// A rejected replay still reports roots.
    RejectedReplayWithRoots,
}

/// One reason a root is not clean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindingCode {
    /// The kernel rejected the replay.
    KernelRejected,
    /// The replayed environment has no such constant.
    RootAbsent,
    /// The declaration is not a theorem.
    WrongKind,
    /// The declaration's type is not the expected proposition.
    StatementMismatch,
    /// The root depends on `sorryAx`.
    SorryAxiom,
    /// The root depends on a native-computation axiom.
    NativeAxiom,
    /// The root depends on an axiom the request did not approve.
    UndeclaredAxiom,
}

/// One finding about a report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The root the finding is about, or `None` for a whole-report finding.
    pub root: Option<usize>,
    /// The axiom's position in the root's axiom list, when the finding is
    /// about one axiom.
    pub item: Option<usize>,
    /// What is wrong.
    pub code: FindingCode,
}

/// What settlement decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Settlement {
    /// The report proves the request.
    Accepted,
    /// The report answers the request and the answer is no.
    Rejected(Vec<Finding>),
    /// The report does not answer the request.
    ToolFailure(ProtocolBreach),
}

}
