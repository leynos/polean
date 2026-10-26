"""Tests of download limits, deadlines, and socket watchdog wiring."""

from __future__ import annotations

import http.client
import io
import shutil
import socket
import ssl
import subprocess
import threading
import time
from collections.abc import Callable, Iterator
from pathlib import Path

import pytest

import provision_tools

OPA_BYTES = b"opa-binary-placeholder"


class RecordedSocket:
    """Record watchdog shutdowns before forwarding them to the socket."""

    def __init__(self, sock: socket.socket, interrupted: list[bool]) -> None:
        """Keep the wrapped socket and shutdown log."""
        self._sock = sock
        self._interrupted = interrupted

    def shutdown(self, how: int) -> None:
        """Record and perform the shutdown."""
        self._interrupted.append(True)
        self._sock.shutdown(how)


@pytest.fixture
def stalled_server() -> Iterator[
    Callable[[Callable[[socket.socket, threading.Event], None]], int]
]:
    """Start a local server and release and join it after each test."""
    release = threading.Event()
    threads: list[threading.Thread] = []

    def start(behaviour: Callable[[socket.socket, threading.Event], None]) -> int:
        """Run one server with the supplied response behaviour."""
        server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server.bind(("127.0.0.1", 0))
        server.listen(1)
        port = server.getsockname()[1]

        def serve() -> None:
            """Serve one connection and close the listener."""
            try:
                with server:
                    conn, _ = server.accept()
                    with conn:
                        behaviour(conn, release)
            except OSError:
                if not release.is_set():
                    raise

        thread = threading.Thread(target=serve, daemon=True)
        thread.start()
        threads.append(thread)
        return port

    yield start
    release.set()
    for thread in threads:
        thread.join(timeout=5)


def test_fetch_url_passes_a_timeout(tmp_path: Path) -> None:
    """A stalled connection cannot hold the tools lock indefinitely."""
    observed: dict[str, object] = {}

    class Response(io.BytesIO):
        """A response body that ends immediately."""

    def opener(request: object, *, timeout: float, watch: object) -> Response:
        """Record the timeout, then return a body that ends at once."""
        observed["timeout"] = timeout
        return Response(OPA_BYTES)

    destination = tmp_path / "opa"

    provision_tools.download(
        "https://example.invalid/opa", destination, len(OPA_BYTES), opener=opener
    )

    assert observed["timeout"] == provision_tools.REQUEST_TIMEOUT_SECONDS, (
        "the download must bound how long a stalled read may block"
    )
    assert destination.read_bytes() == OPA_BYTES, "the fetched bytes are written"


