#!/usr/bin/env python3
"""Validate Polean design fixtures, not the proposed checker or Lean proofs.

Run from any directory with Python 3.11+ and jsonschema 4.x installed.
This script does not parse Rego, execute OPA/Regorus, or invoke Lean.

The reviewed documents are read through the read-only ``Repository`` boundary
and handed to the checks as data, so an ordinary check cannot reach the
filesystem at all; ``check_boundaries`` is the one exception, and it reads the
repository only to prove that the other checks changed nothing. The publishing
command is the only holder of the ``Workspace`` write boundary. A read or write
that fails is reported as one ``HarnessIOError`` naming the file, a run that
fails a check publishes nothing, and any other exception keeps its traceback
because it is a defect in the harness rather than a finding about a fixture.
"""
from __future__ import annotations

import copy
import itertools
import json
import sys
import tempfile
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
FACT_KEYS = ("same_tenant", "write_action", "admin_role", "owns_resource", "locked")
CLAIM_IDS = (
    "tenant-isolation", "write-only", "locked-requires-admin", "admin-or-owner",
    "admin-write-available", "owner-write-available",
)
EXPECTED_FAILURES = {
    "baseline": set(),
    "cross-tenant-admin": {"tenant-isolation"},
    "locked-owner": {"locked-requires-admin"},
    "unowned-unlocked": {"admin-or-owner"},
    "non-write-admin": {"write-only"},
    "deny-all": {"admin-write-available", "owner-write-available"},
    "admin-only": {"owner-write-available"},
}
# The admitted vocabulary is a reviewed constant of this harness, not a
# consequence of whatever contracts/profile.json currently says. Profiles,
# rules, and rendered source are checked against these tuples, so widening the
# profile or the grammar cannot pass by editing one document.
PROFILE_ATOM_NAMES = (
    "same_tenant", "write_action", "admin_role", "owns_resource", "locked", "unlocked",
)
PROFILE_EXCLUDED = (
    "other packages", "other imports", "additional modules", "metadata annotations",
    "other rules", "variables", "unification", "negation", "else", "with", "some",
    "every", "built-in calls", "functions", "data references", "dynamic references",
    "numeric expressions", "collections", "arbitrary Lean source", "custom invariants",
)
NEGATIVE_CONTROL_HELP = "a negative control did not detect its injected fault"
# Repository-relative, so the boundary classes below are the only place a path
# is turned into a filesystem operation.
WITNESS_DIRECTORY = "validation/witnesses"
SUMMARY_DOCUMENT = "validation/design-validation.json"
# Module-level documentation for the seven source fixtures. Ordinary comments
# carry no semantics in the admitted profile, so this text documents each
# fixture without widening the grammar or the policy contract. The harness
# holds the reviewed text and re-renders it into every check, which is why a
# fixture whose comment drifts from this mapping fails the source check rather
# than being silently tolerated.
SOURCE_HEADERS = {
    "baseline": (
        "# Polean tenant-write v0 baseline policy fixture.\n"
        "#\n"
        "# Purpose: the admitted reference module for profile polean.tenant-write.v0.\n"
        "# It satisfies all six claims registered in examples/tenant-write/claims.json,\n"
        "# so it defines the intended decision that each mutation fixture departs from.\n"
        "#\n"
        "# examples/tenant-write/policy.json holds the equivalent rule-list IR.\n"
        "# validation/validate_design.py re-renders that IR into this text and fails on\n"
        "# any byte difference between the two.\n"
        "\n"
    ),
    "admin-only": (
        "# Polean tenant-write v0 mutation fixture: admin-only.\n"
        "#\n"
        "# Purpose: a deliberately weakened policy that drops the owner rule, so\n"
        "# owner-write-available must fail while the other five claims still hold.\n"
        "#\n"
        "# examples/tenant-write/mutations/admin-only/policy.json holds the equivalent\n"
        "# rule-list IR. validation/validate_design.py re-renders that IR into this\n"
        "# text, requires the variant to differ from the baseline only inside its rule\n"
        "# list, and records the counterexample in validation/witnesses/admin-only.json.\n"
        "\n"
    ),
    "cross-tenant-admin": (
        "# Polean tenant-write v0 mutation fixture: cross-tenant-admin.\n"
        "#\n"
        "# Purpose: a deliberately weakened policy that drops the administrator tenant\n"
        "# guard, so tenant-isolation must fail while the other five claims still hold.\n"
        "#\n"
        "# examples/tenant-write/mutations/cross-tenant-admin/policy.json holds the\n"
        "# equivalent rule-list IR. validation/validate_design.py re-renders that IR into\n"
        "# this text, requires the variant to differ from the baseline only inside its\n"
        "# rule list, and records the counterexample in\n"
        "# validation/witnesses/cross-tenant-admin.json.\n"
        "\n"
    ),
    "deny-all": (
        "# Polean tenant-write v0 mutation fixture: deny-all.\n"
        "#\n"
        "# Purpose: a deliberately weakened policy that keeps no non-default rules, so\n"
        "# both availability claims must fail while the four safety claims still hold.\n"
        "# This fixture is what stops the remaining claims from passing by vacuity.\n"
        "#\n"
        "# examples/tenant-write/mutations/deny-all/policy.json holds the equivalent\n"
        "# rule-list IR. validation/validate_design.py re-renders that IR into this\n"
        "# text, requires the variant to differ from the baseline only inside its rule\n"
        "# list, and records the counterexample in validation/witnesses/deny-all.json.\n"
        "\n"
    ),
    "locked-owner": (
        "# Polean tenant-write v0 mutation fixture: locked-owner.\n"
        "#\n"
        "# Purpose: a deliberately weakened policy that drops the owner lock guard, so\n"
        "# locked-requires-admin must fail while the other five claims still hold.\n"
        "#\n"
        "# examples/tenant-write/mutations/locked-owner/policy.json holds the equivalent\n"
        "# rule-list IR. validation/validate_design.py re-renders that IR into this\n"
        "# text, requires the variant to differ from the baseline only inside its rule\n"
        "# list, and records the counterexample in validation/witnesses/locked-owner.json.\n"
        "\n"
    ),
    "non-write-admin": (
        "# Polean tenant-write v0 mutation fixture: non-write-admin.\n"
        "#\n"
        "# Purpose: a deliberately weakened policy that drops the administrator action\n"
        "# guard, so write-only must fail while the other five claims still hold.\n"
        "#\n"
        "# examples/tenant-write/mutations/non-write-admin/policy.json holds the\n"
        "# equivalent rule-list IR. validation/validate_design.py re-renders that IR into\n"
        "# this text, requires the variant to differ from the baseline only inside its\n"
        "# rule list, and records the counterexample in\n"
        "# validation/witnesses/non-write-admin.json.\n"
        "\n"
    ),
    "unowned-unlocked": (
        "# Polean tenant-write v0 mutation fixture: unowned-unlocked.\n"
        "#\n"
        "# Purpose: a deliberately weakened policy that drops the owner identity guard,\n"
        "# so admin-or-owner must fail while the other five claims still hold.\n"
        "#\n"
        "# examples/tenant-write/mutations/unowned-unlocked/policy.json holds the\n"
        "# equivalent rule-list IR. validation/validate_design.py re-renders that IR into\n"
        "# this text, requires the variant to differ from the baseline only inside its\n"
        "# rule list, and records the counterexample in\n"
        "# validation/witnesses/unowned-unlocked.json.\n"
        "\n"
    ),
}


