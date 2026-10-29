//! The corpus manifest: one JSON object per line, parsed without a derive.
//!
//! The manifest is a machine-readable contract, so it is parsed strictly: an
//! unknown key, a missing key, an unknown stage, an unknown reason code, or an
//! unknown `covers` name is an error rather than something ignored. That is the
//! same fail-closed posture the admission spike is testing for, applied to the
//! spike's own inputs.

use serde_json::{Map, Value as Json};

use crate::inventory::NodeKind;

/// Where a corpus entry's source bytes come from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Origin {
    /// A committed file, named by its repository-relative path.
    File(String),
    /// A source built by a function in [`super::generated`].
    Generated(String),
}

/// What the manifest says a fixture must yield.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expected {
    /// The fixture is admitted and its rule list equals the named IR file.
    Admitted {
        /// Repository-relative path of the hand-written IR file.
        ir: String,
    },
    /// The fixture is rejected at the named stage with the named reason code.
    Rejected {
        /// The stage that must reject the fixture.
        stage: Stage,
        /// The reason code that stage must report.
        code: ReasonCode,
    },
}

/// One manifest entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    /// Where the source bytes come from.
    pub origin: Origin,
    /// The AST node kinds this fixture is intended to add to the corpus.
    pub covers: Vec<NodeKind>,
    /// The outcome the fixture must yield.
    pub expected: Expected,
}

impl Entry {
    /// The label used in the observation snapshot and in failure messages.
    #[must_use]
    pub fn label(&self) -> String {
        match &self.origin {
            Origin::File(path) => path.clone(),
            Origin::Generated(name) => format!("examples/admission/generated/{name}"),
        }
    }
}

/// The stage of admission that may reject a source.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Stage {
    /// The frontend refused to parse the source.
    Parse,
    /// The frontend parsed the source but could not serialize its AST.
    Serialize,
    /// The AST JSON did not match the closed decoder.
    Decode,
    /// The module parsed but does not belong to the profile.
    Profile,
    /// The AST walk admitted the module but the canonical-token recognizer did
    /// not.
    Recognizer,
}

impl Stage {
    /// The dotted wire name used in the manifest.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Serialize => "serialize",
            Self::Decode => "decode",
            Self::Profile => "profile",
            Self::Recognizer => "recognizer",
        }
    }

    /// The stage named by `wire`, if there is one.
    #[must_use]
    pub fn from_wire(wire: &str) -> Option<Self> {
        [
            Self::Parse,
            Self::Serialize,
            Self::Decode,
            Self::Profile,
            Self::Recognizer,
        ]
        .into_iter()
        .find(|stage| stage.as_wire() == wire)
    }
}

/// The closed set of reasons a source may be rejected.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ReasonCode {
    /// The package is not the profile's package.
    PackageShape,
    /// The module does not import exactly the profile's import.
    ImportShape,
    /// The default rule is not the profile's default.
    DefaultShape,
    /// A rule head is not the profile's rule head.
    RuleHead,
    /// A rule body is not the profile's single body.
    RuleBody,
    /// A body literal is not an expression statement.
    LiteralKind,
    /// A statement carries a `with` modifier.
    WithModifier,
    /// A statement is not an atom comparison.
    ExprKind,
    /// A comparison uses an operator the profile does not admit.
    OperatorKind,
    /// A well-formed comparison matches no registered atom.
    AtomUnknown,
    /// The module has more rules than the profile permits.
    RuleLimit,
    /// A rule body has a literal count the profile does not permit.
    BodyLimit,
    /// The module sets `__target__`.
    ModuleTarget,
    /// A rule appears before the default, or after a rule the profile cannot
    /// order it with.
    OrderShape,
    /// A comment contains the bytes `METADATA`.
    TriviaMetadata,
    /// Trivia contains a byte the profile does not permit.
    TriviaByte,
    /// The source bytes are not the canonical tokens of the admitted IR.
    TokenMismatch,
    /// The frontend rejected the source's syntax.
    ParseSyntax,
    /// The frontend rejected a number literal.
    ParseNumber,
    /// The AST JSON has an unknown variant tag.
    AstUnknownTag,
    /// The AST JSON has an unknown key.
    AstUnknownKey,
    /// The AST JSON lacks a key the frontend always emits.
    AstMissingKey,
    /// The AST JSON has a value of the wrong type.
    AstType,
    /// The AST JSON has the wrong arity for a tuple array.
    AstArity,
    /// The AST JSON has a span outside the source or off a UTF-8 boundary.
    AstSpan,
    /// The AST JSON holds a number of modules other than one.
    AstModuleCount,
    /// The AST JSON's `source.contents` differs from the submitted text.
    AstSourceMismatch,
    /// The frontend parsed the source but could not serialize its AST.
    AstSerialize,
}

