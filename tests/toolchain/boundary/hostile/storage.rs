//! The storage behaviour: try to write where the boundary says it cannot.
//!
//! The attempts cover the root, the device tmpfs, the read-only input snapshot,
//! the private scratch area, and a host file path under a directory the probe
//! owns, passed as an argument. Only the scratch write and, under the negative control, the host
//! write may succeed. A scratch fill past `--size` follows, because a tmpfs that
//! accepts more than its bound is not private storage with a bound.
//!
//! Filesystem access goes through the same capability helper the rest of the
//! tests use, which keeps `no_std_fs_operations` satisfied without weakening the
//! attempt: ambient authority inside the sandbox is exactly what a hostile child
//! would have.

use std::{io::Write as _, path::Path};

use super::{Targets, attempt};

/// The marker every write attempt creates.
const MARKER: &[u8] = b"polean-storage-marker\n";

/// The private scratch area inside the boundary.
const SCRATCH: &str = "/job/work/marker";

/// The scratch fill's file and blocks.
const FILL: &str = "/job/work/fill";
/// One fill block, in bytes.
const FILL_BYTES: usize = 1024 * 1024;
/// The most the fill writes before calling the tmpfs unbounded.
const FILL_LIMIT: usize = 256 * 1024 * 1024;

/// Try every path and report each outcome.
///
/// # Errors
///
/// Returns only a diagnostic that cannot be reported as an observation.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every behaviour shares the dispatch signature so a failure is reported the same way"
)]
pub fn run(targets: &Targets) -> Result<(), String> {
    let device_marker = targets.marker.as_ref().map_or_else(
        || "/dev/shm/polean-probe".to_owned(),
        |marker| format!("/dev/shm/polean-{marker}"),
    );
    if targets.fixed_paths {
        for (name, path) in [
            ("scratch", SCRATCH.to_owned()),
            ("root", "/marker".to_owned()),
            ("dev-shm", device_marker),
            ("inputs", "/job/in/marker".to_owned()),
        ] {
            attempt(name, &write_marker(Path::new(&path)));
        }
    }
    if let Some(host) = &targets.host {
        attempt("host", &write_marker(host));
    }
    if targets.fixed_paths {
        fill_beyond_scratch();
    }
    Ok(())
}

/// Write the marker to `path`, reporting the outcome.
fn write_marker(path: &Path) -> String {
    match crate::ambient::create(path).and_then(|mut file| file.write_all(MARKER)) {
        Ok(()) => "wrote".to_owned(),
        Err(error) => format!("error: {error} ({:?})", error.kind()),
    }
}

/// Fill the scratch area until the filesystem refuses.
fn fill_beyond_scratch() {
    let Ok(mut file) = crate::ambient::create(Path::new(FILL)) else {
        attempt("scratch-fill", "error: cannot create the fill file");
        return;
    };
    let block = vec![b'x'; FILL_BYTES];
    let mut total = 0usize;
    while total < FILL_LIMIT {
        match file.write_all(&block) {
            Ok(()) => total += FILL_BYTES,
            Err(error) => {
                attempt(
                    "scratch-fill",
                    &format!("error after {total} bytes: {error} ({:?})", error.kind()),
                );
                return;
            }
        }
    }
    attempt(
        "scratch-fill",
        &format!("wrote {total} bytes without a bound"),
    );
}