class HarnessIOError(RuntimeError):
    """One filesystem failure at the harness boundary, naming the file involved.

    A missing document, an unreadable one, malformed JSON, or a failed write is
    reported in this one shape, so the command can report it without guessing
    which library raised what.
    """


@dataclass(frozen=True)
class Repository:
    """Read-only access to the reviewed documents the checks consume.

    Every method either returns a value or raises ``HarnessIOError`` naming the
    document, and every path is built here from a repository-relative name, so
    a check names a document and receives its contents rather than reaching for
    a path of its own.
    """

    root: Path

    def path(self, relative: str) -> Path:
        """Return the absolute path of one repository-relative document."""
        return self.root / relative

    def read_text(self, relative: str) -> str:
        """Read one document as strict UTF-8, without repairing its contents."""
        try:
            return self.path(relative).read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            raise HarnessIOError(f"cannot read {relative}: {error}") from error

    def read_json(self, relative: str) -> Any:
        """Read one JSON document without repairing or normalising it."""
        try:
            return json.loads(self.read_text(relative))
        except json.JSONDecodeError as error:
            raise HarnessIOError(f"invalid JSON in {relative}: {error}") from error

    def list_json(self, directory: str) -> list[str]:
        """List the JSON documents in one directory, sorted by file name."""
        try:
            names = sorted(entry.name for entry in self.path(directory).glob("*.json"))
        except OSError as error:
            raise HarnessIOError(f"cannot list {directory}: {error}") from error
        return [f"{directory}/{name}" for name in names]

    def state(self, relative: str) -> tuple[int, int]:
        """Return a document's size and modification time for seam comparisons.

        Content alone cannot show whether a check rewrote a document with the
        same bytes, so the modification time is compared as well.
        """
        try:
            stat = self.path(relative).stat()
        except OSError as error:
            raise HarnessIOError(f"cannot stat {relative}: {error}") from error
        return stat.st_size, stat.st_mtime_ns


class Workspace(Repository):
    """The read-write boundary the publishing command alone receives.

    A workspace can do everything a repository can, plus the two writes the
    command needs, so the capability to change the tree appears in the
    command's parameter list instead of being implied by a module constant.
    """

    def write_json(self, relative: str, value: Any) -> None:
        """Write one JSON document, creating its directory if necessary."""
        try:
            path = self.path(relative)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n",
                            encoding="utf-8")
        except OSError as error:
            raise HarnessIOError(f"cannot write {relative}: {error}") from error

    def remove(self, relative: str) -> None:
        """Remove one document if it is present; absence is not a failure."""
        try:
            self.path(relative).unlink(missing_ok=True)
        except OSError as error:
            raise HarnessIOError(f"cannot remove {relative}: {error}") from error


def policy_folder(name: str) -> str:
    """Return the repository-relative folder holding one variant's contracts."""
    if name == "baseline":
        return "examples/tenant-write"
    return f"examples/tenant-write/mutations/{name}"


def load_schemas(repo: Repository) -> dict[str, Draft202012Validator]:
    """Meta-validate every contract schema and index it by contract name."""
    validators: dict[str, Draft202012Validator] = {}
    for relative in repo.list_json("contracts"):
        if not relative.endswith(".schema.json"):
            continue
        schema = repo.read_json(relative)
        Draft202012Validator.check_schema(schema)
        validators[Path(relative).stem.removesuffix(".schema")] = Draft202012Validator(schema)
    return validators


def extract(document: Any, label: str) -> Any:
    """Accept one folder's contract document and reject any unknown entry."""
    require(set(document) <= {"rules", "profile"},
            f"Unknown top-level entry in {label}: {sorted(set(document))}")
    return document