impl ReasonCode {
    /// Every reason code, in declaration order.
    pub const ALL: [Self; 28] = [
        Self::PackageShape,
        Self::ImportShape,
        Self::DefaultShape,
        Self::RuleHead,
        Self::RuleBody,
        Self::LiteralKind,
        Self::WithModifier,
        Self::ExprKind,
        Self::OperatorKind,
        Self::AtomUnknown,
        Self::RuleLimit,
        Self::BodyLimit,
        Self::ModuleTarget,
        Self::OrderShape,
        Self::TriviaMetadata,
        Self::TriviaByte,
        Self::TokenMismatch,
        Self::ParseSyntax,
        Self::ParseNumber,
        Self::AstUnknownTag,
        Self::AstUnknownKey,
        Self::AstMissingKey,
        Self::AstType,
        Self::AstArity,
        Self::AstSpan,
        Self::AstModuleCount,
        Self::AstSourceMismatch,
        Self::AstSerialize,
    ];

    /// The codes a corpus fixture cannot reach.
    ///
    /// The `ast.*` codes belong to the closed decoder and to serialization, so
    /// their fixtures are the AST mutation sweep, not a source file. EP-M1
    /// records the partition; EP-M2 keeps both halves honest.
    pub const AST_CODES: [Self; 9] = [
        Self::AstUnknownTag,
        Self::AstUnknownKey,
        Self::AstMissingKey,
        Self::AstType,
        Self::AstArity,
        Self::AstSpan,
        Self::AstModuleCount,
        Self::AstSourceMismatch,
        Self::AstSerialize,
    ];

    /// The dotted wire name used in the manifest.
    #[must_use]
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::PackageShape => "package.shape",
            Self::ImportShape => "import.shape",
            Self::DefaultShape => "default.shape",
            Self::RuleHead => "rule.head",
            Self::RuleBody => "rule.body",
            Self::LiteralKind => "literal.kind",
            Self::WithModifier => "with.modifier",
            Self::ExprKind => "expr.kind",
            Self::OperatorKind => "operator.kind",
            Self::AtomUnknown => "atom.unknown",
            Self::RuleLimit => "rule.limit",
            Self::BodyLimit => "body.limit",
            Self::ModuleTarget => "module.target",
            Self::OrderShape => "order.shape",
            Self::TriviaMetadata => "trivia.metadata",
            Self::TriviaByte => "trivia.byte",
            Self::TokenMismatch => "token.mismatch",
            Self::ParseSyntax => "parse.syntax",
            Self::ParseNumber => "parse.number",
            Self::AstUnknownTag => "ast.unknown_tag",
            Self::AstUnknownKey => "ast.unknown_key",
            Self::AstMissingKey => "ast.missing_key",
            Self::AstType => "ast.type",
            Self::AstArity => "ast.arity",
            Self::AstSpan => "ast.span",
            Self::AstModuleCount => "ast.module_count",
            Self::AstSourceMismatch => "ast.source_mismatch",
            Self::AstSerialize => "ast.serialize",
        }
    }

    /// The reason code named by `wire`, if there is one.
    #[must_use]
    pub fn from_wire(wire: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|code| code.as_wire() == wire)
    }
}

