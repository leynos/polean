# Ratify the assurance boundary and record release prerequisites

This ExecPlan is a living document. Keep `Constraints`, `Tolerances`, `Risks`,
`Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan`
current as work proceeds.

Status: COMPLETE (revision 4)

Roadmap item: [1.1](../roadmap.md), tasks 1.1.1 and 1.1.2. Branch:
`1-1-ratify-assurance-and-release-gates`.

## Purpose / big picture

Polean's value depends on saying exactly what it proves and nothing more. This
work produces one ADR, ADR 0002, that states what the Lean model theorem
establishes, what the trusted Regorus frontend is assumed to do, what OPA and
Regorus differential tests show, and what lies outside the theorem, including
the provenance of request attributes. It also replaces the technical design's
one-sentence list of "open gates" with a short table naming, for each
outstanding prerequisite, the responsible role, the completion evidence, the
roadmap task, and whether E0 needs it or only public release does.

After the change, a reader of
`docs/adrs/adr-0002-first-profile-assurance-boundary.md` and technical design
§13 can answer: "What would a passing run actually prove, what remains trusted,
and what has to happen before E0 and before release?" No new tooling, schema,
or validation code is added.

## Constraints

- Keep the four claims separate (AGENTS.md): admission, model proof,
  source-to-model translation, and runtime correspondence. No wording may imply
  that the frontend or runtime correspondence is proved, or that the theorem
  authenticates attributes or certifies an application.
- No changes to `src/`, `tests/`, Cargo manifests, `lean/`, `examples/`,
  `validation/`, or any JSON contract. The only contract-directory edit is the
  wording fix in `contracts/cli.txt`.
- Record acceptance only after the user states it in conversation. The agent
  then commits an acceptance note to ADR 0002 on their behalf.
- Roadmap dependency edits proposed below take effect only if the user approves
  this plan without striking them.
- British English, Oxford spelling; keep `sha256sum -c SHA256SUMS` passing.

## Tolerances (exception triggers)

- More than 12 files or about 400 changed lines: stop and escalate.
- Any need for new code, schemas, Make targets, or dependencies: stop.
- Any requested statement wording that blurs the four claims: record the
  conflict and ask.

## Risks

- Risk: the ADR overstates what is proved, because no Lean theorem exists yet.
  Mitigation: the ADR states the theorem as the claim a future passing run may
  make, names V1–V6 in technical design §10 as unproved obligations, and says
  so.
- Risk: the E0 classification below conflicts with the current roadmap
  dependencies. Mitigation: the plan proposes four specific roadmap edits for
  approval rather than silently diverging.

## Progress

- [x] (2026-10-15) Reconnaissance, an expert review, and Firecrawl research
  (revisions 1–2).
- [x] (2026-10-15) User rejected revision 2 as over-engineered; rewrote as
  revision 3.
- [x] (2026-10-15) Plan approved by the user, with ADR 0002 accepted as
  written in the same message; the roadmap edits were not struck.
- [x] (2026-10-15) EP-M1 and EP-M2 delivered together, because acceptance
  arrived with plan approval: ADR 0002 written as `accepted` with its
  acceptance note; technical design §§1 and 13; context, README, user's guide,
  `contracts/cli.txt`, contents, and the ToR sponsor row amended; four roadmap
  edits applied; 1.1.1 and 1.1.2 ticked; `SHA256SUMS` refreshed.
- [x] (2026-10-15) Gates and CodeRabbit CLI review run through `scrutineer`
  (see `Outcomes & retrospective`).
- [x] (2026-10-21) Follow-up documentation change: refreshed
  `docs/documentation-style-guide.md` from the canonical
  `leynos/agent-helper-scripts` library edition (commit `c2dfbc7`) and adopted
  its four-digit naming scheme. ADR 0002 moved to
  `docs/adrs/adr-0002-first-profile-assurance-boundary.md`; ADR 0001 moved to
  `docs/adrs/adr-0001-bootstrap-polean-from-the-combobulate-scaffold.md` (the
  slug now derives from its title); every inbound link and numeric reference in
  this plan and in the other documents was updated.

## Surprises & discoveries

- The assurance vocabulary is spelled three ways; `docs/users-guide.md` and
  `contracts/cli.txt` print
  `model_proved + trusted_frontend + tested_agreement`, mixing an outcome name
  with dimension values. EP-M1 fixes the phrase.
- Technical design §1's statement omits attribute provenance, and ToR §2 and
  `docs/context.md` mention only subject and resource attributes, although the
  request also carries `action`. EP-M1 adds a fourth sentence that names all
  three.
