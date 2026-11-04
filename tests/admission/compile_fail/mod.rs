//! A compile-fail guard for the exhaustive matches the walkers rely on.
//!
//! `inventory::walk` and `ast::count` both promise that their matches over the
//! pinned Regorus AST, and over the decoded tree, stop compilation when a
//! variant is added and left unhandled. Nothing exercised that promise, so this
//! module does. It copies the repository into a scratch directory beside the
//! build's target directory and compiles the copy's `admission` test binary
//! three times:
//!
//! 1. unmodified, which must compile, and is the control that rules out an environment failure
//!    masquerading as a detected defect;
//! 2. with one arm removed from each protected match, which must stop the build with the
//!    non-exhaustive-patterns error `E0004` naming both files; and
//! 3. with each removed arm replaced by a catch-all, which must compile, and so shows that phase
//!    2's failure came from exhaustiveness checking alone.
//!
//! Removing an arm produces the same diagnostic a new, unhandled variant
//! produces, and it exercises the real matches rather than a stand-in copied
//! into the harness. A wildcard left in the repository would let phase 2
//! compile, so the guard fails for exactly the weakening it exists to prevent.
//!
//! The copy shares the build's target directory, so the nested build reuses the
//! dependency artefacts the surrounding test run has already produced. This is
//! a compile-time alarm about a completeness contract: it establishes no
//! admission decision and no proof evidence.
//!
//! Every filesystem operation goes through a [`Dir`] capability handle rather
//! than ambient `std::fs` calls, which the `no_std_fs_operations` lint denies.
//! The guard opens two ambient handles, one at the repository and one at the
//! build's target directory, and performs all copying, rewriting, and removal
//! through them.

use std::{
    env,
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU32, Ordering},
};

use cap_std::{ambient_authority, fs::Dir};

/// Directories the copy never needs: version-control state, build output, and
/// tool installations that cost far more than the sources.
const SKIPPED: [&str; 6] = [
    ".git",
    ".polean",
    ".pytest_cache",
    ".verus",
    "__pycache__",
    "target",
];

/// The protected matches, as the file that holds the match and the arm the
/// guard removes from it.
const PROTECTED: [(&str, &str); 2] = [
    (
        "tests/admission/inventory/walk.rs",
        "        Expr::Null { .. } => bump(counts, NodeKind::ExprNull),\n",
    ),
    (
        "tests/admission/ast/count.rs",
        "        ExprKind::Null => bump(counts, NodeKind::ExprNull),\n",
    ),
];

/// The catch-all a weakened match would use in place of the removed arm.
const WILDCARD: &str = "        _ => {}\n";

/// The build's target directory, so a nested build reuses its artefacts.
///
/// Cargo marks the root of its target directory with `CACHEDIR.TAG`, and the
/// running test binary always lives beneath it, so the directory is found by
/// walking up from `current_exe`. Reading `CARGO_TARGET_DIR` is not an option:
/// `clippy.toml` forbids ambient environment reads, and the variable is
/// normally unset.
///
/// # Errors
///
/// Returns a description of a missing binary path or a binary outside a Cargo
/// target directory.
fn target_dir() -> io::Result<PathBuf> {
    let exe = env::current_exe()?;
    let root = Dir::open_ambient_dir("/", ambient_authority())?;
    exe.ancestors()
        .find(|dir| {
            let marker = dir.join("CACHEDIR.TAG");
            let relative = marker.strip_prefix("/").unwrap_or(&marker);
            root.metadata(relative)
                .is_ok_and(|metadata| metadata.is_file())
        })
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            io::Error::other(format!("no Cargo target directory above {}", exe.display()))
        })
}

/// A name no other run of the guard in this process uses.
fn unique_name() -> String {
    /// How many scratch directories this process has created.
    static CREATED: AtomicU32 = AtomicU32::new(0);
    let ordinal = CREATED.fetch_add(1, Ordering::Relaxed);
    format!("scratch-{}-{ordinal}", std::process::id())
}

