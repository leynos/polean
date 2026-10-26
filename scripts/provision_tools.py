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
import http.client
import json
import posixpath
import re
import shutil
import socket as socketmod
import sys
import tarfile
import tempfile
import threading
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
# than patching the standard library. It is called as
# `opener(request, timeout=..., watch=...)`, and every socket it opens must be
# registered with `watch`: the deadline cannot reach a socket it was not told
# about, and a read blocked on that socket cannot be interrupted from outside.
Opener = Callable[..., Any]

# A monotonic-seconds source, injected so a deadline can be driven past in a
# test without patching the `time` module that every other caller shares.
Clock = Callable[[], float]

# Builds the watchdog for one transfer. Injected so a test can observe the
# sockets the **real** opener registers, rather than replacing the module
# attribute `fetch_url` resolves and so being blind to a wiring change.
DeadlineFactory = Callable[[float, Clock], "_Deadline"]


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


class _Deadline:
    """Interrupt every watched socket when the deadline passes.

    Checking the clock between reads is not enough on its own, for three
    separate reasons, and one mechanism covers all of them.

    A single `read1` can block inside `_get_chunk_left`, whose `readline` for a
    chunk-size line makes as many system calls as it needs — CPython says so in
    a comment there — and a plain socket read can block for the whole socket
    timeout. Before any of that, the opener itself blocks: `urlopen` connects,
    sends the request, and parses the status line and headers inside
    `getresponse()`, so a server that dribbles header bytes slowly holds the
    call open before a response object even exists to attach a timer to. No
    code of ours runs during any of those, so a timer is what bounds them, and
    it has to be armed before the opener is called.

    `socket.shutdown` is what actually interrupts a read already in progress:
    it wakes the blocked call in the same process, which returns short or
    raises. Closing the response from another thread does not, because the read
    already holds the file — measured, not assumed.

    Sockets are registered by `watch` as each one is created. A socket the
    caller never registers cannot be interrupted, which is why the production
    opener registers its connection before it is used.

    The returned flag records that the timer fired, so the caller can report a
    deadline rather than the socket error the shutdown provokes. The timer is
    always cancelled by the caller, and is a daemon so a stuck read cannot keep
    the process alive at exit.
    """

    def __init__(self, expire_at: float, clock: Clock) -> None:
        """Record the deadline, the clock to read it from, and empty registries."""
        self._expire_at = expire_at
        self._clock = clock
        self._sockets: list[Any] = []
        self._lock = threading.Lock()
        self._fired: list[bool] = []

    def watch(self, sock: Any) -> None:
        """Register `sock` so the deadline can shut it down.

        Registration is locked against the timer: a socket that connects just
        as the deadline passes must either be seen by the timer or see the
        fired flag, never neither.
        """
        with self._lock:
            self._sockets.append(sock)
            already = bool(self._fired)
        if already:
            self._abort_one(sock)

    def fired(self) -> bool:
        """Whether the deadline has passed and the watchdog has run."""
        with self._lock:
            return bool(self._fired)

    def _abort_one(self, sock: Any) -> None:
        """Shut one socket down, treating an already-closed socket as already safe."""
        try:
            sock.shutdown(socketmod.SHUT_RDWR)
        except OSError:
            # Already closed, or never connected: the read cannot outlive the
            # deadline either way, which is all this has to ensure.
            pass

    def _abort(self) -> None:
        """Mark the deadline fired, then shut every registered socket down."""
        with self._lock:
            self._fired.append(True)
            sockets = list(self._sockets)
        for sock in sockets:
            self._abort_one(sock)

    def start(self) -> threading.Timer:
        """Arm the watchdog and return the timer, for the caller to cancel."""
        delay = max(self._expire_at - self._clock(), 0)
        timer = threading.Timer(delay, self._abort)
        timer.daemon = True
        timer.start()
        return timer


