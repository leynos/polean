//! Proofs for the admission recognizer's specification.
//!
//! Each lemma lifts one step of the kernel's scan to the specification: a
//! whitespace byte or a whole comment extends a trivia gap, and consuming bytes
//! that are not line feeds leaves the gap's line-feed record unchanged. The
//! kernel calls these in its loop invariants; nothing here is assumed.
//!
//! The lemmas are grouped by the scan step they describe, and this file
//! re-exports them so the kernel can import the proof surface as one module.

// The kernel pulls this file in with `#[path]`, which resolves each child
// module against `verus/` rather than against `verus/admission_lemmas/`, so
// the child paths are spelled out.
#[path = "admission_lemmas/baseline.rs"]
mod baseline;
#[path = "admission_lemmas/comments.rs"]
mod comments;
#[path = "admission_lemmas/gaps.rs"]
mod gaps;
#[path = "admission_lemmas/metadata.rs"]
mod metadata;
#[path = "admission_lemmas/permitted.rs"]
mod permitted;
#[path = "admission_lemmas/records.rs"]
mod records;

#[cfg(verus_keep_ghost)]
pub use self::baseline::*;
#[cfg(verus_keep_ghost)]
pub use self::comments::*;
#[cfg(verus_keep_ghost)]
pub use self::gaps::*;
#[cfg(verus_keep_ghost)]
pub use self::metadata::*;
#[cfg(verus_keep_ghost)]
pub use self::permitted::*;
#[cfg(verus_keep_ghost)]
pub use self::records::*;
