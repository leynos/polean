# Documentation contents

This index covers Polean's current development scaffold and proposed first
verification slice. Design documents describe intended behaviour. They do not
establish that the checker, translator, Lean proofs, or runtime correspondence
already exist.

## Product direction and design

- [Terms of reference](terms-of-reference.md): problem, users, goals, scope,
  assumptions, and acceptance criteria.
- [Context and vocabulary](context.md): the shared meanings of profile,
  admission, model theorem, source binding, runtime correspondence, and
  evidence.
- [Technical design](technical-design.md): semantic profile, finite abstraction,
  proof obligations, Rust and Lean boundaries, evidence, CLI outcomes, and
  vertical experiments.
- [Implementation roadmap](roadmap.md): GIST delivery phases, dependencies,
  acceptance gates, and conditional extensions.
- [CLI configuration decision](adrs/adr-0004-cli-configuration.md): planned
  OrthoConfig boundary and hermetic verification.
- [Assurance boundary decision][adr-0002]: what a passing run proves,
  trusts, tests, and leaves outside the theorem.
- [Bootstrap decision][adr-0001]: repository provenance, selected Combobulate
  scaffold elements, and deliberate deferrals.
- [Toolchain tuple decision][adr-0003]: the pinned Rust, Regorus, Lean, and OPA
  tuple, its provenance, and the remaining gaps.
- [Toolchain tuple ExecPlan](execplans/1-2-1-pin-a-tested-compatibility-tuple.md):
  the work that pins the tuple, proves the provisioning path, and records its
  acceptance evidence.
- [Admission feasibility ExecPlan][admission-plan]: the approved plan for the
  spike and record that test whether the pinned frontend can enforce complete,
  fail-closed admission.
- [Admission feasibility record](admission-feasibility-record.md): the 1.2.2
  verdict per construct class, the AST gaps and their closures, and what the
  evidence does not establish.
- [Admission design decision][adr-0005]: AST mapping plus a canonical-token
  recognizer with a Verus-verified kernel.
- [Draft-pack validation](../validation/README.md): checks that ran, results,
  and explicit omissions.

## Project guides

- [User guide](users-guide.md): current availability and the proposed workflow.
- [Developer guide](developers-guide.md): contributor commands and change
  discipline.
- [Repository layout](repository-layout.md): paths and ownership boundaries.
- [Documentation style guide](documentation-style-guide.md): prose, Markdown,
  decision-record, and assurance-language conventions, including the
  `adr-nnnn-title-slug.md` and `rfc-nnnn-title-slug.md` naming scheme.

## Third-party tool documentation

- [OrthoConfig user's guide](ortho-config-users-guide.md): the imported v0.10.0
  configuration and CLI integration guide.
- [OrthoConfig v0.10.0 migration guide](ortho-config-v0-10-0-migration-guide.md):
  release-specific migration notes and capabilities.
- [OrthoConfig documentation provenance](ortho-config-provenance.md): source
  tag, immutable commit, paths, and checksums.

## Shared engineering guidance

- [Reliable testing in Rust via dependency injection][di-testing]: isolate
  system dependencies for deterministic tests.
- [Mastering test fixtures in Rust with `rstest`][rstest-fixtures]: compose
  fixtures and parameterized tests.
- [A systematic guide to effective Rust doctests][dry-doctests]:
  structure reusable, focused Rust doctests.
- [Navigating code complexity][complexity-guide]: recognize and refactor
  complexity antipatterns.
- [Scripting standards][scripting-standards]: guidance for reproducible,
  testable scripts.

[di-testing]: reliable-testing-in-rust-via-dependency-injection.md
[rstest-fixtures]: rust-testing-with-rstest-fixtures.md
[dry-doctests]: rust-doctest-dry-guide.md
[complexity-guide]: complexity-antipatterns-and-refactoring-strategies.md
[scripting-standards]: scripting-standards.md
[adr-0001]: adrs/adr-0001-bootstrap-polean-from-the-combobulate-scaffold.md
[adr-0002]: adrs/adr-0002-first-profile-assurance-boundary.md
[adr-0003]: adrs/adr-0003-pin-the-e0-toolchain-tuple.md
[adr-0005]: adrs/adr-0005-admit-by-ast-mapping-and-canonical-token-recognition.md
[admission-plan]: execplans/1-2-2-complete-module-admission-feasibility-record.md

## Machine-readable design

The [contracts directory](../contracts/) contains the profile, its schema, and
the request, policy, claims, and result JSON schemas. The
[tenant-write example](../examples/tenant-write/) contains the baseline policy,
source-neutral policy IR, claims, mutations, and a sample request. The
[validation directory](../validation/) contains executable design-fixture
checks, recorded results, and mutation witnesses.