- Regorus 0.13.0 was published on 2026-10-14 (crates.io API) with licence
  expression `MIT AND Apache-2.0 AND BSD-3-Clause`, default features `full-opa`,
  `arc`, and `rvm`, and an opt-in `ast` feature. Its `std` feature depends on
  `vstd`. These are inputs to the pins and distribution rows, not decisions. A
  Firecrawl page summary misreported the licence as Apache-2.0, so take such
  facts from registry metadata.
- The current roadmap makes E0 wait on admission coverage (2.1.1 requires
  1.2.2), axiom auditing (2.2.1 requires 1.2.3), and sandbox enforcement (2.3.1
  requires 1.2.4). Several of these are better treated as things E0
  investigates; see `E0 scrutiny`.
- Lean's stock `#print axioms` reports a declaration's axiom dependencies
  (Lean reference, "Axioms"). The foundational set is `propext`,
  `Classical.choice`, and `Quot.sound`; native computation adds
  `Lean.ofReduceBool`, `Lean.ofReduceNat`, or `Lean.trustCompiler`. E0
  therefore needs no bespoke audit tooling to inspect its one theorem.

## Decision log

- Decision: no acceptance ledger, schema, harness module, digest, or comment
  protocol. Rationale: the user rejected revision 2. Git history, the ADR, the
  roadmap, and code review already supply traceability. (User, 2026-10-15.)
- Decision: ratification is a conversation note recorded in ADR 0002 by the
  agent, as with earlier decisions. (User, 2026-10-15.)
- Decision: the prerequisites table lives in technical design §13, replacing
  the "Open gates" sentence. Each row links to the roadmap task that tracks its
  state, so the table does not duplicate status. Rationale: §13 already owns
  the open gates; the roadmap remains the state record.
- Decision: no JSON contract changes. A machine-readable provenance assumption
  belongs with result reporting (roadmap 4.2.1), which must render the
  assurance boundary. ADR 0002 records that obligation.
- Decision: classify prerequisites as E0 needs or public-release needs, and
  amend four roadmap entries accordingly (see `E0 scrutiny`). Approved with the
  plan on 2026-10-15.
- Decision (hosted review, 2026-10-15): recorded in ADR 0002 that `leynos`
  holds both the sponsor and technical-reviewer responsibilities, so the one
  acceptance satisfies roadmap 1.1.1's "sponsor and reviewer" criterion and
  closes ToR OQ2; ADR 0002's decision owners now name both roles. Expanded the
  IR, AST, and OPA acronyms; made the 4.2.1 obligation explicit that the result
  contract must be extended; extended ToR OQ3 to the action attribute;
  corrected the §13 admission and axiom-audit rows to match roadmap 2.3.3's
  dependencies (2.1.2 and 2.2.4) rather than claiming E0 only investigates
  them; and changed the resource row's E0 need to "not needed", because
  measurement is E2 evidence. The statement itself is unchanged. Declined:
  pointing the attribute-provenance row at 1.1.2, because 5.1.1 cites OQ1–OQ4.
- Decision (follow-up assessment, 2026-10-15): corrected technical design §13
  and this plan, which overstated the roadmap edits. The admission spike
  (1.2.2) still gates E0 semantics and supervisor work through 2.1.1; only the
  axiom-audit spike (1.2.3) and the sandbox decision (1.2.4) stopped gating
  them. The approved dependency graph is unchanged.
- Decision: ADR 0002 follows the approved outline. Three small additions,
  reported to the user: boundary 3 states that admission is performed by the
  frontend, that `check` establishes admission only, and that admission is not
  proved (from the four-claim rule); the alternatives carry one sentence of
  rationale each; and the consequences add that release material and user
  documentation quote or link the ADR rather than paraphrasing a stronger
  claim. The ToR sponsor row now reads "acceptance recorded in ADR 0002" rather
  than naming a holder.

## Outcomes & retrospective

Post-review update (2026-10-15): hosted review of `81fe023` led to the
corrections recorded in `Decision log`. The E0 rows now match the approved
roadmap: the axiom-audit spike (1.2.3) no longer gates the start of E0
semantics and supervisor work, the admission spike (1.2.2) still does through
2.1.1, and the E0 command (2.3.3) still depends on fail-closed admission
(2.1.2) and axiom acceptance (2.2.4).

