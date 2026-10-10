# Repository layout

Polean adopts the small Rust repository shape used by Combobulate pull request
1, then adds first-class directories for the Lean boundary, policy contracts,
example source, and design validation. The current crate remains a scaffold.
The module split proposed by the technical design is not yet implemented.

## Paths and responsibilities

| Path                           | Responsibility                                                                         |
| ------------------------------ | -------------------------------------------------------------------------------------- |
| `src/lib.rs`                   | Current Rust crate root and disposable scaffold identity.                              |
| `tests/`                       | Rust integration tests for implemented public behaviour.                               |
| `lean/`                        | Reserved home of the pinned Lean package once experiment E0 resolves its toolchain.    |
| `contracts/`                   | Versioned profile, profile schema, request, policy, claims, result, and CLI contracts. |
| `examples/tenant-write/`       | Baseline Rego source, canonical IR, claims, requests, and mutations.                   |
| `validation/`                  | Design-fixture checks and concrete witnesses; not the production checker.              |
| `docs/`                        | Terms of reference, technical design, guides, vocabulary, and decisions.               |
| `docs/contents.md`             | Canonical documentation index.                                                         |
| `.github/workflows/ci.yml`     | Hosted Rust, Markdown, and basic repository gate.                                      |
| `.github/workflows/design.yml` | Independent design-contract validation lane.                                           |
| `.github/dependabot.yml`       | Cargo and GitHub Actions dependency updates.                                           |
| `Cargo.toml`, `Cargo.lock`     | Package metadata, lint policy, and dependency lockfile.                                |
| `rust-toolchain.toml`          | Pinned Rust toolchain and required components.                                         |
| `Makefile`                     | Public contributor entry points.                                                       |
| `AGENTS.md`                    | Contributor and coding-agent instructions.                                             |
| `.markdownlint-cli2.jsonc`     | Markdown lint policy.                                                                  |
| `.rustfmt.toml`, `clippy.toml` | Rust formatting and lint policy.                                                       |
| `README.md`, `LICENSE`         | Project introduction and ISC licence.                                                  |

## Ownership boundaries

Keep implementation under `src/` until a working vertical slice creates a real
need for multiple crates. Keep Lean semantics and theorems under `lean/`; Rust
must not silently redefine them. Keep source-facing syntax and machine-readable
contracts under `contracts/` and `examples/`, rather than embedding fixtures in
tests or documentation.

The Python validation harness owns design-fixture consistency only. It must not
claim proof discharge, Rego semantic equivalence, or production runtime
correctness. Once the Rust frontend or Lean package exists, their checks belong
in separate lanes with distinct result language.

Update [documentation contents](contents.md) when adding, removing, or renaming
durable documents. Prefer Make targets for contributor entry points. Generated
build output, local virtual environments, editor state, proof caches, and
submitted evidence bundles remain untracked.
