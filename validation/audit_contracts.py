"""Checks over the audit contracts, their samples, and the corpus manifest.

The same samples are read by the Rust decoder, so a schema and a decoder that
drift apart fail on both sides. The manifest is the reviewed expectation the
smoke test compares the worker against, and it is written by hand: this module
is what stops it being edited to match whatever the worker later reports. The
negative controls require each contract to reject one targeted fault, because a
contract that rejects everything proves as little as one that accepts
everything.
"""

from __future__ import annotations

import copy
import json
from typing import TYPE_CHECKING, Any

from harness import NEGATIVE_CONTROL_HELP, check_schema, require

if TYPE_CHECKING:
    from jsonschema import Draft202012Validator

    from validate_design import Repository


# The audit corpus from Table 2 of the 1.2.3 ExecPlan. This is the reviewed
# authority the hand-written manifest is checked against, never a copy of it:
# the manifest must match these entries exactly, so editing one line of the
# manifest to suit the worker cannot pass unnoticed.
AUDIT_CORPUS = (
    ("AcceptedGround", "accepted", None, ()),
    ("AcceptedFoundations", "accepted", None, ()),
    ("AcceptedClaimShape", "accepted", None, ()),
    ("MaximalPolicy", "accepted", None, ()),
    ("InitSentinel", "accepted", None, ()),
    ("MixedRoots", "rejected", None, ((1, "sorry_axiom"),)),
    ("Sorry", "rejected", None, ((0, "sorry_axiom"),)),
    ("SorryImported", "rejected", None, ((0, "sorry_axiom"),)),
    ("Native", "rejected", None, ((0, "native_axiom"),)),
    ("Undeclared", "rejected", None, ((0, "undeclared_axiom"),)),
    ("ReachImported", "rejected", None, ((0, "undeclared_axiom"),)),
    ("NarrowedFoundations", "rejected", None, ((0, "undeclared_axiom"),)),
    ("WrongStatement", "rejected", None, ((0, "statement_mismatch"),)),
    ("WrongPolicyLiteral", "rejected", None, ((0, "statement_mismatch"),)),
    ("Unfolded", "rejected", None, ((0, "statement_mismatch"),)),
    ("AxiomRoot", "rejected", None,
     ((0, "wrong_kind"), (0, "undeclared_axiom"))),
    ("DefRoot", "rejected", None, ((0, "wrong_kind"),)),
    ("RootAbsent", "rejected", None, ((0, "root_absent"),)),
    ("ForgeInit", "rejected", None, ((None, "kernel_rejected"),)),
    ("ForgeImported", "rejected", None, ((None, "kernel_rejected"),)),
    ("ModuleAbsent", "tool_failure", "worker_failed", ()),
    ("WorkerGarbled", "tool_failure", "malformed_report", ()),
)
AUDIT_FINDING_CODES = frozenset({
    "kernel_rejected", "root_absent", "wrong_kind", "statement_mismatch",
    "sorry_axiom", "native_axiom", "undeclared_axiom",
})
AUDIT_MANIFEST_KEYS = frozenset({
    "fixture", "module", "settlement", "findings", "reason",
})
AUDIT_MANIFEST = "examples/audit/manifest.json"
AUDIT_REQUEST_SAMPLE = "examples/audit/sample-request.json"
AUDIT_REPORT_SAMPLE = "examples/audit/sample-report.json"