ADR 0002 records the accepted boundary, technical design §1 repeats the
statement word for word, and §13 carries the prerequisites table. The roadmap
no longer makes E0 semantics work wait for the axiom-audit spike or E0
supervisor work wait for the sandbox decision, and it moves enforced isolation
to 4.3.1. No code, schema, or tooling changed.

Lesson: revision 2 let an expert-review panel's safeguards accumulate into a
governance subsystem that no requirement asked for. Filter review findings
through proportionality before adopting them; the assurance distinctions
deserved the effort, and the approval machinery did not.

## Context and orientation

- `docs/technical-design.md` §1 holds the proposed public assurance statement.
  §2 is the trust-boundary table, §6.2 the Lean worker and axiom policy, §10
  the unproved obligations V1–V10, and §12 experiments E0–E2. Before this work,
  §13 ended with the "Open gates" sentence.
- `docs/terms-of-reference.md`: A2 (trusted frontend acceptable for the first
  experiment, resolved when the sponsor accepts or rejects the statement), A3
  (attribute provenance), A5 (frontend can enforce admission), A6 (resource
  envelope), and OQ2–OQ5 and OQ7.
- `docs/adrs/adr-0001-bootstrap-polean-from-the-combobulate-scaffold.md` sets
  the ADR form (Status, Date, and Decision owners bullets; Context, Decision,
  Consequences, and Alternatives considered sections) and records the ISC
  licence.
- `docs/context.md`, `docs/users-guide.md`, `README.md`, and
  `contracts/cli.txt` restate parts of the statement.

Skills to load: `execplans`, `arch-decision-records`, and
`en-gb-oxendict-style`. Delegate full gate runs to `scrutineer`.

## Conformance basis

Upstream at commit `9a867bd`: ToR draft v0.1 (A2, A3, A5, A6, OQ2–OQ5, OQ7,
SC8); technical design draft v0.1 (§§1, 2, 6, 10, 12, 13); ADR 0001.

```plaintext
ToR A2, A3, OQ2, OQ3 -> TDD §1 -> RM-1.1.1 -> EP-M1/M2 -> ADR 0002 (accepted)
TDD §13 gates, ToR OQ4, OQ5, OQ7, SC8 -> RM-1.1.2 -> EP-M1 -> TDD §13 table
```

## Verification plan

This change is documentation. It introduces no executable logic, invariant, or
lemma, so Verus, Kani, property tests, and new unit or behavioural tests are
not warranted. The technical obligations that matter (V1–V10 in technical
design §10) are unchanged. ADR 0002 names them as unproved and leaves them to
roadmap phases 2–4.

Checks for this milestone are the existing gates: `make check-fmt`,
`make markdownlint`, `make lint`, `make test`, `make design-check` (to confirm
that nothing in the design pack moved), and `sha256sum -c SHA256SUMS`. Before
committing, a reviewer confirms by reading:

- the statement in ADR 0002 and technical design §1 is word-for-word identical;
- every table row names an existing roadmap task; and
- no edited sentence attributes proof to the frontend, the runtime, or
  attribute handling.

## ADR 0002 content

Historical: this section is the approved outline. ADR 0002 is now accepted; the
delivered record is authoritative, and `Decision log` lists where it differs.

Title: `ADR 0002: first-profile assurance boundary`. Status: `Proposed` until
accepted. Decision owners: the project sponsor.

The statement for acceptance, identical in technical design §1:

> The admitted policy model satisfies the six registered tenant-write
> requirements for every valid request. Lean checked the model evidence. The
> source frontend remains trusted; OPA and Regorus agree with the model on the
> recorded conformance cases, not by a refinement theorem. The model theorem
> takes the request's subject, resource, and action attributes as given: it
> does not authenticate them, establish their provenance, or certify how an
> application enforces the decision.

The Decision section then states each boundary precisely:

1. **What Lean proves.** For the admitted IR `p` and each of the six claims,
   for every request in the declared domain, the claim holds under Polean's
   pinned semantics. The proof route is the abstraction lemma
   (`evalRequest p r = evalFacts p (α r)`), fact coverage, and kernel-reduced
   finite checking (V2–V4). None of these is proved yet.
2. **What that proof trusts.** The Lean kernel; the foundational axioms
   `propext`, `Classical.choice`, and `Quot.sound` only, with no `sorryAx` and
   no native-computation axioms; the integrity of the pinned proof package; and
   Polean's own Lean definitions of the six atoms. Those definitions are the
   specification of what each Rego comparison means, and they are not proved
   against Rego. This is the assumption most at risk of being smuggled into the
   trusted computing base, so the ADR names it explicitly.
