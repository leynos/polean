//! Tests for [`require_executable`], the one place an unreadable install
//! becomes contributor advice.
//!
//! The failures must stay distinguishable: advice to re-provision is only
//! correct when the tool is genuinely absent, and it is expensive advice to
//! follow when an installed tree is merely unreadable. These cases take an
//! explicit [`std::path::Path`], so they run under `make test` without provisioning.

use super::require_executable;
use crate::{
    ambient,
    scratch::{Scratch, check, check_eq},
};

/// An installed tool, once found, is accepted without being run.
#[test]
fn an_installed_tool_is_accepted() -> Result<(), String> {
    let mut scratch = Scratch::try_new("installed-tool-probe")
        .map_err(|error| format!("cannot create scratch directory: {error}"))?;
    let path = scratch.path().join("installed");
    let created = ambient::write(&path, b"");
    check!(
        created.is_ok(),
        "could not create the scratch file {}: {created:?}",
        path.display()
    );
    check_eq!(
        require_executable(&path),
        Ok(()),
        "an existing tool must be accepted"
    );
    scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch directory: {error}"))
}

/// An absent tool is reported with the `make tools` remediation.
#[test]
fn an_absent_tool_asks_for_provisioning() -> Result<(), String> {
    let mut scratch = Scratch::try_new("absent-tool-probe")
        .map_err(|error| format!("cannot create scratch directory: {error}"))?;
    let path = scratch.path().join("absent");
    let message = require_executable(&path)
        .err()
        .ok_or_else(|| format!("{} unexpectedly reported an installed tool", path.display()))?;
    check!(
        message.contains(&path.display().to_string()),
        "the diagnostic {message:?} must name {}",
        path.display()
    );
    check!(
        message.contains("make tools"),
        "the diagnostic {message:?} must tell the contributor to provision the tool"
    );
    scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch directory: {error}"))
}

/// An unreadable tool reports why, and does not claim it is absent.
///
/// Without this case, the presence check collapses "not installed" and
/// "could not look" into the same advice. Traversing a regular file yields
/// `ENOTDIR`, which no process can override, so the arm is reachable on
/// every host rather than only where permissions are enforced.
#[test]
fn an_unreachable_tool_reports_the_underlying_diagnostic() -> Result<(), String> {
    let mut scratch = Scratch::try_new("unreachable-tool-probe")
        .map_err(|error| format!("cannot create scratch directory: {error}"))?;
    let parent = scratch.path();
    let blocker = parent.join("blocker");
    let unreachable = blocker.join("opa");

    let created = ambient::create_dir_all(parent).and_then(|()| ambient::write(&blocker, b""));
    check!(
        created.is_ok(),
        "could not lay out the scratch tree under {}: {created:?}",
        parent.display()
    );

    let message = require_executable(&unreachable).err().ok_or_else(|| {
        format!(
            "{} unexpectedly reported an installed tool",
            unreachable.display()
        )
    })?;
    check!(
        message.contains(&unreachable.display().to_string()),
        "the diagnostic {message:?} must name {}",
        unreachable.display()
    );
    check!(
        message.contains("cannot inspect"),
        "the diagnostic {message:?} must carry the underlying reason, not only the path"
    );
    check!(
        !message.contains("make tools"),
        "the diagnostic {message:?} must not send a contributor to re-provision a tree that is \
         already installed but unreadable"
    );
    scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch directory: {error}"))
}
