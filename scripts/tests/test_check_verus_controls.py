"""Tests for the Verus negative controls that `make verus` runs (EP-M5).

No test runs Verus or Cargo. The controls' anchors are checked against the
tracked proof sources, so an edit that moves the code a control describes fails
here rather than turning the control into a silent no-op. The orchestration is
exercised by replacing the verifier run with a stub.
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

import check_verus_controls as controls
import pytest

ROOT = Path(__file__).resolve().parents[2]


def completed(
    returncode: int, stdout: str = "", stderr: str = ""
) -> subprocess.CompletedProcess[str]:
    """A finished verifier run with the given output."""
    return subprocess.CompletedProcess([], returncode, stdout, stderr)


@pytest.mark.parametrize("control", controls.CONTROLS, ids=lambda c: c.name)
def test_every_anchor_occurs_exactly_once_in_the_tracked_source(control):
    """The control describes the source as it is committed."""
    source = (ROOT / control.path).read_text(encoding="utf-8")
    assert source.count(control.anchor) == 1


@pytest.mark.parametrize("control", controls.CONTROLS, ids=lambda c: c.name)
def test_every_control_changes_the_source(control):
    """The edit is a real change, not a rewrite to the same text."""
    assert control.anchor != control.replacement


@pytest.mark.parametrize("control", controls.CONTROLS, ids=lambda c: c.name)
def test_every_control_expects_a_named_obligation(control):
    """A rejection must name the proof item the control targets."""
    assert control.expects
    assert all(name.strip() == name and name for name in control.expects)


def test_the_controls_cover_both_kinds():
    """Both body and specification defects are represented."""
    assert {control.kind for control in controls.CONTROLS} == {"body", "spec"}


def test_apply_control_replaces_the_anchor(tmp_path):
    """A staged control writes the replacement text."""
    control = controls.CONTROLS[0]
    path = tmp_path / control.path
    path.parent.mkdir(parents=True)
    path.write_text(f"before\n{control.anchor}after\n", encoding="utf-8")
    changed = controls.apply_control(tmp_path, control)
    assert control.replacement in changed
    assert control.anchor not in changed


def test_apply_control_rejects_a_missing_anchor(tmp_path):
    """A drifted source fails closed instead of replacing nothing."""
    control = controls.CONTROLS[0]
    path = tmp_path / control.path
    path.parent.mkdir(parents=True)
    path.write_text("unrelated text\n", encoding="utf-8")
    with pytest.raises(controls.ControlError, match="found 0"):
        controls.apply_control(tmp_path, control)


def test_apply_control_rejects_an_ambiguous_anchor(tmp_path):
    """An edit that would not be the bounded change the control claims fails."""
    control = controls.CONTROLS[0]
    path = tmp_path / control.path
    path.parent.mkdir(parents=True)
    path.write_text(control.anchor * 2, encoding="utf-8")
    with pytest.raises(controls.ControlError, match="found 2"):
        controls.apply_control(tmp_path, control)


def test_apply_control_rejects_a_missing_file(tmp_path):
    """A control whose file is absent is a setup failure."""
    with pytest.raises(controls.ControlError, match="cannot read"):
        controls.apply_control(tmp_path, controls.CONTROLS[0])


def test_results_parses_the_summary():
    """The verified and error counts come from Verus's own summary."""
    output = "note: x\nverification results:: 59 verified, 0 errors\n"
    assert controls.results(output) == (59, 0)


def test_results_is_none_without_a_summary():
    """An unrecognised output is not mistaken for a clean run."""
    assert controls.results("killed before verification\n") is None


def test_require_baseline_accepts_a_clean_run():
    """A zero-error baseline is the precondition for every control."""
    controls.require_baseline(
        completed(0, "verification results:: 59 verified, 0 errors\n")
    )


def test_require_baseline_rejects_an_erroring_run():
    """A broken baseline would make every control meaningless."""
    with pytest.raises(controls.ControlError, match="did not verify"):
        controls.require_baseline(
            completed(0, "verification results:: 58 verified, 1 error\n")
        )