def test_download_refuses_a_stream_over_the_limit(tmp_path: Path) -> None:
    """The size guard is in `download`, not only in the test fetcher.

    An injected fetcher that raises `DownloadTooLarge` itself proves nothing
    about the production guard: deleting `total > limit` from `download` would
    leave such a test passing. This test therefore drives the real `download`
    with an opener that yields one byte more than the limit, and asserts the
    three things the guard owes: the refusal, no excess byte on disk, and no
    completed file left behind.
    """
    limit = len(OPA_BYTES)
    oversize = OPA_BYTES + b"X"
    chunks = [oversize[: limit // 2], oversize[limit // 2 :]]

    class Response(io.BytesIO):
        """A response body served in two chunks."""

    def opener(request: object, *, timeout: float, watch: object) -> Response:
        """Return the oversize body in two chunks."""
        return Response(b"".join(chunks))

    destination = tmp_path / "opa"

    with pytest.raises(provision_tools.DownloadTooLarge) as raised:
        provision_tools.download(
            "https://example.invalid/opa", destination, limit, opener=opener
        )

    assert str(limit) in str(raised.value), "the diagnostic names the pinned size"
    written = destination.read_bytes() if destination.exists() else b""
    assert len(written) <= limit, "the guard refuses before writing past the limit"
    assert oversize not in (written,), "the excess byte is never written"


def test_download_stops_at_the_overall_deadline(tmp_path: Path) -> None:
    """A trickling stream cannot run past the deadline.

    A per-read socket timeout is not a bound on the transfer: a server that
    returns one byte just inside `REQUEST_TIMEOUT_SECONDS` keeps the read
    returning for ever, and the run holds the tools lock the whole time. This
    drives the real `download` with a response that never ends and a zero
    deadline, and asserts it refuses instead of looping.
    """
    reads: list[int] = []

    class Trickle(io.BytesIO):
        """A body that yields one byte per read, without end."""

        def read1(self, size: int = -1) -> bytes:
            """Yield one more byte, however many are asked for."""
            reads.append(size)
            return b"x"

    def opener(request: object, *, timeout: float, watch: object) -> Trickle:
        """Return the endless one-byte body."""
        return Trickle()

    destination = tmp_path / "opa"

    with pytest.raises(provision_tools.DownloadTimeout) as refused:
        provision_tools.download(
            "https://example.invalid/opa",
            destination,
            len(OPA_BYTES),
            opener=opener,
            deadline_seconds=0,
        )

    assert "did not finish within" in str(refused.value), (
        "the failure names the deadline rather than a digest or size fault"
    )
    assert destination.read_bytes() == b"", "no trickled byte is kept"


def test_download_deadline_is_checked_before_each_read(tmp_path: Path) -> None:
    """The deadline is not merely a first-iteration guard.

    A guard checked only before the loop would let an endless stream through.
    This response ends after a few chunks, with a clock that advances past the
    deadline part-way, so passing proves the check runs inside the loop.
    """
    now = [0.0]

    class Chunks(io.BytesIO):
        """Three chunks, then end of stream."""

        def __init__(self) -> None:
            """Start the body at three bytes."""
            super().__init__(b"a" * 3)

        def read1(self, size: int = -1) -> bytes:
            """Advance the clock, then yield one byte."""
            now[0] += 0.4
            return super().read1(1)

    def opener(request: object, *, timeout: float, watch: object) -> Chunks:
        """Return the three-chunk body with an advancing clock."""
        return Chunks()

    with pytest.raises(provision_tools.DownloadTimeout):
        provision_tools.download(
            "https://example.invalid/opa",
            tmp_path / "opa",
            len(OPA_BYTES),
            opener=opener,
            deadline_seconds=1.0,
            clock=lambda: now[0],
        )


def test_download_reads_with_read1_so_a_blocking_read_cannot_hide_the_deadline(
    tmp_path: Path,
) -> None:
    """Each read makes one syscall, so the deadline is rechecked between them.

    `BufferedReader.read(n)` keeps issuing system calls until it has `n` bytes,
    so a single `read(CHUNK_SIZE)` on a dribbling connection can outlive the
    whole deadline and the check before it never runs. `read1` returns after at
    most one system call, which is what lets the loop observe a clock that has
    passed the deadline *during* the transfer. This response would have
    completed if `read` were used: it ends after four bytes.
    """
    now = [0.0]

    class Dribble(io.BytesIO):
        """Four bytes, each in its own syscall, with the clock advancing."""

        def __init__(self) -> None:
            """Start the body at four bytes."""
            super().__init__(b"abcd")

        def read1(self, size: int = -1) -> bytes:
            """Advance the clock, then yield one byte."""
            now[0] += 0.5
            return super().read1(1)

        def read(self, size: int = -1) -> bytes:
            """Refuse a blocking read, which would defeat the deadline check."""
            raise AssertionError(
                "download must use read1; read(n) loops on syscalls until n "
                "bytes arrive, which defeats the deadline check"
            )

    def opener(request: object, *, timeout: float, watch: object) -> Dribble:
        """Return the four-byte body that refuses `read`."""
        return Dribble()

    with pytest.raises(provision_tools.DownloadTimeout):
        provision_tools.download(
            "https://example.invalid/opa",
            tmp_path / "opa",
            len(OPA_BYTES),
            opener=opener,
            deadline_seconds=1.0,
            clock=lambda: now[0],
        )


def test_download_aborts_a_stalled_chunk_size_line_at_the_deadline(
    tmp_path: Path,
    stalled_server: Callable[[Callable[[socket.socket, threading.Event], None]], int],
) -> None:
    """A blocked chunked-framing read is interrupted, not merely checked around.

    `read1` on a chunked response calls `_get_chunk_left`, which reads a
    chunk-size line with `readline`. That call makes as many system calls as
    the peer forces it to (CPython's own comment there says it "may cause more
    than one read"), and no code of ours runs while it blocks. Checking the
    clock between reads therefore cannot bound this case, so a timer shuts the
    socket down instead.

    This drives a real `http.client.HTTPResponse` over a real socket: the
    server sends one complete chunk and then stalls part-way through the next
    chunk-size line. Without the watchdog the download would block for the
    full 30-second socket timeout; with it, the deadline wins by a wide margin.
    """

    def serve(conn: socket.socket, release: threading.Event) -> None:
        """Send a complete chunk, then stall inside the next size line."""
        conn.sendall(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n5"
        )
        release.wait(30)

    port = stalled_server(serve)
    interrupted: list[bool] = []

    def opener(
        request: object, *, timeout: float, watch: Callable[[socket.socket], None]
    ) -> http.client.HTTPResponse:
        """Open with a fixed 30-second timeout and a recorded socket."""
        # Deliberately *not* the clipped timeout the download passes.
        # Forwarding it would make the socket timeout a second candidate for
        # ending the stall, and the test could then pass without the watchdog
        # doing anything. Registration is required for the same reason it is
        # required in production: an unregistered socket cannot be shut down,
        # so the watchdog would have nothing to interrupt.
        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=30)
        create = connection._create_connection

        def create_and_watch(*args: object, **kwargs: object) -> socket.socket:
            """Connect, register the socket, then return it."""
            sock = create(*args, **kwargs)
            watch(RecordedSocket(sock, interrupted))
            return sock

        connection._create_connection = create_and_watch
        connection.request("GET", "/opa")
        return connection.getresponse()

    started = time.monotonic()
    with pytest.raises(provision_tools.DownloadTimeout):
        provision_tools.download(
            f"http://127.0.0.1:{port}/opa",
            tmp_path / "opa",
            len(OPA_BYTES),
            opener=opener,
            deadline_seconds=1.0,
        )

    elapsed = time.monotonic() - started
    assert interrupted, (
        "the watchdog never shut the socket down, so something else ended the "
        "read and the deadline did not bound it"
    )
    assert elapsed < 5.0, (
        f"the deadline took {elapsed:.1f}s to bite; the socket timeout alone "
        "would be 30s, so the read was not interrupted in time"
    )


def test_download_aborts_a_stalled_header_block_at_the_deadline(
    tmp_path: Path,
    stalled_server: Callable[[Callable[[socket.socket, threading.Event], None]], int],
) -> None:
    """A stalled *header* parse is bounded too, not only the body.

    `urlopen` connects, sends the request, and parses the status line and
    headers inside `getresponse()`. A server that sends a status line and then
    dribbles header bytes just inside the socket timeout holds that call open
    before any response object exists, so a watchdog armed after the opener
    returns can never fire. Here the socket timeout is 30s and the deadline is
    1s, so only a watchdog armed *before* the opener can end this in time.
    """

    def serve(conn: socket.socket, release: threading.Event) -> None:
        """Dribble header bytes without completing the header block."""
        conn.sendall(b"HTTP/1.1 200 OK\r\n")
        while not release.is_set():
            conn.sendall(b"X")
            time.sleep(0.3)

    port = stalled_server(serve)
    interrupted: list[bool] = []

    def opener(
        request: object, *, timeout: float, watch: Callable[[socket.socket], None]
    ) -> http.client.HTTPResponse:
        """Open with a fixed timeout and register the socket."""
        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=30)
        create = connection._create_connection

        def create_and_watch(*args: object, **kwargs: object) -> socket.socket:
            """Connect, register the socket, then return it."""
            sock = create(*args, **kwargs)
            watch(RecordedSocket(sock, interrupted))
            return sock

        connection._create_connection = create_and_watch
        connection.request("GET", "/opa")
        return connection.getresponse()

    started = time.monotonic()
    with pytest.raises(provision_tools.DownloadTimeout):
        provision_tools.download(
            f"http://127.0.0.1:{port}/opa",
            tmp_path / "opa",
            len(OPA_BYTES),
            opener=opener,
            deadline_seconds=1.0,
        )

    elapsed = time.monotonic() - started
    assert interrupted, (
        "the watchdog never shut the socket down, so the header parse was not "
        "interrupted and nothing bounded the call"
    )
    assert elapsed < 5.0, (
        f"the deadline took {elapsed:.1f}s to bite; the socket timeout alone "
        "would be 30s, so the watchdog did not cover the header parse"
    )


