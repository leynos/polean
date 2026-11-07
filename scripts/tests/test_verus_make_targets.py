"""Wiring tests for the Makefile's Verus targets (EP-M4).

These tests exercise `install-verus`, `verus-proof`, `verus-controls`, and the
aggregate `verus` target in an isolated temporary repository layout, with
controlled stand-ins for `uv`, Python, and the pinned verifier. They check
argument forwarding, the complete pinned `uv` launcher, `VERUS_Z3_PATH`
propagation, the setup-error diagnostics for an absent and a non-executable
verifier, the malformed-pin and malformed-version diagnostics, and failure
propagation.

They establish no proof evidence. The real proof and its controls are the
`make verus` gate's business; a passing wiring test says only that the recipes
call the tools they claim to call.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
VERSION = (REPO / "tools" / "verus" / "VERSION").read_text(encoding="utf-8").strip()
VERUS_BIN = Path(".verus") / VERSION / "verus" / "verus"
Z3_BIN = Path(".verus") / "z3" / "z3"
REF_FILE = Path("tools") / "rust-prover-tools" / "REF"
VERSION_FILE = Path("tools") / "verus" / "VERSION"
PIN = (REPO / REF_FILE).read_text(encoding="utf-8").split()[1]
PINNED_SOURCE = f"git+https://github.com/leynos/rust-prover-tools.git@{PIN}"
MALFORMED_PIN = "PROVER_TOOLS_REF missing or malformed"
MALFORMED_VERSION = "VERUS_VERSION missing or malformed"

RECORDER = '''#!{python}
"""Record one tool invocation, then exit with the configured status."""

import json
import os
import sys
from pathlib import Path

record = {{
    "program": Path(sys.argv[0]).name,
    "argv": sys.argv[1:],
    "z3": os.environ.get("VERUS_Z3_PATH"),
}}
with Path(os.environ["FAKE_LOG"]).open("a", encoding="utf-8") as log:
    log.write(json.dumps(record) + "\\n")
sys.exit(int(os.environ.get("FAKE_EXIT", "0")))
'''


class Layout:
    """An isolated repository root with controlled tools."""

    def __init__(self, root: Path, bin_dir: Path, log: Path) -> None:
        """Record the layout's paths."""
        self.root = root
        self.bin_dir = bin_dir
        self.log = log

    def environment(self, **overrides: str) -> dict[str, str]:
        """The environment a recipe runs under, with the stand-ins first."""
        environment = dict(os.environ)
        environment["PATH"] = os.pathsep.join(
            [str(self.bin_dir), environment.get("PATH", "")]
        )
        environment["FAKE_LOG"] = str(self.log)
        environment.pop("VERUS_Z3_PATH", None)
        environment.update(overrides)
        return environment

    def extractor(self, name: str) -> Path:
        """Write a recording stand-in called `name` and return its path."""
        path = self.bin_dir / name
        path.write_text(RECORDER.format(python=sys.executable), encoding="utf-8")
        path.chmod(0o755)
        return path

    def installed_verus(self) -> Path:
        """Place a recording verifier where the Makefile expects the pin."""
        path = self.root / VERUS_BIN
        path.parent.mkdir(parents=True)
        path.write_text(RECORDER.format(python=sys.executable), encoding="utf-8")
        path.chmod(0o755)
        return path

    def calls(self) -> list[dict[str, object]]:
        """The invocations the stand-ins recorded."""
        if not self.log.is_file():
            return []
        return [
            json.loads(line)
            for line in self.log.read_text(encoding="utf-8").splitlines()
            if line.strip()
        ]

    def make(self, target: str, **overrides: str) -> subprocess.CompletedProcess[str]:
        """Run one target, with `PYTHON` pointed at a recording stand-in."""
        python = self.extractor("fake-python")
        return subprocess.run(
            [
                "make",
                "--no-print-directory",
                "-C",
                str(self.root),
                target,
                f"PYTHON={python}",
            ],
            capture_output=True,
            text=True,
            timeout=120,
            check=False,
            env=self.environment(**overrides),
        )


@pytest.fixture
def layout(tmp_path: Path) -> Layout:
    """Build a repository root holding only what the Makefile reads."""
    root = tmp_path / "repo"
    (root / "tools" / "rust-prover-tools").mkdir(parents=True)
    (root / "tools" / "verus").mkdir(parents=True)
    shutil.copy(REPO / "Makefile", root / "Makefile")
    shutil.copy(REPO / REF_FILE, root / REF_FILE)
    shutil.copy(
        REPO / "tools" / "verus" / "VERSION", root / "tools" / "verus" / "VERSION"
    )
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    return Layout(root, bin_dir, tmp_path / "calls.log")


PINNED_RUN = ["tool", "run", "--from", PINNED_SOURCE, "prover-tools"]


def test_install_verus_calls_both_installers(layout: Layout) -> None:
    """The target runs the prover-tools Verus install, then its Z3 install."""
    layout.extractor("uv")

    result = layout.make("install-verus")

    assert result.returncode == 0, result.stderr
    calls = layout.calls()
    assert [call["program"] for call in calls] == ["uv", "uv"], (
        "both installs go through the pinned tool, not a local script"
    )
    assert calls[0]["argv"] == [
        *PINNED_RUN,
        "verus",
        "install",
        "--repo-root",
        ".",
        "--install-dir",
        str(VERUS_BIN.parents[1]),
    ], "uv runs the pinned prover-tools source, installing where the proof looks"
    assert calls[1]["argv"][: len(PINNED_RUN) + 4] == [
        *PINNED_RUN,
        "z3",
        "install",
        "--repo-root",
        ".",
    ], "the solver installs through the same pinned tool, after the release"
    assert str(Z3_BIN.parent) in calls[1]["argv"], (
        "the solver lands where VERUS_Z3_PATH looks"
    )


