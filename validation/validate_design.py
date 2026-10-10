#!/usr/bin/env python3
"""Validate Polean design fixtures, not the proposed checker or Lean proofs.

Run from any directory with Python 3.11+ and jsonschema 4.x installed.
This script does not parse Rego, execute OPA/Regorus, or invoke Lean.
"""
from __future__ import annotations

import copy
import itertools
import json
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
WITNESS_DIR = ROOT / "validation/witnesses"


def load(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def extract(path: Path, name: str) -> Any:
    """Read one folder's contract document and reject any unknown entry."""
    document = load(path / name)
    require(set(document) <= {"rules", "profile"},
            f"Unknown top-level entry in {path / name}: {sorted(set(document))}")
    return document


def facts_of(request: dict[str, Any]) -> dict[str, bool]:
    subject, resource = request["subject"], request["resource"]
    return {
        "same_tenant": subject["tenant"] == resource["tenant"],
        "write_action": request["action"] == "write",
        "admin_role": subject["role"] == "admin",
        "owns_resource": subject["id"] == resource["owner"],
        "locked": resource["locked"],
    }


def concretize(facts: dict[str, bool]) -> dict[str, Any]:
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
    def atom(name: str) -> bool:
        return not facts["locked"] if name == "unlocked" else facts[name]
    return any(all(atom(name) for name in rule) for rule in rules)


def eval_request(rules: list[list[str]], r: dict[str, Any]) -> bool:
    """A direct concrete interpretation, separate from facts_of."""
    def atom(name: str) -> bool:
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
    if not condition:
        raise AssertionError(message)


def strict_json(raw: bytes) -> Any:
    """Exercise the proposed ingress rules independently of the JSON schema."""
    def unique(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"Duplicate JSON key: {key}")
            result[key] = value
        return result

    def no_constant(value: str) -> None:
        raise ValueError(f"Non-JSON numeric constant: {value}")

    result = json.loads(raw.decode("utf-8", errors="strict"),
                        object_pairs_hook=unique, parse_constant=no_constant)

    def inspect(value: Any) -> None:
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


def render_source(rules: list[list[str]], atoms: dict[str, str]) -> str:
    """Render the reviewed Rego template for a policy's rule list."""
    text = "package authz\n\nimport rego.v1\n\ndefault allow := false\n"
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


def stale_witness_names(directory: Path, generated: set[str]) -> list[str]:
    """List witness files the current run did not generate."""
    return sorted(path.name for path in directory.glob("*.json")
                  if path.name not in generated)


def load_schemas() -> dict[str, Draft202012Validator]:
    """Meta-validate every contract schema and index it by contract name."""
    validators: dict[str, Draft202012Validator] = {}
    for path in sorted((ROOT / "contracts").glob("*.schema.json")):
        schema = load(path)
        Draft202012Validator.check_schema(schema)
        validators[path.stem.removesuffix(".schema")] = Draft202012Validator(schema)
    return validators


def check_profile(validators: dict[str, Draft202012Validator]) -> dict[str, Any]:
    """Validate the profile document and its closed vocabulary."""
    profile = load(ROOT / "contracts/profile.json")
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
    return profile


def check_claims_registry(validators: dict[str, Draft202012Validator]) -> int:
    """Validate the claim manifest and require it to list every fixed claim."""
    manifest = load(ROOT / "examples/tenant-write/claims.json")
    validators["claims"].validate(manifest)
    require(set(manifest["claims"]) == set(CLAIM_IDS), "Claim registry mismatch")
    return len(manifest["claims"])


def validate_policies(validators: dict[str, Draft202012Validator],
                      profile: dict[str, Any],
                      facts: list[dict[str, bool]]) -> tuple[list[dict[str, Any]], int, int]:
    """Compare every variant's fixtures, witnesses, and rendered source."""
    atoms = profile["atoms"]
    baseline = extract(ROOT / "examples/tenant-write", "policy.json")
    WITNESS_DIR.mkdir(exist_ok=True)
    rows: list[dict[str, Any]] = []
    evaluations = 0
    replays = 0
    generated: set[str] = set()
    for name, expected_failures in EXPECTED_FAILURES.items():
        folder = (ROOT / "examples/tenant-write" if name == "baseline"
                  else ROOT / "examples/tenant-write/mutations" / name)
        policy = extract(folder, "policy.json")
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
        require(render_source(rules, atoms) == (folder / "policy.rego").read_text(),
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

        witness_path = WITNESS_DIR / f"{name}.json"
        if failures:
            witness_path.write_text(
                json.dumps(failures, ensure_ascii=False, indent=2) + "\n")
            generated.add(witness_path.name)
            check_witness(name, load(witness_path), rules, expected_failures)
            replays += 1
        elif witness_path.exists():
            witness_path.unlink()
        rows.append({"policy": name, "vectors": len(facts), "allowed_vectors": allow_count,
                     "failed_claims": sorted(failures)})
    stale = stale_witness_names(WITNESS_DIR, generated)
    require(not stale, f"Stale witness files not generated by this run: {stale}")
    return rows, evaluations, replays


def check_request_ingress(validators: dict[str, Draft202012Validator]) -> int:
    """Require the request schema to reject malformed shapes."""
    good = load(ROOT / "examples/tenant-write/request.json")
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


def check_unicode_handling(validators: dict[str, Draft202012Validator]) -> None:
    """Require Unicode text to survive ingress without normalization."""
    good = load(ROOT / "examples/tenant-write/request.json")
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


def negative_controls(validators: dict[str, Draft202012Validator]) -> int:
    """Require fixture checks to reject one targeted injected fault each.

    Each control pairs an accepted fixture with one mutation of it and demands
    that the same predicate accept the first and still accept the second. A
    check that rejects both is as broken as one that accepts both, and a
    control whose fault goes unnoticed would prove nothing.
    """
    profile = load(ROOT / "contracts/profile.json")
    atoms = profile["atoms"]
    baseline = load(ROOT / "examples/tenant-write/policy.json")
    good = load(ROOT / "examples/tenant-write/request.json")
    witness = load(WITNESS_DIR / "deny-all.json")
    baseline_source = (ROOT / "examples/tenant-write/policy.rego").read_text()
    deny_all_rules = load(
        ROOT / "examples/tenant-write/mutations/deny-all/policy.json")["rules"]

    def request_accepted(r: dict[str, Any]) -> bool:
        return validators["request"].is_valid(r)

    def render_accepted(rules: list[list[str]]) -> bool:
        return render_source(rules, atoms) == baseline_source

    def witness_accepted(w: dict[str, Any]) -> bool:
        try:
            check_witness("deny-all", w, deny_all_rules, EXPECTED_FAILURES["deny-all"])
        except (AssertionError, ValueError):
            return False
        return True

    def variant_accepted(variant: dict[str, Any]) -> bool:
        if variant.get("profile") != baseline.get("profile"):
            return False
        return not [path for path in contract_differences(baseline, variant)
                    if not path.startswith("rules")]

    def sweep_accepted(generated: set[str]) -> bool:
        return bool(generated) and not stale_witness_names(WITNESS_DIR, generated)

    def profile_schema_accepted(p: dict[str, Any]) -> bool:
        return validators["profile"].is_valid(p)

    denied = {"same_tenant": False, "write_action": True, "admin_role": True,
              "owns_resource": False, "locked": False}
    renamed = copy.deepcopy(baseline)
    renamed["rules"][0][0] = "write_action"
    extra_rule = copy.deepcopy(baseline)
    extra_rule["rules"].append(["locked"])
    widened = copy.deepcopy(profile)
    widened["atoms"]["same_tenant"] = "input.resource.owner == input.subject.id"
    tampered_decision = copy.deepcopy(witness)
    tampered_decision["admin-write-available"]["decision"] = True
    tampered_facts = copy.deepcopy(witness)
    tampered_facts["admin-write-available"]["facts"]["same_tenant"] = False
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
    names = {path.name for path in WITNESS_DIR.glob("*.json")}
    checks: list[tuple[str, bool, bool]] = [
        ("request rejects a non-Boolean locked value",
         request_accepted(good),
         request_accepted({**good, "resource": {**good["resource"], "locked": 0}})),
        ("request rejects an unknown top-level field",
         request_accepted(good), request_accepted({**good, "admin": True})),
        ("request rejects a missing required entry",
         request_accepted(good),
         request_accepted({k: v for k, v in good.items() if k != "action"})),
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
        ("witness re-derivation accepts the stored witness",
         witness_accepted(witness),
         witness_accepted(tampered_decision) or witness_accepted(tampered_facts)),
        ("witness sweep accepts its own output",
         sweep_accepted(names), sweep_accepted(names - {"deny-all.json"})),
        ("contract pairs requirements with the decision",
         decision_matches(denied, False, claims(denied, False)),
         decision_matches(denied, False, claims(denied, True))),
    ]
    for description, accepted, still_accepted in checks:
        require(accepted, f"{description}: the reviewed fixture was rejected")
        require(not still_accepted, f"{description}: {NEGATIVE_CONTROL_HELP}")
    return len(checks)


def main() -> None:
    validators = load_schemas()
    profile = check_profile(validators)
    claim_count = check_claims_registry(validators)
    facts = [dict(zip(FACT_KEYS, values, strict=True))
             for values in itertools.product((False, True), repeat=len(FACT_KEYS))]
    require(len(facts) == 2 ** len(FACT_KEYS), "Expected the full five-fact product")
    decisions_checked = contract_completeness(facts)
    rows, evaluations, replays = validate_policies(validators, profile, facts)
    malformed = check_request_ingress(validators)
    negatives = check_strict_decoding()
    check_unicode_handling(validators)
    check_result_schema(validators)
    controls = negative_controls(validators)

    summary = {
        "scope": "design-fixture validation only",
        "json_schemas_checked": len(validators),
        "profile_schema_validated": True,
        "fact_vectors_per_policy": len(facts),
        "policy_variants": len(rows),
        "concrete_abstract_comparisons": evaluations,
        "full_contract_decision_checks": decisions_checked,
        "claim_registry_entries": claim_count,
        "witnesses_replayed": replays,
        "negative_controls_passed": controls,
        "malformed_request_shapes_rejected": malformed,
        "strict_decoding_negatives_rejected": negatives,
        "lean_proofs_checked": False,
        "rego_parsed_by_opa_or_regorus": False,
        "runtime_conformance_executed": False,
        "policies": rows,
    }
    (ROOT / "validation/design-validation.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
