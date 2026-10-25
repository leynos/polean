#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.14"
# dependencies = []
# ///
"""Provision the pinned OPA and Lean tools after checking every byte.

The script reads :file:`toolchain-pins.json`, fetches each artefact into
``.polean/tools/downloads/``, and installs it under
``.polean/tools/<name>/<sha256>/`` only when both the byte count and the
SHA-256 equal the pin. Nothing is used before its digest matches, and no tool
is resolved through ``PATH``.

The script is idempotent. It holds an exclusive lock on
``.polean/tools/.lock`` for the whole run, deletes temporary siblings left by
an interrupted run, reuses a complete install, and re-hashes the installed OPA
binary on every run. ``rm -rf .polean/tools`` is always safe.

Downloading is a parameter rather than a module global so that the tests can
inject a fetcher without patching. This script launches no subprocess and uses
the standard library only.

Running ``make tools`` is the supported entry point. The digest pins are
trust-on-first-use for the Lean archive and an attested release for OPA; see
ADR-003.
"""

from __future__ import annotations

import fcntl
import hashlib
import json
import posixpath
import re
import shutil
import sys
import tarfile
import tempfile
import time
import urllib.request
from collections.abc import Callable, Iterable
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any

try:
    import compression.zstd  # noqa: F401  (imported for its clear failure)
except ImportError as error:  # pragma: no cover - depends on the interpreter
    message = (
        "Python 3.14 built with zstd support is required; "
        f"this interpreter cannot import compression.zstd: {error}"
    )
    raise SystemExit(message) from error

ROOT = Path(__file__).resolve().parents[1]
PINS_PATH = ROOT / "toolchain-pins.json"
TOOLS_DIR = ROOT / ".polean" / "tools"

SUPPORTED_FORMAT = 1
EXPECTED_PLATFORM = "x86_64-unknown-linux-gnu"
COMPLETE_MARKER = "COMPLETE"
CHUNK_SIZE = 1024 * 1024

# A socket that goes quiet for this long is treated as a failed download. The
# whole run holds the tools lock, so an unbounded read would block every later
# invocation, and in CI it would hold the job until its timeout.
REQUEST_TIMEOUT_SECONDS = 60

# A per-operation socket timeout does not bound the whole transfer: a server
# that dribbles one byte just inside `REQUEST_TIMEOUT_SECONDS` keeps the read
# returning for ever. The deadline is what makes "this download finishes or
# fails" true regardless of how the bytes arrive.
DOWNLOAD_DEADLINE_SECONDS = 30 * 60

_DIGEST = re.compile(r"\A[0-9a-f]{64}\Z")
_TOP_LEVEL_KEYS = frozenset({"format", "platform", "opa", "lean"})
_TOOL_KEYS = frozenset({"version", "url", "sha256", "size", "kind"})
_KINDS = {"opa": "binary", "lean": "tar.zst"}

Fetcher = Callable[[str, Path, int], None]

# The one call the default fetcher makes, named so the fetch test can supply a
# stand-in through the same parameter the install functions already take rather
# than patching the standard library.
Opener = Callable[..., Any]


class ProvisionError(Exception):
    """A provisioning failure the contributor must act on."""


class PinFileError(ProvisionError):
    """The pin file is missing, malformed, or widens the known schema."""


class SizeMismatch(ProvisionError):
    """A fetched artefact does not have the pinned byte count."""


class DigestMismatch(ProvisionError):
    """A fetched or installed artefact does not have the pinned SHA-256."""


class ArchiveEscape(ProvisionError):
    """An archive member falls outside the pinned top-level directory."""


class DownloadTooLarge(ProvisionError):
    """A download exceeded the pinned size and was abandoned."""


class DownloadTimeout(ProvisionError):
    """A download did not finish inside the overall deadline."""


