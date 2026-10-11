#!/usr/bin/env python3
"""Test the design harness's orchestration and storage boundaries.

``validate_design.py`` checks the fixtures; this module checks the checker.
The negative controls inside the harness exercise lower-level predicates such
as the schema validators and ``check_witness``, so on their own they would not
notice a ``validate_policies`` replaced by one returning a constant review: the
publication path replays whatever review it is handed, and a constant review
whose data matched the committed witnesses would publish it happily. These
tests therefore call the real entry points with deliberately altered inputs and
require the result to change or be rejected.

Two properties are asserted about storage. A run that fails a check must leave
the committed summary and witnesses byte-for-byte and timestamp-for-timestamp
untouched, and a run that succeeds must publish through the workspace it was
given rather than through a module-level path. Neither is established by
reading the code: both are observed by snapshotting the tree.

Run directly (``python validation/test_boundary.py``); it needs no test
framework beyond the standard library and the harness's own dependencies.
"""
from __future__ import annotations

import copy
import dataclasses
import hashlib
import itertools
import json
import sys
import tempfile
from collections.abc import Callable
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))

import validate_design as VD  # noqa: E402  (needs the path entry above)

ROOT = VD.ROOT


class TestFailure(AssertionError):
    """A boundary test's expectation was not met."""


def snapshot(root: Path) -> dict[str, tuple[int, int, str]]:
    """Record every witness's size, modification time, and content digest.

    All three are recorded because none implies the others: a same-size rewrite
    that restores the timestamp changes only the digest, and a rewrite to
    identical bytes changes only the timestamp.
    """
    recorded: dict[str, tuple[int, int, str]] = {}
    for path in sorted(root.glob(f"{VD.WITNESS_DIRECTORY}/*.json")):
        stat = path.stat()
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        recorded[path.name] = (stat.st_size, stat.st_mtime_ns, digest)
    return recorded


class Tampered(VD.Repository):
    """A repository that hands back a mutated document for one path.

    The real ``Repository`` is kept underneath, so a test changes exactly one
    document and every other read is the committed one. That makes the
    mutation the only difference a rejected run can be reacting to. The
    mutation is applied in ``read_text``, which ``read_json`` already routes
    through, so one override covers both JSON and Rego documents.
    """

    def __init__(self, root: Path, relative: str,
                 mutate: Callable[[str], str]) -> None:
        super().__init__(root)
        self.relative = relative
        self.mutate = mutate

    def read_text(self, relative: str) -> str:
        text = super().read_text(relative)
        return self.mutate(text) if relative == self.relative else text


def edit_json(mutate: Callable[[Any], Any]) -> Callable[[str], str]:
    """Lift a mutation of a parsed document to one over its text."""
    def apply(text: str) -> str:
        return json.dumps(mutate(json.loads(text)), ensure_ascii=False, indent=2)

    return apply


def expect_rejected(description: str, action: Callable[[], Any]) -> None:
    """Require an action to fail a check rather than return a review.

    Only the harness's own two failure shapes count. Any other exception is a
    defect in the harness rather than a rejection, so it propagates and fails
    the test rather than being counted as a pass.
    """
    try:
        action()
    except (AssertionError, VD.HarnessIOError):
        return
    raise TestFailure(f"{description}: accepted a fixture that should be rejected")


def fixture_review(repo: VD.Repository) -> VD.PolicyReview:
    """Load fixtures from a repository and produce the review they imply."""
    fixtures = VD.Fixtures.load(repo)
    facts = [dict(zip(VD.FACT_KEYS, values, strict=True))
             for values in itertools.product((False, True),
                                             repeat=len(VD.FACT_KEYS))]
    return VD.validate_policies(fixtures.validators, fixtures, facts)


def test_a_mutated_policy_changes_the_review() -> None:
    """A policy whose rules no longer refute their claims must be rejected.

    The variant's rendered source is regenerated from the mutated rule list, so
    the source-drift check passes and the run has to notice the semantic
    change. A ``validate_policies`` that returned a constant review, or one
    that never evaluated the rules, would accept this.
    """
    def deny_everything(policy: dict[str, Any]) -> dict[str, Any]:
        mutated = copy.deepcopy(policy)
        mutated["rules"] = [["same_tenant", "write_action", "admin_role"]]
        return mutated

    tampered = Tampered(ROOT, VD.policy_folder("baseline") + "/policy.json",
                        edit_json(deny_everything))
    expect_rejected("a baseline policy denying everything",
                    lambda: fixture_review(tampered))


