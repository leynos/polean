//! Tests for [`path_exists`], the helper that separates "not installed"
//! from "could not find out".
//!
//! `Ok(false)` is the one answer that makes a contributor re-download an
//! archive, so every case here is about never returning it for a path that
//! exists. The uninspectable cases rely only on POSIX path resolution on the
//! Linux host the pinned tuple targets: traversing a regular file yields
//! `ENOTDIR`, which no process can override.

use super::path_exists;
use crate::{
    ambient,
    scratch::{Scratch, check, check_eq},
};

/// A path that does not exist is absence, not a failure to inspect.
#[test]
fn a_missing_path_reports_absence() -> Result<(), String> {
    let mut scratch = Scratch::try_new("missing-path-probe")
        .map_err(|error| format!("cannot create scratch directory: {error}"))?;
    let path = scratch.path().join("missing");
    check_eq!(
        path_exists(&path),
        Ok(false),
        "a missing path must report absence, not a failure to inspect"
    );
    scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch directory: {error}"))
}

/// A path that exists and can be inspected reports presence.
#[test]
fn a_readable_path_reports_presence() -> Result<(), String> {
    let mut scratch = Scratch::try_new("readable-path-probe")
        .map_err(|error| format!("cannot create scratch directory: {error}"))?;
    let path = scratch.path().join("readable");
    let created = ambient::create_dir_all(&path);
    check!(
        created.is_ok(),
        "could not create the scratch directory {}: {created:?}",
        path.display()
    );
    check_eq!(
        path_exists(&path),
        Ok(true),
        "an existing readable path must report presence, not absence"
    );
    scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch directory: {error}"))
}

/// A path beneath a regular file cannot be reached, so the helper fails
/// rather than reporting it absent.
///
/// The kernel answers `ENOTDIR` here, which is not `NotFound`, so this
/// reaches the error arm on every host, including one running as root.
#[test]
fn an_unreachable_path_reports_an_error() -> Result<(), String> {
    let mut scratch = Scratch::try_new("unreachable-path-probe")
        .map_err(|error| format!("cannot create scratch directory: {error}"))?;
    let parent = scratch.path();
    let blocker = parent.join("blocker");
    let unreachable = blocker.join("installed");

    let created = ambient::create_dir_all(parent).and_then(|()| ambient::write(&blocker, b""));
    check!(
        created.is_ok(),
        "could not lay out the scratch tree under {}: {created:?}",
        parent.display()
    );

    let message = path_exists(&unreachable).err().ok_or_else(|| {
        format!(
            "{} unexpectedly reported absence or presence",
            unreachable.display()
        )
    })?;
    check!(
        message.contains(&unreachable.display().to_string()),
        "the diagnostic {message:?} must name {}",
        unreachable.display()
    );
    scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch directory: {error}"))
}

/// Withheld permission bits never turn an existing path into absence.
///
/// A process holding `CAP_DAC_OVERRIDE` — root, for example — reads through
/// the withheld mode. That is the one case where presence is the correct
/// answer; `an_unreachable_path_reports_an_error` pins the error arm
/// unconditionally, so neither test is skipped on any host.
#[test]
fn a_permission_denied_path_is_never_reported_as_absent() -> Result<(), String> {
    let mut scratch = Scratch::try_new("permission-denied-path-probe")
        .map_err(|error| format!("cannot create scratch directory: {error}"))?;
    let directory = scratch.path();
    let installed = directory.join("installed");

    let created = ambient::create_dir_all(directory).and_then(|()| ambient::write(&installed, b""));
    check!(
        created.is_ok(),
        "could not lay out the scratch tree under {}: {created:?}",
        directory.display()
    );

    let hidden = ambient::set_mode(directory, 0o000);
    check!(
        hidden.is_ok(),
        "could not withhold permission on {}: {hidden:?}",
        directory.display()
    );

    // Ask twice while the directory is inaccessible: once through the
    // helper under test, and once through `fs::metadata` as an environment
    // probe. Reaching `installed` needs search permission on `directory`.
    let inspected = path_exists(&installed);
    let bypasses_permissions = ambient::metadata(&installed).is_ok();

    let restored = ambient::set_mode(directory, 0o755);
    check!(
        restored.is_ok(),
        "could not restore permission on {}: {restored:?}",
        directory.display()
    );

    if bypasses_permissions {
        check_eq!(
            inspected,
            Ok(true),
            "this process reads through the withheld mode, so an existing path must report \
             presence"
        );
        return scratch
            .cleanup()
            .map_err(|error| format!("cannot clean scratch directory: {error}"));
    }

    let message = inspected.err().ok_or_else(|| {
        format!(
            "{} unexpectedly reported absence or presence",
            installed.display()
        )
    })?;
    check!(
        message.contains(&installed.display().to_string()),
        "the diagnostic {message:?} must name {}",
        installed.display()
    );
    scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch directory: {error}"))
}

/// A path reached through an absolute symlink is found, as `.polean` may be a
/// link to another disk.
#[test]
fn a_path_behind_an_absolute_symlink_is_found() -> Result<(), String> {
    let mut scratch = Scratch::try_new("symlink-path-probe")
        .map_err(|error| format!("cannot create scratch directory: {error}"))?;
    let real = scratch.path().join("real");
    let link = scratch.path().join("link");
    let created = ambient::create_dir_all(&real)
        .and_then(|()| ambient::write(&real.join("tool"), b""))
        .and_then(|()| ambient::symlink(&real, &link));
    check!(
        created.is_ok(),
        "could not lay out the symlinked tree under {}: {created:?}",
        scratch.path().display()
    );
    check_eq!(
        path_exists(&link.join("tool")),
        Ok(true),
        "a path behind an absolute symlink must report presence"
    );
    scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch directory: {error}"))
}

/// A parent that cannot be resolved for a reason other than absence is
/// reported, never folded into "not installed".
///
/// With search permission withheld from the grandparent, resolving the parent
/// fails with permission denied. A process that overrides permission bits
/// resolves it and finds the path, which is the correct answer for it.
#[test]
fn an_unresolvable_parent_is_never_reported_as_absent() -> Result<(), String> {
    let mut scratch = Scratch::try_new("unresolvable-parent-probe")
        .map_err(|error| format!("cannot create scratch directory: {error}"))?;
    let grandparent = scratch.path().join("outer");
    let tool = grandparent.join("inner").join("tool");
    let created = ambient::create_dir_all(&grandparent.join("inner"))
        .and_then(|()| ambient::write(&tool, b""));
    check!(
        created.is_ok(),
        "could not lay out the scratch tree under {}: {created:?}",
        grandparent.display()
    );
    let hidden = ambient::set_mode(&grandparent, 0o000);
    check!(
        hidden.is_ok(),
        "could not withhold permission on {}: {hidden:?}",
        grandparent.display()
    );
    let inspected = path_exists(&tool);
    let restored = ambient::set_mode(&grandparent, 0o755);
    check!(
        restored.is_ok(),
        "could not restore permission on {}: {restored:?}",
        grandparent.display()
    );
    check!(
        inspected != Ok(false),
        "a parent that cannot be resolved must not be reported as absent: {inspected:?}"
    );
    scratch
        .cleanup()
        .map_err(|error| format!("cannot clean scratch directory: {error}"))
}