3. **What the trusted frontend assumes.** Regorus parses the source, and
   Polean's Rust adapter maps the AST to IR faithfully and rejects everything
   outside the profile. There is no source-preservation theorem; spans,
   round-trip checks, and differential tests reduce but do not remove this
   trust (ToR A2).
4. **What differential tests establish.** On the recorded conformance cases,
   OPA and Regorus return the model's decision for the original source. They
   prove neither refinement direction. In particular, transferring a safety
   claim needs `RuntimeAllows(source, r) → ModelAllows(ir, r)`, which remains
   unproved (technical design §9).
5. **What lies outside the theorem.** The authenticity and provenance of the
   subject, resource, and action attributes; application enforcement; later
   state changes; and host integrity (ToR A3, technical design §2).
6. **What a run may report.** Outcome `model_proved` with assurance
   `kernel_checked`, `trusted_frontend`, and `tested_agreement`. No test result
   upgrades the latter two, and release material may not claim more than the
   statement.

Consequences: roadmap 4.2.1 must render the boundary, including provenance, in
every report. The three open questions below stay visible until E0 and E1
answer them.

Open questions, recorded honestly rather than gated:

1. Do Polean's Lean atom definitions mean what OPA means for the admitted forms?
   Today, only differential sampling (V1, V10) tests this.
2. Can Regorus's AST support complete fail-closed admission without Polean
   becoming an unverified parser that must be trusted wholesale (A5)?
3. Can the finite check run by kernel reduction alone, with no native
   computation, within the resource envelope (A6), so that proof-search code
   stays outside the trusted computing base?

