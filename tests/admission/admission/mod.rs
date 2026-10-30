//! Admission for `polean.tenant-write.v0`.
//!
//! Admission is a conjunction of three checks. The closed decoder turns the
//! frontend's AST JSON into a Polean-owned tree, the profile walk decides
//! whether that tree is the profile's module, and the canonical-token
//! recognizer decides whether the source is written as that module. Only the
//! last can see what the AST discards.
//!
//! Two outcomes are distinguished from a harness failure. [`Decision`] is what
//! admission decided about a source; [`Classification`] also records what the
//! frontend itself did and what the recognizer did, so a caller can tell
//! "Regorus refused this" from "the walk refused this" from "the recognizer
//! refused this". A contract or fixture that cannot be read is neither: those
//! surface as `Err(String)` from the corpus module.

use crate::{
    ast,
    corpus::{Expected, Profile, ReasonCode, Stage},
    recognizer,
};

mod walk;

/// A source range, as a rejection reports it.
pub type ByteSpan = (usize, usize);

/// Why admission rejected a source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rejection {
    /// The stage that rejected the source.
    pub stage: Stage,
    /// The reason code that stage reported.
    pub code: ReasonCode,
    /// The source range the rejection points at, when one was available.
    pub span: Option<ByteSpan>,
    /// A reviewable description of the first failed check.
    pub detail: String,
}

impl Rejection {
    /// Build a rejection.
    #[must_use]
    pub fn new(
        stage: Stage,
        code: ReasonCode,
        span: Option<ByteSpan>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            stage,
            code,
            span,
            detail: detail.into(),
        }
    }

    /// Build a profile-stage rejection.
    #[must_use]
    pub(super) fn profile(
        code: ReasonCode,
        span: Option<ByteSpan>,
        detail: impl Into<String>,
    ) -> Self {
        Self::new(Stage::Profile, code, span, detail)
    }

    /// Build a decode-stage rejection from a decoder fault.
    #[must_use]
    pub fn from_fault(fault: &ast::Fault) -> Self {
        Self::new(Stage::Decode, fault.code, None, fault.describe())
    }

    /// Render the rejection for a snapshot or a failure message.
    #[must_use]
    pub fn describe(&self) -> String {
        let span = self
            .span
            .map_or_else(String::new, |(start, end)| format!(" [{start}, {end})"));
        format!(
            "{} {}{span}: {}",
            self.stage.as_wire(),
            self.code.as_wire(),
            self.detail
        )
    }
}

/// What admission decided about a source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Decision {
    /// The module is the profile's module, with one atom name per literal.
    Admitted(Vec<Vec<String>>),
    /// The module is not the profile's module, or was not decodable.
    Rejected(Rejection),
}

/// What the pinned frontend did with one source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Frontend {
    /// The frontend parsed the source and serialized its AST.
    Parsed(String),
    /// The frontend refused the source's syntax.
    Rejected {
        /// The reason code the refusal maps to.
        code: ReasonCode,
        /// The frontend's own diagnostic.
        message: String,
    },
    /// The frontend parsed the source but could not serialize its AST.
    SerializeFailed {
        /// The frontend's own diagnostic.
        message: String,
    },
}

/// The full classification of one source: the frontend, then admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Classification {
    /// What the frontend did.
    pub frontend: Frontend,
    /// What admission decided, when the frontend produced AST JSON.
    pub decision: Option<Decision>,
    /// What the recognizer did, when the walk admitted the module.
    ///
    /// `Some(Ok)` gives every canonical token's start offset; `Some(Err)` gives
    /// the offset at which recognition stopped. `None` means the recognizer did
    /// not run, because the module never reached it.
    pub recognizer: Option<Result<Vec<usize>, usize>>,
}

/// Classify one source with the pinned frontend and admission.
#[must_use]
pub fn classify(profile: &Profile, source: &str) -> Classification {
    match crate::frontend::parse(source) {
        Err(message) => Classification {
            frontend: Frontend::Rejected {
                code: parse_code(&message),
                message,
            },
            decision: None,
            recognizer: None,
        },
        Ok(engine) => match engine.get_ast_as_json() {
            Ok(ast_json) => {
                let (decision, recognizer) = decide(profile, source, &ast_json);
                Classification {
                    frontend: Frontend::Parsed(ast_json),
                    decision: Some(decision),
                    recognizer,
                }
            }
            Err(error) => Classification {
                frontend: Frontend::SerializeFailed {
                    message: error.to_string(),
                },
                decision: None,
                recognizer: None,
            },
        },
    }
}

/// Map a frontend diagnostic to the parse reason code it reports.
///
/// A number the frontend cannot represent is the only parse failure the
/// profile distinguishes (D15). Every other refusal is a syntax failure.
fn parse_code(message: &str) -> ReasonCode {
    if message.contains("invalid number") {
        ReasonCode::ParseNumber
    } else {
        ReasonCode::ParseSyntax
    }
}

