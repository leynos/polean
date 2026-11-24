//! A size-capped read that never trusts a length hint.
//!
//! Ingestion and request decoding both need the same small guarantee: read at
//! most a fixed number of bytes from a caller-supplied [`Read`] and report
//! whether the input fitted. The cap is enforced by reading one byte past it,
//! which is the least work that tells "exactly at the limit" from "over it"
//! without believing a `Read` implementation about how much it will produce.
//!
//! The module is public because roadmap 2.1.3 decodes requests through it. It
//! holds no policy knowledge, so widening its use cannot widen the profile.

#[cfg(test)]
mod tests;

use std::{
    fmt,
    io::{self, Read},
};

/// Why a bounded read did not return the input.
#[derive(Debug)]
pub enum BoundedReadError {
    /// The input holds more bytes than the limit allows.
    ///
    /// `limit` is the bound the caller asked for, not the number of bytes the
    /// reader offered: the read stops as soon as the bound is exceeded, so the
    /// true length of the input is not known and is not reported.
    TooLarge {
        /// The inclusive limit the caller asked for.
        limit: usize,
    },
    /// The reader failed.
    ///
    /// Any bytes that had already arrived are discarded. A failure after a
    /// partial read is an error, never a truncated success.
    Read(io::Error),
}

impl fmt::Display for BoundedReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { limit } => {
                write!(formatter, "the input exceeds the {limit}-byte limit")
            }
            // The reader's own message is not quoted: a caller-supplied reader
            // may word it from attacker-chosen data, and a diagnostic must not
            // become a channel for it. `Error::source` still carries the error
            // for a caller that inspects the kind.
            Self::Read(_) => write!(formatter, "the input could not be read"),
        }
    }
}

impl std::error::Error for BoundedReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TooLarge { .. } => None,
            Self::Read(error) => Some(error),
        }
    }
}

/// Read at most `limit` bytes, or report that the input was larger.
///
/// The limit is inclusive: an input of exactly `limit` bytes is returned
/// unchanged, and one byte more is [`BoundedReadError::TooLarge`]. At most
/// `limit + 1` bytes are ever requested from `reader`, so an unbounded stream
/// is refused rather than consumed.
///
/// # Errors
///
/// Returns [`BoundedReadError::TooLarge`] when the input does not fit, and
/// [`BoundedReadError::Read`] when the reader fails.
///
/// # Reader contract
///
/// The guarantees above are guarantees at this interface, as
/// [`Read::read`] defines it. A buffered reader beneath it may read further
/// ahead into its own buffer; that is not consumption by this function.
///
/// - `Ok(0)` means end of stream and is never retried.
/// - [`io::ErrorKind::Interrupted`] is retried, as [`Read::read_to_end`] documents, so an
///   interrupted read is not a failure.
/// - A reader that reports having produced more bytes than the buffer it was given is outside the
///   contract [`Read`] states, and `std` panics on it.
///
/// # Examples
///
/// ```
/// use polean::bounded::read_bounded;
///
/// let small = read_bounded(b"abc".as_slice(), 3).expect("three bytes fit");
/// assert_eq!(small, b"abc", "the bytes are returned unchanged");
///
/// assert!(
///     read_bounded(b"abcd".as_slice(), 3).is_err(),
///     "one byte too many"
/// );
/// ```
pub fn read_bounded(reader: impl Read, limit: usize) -> Result<Vec<u8>, BoundedReadError> {
    // `usize` is at most 64 bits on every supported target, so the conversion
    // is exact; the fallback keeps a hypothetical wider `usize` from wrapping.
    let allowance = u64::try_from(limit.saturating_add(1)).unwrap_or(u64::MAX);
    let mut buffer = Vec::new();
    reader
        .take(allowance)
        .read_to_end(&mut buffer)
        .map_err(BoundedReadError::Read)?;
    if buffer.len() > limit {
        return Err(BoundedReadError::TooLarge { limit });
    }
    Ok(buffer)
}