def audit_controls(validators: dict[str, Draft202012Validator],
                   request_sample: dict[str, Any],
                   report_sample: dict[str, Any]) -> list[tuple[str, bool, bool]]:
    """Pair the accepted audit samples with mutations the contracts must reject.

    The report contract deliberately has no verdict field, so the first control
    is the one that keeps a worker from choosing its own outcome. The rest cover
    the required nullable fields, the closed declaration-kind set, the ground
    tree's two exclusive forms, the integer range a name component may use, and
    the set semantics of the approved foundations.
    """

    def request_accepted(document: dict[str, Any]) -> bool:
        """Report whether the request contract admits a document."""
        return validators["audit-request"].is_valid(document)

    def report_accepted(document: dict[str, Any]) -> bool:
        """Report whether the report contract admits a document."""
        return validators["audit-report"].is_valid(document)

    no_roots = copy.deepcopy(request_sample)
    no_roots["roots"] = []
    extra_entry = copy.deepcopy(request_sample)
    extra_entry["expected"] = []
    empty_component = copy.deepcopy(request_sample)
    empty_component["module"] = ["AuditFixture", ""]
    duplicate_foundation = copy.deepcopy(request_sample)
    first_foundation = duplicate_foundation["foundations"][0]
    duplicate_foundation["foundations"] = [first_foundation, first_foundation]
    root_without_expected = copy.deepcopy(request_sample)
    del root_without_expected["roots"][0]["expected"]

    verdict = copy.deepcopy(report_sample)
    verdict["accepted"] = True
    missing_declaration = copy.deepcopy(report_sample)
    del missing_declaration["roots"][0]["declaration"]
    missing_statement = copy.deepcopy(report_sample)
    del missing_statement["roots"][0]["declaration"]["statement"]
    unknown_kind = copy.deepcopy(report_sample)
    unknown_kind["roots"][0]["declaration"]["kind"] = "lemma"
    both_forms = copy.deepcopy(report_sample)
    both_forms["roots"][0]["declaration"]["statement"]["app"] = [
        {"const": ["AuditFixture", "Goal"], "levels": []},
        {"const": ["AuditFixture", "Goal"], "levels": []},
    ]
    oversized_component = copy.deepcopy(report_sample)
    oversized_component["roots"][0]["name"] = ["AuditFixture", 1 << 64]
    rejected_without_message = copy.deepcopy(report_sample)
    rejected_without_message["replay"] = {"status": "rejected"}

    return [
        ("audit request rejects an empty roots list",
         request_accepted(request_sample), request_accepted(no_roots)),
        ("audit request rejects an unreviewed top-level entry",
         request_accepted(request_sample), request_accepted(extra_entry)),
        ("audit request rejects an empty name component",
         request_accepted(request_sample), request_accepted(empty_component)),
        ("audit request rejects a repeated foundation",
         request_accepted(request_sample), request_accepted(duplicate_foundation)),
        ("audit request rejects a root without an expected statement",
         request_accepted(request_sample), request_accepted(root_without_expected)),
        ("audit report rejects a verdict field",
         report_accepted(report_sample), report_accepted(verdict)),
        ("audit report rejects a missing declaration field",
         report_accepted(report_sample), report_accepted(missing_declaration)),
        ("audit report rejects a missing statement field",
         report_accepted(report_sample), report_accepted(missing_statement)),
        ("audit report rejects an unknown declaration kind",
         report_accepted(report_sample), report_accepted(unknown_kind)),
        ("audit report rejects a tree with both ground forms",
         report_accepted(report_sample), report_accepted(both_forms)),
        ("audit report rejects a name component above u64",
         report_accepted(report_sample), report_accepted(oversized_component)),
        ("audit report rejects a rejected replay without a message",
         report_accepted(report_sample), report_accepted(rejected_without_message)),
    ]


class ManifestLineError(AssertionError):
    """Report one audit manifest line that is not valid JSON."""

    def __init__(self, number: int, error: json.JSONDecodeError) -> None:
        super().__init__(f"{AUDIT_MANIFEST} line {number} is not JSON: {error}")


