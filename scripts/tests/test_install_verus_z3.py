"""Tests for the pinned Z3 installer that `make verus` depends on (EP-M4).

Every test stays off the network. The install tests inject a fetcher and point
the installer at temporary directories, so no test touches `.verus/z3` or the
pinned release. The download tests replace `urllib.request.urlopen` so the size
bound and the deadline can be exercised without a transfer.

The tests cover the two obligations the plan states for the installer:

* the pin file is the only source of the archive name and digest, so a pin that
  is missing, ambiguous, or malformed fails closed; and
* only the archive's exact `bin/z3` member is written, so no other member can
  place a file outside the install directory.
"""

from __future__ import annotations

import hashlib
import io
import os
import urllib.request
import zipfile
from pathlib import Path

import pytest

import install_verus_z3

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


def archive_bytes(root: str = ROOT_NAME, extra: dict[str, bytes] | None = None) -> bytes:
    """Build a zip holding the solver under `root`, plus any extra members."""
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, mode="w") as bundle:
        bundle.writestr(f"{root}/bin/z3", SOLVER_BYTES)
        for member, payload in (extra or {}).items():
            bundle.writestr(member, payload)
    return buffer.getvalue()


def serving(payload: bytes) -> install_verus_z3.Fetcher:
    """Return a fetcher that writes `payload` and records the URL it saw."""
    seen: list[str] = []

    def fetch(url: str, destination: Path) -> None:
        """Serve the payload, recording the request."""
        seen.append(url)
        destination.write_bytes(payload)

    fetch.seen = seen  # type: ignore[attr-defined]
    return fetch


def refusing_fetcher(url: str, destination: Path) -> None:
    """A fetcher that must never be called; any call is a test failure."""
    raise AssertionError(f"the installer downloaded {url}")


class FakeResponse:
    """A minimal stand-in for the object `urlopen` returns."""

    def __init__(self, chunks: list[bytes]) -> None:
        """Serve `chunks` in order, then indicate the end of the stream."""
        self._chunks = iter(chunks)
        self.closed = False

    def read(self, size: int) -> bytes:
        """Return the next chunk, which ignores the requested size."""
        return next(self._chunks, b"")

    def __enter__(self) -> FakeResponse:
        """Enter the response's context."""
        return self

    def __exit__(self, *exception: object) -> bool:
        """Leave the context without suppressing anything."""
        self.closed = True
        return False


def replace_urlopen(monkeypatch: pytest.MonkeyPatch, chunks: list[bytes]) -> None:
    """Make `urllib.request.urlopen` serve `chunks` from a fresh response."""

    def urlopen(request: urllib.request.Request, *, timeout: int) -> FakeResponse:
        """Return a response over the prepared chunks."""
        assert timeout == install_verus_z3.REQUEST_TIMEOUT_SECONDS, (
            "the installer bounds the request timeout"
        )
        return FakeResponse(chunks)

    monkeypatch.setattr(urllib.request, "urlopen", urlopen)


def test_read_pin_reads_the_single_row(tmp_path: Path) -> None:
    """A well-formed pin yields its archive name and digest."""
    name, digest = install_verus_z3.read_pin(write_pin(tmp_path))

    assert name == ARCHIVE_NAME, "the archive name is read from the pin"
    assert digest == DIGEST, "the digest is read from the pin"


@pytest.mark.parametrize(
    "text",
    [
        pytest.param("", id="empty"),
        pytest.param(f"{DIGEST}  {ARCHIVE_NAME}\n" * 2, id="two-rows"),
        pytest.param(f"{DIGEST}\n", id="one-field"),
    ],
)
def test_read_pin_requires_exactly_one_row(tmp_path: Path, text: str) -> None:
    """A pin that is empty, doubled, or incomplete is refused."""
    path = tmp_path / "Z3_SHA256SUMS"
    path.write_text(text, encoding="utf-8")

    with pytest.raises(ValueError):
        install_verus_z3.read_pin(path)


