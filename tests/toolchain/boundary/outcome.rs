//! The probe's outcome categories and the fail-closed platform verdict.
//!
//! Two finite decisions live here, and both are pure. [`classify`] turns one
//! job's observations into an outcome, and [`verdict`] turns the five
//! enforcement controls into `Ready` or `Unavailable`. Neither starts a process,
//! so their properties are enumerated completely by ordinary tests.
//!
//! These are the probe's observation categories, not the product's outcome
//! taxonomy. The ADR proposes a mapping from them onto `contracts/cli.txt`; it
//! does not add one to the contract.
//!
//! # Properties
//!
//! - `classify` never returns [`Outcome::Completed`] when a bound fired, cancellation was
//!   requested, or the control group was not empty.
//! - `classify` returns [`Outcome::ContainmentFailure`] whenever the control group was not empty
//!   after teardown, whatever else happened.
//! - `classify` lists every bound that fired, once, in a fixed order.
//! - `verdict` returns [`Verdict::Ready`] exactly when all five controls are available, and
//!   otherwise names every missing control.
//!
//! The inputs are a finite product, so the tests enumerate them; that is
//! complete for a finite domain and Verus would add ceremony without rigour.

/// One bound a job can exhaust.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Bound {
    /// The job exceeded its cgroup memory limit.
    Memory,
    /// The job wrote more than its standard-output cap.
    Stdout,
    /// The job wrote more than its standard-error cap.
    Stderr,
    /// The job's deadline expired.
    Deadline,
    /// The caller cancelled the job.
    Cancelled,
}

impl Bound {
    /// Every bound, in the order outcomes list them.
    pub const ALL: [Self; 5] = [
        Self::Memory,
        Self::Stdout,
        Self::Stderr,
        Self::Deadline,
        Self::Cancelled,
    ];

    /// The position of this bound in [`Bound::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Memory => 0,
            Self::Stdout => 1,
            Self::Stderr => 2,
            Self::Deadline => 3,
            Self::Cancelled => 4,
        }
    }

    /// The bound's stable diagnostic name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
            Self::Deadline => "deadline",
            Self::Cancelled => "cancelled",
        }
    }
}

/// A set of bounds that fired, in [`Bound::ALL`] order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bounds {
    /// Which bounds fired, indexed by [`Bound::index`].
    fired: [bool; 5],
}

impl Bounds {
    /// No bound fired.
    #[must_use]
    pub const fn none() -> Self { Self { fired: [false; 5] } }

    /// Whether no bound fired.
    #[must_use]
    pub const fn is_empty(self) -> bool { self.count() == 0 }

    /// Record that `bound` fired.
    #[expect(
        clippy::indexing_slicing,
        reason = "the index is a Bound discriminant below the array length"
    )]
    pub fn insert(&mut self, bound: Bound) { self.fired[bound.index()] = true; }

    /// Whether `bound` fired.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "the index is a Bound discriminant below the array length"
    )]
    pub const fn contains(self, bound: Bound) -> bool { self.fired[bound.index()] }

    /// How many bounds fired.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "the index is a Bound discriminant below the array length"
    )]
    pub const fn count(self) -> usize {
        let mut total = 0;
        let mut index = 0;
        while index < Self::COUNT {
            if self.fired[index] {
                total += 1;
            }
            index += 1;
        }
        total
    }

    /// Every bound that fired, in [`Bound::ALL`] order.
    #[must_use]
    pub fn listed(self) -> Vec<Bound> {
        Bound::ALL
            .into_iter()
            .filter(|bound| self.contains(*bound))
            .collect()
    }

    /// The number of bounds the set can hold.
    const COUNT: usize = 5;
}

/// How a job's directly spawned process ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitForm {
    /// The process exited with this status.
    Exited(i32),
    /// The process was killed by this signal number.
    Signalled(i32),
    /// The supervisor could not report how the process ended.
    Unreported,
}

/// What the supervisor observed about one job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    /// Which bounds fired.
    pub bounds: Bounds,
    /// How the direct child ended.
    pub exit: ExitForm,
    /// Whether the job's control group was empty after teardown.
    pub cgroup_empty: bool,
    /// Whether any process outlived the process-group kill and was removed only
    /// by the cgroup kill.
    pub survivors: bool,
    /// Missing enforcement controls, empty when the platform check was ready.
    pub unavailable: Vec<MissingControl>,
}

