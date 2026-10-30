//! The one place the toolchain tests exercise ambient filesystem authority.
//!
//! The tests inspect and create paths named by absolute `Path`s (the repository
//! root, the pinned tool tree, a scratch directory). Rather than reach for
//! `std::fs`, which bypasses capability-based access, each helper opens the
//! filesystem root as a [`cap_std::fs::Dir`] and resolves the path beneath it.
//! The error kinds the tests depend on (`NotFound`, `ENOTDIR`, permission
//! denied) pass through unchanged.

use std::{
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use cap_std::{
    ambient_authority,
    fs::{Dir, Metadata, Permissions, PermissionsExt},
};

/// Open the filesystem root, the directory every absolute path resolves under.
fn root() -> io::Result<Dir> { Dir::open_ambient_dir("/", ambient_authority()) }

/// The path relative to the filesystem root, as `Dir` requires.
///
/// A capability refuses to follow a symbolic link whose target is absolute, yet
/// `.polean` may legitimately be such a link to another disk. The parent
/// directory is therefore resolved ambiently first, so the capability only ever
/// sees a link-free parent. A parent that is missing or is a file
/// (`NotFound`, `NotADirectory`) is not an error here: the path is used as
/// given and the capability reports the honest error for it. Every other
/// failure to resolve the parent, permission denied included, is returned.
fn beneath_root(path: &Path) -> io::Result<PathBuf> {
    let resolved = match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => match parent.canonicalize() {
            Ok(real) => real.join(name),
            Err(error)
                if matches!(error.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) =>
            {
                path.to_path_buf()
            }
            Err(error) => return Err(error),
        },
        _ => path.to_path_buf(),
    };
    Ok(resolved
        .strip_prefix("/")
        .unwrap_or(&resolved)
        .to_path_buf())
}

/// Inspect `path`, following symlinks.
///
/// # Errors
///
/// Returns the underlying error, with its kind intact.
pub fn metadata(path: &Path) -> io::Result<Metadata> { root()?.metadata(beneath_root(path)?) }

/// Read `path` as UTF-8 text.
///
/// # Errors
///
/// Returns the underlying error.
pub fn read_to_string(path: &Path) -> io::Result<String> {
    root()?.read_to_string(beneath_root(path)?)
}

/// Create `path` and any missing parents.
///
/// # Errors
///
/// Returns the underlying error.
pub fn create_dir_all(path: &Path) -> io::Result<()> { root()?.create_dir_all(beneath_root(path)?) }

/// Create the single directory `path`, failing if it exists.
///
/// # Errors
///
/// Returns the underlying error, `AlreadyExists` included.
pub fn create_dir(path: &Path) -> io::Result<()> { root()?.create_dir(beneath_root(path)?) }

/// Write `contents` to the file `path`, replacing it.
///
/// # Errors
///
/// Returns the underlying error.
pub fn write(path: &Path, contents: &[u8]) -> io::Result<()> {
    root()?.write(beneath_root(path)?, contents)
}

/// Set the permission bits of `path`.
///
/// # Errors
///
/// Returns the underlying error.
pub fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    root()?.set_permissions(beneath_root(path)?, Permissions::from_mode(mode))
}

/// Remove the empty directory `path`.
///
/// # Errors
///
/// Returns the underlying error.
pub fn remove_dir(path: &Path) -> io::Result<()> { root()?.remove_dir(beneath_root(path)?) }

/// Remove the directory `path` and everything beneath it.
///
/// # Errors
///
/// Returns the underlying error.
pub fn remove_dir_all(path: &Path) -> io::Result<()> { root()?.remove_dir_all(beneath_root(path)?) }

/// Create the symbolic link `link` pointing at `target`.
///
/// # Errors
///
/// Returns the underlying error.
pub fn symlink(target: &Path, link: &Path) -> io::Result<()> {
    // A capability cannot create a link to an absolute target, which is the
    // case under test.
    std::os::unix::fs::symlink(target, link)
}
