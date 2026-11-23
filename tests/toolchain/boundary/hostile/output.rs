//! The output behaviour: flood standard output past its cap.
//!
//! The flood is 2 MiB, which is twice the probe's 1 MiB stdout cap and small
//! enough that the negative control, which raises the cap, keeps it whole. The
//! child never checks whether its write was refused: the supervisor's cap is
//! what stops it, by killing the job once the cap trips.

use std::io::Write as _;

use super::attempt;

/// How much the flood writes.
pub const FLOOD_BYTES: usize = 2 * 1024 * 1024;

/// The block each write uses.
const BLOCK_BYTES: usize = 64 * 1024;

/// The byte the flood writes, so the kept prefix can be compared.
pub const FLOOD_BYTE: u8 = b'x';

/// Write [`FLOOD_BYTES`] to standard output and report how far it got.
///
/// # Errors
///
/// Returns only a diagnostic that cannot be reported as an observation.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every behaviour shares the dispatch signature so a failure is reported the same way"
)]
pub fn run() -> Result<(), String> {
    let block = vec![FLOOD_BYTE; BLOCK_BYTES];
    let mut out = std::io::stdout().lock();
    let mut written = 0usize;
    while written < FLOOD_BYTES {
        match out.write_all(&block) {
            Ok(()) => written += BLOCK_BYTES,
            Err(error) => {
                attempt("stdout", &format!("error after {written} bytes: {error}"));
                return Ok(());
            }
        }
    }
    if let Err(error) = out.write_all(b"\n") {
        attempt("stdout", &format!("error after {written} bytes: {error}"));
        return Ok(());
    }
    if let Err(error) = out.flush() {
        attempt("stdout", &format!("error on flush: {error}"));
        return Ok(());
    }
    attempt("stdout", &format!("wrote {written} bytes"));
    Ok(())
}
