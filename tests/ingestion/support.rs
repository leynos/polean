//! Shared readers and fixture loading for the ingestion tests.
//!
//! The readers exist so a test can state how a stream is split, interrupted, or
//! cut short, without the library needing to know. Fixture loading goes through
//! `cap-std`, because Whitaker's `no_std_fs_operations` lint rejects `std::fs`.

use std::{
    io::{self, Read},
    path::Path,
    rc::Rc,
};

use cap_std::{ambient_authority, fs::Dir};

/// Copy the first `take` bytes of `source` into `buffer`.
fn copy_into(buffer: &mut [u8], source: &[u8], take: usize) {
    let destination = buffer.get_mut(..take).expect("take fits the buffer");
    let origin = source.get(..take).expect("take fits the source");
    destination.copy_from_slice(origin);
}

/// A reader that returns at most `chunk` bytes per call and reports
/// `Interrupted` before every successful read.
pub struct ChunkedInterrupting {
    /// The bytes still to hand out.
    bytes: Vec<u8>,
    /// The largest chunk one call returns.
    chunk: usize,
    /// Whether the interruption for the next call has been reported.
    interrupted: bool,
}

impl ChunkedInterrupting {
    /// A reader over `bytes` returning at most `chunk` bytes per call, with one
    /// interruption before each successful read.
    #[must_use]
    pub fn new(bytes: Vec<u8>, chunk: usize) -> Self {
        Self {
            bytes,
            chunk: chunk.max(1),
            interrupted: false,
        }
    }
}

impl Read for ChunkedInterrupting {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.bytes.is_empty() {
            return Ok(0);
        }
        if !self.interrupted {
            self.interrupted = true;
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        self.interrupted = false;
        let take = buffer.len().min(self.chunk).min(self.bytes.len());
        copy_into(buffer, &self.bytes, take);
        self.bytes.drain(..take);
        Ok(take)
    }
}

/// A reader that counts the bytes it hands out.
pub struct Counted<R> {
    /// The reader the count is taken from.
    inner: R,
    /// The total handed out so far.
    given: Rc<std::cell::Cell<usize>>,
}

impl<R> Counted<R> {
    /// Wrap `inner`, returning the reader and its running byte count.
    #[must_use]
    pub fn new(inner: R) -> (Self, Rc<std::cell::Cell<usize>>) {
        let given = Rc::new(std::cell::Cell::new(0));
        (
            Self {
                inner,
                given: Rc::clone(&given),
            },
            given,
        )
    }
}

impl<R: Read> Read for Counted<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buffer)?;
        self.given.set(self.given.get() + read);
        Ok(read)
    }
}

/// The repository root the fixtures are read from.
///
/// `CARGO_MANIFEST_DIR` names it at compile time, so loading does not depend on
/// the working directory the test binary is started from.
fn repository_root() -> &'static Path { Path::new(env!("CARGO_MANIFEST_DIR")) }

/// Read a repository-relative fixture as bytes.
///
/// # Errors
///
/// Returns a message naming the path when it cannot be read.
pub fn fixture_bytes(relative: &str) -> Result<Vec<u8>, String> {
    let directory = Dir::open_ambient_dir(repository_root(), ambient_authority())
        .map_err(|error| format!("cannot open the repository root: {error}"))?;
    // `Dir::read` resolves only relative to the capability it was opened on,
    // so an absolute path is outside it and is refused.
    directory
        .read(relative)
        .map_err(|error| format!("cannot read {relative}: {error}"))
}

/// Read a repository-relative fixture as UTF-8 text.
///
/// # Errors
///
/// Returns a message naming the path when the bytes cannot be read or are not
/// UTF-8.
pub fn fixture_text(relative: &str) -> Result<String, String> {
    let bytes = fixture_bytes(relative)?;
    String::from_utf8(bytes).map_err(|error| format!("{relative} is not UTF-8: {error}"))
}