def test_a_read_clipped_by_the_socket_timeout_reports_the_deadline(
    tmp_path: Path,
) -> None:
    """A read ended by its clipped timeout is reported as a deadline, not raw.

    The socket timeout is clipped to `deadline_seconds`, so a read that blocks
    for the whole deadline raises a plain `TimeoutError` even when the watchdog
    timer has not got round to firing. That is the deadline doing its job, and
    the contributor should be told so rather than shown a bare socket error.
    """
    now = [0.0]

    class Stalled(io.BytesIO):
        """A body whose single read outlasts the socket timeout."""

        def read1(self, size: int) -> bytes:
            """Outlast the socket timeout, then fail as a timed-out read."""
            now[0] += 2.0
            raise TimeoutError("timed out")

    def opener(request: object, *, timeout: float, watch: object) -> Stalled:
        """Return the body whose single read outlives the socket timeout."""
        return Stalled()

    with pytest.raises(provision_tools.DownloadTimeout) as raised:
        provision_tools.download(
            "https://example.invalid/opa",
            tmp_path / "opa",
            len(OPA_BYTES),
            opener=opener,
            deadline_seconds=1.0,
            clock=lambda: now[0],
        )

    assert "did not finish within" in str(raised.value), (
        "the diagnostic names the deadline rather than the socket error"
    )
    assert not (tmp_path / "opa").read_bytes(), "no partial body is kept"