def facts_of(request: dict[str, Any]) -> dict[str, bool]:
    """Compute the five abstract facts by comparison, without a rule list.

    This is the design-stage stand-in for the abstraction α the technical
    design defines over typed request fields: equality of the two tenant names,
    the literal write action, the literal administrator role, equality of
    subject and owner identifiers, and the locked flag. It reads the request
    directly so that ``eval_request`` remains a separate concrete evaluator.
    """
    subject, resource = request["subject"], request["resource"]
    return {
        "same_tenant": subject["tenant"] == resource["tenant"],
        "write_action": request["action"] == "write",
        "admin_role": subject["role"] == "admin",
        "owns_resource": subject["id"] == resource["owner"],
        "locked": resource["locked"],
    }


def concretize(facts: dict[str, bool]) -> dict[str, Any]:
    """Choose one schema-valid request whose facts are exactly the given ones.

    The design calls this γ. It fixes two tenant names, two subject
    identifiers, administrator versus member, write versus read, and the locked
    flag, so that ``facts_of(concretize(f)) == f`` for every fact vector. The
    check that this holds is what lets a witness for an abstract counterexample
    be replayed as a concrete request.
    """
    return {
        "subject": {
            "tenant": "tenant-a", "id": "subject-a",
            "role": "admin" if facts["admin_role"] else "member",
        },
        "resource": {
            "tenant": "tenant-a" if facts["same_tenant"] else "tenant-b",
            "owner": "subject-a" if facts["owns_resource"] else "subject-b",
            "locked": facts["locked"],
        },
        "action": "write" if facts["write_action"] else "read",
    }


def eval_facts(rules: list[list[str]], facts: dict[str, bool]) -> bool:
    """Evaluate a rule list against facts directly. This is ``evalFacts``.

    ``unlocked`` is the derived negation of the locked fact rather than an
    independent sixth fact, matching the rule that a rule says ``allow`` when
    any one of its bodies holds and every comparison in that body holds.
    """
    def atom(name: str) -> bool:
        """Decide one named atom, deriving ``unlocked`` from the locked fact."""
        return not facts["locked"] if name == "unlocked" else facts[name]
    return any(all(atom(name) for name in rule) for rule in rules)


def eval_request(rules: list[list[str]], r: dict[str, Any]) -> bool:
    """A direct concrete interpretation, separate from facts_of."""
    def atom(name: str) -> bool:
        """Decide one named atom from the request's own fields."""
        if name == "same_tenant":
            return r["subject"]["tenant"] == r["resource"]["tenant"]
        if name == "write_action":
            return r["action"] == "write"
        if name == "admin_role":
            return r["subject"]["role"] == "admin"
        if name == "owns_resource":
            return r["subject"]["id"] == r["resource"]["owner"]
        if name == "locked":
            return r["resource"]["locked"] is True
        if name == "unlocked":
            return r["resource"]["locked"] is False
        raise ValueError(f"Unsupported design atom: {name}")
    return any(all(atom(name) for name in rule) for rule in rules)


def claims(f: dict[str, bool], allowed: bool) -> dict[str, bool]:
    """Report which of the six registered claims one fact vector satisfies.

    A safety claim holds when the decision does not grant the access the claim
    forbids, so a denial discharges it. An availability claim instead demands
    that a stated positive combination is allowed, which is what stops the
    safety claims from being satisfied by a policy that denies everything.
    """
    t, w, a, o, locked = (f[k] for k in FACT_KEYS)
    return {
        "tenant-isolation": not allowed or t,
        "write-only": not allowed or w,
        "locked-requires-admin": not (allowed and locked) or a,
        "admin-or-owner": not allowed or a or o,
        "admin-write-available": not (t and w and a) or allowed,
        "owner-write-available": not (t and w and o and not locked) or allowed,
    }


def require(condition: bool, message: str) -> None:
    """Fail the run with a named diagnostic when a design check does not hold."""
    if not condition:
        raise AssertionError(message)


def strict_json(raw: bytes) -> Any:
    """Exercise the proposed ingress rules independently of the JSON schema."""
    def unique(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        """Reject a document that repeats a key instead of keeping the last."""
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"Duplicate JSON key: {key}")
            result[key] = value
        return result

    def no_constant(value: str) -> None:
        """Reject the non-JSON constants NaN, Infinity, and -Infinity."""
        raise ValueError(f"Non-JSON numeric constant: {value}")

    result = json.loads(raw.decode("utf-8", errors="strict"),
                        object_pairs_hook=unique, parse_constant=no_constant)

    def inspect(value: Any) -> None:
        """Reject any string containing an unpaired Unicode surrogate."""
        if isinstance(value, str):
            if any(0xD800 <= ord(ch) <= 0xDFFF for ch in value):
                raise ValueError("Unpaired Unicode surrogate")
        elif isinstance(value, dict):
            for key, item in value.items():
                inspect(key)
                inspect(item)
        elif isinstance(value, list):
            for item in value:
                inspect(item)
    inspect(result)
    return result


