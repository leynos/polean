# ADR 001: bootstrap Polean from the Combobulate scaffold

- **Status:** accepted for the initial repository PR.
- **Date:** 2026-10-08.
- **Decision owners:** repository sponsor and maintainers.

## Context

Polean began as a design conversation and a detached design pack. The target
repository contained only a Rust `.gitignore`. The project needed enough
structure to review the documents, run their fixture validation, and begin a
Rust implementation without inventing a new estate convention.

Combobulate pull request 1 supplies a recent df12 Rust repository baseline with
an ISC licence, pinned Rust toolchain, strict lint policy, Make entry points,
GitHub Actions layout, dependency automation, documentation index, repository
guide, and explicit scaffold status.

Polean also needs boundaries that Combobulate does not: a future Lean package,
versioned policy contracts, exact Rego examples, mutation witnesses, and a
separate design-validation lane.

## Decision

Adopt the following structural patterns from Combobulate pull request 1 at
commit `6c9f1c37e278bf4c8a9c01ad5e47413b4bb60e17`:

- one Rust package while the implementation boundary remains unsettled;
- an ISC licence and package metadata at the repository root;
- strict Rust and Markdown configuration;
- public Make targets for formatting, linting, testing, and design checks;
- separate hosted CI and design-contract workflows;
- Dependabot coverage for Cargo and GitHub Actions;
- `docs/contents.md`, project guides, a repository-layout document, and ADRs;
- explicit wording that proposed APIs and proofs are not implemented; and
- a disposable Rust scaffold rather than speculative product code.

Import the Polean design pack under `docs/`, `contracts/`, `examples/`, and
`validation/`. Reserve `lean/` with a boundary document, but do not invent a
Lean version or package graph before experiment E0 resolves the compatible
toolchain.

Defer Combobulate's hardened mold and Cranelift installation scripts, Act
contract suite, coverage publication, mutation-testing schedule, Whitaker
capability probe, and large generic test harness. Those mechanisms solve real
problems in an established Rust implementation, but copying them before Polean
has functional Rust or Lean code would enlarge the first review without
advancing the vertical slice.

## Consequences

The repository can validate the design pack immediately and has conventional
places for Rust code, Lean work, contracts, examples, tests, and durable
decisions. Contributors can use familiar df12 entry points without treating the
full Combobulate bootstrap as a cargo cult reliquary.

The first CI does not yet exercise coverage, mutation testing, Whitaker, OPA,
Regorus, Lean, sandboxing, or proof replay. Adding any of those requires a
working implementation boundary, explicit pins, and tests for the assurance
claim it supports.

The initial Rust toolchain pin comes from the inspected Combobulate baseline.
The project must revisit it alongside the Regorus, OPA, and Lean tuple during
experiment E0 rather than assuming that shared provenance proves compatibility.

## Alternatives considered

Copying the entire Combobulate tree would provide more infrastructure but also
more unrelated tests, generators, and policy. Starting with documents alone
would leave no maintained implementation or CI shape. Creating the full
multi-crate Rust and Lean architecture now would turn proposed seams into
premature commitments.