/// The probe's outcome categories.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A required control was missing or ineffective, so no job started.
    Unavailable(Vec<MissingControl>),
    /// The job's control group was not empty after teardown.
    ContainmentFailure(Vec<Bound>),
    /// The job was stopped by one or more bounds.
    BoundsExhausted(Vec<Bound>),
    /// No bound fired and the child failed of its own accord.
    ChildFailed(ExitForm),
    /// The child exited zero, no bound fired, and the control group is empty.
    Completed,
}

/// Classify one job's observations.
///
/// Precedence is `Unavailable`, then `ContainmentFailure`, then any bound, then
/// a child failure, then completion. A missing control outranks everything
/// because no job should have started; a surviving process outranks any bound
/// because it is the one failure that makes every other observation untrustworthy.
/// A process that outlived the process-group kill is a containment failure even
/// when the cgroup kill then removed it: the process-group cancellation the
/// boundary promises did not reach every process.
#[must_use]
pub fn classify(observation: &Observation) -> Outcome {
    if !observation.unavailable.is_empty() {
        return Outcome::Unavailable(observation.unavailable.clone());
    }
    let fired = observation.bounds.listed();
    if !observation.cgroup_empty || observation.survivors {
        return Outcome::ContainmentFailure(fired);
    }
    if !fired.is_empty() {
        return Outcome::BoundsExhausted(fired);
    }
    if observation.exit == ExitForm::Exited(0) {
        Outcome::Completed
    } else {
        Outcome::ChildFailed(observation.exit)
    }
}

/// One enforcement control the platform check requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ControlKind {
    /// The launcher programs exist, are executable, and bubblewrap is new enough.
    Programs,
    /// The systemd user manager answers through a cleared environment.
    UserManager,
    /// The user manager delegates the `memory` and `pids` controllers.
    Delegation,
    /// The canary reports the isolation the boundary promises.
    CanaryIsolation,
    /// The canary's scope reads back the limits the probe requested.
    CanaryLimits,
}

impl ControlKind {
    /// Every control, in the order the verdict lists them.
    pub const ALL: [Self; 5] = [
        Self::Programs,
        Self::UserManager,
        Self::Delegation,
        Self::CanaryIsolation,
        Self::CanaryLimits,
    ];

    /// The control's stable diagnostic name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Programs => "programs",
            Self::UserManager => "user-manager",
            Self::Delegation => "delegation",
            Self::CanaryIsolation => "canary-isolation",
            Self::CanaryLimits => "canary-limits",
        }
    }
}

/// Whether one control held, or why it did not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Availability {
    /// The control held.
    Available,
    /// The control did not hold, for this reason.
    Missing(String),
}

/// One missing control and the reason it is missing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingControl {
    /// Which control is missing.
    pub kind: ControlKind,
    /// What the check observed.
    pub reason: String,
}

/// The platform check's verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Every control held, so a job may start.
    Ready,
    /// At least one control is missing, so no job may start.
    Unavailable(Vec<MissingControl>),
    /// The check's own control list was malformed, so nothing may start.
    Malformed(String),
}

/// Decide the verdict from one availability per control.
///
/// The list must name every control exactly once, in [`ControlKind::ALL`] order.
/// A missing control is never degraded to a warning: any
/// [`Availability::Missing`] makes the verdict `Unavailable`, and every missing
/// control is named. A list that is short, long, reordered, or duplicated is
/// [`Verdict::Malformed`]: the check cannot say what it observed, so no job may
/// start.
#[must_use]
pub fn verdict(controls: &[(ControlKind, Availability)]) -> Verdict {
    if let Some(fault) = control_list_fault(controls) {
        return Verdict::Malformed(fault);
    }
    let missing: Vec<MissingControl> = controls
        .iter()
        .filter_map(|(kind, availability)| match availability {
            Availability::Available => None,
            Availability::Missing(reason) => Some(MissingControl {
                kind: *kind,
                reason: reason.clone(),
            }),
        })
        .collect();
    if missing.is_empty() {
        Verdict::Ready
    } else {
        Verdict::Unavailable(missing)
    }
}

/// Why a control list is not exactly the five controls in order, if it is not.
fn control_list_fault(controls: &[(ControlKind, Availability)]) -> Option<String> {
    if controls.len() != ControlKind::ALL.len() {
        return Some(format!(
            "the platform check reported {} controls, expected {}",
            controls.len(),
            ControlKind::ALL.len()
        ));
    }
    for (position, (kind, _availability)) in controls.iter().enumerate() {
        let expected = ControlKind::ALL.get(position);
        if expected != Some(kind) {
            return Some(format!(
                "control {position} is {} where {} was expected; the list is reordered or \
                 duplicated",
                kind.as_str(),
                expected.map_or("no control", |wanted| wanted.as_str())
            ));
        }
    }
    None
}