def contract_differences(before: Any, after: Any, prefix: str = "") -> list[str]:
    """List every JSON path where one contract document departs from another.

    A mutation is a deliberately weakened policy, not licence to widen the
    grammar or edit metadata. Reporting the changed paths lets the caller
    require that each reviewed variant differs from the baseline only inside
    its rule list, so a synchronous edit to an atom mapping, profile identity,
    or any new contract entry cannot slip through unnoticed. The walk reports
    added and removed containers as well as changed scalars, so an added empty
    object or list is a difference rather than an invisible no-op.
    """
    if isinstance(before, dict) and isinstance(after, dict):
        differences: list[str] = []
        for key in sorted(set(before) | set(after)):
            here = f"{prefix}.{key}" if prefix else key
            if key not in before or key not in after:
                differences.append(here)
            else:
                differences.extend(contract_differences(before[key], after[key], here))
        return differences
    if isinstance(before, list) and isinstance(after, list):
        differences = []
        for index in range(max(len(before), len(after))):
            here = f"{prefix}[{index}]"
            if index >= len(before) or index >= len(after):
                differences.append(here)
            else:
                differences.extend(
                    contract_differences(before[index], after[index], here))
        return differences
    return [] if before == after else [prefix]


def rule_rendering_reason(rule: list[str], atoms: dict[str, str]) -> str | None:
    """Return why a rule cannot be rendered as one atom per source line."""
    for atom in rule:
        source = atoms.get(atom)
        if source is None:
            return f"unmapped atom {atom!r}"
        if "\n" in source:
            return f"multi-line atom source for {atom!r}"
    return None


def header_reason(name: str, header: str) -> str | None:
    """Return why a module documentation header is unusable for a fixture.

    The header is the only part of a rendered fixture that is not derived from
    the rule-list IR, so it is checked structurally: it must be a single
    comment block naming its own fixture, and it must end with the blank line
    that separates documentation from the module body. A header copied from
    another fixture satisfies every other check while describing the wrong
    policy, which is why the name is part of the requirement.
    """
    if not header:
        return "empty header"
    if not header.endswith("\n\n"):
        return "header must end with a blank separator line"
    # The separator is a blank line, so it is excluded before the per-line walk
    # rather than being reported as an interior blank line.
    for line in header.rstrip("\n").splitlines():
        if not line:
            return "header must not contain a blank line"
        if not line.startswith("#"):
            return f"header line is not a comment: {line!r}"
    if name not in header.splitlines()[0]:
        return f"header does not name its fixture {name!r} on its first line"
    return None


def render_source(name: str, rules: list[list[str]], atoms: dict[str, str]) -> str:
    """Render the reviewed Rego template for a named policy's rule list."""
    text = SOURCE_HEADERS[name] + "package authz\n\nimport rego.v1\n\ndefault allow := false\n"
    for rule in rules:
        text += "\nallow if {\n" + "".join(f"    {atoms[a]}\n" for a in rule) + "}\n"
    return text


def decision_matches(f: dict[str, bool], allowed: bool,
                     requirements: dict[str, bool]) -> bool:
    """Check six requirements against the intended decision for one vector."""
    expected = (f["same_tenant"] and f["write_action"]
                and (f["admin_role"] or (f["owns_resource"] and not f["locked"])))
    return all(requirements.values()) == (allowed == expected)


def contract_completeness(facts: list[dict[str, bool]]) -> int:
    """Check the six-requirement contract over the whole fact/decision product."""
    combinations = list(itertools.product(facts, (False, True)))
    require(len(combinations) == 2 ** (len(FACT_KEYS) + 1),
            "Contract enumeration is not the complete fact/decision product")
    for f, allowed in combinations:
        require(decision_matches(f, allowed, claims(f, allowed)),
                "Contract permits a decision outside the intended policy")
    always_true = dict.fromkeys(CLAIM_IDS, True)
    require([(f, allowed) for f, allowed in combinations
             if not decision_matches(f, allowed, always_true)],
            NEGATIVE_CONTROL_HELP)
    return len(combinations)


def check_witness(name: str, witnesses: dict[str, Any], rules: list[list[str]],
                  expected: set[str]) -> None:
    """Re-derive a stored witness from its request and require it to refute.

    A refutation is a fact vector the admitted policy decides one way while the
    named requirement demands the other, so a witness may record either an
    allow that a safety requirement forbids or a denial that an availability
    requirement forbids. The request is re-evaluated from the rule list rather
    than trusted, which is why the stored decision and facts are checked.
    """
    require(set(witnesses) == expected,
            f"Unexpected witness failures for {name}: {sorted(witnesses)}")
    for claim_id, witness in witnesses.items():
        r = witness["request"]
        require(facts_of(r) == witness["facts"],
                f"Witness facts mismatch for {name}/{claim_id}")
        decision = eval_request(rules, r)
        require(decision == witness["decision"],
                f"Witness decision mismatch for {name}/{claim_id}")
        require(not claims(witness["facts"], decision)[claim_id],
                f"Witness does not refute {name}/{claim_id}")


def stale_witness_names(stored: set[str], generated: set[str]) -> list[str]:
    """List stored witness file names the current run did not generate."""
    return sorted(stored - generated)


@dataclass(frozen=True)
class Fixtures:
    """Every reviewed document the checks read, loaded once at the boundary.

    Loading is a separate step from checking so the checks receive data: a check
    that wanted a file would have to take it as an argument, which keeps the
    seam visible to a reviewer instead of hiding a read inside a query.
    """

    validators: dict[str, Draft202012Validator]
    profile: dict[str, Any]
    claims_manifest: dict[str, Any]
    policies: dict[str, dict[str, Any]]
    sources: dict[str, str]
    request: dict[str, Any]
    stored_witnesses: dict[str, dict[str, Any]]

    @classmethod
    def load(cls, repo: Repository) -> Fixtures:
        """Read the schemas, contracts, fixtures, and stored witnesses."""
        policies: dict[str, dict[str, Any]] = {}
        sources: dict[str, str] = {}
        for name in EXPECTED_FAILURES:
            folder = policy_folder(name)
            policies[name] = extract(repo.read_json(f"{folder}/policy.json"),
                                     f"{folder}/policy.json")
            sources[name] = repo.read_text(f"{folder}/policy.rego")
        return cls(
            validators=load_schemas(repo),
            profile=repo.read_json("contracts/profile.json"),
            claims_manifest=repo.read_json("examples/tenant-write/claims.json"),
            policies=policies,
            sources=sources,
            request=repo.read_json("examples/tenant-write/request.json"),
            stored_witnesses={Path(relative).name: repo.read_json(relative)
                              for relative in repo.list_json(WITNESS_DIRECTORY)},
        )


