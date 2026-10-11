//! Integration checks for the disposable repository scaffold.

/// `project_name` in a constant context.
///
/// Assigning the call to a `const` requires the compiler to evaluate it during
/// compilation, so making `project_name` non-`const` breaks the build rather
/// than escaping attention until a test run.
const PROJECT_NAME_AT_COMPILE_TIME: &str = polean::project_name();

/// `STATUS` in a constant context, for the same reason.
const STATUS_AT_COMPILE_TIME: &str = polean::STATUS;

/// Check that the scaffold reports its project identity and status.
#[test]
fn scaffold_identifies_the_project() {
    assert_eq!(polean::project_name(), "Polean", "project identity");
    assert_eq!(polean::STATUS, "design scaffold", "implementation status");
}

/// Check that the scaffold API remains usable where a constant is required.
///
/// The scaffold exists to be replaced, so this guards the one property that
/// callers compile against: both values are available at compile time.
#[test]
fn scaffold_api_evaluates_in_a_constant_context() {
    assert_eq!(
        PROJECT_NAME_AT_COMPILE_TIME, "Polean",
        "compile-time project identity"
    );
    assert_eq!(
        STATUS_AT_COMPILE_TIME, "design scaffold",
        "compile-time implementation status"
    );
}
