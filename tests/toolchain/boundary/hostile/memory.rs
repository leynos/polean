//! The memory behaviour: exceed the limit, or fill the scratch area.
//!
//! Both modes are finite. The allocation stops at 256 MiB because the probe's
//! limit is 64 MiB, and the scratch fill stops at 256 MiB because a tmpfs that
//! does not refuse it is itself a finding. Neither can exhaust the host under
//! the negative control, which raises the limit above the attempt.

use std::io::Write as _;

use super::{Targets, attempt};
use crate::ambient;

/// The allocation target, in bytes.
pub const ALLOCATION_BYTES: usize = 256 * 1024 * 1024;

/// The allocation block, in bytes; the loop only adds, so no division is used.
const BLOCK_BYTES: usize = 8 * 1024 * 1024;

/// The fill block, in bytes.
const FILL_BYTES: usize = 1024 * 1024;

/// The most the scratch fill writes before calling the tmpfs unbounded.
const FILL_LIMIT: usize = 256 * 1024 * 1024;

/// The private scratch area inside the boundary.
const SCRATCH: &str = "/job/work/fill";

/// Run the mode named by the target, defaulting to the allocation.
///
/// # Errors
///
/// Returns only a diagnostic that cannot be reported as an observation.
pub fn run(targets: &Targets) -> Result<(), String> {
    if targets.memory.as_deref() == Some("fill") {
        fill()
    } else {
        allocate();
        Ok(())
    }
}

/// Allocate and touch [`ALLOCATION_BYTES`], reporting when it returns.
pub fn allocate() {
    // The start marker makes the kill attributable to this attempt: an
    // out-of-memory kill after the start line but before this marker cannot be
    // read as the allocation's own.
    attempt("memory", "starting allocation");
    let mut blocks: Vec<Vec<u8>> = Vec::new();
    let mut total = 0usize;
    while total < ALLOCATION_BYTES {
        blocks.push(vec![0x7f; BLOCK_BYTES]);
        total += BLOCK_BYTES;
    }
    attempt("memory", &format!("allocated {total}"));
}

/// Fill the scratch area until the filesystem refuses, reporting where.
fn fill() -> Result<(), String> {
    attempt("scratch", "starting fill");
    let mut file = ambient::create(std::path::Path::new(SCRATCH))
        .map_err(|error| format!("cannot create {SCRATCH}: {error}"))?;
    let block = vec![b'x'; FILL_BYTES];
    let mut total = 0usize;
    while total < FILL_LIMIT {
        match file.write_all(&block) {
            Ok(()) => total += FILL_BYTES,
            Err(error) => {
                attempt(
                    "scratch",
                    &format!("error after {total} bytes: {error} ({:?})", error.kind()),
                );
                return Ok(());
            }
        }
    }
    attempt("scratch", &format!("wrote {total} bytes without a bound"));
    Ok(())
}
