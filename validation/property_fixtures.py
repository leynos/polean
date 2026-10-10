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
- renderer injectivity and rule fidelity, inside the profile bounds, together
  with the equality boundary the string cap would otherwise hide.

Injectivity is a statement about the renderer, not about the committed
fixtures. What makes the fixture-drift check in ``validate_design.py``
non-decorative is that the atom sources and module headers are reviewed
constants of the harness, plus its own negative control; the exact template and
the atom order are asserted here because a faithful-looking renderer can be
injective while still disagreeing with the rule-list IR.

These are sampled properties, not proofs. A passing run bounds no universal
quantifier, and the string cap is a search restriction rather than a claim
that longer inputs cannot expose a fault.

This module does not parse Rego, execute OPA or Regorus, or invoke Lean, and it
claims nothing about source-to-model correspondence or runtime refinement. It
passes ``derandomize=True`` and ``database=None`` so a run is reproducible and
writes no example database.

Importing this module reads nothing. The admitted profile and the contract
schemas are read through the harness's read-only repository boundary by one
explicit :func:`configure` call in ``main``, before any property runs, so the
documents the properties depend on are named in one place and a property cannot
silently draw against constants left over from an earlier setup.
"""
from __future__ import annotations

import importlib.util
import sys
from dataclasses import dataclass
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


class ConfigurationError(RuntimeError):
    """Raised when a property runs before the fixtures have been loaded."""


@dataclass(frozen=True)
class Admitted:
    """The admitted profile and schemas the properties are stated against.

    Loaded once by an explicit :func:`configure` call rather than during module
    import, so importing this module reads nothing and the documents the
    properties depend on appear in one place. Loading once matters for cost as
    well as clarity: the schemas are meta-validated and the profile is
    constant, so re-reading them per generated example would dominate the
    search budget.
    """

    validators: dict[str, Any]
    atoms: dict[str, str]
    # Inverse of ``atoms``, for decoding rendered text back to a rule list.
    atom_sources: dict[str, str]


_ADMITTED: Admitted | None = None


def configure(repo: Any) -> Admitted:
    """Read and validate the profile the properties are stated against.

    Called by ``main`` before any property runs. A property invoked without it
    fails with :class:`ConfigurationError` rather than drawing against
    undefined constants.
    """
    global _ADMITTED
    atoms = repo.read_json("contracts/profile.json")["atoms"]
    atom_sources = {source: name for name, source in atoms.items()}
    assert len(atom_sources) == len(atoms), "Atom sources must be distinct to decode"
    _ADMITTED = Admitted(validators=VD.load_schemas(repo), atoms=atoms,
                         atom_sources=atom_sources)
    return _ADMITTED


def admitted() -> Admitted:
    """Return the configured profile, or fail when setup has not run."""
    if _ADMITTED is None:
        raise ConfigurationError(
            "Property fixtures are not configured; call configure() first")
    return _ADMITTED

# Free-text domains, deliberately wider than the fixtures. The profile compares
# strings for equality and does not normalize them, so equality of distinct
# sequences is the only property the abstraction may rely on. Every string
# operation in the harness -- and in the profile it models -- is an equality
# test against a constant or another field, so the decision is determined by
# which strings coincide, and the sampled representatives cover the equality
# patterns the abstraction distinguishes.
#
# A cap of 64 is a search restriction, not a completeness argument. Distinct
# strings can share more than 64 leading characters, and an implementation
# fault could activate only above that length; nothing here rules that out.
# LONG_STRING and SHARED_PREFIXES below deliberately cross that boundary so the
# cap does not also exclude the cases where such a fault would first show, but
# coverage remains bounded and is not claimed to be fault-preserving in
# general. LONG_STRING reaches tenants, ids, and owners alike, since all three
# are compared.
TEXT = st.text(max_size=64)
LONG_STRING = st.integers(min_value=65, max_value=4096).map(lambda n: "a" * n)
# A pair sharing a prefix that itself runs past the cap, so both members are
# longer than anything the capped TEXT domain can draw. The extension is
# non-empty, making the two members unequal and the longer one a strict
# extension of the other.
SHARED_PREFIXES = st.tuples(
    st.integers(min_value=65, max_value=2048).map(lambda n: "a" * n),
    st.text(min_size=1, max_size=8),
).map(lambda pair: (pair[0], pair[0] + pair[1]))
TENANTS = st.sampled_from(["tenant-a", "tenant-b"]) | TEXT | LONG_STRING
IDS = TEXT | LONG_STRING
ROLES = st.sampled_from(["admin", "member"]) | TEXT
ACTIONS = st.sampled_from(["write", "read"]) | TEXT
# A policy's rule list inside the closed profile: zero to sixteen rules, each
# naming one to sixteen admitted atoms in source order. Repeated atoms are
# admitted on purpose: the policy schema sets no uniqueItems, so a grammar-
# legal body may name the same comparison twice, and a strategy that forbade
# repeats would search a strict subset of the bounds these properties claim.
# The vocabulary comes from the harness's reviewed constant rather than from
# contracts/profile.json, so this strategy is defined without reading a file
# and the properties search the admitted domain even if the profile document
# were edited to widen it.
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
            "id": draw(IDS),
            "role": draw(ROLES),
        },
        "resource": {
            "tenant": draw(TENANTS),
            "owner": draw(IDS),
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
    admitted().validators["request"].validate(request)
    subject, resource = request["subject"], request["resource"]
    expected = {
        "same_tenant": subject["tenant"] == resource["tenant"],
        "write_action": request["action"] == "write",
        "admin_role": subject["role"] == "admin",
        "owns_resource": subject["id"] == resource["owner"],
        "locked": resource["locked"],
    }
    assert VD.facts_of(request) == expected, request


@given(pair=SHARED_PREFIXES, locked=st.booleans())
@SETTINGS
def test_equality_ignores_length_and_shared_prefix(
        pair: tuple[str, str], locked: bool) -> None:
    """Only whole-string equality may drive a decision, not length or prefix.

    The general string cap bounds the search, so the cases that cross it are
    drawn explicitly here. Each member of the pair is longer than the cap and
    the shorter (by construction) is a strict prefix of the longer, so an
    implementation that truncated, prefix-compared, or branched on length
    diverges on these draws. Both compared pairs are exercised: the tenants
    drive ``same_tenant`` and the ids drive ``owns_resource``.

    Coverage is still bounded. These are two specific shapes of long input
    among many, so a fault needing some other long-string arrangement may
    escape both this property and the capped domain.
    """
    left, right = pair
    assert len(left) > 64 and len(right) > 64, pair
    assert right.startswith(left) and right != left, pair
    request = {
        "subject": {"tenant": left, "id": left, "role": "member"},
        "resource": {"tenant": right, "owner": right, "locked": locked},
        "action": "write",
    }
    admitted().validators["request"].validate(request)
    facts = VD.facts_of(request)
    # Strict extensions must compare unequal in both compared positions.
    assert facts["same_tenant"] is False, request
    assert facts["owns_resource"] is False, request
    assert VD.eval_request([["same_tenant"]], request) is False, request
    assert VD.eval_request([["owns_resource"]], request) is False, request
    # Control: the same long string against itself must compare equal, so the
    # assertions above cannot be satisfied by an operator that always denies.
    identical = {
        "subject": {"tenant": left, "id": left, "role": "member"},
        "resource": {"tenant": left, "owner": left, "locked": locked},
        "action": "write",
    }
    admitted().validators["request"].validate(identical)
    equal_facts = VD.facts_of(identical)
    assert equal_facts["same_tenant"] is True, identical
    assert equal_facts["owns_resource"] is True, identical
    assert VD.eval_request([["same_tenant"]], identical) is True, identical
    assert VD.eval_request([["owns_resource"]], identical) is True, identical


@given(
    values=st.lists(st.booleans(), min_size=len(VD.FACT_KEYS),
                    max_size=len(VD.FACT_KEYS))
)
@SETTINGS
def test_concretization_is_a_right_inverse(values: list[bool]) -> None:
    """``α(γ(f)) = f``, so every abstract counterexample has a real request."""
    facts = dict(zip(VD.FACT_KEYS, values, strict=True))
    concretized = VD.concretize(facts)
    admitted().validators["request"].validate(concretized)
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

    Only distinct pairs are constrained. This samples pairs rather than
    searching for a collision, so it is the weaker of the two renderer
    properties: a colliding pair the draw does not happen to select passes
    here. It is retained because a collision it does find is a direct
    counterexample, but the round-trip property below is the real guarantee.
    A de-duplicating renderer maps ``[["same_tenant"]]`` and
    ``[["same_tenant", "same_tenant"]]`` to the same text, so it is not
    injective; and a left inverse implies injectivity outright, since
    ``render(r1) = render(r2)`` gives ``r1 = decode(render(r1)) =
    decode(render(r2)) = r2``. The two properties are therefore not
    independent, and this one adds no strength the round trip does not
    already have.
    """
    assert rules == other or VD.render_source("baseline", rules, admitted().atoms) != (
        VD.render_source("baseline", other, admitted().atoms)
    ), f"renderer collided on {rules!r} and {other!r}"