def test_source_drift_is_rejected() -> None:
    """Rendered source that no longer matches the committed fixture fails."""
    def blank_the_source(source: str) -> str:
        return source + "\n# hand edit\n"

    tampered = Tampered(ROOT, VD.policy_folder("baseline") + "/policy.rego",
                        blank_the_source)
    expect_rejected("a hand-edited fixture",
                    lambda: fixture_review(tampered))


def test_a_mutated_profile_widening_the_grammar_is_rejected() -> None:
    """A profile that admits an extra atom cannot pass on the documents alone.

    The admitted vocabulary is a reviewed constant of the harness rather than a
    consequence of the profile document, so widening the document must fail
    the run. This is checked through ``run_checks`` because that is where the
    profile contract is compared against the constant.
    """
    def widen(profile: dict[str, Any]) -> dict[str, Any]:
        mutated = copy.deepcopy(profile)
        mutated["atoms"]["is_root"] = "input.subject.is_root"
        return mutated

    tampered = Tampered(ROOT, "contracts/profile.json", edit_json(widen))
    with tempfile.TemporaryDirectory(prefix="polean-boundary-") as directory:
        workspace = VD.Workspace(Path(directory))
        expect_rejected("a profile with an extra atom",
                        lambda: VD.run_checks(tampered, workspace))


def flip_the_decision(witness: dict[str, Any]) -> dict[str, Any]:
    """Return a witness whose first recorded decision is inverted."""
    mutated = copy.deepcopy(witness)
    first = next(iter(mutated))
    mutated[first]["decision"] = not mutated[first]["decision"]
    return mutated


def test_a_mutated_witness_is_rejected() -> None:
    """A stored witness that no longer refutes its claim is rejected.

    Publication rewrites every witness from the review before replaying it, so
    the stored file is not what that replay reads. The witness controls are
    what consume the committed document, and they run before publication.
    """
    relative = f"{VD.WITNESS_DIRECTORY}/deny-all.json"
    tampered = Tampered(ROOT, relative, edit_json(flip_the_decision))
    with tempfile.TemporaryDirectory(prefix="polean-boundary-") as directory:
        workspace = VD.Workspace(Path(directory))
        expect_rejected("a witness with a flipped decision",
                        lambda: VD.run_checks(tampered, workspace))
        require_empty(directory)


def test_a_missing_witness_is_a_named_boundary_error() -> None:
    """An absent ``deny-all.json`` is reported, not raised as a ``KeyError``.

    The witness controls use the committed deny-all witness as their accepted
    member, so a checkout without it must produce the harness's own diagnostic
    rather than a traceback from an unguarded lookup. The fixture set is
    rebuilt with that one witness removed, which is what a fresh clone without
    the file produces.
    """
    fixtures = VD.Fixtures.load(VD.Repository(ROOT))
    without_deny_all = dataclasses.replace(
        fixtures,
        stored_witnesses={name: value for name, value
                          in fixtures.stored_witnesses.items()
                          if name != "deny-all.json"},
    )
    if "deny-all.json" in without_deny_all.stored_witnesses:
        raise TestFailure("the test did not remove the deny-all witness")
    try:
        VD.witness_controls(without_deny_all)
    except VD.HarnessIOError:
        return
    except KeyError as error:
        raise TestFailure(
            f"an absent witness raised {error!r} instead of HarnessIOError") from error
    raise TestFailure("an absent witness was accepted")