def test_verus_proof_forwards_the_binary_and_solver(layout: Layout) -> None:
    """The proof target runs the pinned tool once per proof entry point."""
    layout.installed_verus()
    layout.extractor("uv")

    result = layout.make("verus-proof", VERUS_FLAGS='-q --triggers "--cfg a b"')

    assert result.returncode == 0, result.stderr
    assert [call["program"] for call in layout.calls()] == ["uv", "uv"], (
        "the pinned tool is the runner, once per proof entry point"
    )
    assert [call["argv"] for call in layout.calls()] == [
        [
            *PINNED_RUN,
            "verus",
            "run",
            "--repo-root",
            ".",
            "--verus-bin",
            str(layout.root / VERUS_BIN),
            "--no-install-missing",
            "--proof-file",
            proof,
            "--extra-arg=-q",
            "--extra-arg=--triggers",
            "--extra-arg=--cfg a b",
        ]
        for proof in ("verus/admission_recognizer.rs", "verus/audit_settlement.rs")
    ], "every proof file reaches the installed verifier, each Verus flag as an extra argument"
    assert all(call["z3"] == str(layout.root / Z3_BIN) for call in layout.calls()), (
        "VERUS_Z3_PATH names the pinned solver for every run"
    )


def test_verus_controls_forwards_both_binaries(layout: Layout) -> None:
    """The controls target passes the verifier and solver to the script."""
    verus = layout.installed_verus()

    result = layout.make("verus-controls")

    assert result.returncode == 0, result.stderr
    (call,) = layout.calls()
    assert call["argv"] == [
        "scripts/check_verus_controls.py",
        "--verus",
        str(verus),
        "--z3",
        str(layout.root / Z3_BIN),
    ], "both pinned binaries are forwarded"


@pytest.mark.parametrize("target", ["verus-proof", "verus-controls"])
def test_missing_verus_is_a_named_setup_error(layout: Layout, target: str) -> None:
    """An absent verifier fails with the install instruction, before running."""
    result = layout.make(target)

    assert result.returncode != 0, "an absent verifier is a failure"
    assert f"Verus is not installed at {layout.root / VERUS_BIN}" in result.stderr, (
        "the diagnostic names the expected verifier path"
    )
    assert "make install-verus" in result.stderr, (
        "the diagnostic names the install target"
    )
    assert layout.calls() == [], "no tool ran"


@pytest.mark.parametrize("target", ["verus-proof", "verus-controls"])
def test_non_executable_verus_is_a_named_setup_error(
    layout: Layout, target: str
) -> None:
    """A verifier present but not executable fails like an absent one."""
    path = layout.root / VERUS_BIN
    path.parent.mkdir(parents=True)
    path.write_text(RECORDER.format(python=sys.executable), encoding="utf-8")
    path.chmod(0o644)

    result = layout.make(target)

    assert result.returncode != 0, "a non-executable verifier is a failure"
    assert f"Verus is not installed at {layout.root / VERUS_BIN}" in result.stderr, (
        "the diagnostic names the expected verifier path"
    )
    assert "make install-verus" in result.stderr, (
        "the diagnostic names the install target"
    )
    assert layout.calls() == [], (
        "the recipe decides on the execute bit, not merely on the file existing"
    )


def test_malformed_pin_stops_every_target(layout: Layout) -> None:
    """A REF without a 40-hex commit is rejected before any tool runs."""
    (layout.root / REF_FILE).write_text("ref: not-a-commit\n", encoding="utf-8")
    layout.extractor("uv")
    layout.installed_verus()

    result = layout.make("verus")

    assert result.returncode != 0, "a malformed pin is a failure"
    assert MALFORMED_PIN in result.stderr, (
        "the diagnostic names the malformed pin rather than failing obscurely"
    )
    assert str(REF_FILE) in result.stderr, "the diagnostic names the pin file"
    assert layout.calls() == [], "the pin is validated before any tool runs"


def test_malformed_version_stops_every_target(layout: Layout) -> None:
    """A VERSION that is not a release name is rejected before any tool runs."""
    (layout.root / VERSION_FILE).write_text("../escape\n", encoding="utf-8")
    layout.extractor("uv")
    layout.installed_verus()

    result = layout.make("verus")

    assert result.returncode != 0, "a malformed version is a failure"
    assert MALFORMED_VERSION in result.stderr, (
        "the diagnostic names the malformed version rather than failing obscurely"
    )
    assert str(VERSION_FILE) in result.stderr, "the diagnostic names the version file"
    assert layout.calls() == [], "the version is validated before any tool runs"


def test_verus_proof_propagates_a_verifier_failure(layout: Layout) -> None:
    """A failing proof run makes the target fail."""
    layout.installed_verus()
    layout.extractor("uv")

    result = layout.make("verus-proof", FAKE_EXIT="7")

    assert result.returncode != 0, "the verifier's failure propagates"
    assert len(layout.calls()) == 1, "the verifier did run"


def test_verus_runs_the_proof_before_the_controls(layout: Layout) -> None:
    """The aggregate target runs both prerequisites, proof first."""
    layout.installed_verus()
    layout.extractor("uv")

    result = layout.make("verus")

    assert result.returncode == 0, result.stderr
    programs = [call["program"] for call in layout.calls()]
    assert programs == ["uv", "uv", "fake-python"], (
        "both proofs run through the pinned tool, before the controls"
    )
    assert layout.calls()[-1]["argv"][0] == "scripts/check_verus_controls.py", (
        "the controls script runs last"
    )
