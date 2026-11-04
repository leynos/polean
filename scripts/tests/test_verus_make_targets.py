"""Wiring tests for the Makefile's Verus targets (EP-M4).

These tests exercise `install-verus`, `verus-proof`, `verus-controls`, and the
aggregate `verus` target in an isolated temporary repository layout, with
controlled stand-ins for `uv`, Python, and the pinned verifier. They check
argument forwarding, `VERUS_Z3_PATH` propagation, the missing-binary
diagnostics, and failure propagation.

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
    shutil.copy(
        REPO / "tools" / "rust-prover-tools" / "REF",
        root / "tools" / "rust-prover-tools" / "REF",
    )
    shutil.copy(
        REPO / "tools" / "verus" / "VERSION", root / "tools" / "verus" / "VERSION"
    )
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    return Layout(root, bin_dir, tmp_path / "calls.log")


def test_install_verus_calls_both_installers(layout: Layout) -> None:
    """The target runs the prover-tools install, then the Z3 installer."""
    layout.extractor("uv")

    result = layout.make("install-verus")

    assert result.returncode == 0, result.stderr
    programs = [call["program"] for call in layout.calls()]
    assert programs == ["uv", "fake-python"], (
        "the pinned release installs before the solver"
    )
    assert layout.calls()[0]["argv"][-5:] == [
        "prover-tools",
        "verus",
        "install",
        "--repo-root",
        ".",
    ], "the prover-tools arguments are forwarded"
    assert layout.calls()[1]["argv"] == ["scripts/install_verus_z3.py"], (
        "the Z3 installer is invoked through $(PYTHON)"
    )


def test_verus_proof_forwards_the_binary_and_solver(layout: Layout) -> None:
    """The proof target runs the pinned verifier with the pinned solver."""
    layout.installed_verus()

    result = layout.make("verus-proof", VERUS_FLAGS="-q")

    assert result.returncode == 0, result.stderr
    (call,) = layout.calls()
    assert call["program"] == "verus", "the pinned verifier is the program"
    assert call["argv"] == ["verus/admission_recognizer.rs", "-q"], (
        "the proof file and flags are forwarded"
    )
    assert call["z3"] == str(layout.root / Z3_BIN), (
        "VERUS_Z3_PATH names the pinned solver"
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
    assert f"Verus is not installed at {layout.root / VERUS_BIN}" in result.stderr
    assert "make install-verus" in result.stderr, (
        "the diagnostic names the install target"
    )
    assert layout.calls() == [], "no tool ran"


def test_verus_proof_propagates_a_verifier_failure(layout: Layout) -> None:
    """A failing proof run makes the target fail."""
    layout.installed_verus()

    result = layout.make("verus-proof", FAKE_EXIT="7")

    assert result.returncode != 0, "the verifier's failure propagates"
    assert len(layout.calls()) == 1, "the verifier did run"


def test_verus_runs_the_proof_before_the_controls(layout: Layout) -> None:
    """The aggregate target runs both prerequisites, proof first."""
    layout.installed_verus()

    result = layout.make("verus")

    assert result.returncode == 0, result.stderr
    programs = [call["program"] for call in layout.calls()]
    assert programs == ["verus", "fake-python"], (
        "the proof runs before the controls"
    )
    assert layout.calls()[1]["argv"][0] == "scripts/check_verus_controls.py"