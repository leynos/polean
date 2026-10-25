//! Tests for [`require_executable`], the one place an unreadable install
//! becomes contributor advice.
//!
//! The failures must stay distinguishable: advice to re-provision is only
//! correct when the tool is genuinely absent, and it is expensive advice to
//! follow when an installed tree is merely unreadable. These cases take an
//! explicit [`Path`], so they run under `make test` without provisioning.

use std::{
    fs,
    path::{Path, PathBuf},
};

use super::require_executable;

/// A scratch path for one test, under the directory Cargo provides for it.
///
/// Each caller passes a name unique to itself: the tests run in one
/// process and share `CARGO_TARGET_TMPDIR`.
fn scratch(name: &str) -> PathBuf { Path::new(env!("CARGO_TARGET_TMPDIR")).join(name) }

/// An installed tool, once found, is accepted without being run.
#[test]
fn an_installed_tool_is_accepted() {
    let path = scratch("installed-tool-probe");
    let created = fs::write(&path, b"");
    assert!(
        created.is_ok(),
        "could not create the scratch file {}: {created:?}",
        path.display()
    );
    assert_eq!(
        require_executable(&path),
        Ok(()),
        "an existing tool must be accepted"
    );
}

/// An absent tool is reported with the `make tools` remediation.
#[test]
fn an_absent_tool_asks_for_provisioning() {
    let path = scratch("absent-tool-probe");
    // A previous run of this test leaves the path absent, but the name is
    // reused across runs, so remove it rather than assume.
    if path.exists() {
        let removed = fs::remove_file(&path);
        assert!(
            removed.is_ok(),
            "could not clear the leftover {}: {removed:?}",
            path.display()
        );
    }

    let Err(message) = require_executable(&path) else {
        panic!(
            "{} does not exist, so the helper must report it missing",
            path.display()
        );
    };
    assert!(
        message.contains(&path.display().to_string()),
        "the diagnostic {message:?} must name {}",
        path.display()
    );
    assert!(
        message.contains("make tools"),
        "the diagnostic {message:?} must tell the contributor to provision the tool"
    );
}

/// An unreadable tool reports why, and does not claim it is absent.
///
/// Without this case, the presence check collapses "not installed" and
/// "could not look" into the same advice. Traversing a regular file yields
/// `ENOTDIR`, which no process can override, so the arm is reachable on
/// every host rather than only where permissions are enforced.
#[test]
fn an_unreachable_tool_reports_the_underlying_diagnostic() {
    let parent = scratch("unreachable-tool-probe");
    let blocker = parent.join("blocker");
    let unreachable = blocker.join("opa");

    let created = fs::create_dir_all(&parent).and_then(|()| fs::write(&blocker, b""));
    assert!(
        created.is_ok(),
        "could not lay out the scratch tree under {}: {created:?}",
        parent.display()
    );

    let Err(message) = require_executable(&unreachable) else {
        panic!(
            "{} cannot be reached through a regular file, so the helper must fail rather than \
             report it missing",
            unreachable.display()
        );
    };
    assert!(
        message.contains(&unreachable.display().to_string()),
        "the diagnostic {message:?} must name {}",
        unreachable.display()
    );
    assert!(
        message.contains("cannot inspect"),
        "the diagnostic {message:?} must carry the underlying reason, not only the path"
    );
    assert!(
        !message.contains("make tools"),
        "the diagnostic {message:?} must not send a contributor to re-provision a tree that is \
         already installed but unreadable"
    );
}