def check_profile(validators: dict[str, Draft202012Validator],
                  profile: dict[str, Any]) -> None:
    """Validate the profile document and its closed vocabulary."""
    validators["profile"].validate(profile)
    require(tuple(profile["atoms"]) == PROFILE_ATOM_NAMES, "Profile atoms mismatch")
    require(tuple(profile["facts"]) == FACT_KEYS, "Profile facts mismatch")
    require(tuple(profile["claims"]) == CLAIM_IDS, "Profile claims mismatch")
    require(tuple(profile["excluded"]) == PROFILE_EXCLUDED, "Profile exclusions mismatch")
    require(profile["limits"]["concurrent_proof_jobs"] == 1,
            "Profile must admit one proof job at a time")
    require(profile["runtime_options"]["network"] is False,
            "Profile must forbid network access")
    require(profile["assurance"]["source_binding"] == "trusted frontend",
            "Profile must keep the source frontend explicitly trusted")


def check_claims_registry(validators: dict[str, Draft202012Validator],
                          manifest: dict[str, Any]) -> int:
    """Validate the claim manifest and require it to list every fixed claim."""
    validators["claims"].validate(manifest)
    require(set(manifest["claims"]) == set(CLAIM_IDS), "Claim registry mismatch")
    return len(manifest["claims"])


@dataclass(frozen=True)
class PolicyReview:
    """What a pure review of every policy variant produced.

    ``witnesses`` maps a variant name to the counterexamples it generated, keyed
    by claim and shaped exactly as the published document; a variant that
    refutes nothing maps to an empty document, which is what removes its witness
    on publication. ``rules`` echoes the rule list each witness was derived
    from, so the publishing command can replay a persisted witness without
    reading the policy contracts a second time.
    """

    rows: list[dict[str, Any]]
    evaluations: int
    witnesses: dict[str, dict[str, dict[str, Any]]]
    rules: dict[str, list[list[str]]]


def validate_policies(validators: dict[str, Draft202012Validator], fixtures: Fixtures,
                      facts: list[dict[str, bool]]) -> PolicyReview:
    """Compare every variant's fixtures, witnesses, and rendered source.

    Pure with respect to storage: it reads only the already-loaded ``fixtures``
    and returns the counterexamples it generated, so nothing reaches the
    filesystem and a caller can inspect exactly what a run would publish. The
    variant set and each expected failure set are reviewed constants, so neither
    can be widened by editing a contract document.
    """
    atoms = fixtures.profile["atoms"]
    baseline = fixtures.policies["baseline"]
    rows: list[dict[str, Any]] = []
    evaluations = 0
    witnesses: dict[str, dict[str, dict[str, Any]]] = {}
    rules_by_name: dict[str, list[list[str]]] = {}
    for name, expected_failures in EXPECTED_FAILURES.items():
        policy = fixtures.policies[name]
        validators["policy"].validate(policy)
        # A variant may differ from the baseline only in its rule list, which is
        # what its reviewed mutation entry describes. Any other changed leaf
        # means a coordinated edit accompanied the mutation and needs review.
        if name != "baseline":
            require(policy.get("profile") == baseline.get("profile"),
                    f"Variant {name} changed the profile identity")
            differences = [path for path in contract_differences(baseline, policy)
                           if not path.startswith("rules")]
            require(not differences,
                    f"Variant {name} changed reviewed contract entries: {differences}")
        rules = policy["rules"]
        for rule in rules:
            reason = rule_rendering_reason(rule, atoms)
            require(reason is None, f"Unrenderable rule in {name}: {reason}")
        require(render_source(name, rules, atoms) == fixtures.sources[name],
                f"Source/fixture text drift: {name}")

        failures: dict[str, Any] = {}
        allow_count = 0
        for f in facts:
            r = concretize(f)
            validators["request"].validate(r)
            require(facts_of(r) == f, "Concretization is not a right inverse")
            abstract = eval_facts(rules, f)
            require(eval_request(rules, r) == abstract,
                    f"Concrete/abstract fixture disagreement: {name}")
            allow_count += int(abstract)
            evaluations += 1
            for claim_id, passed in claims(f, abstract).items():
                if not passed and claim_id not in failures:
                    failures[claim_id] = {"request": r, "decision": abstract, "facts": f}
        require(set(failures) == expected_failures,
                f"Unexpected mutation failures for {name}: {set(failures)}")
        if name == "baseline":
            require(allow_count == 5, "Baseline expected to allow 5 of 32 vectors")

        witnesses[name] = failures
        rules_by_name[name] = rules
        rows.append({"policy": name, "vectors": len(facts), "allowed_vectors": allow_count,
                     "failed_claims": sorted(failures)})
    return PolicyReview(rows=rows, evaluations=evaluations,
                        witnesses=witnesses, rules=rules_by_name)


