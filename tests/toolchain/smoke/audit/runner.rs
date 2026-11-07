//! Build, compile, and run the audit corpus in an isolated scratch tree.
//!
//! The worker is built with the pinned Lake, the fixtures are compiled with the
//! pinned `lean`, and the worker runs with an empty environment apart from
//! `LEAN_SYSROOT` and `LEAN_PATH`. Every process is observed through its exit
//! status and captured streams, so a failure names the command and both streams
//! rather than only that something went wrong.
//!
//! The fixtures stay outside the Lake package: `--root` points at
//! `examples/audit/lean`, and the `.olean` files go to a scratch directory, so
//! the fixtures never change the committed tree. The worker build is the one
//! exception: `lake build` writes `.lake/build` inside the committed package,
//! which is ignored by Git. Tests run on parallel threads, so the build holds a
//! process-wide lock and two builds cannot share that directory at once.

use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::Mutex,
};

use crate::{
    ambient,
    scratch::Scratch,
    smoke::pinned_lean,
    support::{path_exists, repository_path},
};

/// One prepared corpus: the worker, the pinned Lean, and a scratch tree.
pub struct Corpus {
    /// The scratch directory, held so it is removed when the corpus is dropped.
    #[expect(
        dead_code,
        reason = "held only so the scratch directory is removed on drop"
    )]
    scratch: Scratch,
    /// Where the compiled fixtures are written.
    out: PathBuf,
    /// The pinned toolchain root.
    lean_root: PathBuf,
    /// The pinned `lean` binary.
    lean: PathBuf,
    /// The built audit worker.
    worker: PathBuf,
    /// The repository root, used as the working directory.
    repository: PathBuf,
    /// The fixture source root, passed to `lean --root`.
    fixtures: PathBuf,
}

impl Corpus {
    /// Build the worker and compile `sources`, in the order given.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the pinned tools are missing, the worker does
    /// not build, the scratch tree cannot be created, or a fixture does not
    /// compile.
    pub fn prepare(sources: &[&str]) -> Result<Self, String> {
        let repository = repository_path(".");
        let (lean_root, _pin) = pinned_lean()?;
        let lean = lean_root.join("bin").join("lean");
        let lake = lean_root.join("bin").join("lake");
        let package = repository.join("lean");
        build_worker(&lake, &package, &repository)?;
        let worker = package.join(".lake/build/bin/polean-audit");
        if !path_exists(&worker)? {
            return Err(format!(
                "the build reported success but {} is absent; the target did not build",
                worker.display()
            ));
        }
        let scratch = Scratch::try_new("audit-corpus")
            .map_err(|error| format!("cannot create the audit scratch directory: {error}"))?;
        let out = scratch.path().join("out");
        ambient::create_dir_all(&out.join("AuditFixture"))
            .map_err(|error| format!("cannot create {}: {error}", out.display()))?;
        let fixtures = repository.join("examples/audit/lean");
        let corpus = Self {
            scratch,
            out,
            lean_root,
            lean,
            worker,
            repository,
            fixtures,
        };
        for module in sources {
            corpus.compile(module)?;
        }
        Ok(corpus)
    }

    /// Compile one fixture module into the scratch tree.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic naming the module and both streams when `lean`
    /// fails.
    fn compile(&self, module: &str) -> Result<(), String> {
        let relative = module.replace('.', "/");
        let source = self.fixtures.join(format!("{relative}.lean"));
        let olean = self.out.join(format!("{relative}.olean"));
        let output = capture(
            &self.lean,
            &[
                "-o",
                &olean.display().to_string(),
                "--root",
                &self.fixtures.display().to_string(),
                &source.display().to_string(),
            ],
            &self.environment(),
            &self.repository,
        )?;
        require_success(&self.lean, &[module], &output)
    }

    /// Run the worker on `module` and its `roots`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the worker cannot be started. A non-zero exit
    /// is an observation the caller classifies, not an error here.
    pub fn worker(&self, module: &str, roots: &[String]) -> Result<Output, String> {
        let mut arguments = Vec::with_capacity(roots.len().saturating_add(1));
        arguments.push(module.to_owned());
        arguments.extend(roots.iter().cloned());
        let borrowed: Vec<&str> = arguments.iter().map(String::as_str).collect();
        capture(
            &self.worker,
            &borrowed,
            &self.environment(),
            &self.repository,
        )
    }

    /// Run the program that prints a non-report.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the process cannot be started.
    pub fn garbled(&self) -> Result<Output, String> { self.run_program("Garbled.lean") }

    /// Run the positive control that imports the sentinel fixture.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the process cannot be started.
    pub fn sentinel_probe(&self) -> Result<Output, String> {
        self.run_program("SentinelProbe.lean")
    }

    /// Whether the sentinel the fixture's initializer writes exists.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the path cannot be inspected.
    pub fn sentinel_present(&self) -> Result<bool, String> {
        path_exists(&self.out.join("audit-sentinel"))
    }

    /// Run one Lean program file with `--run`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic when the process cannot be started.
    fn run_program(&self, file: &str) -> Result<Output, String> {
        let program = self.fixtures.join(file);
        capture(
            &self.lean,
            &["--run", &program.display().to_string()],
            &self.environment(),
            &self.repository,
        )
    }

    /// The environment every pinned process runs with.
    #[must_use]
    fn environment(&self) -> [(&'static str, &Path); 2] {
        [
            ("LEAN_SYSROOT", self.lean_root.as_path()),
            ("LEAN_PATH", self.out.as_path()),
        ]
    }
}

/// Serialises `lake build`, which writes `.lake/build` in the package.
static BUILD_LOCK: Mutex<()> = Mutex::new(());

/// Build the audit worker with the pinned Lake.
///
/// # Errors
///
/// Returns a diagnostic naming the package and both streams when the build
/// fails, or when another test panicked while holding the build lock.
fn build_worker(lake: &Path, package: &Path, repository: &Path) -> Result<(), String> {
    let _guard = BUILD_LOCK
        .lock()
        .map_err(|_| "the audit worker build lock is poisoned".to_owned())?;
    let package_text = package.display().to_string();
    let arguments = ["--dir", package_text.as_str(), "build", "polean-audit"];
    let output = capture(lake, &arguments, &[], repository)?;
    require_success(lake, &arguments, &output)
}

/// Run a process with an empty environment plus `environment`.
///
/// # Errors
///
/// Returns a diagnostic when the process cannot be started.
fn capture(
    program: &Path,
    arguments: &[&str],
    environment: &[(&str, &Path)],
    cwd: &Path,
) -> Result<Output, String> {
    Command::new(program)
        .args(arguments)
        .current_dir(cwd)
        .env_clear()
        .envs(environment.iter().copied())
        .output()
        .map_err(|error| format!("cannot run {}: {error}", program.display()))
}

/// Require `output` to be a successful exit, or report both streams.
///
/// # Errors
///
/// Returns a diagnostic naming the command, its exit code, and both streams.
fn require_success(program: &Path, arguments: &[&str], output: &Output) -> Result<(), String> {
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "{} {} failed with {:?}\nstdout:\n{}\nstderr:\n{}",
        program.display(),
        arguments.join(" "),
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}
