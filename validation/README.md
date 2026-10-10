# Draft-pack validation

Date: 2026-10-04. Scope: proposed contracts and design fixtures, not
implementation certification.

`validate_design.py` ran successfully in the authoring environment. Its
machine-readable result is [design-validation.json](design-validation.json).

| Check performed                       | Result                                                                                                                                                                  |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| JSON Schema meta-validation           | Five schemas accepted by the installed Draft 2020-12 validator.                                                                                                         |
| Profile contract                      | The profile document conforms to its schema, and its atom, fact, claim, exclusion, limit, and assurance entries match the reviewed constants.                           |
| Claim registry                        | The claim manifest lists exactly the six fixed requirements.                                                                                                            |
| Baseline and mutation IR              | Seven policy variants conform to the proposed policy schema, and each variant differs from the baseline only inside its rule list.                                      |
| Concrete/abstract fixture comparison  | 224 comparisons: 32 vectors for each of seven policies.                                                                                                                 |
| Requirement completeness sanity check | All 64 combinations of five facts and an allow/deny decision match the intended decision exactly when all six requirements hold.                                        |
| Concretization                        | Every enumerated fact vector has a schema-valid request with the same facts.                                                                                            |
| Baseline                              | All six requirements pass; five of 32 vectors allow.                                                                                                                    |
| Six policy mutations                  | Each fails precisely the intended requirement(s); concrete witnesses appear in `witnesses/`.                                                                            |
| Witness replay                        | Every stored witness is re-derived from its request and rule list, and must still refute its named requirement.                                                         |
| Witness sweep                         | A witness file the current run did not generate fails the check, so a retired mutation cannot leave a stale counterexample behind.                                      |
| Malformed request shape               | Five malformed variants rejected.                                                                                                                                       |
| Strict ingress examples               | Duplicate keys, unpaired surrogates, invalid UTF-8, and non-JSON `NaN` rejected.                                                                                        |
| Unicode examples                      | Empty, ASCII, composed/decomposed accented, and supplementary-plane strings accepted; composed and decomposed tenant names remain distinct, with a same-tenant control. |
| Example result structure              | An explicitly inconclusive, unproved report satisfies the result schema.                                                                                                |
| Negative controls                     | Eleven injected fixture faults each detected by the check meant to catch them.                                                                                          |

The source-fixture check compares prepared Rego text with the proposed atom
mapping. It is a consistency check, **not an independently verified parser or
proof of Rego semantics**. The independently stated six-requirement contract
provides a separate check on the intended Boolean behaviour. The negative
controls exist because a consistency check that rejects every input proves no
more than one that accepts every input; each control requires the same
predicate to accept the reviewed fixture and reject one targeted fault.

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

## Reproduction

From the pack root, run:

```bash
python -m pip install -r validation/requirements.txt
python validation/validate_design.py
```

The script rewrites the JSON validation summary and witness files. It removes a
witness file whose variant no longer refutes anything, and fails the run if any
witness file in `validation/witnesses/` was not generated by that run, so the
committed witness set cannot drift away from the mutation set. It does not edit
either design document, call the network, or modify a GitHub repository.

The design workflow runs the same target and then requires a clean working tree
with `git status --porcelain`, so a validator run that changes or adds a
tracked or untracked file fails the lane instead of passing with unreviewed
generated output.

An additional authoring check verified local Markdown links, balanced code
fences, parseable JSON files, and Python syntax. It is not a substitute for the
repository's own Markdown linter. No Mermaid diagrams occur in this pack.