def publish_witnesses(workspace: Workspace, review: PolicyReview) -> int:
    """Persist a review's witnesses, then replay them from the same boundary.

    Each witness is written, read back, and re-derived from its recorded request
    and rule list, so the replay checks the persisted document rather than the
    structure still held in memory. A variant that refutes nothing has its
    witness removed, and any file left in the witness directory that this run
    did not generate fails the command, so a retired mutation cannot leave a
    stale counterexample behind.
    """
    kept: set[str] = set()
    for name, failures in review.witnesses.items():
        relative = f"{WITNESS_DIRECTORY}/{name}.json"
        if failures:
            workspace.write_json(relative, failures)
            kept.add(f"{name}.json")
        else:
            workspace.remove(relative)
    present = {Path(relative).name for relative in workspace.list_json(WITNESS_DIRECTORY)}
    stale = stale_witness_names(present, kept)
    require(not stale, f"Stale witness files not generated by this run: {stale}")

    replays = 0
    for name, failures in review.witnesses.items():
        if not failures:
            continue
        check_witness(name,
                      workspace.read_json(f"{WITNESS_DIRECTORY}/{name}.json"),
                      review.rules[name], EXPECTED_FAILURES[name])
        replays += 1
    return replays


def check_request_ingress(validators: dict[str, Draft202012Validator],
                          good: dict[str, Any]) -> int:
    """Require the request schema to reject malformed shapes."""
    validators["request"].validate(good)
    bad_requests = []
    r = copy.deepcopy(good); r["resource"]["locked"] = "false"; bad_requests.append(r)
    r = copy.deepcopy(good); r["resource"]["locked"] = 0; bad_requests.append(r)
    r = copy.deepcopy(good); r["subject"]["tenant"] = None; bad_requests.append(r)
    r = copy.deepcopy(good); del r["action"]; bad_requests.append(r)
    r = copy.deepcopy(good); r["admin"] = True; bad_requests.append(r)
    for r in bad_requests:
        require(not validators["request"].is_valid(r), "Malformed request admitted")
    return len(bad_requests)


def check_strict_decoding() -> int:
    """Require the strict ingress decoder to reject malformed byte strings."""
    bad_bytes = [b'{"locked":true,"locked":false}', b'{"id":"\\ud800"}', b'\xff', b'NaN']
    for raw in bad_bytes:
        try:
            strict_json(raw)
        except (ValueError, UnicodeError):
            pass
        else:
            raise AssertionError(f"Strict decoder admitted {raw!r}")
    return len(bad_bytes)


def check_unicode_handling(validators: dict[str, Draft202012Validator],
                           good: dict[str, Any]) -> None:
    """Require Unicode text to survive ingress without normalization."""
    for text in ("", "Edinburgh", "é", "é", "🦉"):
        r = copy.deepcopy(good)
        r["subject"]["tenant"] = r["resource"]["tenant"] = text
        validators["request"].validate(r)
        require(facts_of(strict_json(json.dumps(r).encode()))["same_tenant"],
                "Valid Unicode request failed")
    # Composed U+00E9 and decomposed e+U+0301 must stay distinct under the
    # byte-comparison profile, because tenant isolation depends on that
    # distinction. The check runs through ingress and the fact mapping rather
    # than comparing two string literals.
    r = copy.deepcopy(good)
    r["subject"]["tenant"], r["resource"]["tenant"] = "é", "é"
    validators["request"].validate(r)
    decoded = strict_json(json.dumps(r).encode())
    require(not facts_of(decoded)["same_tenant"],
            "The profile must not normalize Unicode")
    # Positive control: the same request becomes same-tenant once the subject
    # tenant is also decomposed, so the rejection above is about code points
    # rather than a request that never reaches the fact mapping.
    control = copy.deepcopy(r)
    control["subject"]["tenant"] = "é"
    require(facts_of(control)["same_tenant"],
            "Unicode control did not restore tenant equality")


def check_result_schema(validators: dict[str, Draft202012Validator]) -> None:
    """Check that an explicitly unproved report satisfies the result schema."""
    report_sample = {
        "schema_version": "polean.result.v0", "status": "unknown",
        "profile": "polean.tenant-write.v0", "entrypoint": "data.authz.allow",
        "digests": None,
        "assurance": {"logic": "not_checked", "source_binding": "not_checked",
                      "runtime_correspondence": "not_run"},
        "claims": [], "diagnostics": [{"code": "DRAFT_ONLY", "message": "No prover executed",
                                        "source_start_byte": None, "source_end_byte": None}],
        "evidence_path": None,
    }
    validators["result"].validate(report_sample)


def request_controls(validators: dict[str, Draft202012Validator],
                     good: dict[str, Any]) -> list[tuple[str, bool, bool]]:
    """Pair the accepted request with mutations the contract must reject."""

    def accepted(r: dict[str, Any]) -> bool:
        """Report whether the request contract admits a document."""
        return validators["request"].is_valid(r)

    return [
        ("request rejects a non-Boolean locked value", accepted(good),
         accepted({**good, "resource": {**good["resource"], "locked": 0}})),
        ("request rejects an unknown top-level field", accepted(good),
         accepted({**good, "admin": True})),
        ("request rejects a missing required entry", accepted(good),
         accepted({k: v for k, v in good.items() if k != "action"})),
    ]


