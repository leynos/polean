# Lean package boundary

The technical design assigns formal syntax, semantics, abstraction lemmas,
registered claims, finite-checker soundness, witness soundness, and theorem
audits to a pinned Lean package.

Experiment E0 resolved the toolchain. `lean-toolchain` selects
`leanprover/lean4:v4.35.0`, `lakefile.toml` declares the package, and
`lake-manifest.json` records that the package graph is empty. The version, the
resolved build, and the provenance of the release archive are recorded in
[ADR 0003](../adrs/adr-0003-pin-the-e0-toolchain-tuple.md).

## The smoke library

`PoleanSmoke` is a compile-time probe, not policy code. It states two `decide`
theorems about a finite Boolean predicate and asserts, through `#guard_msgs`,
the axioms the kernel records for each. `make toolchain-smoke` builds it with
the provisioned binary and requires the `.olean`. It observes that the pinned
Lean and Lake work and what the kernel reports.

Nothing here proves anything about Polean. The smoke library defines no policy
semantics and is never imported by the future worker. 2.2.4 generates its
modules inside isolated job directories against this package's trusted library,
not against `PoleanSmoke`.

## What belongs here

Repository-owned Lake configuration, toolchain selection, trusted package
modules, and their proofs belong in this directory, and changes to them go
through review like any other source. What must never appear here is generated
Rust, arbitrary submitted Lean, submitted Lake configuration, or compiled proof
artefacts copied from an untrusted run. The proposed verifier generates fixed
data modules in isolated job directories and checks them against trusted
package code.

`.lake/` holds build output. It is ignored by version control and by the
Markdown linters, and deleting it costs only a rebuild.
