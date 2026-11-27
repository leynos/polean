//! Ingestion's boundary, encoding, and chunking behaviour.
//!
//! Each row names the property it holds: the size bound is inclusive, a source
//! at the bound is accepted unchanged, UTF-8 and NUL faults carry the offset
//! the bytes actually have, and the result does not depend on how the reader
//! splits the stream.

use std::io::{self, Read};

use googletest::prelude::*;
use rstest::rstest;

use super::{PolicySource, SourceError};
use crate::profile::SOURCE_BYTES_MAX;

/// A source of exactly `len` valid UTF-8 bytes.
fn source_of_length(len: usize) -> Vec<u8> {
    let mut bytes = vec![b'a'; len];
    if let Some(last) = bytes.last_mut() {
        *last = b'\n';
    }
    bytes
}

/// A reader that yields `bytes` and then fails.
struct FailingAfter {
    /// The bytes still to hand out before the failure.
    remaining: Vec<u8>,
}

impl Read for FailingAfter {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.remaining.is_empty() {
            return Err(io::Error::other("the reader failed after a partial read"));
        }
        let take = buffer.len().min(self.remaining.len());
        buffer
            .get_mut(..take)
            .expect("the slice has the buffer's length")
            .copy_from_slice(
                self.remaining
                    .get(..take)
                    .expect("the slice has the taken length"),
            );
        self.remaining.drain(..take);
        Ok(take)
    }
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(SOURCE_BYTES_MAX - 1)]
#[case(SOURCE_BYTES_MAX)]
fn accepts_a_source_at_or_below_the_cap(#[case] length: usize) {
    let bytes = source_of_length(length);

    let source = PolicySource::from_bytes(bytes.clone()).expect("the bytes fit the profile");

    assert_eq!(source.as_bytes(), bytes.as_slice(), "bytes are preserved");
}

#[rstest]
#[case(SOURCE_BYTES_MAX + 1)]
fn rejects_a_source_over_the_cap(#[case] length: usize) {
    let bytes = source_of_length(length);

    let result = PolicySource::from_bytes(bytes);

    assert_that!(
        result,
        err(matches_pattern!(SourceError::TooLarge {
            limit: eq(&SOURCE_BYTES_MAX)
        }))
    );
}

#[test]
fn size_is_checked_before_encoding() {
    // A multi-byte character split by the cap must report the size fault, not
    // an encoding fault: an over-long source is never decoded.
    let mut bytes = source_of_length(SOURCE_BYTES_MAX);
    bytes.truncate(SOURCE_BYTES_MAX - 1);
    bytes.push(0xf0);
    bytes.push(0x9f);

    let result = PolicySource::from_bytes(bytes);

    assert_that!(result, err(matches_pattern!(SourceError::TooLarge { .. })));
}

#[rstest]
#[case(vec![0x80], 0)]
#[case(vec![b'a', 0x80], 1)]
#[case(vec![0xed, 0xa0, 0x80], 0)]
#[case(vec![0xf4, 0x90, 0x80, 0x80], 0)]
#[case(vec![b'a'; 10].into_iter().chain([0x80]).collect(), 10)]
#[case(vec![0xc0, 0xaf], 0)]
fn rejects_invalid_utf8_with_its_offset(#[case] bytes: Vec<u8>, #[case] offset: usize) {
    let result = PolicySource::from_bytes(bytes);

    assert_that!(
        result,
        err(matches_pattern!(SourceError::InvalidUtf8 {
            valid_up_to: eq(&offset)
        }))
    );
}

/// A source of `length` `a` bytes with a NUL written at `offset`.
fn source_with_nul(length: usize, offset: usize) -> Vec<u8> {
    let mut bytes = vec![b'a'; length];
    if let Some(slot) = bytes.get_mut(offset) {
        *slot = 0;
    }
    bytes
}

#[rstest]
#[case(1, 0)]
#[case(3, 2)]
#[case(SOURCE_BYTES_MAX, SOURCE_BYTES_MAX - 1)]
#[case(10, 5)]
fn rejects_nul_with_its_offset(#[case] length: usize, #[case] offset: usize) {
    let bytes = source_with_nul(length, offset);
    let result = PolicySource::from_bytes(bytes);

    assert_that!(
        result,
        err(matches_pattern!(SourceError::Nul {
            offset: eq(&offset)
        }))
    );
}

#[test]
fn an_encoding_fault_outranks_a_nul_fault() {
    // The invalid sequence precedes the NUL, so the encoding check runs first
    // and reports where the bytes stopped being UTF-8.
    let bytes = vec![b'a', 0x80, 0, b'b'];

    let result = PolicySource::from_bytes(bytes);

    assert_that!(
        result,
        err(matches_pattern!(SourceError::InvalidUtf8 {
            valid_up_to: eq(&1)
        }))
    );
}

#[rstest]
#[case(vec![0xef, 0xbb, 0xbf, b'a'])]
#[case(b"a\r\nb".to_vec())]
#[case(b"a\nb".to_vec())]
fn accepts_a_byte_order_mark_and_line_endings_unchanged(#[case] bytes: Vec<u8>) {
    let source = PolicySource::from_bytes(bytes.clone()).expect("the bytes are valid UTF-8");

    assert_eq!(source.as_bytes(), bytes.as_slice(), "bytes are preserved");
}

#[test]
fn an_unbounded_stream_is_refused_without_reading_on() {
    // `repeat` never ends, so a source that returned would prove the bound was
    // not applied to an endless stream.
    let result = PolicySource::read_from(io::repeat(b'a'));

    assert_that!(
        result,
        err(matches_pattern!(SourceError::TooLarge {
            limit: eq(&SOURCE_BYTES_MAX)
        }))
    );
}

#[rstest]
#[case(0)]
#[case(SOURCE_BYTES_MAX)]
fn reads_a_source_at_or_below_the_cap(#[case] length: usize) {
    let bytes = source_of_length(length);

    let source = PolicySource::read_from(bytes.as_slice()).expect("the bytes fit the profile");

    assert_eq!(source.as_bytes(), bytes.as_slice(), "bytes are preserved");
}

#[test]
fn a_reader_failure_after_a_partial_read_is_an_error() {
    let reader = FailingAfter {
        remaining: vec![b'a'; 100],
    };

    let result = PolicySource::read_from(reader);

    assert_that!(result, err(matches_pattern!(SourceError::Read(_))));
}

#[test]
fn a_read_error_keeps_its_cause_without_quoting_the_reader() {
    let error = SourceError::Read(io::Error::other("attacker-chosen text"));

    let rendered = error.to_string();

    assert!(
        !rendered.contains("attacker-chosen"),
        "the diagnostic must not echo the reader's text: {rendered}"
    );
    assert!(
        std::error::Error::source(&error).is_some(),
        "the cause stays available to a caller that inspects it"
    );
}

#[test]
fn the_source_conversions_agree() {
    let text = "package authz\n".to_owned();

    let source = PolicySource::from_bytes(text.clone().into_bytes()).expect("valid UTF-8");
    let cloned = source.clone();

    assert_eq!(source.as_str(), text, "the source keeps its text");
    assert_eq!(
        cloned.into_string(),
        text,
        "the owned form is the same text"
    );
}