def contract_controls(validators: dict[str, Draft202012Validator],
                      fixtures: Fixtures) -> list[tuple[str, bool, bool]]:
    """Pair each accepted contract with a mutation the harness must notice."""
    profile = fixtures.profile
    atoms = profile["atoms"]
    baseline = fixtures.policies["baseline"]
    baseline_source = fixtures.sources["baseline"]

    def render_accepted(rules: list[list[str]]) -> bool:
        """Report whether a rule list renders to the baseline source text."""
        return render_source("baseline", rules, atoms) == baseline_source

    def variant_accepted(variant: dict[str, Any]) -> bool:
        """Report whether a variant keeps the profile and every non-rule entry."""
        if variant.get("profile") != baseline.get("profile"):
            return False
        return not [path for path in contract_differences(baseline, variant)
                    if not path.startswith("rules")]

    def profile_schema_accepted(p: dict[str, Any]) -> bool:
        """Report whether the profile contract admits a document."""
        return validators["profile"].is_valid(p)

    extra_rule = copy.deepcopy(baseline)
    extra_rule["rules"].append(["locked"])
    widened = copy.deepcopy(profile)
    widened["atoms"]["same_tenant"] = "input.resource.owner == input.subject.id"
    drifted_identity = copy.deepcopy(profile)
    drifted_identity["id"] = "polean.other-profile.v0"
    drifted_bounds = copy.deepcopy(profile)
    drifted_bounds["rules"]["max"] = 32
    drifted_runtime = copy.deepcopy(profile)
    drifted_runtime["runtime_options"]["network"] = True
    unadmitted_atom = copy.deepcopy(profile)
    unadmitted_atom["atoms"]["same_tenant"] = "input.tenant == input.tenant"
    extra_profile_entry = copy.deepcopy(profile)
    extra_profile_entry["mode"] = "permissive"
    return [
        ("source template accepts the reviewed rules",
         render_accepted(baseline["rules"]), render_accepted(extra_rule["rules"])),
        ("variant probe accepts a reviewed rule-list-only change",
         variant_accepted({"profile": baseline["profile"], "rules": []}),
         variant_accepted({"profile": "other.v0", "rules": []})),
        ("variant probe rejects an added contract entry",
         variant_accepted(baseline),
         variant_accepted({**baseline, "atoms": {}})),
        ("contract discrepancies describe the mutated rule",
         contract_differences(baseline, extra_rule)
         == [f"rules[{len(baseline['rules'])}]"],
         contract_differences(baseline, extra_rule) == []),
        ("contract discrepancies detect a widened atom expression",
         contract_differences(profile, profile) == [],
         contract_differences(profile, widened) == []),
        ("profile schema rejects a changed profile identity",
         profile_schema_accepted(profile), profile_schema_accepted(drifted_identity)),
        ("profile schema rejects widened rule bounds",
         profile_schema_accepted(profile), profile_schema_accepted(drifted_bounds)),
        ("profile schema rejects an enabled network option",
         profile_schema_accepted(profile), profile_schema_accepted(drifted_runtime)),
        ("profile schema rejects an unadmitted atom expression",
         profile_schema_accepted(profile), profile_schema_accepted(unadmitted_atom)),
        ("profile schema rejects an unreviewed top-level entry",
         profile_schema_accepted(profile), profile_schema_accepted(extra_profile_entry)),
    ]


def witness_controls(fixtures: Fixtures) -> list[tuple[str, bool, bool]]:
    """Pair the accepted witnesses and headers with mutated counterparts."""
    profile = fixtures.profile
    baseline = fixtures.policies["baseline"]
    witness = fixtures.stored_witnesses["deny-all.json"]
    deny_all_rules = fixtures.policies["deny-all"]["rules"]
    stored_witnesses = set(fixtures.stored_witnesses)

    def witness_accepted(w: dict[str, Any]) -> bool:
        """Report whether a stored witness still refutes its named claim."""
        try:
            check_witness("deny-all", w, deny_all_rules, EXPECTED_FAILURES["deny-all"])
        except (AssertionError, ValueError):
            return False
        return True

    def sweep_accepted(generated: set[str]) -> bool:
        """Report whether a generated set covers every stored witness."""
        return not stale_witness_names(stored_witnesses, generated)

    def header_accepted(name: str, header: str) -> bool:
        """Report whether a fixture's module documentation is usable."""
        return header_reason(name, header) is None

    tampered_decision = copy.deepcopy(witness)
    tampered_decision["admin-write-available"]["decision"] = True
    tampered_facts = copy.deepcopy(witness)
    tampered_facts["admin-write-available"]["facts"]["same_tenant"] = False
    denied = {"same_tenant": False, "write_action": True, "admin_role": True,
              "owns_resource": False, "locked": False}
    return [
        ("module documentation accepts the baseline header",
         header_accepted("baseline", SOURCE_HEADERS["baseline"]),
         header_accepted("baseline", SOURCE_HEADERS["admin-only"])),
        ("module documentation rejects a bodyless header",
         header_accepted("baseline", SOURCE_HEADERS["baseline"]),
         header_accepted("baseline", "package authz\n")),
        ("module documentation rejects unseparated prose",
         header_accepted("baseline", SOURCE_HEADERS["baseline"]),
         header_accepted("baseline", SOURCE_HEADERS["baseline"].rstrip("\n") + "\n")),
        ("witness re-derivation accepts the stored witness",
         witness_accepted(witness),
         witness_accepted(tampered_decision) or witness_accepted(tampered_facts)),
        ("witness sweep accepts its own output",
         sweep_accepted(stored_witnesses),
         sweep_accepted(stored_witnesses - {"deny-all.json"})),
        ("contract pairs requirements with the decision",
         decision_matches(denied, False, claims(denied, False)),
         decision_matches(denied, False, claims(denied, True))),
    ]


