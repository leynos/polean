//! Fail-closed submission: no job starts without a ready platform verdict.
//!
//! AC6 says a missing required program, user manager, delegated controller, or
//! canary state makes the supervisor return `Unavailable`, naming every missing
//! control, and start no job. A caller that checks the verdict and then calls a
//! spawn function can forget the check; this module makes the refusal and the
//! spawn the same decision, so a missing control cannot be turned into a
//! started job.

use super::{
    outcome::{MissingControl, Verdict},
    platform::PlatformReport,
};

/// The result of one attempted submission.
#[derive(Debug)]
pub enum Submitted<T> {
    /// The platform was ready and the job ran.
    Ran(T),
    /// At least one control was missing, so nothing started.
    Refused(Vec<MissingControl>),
}

impl<T> Submitted<T> {
    /// Whether the submission was refused.
    #[must_use]
    pub const fn is_refused(&self) -> bool { matches!(self, Self::Refused(_)) }

    /// Require a started job, turning a refusal into a diagnostic.
    ///
    /// A probe that claims an enforcement property calls this after `submit`:
    /// a refusal means the host failed a control, so the property was not
    /// observed and the probe must not read as a pass.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic naming every missing control when refused.
    pub fn require(self, context: &str) -> Result<T, String> {
        match self {
            Self::Ran(value) => Ok(value),
            Self::Refused(missing) => Err(format!(
                "{context}: the platform refused the job and started nothing: {missing:?}"
            )),
        }
    }
}

/// Run `spawn` only when `report` is ready.
///
/// A refusal returns every missing control and never calls `spawn`, so the
/// caller cannot start a job on a host that failed a control.
///
/// # Errors
///
/// Returns the spawn error when the platform is ready and spawning fails.
pub fn submit<T>(
    report: &PlatformReport,
    spawn: impl FnOnce() -> Result<T, String>,
) -> Result<Submitted<T>, String> {
    match report.verdict() {
        Verdict::Ready => spawn().map(Submitted::Ran),
        _ => Ok(Submitted::Refused(report.missing())),
    }
}
