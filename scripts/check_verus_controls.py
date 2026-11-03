#!/usr/bin/env -S uv run python
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Show that the kernel proof is sensitive to the code and the specification.

A passing verifier run establishes that the proof discharges the obligations it
is given. It does not establish that those obligations respond to a defect. This
script supplies the missing negative evidence with two kinds of control:

* a **body control** removes one rule from the executable kernel, leaving the
  specification unchanged, and requires Verus to reject the result; and
* a **specification control** weakens one definition that the exclusion
  theorems rest on, leaving the kernel unchanged, and requires Verus to reject
  the exclusion obligation that depended on it.

Each control is a textual edit to a copy of the proof sources in a scratch
directory. The pinned Verus binary runs the unmodified copy first; the copy must
verify with zero errors, so a control cannot appear to work because the baseline
is already broken. Every edit's anchor must occur exactly once, so a control
fails closed when the source it describes has drifted rather than silently
replacing nothing.

The controls never edit the working tree, and this script runs no Cargo or
repository gate. ``make verus`` invokes it after the ordinary proof run.
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
KERNEL_DIR = Path("tests/admission/recognizer/kernel")
PROOF_DIR = Path("verus")
PROOF_FILE = PROOF_DIR / "admission_recognizer.rs"
VERSION_FILE = Path("tools/verus/VERSION")

#: The summary Verus prints once it has checked every function in the file.
RESULTS = re.compile(r"verification results:: (\d+) verified, (\d+) errors")

#: Verus errors name the function whose obligation failed.
ERROR = re.compile(r"^error", re.MULTILINE)


@dataclass(frozen=True)
class Control:
    """One negative control: an edit and the failure a verifier must report.

    ``expects`` is a tuple of substrings that must all appear in the verifier
    output. Together they name the failing obligation and the file that holds
    it, so a control cannot pass on an unrelated failure elsewhere in the proof.
    """

    kind: str
    name: str
    path: Path
    anchor: str
    replacement: str
    expects: tuple[str, ...]


CONTROLS: tuple[Control, ...] = (
    Control(
        kind="body",
        name="metadata-rule-dropped",
        path=KERNEL_DIR / "trivia.rs",
        anchor="""    if contains_metadata(src, start, end) {
        return Err(hash);
    }
""",
        replacement="""    // NEGATIVE CONTROL: the metadata rule is dropped.
    let _ = contains_metadata(src, start, end);
""",
        expects=("postcondition not satisfied", "kernel/trivia.rs"),
    ),
    Control(
        kind="body",
        name="empty-gap-rule-dropped",
        path=KERNEL_DIR / "mod.rs",
        anchor="""        if index > 0 && gap.end == at {
            proof {
                if admits_canonical(src@, toks@, needs_newline@) {
                    lemma_ws_canonical_gap_positive(
                        src@,
                        toks@,
                        needs_newline@,
                        witness,
                        index as int,
                    );
                    assert(at as int == prev_end(src@, toks@, witness, index as int));
                    assert(witness[index as int - 1] as int + toks@[index as int - 1]@.len()
                        < witness[index as int] as int);
                    assert((at as int) < (gap.end as int));
                    assert(false);
                }
            }
            proof {
                assert(!admits_canonical(src@, toks@, needs_newline@));
                assert(at as int <= src@.len());
            }
            return Err(at);
        }
""",
        replacement="""        // NEGATIVE CONTROL: the non-empty gap rule is dropped.
""",
        expects=("invariant not satisfied", "kernel/mod.rs"),
    ),
    Control(
        kind="body",
        name="line-feed-rule-dropped",
        path=KERNEL_DIR / "mod.rs",
        anchor="""        if flag && !gap.newline {
            proof {
                if admits_canonical(src@, toks@, needs_newline@) {
                    reveal(flags_prefix);
                    lemma_ws_canonical_flags(src@, toks@, needs_newline@, witness);
                    assert(flag == needs_newline@[index as int]);
                    assert(flags_hold(src@, toks@, witness, needs_newline@));
                    assert(flags_prefix(
                        src@,
                        toks@,
                        witness,
                        needs_newline@,
                        toks@.len() as int,
                    ));
                    assert(gap_has_newline(
                        src@,
                        prev_end(src@, toks@, witness, index as int),
                        witness[index as int] as int,
                    ));
                    assert(gap.newline == gap_has_newline(src@, at as int, gap.end as int));
                    assert(false);
                }
            }
            return Err(gap.end);
        }
""",
        replacement="""        // NEGATIVE CONTROL: the line-feed rule is dropped.
""",
        expects=("assertion failed", "kernel/mod.rs"),
    ),
    Control(
        kind="spec",
        name="body-byte-admits-nul",
        path=PROOF_DIR / "admission_spec.rs",
        anchor=(
            "pub open spec fn body_byte(b: u8) -> bool { b == b'\\t' || "
            "(0x20 <= b && b <= 0x7e) }\n"
        ),
        replacement="""pub open spec fn body_byte(b: u8) -> bool {
    b == b'\\t' || (0x20 <= b && b <= 0x7e) || b == 0
}
""",
        expects=(
            "postcondition not satisfied",
            "admission_lemmas/permitted.rs",
            "permitted_byte(s, i)",
        ),
    ),
    Control(
        kind="spec",
        name="comment-ok-admits-metadata",
        path=PROOF_DIR / "admission_spec.rs",
        anchor="""        && (comment_end(s, hash) == s.len() || line_end_at(s, comment_end(s, hash)))
        && !metadata_in(s, hash + 1, comment_end(s, hash))
""",
        replacement="""        && (comment_end(s, hash) == s.len() || line_end_at(s, comment_end(s, hash)))
""",
        expects=(
            "assertion failed",
            "admission_lemmas/metadata.rs",
            "false",
        ),
    ),
)