def test_a_failed_run_leaves_the_tree_untouched() -> None:
    """A rejected fixture must not publish a summary or rewrite a witness."""
    def blank_the_source(source: str) -> str:
        return source + "\n# hand edit\n"

    before_witnesses = snapshot(ROOT)
    before_summary = VD.Repository(ROOT).state(VD.SUMMARY_DOCUMENT)
    tampered = Tampered(ROOT, VD.policy_folder("baseline") + "/policy.rego",
                        blank_the_source)
    with tempfile.TemporaryDirectory(prefix="polean-boundary-") as directory:
        workspace = VD.Workspace(Path(directory))
        expect_rejected("a hand-edited fixture",
                        lambda: VD.run_checks(tampered, workspace))
        require_empty(directory)
    if snapshot(ROOT) != before_witnesses:
        raise TestFailure("a failed run rewrote a committed witness")
    if VD.Repository(ROOT).state(VD.SUMMARY_DOCUMENT) != before_summary:
        raise TestFailure("a failed run rewrote the committed summary")


def require_empty(directory: str) -> None:
    """Require a temporary workspace to hold nothing at all."""
    written = sorted(path.name for path in Path(directory).rglob("*"))
    if written:
        raise TestFailure(f"a failed run wrote to its workspace: {written}")


def test_publication_uses_the_workspace_it_is_given() -> None:
    """A successful run writes under the supplied root and not the repository."""
    before_witnesses = snapshot(ROOT)
    before_summary = VD.Repository(ROOT).state(VD.SUMMARY_DOCUMENT)
    with tempfile.TemporaryDirectory(prefix="polean-boundary-") as directory:
        workspace = VD.Workspace(Path(directory))
        summary = VD.run_checks(VD.Repository(ROOT), workspace)
        written = sorted(path.name for path
                         in (workspace.path(VD.WITNESS_DIRECTORY)).glob("*.json"))
        expected = sorted(_expected_witness_names())
        if written != expected:
            raise TestFailure(f"published {written}, expected {expected}")
        published = workspace.read_json(VD.SUMMARY_DOCUMENT)
        if published != summary:
            raise TestFailure("the returned summary is not the published one")
        committed = VD.Repository(ROOT).read_json(VD.SUMMARY_DOCUMENT)
        if published != committed:
            raise TestFailure("a fresh run disagreed with the committed summary")
    if snapshot(ROOT) != before_witnesses:
        raise TestFailure("a successful run modified a committed witness")
    if VD.Repository(ROOT).state(VD.SUMMARY_DOCUMENT) != before_summary:
        raise TestFailure("a successful run modified the committed summary")


def _expected_witness_names() -> list[str]:
    """List the witness files the committed mutation set must produce."""
    return [f"{name}.json" for name, failures in VD.EXPECTED_FAILURES.items()
            if failures]


def test_stale_witnesses_are_rejected_before_anything_is_written() -> None:
    """An unexpected witness file fails the run without touching the tree.

    The stale set is computed from the directory as it was found, so a
    workspace holding a retired mutation's counterexample must be rejected
    before the run writes its own witnesses over the top. The seeded name
    belongs to no reviewed variant, which is what makes it stale.
    """
    with tempfile.TemporaryDirectory(prefix="polean-boundary-") as directory:
        workspace = VD.Workspace(Path(directory))
        stale = workspace.path(f"{VD.WITNESS_DIRECTORY}/retired-mutation.json")
        stale.parent.mkdir(parents=True, exist_ok=True)
        stale.write_text("{}\n", encoding="utf-8")
        before = stale.read_bytes()
        expect_rejected("a workspace with a stale witness",
                        lambda: VD.run_checks(VD.Repository(ROOT), workspace))
        written = sorted(path.name for path in stale.parent.glob("*.json"))
        if written != ["retired-mutation.json"]:
            raise TestFailure(f"the run published over a stale workspace: {written}")
        if stale.read_bytes() != before:
            raise TestFailure("the run rewrote the stale witness it rejected")


def published_witnesses(workspace: VD.Workspace) -> dict[str, bytes]:
    """Record every published witness under a workspace by name and content."""
    directory = workspace.path(VD.WITNESS_DIRECTORY)
    return {path.name: path.read_bytes() for path in directory.glob("*.json")}


