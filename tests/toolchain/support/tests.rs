//! Tests for [`path_exists`], the helper that separates "not installed"
//! from "could not find out".
//!
//! `Ok(false)` is the one answer that makes a contributor re-download an
//! archive, so every case here is about never returning it for a path that
//! exists. The uninspectable cases rely only on POSIX path resolution on the
//! Linux host the pinned tuple targets: traversing a regular file yields
//! `ENOTDIR`, which no process can override.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use super::path_exists;

/// A scratch path for one test, under the directory Cargo provides for it.
///
/// Each caller passes a name unique to itself: the tests run in one
/// process and share `CARGO_TARGET_TMPDIR`.
fn scratch(name: &str) -> PathBuf { Path::new(env!("CARGO_TARGET_TMPDIR")).join(name) }

/// A path that does not exist is absence, not a failure to inspect.
#[test]
fn a_missing_path_reports_absence() {
    let path = scratch("missing-path-probe");
    assert_eq!(
        path_exists(&path),
        Ok(false),
        "a missing path must report absence, not a failure to inspect"
    );
}

/// A path that exists and can be inspected reports presence.
#[test]
fn a_readable_path_reports_presence() {
    let path = scratch("readable-path-probe");
    let created = fs::create_dir_all(&path);
    assert!(
        created.is_ok(),
        "could not create the scratch directory {}: {created:?}",
        path.display()
    );
    assert_eq!(
        path_exists(&path),
        Ok(true),
        "an existing readable path must report presence, not absence"
    );
}

/// A path beneath a regular file cannot be reached, so the helper fails
/// rather than reporting it absent.
///
/// The kernel answers `ENOTDIR` here, which is not `NotFound`, so this
/// reaches the error arm on every host, including one running as root.
#[test]
fn an_unreachable_path_reports_an_error() {
    let parent = scratch("unreachable-path-probe");
    let blocker = parent.join("blocker");
    let unreachable = blocker.join("installed");

    let created = fs::create_dir_all(&parent).and_then(|()| fs::write(&blocker, b""));
    assert!(
        created.is_ok(),
        "could not lay out the scratch tree under {}: {created:?}",
        parent.display()
    );

    let Err(message) = path_exists(&unreachable) else {
        panic!(
            "{} cannot be reached through a regular file, so the helper must fail rather than \
             report absence",
            unreachable.display()
        );
    };
    assert!(
        message.contains(&unreachable.display().to_string()),
        "the diagnostic {message:?} must name {}",
        unreachable.display()
    );
}

/// Withheld permission bits never turn an existing path into absence.
///
/// A process holding `CAP_DAC_OVERRIDE` — root, for example — reads through
/// the withheld mode. That is the one case where presence is the correct
/// answer; `an_unreachable_path_reports_an_error` pins the error arm
/// unconditionally, so neither test is skipped on any host.
#[test]
fn a_permission_denied_path_is_never_reported_as_absent() {
    let directory = scratch("permission-denied-path-probe");
    let installed = directory.join("installed");

    // A previous failed run may have left the directory hidden; a test must
    // not be unable to lay out its own fixture because it failed before.
    if directory.exists() {
        let reset = fs::set_permissions(&directory, fs::Permissions::from_mode(0o755));
        assert!(
            reset.is_ok(),
            "could not reset permission on the leftover {}: {reset:?}",
            directory.display()
        );
    }

    let created = fs::create_dir_all(&directory).and_then(|()| fs::write(&installed, b""));
    assert!(
        created.is_ok(),
        "could not lay out the scratch tree under {}: {created:?}",
        directory.display()
    );

    let hidden = fs::set_permissions(&directory, fs::Permissions::from_mode(0o000));
    assert!(
        hidden.is_ok(),
        "could not withhold permission on {}: {hidden:?}",
        directory.display()
    );

    // Ask twice while the directory is inaccessible: once through the
    // helper under test, and once through `fs::metadata` as an environment
    // probe. Reaching `installed` needs search permission on `directory`.
    let inspected = path_exists(&installed);
    let bypasses_permissions = fs::metadata(&installed).is_ok();

    let restored = fs::set_permissions(&directory, fs::Permissions::from_mode(0o755));
    assert!(
        restored.is_ok(),
        "could not restore permission on {}: {restored:?}",
        directory.display()
    );

    if bypasses_permissions {
        assert_eq!(
            inspected,
            Ok(true),
            "this process reads through the withheld mode, so an existing path must report \
             presence"
        );
        return;
    }

    let Err(message) = inspected else {
        panic!(
            "{} exists but cannot be inspected, so the helper must fail rather than report absence",
            installed.display()
        );
    };
    assert!(
        message.contains(&installed.display().to_string()),
        "the diagnostic {message:?} must name {}",
        installed.display()
    );
}
