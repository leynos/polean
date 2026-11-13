# Lean package boundary

The technical design assigns formal syntax, semantics, abstraction lemmas,
registered claims, finite-checker soundness, witness soundness, and theorem
audits to a pinned Lean package.

Experiment E0 resolved the toolchain. `lean-toolchain` selects
`leanprover/lean4:v4.35.0`, `lakefile.toml` declares the package, and
`lake-manifest.json` records that the package graph is empty. The version, the
resolved build, and the provenance of the release archive are recorded in
[ADR 0003](../docs/adrs/adr-0003-pin-the-e0-toolchain-tuple.md).

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

## The audit worker

`PoleanAudit.lean` is the 1.2.3 worker, declared in `lakefile.toml` as the
`polean-audit` executable. It is not a verifier. Given a module and a list of
roots, it replays the module's non-toolchain imports over a base of the
toolchain's `Init` plus the toolchain modules those imports directly use,
collects each root's transitive axioms with `Lean.collectAxioms`, and prints
one compact JSON report. It never sees the expected statement and emits no
verdict; Rust settlement decides.

The worker fails closed on anything outside its grammar. It reports a statement
tree over 4,096 nodes or depth 60 as `null` and never truncates it. It reports
a replay failure and does not work around it. `tests/toolchain/smoke/` builds
the worker and runs it over the corpus in `examples/audit/`.

## The bounded-output module and its tests

The statement renderer and the diagnostic truncation live in
`PoleanAudit/Render.lean` rather than in the worker, so they can be tested
without the worker's `main`. `PoleanAuditRenderTests.lean` is a second
executable, `polean-audit-render-tests`, that drives `treeBounded` directly
over in-bound and over-bound spines and over a self-applying chain whose
expanded tree is exponential in the graph it denotes, and drives `truncateUtf8`
at and beyond the contract's message bound, including characters that would
cross it. Beside those fixed cases it generates inputs: mixed-width strings at
every limit, checked for prefix, fit and maximality against the input's own
prefixes, and a tree family under every combination of depth and node bounds,
checked against a fold over the `Expr` that counts nodes and depth from the
grammar alone. `tests/toolchain/smoke/audit/render.rs` builds it and runs it
under a deadline, so a bypassed node budget fails as a timeout rather than
hanging the gate.

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
