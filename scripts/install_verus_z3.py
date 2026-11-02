#!/usr/bin/env -S uv run python
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Install the pinned Z3 solver that the pinned Verus binary needs.

The Verus release archive does not contain a solver. Its own
``source/tools/get-z3.sh`` names the release this module pins, and the digest in
:file:`tools/verus/Z3_SHA256SUMS` is that release's published asset digest.
Verus resolves the solver from ``VERUS_Z3_PATH``, which ``make verus`` points at
the binary this script writes.

Only ``bin/z3`` is extracted: the archive's headers, libraries, and examples are
not installed, and no archive member path is followed. The pin file is the only
source of the archive name and digest, so a missing or malformed pin fails
closed rather than falling back to a solver found on ``PATH``.

The install directory carries a stamp recording the pin that produced the
solver and the solver's own digest. An existing binary is reused only when the
stamp matches the current pin and the binary still hashes to the recorded
digest; a changed pin, a missing or malformed stamp, or a modified binary makes
the installer fetch and verify the archive again. Cached state therefore cannot
outlive the pin that justified it.

Downloading is a parameter rather than a module global, so the tests can inject
a fetcher without patching. The script launches no subprocess and uses the
standard library only.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import sys
import tempfile
import time
import urllib.request
import zipfile
from collections.abc import Callable
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PINS_PATH = ROOT / "tools" / "verus" / "Z3_SHA256SUMS"
INSTALL_DIR = ROOT / ".verus" / "z3"
STAMP_NAME = "z3.pin"

BASE_URL = "https://github.com/Z3Prover/z3/releases/download"
CHUNK_SIZE = 1024 * 1024
REQUEST_TIMEOUT_SECONDS = 60
DOWNLOAD_DEADLINE_SECONDS = 10 * 60
MAX_ARCHIVE_BYTES = 64 * 1024 * 1024

_ARCHIVE = re.compile(r"\A(z3-(?P<version>[0-9][0-9.]*)-(?P<platform>\S+))[.]zip\Z")
_DIGEST = re.compile(r"\A[0-9a-f]{64}\Z")

Fetcher = Callable[[str, Path], None]


def read_pin(path: Path = PINS_PATH) -> tuple[str, str]:
    """Return the archive name and digest the pin file states.

    Parameters
    ----------
    path : Path
        The checksum file. It must hold exactly one row.

    Returns
    -------
    tuple[str, str]
        The archive name and its expected SHA-256 digest.

    Raises
    ------
    ValueError
        If the file is missing, empty, holds more than one row, names an archive
        the release URL cannot be built from, or states a malformed digest.
    """
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise ValueError(f"cannot read the Z3 pin {path}: {error}") from error
    rows = [row.split() for row in text.splitlines() if row.strip()]
    if len(rows) != 1:
        raise ValueError(f"{path} must hold exactly one checksum row, found {len(rows)}")
    fields = rows[0]
    if len(fields) < 2:
        raise ValueError(f"{path} must give a digest and an archive name")
    digest, name = fields[0], fields[1]
    if not _DIGEST.fullmatch(digest):
        raise ValueError(f"{path} gives a malformed SHA-256 digest")
    if _ARCHIVE.fullmatch(name) is None:
        raise ValueError(f"{path} gives an archive name the release URL cannot use")
    return name, digest


def archive_url(name: str) -> str:
    """Return the release URL for the pinned Z3 archive."""
    match = _ARCHIVE.fullmatch(name)
    if match is None:
        raise ValueError(f"cannot build a Z3 release URL from {name}")
    return f"{BASE_URL}/z3-{match.group('version')}/{name}"


def download(url: str, destination: Path) -> None:
    """Fetch `url` into `destination`, bounded in time and size.

    Raises
    ------
    ValueError
        If the transfer exceeds the size bound.
    OSError
        If the request fails.
    """
    request = urllib.request.Request(url, headers={"User-Agent": "polean-z3-install"})
    expires_at = time.monotonic() + DOWNLOAD_DEADLINE_SECONDS
    with urllib.request.urlopen(request, timeout=REQUEST_TIMEOUT_SECONDS) as response:
        written = 0
        with destination.open("wb") as sink:
            while True:
                chunk = response.read(CHUNK_SIZE)
                if not chunk:
                    return
                written += len(chunk)
                if written > MAX_ARCHIVE_BYTES:
                    raise ValueError(f"{url} is larger than the size bound")
                if time.monotonic() > expires_at:
                    raise ValueError(f"{url} did not finish within the deadline")
                sink.write(chunk)


