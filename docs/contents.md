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
- [Assurance boundary decision][adr-0002]: what a passing run proves,
  trusts, tests, and leaves outside the theorem.
- [Bootstrap decision][adr-0001]: repository provenance, selected Combobulate
  scaffold elements, and deliberate deferrals.
- [Toolchain tuple decision][adr-0003]: the pinned Rust, Regorus, Lean, and OPA
  tuple, its provenance, and the remaining gaps.
- [Toolchain tuple ExecPlan](execplans/1-2-1-pin-a-tested-compatibility-tuple.md):
  the work that pins the tuple, proves the provisioning path, and records its
  acceptance evidence.
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

## Machine-readable design

The [contracts directory](../contracts/) contains the profile, its schema, and
the request, policy, claims, and result JSON schemas. The
[tenant-write example](../examples/tenant-write/) contains the baseline policy,
source-neutral policy IR, claims, mutations, and a sample request. The
[validation directory](../validation/) contains executable design-fixture
checks, recorded results, and mutation witnesses.