def test_a_corrected_variant_has_its_witness_removed_not_rejected() -> None:
    """A witness whose variant stopped refuting must be removed, not called stale.

    A reviewed variant whose failures disappear is scheduled for removal. If
    staleness were computed from the variants that still refute something, that
    surviving file would be reported as stale and the run would fail before
    reaching the removal — losing the behaviour that keeps a corrected
    mutation from leaving its counterexample behind.

    The surviving witnesses are compared by content, not only by name: their
    bytes must match what the first publication wrote, so removing one variant
    cannot quietly rewrite another. Their modification times are not compared,
    because publication rewrites every refuting witness on each run.
    """
    with tempfile.TemporaryDirectory(prefix="polean-boundary-") as directory:
        workspace = VD.Workspace(Path(directory))
        review = fixture_review(VD.Repository(ROOT))
        VD.publish_witnesses(workspace, review)
        published = published_witnesses(workspace)
        if "admin-only.json" not in published:
            raise TestFailure(
                f"the fixture witness set is unexpected: {sorted(published)}")

        corrected = dataclasses.replace(
            review,
            witnesses={**review.witnesses, "admin-only": []},
        )
        replays = VD.publish_witnesses(workspace, corrected)
        remaining = published_witnesses(workspace)
        if "admin-only.json" in remaining:
            raise TestFailure("a corrected variant kept its witness")
        if remaining != {name: body for name, body in published.items()
                         if name != "admin-only.json"}:
            raise TestFailure(
                f"removal disturbed another witness: {sorted(remaining)}")
        if replays != len(remaining):
            raise TestFailure(f"replayed {replays}, expected {len(remaining)}")


def test_the_property_suite_needs_no_global_configuration() -> None:
    """A suite runs from its own admitted data, with no module state set up.

    The properties previously drew on a module-level value written by
    ``configure``, which made a property depend on setup that had to happen
    first. Constructing the suite is now the only way to supply its inputs, and
    this checks that a freshly built suite runs every property without any
    configuration step, and that the module exposes no configuration global.
    """
    import property_fixtures as PF

    if hasattr(PF, "_ADMITTED") or hasattr(PF, "configure"):
        raise TestFailure("the property module still exposes configuration state")
    suite = PF.Suite(PF.load_admitted(VD.Repository(ROOT)))
    for name in PF.PROPERTY_NAMES:
        check = getattr(suite, name)
        if not getattr(check, "is_hypothesis_test", False):
            raise TestFailure(f"{name} lost its @given decorator")
        check()


def test_the_property_suite_reads_the_profile_it_is_handed() -> None:
    """The suite takes its atoms from the given repository, not a hard-coded root.

    A property suite that resolved ``ROOT`` for itself would ignore the
    repository it was handed. This passes a repository whose profile differs
    and requires the difference to arrive.
    """
    import property_fixtures as PF

    def rename_an_atom(profile: dict[str, Any]) -> dict[str, Any]:
        mutated = copy.deepcopy(profile)
        mutated["atoms"]["same_tenant"] = "tenant_match"
        return mutated

    tampered = Tampered(ROOT, "contracts/profile.json", edit_json(rename_an_atom))
    admitted = PF.load_admitted(tampered)
    if admitted.atoms["same_tenant"] != "tenant_match":
        raise TestFailure("the suite did not read the profile it was handed")
    if admitted.atom_sources["tenant_match"] != "same_tenant":
        raise TestFailure("the decoded atom sources were not rebuilt")


def main() -> int:
    """Run every boundary test and report one line per check."""
    tests = [
        test_a_mutated_policy_changes_the_review,
        test_source_drift_is_rejected,
        test_a_mutated_profile_widening_the_grammar_is_rejected,
        test_a_mutated_witness_is_rejected,
        test_a_missing_witness_is_a_named_boundary_error,
        test_a_failed_run_leaves_the_tree_untouched,
        test_publication_uses_the_workspace_it_is_given,
        test_stale_witnesses_are_rejected_before_anything_is_written,
        test_a_corrected_variant_has_its_witness_removed_not_rejected,
        test_the_property_suite_needs_no_global_configuration,
        test_the_property_suite_reads_the_profile_it_is_handed,
    ]
    for test in tests:
        test()
        print(f"ok  {test.__name__}")
    print(f"\n{len(tests)} boundary tests passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
