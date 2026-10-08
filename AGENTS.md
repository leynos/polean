# Assistant instructions

## Orientation

Start with [documentation contents](docs/contents.md), the
[terms of reference](docs/terms-of-reference.md), the
[technical design](docs/technical-design.md), and the
[repository layout](docs/repository-layout.md).

The repository currently contains a design scaffold, not a working policy
checker. Do not describe proposed commands, Lean theorems, Regorus integration,
OPA replay, source translation, or evidence verification as implemented.

## Assurance discipline

Keep these claims separate in code, tests, diagnostics, and documentation:

1. a source policy belongs to an admitted profile;
2. the Polean model satisfies a registered requirement;
3. source-to-model translation corresponds to the submitted Rego; and
4. OPA or another runtime corresponds to the model.

A passing differential test does not prove translation. A checked model theorem
does not authenticate request attributes or certify an application integration.
Unsupported syntax fails closed.

Do not accept arbitrary user Lean, Lake files, compiled proof modules, native
libraries, or plug-ins as verifier input. Generated proof jobs must contain data
inside fixed trusted templates.

## Code and documentation

Write clear, small functions with precise names. Comment assumptions and
trade-offs rather than narrating syntax. Use British English with Oxford
spelling in prose. Keep Rust modules under 400 lines and ordinary functions
under 70 lines unless a documented reason justifies an exception.

Every Rust module begins with a module-level Rustdoc comment. Document public
items and include useful examples. Avoid `unsafe`; if it becomes unavoidable,
record the boundary and rationale before implementation.

Update durable documentation and machine-readable contracts whenever behaviour,
assumptions, profile syntax, claims, failure outcomes, or trust boundaries
change. Add or update an ADR for substantive architectural decisions.

## Quality gates

Run the relevant subset of:

```bash
make check-fmt
make lint
make test
make design-check
make markdownlint
```

Add regression tests at the boundary where a fault could recur. A new policy
atom or input distinction requires model semantics, abstraction analysis,
fixtures, claims or proof updates, and source-runtime conformance coverage. Do
not widen the grammar merely to make a parser test pass.

Use imperative commit subjects and explain the reason and assurance impact in
the body. Keep refactoring separate from functional changes where practical.
