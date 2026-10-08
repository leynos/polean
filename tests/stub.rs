//! Integration checks for the disposable repository scaffold.

#[test]
fn scaffold_identifies_the_project() {
    assert_eq!(polean::project_name(), "Polean", "project identity");
    assert_eq!(polean::STATUS, "design scaffold", "implementation status");
}