def _present(path: Path) -> bool:
    """Whether `path` exists, letting an environment failure propagate.

    `Path.exists` answers `False` for a permission error as well as for a
    genuinely absent path. Every caller here treats `False` as "not installed"
    and deletes or re-downloads; answering that for an unreadable tree would
    destroy a complete install because the contributor cannot read it. `stat`
    separates the one answer that means "absent" from every failure to find
    out.
    """
    try:
        path.stat()
    except FileNotFoundError:
        return False
    return True


@dataclass(frozen=True)
class ToolPin:
    """One pinned artefact."""

    name: str
    version: str
    url: str
    sha256: str
    size: int
    kind: str
    root: str | None
    identity: str | None


@dataclass(frozen=True)
class Pins:
    """The whole pin file."""

    format: int
    platform: str
    opa: ToolPin
    lean: ToolPin


@dataclass(frozen=True)
class Installed:
    """Where the verified tools ended up."""

    opa: Path
    lean_root: Path


def load_pins(path: Path) -> Pins:
    """Read and validate the pin file.

    The schema is closed: unknown keys are rejected rather than ignored, so a
    typo cannot leave a security-relevant field unchecked. A file that cannot
    be read as JSON, is not an object, has an unknown or missing key, or
    carries a malformed value raises `PinFileError`.
    """
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise PinFileError(f"{path}: cannot be read as JSON pins: {error}") from error
    if not isinstance(document, dict):
        raise PinFileError(f"{path}: the pin file must be a JSON object")
    _expect_exact_keys(document, _TOP_LEVEL_KEYS, str(path))
    if document["format"] != SUPPORTED_FORMAT:
        raise PinFileError(
            f"{path}: unsupported format {document['format']!r}; "
            f"this script understands format {SUPPORTED_FORMAT}"
        )
    if document["platform"] != EXPECTED_PLATFORM:
        raise PinFileError(
            f"{path}: platform {document['platform']!r} is not "
            f"{EXPECTED_PLATFORM!r}; provisioning only this platform is tested"
        )
    return Pins(
        format=document["format"],
        platform=document["platform"],
        opa=_load_tool("opa", document["opa"]),
        lean=_load_tool("lean", document["lean"]),
    )


def _load_tool(name: str, raw: Any) -> ToolPin:
    """Validate one tool entry from the pin file."""
    if not isinstance(raw, dict):
        raise PinFileError(f"{name}: the pin must be a JSON object")
    expected = _TOOL_KEYS | ({"root", "identity"} if name == "lean" else set())
    _expect_exact_keys(raw, expected, name)
    version = _require_string(raw["version"], f"{name}.version")
    url = _require_string(raw["url"], f"{name}.url")
    if not url.startswith("https://"):
        raise PinFileError(f"{name}.url: must be an https:// URL, found {url!r}")
    sha256 = _require_string(raw["sha256"], f"{name}.sha256")
    if not _DIGEST.fullmatch(sha256):
        raise PinFileError(
            f"{name}.sha256: expected 64 lowercase hex characters, found {sha256!r}"
        )
    size = raw["size"]
    if not isinstance(size, int) or isinstance(size, bool) or size <= 0:
        raise PinFileError(f"{name}.size: expected a positive integer, found {size!r}")
    kind = _require_string(raw["kind"], f"{name}.kind")
    if kind != _KINDS[name]:
        raise PinFileError(f"{name}.kind: expected {_KINDS[name]!r}, found {kind!r}")
    root = None
    identity = None
    if name == "lean":
        root = _require_string(raw["root"], f"{name}.root")
        if version not in root:
            raise PinFileError(
                f"{name}.root: {root!r} does not embed the pinned version "
                f"{version!r}; the archive layout would change silently"
            )
        identity = _require_string(raw["identity"], f"{name}.identity")
        if version not in identity:
            raise PinFileError(
                f"{name}.identity: {identity!r} does not embed the pinned "
                f"version {version!r}; the reported identity would drift from "
                "the version it is checked against"
            )
    return ToolPin(
        name=name,
        version=version,
        url=url,
        sha256=sha256,
        size=size,
        kind=kind,
        root=root,
        identity=identity,
    )


