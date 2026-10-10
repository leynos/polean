# Polean

Polean is an experimental, source-first checker for establishing named security
properties of deliberately constrained Rego policies with Lean-checked model
evidence.

The first vertical slice is intentionally small. It admits one tenant-scoped
write-policy profile, reduces its observable behaviour to five Boolean facts,
and proposes Lean theorems connecting that finite abstraction to every valid
request in the declared input domain. The checker must also preserve positive
access requirements, so replacing a policy with “deny everything” does not pass
merely because it grants nothing dangerous.

## Current reality

This repository contains a Rust development scaffold and the revision 0.1
design pack. It does **not** yet contain:

- a Polean command-line implementation;
- a Regorus frontend;
- an OPA conformance runner;
- a Lean package or checked proof;
- verified Rego-to-model translation; or
- evidence that a production authorization path refines the model.

The crate exports only a disposable scaffold identity. Commands and theorem
interfaces shown in the design documents are proposals rather than functioning
APIs.

The distinction matters. The intended first assurance statement is that Lean
checked the admitted policy *model* against six requirements. The source
frontend remains explicitly trusted, while OPA and Regorus supply tested
runtime observations rather than a formal refinement theorem.

## The example policy

The initial profile models a multi-tenant document service:

- administrators may write resources within their own tenant;
- owners may write their own unlocked resources within their tenant;
- administrator status does not bypass tenant isolation; and
- only write actions may be authorized.

Four safety requirements and two positive access requirements characterize that
behaviour. Six deliberately weakened policies provide concrete regressions,
including cross-tenant administrator access, locked owner writes, non-owner
access, non-write administrator access, deny-all, and administrator-only access.

## Start here

Read the [terms of reference](docs/terms-of-reference.md) for the problem,
users, scope, assumptions, and acceptance criteria. Then read the
[technical design](docs/technical-design.md) for the semantic profile, proof
strategy, trust boundaries, evidence format, orchestration responsibilities,
and vertical experiments.

The [documentation index](docs/contents.md) links the remaining guides and
contracts. The [bootstrap ADR](docs/adr-001-repository-bootstrap.md) records
which parts of the Combobulate scaffold were adopted and which were
deliberately deferred.

## Reproduce the design checks

Use Python 3.11 or later:

```bash
python -m venv .venv
. .venv/bin/activate
python -m pip install -r validation/requirements.txt
make design-check
```

The design check validates JSON contracts, including the profile against its
own schema and reviewed vocabulary; enumerates the 32 abstract fact vectors for
seven policy variants; compares concrete and abstract fixture interpretations;
checks that the six requirements characterize the intended decision; replays
each mutation witness against its rule list; and fails if
`validation/witnesses/` holds a witness the current run did not generate. It
does not parse Rego with OPA or Regorus and does not invoke Lean.

The Rust scaffold gates are:

```bash
make check-fmt
make typecheck
make lint
make test
```

## Repository status

Polean is a feasibility experiment. Full Rego support, arbitrary user-defined
invariants, temporal authorization properties, a hosted proving service, and a
replacement policy engine all remain outside the first slice. Unsupported
syntax must fail closed rather than receive an approximation dressed in a lab
coat.

Polean is licensed under the [ISC licence](LICENSE).