def test_require_baseline_rejects_a_missing_summary():
    """A run that never reported a summary is not a pass."""
    with pytest.raises(controls.ControlError, match="did not verify"):
        controls.require_baseline(completed(1, "error: cannot find crate\n"))


def test_require_rejection_accepts_a_failure_at_the_obligation():
    """A rejection naming the target obligation is the expected evidence."""
    control = controls.CONTROLS[0]
    evidence = " ".join(control.expects)
    obligation = controls.require_rejection(
        control,
        completed(
            1,
            f"error: {evidence}\n",
            "verification results:: 58 verified, 1 error\n",
        ),
    )
    assert obligation == " + ".join(control.expects)


def test_require_rejection_rejects_an_accepted_defect():
    """A verifier that accepts the defect proves nothing about sensitivity."""
    control = controls.CONTROLS[0]
    with pytest.raises(controls.ControlError, match="accepted the defect"):
        controls.require_rejection(
            control, completed(0, "verification results:: 59 verified, 0 errors\n")
        )


def test_require_rejection_rejects_an_unrelated_failure():
    """A failure elsewhere does not show the control reached its obligation."""
    control = controls.CONTROLS[0]
    with pytest.raises(controls.ControlError, match="output lacks"):
        controls.require_rejection(
            control, completed(1, "error: something else failed\n")
        )


def test_require_rejection_rejects_a_run_without_an_error():
    """A non-zero exit with no error message is a tool failure, not a verdict."""
    control = controls.CONTROLS[0]
    with pytest.raises(controls.ControlError, match="without reporting an error"):
        controls.require_rejection(control, completed(101, "interrupted\n"))


def test_check_runs_the_baseline_before_the_controls(tmp_path, monkeypatch):
    """Each control runs on a tree restored to the baseline text."""
    seen: list[str] = []

    def fake_run(_verus, _z3, scratch):
        seen.append("run")
        if len(seen) == 1:
            return completed(0, "verification results:: 59 verified, 0 errors\n")
        evidence = " ".join(name for control in controls.CONTROLS for name in control.expects)
        return completed(
            1,
            f"error: rejected {evidence}\nverification results:: 58 verified, 1 errors\n",
        )

    monkeypatch.setattr(controls, "run_verus", fake_run)
    lines = controls.check(ROOT, tmp_path / "verus", tmp_path / "z3")
    assert len(lines) == len(controls.CONTROLS)
    assert len(seen) == len(controls.CONTROLS) + 1
    assert lines[0].startswith("body:")


def test_stage_preserves_the_relative_paths(tmp_path):
    """The proof's relative includes resolve in the staged tree."""
    controls.stage(ROOT, tmp_path)
    assert (tmp_path / controls.PROOF_FILE).is_file()
    assert (tmp_path / controls.KERNEL_DIR / "mod.rs").is_file()
    assert (tmp_path / controls.PROOF_DIR / "admission_spec.rs").is_file()


def test_verus_paths_reads_the_pin():
    """The default binary paths follow the pin file."""
    version = (ROOT / controls.VERSION_FILE).read_text(encoding="utf-8").strip()
    verus, z3 = controls.verus_paths(ROOT)
    assert verus == ROOT / ".verus" / version / "verus" / "verus"
    assert z3 == ROOT / ".verus" / "z3" / "z3"


def test_pinned_version_rejects_an_empty_pin(tmp_path):
    """An empty pin cannot name a binary, so it fails closed."""
    pin = tmp_path / controls.VERSION_FILE
    pin.parent.mkdir(parents=True)
    pin.write_text("\n", encoding="utf-8")
    with pytest.raises(controls.ControlError, match="empty"):
        controls.pinned_version(tmp_path)


def test_main_reports_a_missing_verifier(tmp_path, capsys):
    """An absent Verus install is a setup error, not a control failure."""
    code = controls.main(["--root", str(ROOT), "--verus", str(tmp_path / "missing")])
    assert code == 2
    assert "make install-verus" in capsys.readouterr().err