def _expect_exact_keys(raw: dict[str, Any], expected: frozenset[str] | set[str], where: str) -> None:
    """Require exactly the expected keys, naming every difference."""
    unknown = sorted(set(raw) - expected)
    if unknown:
        raise PinFileError(f"{where}: unknown key(s) {', '.join(unknown)}")
    missing = sorted(expected - set(raw))
    if missing:
        raise PinFileError(f"{where}: missing key(s) {', '.join(missing)}")


def _require_string(value: Any, where: str) -> str:
    """Require a non-empty string."""
    if not isinstance(value, str) or not value:
        raise PinFileError(f"{where}: expected a non-empty string, found {value!r}")
    return value


def install_tools(pins_path: Path, tools_dir: Path, fetcher: Fetcher) -> Installed:
    """Install every pinned tool, reusing a complete install.

    A `ProvisionError` subclass is raised when the pin file is invalid, a
    fetched artefact is the wrong size or digest, or an archive member falls
    outside the pinned root.
    """
    pins = load_pins(pins_path)
    tools_dir.mkdir(parents=True, exist_ok=True)
    lock_path = tools_dir / ".lock"
    with lock_path.open("a+", encoding="utf-8") as lock:
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
        downloads = tools_dir / "downloads"
        downloads.mkdir(exist_ok=True)
        _remove_stale(tools_dir, downloads)
        opa = _install_opa(pins.opa, tools_dir, downloads, fetcher)
        lean_root = _install_lean(pins.lean, tools_dir, downloads, fetcher)
    return Installed(opa=opa, lean_root=lean_root)


def _remove_stale(tools_dir: Path, downloads: Path) -> None:
    """Delete temporary siblings an interrupted run may have left behind."""
    for entry in _entries(tools_dir, prefixes=(".install-",)):
        shutil.rmtree(entry, ignore_errors=True)
    for entry in _entries(downloads, prefixes=(".part-",)):
        entry.unlink(missing_ok=True)


def _entries(directory: Path, prefixes: tuple[str, ...]) -> Iterable[Path]:
    """Yield the direct children whose name begins with one of `prefixes`."""
    for entry in directory.iterdir():
        if entry.name.startswith(prefixes):
            yield entry


def _install_opa(pin: ToolPin, tools_dir: Path, downloads: Path, fetcher: Fetcher) -> Path:
    """Install the OPA binary, re-hashing an existing install first."""
    final = tools_dir / "opa" / pin.sha256 / "opa"
    if _present(final):
        _verify_digest(final, pin)
        return final
    artefact = _ensure_downloaded(pin, downloads, fetcher)
    parent = tools_dir / "opa"
    parent.mkdir(exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".install-opa-", dir=tools_dir))
    try:
        binary = staging / "opa"
        shutil.copyfile(artefact, binary)
        _verify_digest(binary, pin)
        binary.chmod(0o755)
        _write_marker(staging, pin.sha256)
        target = parent / pin.sha256
        staging.rename(target)
    except ProvisionError:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    return target / "opa"


def _install_lean(pin: ToolPin, tools_dir: Path, downloads: Path, fetcher: Fetcher) -> Path:
    """Install the Lean tree, refusing any member outside the pinned root."""
    final = tools_dir / "lean" / pin.sha256
    if _is_complete(final, pin):
        return final
    if _present(final):
        shutil.rmtree(final)
    artefact = _ensure_downloaded(pin, downloads, fetcher)
    _check_archive(artefact, pin)
    staging = Path(tempfile.mkdtemp(prefix=".install-lean-", dir=tools_dir))
    try:
        _extract_archive(artefact, staging)
        inner = staging / _require_root(pin)
        _write_marker(inner, pin.sha256)
        parent = tools_dir / "lean"
        parent.mkdir(exist_ok=True)
        target = parent / pin.sha256
        if _present(target):
            shutil.rmtree(target)
        inner.rename(target)
    finally:
        shutil.rmtree(staging, ignore_errors=True)
    return final


