# Draft-pack validation

Date: 2026-10-04. Scope: proposed contracts and design fixtures, not
implementation certification.

`validate_design.py` ran successfully in the authoring environment. Its
machine-readable result is [design-validation.json](design-validation.json).

| Check performed                       | Result                                                                                                                                                                                                                        |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| JSON Schema meta-validation           | Five schemas accepted by the installed Draft 2020-12 validator.                                                                                                                                                               |
| Profile contract                      | The profile document conforms to its schema, and its atom, fact, claim, exclusion, limit, and assurance entries match the reviewed constants.                                                                                 |
| Claim registry                        | The claim manifest lists exactly the six fixed requirements.                                                                                                                                                                  |
| Baseline and mutation IR              | Seven policy variants conform to the proposed policy schema, and each variant differs from the baseline only inside its rule list.                                                                                            |
| Module documentation                  | Each of the seven fixtures is rendered with the reviewed module header that names it, and that header is checked structurally before use.                                                                                     |
| Concrete/abstract fixture comparison  | 224 comparisons: 32 vectors for each of seven policies.                                                                                                                                                                       |
| Requirement completeness sanity check | All 64 combinations of five facts and an allow/deny decision match the intended decision exactly when all six requirements hold.                                                                                              |
| Concretization                        | Every enumerated fact vector has a schema-valid request with the same facts.                                                                                                                                                  |
| Baseline                              | All six requirements pass; five of 32 vectors allow.                                                                                                                                                                          |
| Six policy mutations                  | Each fails precisely the intended requirement(s); concrete witnesses appear in `witnesses/`.                                                                                                                                  |
| Witness replay                        | Every stored witness is re-derived from its request and rule list, and must still refute its named requirement.                                                                                                               |
| Witness sweep                         | A witness file the current run did not generate fails the check and is reported rather than deleted, so a retired mutation cannot leave an unnoticed stale counterexample behind.                                             |
| Malformed request shape               | Five malformed variants rejected.                                                                                                                                                                                             |
| Strict ingress examples               | Duplicate keys, unpaired surrogates, invalid UTF-8, and non-JSON `NaN` rejected.                                                                                                                                              |
| Unicode examples                      | Empty, ASCII, composed/decomposed accented, and supplementary-plane strings accepted; composed and decomposed tenant names remain distinct, with a same-tenant control.                                                       |
| Example result structure              | An explicitly inconclusive, unproved report satisfies the result schema.                                                                                                                                                      |
| Negative controls                     | Nineteen injected fixture faults each detected by the check meant to catch them.                                                                                                                                              |
| Storage-boundary seams                | Two seams: reviewing the loaded fixtures leaves every witness in `witnesses/` unchanged in size, modification time, and SHA-256 digest, and publishing through a temporary root writes there and leaves the repository alone. |
| Boundary tests                        | Ten independent tests of the orchestration and storage boundary, each falsified against a targeted injected fault.                                                                                                            |

`property_fixtures.py` states the same model-side obligations as properties and
searches the admitted input domain for counterexamples, with Hypothesis
shrinking any it finds while keeping the value valid. It checks that concrete
and abstract evaluation agree, that abstraction reads the declared comparisons
rather than merely the declared key names, that only whole-string equality
drives a decision (pinned by explicit shared-prefix and over-cap boundary
cases), that concretization is a right inverse for the abstraction, that
`unlocked` is derived from the locked fact rather than an independent sixth
fact, that the six claims permit exactly the intended decision, and that
rendering is injective inside the profile bounds. It also decodes rendered text
back to its rule list, which is what a de-duplicating or reordering renderer
violates, and it requires the region between the default declaration and the
first rule to be exactly the expected template, so stray text there cannot
escape inspection.

The two renderer properties are not independent. The round trip is a left
inverse, which implies injectivity outright: if `render(r1) = render(r2)` then
`r1 = decode(render(r1)) = decode(render(r2)) = r2`. The sampled pair
comparison is retained because a collision it finds is a direct counterexample,
but it is the weaker of the two and adds no guarantee the round trip does not
already give.

The string domains are capped for search cost. A cap is a restriction, not a
completeness argument: distinct strings can share more than the cap in leading
characters, and an implementation fault could activate only beyond it. The
boundary property draws pairs that cross the cap, with both members longer than
it and one a strict extension of the other, and controls them against the same
long string compared with itself; long strings also reach the tenant, id, and
owner draws. That covers two specific shapes of long input, not every
arrangement, so the cap still bounds what the general properties can reach.

The example count the module prints is the budget passed to Hypothesis, not a
measured execution count. Hypothesis can stop before reaching it, including
when it exhausts a finite strategy, so the output reports neither an execution
count nor exhaustive coverage.

The module shares its definitions with `validate_design.py` rather than
restating them, so it tests the harness the other check runs rather than a
parallel copy, and it asserts that each property is Hypothesis-wrapped before
running it, so a property that lost its `@given` decorator fails rather than
sampling no examples and reporting a false pass.

It is a search, not a proof, and it does not replace the enumeration: a
property suite can miss a fault that the fixed vectors happen to cover, and the
fixed vectors cannot cover inputs outside the fixture set. Passing either check
establishes model-side consistency only. Neither parses Rego, compares OPA or
Regorus behaviour, or checks a Lean theorem, so neither supports a
source-to-model or runtime-refinement claim.