/// Parse every non-empty line of the manifest.
pub fn parse(text: &str) -> Result<Vec<Entry>, String> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_entry)
        .collect()
}

/// Parse one manifest line.
fn parse_entry(line: &str) -> Result<Entry, String> {
    let value: Json = serde_json::from_str(line)
        .map_err(|error| format!("manifest line is not JSON: {error}"))?;
    let object = as_object(&value, "manifest entry")?;
    check_keys(object, &["path", "generated", "covers", "expected"])?;
    Ok(Entry {
        origin: parse_origin(object)?,
        covers: parse_covers(object)?,
        expected: parse_expected(object)?,
    })
}

/// Read the entry's origin, requiring exactly one of the two keys.
fn parse_origin(object: &Map<String, Json>) -> Result<Origin, String> {
    match (object.get("path"), object.get("generated")) {
        (Some(path), None) => Ok(Origin::File(string(path, "path")?.to_owned())),
        (None, Some(name)) => Ok(Origin::Generated(string(name, "generated")?.to_owned())),
        _ => Err("a manifest entry needs exactly one of `path` and `generated`".to_owned()),
    }
}

/// Read the entry's declared coverage.
fn parse_covers(object: &Map<String, Json>) -> Result<Vec<NodeKind>, String> {
    let values = object
        .get("covers")
        .and_then(Json::as_array)
        .ok_or_else(|| "a manifest entry needs a `covers` array".to_owned())?;
    values
        .iter()
        .map(|value| {
            let name = string(value, "covers item")?;
            NodeKind::from_name(name)
                .ok_or_else(|| format!("`covers` names an unknown node kind `{name}`"))
        })
        .collect()
}

/// Read the entry's expected outcome.
fn parse_expected(object: &Map<String, Json>) -> Result<Expected, String> {
    let raw = object
        .get("expected")
        .ok_or_else(|| "a manifest entry needs an `expected` object".to_owned())?;
    let expected = as_object(raw, "expected")?;
    match expected.get("outcome").and_then(Json::as_str) {
        Some("admitted") => {
            check_keys(expected, &["outcome", "ir"])?;
            Ok(Expected::Admitted {
                ir: string(
                    expected
                        .get("ir")
                        .ok_or_else(|| "an admitted entry needs an `ir` path".to_owned())?,
                    "ir",
                )?
                .to_owned(),
            })
        }
        Some("rejected") => {
            check_keys(expected, &["outcome", "stage", "code"])?;
            let stage = string(
                expected
                    .get("stage")
                    .ok_or_else(|| "a rejected entry needs a `stage`".to_owned())?,
                "stage",
            )?;
            let code = string(
                expected
                    .get("code")
                    .ok_or_else(|| "a rejected entry needs a `code`".to_owned())?,
                "code",
            )?;
            Ok(Expected::Rejected {
                stage: Stage::from_wire(stage).ok_or_else(|| format!("unknown stage `{stage}`"))?,
                code: ReasonCode::from_wire(code)
                    .ok_or_else(|| format!("unknown reason code `{code}`"))?,
            })
        }
        Some(other) => Err(format!("unknown outcome `{other}`")),
        None => Err("an expected object needs an `outcome`".to_owned()),
    }
}

/// Require `value` to be an object.
fn as_object<'a>(value: &'a Json, what: &str) -> Result<&'a Map<String, Json>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{what} is not an object"))
}

/// Require `value` to be a string.
fn string<'a>(value: &'a Json, what: &str) -> Result<&'a str, String> {
    value
        .as_str()
        .ok_or_else(|| format!("{what} is not a string"))
}

/// Reject every key outside `permitted`.
fn check_keys(object: &Map<String, Json>, permitted: &[&str]) -> Result<(), String> {
    for key in object.keys() {
        if !permitted.contains(&key.as_str()) {
            return Err(format!("unknown manifest key `{key}`"));
        }
    }
    Ok(())
}
