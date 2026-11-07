"""Tests for the Verus negative controls that `make verus` runs (EP-M5).

No test runs Verus or Cargo. The controls' anchors are checked against the
tracked proof sources, so an edit that moves the code a control describes fails
here rather than turning the control into a silent no-op. The orchestration is
exercised by replacing the verifier run with a stub.
"""

from __future__ import annotations

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
def test_every_anchor_occurs_exactly_once_in_the_tracked_source(
    control: controls.Control,
) -> None:
    """The control describes the source as it is committed."""
    source = (ROOT / control.path).read_text(encoding="utf-8")
    assert source.count(control.anchor) == 1, (
        f"control {control.name} must name exactly one anchor in {control.path}"
    )


@pytest.mark.parametrize("control", controls.CONTROLS, ids=lambda c: c.name)
def test_every_control_changes_the_source(control: controls.Control) -> None:
    """The edit is a real change, not a rewrite to the same text."""
    assert control.anchor != control.replacement, (
        f"control {control.name} must change the source"
    )


@pytest.mark.parametrize("control", controls.CONTROLS, ids=lambda c: c.name)
def test_every_control_expects_a_named_obligation(control: controls.Control) -> None:
    """A rejection must name the proof item the control targets."""
    assert control.expects, f"control {control.name} must expect an obligation"
    assert all(name.strip() == name and name for name in control.expects), (
        f"control {control.name} must name its obligations without stray space"
    )


def test_the_controls_cover_both_kinds() -> None:
    """Both body and specification defects are represented."""
    assert {control.kind for control in controls.CONTROLS} == {"body", "spec"}, (
        "the controls must cover both body and specification defects"
    )


def test_apply_control_replaces_the_anchor(tmp_path: Path) -> None:
    """A staged control writes the replacement text."""
    control = controls.CONTROLS[0]
    path = tmp_path / control.path
    path.parent.mkdir(parents=True)
    path.write_text(f"before\n{control.anchor}after\n", encoding="utf-8")
    changed = controls.apply_control(tmp_path, control)
    assert control.replacement in changed, "the replacement must be written"
    assert control.anchor not in changed, "the anchor must be replaced"


def test_apply_control_rejects_a_missing_anchor(tmp_path: Path) -> None:
    """A drifted source fails closed instead of replacing nothing."""
    control = controls.CONTROLS[0]
    path = tmp_path / control.path
    path.parent.mkdir(parents=True)
    path.write_text("unrelated text\n", encoding="utf-8")
    with pytest.raises(controls.ControlError, match="found 0"):
        controls.apply_control(tmp_path, control)


def test_apply_control_rejects_an_ambiguous_anchor(tmp_path: Path) -> None:
    """An edit that would not be the bounded change the control claims fails."""
    control = controls.CONTROLS[0]
    path = tmp_path / control.path
    path.parent.mkdir(parents=True)
    path.write_text(control.anchor * 2, encoding="utf-8")
    with pytest.raises(controls.ControlError, match="found 2"):
        controls.apply_control(tmp_path, control)


def test_apply_control_rejects_a_missing_file(tmp_path: Path) -> None:
    """A control whose file is absent is a setup failure."""
    with pytest.raises(controls.ControlError, match="cannot read"):
        controls.apply_control(tmp_path, controls.CONTROLS[0])


def test_results_parses_the_summary() -> None:
    """The verified and error counts come from Verus's own summary."""
    output = "note: x\nverification results:: 59 verified, 0 errors\n"
    assert controls.results(output) == (59, 0), "the summary's counts must be read"


def test_results_is_none_without_a_summary() -> None:
    """An unrecognized output is not mistaken for a clean run."""
    assert controls.results("killed before verification\n") is None, (
        "output without a summary must not read as a pass"
    )


def test_require_baseline_accepts_a_clean_run() -> None:
    """A zero-error baseline is the precondition for every control."""
    controls.require_baseline(
        completed(0, "verification results:: 59 verified, 0 errors\n")
    )


def test_require_baseline_rejects_an_erroring_run() -> None:
    """A broken baseline would make every control meaningless."""
    with pytest.raises(controls.ControlError, match="did not verify"):
        controls.require_baseline(
            completed(0, "verification results:: 58 verified, 1 errors\n")
        )


def test_require_baseline_rejects_a_missing_summary() -> None:
    """A run that never reported a summary is not a pass."""
    with pytest.raises(controls.ControlError, match="did not verify"):
        controls.require_baseline(completed(1, "error: cannot find crate\n"))


def test_require_rejection_accepts_a_failure_at_the_obligation() -> None:
    """A rejection naming the target obligation is the expected evidence."""
    control = controls.CONTROLS[0]
    evidence = " ".join(control.expects)
    obligation = controls.require_rejection(
        control,
        completed(
            1,
            f"error: {evidence}\n",
            "verification results:: 58 verified, 1 errors\n",
        ),
    )
    assert obligation == " + ".join(control.expects), (
        "the evidence line names every expected obligation"
    )


def test_require_rejection_rejects_an_accepted_defect() -> None:
    """A verifier that accepts the defect proves nothing about sensitivity."""
    control = controls.CONTROLS[0]
    with pytest.raises(controls.ControlError, match="accepted the defect"):
        controls.require_rejection(
            control, completed(0, "verification results:: 59 verified, 0 errors\n")
        )


def test_require_rejection_rejects_an_unrelated_failure() -> None:
    """A failure elsewhere does not show the control reached its obligation."""
    control = controls.CONTROLS[0]
    with pytest.raises(controls.ControlError, match="output lacks"):
        controls.require_rejection(
            control, completed(1, "error: something else failed\n")
        )