def _require_root(pin: ToolPin) -> str:
    """The pinned archive root, which validation has already required."""
    if pin.root is None:
        raise PinFileError(f"{pin.name}: a tar.zst pin needs a root")
    return pin.root


def _is_complete(directory: Path, pin: ToolPin) -> bool:
    """Whether `directory` holds a completed install of `pin`.

    A marker that cannot be inspected is an environment failure, not an
    incomplete install, so `OSError` propagates. `Path.is_file` would answer
    `False` for a permission error too, sending the caller down the repair path
    and deleting a tree that may be complete; a re-download of 600 MiB is the
    wrong response to an unreadable directory. `_present` distinguishes the one
    answer that means "not installed" from every failure to find out.
    """
    marker = directory / COMPLETE_MARKER
    if not _present(marker):
        return False
    recorded = marker.read_text(encoding="utf-8").strip()
    return recorded == pin.sha256


def _write_marker(directory: Path, digest: str) -> None:
    """Record that `directory` holds a completed install of `digest`."""
    (directory / COMPLETE_MARKER).write_text(f"{digest}\n", encoding="utf-8")


def _ensure_downloaded(pin: ToolPin, downloads: Path, fetcher: Fetcher) -> Path:
    """Return a verified local copy of `pin`, fetching it if necessary.

    A cached download is re-verified. A corrupt cached copy is discarded and
    fetched again rather than trusted.
    """
    cached = downloads / f"{pin.name}-{pin.sha256}"
    if _present(cached):
        try:
            _verify(cached, pin)
        except (SizeMismatch, DigestMismatch):
            cached.unlink()
        else:
            return cached
    partial = downloads / f".part-{pin.sha256}"
    partial.unlink(missing_ok=True)
    try:
        fetcher(pin.url, partial, pin.size)
        _verify(partial, pin)
        partial.replace(cached)
    finally:
        partial.unlink(missing_ok=True)
    return cached


def _verify(path: Path, pin: ToolPin) -> None:
    """Require `path` to have the pinned byte count and digest."""
    actual_size = path.stat().st_size
    if actual_size != pin.size:
        raise SizeMismatch(
            f"{pin.name}: {path} has {actual_size} bytes; "
            f"the pin records {pin.size}. Delete {path} and retry."
        )
    _verify_digest(path, pin)


def _verify_digest(path: Path, pin: ToolPin) -> None:
    """Require `path` to have the pinned SHA-256."""
    actual = _sha256(path)
    if actual != pin.sha256:
        raise DigestMismatch(
            f"{pin.name}: {path} has SHA-256 {actual}; the pin records "
            f"{pin.sha256}. Nothing was installed."
        )


def _sha256(path: Path) -> str:
    """The lower-case hexadecimal SHA-256 of a file."""
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(CHUNK_SIZE):
            digest.update(chunk)
    return digest.hexdigest()


def _check_archive(path: Path, pin: ToolPin) -> None:
    """Refuse the archive unless every member stays under the pinned root.

    A leading component equal to the root is not enough. `root/../escape`
    starts with the root and is neutralized by the `data` filter during
    extraction, so the member would pass this gate and then disappear into the
    staging directory rather than being refused. Any `..` component, and any
    link whose target leaves the root, is therefore rejected here.
    """
    root = _require_root(pin)
    with tarfile.open(path, mode="r:zst") as archive:
        for member in archive:
            parts = PurePosixPath(member.name).parts
            if (
                PurePosixPath(member.name).is_absolute()
                or not parts
                or parts[0] != root
                or ".." in parts
            ):
                raise _escape(pin, member.name, root, path)
            if member.issym() or member.islnk():
                _require_link_inside(member, root, pin, path)


def _require_link_inside(
    member: tarfile.TarInfo, root: str, pin: ToolPin, path: Path
) -> None:
    """Refuse a link whose target would resolve outside the pinned root."""
    target = PurePosixPath(member.linkname)
    # A symbolic link's target is relative to its own directory; a hard link's
    # name is the target's path from the archive root.
    if member.issym():
        base = PurePosixPath(*PurePosixPath(member.name).parts[:-1])
    else:
        base = PurePosixPath()
    resolved = posixpath.normpath((base / target).as_posix())
    if target.is_absolute() or not (resolved == root or resolved.startswith(f"{root}/")):
        raise _escape(pin, f"{member.name} -> {member.linkname}", root, path)