@pytest.mark.parametrize(
    ("digest", "name"),
    [
        pytest.param("not-a-digest", ARCHIVE_NAME, id="malformed-digest"),
        pytest.param(DIGEST, "z3-latest.zip", id="unusable-name"),
        pytest.param(DIGEST, "not-a-z3-archive.tar.gz", id="not-a-release"),
    ],
)
def test_read_pin_rejects_fields_the_release_url_cannot_use(
    tmp_path: Path, digest: str, name: str
) -> None:
    """A malformed digest or an archive name the URL cannot use is refused."""
    with pytest.raises(ValueError):
        install_verus_z3.read_pin(write_pin(tmp_path, digest=digest, name=name))


def test_read_pin_reports_a_missing_file(tmp_path: Path) -> None:
    """A missing pin fails closed rather than falling back to `PATH`."""
    with pytest.raises(ValueError) as raised:
        install_verus_z3.read_pin(tmp_path / "absent")

    assert "absent" in str(raised.value), "the diagnostic names the missing pin"


def test_archive_url_names_the_pinned_release() -> None:
    """The URL is built from the version and platform in the archive name."""
    assert install_verus_z3.archive_url(ARCHIVE_NAME) == (
        f"{install_verus_z3.BASE_URL}/z3-{VERSION}/{ARCHIVE_NAME}"
    ), "the release URL holds the pinned version"


def test_archive_url_rejects_a_name_outside_the_pin_format() -> None:
    """A name the pattern does not match cannot produce a URL."""
    with pytest.raises(ValueError):
        install_verus_z3.archive_url("z3-latest.zip")


def test_verify_digest_accepts_matching_bytes(tmp_path: Path) -> None:
    """A file whose digest equals the pin verifies."""
    path = tmp_path / "archive.zip"
    path.write_bytes(b"archive-bytes")

    install_verus_z3.verify_digest(path, DIGEST)


def test_verify_digest_rejects_a_flipped_byte(tmp_path: Path) -> None:
    """A one-byte change to the archive is refused."""
    path = tmp_path / "archive.zip"
    path.write_bytes(b"archive-bytez")

    with pytest.raises(ValueError) as raised:
        install_verus_z3.verify_digest(path, DIGEST)

    assert "SHA-256" in str(raised.value), "the diagnostic names the digest check"


def test_extract_solver_writes_only_the_solver(tmp_path: Path) -> None:
    """The archive's `bin/z3` member is written, executable, and nothing else."""
    archive = tmp_path / "archive.zip"
    archive.write_bytes(archive_bytes(extra={f"{ROOT_NAME}/bin/z3.exe": b"other"}))
    destination = tmp_path / "out" / "z3"
    destination.parent.mkdir()

    install_verus_z3.extract_solver(archive, ARCHIVE_NAME, destination)

    assert destination.read_bytes() == SOLVER_BYTES, "the solver bytes are intact"
    assert list(destination.parent.iterdir()) == [destination], (
        "only the solver member is written"
    )
    if os.name == "posix":
        assert destination.stat().st_mode & 0o111, "the solver is executable"


def test_extract_solver_rejects_an_archive_without_the_solver(tmp_path: Path) -> None:
    """An archive whose member names differ from the pin is refused."""
    archive = tmp_path / "archive.zip"
    archive.write_bytes(archive_bytes(root="z3-4.16.0-other-platform"))
    destination = tmp_path / "z3"

    with pytest.raises(ValueError) as raised:
        install_verus_z3.extract_solver(archive, ARCHIVE_NAME, destination)

    assert f"{ROOT_NAME}/bin/z3" in str(raised.value), (
        "the diagnostic names the expected member"
    )
    assert not destination.exists(), "no binary is written"


def test_extract_solver_ignores_a_lookalike_outside_the_pinned_root(
    tmp_path: Path,
) -> None:
    """A solver member outside the pinned root is not the member the pin names."""
    archive = tmp_path / "archive.zip"
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, mode="w") as bundle:
        bundle.writestr(f"../{ROOT_NAME}/bin/z3", b"escape")
    archive.write_bytes(buffer.getvalue())
    destination = tmp_path / "z3"

    with pytest.raises(ValueError):
        install_verus_z3.extract_solver(archive, ARCHIVE_NAME, destination)

    assert not destination.exists(), "the lookalike member is not extracted"


