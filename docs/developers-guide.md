# Developer guide

Polean currently consists of a Rust scaffold and an executable design pack. The
first implementation milestone must preserve the assurance distinctions in the
technical design rather than racing ahead to a broad Rego interpreter.

## Prerequisites

- Git;
- the Rust toolchain pinned by `rust-toolchain.toml`;
- Python 3.11 or later; and
- `jsonschema` from `validation/requirements.txt` for design checks.

Hosted CI installs Markdown tooling separately. Local contributors may install
`mdtablefix` 0.6.0 or later and `markdownlint-cli2` to reproduce every formatting
and prose gate.

## Common commands

```bash
make build
make check-fmt
make lint
make test
make design-check
make all
```

`make design-check` may rewrite the committed validation summary and witness
files after verifying them. Review those changes rather than assuming any
regeneration is harmless.

## Change discipline

Keep functional changes and assurance-claim changes explicit. A parser feature,
new atom, wider input domain, changed failure outcome, or revised trust boundary
must update the relevant contracts and design text before implementation is
reported complete.

Do not add a Rego form merely because Regorus can parse it. Admission requires a
specified model, source mapping, proof strategy, conformance fixtures, and
failure semantics. Unknown syntax fails closed.

Do not accept submitted Lean, Lake, native-library, or compiled proof artefacts
as executable input. The proposed verifier regenerates fixed modules from
admitted data in a pinned environment.

## Tests

Add the smallest test at the boundary where a defect could recur:

- Rust unit and integration tests for orchestration and contracts;
- design-fixture validation for schema and example consistency;
- Lean theorems for semantic, abstraction, checker-soundness, and witness claims;
- OPA and Regorus replay tests for observed source behaviour; and
- hostile-input tests for evidence binding and process supervision.

Passing one class of test does not upgrade another assurance dimension. In
particular, differential testing is not a source-to-model proof.

## Documentation

Start with [documentation contents](contents.md). Update the terms of reference
when the problem, scope, users, or acceptance criteria change. Update the
technical design when implementation or assurance mechanisms change. Record
substantive, durable choices as ADRs.