@given(rules=RULES)
@SETTINGS
def test_rendered_text_preserves_the_rule_list(rules: list[list[str]]) -> None:
    """The rendered text must decode back to exactly the rule list given.

    Splitting on the fixed template recovers the atom sources, so the rule
    list is recoverable from the text. This is what a de-duplicating or
    reordering renderer violates, and it asserts the template, the atom
    order, and every atom's presence rather than a subset of them. Unlike the
    sampled pair comparison above, a left inverse of the renderer holds for
    every admitted rule list, so this property implies injectivity rather
    than merely sampling it.
    """
    text = VD.render_source("baseline", rules, admitted().atoms)
    body = [line for line in text.splitlines() if not line.startswith("#")]
    while body and not body[0]:  # the documented header ends with a blank line
        body.pop(0)
    assert body[:5] == ["package authz", "", "import rego.v1", "",
                        "default allow := false"], body[:5]
    # Decode from the raw text rather than the re-joined lines, which would
    # drop the trailing newline each rule block ends with.
    prefix = "default allow := false\n"
    rest = text[text.index(prefix) + len(prefix):]
    delimiter = "\nallow if {\n"
    # Everything between the default declaration and the first rule must be
    # accounted for, or arbitrary text -- a stray declaration, say -- could sit
    # there unnoticed; with zero rules there would be no chunk to inspect at
    # all. So the region must be empty when there are no rules, and otherwise
    # must begin with the delimiter and contain exactly one per rule, which
    # leaves no room for unexamined text before, between, or after the blocks.
    if not rules:
        assert rest == "", repr(rest[:60])
    else:
        assert rest.startswith(delimiter), repr(rest[:60])
        assert rest.count(delimiter) == len(rules), repr(rest[:60])
    recovered: list[list[str]] = []
    indent = "    "
    for chunk in rest.split(delimiter)[1:]:
        assert chunk.endswith("}\n"), chunk
        body_lines = chunk[:-2].splitlines()
        for line in body_lines:
            assert line.startswith(indent), chunk
        recovered.append([admitted().atom_sources[line[len(indent):]] for line in body_lines])
    assert recovered == rules, f"round trip changed {rules!r} into {recovered!r}"
    assert VD.header_reason("baseline", text[:text.index("package authz")]) is None


def main() -> int:
    """Run each property over the admitted domain and report the coverage.

    The admitted profile and schemas are read here, once, through the harness's
    read-only repository boundary, before any property runs. Every entry is
    asserted to be a Hypothesis-wrapped test, so a property accidentally
    stripped of its ``@given`` decorator fails here instead of silently running
    once against no examples and reporting a false pass.

    The printed figure is the *budget* passed to Hypothesis, not a measured
    execution count. Hypothesis can stop before reaching the budget, including
    when it exhausts a finite strategy. This output does not report an
    execution count or establish exhaustive coverage.
    """
    configure(VD.Repository(ROOT))
    properties = [
        test_concrete_and_abstract_evaluation_agree,
        test_abstraction_reads_the_declared_comparisons,
        test_equality_ignores_length_and_shared_prefix,
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
        print(f"ok  {property_check.__name__}  "
              f"(max_examples={SETTINGS.max_examples})")
    print(f"\n{len(properties)} properties passed under "
          f"max_examples={SETTINGS.max_examples}, derandomize=True")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
