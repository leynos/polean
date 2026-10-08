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


def load(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


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


def main() -> None:
    validators = {}
    for path in sorted((ROOT / "contracts").glob("*.schema.json")):
        schema = load(path)
        Draft202012Validator.check_schema(schema)
        validators[path.stem.removesuffix(".schema")] = Draft202012Validator(schema)

    profile = load(ROOT / "contracts/profile.json")
    claim_manifest = load(ROOT / "examples/tenant-write/claims.json")
    validators["claims"].validate(claim_manifest)
    require(tuple(claim_manifest["claims"]) == CLAIM_IDS, "Claim registry mismatch")
    require(profile["claims"] == list(CLAIM_IDS), "Profile claims mismatch")
    facts = [dict(zip(FACT_KEYS, values, strict=True))
             for values in itertools.product((False, True), repeat=5)]
    require(len(facts) == 32, "Expected five-fact Cartesian product")

    # Check the complete six-claim contract against the separately stated formula.
    for f in facts:
        t, w, a, o, locked = (f[k] for k in FACT_KEYS)
        expected = t and w and (a or (o and not locked))
        for allowed in (False, True):
            require(all(claims(f, allowed).values()) == (allowed == expected),
                    "Contract permits a decision outside the intended policy")

    rows = []
    evaluations = 0
    witness_dir = ROOT / "validation/witnesses"
    witness_dir.mkdir(exist_ok=True)
    for name, expected_failures in EXPECTED_FAILURES.items():
        folder = (ROOT / "examples/tenant-write" if name == "baseline"
                  else ROOT / "examples/tenant-write/mutations" / name)
        policy = load(folder / "policy.json")
        validators["policy"].validate(policy)
        rules = policy["rules"]
        # Consistency check only, not a Rego parsing or semantic-equivalence test.
        expected_source = "package authz\n\nimport rego.v1\n\ndefault allow := false\n"
        for rule in rules:
            expected_source += "\nallow if {\n" + "".join(
                "    " + profile["atoms"][a] + "\n" for a in rule) + "}\n"
        require((folder / "policy.rego").read_text() == expected_source,
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
        if failures:
            (witness_dir / f"{name}.json").write_text(
                json.dumps(failures, ensure_ascii=False, indent=2) + "\n")
        rows.append({"policy": name, "vectors": 32, "allowed_vectors": allow_count,
                     "failed_claims": sorted(failures)})

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

    bad_bytes = [b'{"locked":true,"locked":false}', b'{"id":"\\ud800"}', b'\xff', b'NaN']
    for raw in bad_bytes:
        try:
            strict_json(raw)
        except (ValueError, UnicodeError):
            pass
        else:
            raise AssertionError(f"Strict decoder admitted {raw!r}")

    for text in ("", "Edinburgh", "é", "e\u0301", "🦉"):
        r = copy.deepcopy(good)
        r["subject"]["tenant"] = r["resource"]["tenant"] = text
        validators["request"].validate(r)
        require(facts_of(strict_json(json.dumps(r).encode()))["same_tenant"],
                "Valid Unicode request failed")
    require("é" != "e\u0301", "The profile must not normalize Unicode")

    # Basic report-schema validation must not be confused with certificate checks.
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

    summary = {
        "scope": "design-fixture validation only",
        "json_schemas_checked": len(validators),
        "fact_vectors_per_policy": 32,
        "policy_variants": len(rows),
        "concrete_abstract_comparisons": evaluations,
        "full_contract_decision_checks": 64,
        "malformed_request_shapes_rejected": len(bad_requests),
        "strict_decoding_negatives_rejected": len(bad_bytes),
        "lean_proofs_checked": False,
        "rego_parsed_by_opa_or_regorus": False,
        "runtime_conformance_executed": False,
        "policies": rows,
    }
    (ROOT / "validation/design-validation.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