def _watched_connection(base: type[http.client.HTTPConnection]) -> type:
    """A connection class that hands its socket to the watchdog once connected.

    Registration happens at the end of `connect`, on `self.sock`. That is the
    only socket that can usefully be shut down, and for HTTPS it is not the one
    the TCP layer produced: `HTTPSConnection.connect` replaces `self.sock` with
    the TLS socket from `wrap_socket`, and the socket underneath is closed by
    that call — measured, its `fileno()` is `-1` afterwards and `shutdown`
    raises `EBADF`. Registering at creation would therefore register a dead
    handle for every HTTPS download, which is exactly the case the watchdog
    exists for.

    Registering after `connect` rather than after `getresponse` returns still
    covers the header parse, because that parse happens inside
    `getresponse()` — after `connect` has returned and this registration has
    run. What lies outside is name resolution and the TCP and TLS handshakes
    themselves: each handshake read is bounded by the clipped socket timeout,
    but `getaddrinfo` is not bounded by anything here.
    """

    class Watched(base):  # type: ignore[misc, valid-type]
        """A connection class whose finished socket is registered on connect."""

        def __init__(
            self, *args: Any, watch: Callable[[Any], None], **kwargs: Any
        ) -> None:
            """Record the callback that registers each connected socket."""
            super().__init__(*args, **kwargs)
            self._polean_watch = watch

        def connect(self) -> None:
            """Connect, then hand the finished socket to the watchdog."""
            super().connect()
            # `self.sock` is the finished socket: for HTTPS this is the TLS
            # wrapper, because `HTTPSConnection.connect` assigns it last.
            self._polean_watch(self.sock)

    return Watched


def _watched_opener(watch: Callable[[Any], None]) -> Any:
    """An opener whose every HTTP and HTTPS connection is watched.

    `build_opener` drops a default handler when it is given a subclass of it,
    so passing these two replaces the built-in pair rather than adding to them:
    no request can reach the network on a socket the watchdog was not told
    about. Redirects go back through these handlers, so the connection a
    redirect opens is watched too.
    """
    http_class = _watched_connection(http.client.HTTPConnection)
    https_class = _watched_connection(http.client.HTTPSConnection)

    class HTTPHandler(urllib.request.HTTPHandler):
        """Replace the default HTTP handler with watched connections."""

        def http_open(self, req: Any) -> Any:
            """Open an HTTP request on a watched connection."""
            return self.do_open(http_class, req, watch=watch)

    class HTTPSHandler(urllib.request.HTTPSHandler):
        """Replace the default HTTPS handler with watched TLS connections."""

        def https_open(self, req: Any) -> Any:
            """Open an HTTPS request on a watched TLS connection."""
            return self.do_open(https_class, req, context=self._context, watch=watch)

    return urllib.request.build_opener(HTTPHandler, HTTPSHandler)


def watched_opener(
    request: Any, *, timeout: float, watch: Callable[[Any], None]
) -> Any:
    """The production opener: every socket it opens registers with `watch`.

    It is module-level, rather than a closure inside `fetch_url`, so a test can
    drive the *shipped* opener through `download` while supplying something
    other than the default watchdog. A closure would force such a test to
    reproduce the wiring it is meant to be checking.
    """
    return _watched_opener(watch).open(request, timeout=timeout)


def fetch_url(
    url: str,
    destination: Path,
    limit: int,
    *,
    deadline_factory: DeadlineFactory = _Deadline,
) -> None:
    """Download `url` to `destination`, abandoning it beyond `limit` bytes.

    `DownloadTooLarge` is raised as soon as the stream exceeds `limit`, and
    `urllib` errors propagate for a network or HTTP failure. A read that blocks
    for longer than `REQUEST_TIMEOUT_SECONDS`, or a transfer that runs past
    `DOWNLOAD_DEADLINE_SECONDS` in total, fails as `DownloadTimeout` rather
    than waiting indefinitely while the tools lock is held.

    `deadline_factory` is threaded through to `download` so a test can watch
    the real opener register its socket, without replacing any module
    attribute. Production callers use the default.
    """

    download(
        url,
        destination,
        limit,
        opener=watched_opener,
        deadline_factory=deadline_factory,
    )


