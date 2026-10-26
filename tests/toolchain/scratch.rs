//! Isolated, automatically cleaned scratch directories for toolchain tests.

use std::{
    fs,
    io::ErrorKind,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

/// Distinguishes directories created by concurrent tests in this process.
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// A uniquely created directory beneath Cargo's test scratch directory.
pub struct Scratch(PathBuf);

impl Scratch {
    /// Create a scratch directory without reusing a leftover or concurrent one.
    pub fn new(name: &str) -> Self {
        loop {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let path = Path::new(env!("CARGO_TARGET_TMPDIR"))
                .join(format!("{name}-{}-{id}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => (),
                Err(error) => panic!(
                    "cannot create scratch directory {}: {error}",
                    path.display()
                ),
            }
        }
    }

    /// The directory owned by this guard.
    pub fn path(&self) -> &Path { &self.0 }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // A permission test may unwind while its root is inaccessible.
        let result = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755))
            .and_then(|()| fs::remove_dir_all(&self.0));
        if let Err(error) = result
            && !std::thread::panicking()
        {
            panic!(
                "cannot remove scratch directory {}: {error}",
                self.0.display()
            );
        }
    }
}
