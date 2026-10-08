# Draft-pack validation

Date: 2026-10-04. Scope: proposed contracts and design fixtures, not
implementation certification.

`validate_design.py` ran successfully in the authoring environment. Its
machine-readable result is [design-validation.json](design-validation.json).

| Check performed                       | Result                                                                                                                           |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| JSON Schema meta-validation           | Four schemas accepted by the installed Draft 2020-12 validator.                                                                  |
| Baseline and mutation IR              | Seven policy variants conform to the proposed policy schema.                                                                     |
| Concrete/abstract fixture comparison  | 224 comparisons: 32 vectors for each of seven policies.                                                                          |
| Requirement completeness sanity check | All 64 combinations of five facts and an allow/deny decision match the intended decision exactly when all six requirements hold. |
| Concretization                        | Every enumerated fact vector has a schema-valid request with the same facts.                                                     |
| Baseline                              | All six requirements pass; five of 32 vectors allow.                                                                             |
| Six policy mutations                  | Each fails precisely the intended requirement(s); concrete witnesses appear in `witnesses/`.                                     |
| Malformed request shape               | Five malformed variants rejected.                                                                                                |
| Strict ingress examples               | Duplicate keys, unpaired surrogates, invalid UTF-8, and non-JSON `NaN` rejected.                                                 |
| Unicode examples                      | Empty, ASCII, composed/decomposed accented, and supplementary-plane strings accepted without normalization.                      |
| Example result structure              | An explicitly inconclusive, unproved report satisfies the result schema.                                                         |

The source-fixture check compares prepared Rego text with the proposed atom
mapping. It is a consistency check, **not an independently verified parser or
proof of Rego semantics**. The independently stated six-requirement contract
provides a separate check on the intended Boolean behaviour.

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

The script rewrites the JSON validation summary and witness files. It does not
edit either design document, call the network, or modify a GitHub repository.

An additional authoring check verified local Markdown links, balanced code
fences, parseable JSON files, and Python syntax. It is not a substitute for the
repository's own Markdown linter. No Mermaid diagrams occur in this pack.
