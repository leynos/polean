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
//! # Milestone
//!
//! EP-M1 establishes the entry point and its token contract only. The behaviour
//! families (canary, offline, memory, output, escapee, storage) arrive with the
//! supervisor in EP-M2, so until then the entry parses its token and returns.
//! No probe runs before EP-M2.

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

/// The exact libtest path of the entry point.
#[must_use]
pub fn entry_name() -> String { format!("{MODULE}::entry") }

/// The hostile child's entry point.
///
/// The test runs only when the supervisor passes the exact libtest filter and a
/// behaviour token. It is `#[ignore]`d so `make test` never selects it, and it
/// returns immediately when the token is absent.
#[test]
#[ignore = "run only by the execution-boundary probe"]
fn entry() {
    let args: Vec<String> = std::env::args().collect();
    let Ok(Some(_request)) = request(&args) else {
        return;
    };
    // The behaviour families arrive with the supervisor in EP-M2. Until then the
    // entry point is inert: no probe runs, and a stray token starts nothing.
}