class ControlError(Exception):
    """A control could not be applied or did not fail as required."""


def pinned_version(root: Path) -> str:
    """The pinned Verus version, read from the pin file."""
    try:
        version = (root / VERSION_FILE).read_text(encoding="utf-8").strip()
    except OSError as error:
        raise ControlError(f"cannot read the Verus pin: {error}") from error
    if not version:
        raise ControlError(f"the Verus pin in {VERSION_FILE} is empty")
    return version


def verus_paths(root: Path) -> tuple[Path, Path]:
    """The pinned Verus and Z3 binaries."""
    version = pinned_version(root)
    return (
        root / ".verus" / version / "verus" / "verus",
        root / ".verus" / "z3" / "z3",
    )


def apply_control(root: Path, control: Control) -> str:
    """Return ``control.path``'s text with the control's single edit applied.

    The anchor must occur exactly once. Zero occurrences means the source has
    drifted from what the control describes; more than one means the edit would
    not be the bounded change the control claims, so both fail closed.
    """
    path = root / control.path
    try:
        source = path.read_text(encoding="utf-8")
    except OSError as error:
        raise ControlError(f"cannot read {control.path}: {error}") from error
    found = source.count(control.anchor)
    if found != 1:
        raise ControlError(
            f"control {control.name}: expected one anchor in {control.path}, "
            f"found {found}"
        )
    return source.replace(control.anchor, control.replacement, 1)


def stage(root: Path, scratch: Path) -> None:
    """Copy the proof sources into ``scratch``, preserving their relative paths.

    The proof entry point includes the kernel with a relative ``#[path]``, and
    the kernel includes the specification and lemmas the same way, so the
    directory shape is part of the proof input.
    """
    for source in (root / PROOF_DIR, root / KERNEL_DIR):
        destination = scratch / source.relative_to(root)
        shutil.copytree(source, destination)


def run_verus(verus: Path, z3: Path, scratch: Path) -> subprocess.CompletedProcess[str]:
    """Run the pinned Verus binary on the staged proof entry point."""
    environment = dict(os.environ)
    environment["VERUS_Z3_PATH"] = str(z3)
    try:
        return subprocess.run(
            [str(verus), str(PROOF_FILE)],
            cwd=scratch,
            env=environment,
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as error:
        raise ControlError(f"cannot run {verus}: {error}") from error


def results(output: str) -> tuple[int, int] | None:
    """The verified and error counts from Verus's summary, if it printed one."""
    match = RESULTS.search(output)
    if match is None:
        return None
    return int(match.group(1)), int(match.group(2))


def require_baseline(control_run: subprocess.CompletedProcess[str]) -> None:
    """Require the unmutated staged proof to verify with no errors."""
    output = control_run.stdout + control_run.stderr
    summary = results(output)
    if control_run.returncode != 0 or summary is None or summary[1] != 0:
        raise ControlError(
            "the unmodified staged proof did not verify, so no control can be "
            f"attributed to its edit (exit {control_run.returncode}); "
            f"summary: {summary}"
        )


def require_rejection(
    control: Control, control_run: subprocess.CompletedProcess[str]
) -> str:
    """Require Verus to reject the edited proof at the control's obligation."""
    output = control_run.stdout + control_run.stderr
    if control_run.returncode == 0:
        raise ControlError(f"control {control.name}: Verus accepted the defect")
    summary = results(output)
    if summary is None or summary[1] == 0:
        if not ERROR.search(output):
            raise ControlError(
                f"control {control.name}: Verus failed without reporting an "
                f"error (exit {control_run.returncode})"
            )
    missing = [expected for expected in control.expects if expected not in output]
    if missing:
        raise ControlError(
            f"control {control.name}: Verus failed, but the output lacks "
            f"{', '.join(repr(item) for item in missing)}"
        )
    return " + ".join(control.expects)


def check(root: Path, verus: Path, z3: Path) -> list[str]:
    """Run the baseline and every control in a fresh scratch tree.

    Returns one evidence line per control. Raises :class:`ControlError` on the
    first control that does not behave as required.
    """
    lines: list[str] = []
    with tempfile.TemporaryDirectory(prefix="polean-verus-controls-") as directory:
        scratch = Path(directory)
        stage(root, scratch)
        require_baseline(run_verus(verus, z3, scratch))
        staged_baseline = {
            path: (scratch / path).read_text(encoding="utf-8")
            for path in {control.path for control in CONTROLS}
        }
        for control in CONTROLS:
            for path, text in staged_baseline.items():
                (scratch / path).write_text(text, encoding="utf-8")
            (scratch / control.path).write_text(
                apply_control(root, control), encoding="utf-8"
            )
            obligation = require_rejection(control, run_verus(verus, z3, scratch))
            lines.append(f"{control.kind}:{control.name}: rejected at {obligation}")
    return lines


def main(argv: Sequence[str] | None = None) -> int:
    """Parse the command line and run the controls."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=ROOT, help="repository root")
    parser.add_argument("--verus", type=Path, help="pinned Verus binary")
    parser.add_argument("--z3", type=Path, help="pinned Z3 binary")
    arguments = parser.parse_args(argv)
    root = arguments.root.resolve()
    default_verus, default_z3 = verus_paths(root)
    verus = arguments.verus or default_verus
    z3 = arguments.z3 or default_z3
    if not verus.is_file():
        print(f"Verus is not installed at {verus}; run 'make install-verus'", file=sys.stderr)
        return 2
    if not z3.is_file():
        print(f"Z3 is not installed at {z3}; run 'make install-verus'", file=sys.stderr)
        return 2
    try:
        for line in check(root, verus, z3):
            print(line)
    except ControlError as error:
        print(f"negative control failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