def file_digest(path: Path) -> str:
    """Return the SHA-256 digest of `path`'s bytes."""
    hasher = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(CHUNK_SIZE), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def verify_digest(path: Path, digest: str) -> None:
    """Raise `ValueError` unless `path` hashes to `digest`."""
    if file_digest(path) != digest:
        raise ValueError(f"{path} does not match the pinned SHA-256 digest")


def read_stamp(path: Path) -> dict[str, str] | None:
    """Return the recorded pin identity, or `None` if it is unusable.

    A missing, unreadable, malformed, or partially recorded stamp is treated as
    absent: the caller reinstalls rather than trusting the cached binary.
    """
    try:
        recorded = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return None
    if not isinstance(recorded, dict):
        return None
    fields = ("archive", "archive_sha256", "solver_sha256")
    if any(not isinstance(recorded.get(field), str) for field in fields):
        return None
    return {field: recorded[field] for field in fields}


def write_stamp(path: Path, archive: str, archive_digest: str, solver_digest: str) -> None:
    """Record the pin identity and the extracted solver's digest."""
    path.write_text(
        json.dumps(
            {
                "archive": archive,
                "archive_sha256": archive_digest,
                "solver_sha256": solver_digest,
            },
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )


def stamp_matches(
    stamp: Path, binary: Path, name: str, digest: str
) -> bool:
    """Whether the cached solver was produced by the current pin and is intact."""
    recorded = read_stamp(stamp)
    if recorded is None or not binary.is_file():
        return False
    if recorded["archive"] != name or recorded["archive_sha256"] != digest:
        return False
    try:
        return file_digest(binary) == recorded["solver_sha256"]
    except OSError:
        return False


def extract_solver(archive: Path, name: str, destination: Path) -> None:
    """Write the archive's `bin/z3` member to `destination`.

    Raises
    ------
    ValueError
        If the archive is not a zip file or lacks the expected member.
    """
    member = f"{name.removesuffix('.zip')}/bin/z3"
    try:
        with zipfile.ZipFile(archive) as bundle:
            payload = bundle.read(member)
    except (zipfile.BadZipFile, KeyError) as error:
        raise ValueError(f"{archive} does not hold {member}: {error}") from error
    destination.write_bytes(payload)
    destination.chmod(0o755)


def install(
    *,
    pins: Path = PINS_PATH,
    install_dir: Path = INSTALL_DIR,
    fetch: Fetcher = download,
) -> Path:
    """Install the pinned solver and return the path to the binary.

    The install is idempotent only while the cached solver matches the current
    pin: the stamp must name the pinned archive and digest, and the binary must
    still hash to the digest the stamp records. Any mismatch reinstalls, and a
    failed verification leaves neither a binary nor a stamp behind.

    Raises
    ------
    ValueError
        If the pin is malformed, the archive is the wrong size or digest, or
        the archive lacks the solver.
    """
    name, digest = read_pin(pins)
    binary = install_dir / "z3"
    stamp = install_dir / STAMP_NAME
    if stamp_matches(stamp, binary, name, digest):
        return binary
    install_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=install_dir) as temporary:
        archive = Path(temporary) / name
        fetch(archive_url(name), archive)
        verify_digest(archive, digest)
        staged = Path(temporary) / "z3"
        extract_solver(archive, name, staged)
        staged_stamp = Path(temporary) / STAMP_NAME
        write_stamp(staged_stamp, name, digest, file_digest(staged))
        # The stamp lands last, so a crash between the two replacements leaves
        # a binary with an old stamp: the next run sees a mismatch and
        # reinstalls instead of trusting it.
        os.replace(staged, binary)
        os.replace(staged_stamp, stamp)
    return binary


def main() -> None:
    """Install the solver, naming the result."""
    try:
        binary = install()
    except (OSError, ValueError) as error:
        raise SystemExit(f"cannot install Z3 for Verus: {error}") from error
    print(f"Installed Z3 at {binary}")


if __name__ == "__main__":
    sys.exit(main())