/// A scratch copy of the repository, removed when it is dropped.
struct Scratch {
    /// The directory that holds the copy, held open so removal needs no second
    /// ambient lookup.
    parent: Dir,
    /// The copy's name beneath `parent`.
    relative: PathBuf,
    /// The copy's root, opened for reading and rewriting its files.
    copy: Dir,
    /// The copy's root as an absolute path, which the nested build needs as a
    /// working directory.
    path: PathBuf,
}

impl Scratch {
    /// Copy the tree at `source` into a fresh directory under `parent`.
    fn create(source: &Dir, parent: &Path) -> io::Result<Self> {
        let holding = Dir::open_ambient_dir(parent, ambient_authority())?;
        holding.create_dir_all("compile-fail")?;
        let relative = Path::new("compile-fail").join(unique_name());
        holding.create_dir(&relative)?;
        let copy = holding.open_dir(&relative)?;
        copy_tree(source, &copy)?;
        Ok(Self {
            path: parent.join(&relative),
            parent: holding,
            relative,
            copy,
        })
    }

    /// The copy's root as a path.
    fn root(&self) -> &Path { &self.path }

    /// The copy's root as a capability handle.
    const fn dir(&self) -> &Dir { &self.copy }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // A directory left behind under the target directory is untidy, not a
        // failure of the guard, and reporting it would mask the real result.
        drop(self.parent.remove_dir_all(&self.relative));
    }
}

/// Copy the tree at `source` into `destination`, skipping [`SKIPPED`].
fn copy_tree(source: &Dir, destination: &Dir) -> io::Result<()> {
    for entry in source.entries()? {
        let item = entry?;
        let name = item.file_name();
        if SKIPPED.iter().any(|skipped| name == OsStr::new(skipped)) {
            continue;
        }
        if item.file_type()?.is_dir() {
            destination.create_dir(&name)?;
            copy_tree(&item.open_dir()?, &destination.open_dir(&name)?)?;
        } else {
            source.copy(&name, destination, &name)?;
        }
    }
    Ok(())
}

/// Compile the copy's `admission` test binary without running it.
fn compile(root: &Path, target: &Path) -> io::Result<Output> {
    Command::new(env!("CARGO"))
        .args([
            "test",
            "--no-run",
            "--test",
            "admission",
            "--all-features",
            "--offline",
        ])
        .current_dir(root)
        .env("CARGO_TARGET_DIR", target)
        .output()
}

/// Replace `arm` with `replacement` in one file of the copy.
///
/// The arm must occur exactly once, so an edited match is reported rather than
/// silently left untested.
fn substitute(copy: &Dir, relative: &str, arm: &str, replacement: &str) -> io::Result<()> {
    let text = copy.read_to_string(relative)?;
    let occurrences = text.matches(arm).count();
    if occurrences != 1 {
        return Err(io::Error::other(format!(
            "{relative} should hold its protected arm exactly once, found {occurrences}"
        )));
    }
    copy.write(relative, text.replacen(arm, replacement, 1))
}

/// Copy the pristine `relative` from `source` over the copy's version.
fn restore(source: &Dir, copy: &Dir, relative: &str) -> io::Result<()> {
    source.copy(relative, copy, relative).map(drop)
}

/// The files at which `stderr` reports a non-exhaustive match.
///
/// A Rust diagnostic puts the location on the line after its heading. Only the
/// heading form `error[E0004]` is read, so an unrelated error cannot satisfy an
/// assertion meant for the exhaustiveness check.
fn e0004_sites(stderr: &str) -> Vec<String> {
    let mut sites = Vec::new();
    let mut awaiting_location = false;
    for line in stderr.lines() {
        if line.starts_with("error[E0004]") {
            awaiting_location = true;
        } else if awaiting_location {
            if let Some(location) = line.trim_start().strip_prefix("--> ") {
                let file = location.split(':').next().unwrap_or(location);
                sites.push(file.to_owned());
                awaiting_location = false;
            } else if line.starts_with("error") {
                awaiting_location = false;
            }
        }
    }
    sites
}

/// The tail of the compiler's diagnostics, for an assertion message.
fn excerpt(output: &Output) -> String {
    let text = String::from_utf8_lossy(&output.stderr);
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(40);
    lines
        .iter()
        .skip(start)
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
}

mod tests;
