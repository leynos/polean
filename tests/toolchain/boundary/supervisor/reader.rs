//! The bounded stream readers.
//!
//! Each stream is drained on its own thread through its own [`Capture`], so a
//! flood is bounded while it happens rather than after. A reader reports the
//! first trip over a channel, which stops the job, and delivers its kept bytes
//! and final trip state when the pipe closes. The supervisor's wait on those
//! bytes is bounded: an ordinary join, with a reported diagnostic instead of a
//! hang when authoritative cleanup could not close the pipes.
//!
//! The readers and the supervisor run in one process; nothing here decides
//! whether a trip is a failure, only that one happened.

use std::{
    io::Read,
    process::Child,
    sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel},
    thread::{self, JoinHandle},
    time::Duration,
};

use super::{
    super::{
        capture::Capture,
        outcome::{Bound, Bounds},
    },
    Cancel,
    Job,
};

/// How long a reader may take to deliver its bytes after teardown.
pub(super) const DRAIN_BOUND: Duration = Duration::from_secs(5);

/// The channels one reader reports through.
struct Signals {
    /// The first-trip signal.
    trips: Sender<Bound>,
    /// The delivered bytes.
    done: Sender<Drained>,
    /// The cancellation request this stream arms, when it carries the marker.
    arm: Option<Cancel>,
}

/// What one drained stream delivered.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Drained {
    /// The kept bytes.
    bytes: Vec<u8>,
    /// Whether the stream ever exceeded its cap.
    tripped: bool,
    /// The read error, when the stream failed after a valid prefix.
    error: Option<String>,
}

/// The readers and the trip channel for one job.
pub(super) struct Readers {
    /// Standard output's delivered bytes.
    stdout: Receiver<Drained>,
    /// Standard error's delivered bytes.
    stderr: Receiver<Drained>,
    /// The first-trip signal from either reader.
    pub(super) trips: Receiver<Bound>,
    /// The reader threads.
    handles: [JoinHandle<()>; 2],
}

/// Request cancellation once the kept bytes carry the arming marker.
///
/// The whole kept prefix is searched, not the latest chunk, so a marker split
/// across two pipe reads is still found. A marker longer than the cap could
/// not be observed, so the request would never be armed and the job would end
/// at its deadline instead; the cancellation probe uses a short marker and
/// asserts the cancelled bound, so that case fails rather than passing
/// silently.
fn arm(request: Option<&Cancel>, kept: &[u8]) {
    let Some(cancel) = request else {
        return;
    };
    if cancel.request.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    if contains_marker(kept, cancel.marker.as_bytes()) {
        cancel
            .request
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Whether `kept` contains `marker`.
pub(crate) fn contains_marker(kept: &[u8], marker: &[u8]) -> bool {
    !marker.is_empty() && kept.windows(marker.len()).any(|window| window == marker)
}

/// Start one bounded reader per stream.
pub(super) fn readers(child: &mut Child, job: &Job) -> Result<Readers, String> {
    let (trip_sender, trips) = channel();
    let (stdout_sender, stdout) = channel();
    let (stderr_sender, stderr) = channel();
    let stdout_pipe = child
        .stdout
        .take()
        .ok_or_else(|| "standard output is not piped".to_owned())?;
    let stderr_pipe = child
        .stderr
        .take()
        .ok_or_else(|| "standard error is not piped".to_owned())?;
    let stdout_handle = drain(
        stdout_pipe,
        job.stdout_cap,
        Bound::Stdout,
        Signals {
            trips: trip_sender.clone(),
            done: stdout_sender,
            arm: job.cancel.clone(),
        },
    );
    let stderr_handle = drain(
        stderr_pipe,
        job.stderr_cap,
        Bound::Stderr,
        Signals {
            trips: trip_sender,
            done: stderr_sender,
            arm: None,
        },
    );
    Ok(Readers {
        stdout,
        stderr,
        trips,
        handles: [stdout_handle, stderr_handle],
    })
}

/// Read one stream into a capture until end of file or cap trip.
///
/// The trip is signalled once. After it, the reader keeps draining into the
/// capture, which keeps nothing more, so the child never blocks on a full pipe
/// before the supervisor's kill lands.
fn drain(
    mut stream: impl Read + Send + 'static,
    cap: usize,
    bound: Bound,
    signals: Signals,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut capture = Capture::new(cap);
        let mut signalled = false;
        let mut error = None;
        let mut buffer = [0u8; 16 * 1024];
        loop {
            let read = match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(failure) if failure.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(failure) => {
                    error = Some(failure.to_string());
                    break;
                }
            };
            let chunk = buffer.get(..read).unwrap_or(&[]);
            capture.feed(chunk);
            arm(signals.arm.as_ref(), capture.kept());
            if !record_trip(&capture, &signals.trips, bound, &mut signalled) {
                break;
            }
        }
        drop(signals.done.send(Drained {
            bytes: capture.kept().to_vec(),
            tripped: capture.tripped(),
            error,
        }));
    })
}

