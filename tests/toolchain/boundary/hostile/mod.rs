//! The hostile child's entry point, selected by an argument token.
//!
//! One ignored test is the entry point every hostile behaviour runs through:
//!
//! ```plaintext
//! <test binary> --exact --ignored <module path>::entry \
//!     polean-hostile=<behaviour>:<nonce> --test-threads=1 --nocapture -q
//! ```
//!
//! libtest treats the extra token as another filter, which matches no test, and
//! the entry reads it through [`std::env::args`]. Without a token the entry does
//! nothing, so an accidental `--ignored` run of the whole binary starts no
//! hostile behaviour; the supervisor requires the child's start line carrying
//! the run's nonce, so a child that never ran cannot pass.
//!
//! # Why an argument token, not a file
//!
//! A file would be another input the boundary must bind, and the negative
//! control without bubblewrap has no allowlist to place it in. A token travels
//! with the process, cannot be planted by another local user, and is inert in
//! every other context.
//!
//! # Behaviour families
//!
//! Each family is one module and reports every attempt it makes as a line of its
//! own, so a probe can require positive evidence per attempt rather than one
//! opaque exit status. The families are finite by construction: no fork bomb,
//! no unbounded allocation, and every path is supplied by the probe.

use std::{io::Write as _, path::PathBuf, thread, time::Duration};

mod canary;
mod escapee;
mod memory;
mod offline;
mod output;
mod storage;

/// The module path the entry point is reached through.
pub const MODULE: &str = module_path!();

/// The argument-token prefix.
pub const TOKEN_PREFIX: &str = "polean-hostile=";

/// One requested hostile behaviour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Behaviour {
    /// Report the security state the child observes from inside.
    Canary,
    /// Try to reach host listeners and count inherited descriptors.
    Offline,
    /// Exceed the memory limit, or fill the scratch area.
    Memory,
    /// Flood standard output past its cap.
    Output,
    /// Start a grandchild in its own process group and sleep.
    Escapee,
    /// Write outside the scratch area.
    Storage,
}

impl Behaviour {
    /// Every behaviour, in a fixed order.
    pub const ALL: [Self; 6] = [
        Self::Canary,
        Self::Offline,
        Self::Memory,
        Self::Output,
        Self::Escapee,
        Self::Storage,
    ];

    /// The behaviour's token name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Canary => "canary",
            Self::Offline => "offline",
            Self::Memory => "memory",
            Self::Output => "output",
            Self::Escapee => "escapee",
            Self::Storage => "storage",
        }
    }

    /// Parse a behaviour's token name.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|behaviour| behaviour.as_str() == name)
    }
}

/// One parsed hostile request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    /// The behaviour to run.
    pub behaviour: Behaviour,
    /// The run's nonce, which every report line must carry.
    pub nonce: String,
}

/// Parse the one hostile token from an argument list.
///
/// A list with no token is inert and returns `Ok(None)`. Two tokens, or a token
/// whose behaviour or nonce is malformed, is an error rather than a silent
/// no-op: the supervisor built that argument list, so a malformed token is a
/// defect in the probe, and a child that quietly did nothing would look like a
/// failure of the boundary.
///
/// # Errors
///
/// Returns a diagnostic naming the malformed token.
pub fn request(args: &[String]) -> Result<Option<Request>, String> {
    let mut tokens = args
        .iter()
        .filter_map(|argument| argument.strip_prefix(TOKEN_PREFIX));
    let Some(token) = tokens.next() else {
        return Ok(None);
    };
    if let Some(extra) = tokens.next() {
        return Err(format!(
            "more than one hostile token was supplied; the second is {TOKEN_PREFIX}{extra}"
        ));
    }
    parse_token(token).map(Some)
}

/// Parse `<behaviour>:<nonce>`.
fn parse_token(token: &str) -> Result<Request, String> {
    let Some((behaviour, nonce)) = token.split_once(':') else {
        return Err(format!(
            "the hostile token {token:?} must be <behaviour>:<nonce>"
        ));
    };
    let Some(parsed) = Behaviour::parse(behaviour) else {
        return Err(format!(
            "the hostile behaviour {behaviour:?} is not one of {}",
            names()
        ));
    };
    validate_nonce(nonce)?;
    Ok(Request {
        behaviour: parsed,
        nonce: nonce.to_owned(),
    })
}

/// Validate the nonce: non-empty, ASCII letters, digits, `-`, and `_`.
///
/// # Errors
///
/// Returns a diagnostic for an empty or out-of-alphabet nonce. The nonce ends up
/// in a unit name, so a character systemd could not accept is refused here.
pub fn validate_nonce(nonce: &str) -> Result<(), String> {
    if nonce.is_empty() {
        return Err("the hostile nonce is empty".to_owned());
    }
    if nonce.len() > 64 {
        return Err("the hostile nonce is longer than 64 bytes".to_owned());
    }
    if !nonce
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        return Err(format!(
            "the hostile nonce {nonce:?} must use only ASCII letters, digits, `-`, and `_`"
        ));
    }
    Ok(())
}

/// The comma-separated behaviour names, for diagnostics.
#[must_use]
pub fn names() -> String {
    let mut names = String::new();
    for behaviour in Behaviour::ALL {
        if !names.is_empty() {
            names.push_str(", ");
        }
        names.push_str(behaviour.as_str());
    }
    names
}

