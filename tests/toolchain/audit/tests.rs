//! Settlement unit tests over the shared fixture pairs.
//!
//! Each test names one corpus fixture and asserts the exact outcome, including
//! the order of findings. Exactness matters in both directions: a settlement
//! that accepted a `sorry` root would be unsound, and one that rejected a clean
//! root would be useless, and S1's proof runs in both directions for the same
//! reason.

use rstest::rstest;

use super::{
    fixtures::{self, Case},
    settlement::decision::{
        Finding,
        FindingCode,
        ProtocolBreach,
        RequestRefusal,
        Settlement,
        settle,
        validate_request,
    },
};

/// Decide a fixture pair.
fn decide(case: &Case) -> Result<Settlement, RequestRefusal> {
    settle(&case.request, &case.report, &case.pin)
}

/// Decide a fixture pair and require rejection, returning its findings.
fn expect_rejected(case: &Case) -> Vec<Finding> {
    match decide(case) {
        Ok(Settlement::Rejected(findings)) => findings,
        other => panic!("expected a rejection, got {other:?}"),
    }
}

/// Decide a fixture pair and require acceptance.
fn expect_accepted(case: &Case) {
    match decide(case) {
        Ok(Settlement::Accepted) => {}
        other => panic!("expected acceptance, got {other:?}"),
    }
}

/// Decide a fixture pair and require the named protocol breach.
fn expect_tool_failure(case: &Case, breach: ProtocolBreach) {
    match decide(case) {
        Ok(Settlement::ToolFailure(observed)) => {
            assert_eq!(observed, breach, "the wrong protocol breach was reported");
        }
        other => panic!("expected a tool failure, got {other:?}"),
    }
}

/// A finding about a whole root.
const fn root_finding(root: usize, code: FindingCode) -> Finding {
    Finding {
        root: Some(root),
        item: None,
        code,
    }
}

/// A finding about one axiom of a root.
const fn axiom_finding(root: usize, item: usize, code: FindingCode) -> Finding {
    Finding {
        root: Some(root),
        item: Some(item),
        code,
    }
}

/// A theorem with no axioms is accepted with no foundations declared.
#[test]
fn accepted_ground_is_accepted() { expect_accepted(&fixtures::accepted_ground()); }

/// A theorem whose transitive axioms are all approved is accepted.
#[test]
fn accepted_foundations_is_accepted() { expect_accepted(&fixtures::accepted_foundations()); }

/// Dependence on `sorryAx` is rejected, naming the axiom.
///
/// A directly used `sorry` and one reached through an import are the same
/// finding, because settlement sees the transitive axiom, not the syntax that
/// introduced it.
#[rstest]
#[case::direct("Sorry")]
#[case::imported("SorryImported")]
fn sorry_is_rejected_with_a_sorry_finding(#[case] fixture: &str) {
    let case = fixtures::sorry(fixture);
    assert_eq!(
        expect_rejected(&case),
        vec![axiom_finding(0, 0, FindingCode::SorryAxiom)],
        "a sorry root did not produce exactly one sorry finding"
    );
}

/// A native-computation axiom is rejected, naming the axiom.
#[test]
fn native_axiom_is_rejected_as_native() {
    let case = fixtures::native();
    assert_eq!(
        expect_rejected(&case),
        vec![axiom_finding(0, 0, FindingCode::NativeAxiom)],
        "a native axiom was not classified as native"
    );
}

/// An axiom the request never declares is rejected as undeclared.
#[test]
fn undeclared_axiom_is_rejected() {
    let case = fixtures::undeclared("Undeclared");
    assert_eq!(
        expect_rejected(&case),
        vec![axiom_finding(0, 0, FindingCode::UndeclaredAxiom)],
        "an undeclared axiom was not rejected"
    );
}

/// Approving one pinned foundation does not approve the others.
#[test]
fn narrowed_foundations_are_rejected() {
    let case = fixtures::narrowed_foundations();
    assert_eq!(
        expect_rejected(&case),
        vec![axiom_finding(0, 0, FindingCode::UndeclaredAxiom)],
        "a foundation outside the request's declared set was approved"
    );
}

/// A theorem of a different proposition is rejected.
#[test]
fn wrong_statement_is_rejected() {
    let case = fixtures::wrong_statement("WrongStatement");
    assert_eq!(
        expect_rejected(&case),
        vec![root_finding(0, FindingCode::StatementMismatch)],
        "a theorem of the wrong proposition was not rejected"
    );
}

/// An axiom root is the wrong kind and depends on an unapproved name.
#[test]
fn axiom_root_reports_wrong_kind_then_undeclared_axiom() {
    let case = fixtures::axiom_root();
    assert_eq!(
        expect_rejected(&case),
        vec![
            root_finding(0, FindingCode::WrongKind),
            axiom_finding(0, 0, FindingCode::UndeclaredAxiom),
        ],
        "an axiom root did not report both its kind and its dependency"
    );
}