def read_audit_manifest(repo: Repository) -> list[dict[str, Any]]:
    """Read the hand-written corpus manifest and hold it to the reviewed table.

    The manifest is JSON Lines, one object per corpus entry. It is written by
    hand before the worker exists, so this check is what stops it being edited
    to match whatever the worker later reports. Findings are compared in order,
    because settlement emits them in a fixed order and the smoke test compares
    the same sequence.
    """
    text = repo.read_text(AUDIT_MANIFEST)
    lines = [line for line in text.splitlines() if line.strip()]
    require(len(lines) == len(AUDIT_CORPUS),
            f"{AUDIT_MANIFEST} must hold {len(AUDIT_CORPUS)} entries, "
            f"found {len(lines)}")
    reviewed = {fixture: (settlement, reason, findings)
                for fixture, settlement, reason, findings in AUDIT_CORPUS}
    entries: list[dict[str, Any]] = []
    seen: set[str] = set()
    for number, line in enumerate(lines, start=1):
        try:
            entry = json.loads(line)
        except json.JSONDecodeError as error:
            raise ManifestLineError(number, error) from error
        require(isinstance(entry, dict),
                f"{AUDIT_MANIFEST} line {number} is not an object")
        require(set(entry) == AUDIT_MANIFEST_KEYS,
                f"{AUDIT_MANIFEST} line {number} has entries "
                f"{sorted(set(entry))}")
        fixture = entry["fixture"]
        require(isinstance(fixture, str) and fixture not in seen,
                f"{AUDIT_MANIFEST} line {number} repeats or omits a fixture name")
        seen.add(fixture)
        require(fixture in reviewed,
                f"{AUDIT_MANIFEST} line {number} names an unreviewed fixture "
                f"{fixture!r}")
        require(entry["module"] == ["AuditFixture", fixture],
                f"{AUDIT_MANIFEST} line {number} names the wrong module")
        settlement, reason, findings = reviewed[fixture]
        require(entry["settlement"] == settlement,
                f"{AUDIT_MANIFEST} line {number} expects {entry['settlement']!r}, "
                f"reviewed as {settlement!r}")
        require(entry["reason"] == reason,
                f"{AUDIT_MANIFEST} line {number} gives reason {entry['reason']!r}, "
                f"reviewed as {reason!r}")
        require(isinstance(entry["findings"], list),
                f"{AUDIT_MANIFEST} line {number} has findings that are not a list")
        observed = []
        for finding in entry["findings"]:
            require(isinstance(finding, dict)
                    and set(finding) == {"root", "code"},
                    f"{AUDIT_MANIFEST} line {number} has a malformed finding")
            require(isinstance(finding["code"], str)
                    and finding["code"] in AUDIT_FINDING_CODES,
                    f"{AUDIT_MANIFEST} line {number} names an unknown finding "
                    f"code {finding['code']!r}")
            require(finding["root"] is None
                    or (isinstance(finding["root"], int)
                        and not isinstance(finding["root"], bool)
                        and finding["root"] >= 0),
                    f"{AUDIT_MANIFEST} line {number} has a malformed root index")
            observed.append((finding["root"], finding["code"]))
        require(tuple(observed) == findings,
                f"{AUDIT_MANIFEST} line {number} expects findings {observed}, "
                f"reviewed as {list(findings)}")
        entries.append(entry)
    require(seen == set(reviewed),
            f"{AUDIT_MANIFEST} omits reviewed fixtures "
            f"{sorted(set(reviewed) - seen)}")
    return entries


def check_audit_contracts(repo: Repository,
                          validators: dict[str, Draft202012Validator]
                          ) -> tuple[int, int]:
    """Validate the audit contracts, their samples, and the corpus manifest.

    The samples are the one place the Python harness and the Rust decoder read
    the same bytes, so a schema and a decoder that drift apart are caught by
    both sides failing on the same document. The negative controls require each
    contract to reject one targeted fault, because a contract that rejects
    everything proves as little as one that accepts everything.
    """
    request_sample = repo.read_json(AUDIT_REQUEST_SAMPLE)
    report_sample = repo.read_json(AUDIT_REPORT_SAMPLE)
    check_schema(validators, "audit-request", request_sample, AUDIT_REQUEST_SAMPLE)
    check_schema(validators, "audit-report", report_sample, AUDIT_REPORT_SAMPLE)
    require(report_sample["module"] == request_sample["module"],
            "Audit samples disagree about the job module")
    require([root["name"] for root in report_sample["roots"]]
            == [root["name"] for root in request_sample["roots"]],
            "Audit samples disagree about the audited roots")
    require(report_sample["roots"][0]["declaration"]["statement"]
            == request_sample["roots"][0]["expected"],
            "Audit samples disagree about the expected statement")
    controls = audit_controls(validators, request_sample, report_sample)
    for description, accepted, still_accepted in controls:
        require(accepted, f"{description}: the reviewed sample was rejected")
        require(not still_accepted, f"{description}: {NEGATIVE_CONTROL_HELP}")
    entries = read_audit_manifest(repo)
    return len(controls), len(entries)