/// The exact libtest filter that selects the entry point.
///
/// libtest names each test by its module path inside the test binary, without
/// the crate name `module_path!()` carries. The filter must match that name
/// exactly, because the supervisor passes `--exact`.
#[must_use]
pub fn entry_name() -> String {
    match MODULE.split_once("::") {
        Some((_crate_name, path)) => format!("{path}::entry"),
        None => format!("{MODULE}::entry"),
    }
}

/// The prefix of every report line.
pub const REPORT_PREFIX: &str = "polean-hostile";

/// The prefix of a probe-supplied target argument.
pub const TARGET_PREFIX: &str = "polean-target=";

/// How long the child stays alive after its start line.
///
/// `systemd-run --scope` creates the scope before it executes the command, so a
/// job is always inside its scope when it starts. A job that finishes in a few
/// milliseconds can, however, be gone before the supervisor's first successful
/// poll, which would leave the supervisor unable to confirm the containment it
/// must report on. This window, far longer than the supervisor's poll interval,
/// makes the observation deterministic without changing what the job does.
pub const OBSERVATION_WINDOW: Duration = Duration::from_millis(250);

/// Host inputs a behaviour may need, supplied by the probe as arguments.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Targets {
    /// A loopback TCP address on the host to try to reach.
    pub tcp: Option<String>,
    /// An abstract Unix socket name on the host to try to reach.
    pub abstract_name: Option<String>,
    /// A Unix socket file the probe created to try to reach.
    pub socket: Option<PathBuf>,
    /// A host directory path to try to write into.
    pub host: Option<PathBuf>,
    /// The memory behaviour's mode: `allocate` or `fill`.
    pub memory: Option<String>,
    /// The run's nonce, used to name a marker the host must not keep.
    pub marker: Option<String>,
}

/// Parse every target token from an argument list.
///
/// An argument that is not a target token is ignored, because libtest's own
/// arguments share the list. A target token that cannot be parsed is an error:
/// the supervisor built the list, so a malformed target is a probe defect and a
/// silently ignored one would make a probe pass vacuously.
///
/// # Errors
///
/// Returns a diagnostic naming the malformed token.
pub fn targets(args: &[String]) -> Result<Targets, String> {
    let mut targets = Targets::default();
    for argument in args {
        let Some(token) = argument.strip_prefix(TARGET_PREFIX) else {
            continue;
        };
        let Some((kind, value)) = token.split_once(':') else {
            return Err(format!("the target token {token:?} must be <kind>:<value>"));
        };
        match kind {
            "tcp" => targets.tcp = Some(value.to_owned()),
            "abstract" => targets.abstract_name = Some(value.to_owned()),
            "socket" => targets.socket = Some(PathBuf::from(value)),
            "host" => targets.host = Some(PathBuf::from(value)),
            "memory" => targets.memory = Some(value.to_owned()),
            "marker" => targets.marker = Some(value.to_owned()),
            other => {
                return Err(format!(
                    "the target kind {other:?} is not one of tcp, abstract, socket, host, memory, \
                     marker"
                ));
            }
        }
    }
    Ok(targets)
}

/// Write one report line to standard output.
///
/// Writing directly avoids the printing macros the repository denies, and a
/// closed pipe ends the report rather than panicking: the supervisor may already
/// have killed this process.
pub fn report(text: &str) {
    let mut out = std::io::stdout().lock();
    let mut line = String::from(REPORT_PREFIX);
    line.push(' ');
    line.push_str(text);
    line.push('\n');
    if out.write_all(line.as_bytes()).is_err() {
        return;
    }
    drop(out.flush());
}

/// Report one attempt and its observation.
pub fn attempt(name: &str, detail: &str) { report(&format!("attempt {name} {detail}")); }

/// Run the behaviour the request names.
///
/// # Errors
///
/// Returns a diagnostic for a failure the behaviour could not report as an
/// observation.
pub fn run(request: &Request, targets: &Targets) -> Result<(), String> {
    match request.behaviour {
        Behaviour::Canary => canary::run(),
        Behaviour::Offline => offline::run(targets),
        Behaviour::Memory => memory::run(targets),
        Behaviour::Output => output::run(),
        Behaviour::Escapee => escapee::run(),
        Behaviour::Storage => storage::run(targets),
    }
}

/// The hostile child's entry point.
///
/// The test runs only when the supervisor passes the exact libtest filter and a
/// behaviour token. It is `#[ignore]`d so `make test` never selects it, and it
/// returns immediately when the token is absent.
#[test]
#[ignore = "run only by the execution-boundary probe"]
fn entry() {
    let args: Vec<String> = std::env::args().collect();
    let Ok(Some(parsed)) = request(&args) else {
        return;
    };
    let targets = match targets(&args) {
        Ok(targets) => targets,
        Err(reason) => {
            report(&format!("failed {reason}"));
            return;
        }
    };
    report(&format!("start {}", parsed.nonce));
    thread::sleep(OBSERVATION_WINDOW);
    match run(&parsed, &targets) {
        Ok(()) => report("done"),
        Err(reason) => report(&format!("failed {reason}")),
    }
}
