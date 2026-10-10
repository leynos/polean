#!/usr/bin/env python3
"""Property checks over the closed tenant-write profile, not its fixed vectors.

``validate_design.py`` enumerates the 32 Boolean fact vectors and seven
hand-authored policies. Enumeration covers exactly the cases that were
enumerated, so it cannot notice a fault that only appears outside the frozen
fixture set. This module states the same obligations as properties and lets
Hypothesis search the admitted input domain for counterexamples, shrinking any
it finds while keeping the value schema-valid.

The properties asserted here are the model-side obligations the technical
design names for the first slice:

- abstraction preservation, ``evalRequest p r = evalFacts p (α r)``;
- abstraction coverage, ``α(γ(f)) = f`` for every fact vector;
- the claim semantics, including that ``unlocked`` is derived rather than an
  independent sixth fact; and
- renderer injectivity, and rule fidelity, inside the profile bounds.

Injectivity is a statement about the renderer, not about the committed
fixtures. What makes the fixture-drift check in ``validate_design.py``
non-decorative is that the atom sources and module headers are reviewed
constants of the harness, plus its own negative control; commutativity,
atom-order stability, and the exact template are asserted here because a
faithful-looking renderer can be injective while still disagreeing with the IR.

This module does not parse Rego, execute OPA or Regorus, or invoke Lean, and it
claims nothing about source-to-model correspondence or runtime refinement. It
passes ``derandomize=True`` and ``database=None`` so a run is reproducible and
writes no example database.
"""
from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from typing import Any

from hypothesis import HealthCheck, given, settings
from hypothesis import strategies as st

ROOT = Path(__file__).resolve().parents[1]
SETTINGS = settings(
    derandomize=True,
    database=None,
    deadline=None,
    max_examples=400,
    suppress_health_check=[HealthCheck.too_slow],
    report_multiple_bugs=False,
)


