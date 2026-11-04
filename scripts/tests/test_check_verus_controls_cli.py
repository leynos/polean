"""Command-boundary tests for the Verus negative controls (EP-M4).

The unit tests call `check()` with an injected runner. These tests cross the
process boundary: they invoke the script the way `make verus-controls` does,
with a stand-in verifier that records how it was called and answers according to
the staged tree. No test runs the real Verus binary, and none establishes proof
evidence; the real proof and its controls are the `make verus` gate's business.

Covered at this boundary: successful orchestration, missing tools, a broken
baseline, a mutation the verifier accepts, and a rejection attributed to an
unrelated obligation. Each test asserts the exit status and the diagnostic.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

import check_verus_controls as controls
import pytest

ROOT = Path(controls.__file__).resolve().parents[1]
SCRIPT = Path(controls.__file__).resolve()

FAKE_VERUS = '''#!{python}
"""A stand-in for the pinned Verus binary, driven by the staged tree."""

import hashlib
import json
import os
import sys
from pathlib import Path

here = Path(__file__).resolve().parent
manifest = json.loads((here / "controls.json").read_text(encoding="utf-8"))
mode = os.environ.get("FAKE_VERUS_MODE", "success")
record = {{"argv": sys.argv[1:], "z3": os.environ.get("VERUS_Z3_PATH")}}
with (here / "calls.log").open("a", encoding="utf-8") as log:
    log.write(json.dumps(record) + "\\n")

mutated = None
by_digest = {{control["digest"]: control["expects"] for control in manifest["controls"]}}
for path in sorted(Path.cwd().rglob("*.rs")):
    if hashlib.sha256(path.read_bytes()).hexdigest() in by_digest:
        mutated = by_digest[hashlib.sha256(path.read_bytes()).hexdigest()]
        break

if mode == "baseline-failure":
    print("error: the baseline is broken")
    print("verification results:: 59 verified, 1 errors")
    sys.exit(1)
if mode == "always-accept":
    print("verification results:: 59 verified, 0 errors")
    sys.exit(0)
if mutated is not None and mode == "unrelated":
    print("error: an unrelated obligation failed")
    print("verification results:: 59 verified, 1 errors")
    sys.exit(1)
if mutated is not None:
    print(" ".join(mutated))
    print("error: rejected at the control's obligation")
    print("verification results:: 58 verified, 1 errors")
    sys.exit(1)
print("verification results:: 59 verified, 0 errors")
sys.exit(0)
'''


@pytest.fixture
def fake_tools(tmp_path: Path) -> Path:
    """Write the stand-in verifier and the scenario manifest beside it."""
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    verus = bin_dir / "verus"
    verus.write_text(FAKE_VERUS.format(python=sys.executable), encoding="utf-8")
    verus.chmod(0o755)
    manifest = {
        "controls": [
            {
                "name": control.name,
                "digest": hashlib.sha256(
                    controls.apply_control(ROOT, control).encode("utf-8")
                ).hexdigest(),
                "expects": list(control.expects),
            }
            for control in controls.CONTROLS
        ]
    }
    (bin_dir / "controls.json").write_text(json.dumps(manifest), encoding="utf-8")
    return bin_dir


def z3_binary(fake_tools: Path) -> Path:
    """A file that satisfies the script's `is_file` check for Z3."""
    z3 = fake_tools / "z3"
    z3.write_text("", encoding="utf-8")
    return z3


def run_controls(
    *arguments: str | Path, mode: str = "success"
) -> subprocess.CompletedProcess[str]:
    """Run the controls script as the Makefile's recipe would."""
    environment = dict(os.environ)
    environment["FAKE_VERUS_MODE"] = mode
    return subprocess.run(
        [sys.executable, str(SCRIPT), *(str(argument) for argument in arguments)],
        capture_output=True,
        text=True,
        timeout=120,
        check=False,
        env=environment,
    )


def calls(fake_tools: Path) -> list[dict[str, object]]:
    """The verifier invocations recorded by the stand-in."""
    log = fake_tools / "calls.log"
    if not log.is_file():
        return []
    return [
        json.loads(line)
        for line in log.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]


def test_runs_the_baseline_and_every_control(fake_tools: Path) -> None:
    """A clean orchestration runs each control once after a clean baseline."""
    verus = fake_tools / "verus"
    z3 = z3_binary(fake_tools)

    result = run_controls("--root", ROOT, "--verus", verus, "--z3", z3)

    assert result.returncode == 0, result.stderr
    lines = result.stdout.splitlines()
    assert len(lines) == len(controls.CONTROLS), "one evidence line per control"
    for control, line in zip(controls.CONTROLS, lines, strict=True):
        assert line.startswith(f"{control.kind}:{control.name}: rejected at"), line
    recorded = calls(fake_tools)
    assert len(recorded) == len(controls.CONTROLS) + 1, (
        "the baseline runs once before the controls"
    )
    assert all(call["z3"] == str(z3) for call in recorded), (
        "the runner forwards the pinned Z3 path"
    )
    assert recorded[0]["argv"] == [controls.PROOF_FILE.as_posix()], (
        "the runner names the staged proof entry point"
    )


@pytest.mark.parametrize("missing", ["verus", "z3"])
def test_reports_missing_tools(fake_tools: Path, missing: str) -> None:
    """An absent tool is a setup error, reported before any control runs."""
    verus = fake_tools / "verus"
    z3 = z3_binary(fake_tools)
    absent = fake_tools / f"absent-{missing}"

    result = run_controls(
        "--root",
        ROOT,
        "--verus",
        absent if missing == "verus" else verus,
        "--z3",
        absent if missing == "z3" else z3,
    )

    assert result.returncode == 2, "a missing tool is a setup error"
    assert "make install-verus" in result.stderr, (
        "the diagnostic names the install target"
    )
    assert calls(fake_tools) == [], "no control ran"


def test_reports_a_baseline_failure(fake_tools: Path) -> None:
    """A baseline that does not verify stops before any control is attributed."""
    result = run_controls(
        "--root",
        ROOT,
        "--verus",
        fake_tools / "verus",
        "--z3",
        z3_binary(fake_tools),
        mode="baseline-failure",
    )

    assert result.returncode == 1, "a broken baseline is a failure"
    assert "unmodified staged proof did not verify" in result.stderr
    assert len(calls(fake_tools)) == 1, "only the baseline ran"


def test_reports_an_accepted_mutation(fake_tools: Path) -> None:
    """A verifier that accepts a mutated kernel is reported as a failure."""
    result = run_controls(
        "--root",
        ROOT,
        "--verus",
        fake_tools / "verus",
        "--z3",
        z3_binary(fake_tools),
        mode="always-accept",
    )

    assert result.returncode == 1, "an accepted defect is a failure"
    assert "Verus accepted the defect" in result.stderr


def test_reports_an_unrelated_rejection(fake_tools: Path) -> None:
    """A rejection elsewhere is not credited to the control's obligation."""
    result = run_controls(
        "--root",
        ROOT,
        "--verus",
        fake_tools / "verus",
        "--z3",
        z3_binary(fake_tools),
        mode="unrelated",
    )

    assert result.returncode == 1, "an unrelated failure cannot pass a control"
    assert "the output lacks" in result.stderr, (
        "the diagnostic names the missing obligation"
    )