def test_fetch_url_watches_the_connection_it_opens(tmp_path: Path) -> None:
    """The production path registers its socket, so the watchdog can reach it.

    `fetch_url` is the only caller that builds the real opener. If its handler
    stopped passing `watch` through, every test using an injected opener would
    still pass while the shipped script had an uninterruptible download.

    The recording watchdog is supplied through `deadline_factory`, so this
    drives the real `fetch_url` and the real opener. Replacing the module
    attribute instead would test a name rather than the wiring: `fetch_url`
    resolving a different factory would go unnoticed.
    """
    registered: list[object] = []

    class Recorder:
        """A deadline that records every socket the opener registers."""

        def __init__(self, expire_at: float, clock: object) -> None:
            """Record the deadline the download computed."""
            self._expire_at = expire_at

        def watch(self, sock: object) -> None:
            """Record each socket the opener registers."""
            registered.append(sock)

        def fired(self) -> bool:
            """Report that the deadline has not passed."""
            return False

        def start(self) -> threading.Timer:
            """Arm a timer long enough never to fire during the test."""
            timer = threading.Timer(3600, lambda: None)
            timer.daemon = True
            timer.start()
            return timer

    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind(("127.0.0.1", 0))
    server.listen(1)
    port = server.getsockname()[1]

    def serve() -> None:
        """Answer one request with a complete `Content-Length` body."""
        try:
            conn, _ = server.accept()
            conn.recv(65536)
            conn.sendall(
                b"HTTP/1.1 200 OK\r\n"
                b"Content-Length: " + str(len(OPA_BYTES)).encode() + b"\r\n"
                b"\r\n" + OPA_BYTES
            )
            conn.close()
        finally:
            server.close()

    threading.Thread(target=serve, daemon=True).start()

    provision_tools.fetch_url(
        f"http://127.0.0.1:{port}/opa",
        tmp_path / "opa",
        4096,
        deadline_factory=Recorder,
    )

    assert registered, "the connection's socket reached the watchdog"
    assert isinstance(registered[0], socket.socket), "it is the connected socket"
    assert (tmp_path / "opa").read_bytes() == OPA_BYTES, "the body still arrived"


