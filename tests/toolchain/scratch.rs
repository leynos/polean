//! Isolated, automatically cleaned scratch directories for toolchain tests.

use std::{
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::ambient;

/// Return the test's diagnostic when its condition is false.
macro_rules! check {
    ($condition:expr, $($message:tt)+) => {
        if !$condition {
            return Err(format!($($message)+));
        }
    };
}

/// Compare values while preserving their actual values in the diagnostic.
macro_rules! check_eq {
    ($actual:expr, $expected:expr, $($message:tt)+) => {
        match (&$actual, &$expected) {
            (actual, expected) if actual != expected => {
                return Err(format!("{}: actual {actual:?}, expected {expected:?}", format_args!($($message)+)));
            }
            _ => (),
        }
    };
}

/// Check that values differ.
macro_rules! check_ne {
    ($actual:expr, $expected:expr, $($message:tt)+) => {
        match (&$actual, &$expected) {
            (actual, expected) if actual == expected => {
                return Err(format!("{}: both {actual:?}", format_args!($($message)+)));
            }
            _ => (),
        }
    };
}

pub(crate) use check;
pub(crate) use check_eq;

/// Distinguishes directories created by concurrent tests in this process.
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// A uniquely created directory beneath Cargo's test scratch directory.
pub struct Scratch {
    /// Directory owned until explicitly cleaned.
    path: PathBuf,
    /// Whether cleanup has already completed.
    cleaned: bool,
}

impl Scratch {
    /// Create a scratch directory without reusing a leftover or concurrent one.
    ///
    /// # Errors
    ///
    /// Returns the filesystem error if the directory cannot be created.
    pub fn try_new(name: &str) -> io::Result<Self> {
        loop {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let path = Path::new(env!("CARGO_TARGET_TMPDIR"))
                .join(format!("{name}-{}-{id}", std::process::id()));
            match ambient::create_dir(&path) {
                Ok(()) => {
                    return Ok(Self {
                        path,
                        cleaned: false,
                    });
                }
                Err(error) if error.kind() == ErrorKind::AlreadyExists => (),
                Err(error) => return Err(error),
            }
        }
    }

    /// The directory owned by this guard.
    pub fn path(&self) -> &Path { &self.path }

    /// Restore permissions and remove the owned directory.
    ///
    /// # Errors
    ///
    /// Returns the filesystem error if permissions or removal fail.
    pub fn cleanup(&mut self) -> io::Result<()> {
        ambient::set_mode(&self.path, 0o755)?;
        ambient::remove_dir_all(&self.path)?;
        self.cleaned = true;
        Ok(())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // Explicit cleanup reports errors; this is a best-effort unwind fallback.
        if !self.cleaned {
            drop(self.cleanup());
        }
    }
}

#[cfg(test)]
mod tests {
    //! Check isolation, cleanup, and error reporting at the scratch boundary.

    use super::Scratch;
    use crate::ambient;

    /// Distinct guards do not share state; cleanup restores withheld permissions.
    #[test]
    fn unique_directories_are_removed_after_permission_change() -> Result<(), String> {
        let mut first = Scratch::try_new("isolation").map_err(|error| error.to_string())?;
        let second = Scratch::try_new("isolation").map_err(|error| error.to_string())?;
        check_ne!(
            first.path(),
            second.path(),
            "scratch directories must be unique"
        );
        let first_path = first.path().to_path_buf();
        ambient::set_mode(&first_path, 0o000).map_err(|error| error.to_string())?;
        first.cleanup().map_err(|error| error.to_string())?;
        check!(!first_path.exists(), "cleanup must remove the directory");
        let second_path = second.path().to_path_buf();
        drop(second);
        check!(!second_path.exists(), "drop must remove the directory");
        Ok(())
    }

    /// A failed explicit cleanup is returned rather than panicking in the guard.
    #[test]
    fn cleanup_reports_a_missing_directory() -> Result<(), String> {
        let mut scratch = Scratch::try_new("missing").map_err(|error| error.to_string())?;
        ambient::remove_dir(scratch.path()).map_err(|error| error.to_string())?;
        check!(
            scratch.cleanup().is_err(),
            "missing directory must report a cleanup error"
        );
        drop(scratch);
        Ok(())
    }
}
