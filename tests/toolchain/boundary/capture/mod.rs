//! The stream-capture kernel: keep exactly the first `cap` bytes of a stream.
//!
//! Every supervised job has two piped streams. The supervisor reads each on its
//! own thread and keeps a bounded prefix, because a hostile child may write
//! without end. This kernel is that bound, and it is the one limit the
//! supervisor enforces itself rather than delegating to the kernel or systemd.
//!
//! # Contract
//!
//! ```plaintext
//! Capture::new(cap)          -> a capture holding no bytes and not tripped
//! feed(capture, chunk)       -> how many bytes of the chunk were kept, and
//!                               whether this call crossed the cap
//!   requires  the capture holds at most `cap` bytes
//!   ensures   it still does, the kept bytes grew by the chunk's first
//!             `keep_count` bytes, and `tripped` is the disjunction of its old
//!             value with "this chunk did not fit whole"
//! ```
//!
//! The whole-stream property follows by induction over the chunk sequence and
//! is stated in [`spec`]: after any finite stream, the kept bytes are exactly
//! its first `min(cap, |stream|)` bytes and `tripped` holds exactly when the
//! stream is longer than the cap. That induction is what the Verus entry point
//! verifies; this file's `feed` is verified against one step of it.
//!
//! # Verification
//!
//! `verus/boundary_capture.rs` pulls this module into the Verus entry point
//! with `#[path]`, so the executable code and the verified code are one text.
//! The specification is in `verus/boundary_capture_spec.rs`, the induction in
//! `verus/boundary_capture_lemmas.rs`, and the reachability witnesses in
//! `verus/boundary_capture_witnesses.rs`. The proof establishes properties of
//! this kernel alone. It says nothing about the reader loop that calls it, and
//! nothing about whether a child respects the bound: a job that ignores the cap
//! is killed by [`super::supervisor`], not by this kernel.

use vstd::prelude::*;

// The specification, the lemmas, and the witnesses exist only for the
// verifier. Under plain `rustc` the `verus!` macro erases every item in them,
// so compiling the modules would leave their imports unused and `-D warnings`
// would fail the build. They are gated together for that reason, as the
// settlement decision module is.
#[cfg(verus_keep_ghost)]
#[path = "../../../../verus/boundary_capture_spec.rs"]
mod spec;

#[cfg(verus_keep_ghost)]
#[path = "../../../../verus/boundary_capture_lemmas.rs"]
mod lemmas;

#[cfg(verus_keep_ghost)]
#[path = "../../../../verus/boundary_capture_witnesses.rs"]
mod witnesses;

#[cfg(verus_keep_ghost)]
pub use self::{lemmas::*, spec::*, witnesses::*};

#[cfg(test)]
mod tests;

verus! {

/// The most bytes a capture keeps, and the bytes it has kept so far.
///
/// `cap` is fixed at construction. `kept` grows only while it is shorter than
/// `cap`, and `tripped` records that the stream has already exceeded the cap,
/// after which `kept` never changes again.
pub struct Capture {
    /// The fixed cap, set at construction.
    pub cap: usize,
    /// The stream prefix kept so far.
    pub kept: Vec<u8>,
    /// Whether the stream has already exceeded the cap.
    pub tripped: bool,
}

/// What one [`Capture::feed`] call did.
///
/// `tripped` is about this chunk, not about the first crossing: it says the
/// chunk did not fit whole below the cap, which is also true for every later
/// chunk once the cap is full. The capture's own [`Capture::tripped`] is the
/// latched flag, and the reader thread signals on that.
pub struct Fed {
    /// How many bytes of this chunk were kept.
    pub kept: usize,
    /// Whether this chunk did not fit whole below the cap.
    pub tripped: bool,
}

impl Capture {
    /// The state invariant: a capture never holds more than its cap.
    pub open spec fn well_formed(self) -> bool { self.kept@.len() <= self.cap as int }

    /// Start a capture with a fixed cap.
    #[must_use]
    pub fn new(cap: usize) -> (capture: Self)
        ensures
            capture.well_formed(),
            capture.cap == cap,
            capture.kept@.len() == 0,
            !capture.tripped,
    {
        Self {
            cap,
            kept: Vec::new(),
            tripped: false,
        }
    }

    /// The cap this capture enforces.
    #[must_use]
    pub fn cap(&self) -> (cap: usize)
        ensures cap == self.cap,
    {
        self.cap
    }

    /// The bytes kept so far, which are the stream's first kept bytes.
    #[must_use]
    pub fn kept(&self) -> (kept: &[u8])
        ensures kept@ == self.kept@,
    {
        self.kept.as_slice()
    }

    /// Whether the stream has already exceeded the cap.
    #[must_use]
    pub fn tripped(&self) -> (tripped: bool)
        ensures tripped == self.tripped,
    {
        self.tripped
    }

    /// Feed one chunk of the stream.
    ///
    /// The call keeps the chunk's first `keep_count` bytes, where `keep_count`
    /// is the smaller of the chunk's length and the room left below the cap,
    /// and trips when the chunk does not fit whole. A chunk that lands exactly
    /// on the cap is kept whole and does not trip: the cap is exceeded, not
    /// reached.
    #[expect(
        clippy::indexing_slicing,
        reason = "the loop guard bounds the index by the chunk length"
    )]
    pub fn feed(&mut self, chunk: &[u8]) -> (fed: Fed)
        requires old(self).well_formed()
        ensures
            final(self).well_formed(),
            final(self).cap == old(self).cap,
            fed.kept as int == keep_count(old(self).cap as nat, old(self).kept@, chunk@),
            fed.tripped == (chunk@.len() as int > room(old(self).cap as nat, old(self).kept@)),
            final(self).kept@ == old(self).kept@ + chunk@.subrange(0, fed.kept as int),
            final(self).tripped == (old(self).tripped || fed.tripped),
    {
        let remaining = self.cap - self.kept.len();
        let keep = if chunk.len() < remaining {
            chunk.len()
        } else {
            remaining
        };
        let tripped = chunk.len() > remaining;
        proof {
            assert(remaining as int == room(old(self).cap as nat, old(self).kept@));
        }
        let mut index: usize = 0;
        while index < keep
            invariant
                index <= keep,
                keep as int == keep_count(old(self).cap as nat, old(self).kept@, chunk@),
                self.kept@ == old(self).kept@ + chunk@.subrange(0, index as int),
                self.cap == old(self).cap,
                self.tripped == old(self).tripped,
                self.well_formed(),
                index <= chunk@.len(),
            decreases keep - index,
        {
            proof {
                assert(index < chunk.len());
            }
            self.kept.push(chunk[index]);
            index += 1;
        }
        self.tripped = self.tripped || tripped;
        Fed {
            kept: keep,
            tripped,
        }
    }
}

} // verus!
