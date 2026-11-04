"""Command-boundary tests for the Z3 installer (EP-M4).

The unit tests call `install()` with an injected fetcher. These tests cross the
process boundary instead: they run the script the way `make install-verus` does,
against an isolated temporary layout and a loopback release server. Nothing here
contacts a public release service, and no test touches the repository's
`.verus/z3`.

Covered at this boundary: successful installation, malformed pins, download
failure, digest failure, and cached-solver reuse without another download. Each
test asserts the exit status, the diagnostic, the installed bytes, the stamp, or
the cleanup that the boundary is responsible for.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import threading
from collections.abc import Iterator
from dataclasses import dataclass, field
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import install_verus_z3
import pytest
from z3_archive import ARCHIVE_NAME, SOLVER_BYTES, VERSION, archive_bytes, write_pin

SCRIPT = Path(install_verus_z3.__file__).resolve()
ARCHIVE_PATH = f"/z3-{VERSION}/{ARCHIVE_NAME}"


def digest(payload: bytes) -> str:
    """The SHA-256 digest of `payload`."""
    return hashlib.sha256(payload).hexdigest()


@dataclass
class Release:
    """A loopback release server and the requests it answered."""

    base_url: str
    routes: dict[str, tuple[int, bytes]]
    requests: list[str] = field(default_factory=list)


def _serve(release: Release) -> type[BaseHTTPRequestHandler]:
    """Build a handler that answers from `release.routes` and records paths."""

    class Handler(BaseHTTPRequestHandler):
        """Serve only the routes the test registered."""

        def do_GET(self) -> None:
            """Record the request and answer it from the route table."""
            release.requests.append(self.path)
            status, payload = release.routes.get(self.path, (404, b"not found"))
            self.send_response(status)
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

        def log_message(self, *arguments: object) -> None:
            """Keep the test output free of access logs."""

    return Handler


@pytest.fixture
def release() -> Iterator[Release]:
    """Serve registered routes on a loopback port until the test ends."""
    live = Release("", {})
    server = ThreadingHTTPServer(("127.0.0.1", 0), _serve(live))
    live.base_url = f"http://127.0.0.1:{server.server_port}"
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield live
    finally:
        server.shutdown()
        thread.join(timeout=10)
        server.server_close()


def run_installer(*arguments: str | Path) -> subprocess.CompletedProcess[str]:
    """Run the installer script as the Makefile's recipe would.

    Proxy variables are removed so a configured HTTP proxy cannot intercept the
    loopback request; the digest check would still fail closed, but the test
    would then be asserting through an unrelated transport failure.
    """
    environment = {
        name: value
        for name, value in os.environ.items()
        if name.lower() not in {"http_proxy", "https_proxy", "all_proxy", "no_proxy"}
    }
    return subprocess.run(
        [sys.executable, str(SCRIPT), *(str(argument) for argument in arguments)],
        capture_output=True,
        text=True,
        timeout=120,
        check=False,
        env=environment,
    )


def layout(tmp_path: Path, *, payload: bytes | None = None) -> tuple[Path, Path]:
    """Write an isolated pin file and return it with a fresh install dir."""
    payload = payload if payload is not None else archive_bytes()
    pins_dir = tmp_path / "pins"
    pins_dir.mkdir()
    return write_pin(pins_dir, digest=digest(payload)), tmp_path / "install"


def stamp(install_dir: Path) -> dict[str, object]:
    """Read back the stamp the installer wrote."""
    text = (install_dir / install_verus_z3.STAMP_NAME).read_text(encoding="utf-8")
    return json.loads(text)


def test_installs_from_a_loopback_release(release: Release, tmp_path: Path) -> None:
    """A successful run installs the solver, stamps it, and cleans up."""
    payload = archive_bytes()
    release.routes[ARCHIVE_PATH] = (200, payload)
    pins, install_dir = layout(tmp_path, payload=payload)

    result = run_installer(
        "--pins", pins, "--install-dir", install_dir, "--base-url", release.base_url
    )

    assert result.returncode == 0, result.stderr
    assert f"Installed Z3 at {install_dir / 'z3'}" in result.stdout
    binary = install_dir / "z3"
    assert binary.read_bytes() == SOLVER_BYTES, "the solver bytes survive the install"
    assert binary.stat().st_mode & 0o777 == 0o755, "the solver is executable"
    assert sorted(path.name for path in install_dir.iterdir()) == [
        "z3",
        install_verus_z3.STAMP_NAME,
    ], "the staging directory is removed"
    recorded = stamp(install_dir)
    assert recorded["archive"] == ARCHIVE_NAME, "the stamp names the pinned archive"
    assert recorded["archive_sha256"] == digest(payload)
    assert recorded["solver_sha256"] == digest(SOLVER_BYTES)
    assert release.requests == [ARCHIVE_PATH], "exactly one archive request"


def test_reports_a_malformed_pin(release: Release, tmp_path: Path) -> None:
    """A pin file that is not exactly one row fails before any download."""
    payload = archive_bytes()
    pins = tmp_path / "Z3_SHA256SUMS"
    pins.write_text(
        f"{digest(payload)}  {ARCHIVE_NAME}\n{digest(payload)}  {ARCHIVE_NAME}\n",
        encoding="utf-8",
    )

    result = run_installer(
        "--pins",
        pins,
        "--install-dir",
        tmp_path / "install",
        "--base-url",
        release.base_url,
    )

    assert result.returncode == 1, "a malformed pin is a failure"
    assert "exactly one checksum row" in result.stderr
    assert release.requests == [], "a malformed pin never reaches the network"


def test_reports_a_download_failure(release: Release, tmp_path: Path) -> None:
    """A release host that does not serve the archive is reported as such."""
    release.routes[ARCHIVE_PATH] = (404, b"missing")
    pins, install_dir = layout(tmp_path)

    result = run_installer(
        "--pins", pins, "--install-dir", install_dir, "--base-url", release.base_url
    )

    assert result.returncode == 1, "a failed transfer is a failure"
    assert "cannot install Z3 for Verus" in result.stderr
    assert "404" in result.stderr, "the diagnostic carries the transport failure"
    assert not (install_dir / "z3").exists(), "no solver is installed"
    assert not (install_dir / install_verus_z3.STAMP_NAME).exists(), "no stamp is left"


def test_reports_a_digest_failure(release: Release, tmp_path: Path) -> None:
    """Bytes that do not match the pin leave neither a solver nor a stamp."""
    release.routes[ARCHIVE_PATH] = (200, archive_bytes(solver=b"tampered"))
    pins, install_dir = layout(tmp_path)

    result = run_installer(
        "--pins", pins, "--install-dir", install_dir, "--base-url", release.base_url
    )

    assert result.returncode == 1, "a digest mismatch is a failure"
    assert "does not match the pinned SHA-256 digest" in result.stderr
    assert not (install_dir / "z3").exists(), "the unverified solver is not installed"
    assert not (install_dir / install_verus_z3.STAMP_NAME).exists(), "no stamp is left"
    assert list(install_dir.iterdir()) == [], "the staging directory is removed"


def test_reuses_a_cached_solver_without_downloading(
    release: Release, tmp_path: Path
) -> None:
    """A second run trusts the stamp and makes no archive request."""
    payload = archive_bytes()
    release.routes[ARCHIVE_PATH] = (200, payload)
    pins, install_dir = layout(tmp_path, payload=payload)
    arguments = (
        "--pins",
        pins,
        "--install-dir",
        install_dir,
        "--base-url",
        release.base_url,
    )

    first = run_installer(*arguments)
    second = run_installer(*arguments)

    assert first.returncode == 0, first.stderr
    assert second.returncode == 0, second.stderr
    assert release.requests == [ARCHIVE_PATH], "the cached solver is reused"
    assert (install_dir / "z3").read_bytes() == SOLVER_BYTES