def download(
    url: str,
    destination: Path,
    limit: int,
    *,
    opener: Opener,
    deadline_seconds: float = DOWNLOAD_DEADLINE_SECONDS,
    clock: Clock = time.monotonic,
    deadline_factory: DeadlineFactory = _Deadline,
) -> None:
    """`fetch_url` with the HTTP call and clock supplied, so a test sees both.

    The opener is called as `opener(request, timeout=..., watch=...)` and must
    return a readable, context-manageable response, passing every socket it
    creates to `watch`. Keeping it a keyword parameter of a real production
    path is what lets the timeout be tested without replacing anything in the
    standard library. `clock` is injectable for the same reason: a test that
    wants the deadline to pass must be able to say so without patching the
    `time` module out from under every other caller.

    `deadline_factory` is the same idea for the watchdog itself. It exists so
    a test can observe what reaches `watch` through the real production path
    without replacing the module attribute the shipped code calls: patching
    `provision_tools._Deadline` would pass even if `fetch_url` stopped routing
    the real opener's registrations here, which is the very thing such a test
    is for.

    `deadline_seconds` bounds the whole transfer, not each read. The bound
    works because the loop reads with `read1`, which makes at most one
    underlying system call per iteration, so control returns to the deadline
    check as soon as one byte arrives. Reading with `read` instead would
    defeat the check: `BufferedReader.read(n)` keeps issuing system calls until
    it has `n` bytes, so a server dribbling a byte just inside
    `REQUEST_TIMEOUT_SECONDS` would hold a single call open long past the
    deadline. A read that has already begun can still block for its socket
    timeout, which is why that timeout is itself clipped to
    `deadline_seconds`.

    The watchdog covers what the between-reads check cannot: the request write
    and the header parse, which happen inside the opener and return no control
    to this function. It is armed before the opener is called, so a server that
    never finishes its headers is interrupted rather than waited out. It cannot
    cover name resolution or the TCP and TLS handshakes, because no socket
    exists to register until those finish; each handshake read is bounded by
    the clipped socket timeout, and `getaddrinfo` is bounded only by the
    resolver.

    The failure is a `ProvisionError` because a pinned asset that cannot be
    fetched in `deadline_seconds` is a pin or network fault the contributor
    must act on, not an internal error.
    """
    request = urllib.request.Request(
        url, headers={"User-Agent": "polean-provisioner/1"}
    )
    total = 0
    expire_at = clock() + deadline_seconds
    # A read that has already begun blocks for its socket timeout before it
    # can return, so the per-operation timeout must not outlive the deadline.
    socket_timeout = min(REQUEST_TIMEOUT_SECONDS, deadline_seconds)
    watchdog = deadline_factory(expire_at, clock)

    def expired() -> DownloadTimeout:
        """The refusal raised when the transfer outlives the deadline."""
        return DownloadTimeout(
            f"{url}: the transfer did not finish within "
            f"{deadline_seconds:g} seconds; the pin file and the "
            "upstream asset disagree, or the network is too slow."
        )

    timer = watchdog.start()
    try:
        with (
            opener(request, timeout=socket_timeout, watch=watchdog.watch) as response,
            destination.open("wb") as handle,
        ):
            while True:
                if clock() >= expire_at:
                    raise expired()
                chunk = response.read1(CHUNK_SIZE)
                if not chunk:
                    # An empty read means end of stream, not only a clean one.
                    # The watchdog implements the deadline by shutting the
                    # socket down, and a shutdown makes `read1` return `b""`
                    # — measured — so a body cut short at the deadline, of a
                    # promised `Content-Length` or of chunked framing, arrives
                    # here looking exactly like a complete one. Reporting the
                    # deadline first is what keeps a truncated download from
                    # being accepted as whole.
                    if watchdog.fired() or clock() >= expire_at:
                        raise expired()
                    return
                total += len(chunk)
                if total > limit:
                    raise DownloadTooLarge(
                        f"{url}: the stream exceeds the pinned {limit} bytes; "
                        "the pin file and the upstream asset disagree."
                    )
                handle.write(chunk)
    except ProvisionError:
        # A guard's own refusal is already the right answer; only a foreign
        # exception can be the deadline's doing and need relabelling.
        raise
    except Exception:
        # One guard covers the whole transfer, the opener included, because
        # the watchdog covers the opener: a connect or header parse cut off at
        # the deadline surfaces from the `with` itself, before there is a
        # response to read from. Every way the deadline can end the transfer
        # arrives here as a foreign exception whose type depends on where the
        # cut landed — `OSError` for a plain read, `IncompleteRead` in chunk
        # framing, a plain timeout for a read the clipped socket timeout ended.
        # Once the deadline has passed it is the cause in all three cases, so
        # it is what gets reported.
        if watchdog.fired() or clock() >= expire_at:
            raise expired() from None
        raise
    finally:
        timer.cancel()


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