Alternatives considered: requiring checked source correspondence before v0
(A2's consequence if false); omitting provenance from the statement.

Acceptance: a short dated note, added in EP-M2, recording the user's
conversational acceptance or rejection with any reason.

## Prerequisites table (technical design §13)

Historical: the approved draft. The delivered table in technical design §13 is
authoritative; review corrected its admission, axiom-audit, and resource rows
(see `Decision log`).

Replace the "Open gates" sentence with this table and one sentence saying the
roadmap tasks track state:

| Prerequisite            | Role                   | E0 needs                                                                                        | Public release needs                                                                                   | Roadmap      |
| ----------------------- | ---------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ------------ |
| Toolchain tuple         | maintainer             | A working, recorded Rust, Regorus, Lean, and OPA tuple                                          | Lockfiles, Regorus features, and OPA digest pinned and replayed cleanly                                | 1.2.1, 4.3.2 |
| Admission coverage      | maintainer             | Investigated: a record of what the AST exposes and where gaps are; E0 runs only on own fixtures | Complete fail-closed admission over every AST variant                                                  | 1.2.2, 2.1.2 |
| Axiom audit             | maintainer             | Investigated: `#print axioms` on the E0 theorem shows only the foundational set                 | Automated audit in worker acceptance that rejects `sorryAx`, native-computation, and undeclared axioms | 1.2.3, 2.2.4 |
| Execution environment   | maintainer             | Supervised local runs with a deadline, on trusted repository inputs                             | Enforced offline sandbox with memory and output limits, process-group kill, and private storage        | 1.2.4, 4.3.1 |
| Resource budgets        | maintainer             | Record what E0 measures                                                                         | Measured limits on an agreed machine                                                                   | 4.3.2        |
| Assurance statement     | sponsor                | ADR 0002 accepted                                                                               | Release wording no stronger than ADR 0002                                                              | 1.1.1        |
| Dependency distribution | sponsor                | Not needed                                                                                      | Decision covering dependency licences; Polean's own ISC licence is already settled (ADR 0001)          | 5.2.2        |
| First user review       | sponsor, policy author | Not needed                                                                                      | SC8 proceed, revise, or stop decision                                                                  | 5.1.1        |
| Attribute provenance    | application owner      | Outside the theorem                                                                             | Still outside the theorem; integrator documents ingress (OQ3); not a release blocker                   | 5.1.1        |
| Design transcript (OQ7) | sponsor                | Not needed                                                                                      | Not a blocker; the transcript is already excluded from acceptance evidence (ToR §8.3)                  | none         |

## E0 scrutiny

E0 is an experiment. Its genuine prerequisites are a working toolchain and a
way to run it locally. Admission coverage and axiom auditing are questions E0
should answer, not conditions for asking them. A sandbox matters once inputs
are untrusted, which E0's inputs are not. Proposed roadmap edits, applied in
EP-M2 if approved:

1. 1.2.2: change "Missing API coverage has a demonstrated bounded check or
   blocks E0" to "… or is recorded as an E0 finding that blocks 2.1.2".
2. 2.2.1: change "Requires 2.1.1 and 1.2.3" to "Requires 2.1.1", because Lean
   semantics need a pinned Lean, not audit tooling. 2.2.4 keeps 1.2.3.
3. 2.3.1: change "Requires 1.2.4 and 2.1.1" to "Requires 2.1.1", and change its
   success criterion from "enforced isolation" to a supervised local process
   with one deadline whose children are killed and reaped.
4. 4.3.1: add "Requires 1.2.4" and take over the enforced-isolation criterion.

If the user strikes these edits, the table's E0 column still records the
distinction, and the roadmap keeps its stricter ordering.

## Plan of work

Historical: both milestones are complete; see `Progress`.

EP-M1 (one commit, or two if the ADR and the design edits read better
separately):

1. Write ADR 0002 as above, with Status `Proposed`.
2. In technical design §1, replace the statement with the four-sentence text and
   link ADR 0002 from Companions. In §13, replace the "Open gates" sentence
   with the table.
3. `docs/context.md`: make the closing provenance sentence name the subject,
   resource, and action attributes.
4. `README.md` (lines 30–33) and `docs/users-guide.md`: add the provenance
   clause and link ADR 0002. In the user's guide and `contracts/cli.txt`,
   replace `model_proved + trusted_frontend + tested_agreement` with "outcome
   `model_proved` with assurance `kernel_checked`, `trusted_frontend`, and
   `tested_agreement`".
5. `docs/contents.md`: list ADR 0002.
6. Refresh the `SHA256SUMS` lines for the covered files edited (`README.md`,
   `contracts/cli.txt`, `docs/context.md`, `docs/technical-design.md`, and
   `docs/terms-of-reference.md`).
7. Run the gates; push; ask the user to accept or reject ADR 0002 and the
   roadmap edits.

EP-M2, after the user's answer:

1. Add the acceptance note to ADR 0002 and set Status to `Accepted` or
   `Rejected`.
2. Apply the approved roadmap edits; tick 1.1.1 and 1.1.2. On rejection, leave
   1.1.1 unticked with a one-line note and wait for a revise-or-stop direction.
3. Update the ToR §4 Sponsor row's "individual authority not assigned" to point
   at ADR 0002.
4. Run the gates, update this plan's living sections, and set Status to
   `COMPLETE`.

## Concrete steps

From the worktree root, sequentially, each teed to
`/tmp/<action>-polean-1-1-ratify-assurance-and-release-gates.out`:

```bash
make check-fmt
make markdownlint
make lint
make test
make design-check PYTHON=/tmp/polean-venv/bin/python   # expect no diff
sha256sum -c --quiet SHA256SUMS                         # expect no output
git status --porcelain                                  # clean after commit
```

## Validation and acceptance

Done when ADR 0002 exists with the statement and six boundary paragraphs; the
technical design §1 statement is identical; §13 carries the table; the user's
acceptance (or rejection) is noted in ADR 0002; approved roadmap edits are
applied; and every command above passes.

## Idempotence and recovery

All edits are plain documentation; revert per commit. A wrongly recorded
acceptance is corrected in a new commit quoting the user's correction.

## Interfaces and dependencies

None added or changed.

## Revision notes

- Revision 1 (2026-10-15): initial draft.
- Revision 2 (2026-10-15): expanded after an expert review into a
  machine-readable acceptance ledger with schema, harness checks, digests, and
  a GitHub comment protocol.
- Revision 3 (2026-10-15): the user rejected revision 2 as substantially
  over-engineered. Removed the ledger, schema, harness module, controls,
  digests, comment protocol, staged pull requests, and role ceremonies.
  Retained and sharpened the assurance boundaries in ADR 0002, including the
  trusted Lean atom semantics and three open technical questions. Replaced gate
  machinery with a §13 table that separates E0 needs from public-release needs,
  and proposed four roadmap dependency edits so that E0 investigates admission
  and axiom auditing rather than waiting for them. The remaining work is one
  ADR and small documentation amendments.
- Revision 4 (2026-10-21): documentation-only follow-up after completion. The
  shared style guide was refreshed from its canonical library edition, so the
  ADR paths and numbers moved to `docs/adrs/adr-nnnn-title-slug.md`. No
  assurance statement, roadmap task, contract, or gate changed.
