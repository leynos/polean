"""Tests for the digest-checked toolchain provisioner (EP-M2).

Each test injects a fetcher and points the provisioner at a temporary root, so
no test touches the network, `.polean/tools/`, or the pinned artefacts. The
tests cover the two obligations in the plan:

* O1, the digest gate: an artefact installs only when its byte count and
  SHA-256 both equal the pin.
* O2, containment: an archive member outside the pinned root is refused before
  extraction.

They also cover pin-file validation, re-hashing an installed binary, and the
offline re-run.
"""

from __future__ import annotations

import compression.zstd
import hashlib
import io
import json
import tarfile
from pathlib import Path

import pytest

import provision_tools

OPA_BYTES = b"opa-binary-placeholder"
LEAN_VERSION = "4.35.0"
LEAN_ROOT = f"lean-{LEAN_VERSION}-linux"


def make_archive(
    members: dict[str, bytes],
    links: dict[str, str] | None = None,
    hard_links: dict[str, str] | None = None,
) -> bytes:
    """Build a zstd-compressed tar archive from relative names and contents.

    `links` adds a symbolic link per name, with the given target; `hard_links`
    does the same for a hard link, whose target is an archive-relative name.
    """
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w") as archive:
        for name, payload in members.items():
            info = tarfile.TarInfo(name)
            info.size = len(payload)
            archive.addfile(info, io.BytesIO(payload))
        for kind, table in ((tarfile.SYMTYPE, links), (tarfile.LNKTYPE, hard_links)):
            for name, target in (table or {}).items():
                info = tarfile.TarInfo(name)
                info.type = kind
                info.linkname = target
                archive.addfile(info)
    return compression.zstd.compress(buffer.getvalue())


def _write_archive(directory: Path, payload: bytes) -> Path:
    """Write archive bytes to a fresh file under `directory`."""
    path = directory / f"archive-{hashlib.sha256(payload).hexdigest()[:16]}.tar.zst"
    path.write_bytes(payload)
    return path


def _check_archive(directory: Path, payload: bytes) -> None:
    """Run the archive containment check against `payload` and its own pin."""
    pins_path = write_pins(directory, lean=lean_pin_for(payload))
    provision_tools._check_archive(
        _write_archive(directory, payload), provision_tools.load_pins(pins_path).lean
    )


def lean_archive_bytes(root_name: str = LEAN_ROOT) -> bytes:
    """A well-formed Lean archive with one file under the pinned root."""
    return make_archive({f"{root_name}/bin/lean": b"lean" * 100})


def pin_for(artefact: bytes, **overrides: object) -> dict[str, object]:
    """A valid binary pin for `artefact`."""
    pin = {
        "version": "1.21.2",
        "url": "https://example.invalid/opa",
        "sha256": hashlib.sha256(artefact).hexdigest(),
        "size": len(artefact),
        "kind": "binary",
    }
    pin.update(overrides)
    return pin


def lean_pin_for(artefact: bytes, **overrides: object) -> dict[str, object]:
    """A valid tar.zst pin for `artefact`."""
    pin = {
        "version": LEAN_VERSION,
        "url": "https://example.invalid/lean.tar.zst",
        "sha256": hashlib.sha256(artefact).hexdigest(),
        "size": len(artefact),
        "kind": "tar.zst",
        "root": LEAN_ROOT,
        "identity": f"Lean (version {LEAN_VERSION}, x86_64-unknown-linux-gnu)",
    }
    pin.update(overrides)
    return pin


def write_pins(
    root: Path,
    opa: dict[str, object] | None = None,
    lean: dict[str, object] | None = None,
    **overrides: object,
) -> Path:
    """Write a pin file with a valid shape, allowing per-test overrides."""
    pins: dict[str, object] = {
        "format": provision_tools.SUPPORTED_FORMAT,
        "platform": "x86_64-unknown-linux-gnu",
        "opa": opa if opa is not None else pin_for(OPA_BYTES),
        "lean": lean if lean is not None else lean_pin_for(lean_archive_bytes()),
    }
    pins.update(overrides)
    path = root / "toolchain-pins.json"
    path.write_text(json.dumps(pins), encoding="utf-8")
    return path


