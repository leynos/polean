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
- [Bootstrap decision](adr-001-repository-bootstrap.md): repository provenance,
  selected Combobulate scaffold elements, and deliberate deferrals.
- [Draft-pack validation](../validation/README.md): checks that ran, results,
  and explicit omissions.

## Project guides

- [User guide](users-guide.md): current availability and the proposed workflow.
- [Developer guide](developers-guide.md): contributor commands and change
  discipline.
- [Repository layout](repository-layout.md): paths and ownership boundaries.
- [Documentation style guide](documentation-style-guide.md): prose, Markdown,
  decision-record, and assurance-language conventions.

## Machine-readable design

The [contracts directory](../contracts/) contains the profile and JSON schemas.
The [tenant-write example](../examples/tenant-write/) contains the baseline
policy, source-neutral policy IR, claims, mutations, and a sample request. The
[validation directory](../validation/) contains executable design-fixture
checks, recorded results, and mutation witnesses.
