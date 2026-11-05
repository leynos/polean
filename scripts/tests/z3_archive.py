"""Synthetic Z3 release artefacts shared by the installer's test modules.

The fixtures are constructed, not downloaded: a release archive holding only the
solver member the installer extracts, and a one-row checksum file naming it. The
unit tests inject a fetcher over them, and the command-boundary tests serve them
from a loopback HTTP server, so neither route reaches a public release service.
"""

from __future__ import annotations

import hashlib
import io
import zipfile
from pathlib import Path

VERSION = "4.16.0"
ROOT_NAME = f"z3-{VERSION}-x64-glibc-2.39"
ARCHIVE_NAME = f"{ROOT_NAME}.zip"
SOLVER_BYTES = b"\x7fELF-z3-placeholder"
DIGEST = hashlib.sha256(b"archive-bytes").hexdigest()


def write_pin(directory: Path, digest: str = DIGEST, name: str = ARCHIVE_NAME) -> Path:
    """Write a one-row checksum file in the pinned format."""
    path = directory / "Z3_SHA256SUMS"
    path.write_text(f"{digest}  {name}\n", encoding="utf-8")
    return path


def archive_bytes(
    root: str = ROOT_NAME,
    extra: dict[str, bytes] | None = None,
    solver: bytes = SOLVER_BYTES,
) -> bytes:
    """Build a zip holding `solver` under `root`, plus any extra members."""
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, mode="w") as bundle:
        bundle.writestr(f"{root}/bin/z3", solver)
        for member, payload in (extra or {}).items():
            bundle.writestr(member, payload)
    return buffer.getvalue()


def archive_digest() -> str:
    """The digest of the default synthetic archive."""
    return hashlib.sha256(archive_bytes()).hexdigest()