def _load_design_harness() -> Any:
    """Import the sibling validator, whose names these properties are about."""
    path = ROOT / "validation/validate_design.py"
    spec = importlib.util.spec_from_file_location("polean_design_harness", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Cannot import the design harness from {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


VD = _load_design_harness()
# Loaded once: the schemas are meta-validated and the profile is constant, so
# re-reading them per generated example would dominate the search budget.
VALIDATORS = VD.load_schemas()
ATOMS = VD.load(ROOT / "contracts/profile.json")["atoms"]
# Inverse of ATOMS, for decoding rendered text back to a rule list.
ATOM_SOURCES = {source: name for name, source in ATOMS.items()}
assert len(ATOM_SOURCES) == len(ATOMS), "Atom sources must be distinct to decode"

# Free-text domains, deliberately wider than the fixtures. The profile compares
# strings for equality and does not normalize them, so equality of distinct
# sequences is the only property the abstraction may rely on. Every string
# operation in the harness -- and in the profile it models -- is an equality
# test against a constant or another field, so the decision is fully determined
# by which strings coincide. A cap of 64 therefore bounds the search without
# narrowing the faults it can expose: two distinct strings already differ
# somewhere in their first few characters, and any normalization, prefix
# comparison, or case folding shows up well inside that bound.
TEXT = st.text(max_size=64)
TENANTS = st.sampled_from(["tenant-a", "tenant-b"]) | TEXT
ROLES = st.sampled_from(["admin", "member"]) | TEXT
ACTIONS = st.sampled_from(["write", "read"]) | TEXT
# A policy's rule list inside the closed profile: zero to sixteen rules, each
# naming one to sixteen admitted atoms in source order. Repeated atoms are
# admitted on purpose: the policy schema sets no uniqueItems, so a grammar-
# legal body may name the same comparison twice, and a strategy that forbade
# repeats would search a strict subset of the bounds these properties claim.
RULES = st.lists(
    st.lists(
        st.sampled_from(VD.PROFILE_ATOM_NAMES),
        min_size=1,
        max_size=16,
    ),
    min_size=0,
    max_size=16,
)


# One fact vector over exactly the harness's declared fact keys, so adding or
# renaming a fact cannot leave these properties drawing a stale key set.
FACT_VECTORS = st.fixed_dictionaries(
    {key: st.booleans() for key in VD.FACT_KEYS},
)


@st.composite
def requests(draw: st.DrawFn) -> dict[str, Any]:
    """Draw one schema-valid request inside the declared input domain."""
    return {
        "subject": {
            "tenant": draw(TENANTS),
            "id": draw(TEXT),
            "role": draw(ROLES),
        },
        "resource": {
            "tenant": draw(TENANTS),
            "owner": draw(TEXT),
            "locked": draw(st.booleans()),
        },
        "action": draw(ACTIONS),
    }


@given(rules=RULES, request=requests())
@SETTINGS
def test_concrete_and_abstract_evaluation_agree(
        rules: list[list[str]], request: dict[str, Any]) -> None:
    """``evalRequest`` and ``evalFacts ∘ α`` must decide the same way."""
    assert VD.eval_request(rules, request) == VD.eval_facts(
        rules, VD.facts_of(request)), f"abstraction mismatch for {request!r}"


@given(request=requests())
@SETTINGS
def test_abstraction_reads_the_declared_comparisons(
        request: dict[str, Any]) -> None:
    """Every drawn request is ingress-valid, and α computes the stated facts.

    Checking key names alone would pass for any value at all, so each fact is
    compared against an independently written expression over the request.
    """
    VALIDATORS["request"].validate(request)
    subject, resource = request["subject"], request["resource"]
    expected = {
        "same_tenant": subject["tenant"] == resource["tenant"],
        "write_action": request["action"] == "write",
        "admin_role": subject["role"] == "admin",
        "owns_resource": subject["id"] == resource["owner"],
        "locked": resource["locked"],
    }
    assert VD.facts_of(request) == expected, request


@given(
    values=st.lists(st.booleans(), min_size=len(VD.FACT_KEYS),
                    max_size=len(VD.FACT_KEYS))
)
@SETTINGS
def test_concretization_is_a_right_inverse(values: list[bool]) -> None:
    """``α(γ(f)) = f``, so every abstract counterexample has a real request."""
    facts = dict(zip(VD.FACT_KEYS, values, strict=True))
    concretized = VD.concretize(facts)
    VALIDATORS["request"].validate(concretized)
    assert VD.facts_of(concretized) == facts, f"γ is not a section for {facts!r}"


@given(facts=FACT_VECTORS)
@SETTINGS
def test_unlocked_is_derived_not_independent(facts: dict[str, bool]) -> None:
    """``unlocked`` must negate the locked fact rather than stand alone."""
    assert VD.eval_facts([["unlocked"]], facts) is (not facts["locked"])
    assert VD.eval_facts([["locked"]], facts) is facts["locked"]


@given(facts=FACT_VECTORS)
@SETTINGS
def test_the_six_claims_characterize_the_intended_decision(
        facts: dict[str, bool]) -> None:
    """Some decision satisfies all six claims only when it is the intended one."""
    satisfying = [allowed for allowed in (False, True)
                  if all(VD.claims(facts, allowed).values())]
    intended = (facts["same_tenant"] and facts["write_action"]
                and (facts["admin_role"]
                     or (facts["owns_resource"] and not facts["locked"])))
    assert satisfying == [intended], f"claims do not pin the decision for {facts!r}"


@given(rules=RULES, other=RULES)
@SETTINGS
def test_rendering_is_injective_within_profile_bounds(
        rules: list[list[str]], other: list[list[str]]) -> None:
    """Distinct admitted rule lists must not render to identical source text.

    Only distinct pairs are constrained. A renderer that de-duplicated a rule
    body would still be injective on distinct lists and would still satisfy
    this property, which is why ``test_rendered_text_preserves_the_rule_list``
    below decodes the text instead of merely comparing two renderings.
    """
    assert rules == other or VD.render_source("baseline", rules, ATOMS) != (
        VD.render_source("baseline", other, ATOMS)
    ), f"renderer collided on {rules!r} and {other!r}"


@given(rules=RULES)
@SETTINGS
def test_rendered_text_preserves_the_rule_list(rules: list[list[str]]) -> None:
    """The rendered text must decode back to exactly the rule list given.

    Splitting on the fixed template recovers the atom sources, so the rule
    list is recoverable from the text. This is what a de-duplicating or
    reordering renderer violates, and it asserts the template, the atom
    order, and every atom's presence rather than a subset of them.
    """
    text = VD.render_source("baseline", rules, ATOMS)
    body = [line for line in text.splitlines() if not line.startswith("#")]
    while body and not body[0]:  # the documented header ends with a blank line
        body.pop(0)
    assert body[:5] == ["package authz", "", "import rego.v1", "",
                        "default allow := false"], body[:5]
    # Decode from the raw text rather than the re-joined lines, which would
    # drop the trailing newline each rule block ends with.
    prefix = "default allow := false\n"
    rest = text[text.index(prefix) + len(prefix):]
    recovered: list[list[str]] = []
    indent = "    "
    for chunk in rest.split("\nallow if {\n")[1:]:
        assert chunk.endswith("}\n"), chunk
        body_lines = chunk[:-2].splitlines()
        for line in body_lines:
            assert line.startswith(indent), chunk
        recovered.append([ATOM_SOURCES[line[len(indent):]] for line in body_lines])
    assert recovered == rules, f"round trip changed {rules!r} into {recovered!r}"
    assert VD.header_reason("baseline", text[:text.index("package authz")]) is None


def main() -> int:
    """Run each property over the admitted domain and report the coverage.

    Every entry is asserted to be a Hypothesis-wrapped test, so a property
    accidentally stripped of its ``@given`` decorator fails here instead of
    silently running once against no examples and reporting a false pass.
    """
    properties = [
        test_concrete_and_abstract_evaluation_agree,
        test_abstraction_reads_the_declared_comparisons,
        test_concretization_is_a_right_inverse,
        test_unlocked_is_derived_not_independent,
        test_the_six_claims_characterize_the_intended_decision,
        test_rendering_is_injective_within_profile_bounds,
        test_rendered_text_preserves_the_rule_list,
    ]
    for property_check in properties:
        assert getattr(property_check, "is_hypothesis_test", False), (
            f"{property_check.__name__} is not Hypothesis-wrapped, so it would "
            "sample no examples"
        )
        property_check()
        print(f"ok  {property_check.__name__}  ({SETTINGS.max_examples} examples)")
    print(f"\n{len(properties)} properties held over each drawn example")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
