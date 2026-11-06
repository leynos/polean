# Architectural decision record (ADR) 0006: separate proof integrity from operational integrity

## Status

Accepted. Accepted with the revised
[1.2.3 ExecPlan](../execplans/1-2-3-demonstrate-type-and-axiom-auditing.md) on
2026-11-06.

## Date

2026-11-06.

## Context and problem statement

Roadmap 1.2.3 asks whether Polean can determine that Lean checked a requested
theorem. While planning it, an adversarial review attacked the proposed audit
on the pinned Lean 4.35.0 and defeated it. The attacks are real, and are
recorded in the [findings](../lean-environment-integrity-findings.md). A
`prelude` module that redefines the kernel-accelerated `Nat.gcd` derives
`False` with no axiom, and a same-named module can replace a trusted statement.

The attacks need Lean that Polean never accepts. Polean generates its modules
from trusted templates and constrained policy data, and technical design §7
forbids importing submitted proof artefacts. Defending against them inside
1.2.3 made a feasibility demonstration carry module provenance, search-path
control, and forgery fixtures, and blurred what the demonstration established.

The two concerns also have different natural owners. Phase 1 needs to show that
the audit interface is usable. Phase 2 needs it to build policy verification.
Phase 4 already owns evidence binding, fresh verification, and execution
isolation.

## Decision drivers

- Phase 1 stays a genuine feasibility demonstration.
- Phase 2 can build useful policy verification on a stated input model.
- No milestone defends against threats outside its stated inputs, and none
  claims to.
- The public assurance statement in ADR 0002 stays no stronger than its
  evidence.
- The adversarial findings stay available, so hardening does not start from
  nothing.

## Decision

Treat these as two separate properties.

**Proof integrity:** Lean's kernel checked exactly the requested proposition,
with complete theorem-root coverage and only approved transitive axioms.
Roadmap 1.2.3 demonstrates it, and 2.2.4 promotes it. It holds under the
generated-module input model in technical design §2.1, and does not detect a
violation of that model.

**Operational integrity:** the Lean environment that answered is the one the
evidence names, with no shadowed module, forged privileged name, or altered
trusted proof package. New roadmap item 4.3.2 owns it.

Responsibilities divide as follows.

| Concern                                     | Owner | Note                                           |
| ------------------------------------------- | ----- | ---------------------------------------------- |
| Kernel replay of non-toolchain modules      | 1.2.3 | The observation that makes the claim checkable |
| Exact statement, root coverage, kind        | 1.2.3 | Decided by Rust settlement                     |
| Approved foundations, by name               | 1.2.3 | Sound only under the input model               |
| Fixed templates that keep jobs in the model | 2.2.4 | No `prelude`, options, or metaprograms         |
| Bounded execution of a proof job            | 2.3.1 | Deadline, cleared environment, reaping         |
| Naming the identities to bind               | 4.1.1 | Includes the proof package and axiom policy    |
| Regenerating jobs and re-auditing them      | 4.1.3 | Never imports submitted artefacts              |
| Hostile evidence across bound identities    | 4.2.3 | Substitution of what 4.1.1 names               |
| Isolation, hangs, floods, enforced budgets  | 4.3.1 | Depends on 1.2.4                               |
| Module provenance and shadowing             | 4.3.2 | Fixed search order; class by location          |
| Privileged-name forgery                     | 4.3.2 | Replay always on the toolchain's `Init`        |
| Trusted package matches its bound identity  | 4.3.2 | Enforces what 4.1.1 names and 4.1.3 compares   |
| Toolchain replay depth, independent checker | 4.3.2 | A measured decision                            |
| Acceptance measurements                     | 4.3.3 | Runs after hardening, so it measures it        |

_Table 1: Which milestone owns each integrity concern._

Development reports from E0 and E1 name the input model. Only public `prove` and
`verify`, which depend on 4.3.2, may make the ADR 0002 statement.

## Consequences

The 1.2.3 worker shrinks to replay plus observation, using upstream
`Lean.collectAxioms`. A prototype of about 85 lines reported its corpus as the
plan expects. Its corpus drops the forgery fixtures and keeps representative
positive and negative cases, and Phase 2 inherits a small interface.

Between 1.2.3 and 4.3.2, an audit can accept a forged result if the input model
is broken. A template defect that emitted `prelude`, or a tampered proof
package, would go undetected, and the result would look like any other. This is
the cost of the decision. It is bounded because no external party can supply
Lean, nothing public makes the ADR 0002 statement before 4.3.2, and development
reports state the condition. One of the deferred checks is unusually cheap:
pinning `Init` into the replay base is one line, so 4.3.2 could adopt it early
if a template defect ever looks plausible.

The first release still depends on 4.3.2. The sequence delays hardening without
removing it.

## Alternatives considered

**Keep the hardening in 1.2.3.** This gives the strongest early guarantee, and
the prototype shows the code is small. It was the previous revision of the
plan. The tests, contract fields, and specification it adds defend against
inputs that cannot occur until a later phase changes the threat model, and they
would need revisiting once 4.1.1 defines package identity.

**Drop kernel replay too, and trust compilation.** Fixed templates cannot set
`debug.skipKernelTC`, so this is defensible under the input model, and simpler
still. It would leave "kernel-checked" as an assumption, with no observation
that could ever fail, which is the question 1.2.3 exists to answer. Replay
costs about a second.

**Adopt `lake comparator`.** It is the upstream gold standard for hostile
proofs, and does not trust toolchain `.olean` files. It needs `bwrap` or an
explicit unsandboxed flag, exports the environment on each run at a cost of
about two minutes, and reports in text. It remains an option for 4.3.2's
independent check.

## Outstanding decisions

Roadmap 4.3.2 decides whether a release mode replays toolchain modules, at a
measured 91 s against a proposed 60 s budget, and whether to add an independent
checker.

## Acceptance

Accepted by the project sponsor on 2026-11-06, together with approval of
revision 4 of the
[1.2.3 ExecPlan](../execplans/1-2-3-demonstrate-type-and-axiom-auditing.md).
The measured results are recorded in that plan's `Outcomes & retrospective`
section as implementation completes.