/// Admit one source from its AST JSON.
#[must_use]
pub fn admit(profile: &Profile, source: &str, ast_json: &str) -> Decision {
    decide(profile, source, ast_json).0
}

/// Run admission, recording what the recognizer did as well as what it decided.
///
/// The recognizer runs only on a module the walk admitted, so its trace is
/// absent exactly when the module never reached it.
fn decide(
    profile: &Profile,
    source: &str,
    ast_json: &str,
) -> (Decision, Option<Result<Vec<usize>, usize>>) {
    let module = match ast::decode(source, ast_json) {
        Ok(module) => module,
        Err(fault) => return (Decision::Rejected(Rejection::from_fault(&fault)), None),
    };
    let ir = match walk::map(profile, &module) {
        Ok(ir) => ir,
        Err(rejection) => return (Decision::Rejected(rejection), None),
    };
    match recognizer::check(profile, source, &ir) {
        Ok(offsets) => (Decision::Admitted(ir), Some(Ok(offsets))),
        Err(recognizer::Refusal::Offset(offset)) => {
            let rejection = recognizer::rejection(source, offset);
            (Decision::Rejected(rejection), Some(Err(offset)))
        }
        // The walk rejects an unregistered atom before the recognizer runs, so
        // this branch is a defect in the walk. Reporting it as the profile
        // rejection the source would deserve keeps admission failing closed.
        Err(recognizer::Refusal::Harness(detail)) => (
            Decision::Rejected(Rejection::profile(ReasonCode::AtomUnknown, None, detail)),
            None,
        ),
    }
}

/// How one classification compares with its manifest entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Comparison {
    /// The classification is exactly what the manifest states.
    Match,
    /// The classification is something else; the string describes the
    /// disagreement.
    Mismatch(String),
}

/// Compare one classification with the outcome the manifest states.
#[must_use]
pub fn compare(expected: &Expected, classification: &Classification) -> Comparison {
    match (expected, &classification.frontend) {
        (Expected::Rejected { stage, code }, Frontend::Rejected { code: actual, .. })
            if *stage == Stage::Parse =>
        {
            matched(*code == *actual, || {
                format!(
                    "expected parse/{} but the frontend reported {}",
                    code.as_wire(),
                    actual.as_wire()
                )
            })
        }
        (Expected::Rejected { stage, code }, Frontend::SerializeFailed { .. })
            if *stage == Stage::Serialize =>
        {
            matched(*code == ReasonCode::AstSerialize, || {
                "expected serialize/ast.serialize".to_owned()
            })
        }
        (declared, Frontend::Rejected { code, .. }) => Comparison::Mismatch(format!(
            "expected {declared:?} but the frontend rejected the source with {}",
            code.as_wire()
        )),
        (declared, Frontend::SerializeFailed { message }) => Comparison::Mismatch(format!(
            "expected {declared:?} but the frontend could not serialize its AST: {message}"
        )),
        (declared, Frontend::Parsed(_)) => compare_decision(declared, classification),
    }
}

/// Compare the walk's decision with the manifest entry.
fn compare_decision(expected: &Expected, classification: &Classification) -> Comparison {
    match (expected, &classification.decision) {
        (_, None) => Comparison::Mismatch(
            "the frontend parsed the source but no decision was recorded".to_owned(),
        ),
        (Expected::Admitted { ir }, Some(Decision::Admitted(actual))) => {
            match crate::corpus::ir_rules(ir) {
                Ok(want) if want == *actual => Comparison::Match,
                Ok(want) => Comparison::Mismatch(format!(
                    "expected IR {want:?} but the walk admitted {actual:?}"
                )),
                Err(error) => Comparison::Mismatch(error),
            }
        }
        (Expected::Admitted { .. }, Some(Decision::Rejected(rejection))) => {
            Comparison::Mismatch(format!(
                "expected admission but the walk rejected: {}",
                rejection.describe()
            ))
        }
        (Expected::Rejected { stage, code }, Some(Decision::Rejected(rejection))) => {
            if rejection.stage == *stage && rejection.code == *code {
                Comparison::Match
            } else {
                Comparison::Mismatch(format!(
                    "expected {}/{} but admission reported {}",
                    stage.as_wire(),
                    code.as_wire(),
                    rejection.describe()
                ))
            }
        }
        (Expected::Rejected { stage, code }, Some(Decision::Admitted(ir))) => {
            Comparison::Mismatch(format!(
                "expected {}/{} but admission admitted {ir:?}",
                stage.as_wire(),
                code.as_wire()
            ))
        }
    }
}

/// Turn a boolean comparison into a [`Comparison`], naming the fault if false.
fn matched<F>(ok: bool, fault: F) -> Comparison
where
    F: FnOnce() -> String,
{
    if ok {
        Comparison::Match
    } else {
        Comparison::Mismatch(fault())
    }
}

#[cfg(test)]
mod tests;