def good_fetcher(artefacts: dict[str, bytes]) -> provision_tools.Fetcher:
    """Return a fetcher that serves `artefacts` by URL."""

    def fetch(url: str, destination: Path, limit: int) -> None:
        payload = artefacts[url]
        if len(payload) > limit:
            raise provision_tools.DownloadTooLarge(url, limit)
        destination.write_bytes(payload)

    return fetch


def raising_fetcher(url: str, destination: Path, limit: int) -> None:
    """A fetcher that must never be called; any call is a test failure."""
    raise AssertionError(f"the provisioner downloaded {url}")


def standard_fetcher(lean: bytes = lean_archive_bytes()) -> provision_tools.Fetcher:
    """The fetcher serving the two URLs the standard pin file names."""
    return good_fetcher(
        {
            "https://example.invalid/opa": OPA_BYTES,
            "https://example.invalid/lean.tar.zst": lean,
        }
    )


def test_install_accepts_correct_bytes(tmp_path: Path) -> None:
    """Both artefacts install when size and digest match the pin."""
    pins_path = write_pins(tmp_path)
    tools_dir = tmp_path / "tools"

    installed = provision_tools.install_tools(pins_path, tools_dir, standard_fetcher())

    opa_sha = hashlib.sha256(OPA_BYTES).hexdigest()
    lean_sha = hashlib.sha256(lean_archive_bytes()).hexdigest()
    assert installed.opa == tools_dir / "opa" / opa_sha / "opa", (
        "the installed OPA path is digest-named"
    )
    assert installed.lean_root == tools_dir / "lean" / lean_sha, (
        "the installed Lean root is digest-named"
    )
    assert installed.opa.read_bytes() == OPA_BYTES, "the OPA bytes are intact"
    assert (installed.lean_root / "bin" / "lean").read_bytes() == b"lean" * 100, (
        "the Lean member is extracted"
    )
    assert installed.opa.stat().st_mode & 0o111, "the OPA binary is executable"
    assert (installed.lean_root / "COMPLETE").read_text(encoding="utf-8").strip() == lean_sha, (
        "the completion marker records the Lean digest"
    )


def test_install_rejects_a_flipped_byte(tmp_path: Path) -> None:
    """A one-byte change to the OPA stream is refused, and nothing installs."""
    corrupted = OPA_BYTES[:-1] + b"X"
    pins_path = write_pins(tmp_path)
    tools_dir = tmp_path / "tools"
    fetcher = good_fetcher(
        {
            "https://example.invalid/opa": corrupted,
            "https://example.invalid/lean.tar.zst": lean_archive_bytes(),
        }
    )

    with pytest.raises(provision_tools.DigestMismatch) as raised:
        provision_tools.install_tools(pins_path, tools_dir, fetcher)

    assert hashlib.sha256(OPA_BYTES).hexdigest() in str(raised.value), (
        "the diagnostic names the pinned digest"
    )
    assert not (tools_dir / "opa").exists(), "no OPA install appears"
    assert not list((tools_dir / "downloads").glob(".part-*")), (
        "the partial download is removed"
    )


def test_install_rejects_a_truncated_stream(tmp_path: Path) -> None:
    """A stream shorter than the pinned size is refused."""
    truncated = OPA_BYTES[:-1]
    pins_path = write_pins(tmp_path, opa=pin_for(OPA_BYTES))
    tools_dir = tmp_path / "tools"
    fetcher = good_fetcher(
        {
            "https://example.invalid/opa": truncated,
            "https://example.invalid/lean.tar.zst": lean_archive_bytes(),
        }
    )

    with pytest.raises(provision_tools.SizeMismatch) as raised:
        provision_tools.install_tools(pins_path, tools_dir, fetcher)

    assert str(len(OPA_BYTES)) in str(raised.value), "the diagnostic names the pin"
    assert not (tools_dir / "opa").exists(), "no OPA install appears"