def negative_controls(validators: dict[str, Draft202012Validator],
                      fixtures: Fixtures) -> int:
    """Require fixture checks to reject one targeted injected fault each.

    Each control pairs an accepted fixture with one mutation of it and demands
    that the same predicate accept the first and reject the second. A check
    that rejects both is as broken as one that accepts both, and a control
    whose fault goes unnoticed would prove nothing.
    """
    checks = [
        *request_controls(validators, fixtures.request),
        *contract_controls(validators, fixtures),
        *witness_controls(fixtures),
    ]
    for description, accepted, still_accepted in checks:
        require(accepted, f"{description}: the reviewed fixture was rejected")
        require(not still_accepted, f"{description}: {NEGATIVE_CONTROL_HELP}")
    return len(checks)


def check_boundaries(repo: Repository,
                     produce: Callable[[], PolicyReview]) -> tuple[PolicyReview, int]:
    """Prove validation is pure and publishing follows the given workspace.

    Two seams are asserted rather than trusted. Producing the review through
    ``produce`` must leave every published witness exactly as it was, in size
    and modification time as well as contents, so a check that reached for the
    filesystem would be caught here. Publishing that same review through a
    workspace rooted in a temporary directory must put the witnesses under that
    root and leave the repository alone, so the command persists through the
    boundary it was handed rather than through a module-level path.

    The review is returned so the caller publishes the very run that was
    measured, rather than a second run whose purity would go unobserved.
    """
    def published() -> dict[str, tuple[int, int]]:
        """Snapshot the published witnesses' size and modification time."""
        return {relative: repo.state(relative)
                for relative in repo.list_json(WITNESS_DIRECTORY)}

    before = published()
    review = produce()
    require(before == published(), "A pure check modified the published witnesses")

    with tempfile.TemporaryDirectory(prefix="polean-boundary-") as directory:
        workspace = Workspace(Path(directory))
        replays = publish_witnesses(workspace, review)
        written = {Path(relative).name
                   for relative in workspace.list_json(WITNESS_DIRECTORY)}
        expected_names = {f"{name}.json" for name, failures in review.witnesses.items()
                          if failures}
        require(written == expected_names and replays == len(expected_names),
                "Publishing did not write every witness through the given root")
        require(before == published(),
                "Publishing through a temporary root modified the repository")
    return review, 2


def run_checks(repo: Repository, workspace: Workspace) -> dict[str, Any]:
    """Run every fixture check and publish the result through the boundary.

    Each check raises AssertionError with a named diagnostic on failure, and a
    read or write at the boundary raises HarnessIOError naming the document, so
    a non-zero exit means the recorded summary was not written for the fixture
    set that was read.
    """
    require(set(SOURCE_HEADERS) == set(EXPECTED_FAILURES),
            "Every policy variant needs exactly one module documentation header")
    for name, header in SOURCE_HEADERS.items():
        reason = header_reason(name, header)
        require(reason is None, f"Unusable module documentation for {name}: {reason}")
    fixtures = Fixtures.load(repo)
    validators = fixtures.validators
    check_profile(validators, fixtures.profile)
    claim_count = check_claims_registry(validators, fixtures.claims_manifest)
    facts = [dict(zip(FACT_KEYS, values, strict=True))
             for values in itertools.product((False, True), repeat=len(FACT_KEYS))]
    require(len(facts) == 2 ** len(FACT_KEYS), "Expected the full five-fact product")
    decisions_checked = contract_completeness(facts)
    review, boundaries = check_boundaries(
        repo, lambda: validate_policies(validators, fixtures, facts))
    malformed = check_request_ingress(validators, fixtures.request)
    negatives = check_strict_decoding()
    check_unicode_handling(validators, fixtures.request)
    check_result_schema(validators)
    controls = negative_controls(validators, fixtures)
    replays = publish_witnesses(workspace, review)

    summary = {
        "scope": "design-fixture validation only",
        "json_schemas_checked": len(validators),
        "source_headers_documented": len(SOURCE_HEADERS),
        "profile_schema_validated": True,
        "fact_vectors_per_policy": len(facts),
        "policy_variants": len(review.rows),
        "concrete_abstract_comparisons": review.evaluations,
        "full_contract_decision_checks": decisions_checked,
        "claim_registry_entries": claim_count,
        "witnesses_replayed": replays,
        "negative_controls_passed": controls,
        "boundary_seam_checks_passed": boundaries,
        "malformed_request_shapes_rejected": malformed,
        "strict_decoding_negatives_rejected": negatives,
        "lean_proofs_checked": False,
        "rego_parsed_by_opa_or_regorus": False,
        "runtime_conformance_executed": False,
        "policies": review.rows,
    }
    workspace.write_json(SUMMARY_DOCUMENT, summary)
    return summary


def main(repo: Repository | None = None, workspace: Workspace | None = None) -> int:
    """Run the checks at the command boundary and report one diagnostic on failure.

    The boundaries are parameters so a caller can inject a different repository
    or a temporary workspace; the defaults are the repository this script lives
    in. A boundary failure or a failed check is reported as a single named
    diagnostic. Any other exception is a defect in the harness rather than a
    finding about the fixtures, so it keeps its traceback.
    """
    try:
        summary = run_checks(repo or Repository(ROOT), workspace or Workspace(ROOT))
    except HarnessIOError as error:
        print(f"design validation I/O failure: {error}", file=sys.stderr)
        return 1
    except AssertionError as error:
        print(f"design validation check failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
