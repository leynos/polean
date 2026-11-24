//! Bounded ingestion of a policy source.
//!
//! A [`PolicySource`] is the exact bytes of a committed policy file, checked
//! only for the properties ingestion owns: it fits the profile's size bound, it
//! is well-formed UTF-8, and it holds no NUL byte. Ingestion establishes
//! nothing about whether the bytes belong to the profile; that is admission in
//! roadmap 2.1.2, and it consumes this type rather than re-reading the stream.
//!
//! Bytes are preserved exactly. A byte-order mark is kept, line endings are
//! kept, and no normalisation of any kind runs, so a hash of these bytes is a
//! hash of the source that was read. Rejecting NUL is the one filtering
//! decision, and it removes nothing: a source that holds NUL is refused rather
//! than having it stripped.

#[cfg(test)]
mod tests;

use std::{fmt, io::Read, str};

use crate::{
    bounded::{self, BoundedReadError},
    profile::SOURCE_BYTES_MAX,
};

/// The bytes of one policy source, checked against the ingestion bounds.
///
/// A value is obtained from [`PolicySource::from_bytes`] or
/// [`PolicySource::read_from`] and cannot be constructed any other way, so the
/// three checks ingestion performs hold for every value. The type does not
/// implement [`serde::Deserialize`], and implementing it would be the only way
/// to bypass those checks.
///
/// # Examples
///
/// ```
/// use polean::source::PolicySource;
///
/// let source =
///     PolicySource::from_bytes(b"package authz\n".to_vec()).expect("the bytes fit the profile");
/// assert_eq!(
///     source.as_str(),
///     "package authz\n",
///     "the bytes are unchanged"
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicySource(String);

impl PolicySource {
    /// Take ownership of `bytes` when they satisfy the ingestion bounds.
    ///
    /// # Errors
    ///
    /// Returns [`SourceError::TooLarge`] when `bytes` exceeds
    /// [`SOURCE_BYTES_MAX`], [`SourceError::InvalidUtf8`] when it is not
    /// well-formed UTF-8, and [`SourceError::Nul`] when it contains a NUL byte.
    /// The checks run in that order, so a source that is both too large and
    /// malformed reports the size fault and is never decoded.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, SourceError> {
        if bytes.len() > SOURCE_BYTES_MAX {
            return Err(SourceError::TooLarge {
                limit: SOURCE_BYTES_MAX,
            });
        }
        // Owned bytes mean the check can hand back the `String` it validated
        // rather than re-validating the same bytes to build one.
        let text = String::from_utf8(bytes).map_err(|error| SourceError::InvalidUtf8 {
            valid_up_to: error.utf8_error().valid_up_to(),
        })?;
        text.find('\0')
            .map_or(Ok(Self(text)), |offset| Err(SourceError::Nul { offset }))
    }

    /// Read a source from `reader`, consuming at most [`SOURCE_BYTES_MAX`] + 1
    /// bytes.
    ///
    /// # Errors
    ///
    /// Returns [`SourceError::Read`] when the reader fails, even after some
    /// bytes have arrived. Otherwise it returns what [`PolicySource::from_bytes`]
    /// would for the bytes read.
    ///
    /// The caller owns opening the file or stream. A buffered reader beneath
    /// this function may read further ahead into its own buffer; the bound
    /// applies to the bytes this function requests. See
    /// [`crate::bounded::read_bounded`] for the reader contract.
    pub fn read_from(mut reader: impl Read) -> Result<Self, SourceError> {
        let bytes = bounded::read_bounded(&mut reader, SOURCE_BYTES_MAX).map_err(map_read)?;
        Self::from_bytes(bytes)
    }

    /// The source bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] { self.0.as_bytes() }

    /// The source as text, which is what a frontend consumes.
    #[must_use]
    pub fn as_str(&self) -> &str { &self.0 }

    /// The source as an owned string.
    #[must_use]
    pub fn into_string(self) -> String { self.0 }
}

/// Translate a bounded-read failure into the ingestion error.
///
/// The two error types stay separate because [`bounded::read_bounded`] serves
/// request decoding as well, but a caller of ingestion sees one closed set.
fn map_read(error: BoundedReadError) -> SourceError {
    match error {
        BoundedReadError::TooLarge { limit } => SourceError::TooLarge { limit },
        BoundedReadError::Read(inner) => SourceError::Read(inner),
    }
}

/// Why bytes are not a policy source.
///
/// The variants name the fault and its offset; none holds text from the input,
/// so a diagnostic cannot echo attacker-chosen content.
#[derive(Debug)]
pub enum SourceError {
    /// The source holds more bytes than the profile allows.
    TooLarge {
        /// The inclusive limit, which is [`SOURCE_BYTES_MAX`].
        limit: usize,
    },
    /// The source is not well-formed UTF-8.
    InvalidUtf8 {
        /// The length of the longest prefix that is valid UTF-8, which is
        /// where the first invalid byte starts.
        valid_up_to: usize,
    },
    /// The source holds a NUL byte.
    Nul {
        /// The offset of the first NUL byte.
        offset: usize,
    },
    /// The reader failed.
    Read(std::io::Error),
}

impl fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { limit } => {
                write!(
                    formatter,
                    "the policy source exceeds the {limit}-byte limit"
                )
            }
            Self::InvalidUtf8 { valid_up_to } => {
                write!(
                    formatter,
                    "the policy source is not valid UTF-8: the first invalid byte is at offset \
                     {valid_up_to}"
                )
            }
            Self::Nul { offset } => {
                write!(
                    formatter,
                    "the policy source holds a NUL byte at offset {offset}"
                )
            }
            Self::Read(_) => write!(formatter, "the policy source could not be read"),
        }
    }
}

impl std::error::Error for SourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TooLarge { .. } | Self::InvalidUtf8 { .. } | Self::Nul { .. } => None,
            Self::Read(error) => Some(error),
        }
    }
}