@pytest.mark.parametrize(
    ("label", "members"),
    [
        pytest.param(
            "parent-relative",
            {f"{LEAN_ROOT}/bin/lean": b"lean", "../escape": b"nope"},
            id="parent-relative",
        ),
        pytest.param(
            "dot-dot-inside-the-root",
            {f"{LEAN_ROOT}/bin/lean": b"lean", f"{LEAN_ROOT}/../escape": b"nope"},
            id="dot-dot-inside-the-root",
        ),
        pytest.param("absolute", {"/abs": b"nope"}, id="absolute"),
        pytest.param(
            "other-top-level", {"other-root/bin/lean": b"lean"}, id="other-top-level"
        ),
    ],
)
def test_install_refuses_an_escaping_member(
    tmp_path: Path, label: str, members: dict[str, bytes]
) -> None:
    """A member that would leave the pinned root is refused before extraction."""
    escaping = make_archive(members)
    pins_path = write_pins(
        tmp_path, opa=pin_for(OPA_BYTES), lean=lean_pin_for(escaping)
    )
    tools_dir = tmp_path / "tools"

    with pytest.raises(provision_tools.ArchiveEscape) as raised:
        provision_tools.install_tools(
            pins_path, tools_dir, standard_fetcher(lean=escaping)
        )

    assert "outside" in str(raised.value), f"{label}: the diagnostic names the escape"
    assert not (tmp_path / "escape").exists(), f"{label}: nothing escaped the tools directory"
    assert not (tools_dir / "lean" / hashlib.sha256(escaping).hexdigest()).exists(), (
        f"{label}: no Lean install appears"
    )


def test_install_refuses_a_dot_dot_member_that_the_filter_would_hide(
    tmp_path: Path,
) -> None:
    """`root/../escape` starts with the root, so the root check alone misses it.

    The `data` filter rewrites this member to `escape` inside the staging
    directory, so without an explicit `..` check the archive would be accepted
    and the member silently dropped rather than refused.
    """
    escaping = make_archive(
        {f"{LEAN_ROOT}/bin/lean": b"lean", f"{LEAN_ROOT}/../escape": b"nope"}
    )

    with pytest.raises(provision_tools.ArchiveEscape):
        _check_archive(tmp_path, escaping)


def test_install_refuses_a_link_leaving_the_root(tmp_path: Path) -> None:
    """A symbolic link whose target leaves the pinned root is refused."""
    escaping = make_archive(
        {f"{LEAN_ROOT}/bin/lean": b"lean"}, links={f"{LEAN_ROOT}/link": "../../etc/passwd"}
    )

    with pytest.raises(provision_tools.ArchiveEscape):
        _check_archive(tmp_path, escaping)


def test_install_accepts_a_link_inside_the_root(tmp_path: Path) -> None:
    """A link that stays under the pinned root is accepted."""
    archive = make_archive(
        {f"{LEAN_ROOT}/bin/lean": b"lean"},
        links={f"{LEAN_ROOT}/bin/alias": "lean"},
        hard_links={f"{LEAN_ROOT}/bin/hard": f"{LEAN_ROOT}/bin/lean"},
    )

    _check_archive(tmp_path, archive)


def test_install_refuses_a_hard_link_resolved_from_the_wrong_base(
    tmp_path: Path,
) -> None:
    """A hard link's target is archive-relative, not relative to its own directory.

    Resolving `link` against `root/bin` would produce `root/etc/passwd`, which
    stays under the root and so passes a containment test that used the wrong
    base.
    """
    escaping = make_archive(
        {f"{LEAN_ROOT}/bin/lean": b"lean"},
        hard_links={f"{LEAN_ROOT}/bin/link": f"{LEAN_ROOT}/bin/../../etc/passwd"},
    )

    with pytest.raises(provision_tools.ArchiveEscape):
        _check_archive(tmp_path, escaping)