/// A definition root is the wrong kind and has no axiom finding.
#[test]
fn def_root_reports_wrong_kind_only() {
    let case = fixtures::def_root();
    assert_eq!(
        expect_rejected(&case),
        vec![root_finding(0, FindingCode::WrongKind)],
        "a definition root reported more than its kind"
    );
}

/// A root the environment does not hold is reported absent.
#[test]
fn root_absent_is_reported() {
    let case = fixtures::root_absent();
    assert_eq!(
        expect_rejected(&case),
        vec![root_finding(0, FindingCode::RootAbsent)],
        "an absent root was not reported"
    );
}

/// A rejected replay is a completed audit that rejects.
#[test]
fn rejected_replay_reports_kernel_rejection() {
    let case = fixtures::rejected_replay("ForgeInit");
    assert_eq!(
        expect_rejected(&case),
        vec![Finding {
            root: None,
            item: None,
            code: FindingCode::KernelRejected,
        }],
        "a rejected replay did not report a whole-report kernel finding"
    );
}

/// A clean root beside an unsound root keeps the unsound root's evidence.
#[test]
fn mixed_roots_keep_per_root_evidence() {
    let case = fixtures::mixed_roots();
    assert_eq!(
        expect_rejected(&case),
        vec![axiom_finding(1, 0, FindingCode::SorryAxiom)],
        "a mixed report did not place the finding at the unsound root"
    );
}

/// A report that does not answer its request is a tool failure, not a
/// rejection.
///
/// The distinctions are contractual: a mismatched toolchain, module, or root
/// coverage means the report is about something else, and reporting that as a
/// policy rejection would blame the policy for the worker's fault.
#[rstest]
#[case::toolchain(fixtures::toolchain_mismatch, ProtocolBreach::ToolchainMismatch)]
#[case::module(fixtures::module_mismatch, ProtocolBreach::ModuleMismatch)]
#[case::coverage(fixtures::coverage_mismatch, ProtocolBreach::CoverageMismatch)]
#[case::root(fixtures::root_mismatch, ProtocolBreach::CoverageMismatch)]
#[case::rejected_with_roots(fixtures::rejected_with_roots, ProtocolBreach::RejectedReplayWithRoots)]
fn reports_about_another_request_are_tool_failures(
    #[case] fixture: fn() -> Case,
    #[case] breach: ProtocolBreach,
) {
    expect_tool_failure(&fixture(), breach);
}

/// A protocol breach outranks the findings the report would otherwise produce.
#[test]
fn protocol_breach_takes_precedence_over_findings() {
    let mut case = fixtures::sorry("Sorry");
    case.report.githash = b"0000000000000000000000000000000000000000".to_vec();
    expect_tool_failure(&case, ProtocolBreach::ToolchainMismatch);
}

/// Validation accepts an admissible request.
#[test]
fn validate_request_accepts_the_fixture_request() {
    let case = fixtures::accepted_foundations();
    assert_eq!(
        validate_request(&case.request),
        Ok(()),
        "an admissible request was refused"
    );
}

/// Validation and settlement name the same refusal for the same request.
///
/// The Verus proof and the Cargo build both rely on this parity: settlement
/// validates before deciding, so a divergence would let an inadmissible
/// request reach the decision logic.
#[rstest]
#[case::no_roots(RequestRefusal::NoRoots)]
#[case::duplicate_root(RequestRefusal::DuplicateRoot(1))]
#[case::unapproved_foundation(RequestRefusal::UnapprovedFoundation(0))]
fn validate_request_and_settle_name_the_same_refusal(#[case] refusal: RequestRefusal) {
    let mut case = fixtures::accepted_ground();
    match refusal {
        RequestRefusal::NoRoots => case.request.roots.clear(),
        RequestRefusal::DuplicateRoot(_) => {
            let Some(root) = case.request.roots.first().cloned() else {
                panic!("the fixture has no root");
            };
            case.request.roots.push(root);
        }
        RequestRefusal::UnapprovedFoundation(_) => {
            case.request.foundations = vec![fixtures::key(&["sorryAx"])];
        }
    }
    assert_eq!(
        validate_request(&case.request),
        Err(refusal),
        "validation named another refusal"
    );
    assert_eq!(
        decide(&case),
        Err(refusal),
        "settlement named another refusal"
    );
}

/// Acceptance is closed over the three pinned foundations.
#[test]
fn accepted_axioms_are_exactly_the_pinned_foundations() {
    let case = fixtures::accepted_foundations();
    expect_accepted(&case);
    let Some(root) = case.report.roots.first() else {
        panic!("the fixture has no root");
    };
    let Some(decl) = &root.decl else {
        panic!("the fixture root has no declaration");
    };
    let names: Vec<&Vec<u8>> = decl.axioms.iter().map(|axiom| &axiom.name).collect();
    let expected = [
        fixtures::key(&["propext"]),
        fixtures::key(&["Quot", "sound"]),
        fixtures::key(&["Classical", "choice"]),
    ];
    assert_eq!(
        names.len(),
        expected.len(),
        "an accepted report did not carry exactly the pinned foundations"
    );
    for key in &expected {
        assert!(
            names.contains(&key),
            "an accepted report omitted a pinned foundation"
        );
    }
}