def test_extract_solver_rejects_a_non_zip_file(tmp_path: Path) -> None:
    """A file that is not a zip archive is refused."""
    archive = tmp_path / "archive.zip"
    archive.write_bytes(b"not a zip file")
    destination = tmp_path / "z3"

    with pytest.raises(ValueError):
        install_verus_z3.extract_solver(archive, ARCHIVE_NAME, destination)

    assert not destination.exists(), "no binary is written"


def test_install_writes_the_solver_from_the_pin(tmp_path: Path) -> None:
    """A verified archive installs its solver and leaves no temporary files."""
    payload = archive_bytes()
    pins = write_pin(tmp_path, digest=hashlib.sha256(payload).hexdigest())
    install_dir = tmp_path / "z3-home"
    fetch = serving(payload)

    binary = install_verus_z3.install(pins=pins, install_dir=install_dir, fetch=fetch)

    assert binary == install_dir / "z3", "the binary lands at the install path"
    assert binary.read_bytes() == SOLVER_BYTES, "the solver bytes are intact"
    assert sorted(path.name for path in install_dir.iterdir()) == ["z3"], (
        "the temporary directory is removed"
    )
    assert getattr(fetch, "seen") == [
        install_verus_z3.archive_url(ARCHIVE_NAME)
    ], "the installer fetches the pinned archive"


def test_install_is_idempotent(tmp_path: Path) -> None:
    """An existing binary short-circuits before any download."""
    install_dir = tmp_path / "z3-home"
    install_dir.mkdir()
    existing = install_dir / "z3"
    existing.write_bytes(SOLVER_BYTES)

    binary = install_verus_z3.install(
        pins=write_pin(tmp_path), install_dir=install_dir, fetch=refusing_fetcher
    )

    assert binary == existing, "the installed binary is returned"
    assert binary.read_bytes() == SOLVER_BYTES, "the installed binary is untouched"


def test_install_rejects_a_digest_mismatch_and_leaves_no_binary(tmp_path: Path) -> None:
    """An archive that does not match the pin installs nothing."""
    pins = write_pin(tmp_path)
    install_dir = tmp_path / "z3-home"
    payload = archive_bytes() + b"tampered"

    with pytest.raises(ValueError):
        install_verus_z3.install(
            pins=pins, install_dir=install_dir, fetch=serving(payload)
        )

    assert not (install_dir / "z3").exists(), "no solver is installed"
    assert list(install_dir.iterdir()) == [], "the temporary directory is removed"


def test_download_writes_the_stream(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """A stream inside both bounds is written to the destination."""
    replace_urlopen(monkeypatch, [b"first", b"second", b""])
    destination = tmp_path / "archive.zip"

    install_verus_z3.download(install_verus_z3.archive_url(ARCHIVE_NAME), destination)

    assert destination.read_bytes() == b"firstsecond", "every chunk is written"


def test_download_refuses_a_stream_past_the_size_bound(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """A stream larger than the size bound is refused."""
    monkeypatch.setattr(install_verus_z3, "MAX_ARCHIVE_BYTES", 4)
    replace_urlopen(monkeypatch, [b"abc", b"def", b""])

    with pytest.raises(ValueError) as raised:
        install_verus_z3.download(
            install_verus_z3.archive_url(ARCHIVE_NAME), tmp_path / "archive.zip"
        )

    assert "size bound" in str(raised.value), "the diagnostic names the bound"


def test_download_refuses_a_stream_past_the_deadline(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """A stream that outlives the deadline is refused."""
    monkeypatch.setattr(install_verus_z3, "DOWNLOAD_DEADLINE_SECONDS", 0)
    replace_urlopen(monkeypatch, [b"abc", b""])

    with pytest.raises(ValueError) as raised:
        install_verus_z3.download(
            install_verus_z3.archive_url(ARCHIVE_NAME), tmp_path / "archive.zip"
        )

    assert "deadline" in str(raised.value), "the diagnostic names the deadline"