def test_complete_install_is_reused_offline(tmp_path: Path) -> None:
    """A second run with a raising fetcher succeeds without downloading."""
    pins_path = write_pins(tmp_path)
    tools_dir = tmp_path / "tools"
    first = provision_tools.install_tools(pins_path, tools_dir, standard_fetcher())

    second = provision_tools.install_tools(pins_path, tools_dir, raising_fetcher)

    assert second == first, "the same installed paths are reported"


def test_opa_is_rehashed_on_every_run(tmp_path: Path) -> None:
    """A replaced OPA binary fails before the run reports success."""
    pins_path = write_pins(tmp_path)
    tools_dir = tmp_path / "tools"
    installed = provision_tools.install_tools(pins_path, tools_dir, standard_fetcher())
    installed.opa.write_bytes(b"tampered")

    with pytest.raises(provision_tools.DigestMismatch):
        provision_tools.install_tools(pins_path, tools_dir, raising_fetcher)


def test_corrupt_cached_download_is_refetched(tmp_path: Path) -> None:
    """A corrupt cached download is discarded rather than trusted."""
    pins_path = write_pins(tmp_path)
    tools_dir = tmp_path / "tools"
    opa_sha = hashlib.sha256(OPA_BYTES).hexdigest()
    (tools_dir / "downloads").mkdir(parents=True)
    (tools_dir / "downloads" / f"opa-{opa_sha}").write_bytes(b"corrupt")

    installed = provision_tools.install_tools(pins_path, tools_dir, standard_fetcher())

    assert installed.opa.read_bytes() == OPA_BYTES, "the fetched copy replaced it"


def test_fetch_url_passes_a_timeout(tmp_path: Path) -> None:
    """A stalled connection cannot hold the tools lock indefinitely."""
    observed: dict[str, object] = {}

    class Response(io.BytesIO):
        """A response body that ends immediately."""

    def opener(request: object, timeout: float) -> Response:
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

    def opener(request: object, timeout: float) -> Response:
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