def test_download_aborts_a_stalled_content_length_body_at_the_deadline(
    tmp_path: Path,
    stalled_server: Callable[[Callable[[socket.socket, threading.Event], None]], int],
) -> None:
    """A body cut short at the deadline is not mistaken for a complete one.

    The watchdog implements the deadline by shutting the socket down, and a
    shutdown makes `read1` return `b""` — measured. For a `Content-Length`
    body that is indistinguishable from a clean end of stream, so without an
    explicit check a truncated download would be written out and then fail the
    digest check with a misleading message. The chunked case raises instead;
    this is the case where it does not.

    The opener fixes the socket timeout at 30 s and records that a shutdown
    happened, for the same reason as the chunked test above: `download` clips
    the socket timeout to `deadline_seconds`, and with a 1 s deadline here
    that clip is 1 s, so the socket timeout is an equally good candidate for
    ending the stall. Asserting the shutdown is what distinguishes the
    watchdog from the timeout, and the timeout case has its own test.
    """
    promised = 1000

    def serve(conn: socket.socket, release: threading.Event) -> None:
        """Promise more bytes than are sent, then stall the body."""
        conn.sendall(
            b"HTTP/1.1 200 OK\r\n"
            b"Content-Length: " + str(promised).encode() + b"\r\n\r\n" + OPA_BYTES
        )
        release.wait(30)

    port = stalled_server(serve)
    interrupted: list[bool] = []

    def opener(
        request: object, *, timeout: float, watch: Callable[[socket.socket], None]
    ) -> http.client.HTTPResponse:
        """Drive the shipped opener with a recorded socket and a pinned timeout."""
        # The shipped opener, so the production wiring stays under test, with
        # two deliberate substitutions. The socket timeout is pinned at 30 s
        # rather than forwarded, because `download` clips it to the 1 s
        # deadline and a 1 s socket timeout could end the stall on its own. And
        # every socket is wrapped so a shutdown is recorded: without that
        # assertion the test cannot tell an interrupted read from one the
        # timeout ended.
        return provision_tools.watched_opener(
            request,
            timeout=30,
            watch=lambda sock: watch(RecordedSocket(sock, interrupted)),
        )

    started = time.monotonic()
    with pytest.raises(provision_tools.DownloadTimeout):
        provision_tools.download(
            f"http://127.0.0.1:{port}/opa",
            tmp_path / "opa",
            4096,
            opener=opener,
            deadline_seconds=1.0,
        )

    assert interrupted, (
        "the watchdog never shut the socket down, so the socket timeout ended "
        "the read and the test proves nothing about the deadline"
    )
    assert time.monotonic() - started < 5.0, "the deadline, not the timeout, ended it"


def _tls_credentials(directory: Path) -> tuple[Path, Path] | None:
    """A self-signed certificate for 127.0.0.1, or `None` if `openssl` is absent.

    Only the *test* shells out. The provisioner itself launches no subprocess,
    so the certificate is generated here rather than by anything under test.

    `timeout` is set so a hung `openssl` cannot block the whole suite. It is
    deliberately not caught: a `TimeoutExpired` escaping here is the clearest
    possible statement that the environment is broken, and swallowing it would
    turn a real fault into a `None` that reads as "openssl is unavailable" and
    skips the TLS coverage instead.
    """
    if shutil.which("openssl") is None:
        return None
    certificate = directory / "cert.pem"
    key = directory / "key.pem"
    generated = subprocess.run(
        [
            "openssl",
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
            str(key),
            "-out",
            str(certificate),
            "-days",
            "1",
            "-nodes",
            "-subj",
            "/CN=127.0.0.1",
            "-addext",
            "subjectAltName=IP:127.0.0.1",
        ],
        capture_output=True,
        check=False,
        timeout=60,
    )
    if generated.returncode != 0 or not certificate.is_file():
        return None
    return certificate, key