/// Signal the first trip, reporting whether reading should continue.
fn record_trip(
    capture: &Capture,
    trips: &Sender<Bound>,
    bound: Bound,
    signalled: &mut bool,
) -> bool {
    if !capture.tripped() || *signalled {
        return true;
    }
    *signalled = true;
    trips.send(bound).is_ok()
}

/// Collect one reader's bytes, bounded so an unclosable pipe cannot hang.
fn collect(
    reader: &Receiver<Drained>,
    handle: JoinHandle<()>,
    name: &str,
    diagnostics: &mut Vec<String>,
) -> Drained {
    match reader.recv_timeout(DRAIN_BOUND) {
        Ok(drained) => {
            if let Some(error) = &drained.error {
                diagnostics.push(format!("the {name} failed after a valid prefix: {error}"));
            }
            drop(handle.join());
            drained
        }
        Err(RecvTimeoutError::Timeout) => {
            diagnostics.push(format!(
                "the {name} reader did not finish within {DRAIN_BOUND:?} after teardown"
            ));
            Drained {
                bytes: Vec::new(),
                tripped: false,
                error: None,
            }
        }
        Err(RecvTimeoutError::Disconnected) => {
            diagnostics.push(format!(
                "the {name} reader ended without delivering its bytes"
            ));
            Drained {
                bytes: Vec::new(),
                tripped: false,
                error: None,
            }
        }
    }
}

/// Collect both readers and merge their final trip states into the bounds.
pub(super) fn finish_readers(
    readers: Readers,
    mut bounds: Bounds,
    diagnostics: &mut Vec<String>,
) -> (Vec<u8>, Vec<u8>, Bounds) {
    let Readers {
        stdout,
        stderr,
        trips: _trips,
        handles: [stdout_handle, stderr_handle],
    } = readers;
    let out = collect(&stdout, stdout_handle, "standard output", diagnostics);
    let err = collect(&stderr, stderr_handle, "standard error", diagnostics);
    if out.tripped {
        bounds.insert(Bound::Stdout);
    }
    if err.tripped {
        bounds.insert(Bound::Stderr);
    }
    (out.bytes, err.bytes, bounds)
}

/// Drain both readers on the failure path, without blocking on a live thread.
///
/// The caller is already reporting a diagnostic. The delivered bytes are taken
/// so a reader is not left blocked on a full channel, and a thread that has
/// finished is joined; one still running is left to end when its pipe closes.
pub(super) fn drain_readers(readers: &Readers, diagnostics: &mut Vec<String>) {
    for (reader, name) in [
        (&readers.stdout, "standard output"),
        (&readers.stderr, "standard error"),
    ] {
        match reader.recv_timeout(DRAIN_BOUND) {
            Ok(drained) => {
                if let Some(error) = drained.error {
                    diagnostics.push(format!("the {name} failed after a valid prefix: {error}"));
                }
            }
            Err(error) => diagnostics.push(format!("the {name} reader reported {error}")),
        }
    }
}
