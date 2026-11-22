//! Bounded control subprocesses.
//!
//! Every program the supervisor runs to *observe* the job — `systemctl`,
//! `systemd-run`'s client, the launcher version check — must finish within a
//! bound. `Command::output` waits indefinitely, so a stalled `systemctl` would
//! hold the supervisor past the job's deadline and past teardown. This module
//! spawns the program, drains both streams on their own threads, polls against
//! one bound, and kills and reaps a program that outlives it.
//!
//! The stream caps are small and fixed: control programs print diagnostics, not
//! job output. A program that floods them is stopped by the cap and reported as
//! a control failure rather than buffered without limit.

use std::{
    io::Read,
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel},
    thread,
    time::{Duration, Instant},
};

use super::capture::Capture;

/// How long a control program may run.
pub const CONTROL_BOUND: Duration = Duration::from_secs(5);

/// How often a control program is polled.
const POLL: Duration = Duration::from_millis(20);

/// The cap on one control stream.
const STREAM_CAP: usize = 64 * 1024;

/// A finished control program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    /// The exit status.
    pub status: ExitStatus,
    /// The kept standard output.
    pub stdout: Vec<u8>,
    /// The kept standard error.
    pub stderr: Vec<u8>,
}

impl Output {
    /// Whether the program exited zero.
    #[must_use]
    pub fn success(&self) -> bool { self.status.success() }

    /// Standard output as text, lossily.
    #[must_use]
    pub fn stdout_text(&self) -> String { String::from_utf8_lossy(&self.stdout).trim().to_owned() }

    /// Standard error as text, lossily.
    #[must_use]
    pub fn stderr_text(&self) -> String { String::from_utf8_lossy(&self.stderr).trim().to_owned() }
}

/// One environment variable a control program needs.
pub type Env<'a> = (&'a str, &'a Path);

/// Run `program` with `args` and exactly `env`, bounded by `bound`.
///
/// # Errors
///
/// Returns a diagnostic when the program cannot be started, does not finish
/// within `bound` (it is killed and reaped first), or a stream reader fails.
pub fn run(
    program: &Path,
    args: &[&str],
    env: &[Env<'_>],
    bound: Duration,
) -> Result<Output, String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot run {}: {error}", program.display()))?;
    let (trip_sender, trips) = channel();
    let (stdout, stderr) = readers(&mut child, trip_sender)?;
    let status = match wait(&mut child, bound, &trips) {
        Ok(status) => status,
        Err(reason) => {
            stop(&mut child);
            return Err(format!("{} {reason}", program.display()));
        }
    };
    Ok(Output {
        status,
        stdout: collect(&stdout, "standard output")?,
        stderr: collect(&stderr, "standard error")?,
    })
}

/// One stream's delivered bytes and whether it exceeded the cap.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Drained {
    /// The kept bytes.
    bytes: Vec<u8>,
    /// Whether the stream exceeded the control cap.
    tripped: bool,
    /// The read error, when the stream failed after a valid prefix.
    error: Option<String>,
}

/// One stream's delivery channel.
type Reader = Receiver<Drained>;

/// Start one bounded reader per stream.
fn readers(child: &mut Child, trips: Sender<()>) -> Result<(Reader, Reader), String> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "standard output is not piped".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "standard error is not piped".to_owned())?;
    Ok((drain(stdout, trips.clone()), drain(stderr, trips)))
}

/// Read one stream into a capture and send the kept bytes when it ends.
fn drain(mut stream: impl Read + Send + 'static, trips: Sender<()>) -> Reader {
    let (sender, receiver) = channel();
    thread::spawn(move || {
        let mut capture = Capture::new(STREAM_CAP);
        let mut signalled = false;
        let mut error = None;
        let mut buffer = [0u8; 8 * 1024];
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
            if trips_send(&capture, &trips, &mut signalled) {
                break;
            }
        }
        drop(sender.send(Drained {
            bytes: capture.kept().to_vec(),
            tripped: capture.tripped(),
            error,
        }));
    });
    receiver
}

/// Signal the first cap trip, reporting whether the supervisor has gone.
fn trips_send(capture: &Capture, trips: &Sender<()>, signalled: &mut bool) -> bool {
    if !capture.tripped() || *signalled {
        return false;
    }
    *signalled = true;
    trips.send(()).is_err()
}

/// Poll the child against the bound, killing it when the bound expires.
fn wait(child: &mut Child, bound: Duration, trips: &Receiver<()>) -> Result<ExitStatus, String> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => (),
            Err(error) => return Err(format!("cannot wait for the program: {error}")),
        }
        if trips.try_recv().is_ok() {
            stop(child);
            return Err(format!(
                "a control stream exceeded the {STREAM_CAP} byte cap; the program was stopped"
            ));
        }
        if started.elapsed() >= bound {
            stop(child);
            return Err(format!("did not finish within {bound:?}"));
        }
        thread::sleep(POLL);
    }
}

/// Kill and reap the child, ignoring an already-dead process.
fn stop(child: &mut Child) {
    drop(child.kill());
    drop(child.wait());
}

/// Collect one reader's bytes, bounded by the control bound.
fn collect(reader: &Reader, name: &str) -> Result<Vec<u8>, String> {
    match reader.recv_timeout(CONTROL_BOUND) {
        Ok(drained) if drained.tripped => Err(format!(
            "the {name} exceeded the {STREAM_CAP} byte control cap; its output was truncated"
        )),
        Ok(drained) if drained.error.is_some() => Err(format!(
            "the {name} failed after a valid prefix: {}",
            drained.error.unwrap_or_default()
        )),
        Ok(drained) => Ok(drained.bytes),
        Err(RecvTimeoutError::Timeout) => Err(format!("the {name} reader did not finish")),
        Err(RecvTimeoutError::Disconnected) => Err(format!("the {name} reader ended without data")),
    }
}
