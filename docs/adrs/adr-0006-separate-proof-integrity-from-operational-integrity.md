# Architectural decision record (ADR) 0006: separate proof integrity from operational integrity

## Status

Accepted. Accepted with the revised
[1.2.3 ExecPlan](../execplans/1-2-3-demonstrate-type-and-axiom-auditing.md) on
2026-11-06.

Proof integrity and operational integrity are separate properties with separate
owners: 1.2.3 demonstrates what a settled audit report means, and 4.3.2 owns
defending the environment that produced it.

## Date

2026-11-06.

## Context and problem statement

Roadmap 1.2.3 asks whether Polean can determine that Lean checked a requested
theorem. While planning it, an adversarial review attacked the proposed audit
on the pinned Lean 4.35.0 and defeated it. The attacks are real, and are
recorded in the [findings](../lean-environment-integrity-findings.md). A
`prelude` module that redefines the kernel-accelerated `Nat.gcd` derives
`False` with no axiom, and a same-named module can replace a trusted statement.

The attacks need Lean that Polean does not accept as job source. Polean
generates its modules from trusted templates and constrained policy data, and
technical design §7 forbids importing submitted proof artefacts. That excludes
a hostile job, not a tampered trusted proof package: a package altered on disk
is still loaded, and 4.3.2 owns detecting it. Defending against the attacks
inside 1.2.3 made a feasibility demonstration carry module provenance,
search-path control, and forgery fixtures, and blurred what the demonstration
established.

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
`verify` may make the ADR 0002 statement, and only once 4.3.2 has established
environment integrity. Roadmap 4.2.2 requires 4.2.1 for the command contract
alone; until 4.3.2 completes, its success outcomes are development reports that
name the generated-module input model. The gate sits on the statement rather
than on the command contract, so roadmap 5.2.1 — which documents the workflow
and the assurance ledger — is what requires 4.3.2.

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
reports state the condition. The worker already pins `Init` into the replay
base, so that part of 4.3.2 is in place; what remains deferred is fixing the
whole base and classifying each module by where it resolved, which is what
makes the base trustworthy rather than merely present.

The first release still depends on 4.3.2. The sequence delays hardening without
removing it.

## Measured results

The decision was implemented in 1.2.3 and measured on the pinned tuple.

- The worker is `lean/PoleanAudit.lean`, 186 lines. It replays the requested
  non-toolchain modules over a base of the toolchain's `Init` plus the
  toolchain modules those imports directly use, collects axioms with
  `Lean.collectAxioms`, and prints one compact JSON report. It never sees the
  expected statement and emits no verdict.
- `tests/toolchain/audit/settlement/` decides acceptance. Verus verifies 64
  obligations over it, and the recognizer proof still verifies 89. Fifteen
  negative controls (five admission, ten settlement) are rejected with a
  diagnostic naming the expected failure kind and source path; the checker
  matches that text, so a control shows its edit is detected and located, not
  that the first failing obligation is the one the specification intended.
- The corpus is twenty-five modules under `examples/audit/lean/AuditFixture/`
  — twenty audited fixtures and five support modules — plus two programs, all
  checked against a hand-written manifest rather than a generated one.
- `make toolchain-smoke` runs the corpus end to end as nine tests.
- On the shared development host, fixture compilation took 0.66–4.94 s and an
  audit took 1.1–14.9 s with peak child RSS of 425–799 MiB. Those are upper
  bounds under contention, not budgets. The accepted report was 404 bytes and
  the sixteen-rule maximal policy report 54,371 bytes, against a 1 MiB bound.

## Outcome mapping for 2.2.4

Promotion is 2.2.4's work, not 1.2.3's. When it happens, settlement's three
outcomes map onto the public contract as follows.

| Settlement outcome    | Public outcome     | Exit | Why                                                              |
| --------------------- | ------------------ | ---- | ---------------------------------------------------------------- |
| `Rejected(findings)`  | `invalid_evidence` | 7    | A theorem, root, or axiom-policy requirement was not met         |
| `ToolFailure(breach)` | `tool_error`       | 6    | The report did not answer the request, so no policy claim exists |
| `Err(RequestRefusal)` | `tool_error`       | 6    | Rust generated the request, so a refusal is an internal fault    |
| deadline kill         | `unknown`          | 3    | Available only once 2.3.1 enforces a deadline                    |

_Table 2: How 1.2.3's settlement outcomes reach the public command contract._

An `accepted` settlement alone never exits 0. Exit 0 additionally requires the
six proved claims, bound identities, and tested runtime agreement, and it may
carry the ADR 0002 statement only after 4.3.2.

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
