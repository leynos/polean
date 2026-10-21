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
import urllib.request
from pathlib import Path

import pytest

import provision_tools

OPA_BYTES = b"opa-binary-placeholder"
LEAN_VERSION = "4.35.0"
LEAN_ROOT = f"lean-{LEAN_VERSION}-linux"


def make_archive(members: dict[str, bytes]) -> bytes:
    """Build a zstd-compressed tar archive from relative names and contents."""
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w") as archive:
        for name, payload in members.items():
            info = tarfile.TarInfo(name)
            info.size = len(payload)
            archive.addfile(info, io.BytesIO(payload))
    return compression.zstd.compress(buffer.getvalue())


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


def test_install_refuses_an_escaping_member(tmp_path: Path) -> None:
    """An archive member outside the pinned root is refused before extraction."""
    escaping = make_archive(
        {f"{LEAN_ROOT}/bin/lean": b"lean", "../escape": b"nope"}
    )
    pins_path = write_pins(
        tmp_path,
        opa=pin_for(OPA_BYTES),
        lean=lean_pin_for(escaping),
    )
    tools_dir = tmp_path / "tools"

    with pytest.raises(provision_tools.ArchiveEscape):
        provision_tools.install_tools(
            pins_path, tools_dir, standard_fetcher(lean=escaping)
        )

    assert not (tmp_path / "escape").exists(), "nothing escaped the tools directory"
    assert not (tmp_path / "tools" / "escape").exists(), (
        "nothing appeared beside the tools directory"
    )
    assert not (tools_dir / "lean" / hashlib.sha256(escaping).hexdigest()).exists(), (
        "no Lean install appears"
    )


def test_install_refuses_an_absolute_member(tmp_path: Path) -> None:
    """An absolute archive member is refused before extraction."""
    escaping = make_archive({"/abs": b"nope"})
    pins_path = write_pins(
        tmp_path, opa=pin_for(OPA_BYTES), lean=lean_pin_for(escaping)
    )

    with pytest.raises(provision_tools.ArchiveEscape):
        provision_tools.install_tools(
            pins_path, tmp_path / "tools", standard_fetcher(lean=escaping)
        )

    assert not Path("/abs").exists(), "nothing was written at an absolute path"


def test_install_refuses_a_member_outside_the_root(tmp_path: Path) -> None:
    """A member under a different top-level directory is refused."""
    escaping = make_archive({"other-root/bin/lean": b"lean"})
    pins_path = write_pins(
        tmp_path, opa=pin_for(OPA_BYTES), lean=lean_pin_for(escaping)
    )

    with pytest.raises(provision_tools.ArchiveEscape):
        provision_tools.install_tools(
            pins_path, tmp_path / "tools", standard_fetcher(lean=escaping)
        )


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


def test_fetch_url_passes_a_timeout(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """A stalled connection cannot hold the tools lock indefinitely."""
    observed: dict[str, object] = {}

    class Response(io.BytesIO):
        """A response body that ends immediately."""

    def urlopen(request: object, timeout: float) -> Response:
        observed["timeout"] = timeout
        return Response(OPA_BYTES)

    monkeypatch.setattr(urllib.request, "urlopen", urlopen)
    destination = tmp_path / "opa"

    provision_tools.fetch_url("https://example.invalid/opa", destination, len(OPA_BYTES))

    assert observed["timeout"] == provision_tools.REQUEST_TIMEOUT_SECONDS, (
        "the download must bound how long a stalled read may block"
    )
    assert destination.read_bytes() == OPA_BYTES, "the fetched bytes are written"


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


def test_load_pins_rejects_a_bad_digest(tmp_path: Path) -> None:
    """A digest that is not 64 lowercase hex characters is rejected."""
    pins_path = write_pins(tmp_path, opa=pin_for(OPA_BYTES, sha256="not-a-digest"))

    with pytest.raises(provision_tools.PinFileError):
        provision_tools.load_pins(pins_path)