Validation and storage are separated by an explicit boundary. The reviewed
documents are read once through a read-only `Repository`, and the checks
receive them as data, so the ordinary checks have no storage dependency to
exercise. Python does not prevent a function from opening a file, so what the
boundary establishes is architectural separation, not enforced isolation: the
observed functions did not reach for the filesystem. Only the publishing
command holds the `Workspace` write boundary, and it receives that workspace as
an argument rather than reaching for a module-level path. A read or write that
fails is reported as one `HarnessIOError` naming the document, and a document
that does not satisfy its contract is reported as one named diagnostic rather
than as a `jsonschema` traceback. A run that fails a check before publication
writes nothing at all; a failure inside publication can leave the witnesses it
had already written, but never `design-validation.json`, which is written only
after every check and the publication itself have returned.

Two seam checks make the boundary observable rather than aspirational.
Producing a review must leave every file in `validation/witnesses/` unchanged
in size, modification time, and SHA-256 digest, which catches a check that
rewrote one. This covers those JSON files around `validate_policies` alone, not
every repository file and not every check, and it cannot see a read or a
transient change that was undone before the second snapshot. Publishing that
same review through a temporary workspace must write the witnesses under that
root, replay them from there, and leave the repository alone, which catches a
command that ignored the boundary it was handed. The stale-witness set is
computed from the directory as it was found, before anything is written, so a
run rejected for a stale file leaves the tree untouched. Rejection is not
removal: an unexpected file is reported and left where it is for a person to
delete, because the run stops before writing anything. Only a reviewed variant
that no longer refutes anything has its own witness removed, and only in a run
that passes the sweep.

Importing `property_fixtures.py` reads no profile or schema document, though it
does execute the sibling harness to reuse its definitions. The admitted profile
and schemas are read by one explicit `load_admitted()` call in `main`, and the
result is passed to `Suite` as a constructor argument, so a property reads
`self.admitted` rather than a module-level value and there is no configured
global for a run to inherit.

`test_boundary.py` tests the harness the other checks run. The negative
controls exercise lower-level predicates, so on their own they would not notice
a `validate_policies` replaced by one producing the review from the committed
documents rather than from the fixtures it was handed, because publication
replays whatever review it is given. The boundary tests call the real entry
points with deliberately altered inputs and require the result to change or be
rejected, and they snapshot the tree to observe both storage seams directly.
Each of the ten was falsified against a targeted injected fault before being
relied on.

The source-fixture check compares prepared Rego text with the proposed atom
mapping. It is a consistency check, **not an independently verified parser or
proof of Rego semantics**. Each fixture is re-rendered from its rule-list IR
under a reviewed module header, and the check fails on any byte difference from
the committed file, so a hand-edited fixture is rejected rather than silently
tolerated. The header is the one part of the text not derived from the IR, so
it is checked structurally as well: it must be a single comment block whose
first line names its own fixture. The independently stated six-requirement
contract provides a separate check on the intended Boolean behaviour. The
negative controls exist because a consistency check that rejects every input
proves no more than one that accepts every input; each control requires the
same predicate to accept the reviewed fixture and reject one targeted fault.

## Checks not performed

Lean, OPA, Regorus, and a Rust toolchain were not available as installed
executables for this drafting run. No Lean proof was compiled or kernel
checked; no OPA/Regorus source parsing or runtime conformance comparison ran;
no Rust orchestrator, sandbox, evidence verifier, or axiom-audit implementation
was built. Those remain the technical design's explicit implementation and
release gates.

Python enumeration is a finite design sanity check. The proposed universal
request theorem requires the Lean abstraction, coverage, checker-soundness, and
witness lemmas. Runtime equivalence and source translation require their own
assurance work even after those lemmas have been proved.

`abstraction_preserves`, `check_sound`, and `witness_sound` remain **design
obligations, not proved theorems**. The Python properties are stated in the
same shape as those obligations and give evidence that the statements are not
vacuous, but a passing Hypothesis run is sampling, not kernel-checked
quantification: it cannot discharge a `∀` over the request domain, and it
depends on the same Python definitions whose soundness a Lean development would
establish independently. No Lean package exists in this repository, and the
theorem interfaces in the technical design remain signatures rather than
compiled source.

## Reproduction

From the pack root, run:

```bash
python -m pip install -r validation/requirements.txt
python validation/validate_design.py
python validation/property_fixtures.py
python validation/test_boundary.py
```

The publishing command rewrites the JSON validation summary and witness files
through its workspace boundary. It removes a witness file whose variant no
longer refutes anything, and fails the run if any witness file in
`validation/witnesses/` was not generated by that run, so the committed witness
set cannot drift away from the mutation set. It does not edit either design
document, call the network, or modify a GitHub repository.

The design workflow runs the same target and then requires a clean working tree
with `git status --porcelain`, so a validator run that changes or adds a
tracked or untracked file fails the lane instead of passing with unreviewed
generated output.

An additional authoring check verified local Markdown links, balanced code
fences, parseable JSON files, and Python syntax. It is not a substitute for the
repository's own Markdown linter. No Mermaid diagrams occur in this pack.