def test_unreadable_marker_is_not_treated_as_an_incomplete_install(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """A permission failure is reported, not repaired by deleting the install.

    `_is_complete` answering `False` for an unreadable marker sends
    `_install_lean` into the repair path, which removes the existing tree and
    re-downloads 600 MiB. A directory the process cannot read is an environment
    failure, so the `OSError` must reach `main`, which reports it as one line.
    """
    pins_path = write_pins(tmp_path)
    tools_dir = tmp_path / "tools"
    lean_sha = hashlib.sha256(lean_archive_bytes()).hexdigest()
    installed = tools_dir / "lean" / lean_sha
    installed.mkdir(parents=True)
    marker = installed / provision_tools.COMPLETE_MARKER
    marker.write_text(f"{lean_sha}\n", encoding="utf-8")

    real_read_text = Path.read_text

    def deny_marker(self: Path, *args: object, **kwargs: object) -> str:
        if self == marker:
            raise PermissionError(13, "Permission denied", str(marker))
        return real_read_text(self, *args, **kwargs)

    monkeypatch.setattr(Path, "read_text", deny_marker)

    fetched: list[str] = []
    standard = standard_fetcher()

    def recording(url: str, destination: Path, limit: int) -> None:
        fetched.append(url)
        standard(url, destination, limit)

    status = provision_tools.main(pins_path, tools_dir, recording)

    assert status == 1, "an unreadable marker is an environment failure"
    assert installed.is_dir(), "the existing install is not deleted by the repair path"
    assert "https://example.invalid/lean.tar.zst" not in fetched, (
        "the unreadable marker does not send the provisioner to the network"
    )


def test_load_pins_rejects_an_unknown_key(tmp_path: Path) -> None:
    """Pin-file validation rejects a key the script does not understand."""
    pins_path = write_pins(tmp_path, extra="unexpected")

    with pytest.raises(provision_tools.PinFileError) as raised:
        provision_tools.load_pins(pins_path)

    assert "extra" in str(raised.value), "the diagnostic names the key"


def test_load_pins_rejects_a_root_without_the_version(tmp_path: Path) -> None:
    """The archive root must embed the pinned version."""
    pins_path = write_pins(
        tmp_path,
        lean=lean_pin_for(lean_archive_bytes(), root="lean-unversioned"),
    )

    with pytest.raises(provision_tools.PinFileError) as raised:
        provision_tools.load_pins(pins_path)

    assert LEAN_VERSION in str(raised.value), "the diagnostic names the version"


def test_load_pins_rejects_an_identity_without_the_version(tmp_path: Path) -> None:
    """The reported identity must embed the pinned version.

    The smoke test compares Lean's reported line against this string. An
    identity naming a different version would make that assertion demand the
    wrong toolchain, so the pin file refuses it rather than letting the two
    authorities disagree.
    """
    pins_path = write_pins(
        tmp_path,
        lean=lean_pin_for(
            lean_archive_bytes(), identity="Lean (version 4.34.1, x86_64-linux)"
        ),
    )

    with pytest.raises(provision_tools.PinFileError) as raised:
        provision_tools.load_pins(pins_path)

    assert LEAN_VERSION in str(raised.value), "the diagnostic names the version"


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

        def read(self, size: int = -1) -> bytes:
            reads.append(size)
            return b"x"

    def opener(request: object, timeout: float) -> Trickle:
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
            super().__init__(b"a" * 3)

        def read(self, size: int = -1) -> bytes:
            now[0] += 0.4
            return super().read(1)

    def opener(request: object, timeout: float) -> Chunks:
        return Chunks()

    with (
        pytest.MonkeyPatch.context() as patch,
        pytest.raises(provision_tools.DownloadTimeout),
    ):
        patch.setattr(provision_tools.time, "monotonic", lambda: now[0])
        provision_tools.download(
            "https://example.invalid/opa",
            tmp_path / "opa",
            len(OPA_BYTES),
            opener=opener,
            deadline_seconds=1.0,
        )


def test_present_lets_an_unreadable_parent_raise(tmp_path: Path) -> None:
    """`_present` reports absence, not every failure to find out.

    `Path.exists` answers `False` for a permission error too. Here a directory
    with no execute permission makes `stat` fail with `PermissionError`; the
    helper must raise it rather than answer `False`, because a caller that
    treats `False` as "not installed" would delete a complete tree.
    """
    locked = tmp_path / "locked"
    locked.mkdir()
    locked.chmod(0o000)
    try:
        with pytest.raises(PermissionError):
            provision_tools._present(locked / "COMPLETE")
    finally:
        locked.chmod(0o755)


def test_present_reports_an_absent_path_as_absent(tmp_path: Path) -> None:
    """The one answer that means "not installed" stays `False`."""
    assert provision_tools._present(tmp_path / "COMPLETE") is False, (
        "a genuinely missing marker is absence, not an environment failure"
    )


def test_load_pins_rejects_a_bad_digest(tmp_path: Path) -> None:
    """A digest that is not 64 lowercase hex characters is rejected."""
    pins_path = write_pins(tmp_path, opa=pin_for(OPA_BYTES, sha256="not-a-digest"))

    with pytest.raises(provision_tools.PinFileError):
        provision_tools.load_pins(pins_path)


def test_main_reports_an_environment_failure_as_a_diagnostic(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """An unreachable network is a one-line diagnosis, not a traceback.

    `urllib` raises `URLError`, which is an `OSError` and not a
    `ProvisionError`. The contributor sees the same `provisioning failed:` line
    and non-zero status as for a refused digest.
    """

    def offline(url: str, destination: Path, limit: int) -> None:
        raise OSError("network is unreachable")

    status = provision_tools.main(write_pins(tmp_path), tmp_path / "tools", offline)

    captured = capsys.readouterr()
    assert status == 1, "an environment failure must exit non-zero"
    assert "provisioning failed: network is unreachable" in captured.err, (
        "the failure is reported on stderr as a diagnostic"
    )
    assert "Traceback" not in captured.err, "no traceback reaches the contributor"
