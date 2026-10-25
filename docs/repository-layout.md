# Repository layout

Polean adopts the small Rust repository shape used by Combobulate pull request
1, then adds first-class directories for the Lean boundary, policy contracts,
example source, and design validation. The current crate remains a scaffold.
The module split proposed by the technical design is not yet implemented.

## Paths and responsibilities

| Path                              | Responsibility                                                                         |
| --------------------------------- | -------------------------------------------------------------------------------------- |
| `src/lib.rs`                      | Current Rust crate root and disposable scaffold identity.                              |
| `tests/`                          | Rust integration tests for implemented public behaviour.                               |
| `tests/toolchain/`                | Observations of the pinned Rust, Regorus, Lean, and OPA tuple.                         |
| `lean/`                           | Pinned Lean package: toolchain selection, Lake configuration, and the smoke library.   |
| `contracts/`                      | Versioned profile, profile schema, request, policy, claims, result, and CLI contracts. |
| `examples/tenant-write/`          | Baseline Rego source, canonical IR, claims, requests, and mutations.                   |
| `validation/`                     | Design-fixture checks, property checks, and witnesses; not the production checker.     |
| `scripts/`                        | Digest-checked toolchain provisioning, with its pytest suite in `scripts/tests/`.      |
| `toolchain-pins.json`             | The authoritative upstream URL, digest, size, and version of each pinned artefact.     |
| `.polean/tools/`                  | Untracked, digest-named install root for the provisioned OPA binary and Lean archive.  |
| `docs/`                           | Terms of reference, technical design, guides, vocabulary, and decisions.               |
| `docs/contents.md`                | Canonical documentation index.                                                         |
| `docs/adrs/`                      | Accepted architecture decision records, named `adr-nnnn-title-slug.md`.                |
| `docs/rfcs/`                      | Reserved for proposed changes, named `rfc-nnnn-title-slug.md`; none are committed yet. |
| `.github/workflows/ci.yml`        | Hosted Rust, Markdown, and basic repository gate.                                      |
| `.github/workflows/design.yml`    | Independent design-contract validation lane.                                           |
| `.github/workflows/toolchain.yml` | Path-filtered provisioning and smoke run for the pinned tuple and its example policy.  |
| `.github/dependabot.yml`          | Cargo and GitHub Actions dependency updates.                                           |
| `Cargo.toml`, `Cargo.lock`        | Package metadata, lint policy, and dependency lockfile.                                |
| `rust-toolchain.toml`             | Pinned Rust toolchain and required components.                                         |
| `Makefile`                        | Public contributor entry points.                                                       |
| `AGENTS.md`                       | Contributor and coding-agent instructions.                                             |
| `.markdownlint-cli2.jsonc`        | Markdown lint policy.                                                                  |
| `.rustfmt.toml`, `clippy.toml`    | Rust formatting and lint policy.                                                       |
| `README.md`, `LICENSE`            | Project introduction and ISC licence.                                                  |

_Table 1: Repository paths and their responsibilities._

## Ownership boundaries

Keep implementation under `src/` until a working vertical slice creates a real
need for multiple crates. Keep Lean semantics and theorems under `lean/`; Rust
must not silently redefine them. Keep source-facing syntax and machine-readable
contracts under `contracts/` and `examples/`, rather than embedding fixtures in
tests or documentation.

`lean/` is repository-owned. Its `lakefile.toml`, `lean-toolchain`, and
`lake-manifest.json` are reviewed like any other source, and a change to them
is a toolchain-tuple change that needs the evidence ADR-003 describes. The
earlier prohibition on Lake configuration applies to _submitted_ configuration:
a policy author's Lake file, package, or compiled proof module is never
verifier input, and the worker checks generated data against the trusted
package in an isolated job directory instead.

`toolchain-pins.json` is the single authority for the upstream identity of the
provisioned artefacts; `scripts/provision_tools.py` reads it, verifies each
download against it, and installs under `.polean/tools/<name>/<sha256>/`. Do
not repeat those digests in prose, where they would become a second authority
and drift. `tests/toolchain/` observes the same pins at run time. OPA and Lean
live behind `#[ignore]`, because they need `make tools`; the Regorus
observations run everywhere.

The Python validation harness owns design-fixture consistency only. It must not
claim proof discharge, Rego semantic equivalence, or production runtime
correctness. Its properties are stated in the shape of the model obligations so
a false statement is caught early, but a passing search is sampling rather than
kernel-checked quantification, and it cannot stand in for the Lean development.
Once the Rust frontend or Lean package exists, their checks belong in separate
lanes with distinct result language.

Update [documentation contents](contents.md) when adding, removing, or renaming
durable documents. Prefer Make targets for contributor entry points. Generated
build output, local virtual environments, editor state, proof caches, and
submitted evidence bundles remain untracked.