def _escape(pin: ToolPin, member: str, root: str, path: Path) -> ArchiveEscape:
    """The one diagnostic for a member that leaves the pinned root."""
    return ArchiveEscape(
        f"{pin.name}: archive member {member!r} is outside "
        f"{root!r}; refusing to extract. The archive at {path} is "
        "not the pinned layout."
    )


def _extract_archive(path: Path, destination: Path) -> None:
    """Extract with the standard library's `data` filter."""
    with tarfile.open(path, mode="r:zst") as archive:
        archive.extractall(destination, filter="data")


def fetch_url(url: str, destination: Path, limit: int) -> None:
    """Download `url` to `destination`, abandoning it beyond `limit` bytes.

    `DownloadTooLarge` is raised as soon as the stream exceeds `limit`, and
    `urllib` errors propagate for a network or HTTP failure. A read that blocks
    for longer than `REQUEST_TIMEOUT_SECONDS`, or a transfer that runs past
    `DOWNLOAD_DEADLINE_SECONDS` in total, fails the same way rather than
    waiting indefinitely while the tools lock is held.
    """
    download(url, destination, limit, opener=urllib.request.urlopen)


def download(
    url: str,
    destination: Path,
    limit: int,
    *,
    opener: Opener,
    deadline_seconds: float = DOWNLOAD_DEADLINE_SECONDS,
) -> None:
    """`fetch_url` with the HTTP call supplied, so a test can observe it.

    The opener is called as `opener(request, timeout=...)` and must return a
    readable, context-manageable response. Keeping it a keyword parameter of a
    real production path is what lets the timeout be tested without replacing
    anything in the standard library.

    `deadline_seconds` bounds the whole transfer, not each `read`. The check
    runs before every read, so a stream that keeps trickling cannot extend the
    download past the deadline; the failure is a `ProvisionError` because a
    pinned asset that cannot be fetched in `deadline_seconds` is a pin or
    network fault the contributor must act on, not an internal error.
    """
    request = urllib.request.Request(
        url, headers={"User-Agent": "polean-provisioner/1"}
    )
    total = 0
    expire_at = time.monotonic() + deadline_seconds
    with (
        opener(request, timeout=REQUEST_TIMEOUT_SECONDS) as response,
        destination.open("wb") as handle,
    ):
        while True:
            if time.monotonic() >= expire_at:
                raise DownloadTimeout(
                    f"{url}: the transfer did not finish within "
                    f"{deadline_seconds:g} seconds; the pin file and the "
                    "upstream asset disagree, or the network is too slow."
                )
            chunk = response.read(CHUNK_SIZE)
            if not chunk:
                return
            total += len(chunk)
            if total > limit:
                raise DownloadTooLarge(
                    f"{url}: the stream exceeds the pinned {limit} bytes; "
                    "the pin file and the upstream asset disagree."
                )
            handle.write(chunk)


def main(
    pins_path: Path = PINS_PATH,
    tools_dir: Path = TOOLS_DIR,
    fetcher: Fetcher = fetch_url,
) -> int:
    """Install the pinned tools and print where they are.

    A `ProvisionError` is the provisioner's own diagnosis. An `OSError` is the
    environment failing instead: an unreachable network, a full disk, or a
    directory the contributor cannot write. Both are reported as one line with
    exit status 1, because a traceback tells a contributor nothing about which
    of the two happened or what to try next.
    """
    try:
        installed = install_tools(pins_path, tools_dir, fetcher)
    except (ProvisionError, OSError) as error:
        print(f"provisioning failed: {error}", file=sys.stderr)
        return 1
    print(f"opa:  {installed.opa}")
    print(f"lean: {installed.lean_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
