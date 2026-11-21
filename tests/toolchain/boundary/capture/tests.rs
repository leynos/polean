//! Named cases and a generated property for the capture kernel.
//!
//! The named cases pin the boundaries the specification turns on: an empty
//! stream, an exact fit, one byte over, a crossing inside a chunk, empty chunks,
//! and the zero cap. The property covers chunk sequences the cases do not name,
//! so it is not a table of examples restated as a proof.
//!
//! The property splits a generated stream into generated chunks and compares the
//! kept bytes with `stream[..min(len, cap)]`. The cap is drawn relative to the
//! stream length, so overflow, exact fit, and underflow all occur by
//! construction rather than by luck.

use proptest::{
    collection::vec,
    prelude::*,
    test_runner::{
        Config,
        FileFailurePersistence,
        RngAlgorithm,
        TestCaseError,
        TestRng,
        TestRunner,
    },
};

use super::Capture;
use crate::scratch::{check, check_eq};

/// The number of generated cases, matching the toolchain property suite.
const CASES: u32 = 64;

/// A shrinking runner with a fixed seed and no persisted regressions.
fn runner() -> TestRunner {
    let config = Config {
        cases: CASES,
        failure_persistence: Some(Box::new(FileFailurePersistence::Off)),
        ..Config::default()
    };
    TestRunner::new_with_rng(config, TestRng::deterministic_rng(RngAlgorithm::ChaCha))
}

/// Feed every chunk in order and report what the capture kept.
fn feed_all(cap: usize, chunks: &[Vec<u8>]) -> Result<(Vec<u8>, bool), String> {
    let mut capture = Capture::new(cap);
    for chunk in chunks {
        let fed = capture.feed(chunk);
        check!(
            fed.kept <= chunk.len(),
            "a call reported keeping {} bytes of a {} byte chunk",
            fed.kept,
            chunk.len()
        );
        check!(
            capture.kept().len() <= cap,
            "the capture holds {} bytes under a {cap} byte cap",
            capture.kept().len()
        );
    }
    Ok((capture.kept().to_vec(), capture.tripped()))
}

/// An empty stream keeps nothing and does not trip.
#[test]
fn an_empty_stream_keeps_nothing() -> Result<(), String> {
    let (kept, tripped) = feed_all(4, &[])?;
    check!(kept.is_empty(), "nothing may be kept from an empty stream");
    check!(!tripped, "an empty stream must not trip a positive cap");
    Ok(())
}

/// A stream landing exactly on the cap is kept whole and does not trip.
#[test]
fn an_exact_fit_does_not_trip() -> Result<(), String> {
    let (kept, tripped) = feed_all(4, &[b"ab".to_vec(), b"cd".to_vec()])?;
    check_eq!(kept, b"abcd".to_vec(), "the exact fit must be kept whole");
    check!(!tripped, "the cap is passed, not reached");
    Ok(())
}

/// One byte over the cap trips and keeps only the cap.
#[test]
fn one_byte_over_trips() -> Result<(), String> {
    let (kept, tripped) = feed_all(4, &[b"abc".to_vec(), b"de".to_vec()])?;
    check_eq!(kept, b"abcd".to_vec(), "only the first cap bytes are kept");
    check!(tripped, "one byte past the cap must trip");
    Ok(())
}

/// A crossing inside a chunk keeps the part that fits and trips.
#[test]
fn a_crossing_inside_a_chunk_trips() -> Result<(), String> {
    let (kept, tripped) = feed_all(4, &[b"ab".to_vec(), b"cdef".to_vec()])?;
    check_eq!(kept, b"abcd".to_vec(), "the crossing chunk is clamped");
    check!(tripped, "a crossing chunk must trip");
    Ok(())
}

/// Empty chunks neither add bytes nor trip the cap.
#[test]
fn empty_chunks_are_inert() -> Result<(), String> {
    let chunks = vec![
        Vec::new(),
        Vec::new(),
        b"a".to_vec(),
        Vec::new(),
        b"b".to_vec(),
        Vec::new(),
    ];
    let (kept, tripped) = feed_all(2, &chunks)?;
    check_eq!(
        kept,
        b"ab".to_vec(),
        "empty chunks must not disturb the stream"
    );
    check!(!tripped, "empty chunks must not trip the cap");
    Ok(())
}

/// Each call reports what it kept and whether it tripped.
#[test]
fn each_call_reports_its_own_contribution() -> Result<(), String> {
    let mut capture = Capture::new(4);
    let first = capture.feed(b"abc");
    let second = capture.feed(b"def");
    check_eq!(first.kept, 3, "the first call keeps the whole chunk");
    check!(!first.tripped, "the first call does not trip");
    check_eq!(second.kept, 1, "the second call keeps only the room left");
    check!(second.tripped, "the second call trips");
    check_eq!(capture.cap(), 4, "the cap is fixed at construction");
    check!(capture.tripped(), "the capture records the trip");
    Ok(())
}

/// A tripped capture keeps nothing more and stays tripped.
#[test]
fn a_tripped_capture_is_stable() -> Result<(), String> {
    let mut capture = Capture::new(2);
    capture.feed(b"ab");
    let tripping = capture.feed(b"c");
    let after = capture.feed(b"def");
    check!(tripping.tripped, "the crossing chunk did not fit");
    check_eq!(after.kept, 0, "nothing is kept after the trip");
    check!(
        after.tripped,
        "a later non-empty chunk also does not fit below the full cap"
    );
    check_eq!(
        capture.kept(),
        &b"ab"[..],
        "the kept bytes never change after the trip"
    );
    Ok(())
}

/// A zero cap keeps nothing from a non-empty stream and trips.
#[test]
fn a_zero_cap_trips_on_any_byte() -> Result<(), String> {
    let (kept, tripped) = feed_all(0, &[b"x".to_vec()])?;
    check!(kept.is_empty(), "a zero cap keeps nothing");
    check!(tripped, "a zero cap is passed by any byte");
    Ok(())
}

/// A zero cap does not trip on empty chunks, because the cap is passed.
#[test]
fn a_zero_cap_accepts_an_empty_stream() -> Result<(), String> {
    let (kept, tripped) = feed_all(0, &[Vec::new(), Vec::new()])?;
    check!(kept.is_empty(), "a zero cap keeps nothing");
    check!(!tripped, "empty chunks do not pass a zero cap");
    Ok(())
}

/// The generated property: kept bytes are the stream's first `min(len, cap)`.
#[test]
fn kept_bytes_are_the_streams_prefix() {
    let chunk = vec(any::<u8>(), 0..=6);
    let strategy = (0usize..=8, vec(chunk, 0..=6));
    if let Err(error) = runner().run(&strategy, |(cap, chunks)| {
        let stream: Vec<u8> = chunks.iter().flatten().copied().collect();
        let split = stream.len().min(cap);
        let (expected, _rest) = stream.split_at(split);
        let (kept, tripped) = match feed_all(cap, &chunks) {
            Ok(pair) => pair,
            Err(reason) => return Err(TestCaseError::fail(reason)),
        };
        prop_assert_eq!(kept.as_slice(), expected);
        prop_assert_eq!(tripped, stream.len() > cap);
        Ok(())
    }) {
        panic!("the capture property failed: {error}");
    }
}
