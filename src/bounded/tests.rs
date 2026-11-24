//! The bounded reader's limit, chunking, and failure behaviour.
//!
//! The limit is inclusive, the read stops one byte past it, chunk boundaries do
//! not change the result, and an interrupted read is retried rather than
//! reported.

use std::io::{self, Read};

use googletest::prelude::*;
use rstest::rstest;

use super::{BoundedReadError, read_bounded};

/// A reader that records how many bytes it was asked for and how many it gave.
struct Counting {
    /// The bytes the reader holds.
    bytes: Vec<u8>,
    /// The total number of bytes handed out so far.
    given: std::rc::Rc<std::cell::Cell<usize>>,
}

impl Counting {
    /// A reader over `bytes`, with a shared record of what it handed out.
    fn new(bytes: Vec<u8>) -> (Self, std::rc::Rc<std::cell::Cell<usize>>) {
        let given = std::rc::Rc::new(std::cell::Cell::new(0));
        (
            Self {
                bytes,
                given: std::rc::Rc::clone(&given),
            },
            given,
        )
    }
}

impl Read for Counting {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let take = buffer.len().min(self.bytes.len());
        buffer
            .get_mut(..take)
            .expect("the slice has the buffer's length")
            .copy_from_slice(
                self.bytes
                    .get(..take)
                    .expect("the slice has the taken length"),
            );
        self.bytes.drain(..take);
        self.given.set(self.given.get() + take);
        Ok(take)
    }
}

/// A reader that hands out at most `chunk` bytes per call.
struct Chunked {
    /// The bytes the reader holds.
    bytes: Vec<u8>,
    /// The largest chunk one call returns.
    chunk: usize,
}

impl Read for Chunked {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let take = buffer.len().min(self.chunk).min(self.bytes.len());
        buffer
            .get_mut(..take)
            .expect("the slice has the buffer's length")
            .copy_from_slice(
                self.bytes
                    .get(..take)
                    .expect("the slice has the taken length"),
            );
        self.bytes.drain(..take);
        Ok(take)
    }
}

/// A reader that reports `Interrupted` first and then succeeds.
struct InterruptingOnce {
    /// The bytes to hand out after the interruption.
    bytes: Vec<u8>,
    /// Whether the interruption has been reported.
    interrupted: bool,
}

impl Read for InterruptingOnce {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        let take = buffer.len().min(self.bytes.len());
        buffer
            .get_mut(..take)
            .expect("the slice has the buffer's length")
            .copy_from_slice(
                self.bytes
                    .get(..take)
                    .expect("the slice has the taken length"),
            );
        self.bytes.drain(..take);
        Ok(take)
    }
}

/// A reader that fails immediately.
struct Failing;

impl Read for Failing {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("the reader failed"))
    }
}

#[rstest]
#[case(0, 0)]
#[case(1, 1)]
#[case(1, 2)]
fn reads_an_input_at_or_below_the_limit(#[case] length: usize, #[case] limit: usize) {
    let bytes = vec![b'a'; length];

    let read = read_bounded(bytes.as_slice(), limit).expect("the input fits");

    assert_eq!(read, bytes, "the bytes are returned unchanged");
}

#[rstest]
#[case(2, 1)]
#[case(3, 2)]
#[case(100, 99)]
fn rejects_an_input_over_the_limit(#[case] length: usize, #[case] limit: usize) {
    let bytes = vec![b'a'; length];

    let result = read_bounded(bytes.as_slice(), limit);

    assert_that!(
        result,
        err(matches_pattern!(BoundedReadError::TooLarge {
            limit: eq(&limit)
        }))
    );
}

#[test]
fn an_unbounded_stream_is_refused_after_the_limit() {
    // `repeat` never ends, so a returned value would prove the read is bounded.
    let result = read_bounded(io::repeat(b'a'), 16);

    assert_that!(
        result,
        err(matches_pattern!(BoundedReadError::TooLarge {
            limit: eq(&16)
        }))
    );
}

#[test]
fn the_read_stops_one_byte_past_the_limit() {
    let (reader, given) = Counting::new(vec![b'a'; 100]);

    let result = read_bounded(reader, 10);

    assert_that!(
        result,
        err(matches_pattern!(BoundedReadError::TooLarge { .. }))
    );
    assert_eq!(
        given.get(),
        11,
        "the read must stop at the limit plus one, not consume the stream"
    );
}

#[rstest]
#[case(1)]
#[case(3)]
#[case(7)]
fn a_smaller_chunk_size_gives_the_same_bytes(#[case] chunk: usize) {
    let bytes = b"the profile source".to_vec();

    let read = read_bounded(
        Chunked {
            bytes: bytes.clone(),
            chunk,
        },
        32,
    )
    .expect("the input fits");

    assert_eq!(read, bytes, "chunking does not change the bytes");
}

#[test]
fn an_interrupted_read_is_retried() {
    let bytes = b"package authz\n".to_vec();
    let reader = InterruptingOnce {
        bytes: bytes.clone(),
        interrupted: false,
    };

    let read = read_bounded(reader, 32).expect("the interruption is retried");

    assert_eq!(read, bytes, "the bytes are returned unchanged");
}

#[test]
fn a_reader_failure_is_reported() {
    let result = read_bounded(Failing, 32);

    assert_that!(result, err(matches_pattern!(BoundedReadError::Read(_))));
}

#[test]
fn a_read_error_does_not_quote_the_reader() {
    let error = BoundedReadError::Read(io::Error::other("attacker-chosen text"));

    let rendered = error.to_string();

    assert!(
        !rendered.contains("attacker-chosen"),
        "the diagnostic must not echo the reader's text: {rendered}"
    );
}

#[test]
fn a_read_error_exposes_its_cause() {
    let error = BoundedReadError::Read(io::Error::other("cause"));

    assert!(
        std::error::Error::source(&error).is_some(),
        "a read failure keeps its cause for a caller that inspects it"
    );
}

#[test]
fn a_size_failure_has_no_cause() {
    let error = BoundedReadError::TooLarge { limit: 1 };

    assert!(
        std::error::Error::source(&error).is_none(),
        "a size failure has no underlying error"
    );
}