def test_require_rejection_rejects_a_run_without_an_error() -> None:
    """A non-zero exit with no error message is a tool failure, not a verdict."""
    control = controls.CONTROLS[0]
    with pytest.raises(controls.ControlError, match="without reporting an error"):
        controls.require_rejection(control, completed(101, "interrupted\n"))


def test_check_runs_each_tree_baseline_before_its_controls(tmp_path: Path) -> None:
    """Each control runs on a tree restored to that tree's baseline text."""
    snapshots: list[tuple[Path, dict[str, str]]] = []
    seen: dict[Path, int] = {}

    def fake_run(
        _verus: Path, _z3: Path, scratch: Path, entry: Path
    ) -> subprocess.CompletedProcess[str]:
        """Record the staged files, then report the verifier's verdict."""
        tree = next(tree for tree in controls.TREES if tree.entry == entry)
        paths = {
            control.path for control in controls.CONTROLS if control.tree == tree
        }
        snapshots.append(
            (
                entry,
                {
                    path.as_posix(): (scratch / path).read_text(encoding="utf-8")
                    for path in paths
                },
            )
        )
        seen[entry] = seen.get(entry, 0) + 1
        if seen[entry] == 1:
            return completed(0, "verification results:: 59 verified, 0 errors\n")
        evidence = " ".join(
            name
            for control in controls.CONTROLS
            if control.tree == tree
            for name in control.expects
        )
        return completed(
            1,
            f"error: rejected {evidence}\nverification results:: 58 verified, 1 errors\n",
        )

    lines = controls.check(
        ROOT, tmp_path / "verus", tmp_path / "z3", runner=fake_run
    )
    assert len(lines) == len(controls.CONTROLS), "one evidence line per control"
    assert len(snapshots) == len(controls.CONTROLS) + len(controls.TREES), (
        "each tree's baseline runs once before its controls"
    )
    for tree in controls.TREES:
        tree_controls = [
            control for control in controls.CONTROLS if control.tree == tree
        ]
        tree_snapshots = [
            snapshot for entry, snapshot in snapshots if entry == tree.entry
        ]
        baseline = tree_snapshots[0]
        for control, snapshot in zip(tree_controls, tree_snapshots[1:], strict=True):
            changed = {
                path: text for path, text in snapshot.items() if text != baseline[path]
            }
            own = control.path.as_posix()
            assert set(changed) == {own}, (
                f"control {control.name} must change only {own} from the baseline"
            )
            assert control.replacement in changed[own], (
                f"control {control.name} must write its replacement to {own}"
            )
    assert lines[0].startswith("body:"), "the first evidence line names a body control"


def test_every_tree_has_body_and_specification_controls() -> None:
    """Each proof is shown sensitive to both its code and its specification."""
    for tree in controls.TREES:
        kinds = {control.kind for control in controls.CONTROLS if control.tree == tree}
        assert kinds == {"body", "spec"}, (
            f"the {tree.name} proof must cover both kinds of defect"
        )


def test_stage_preserves_the_admission_relative_paths(tmp_path: Path) -> None:
    """The admission proof's relative includes resolve in the staged tree."""
    controls.stage(ROOT, tmp_path, controls.ADMISSION)
    assert (tmp_path / controls.ADMISSION.entry).is_file(), (
        "the proof entry point is staged"
    )
    assert (tmp_path / controls.KERNEL_DIR / "mod.rs").is_file(), "the kernel is staged"
    assert (tmp_path / controls.PROOF_DIR / "admission_spec.rs").is_file(), (
        "the specification is staged"
    )


def test_stage_preserves_the_settlement_relative_paths(tmp_path: Path) -> None:
    """The settlement proof's relative includes resolve in the staged tree."""
    controls.stage(ROOT, tmp_path, controls.SETTLEMENT)
    assert (tmp_path / controls.SETTLEMENT.entry).is_file(), (
        "the proof entry point is staged"
    )
    assert (tmp_path / controls.SETTLEMENT_DIR / "mod.rs").is_file(), (
        "the decision module is staged"
    )
    assert (tmp_path / controls.PROOF_DIR / "audit_settlement_spec.rs").is_file(), (
        "the specification is staged"
    )


def test_verus_paths_reads_the_pin() -> None:
    """The default binary paths follow the pin file."""
    version = (ROOT / controls.VERSION_FILE).read_text(encoding="utf-8").strip()
    verus, z3 = controls.verus_paths(ROOT)
    assert verus == ROOT / ".verus" / version / "verus" / "verus", (
        "the Verus path follows the pin"
    )
    assert z3 == ROOT / ".verus" / "z3" / "z3", "the Z3 path is the install path"


def test_pinned_version_rejects_an_empty_pin(tmp_path: Path) -> None:
    """An empty pin cannot name a binary, so it fails closed."""
    pin = tmp_path / controls.VERSION_FILE
    pin.parent.mkdir(parents=True)
    pin.write_text("\n", encoding="utf-8")
    with pytest.raises(controls.ControlError, match="empty"):
        controls.pinned_version(tmp_path)


def test_main_reports_a_missing_verifier(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    """An absent Verus install is a setup error, not a control failure."""
    code = controls.main(["--root", str(ROOT), "--verus", str(tmp_path / "missing")])
    assert code == 2, "a missing verifier is a setup error"
    assert "make install-verus" in capsys.readouterr().err, (
        "the diagnostic names the install target"
    )
