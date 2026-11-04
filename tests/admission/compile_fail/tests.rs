//! The compile-fail guard's own checks.
//!
//! The parser test is cheap and runs everywhere. The guard test compiles a copy
//! of the repository three times, so it is the slowest test in the binary; it
//! earns that cost by turning the walkers' documented completeness promise into
//! something a future edit cannot quietly break.

use std::{
    path::{Path, PathBuf},
    process::Output,
};

use cap_std::fs::Dir;

use super::{
    PROTECTED,
    Scratch,
    WILDCARD,
    compile,
    e0004_sites,
    excerpt,
    restore,
    substitute,
    target_dir,
};

/// A diagnostic shaped like the one `rustc` prints for an unhandled variant.
const SAMPLE: &str = "error[E0004]: non-exhaustive patterns: `Expr::Null { .. }` not covered\n \
                      --> tests/admission/inventory/walk.rs:143:9\n  |\n  |                        \
                      ^^^^^^^^^^^^^^^^^^^^ pattern `Expr::Null { .. }` not covered\n\nerror: \
                      could not compile `polean` (test \"admission\") due to 1 previous error\n";

#[test]
fn the_diagnostic_parser_reads_rustc_output() {
    assert_eq!(
        e0004_sites(SAMPLE),
        ["tests/admission/inventory/walk.rs"],
        "the parser names the file the exhaustiveness error points at"
    );
}

#[test]
fn unrelated_errors_are_not_read_as_exhaustiveness_failures() {
    let stderr = "error[E0433]: failed to resolve: use of undeclared crate or module `absent`\n \
                  --> tests/admission/ast/count.rs:12:9\n";

    assert!(
        e0004_sites(stderr).is_empty(),
        "only the exhaustiveness heading may satisfy the guard"
    );
}

#[test]
fn the_protected_matches_stop_the_build_when_a_variant_is_unhandled() {
    let source = repository();
    let target = target_directory();
    let copy = scratch(&source, &target);

    let control = compiled(copy.root(), &target, "the unmodified copy");
    assert!(
        control.status.success(),
        "the unmodified copy must compile, or a later failure proves nothing:\n{}",
        excerpt(&control)
    );

    remove_protected_arms(copy.dir());
    let removed = compiled(copy.root(), &target, "the copy without the protected arms");
    assert!(
        !removed.status.success(),
        "an unhandled variant must stop the build:\n{}",
        excerpt(&removed)
    );
    assert_both_files_are_reported(&removed);

    weaken_protected_matches(&source, copy.dir());
    let weakened = compiled(copy.root(), &target, "the copy with catch-all arms");
    assert!(
        weakened.status.success(),
        "a catch-all conceals the missing variant, so the guard must depend on the compiler's \
         exhaustiveness check rather than on an incidental error:\n{}",
        excerpt(&weakened)
    );
}

/// The repository root, as the capability handle the copy is taken from.
fn repository() -> Dir {
    match crate::corpus::root() {
        Ok(root) => root,
        Err(error) => panic!("cannot open the repository: {error}"),
    }
}

/// The build's target directory, as the parent of the scratch copy.
fn target_directory() -> PathBuf {
    match target_dir() {
        Ok(dir) => dir,
        Err(error) => panic!("cannot find the build's target directory: {error}"),
    }
}

/// A fresh copy of the repository beside the target directory.
fn scratch(source: &Dir, target: &Path) -> Scratch {
    match Scratch::create(source, target) {
        Ok(scratch) => scratch,
        Err(error) => panic!("cannot prepare a copy of the repository: {error}"),
    }
}

/// Remove every protected arm, so each match must become non-exhaustive.
fn remove_protected_arms(copy: &Dir) {
    for (relative, arm) in PROTECTED {
        if let Err(error) = substitute(copy, relative, arm, "") {
            panic!("cannot remove the protected arm from {relative}: {error}");
        }
    }
}

/// Require the failed build to name every protected file.
fn assert_both_files_are_reported(removed: &Output) {
    let sites = e0004_sites(&String::from_utf8_lossy(&removed.stderr));
    for (relative, _) in PROTECTED {
        assert!(
            sites.iter().any(|site| site == relative),
            "the build must report a non-exhaustive match in {relative}, saw {sites:?}:\n{}",
            excerpt(removed)
        );
    }
}

/// Replace every protected arm with a catch-all, concealing the variant.
fn weaken_protected_matches(source: &Dir, copy: &Dir) {
    for (relative, arm) in PROTECTED {
        if let Err(error) =
            restore(source, copy, relative).and_then(|()| substitute(copy, relative, arm, WILDCARD))
        {
            panic!("cannot weaken the protected match in {relative}: {error}");
        }
    }
}

/// Compile the copy, naming it in any failure to start the build.
fn compiled(root: &Path, target: &Path, phase: &str) -> Output {
    match compile(root, target) {
        Ok(output) => output,
        Err(error) => panic!("cannot build {phase}: {error}"),
    }
}