def test_download_watches_the_tls_socket_not_the_one_it_replaces(
    tmp_path: Path,
) -> None:
    """HTTPS registers the socket that survives the handshake.

    `HTTPSConnection.connect` replaces `self.sock` with the TLS socket from
    `wrap_socket`, and that call closes the socket underneath it. Registering
    at creation — as an earlier revision did — therefore registered a dead
    handle for every HTTPS download: `fileno()` is `-1` and `shutdown` raises
    `EBADF`, so the watchdog could never interrupt anything. This test asserts
    the registered socket is usable after `connect()` returns.
    """
    credentials = _tls_credentials(tmp_path)
    if credentials is None:
        pytest.skip("openssl is unavailable, so no TLS server can be started")
    certificate, key = credentials

    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(str(certificate), str(key))

    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind(("127.0.0.1", 0))
    server.listen(1)
    port = server.getsockname()[1]

    def serve() -> None:
        """Accept one TLS connection, then stall before answering it."""
        try:
            raw, _ = server.accept()
            tls = context.wrap_socket(raw, server_side=True)
            try:
                tls.recv(65536)
                tls.sendall(
                    b"HTTP/1.1 200 OK\r\n"
                    b"Content-Length: " + str(len(OPA_BYTES)).encode() + b"\r\n"
                    b"\r\n" + OPA_BYTES
                )
            finally:
                tls.close()
        except (OSError, ssl.SSLError):
            # The client shuts the socket down on purpose; that is the behaviour
            # under test, not a fault in this thread.
            pass
        finally:
            server.close()

    threading.Thread(target=serve, daemon=True).start()

    registered: list[socket.socket] = []
    client_context = ssl.create_default_context()
    client_context.check_hostname = False
    client_context.verify_mode = ssl.CERT_NONE

    connection = provision_tools._watched_connection(http.client.HTTPSConnection)(  # noqa: SLF001
        "127.0.0.1",
        port,
        timeout=30,
        context=client_context,
        watch=registered.append,
    )
    connection.connect()
    try:
        assert registered, "the connection registered a socket"
        live = registered[0]
        assert live.fileno() != -1, (
            "the registered socket was the pre-TLS one; wrap_socket closed it, "
            "so shutdown would raise EBADF and the watchdog could not interrupt"
        )
        live.shutdown(socket.SHUT_RDWR)
    finally:
        connection.close()


def test_download_clips_the_socket_timeout_to_the_deadline(tmp_path: Path) -> None:
    """A read already blocked cannot outlive the deadline by its own timeout.

    The deadline check runs between reads, so a read that has begun blocks for
    its socket timeout before returning. If that timeout stayed at
    `REQUEST_TIMEOUT_SECONDS` it could exceed a shorter deadline; it is clipped
    so no single blocking read outlasts the bound.
    """
    seen: list[float] = []

    def opener(request: object, *, timeout: float, watch: object) -> io.BytesIO:
        """Record the timeout the download clipped, then serve the body."""
        seen.append(timeout)
        return io.BytesIO(OPA_BYTES)

    provision_tools.download(
        "https://example.invalid/opa",
        tmp_path / "opa",
        len(OPA_BYTES),
        opener=opener,
        deadline_seconds=5.0,
    )

    assert seen == [5.0], "the socket timeout is the smaller of the two bounds"
    assert provision_tools.REQUEST_TIMEOUT_SECONDS > 5.0, (
        "this test is vacuous unless the deadline is the smaller bound"
    )
