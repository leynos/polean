# Ratify the first-profile assurance statement and allocate release-gate owners

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: DRAFT

Roadmap item: [1.1. Ratify the assurance and release gates](../roadmap.md),
tasks 1.1.1 and 1.1.2. Branch: `1-1-ratify-assurance-and-release-gates`.

## Purpose / big picture

Polean is a proposed checker that proves properties of a small, closed subset
of Rego authorization policies. Before anyone writes production code, the
project must settle two things that later work depends on.

First, it must decide exactly what a successful Polean run is allowed to claim.
The technical design proposes a three-part answer: Lean checked a theorem about
the policy *model*; the Rego-to-model frontend remains *trusted* rather than
proved; and OPA and Regorus *agree on tested cases* rather than by a refinement
proof. The roadmap also requires the statement to say plainly that the
authenticity and provenance of request attributes lie outside the theorem. A
named sponsor and a named technical reviewer must accept that exact wording or
record a blocking rejection. That decision becomes Architecture Decision Record
(ADR) 002.

Second, the technical design lists seven "open gates" (toolchain pins,
admission coverage, axiom audit, sandbox, budgets, licence and distribution,
sponsor acceptance) and says "assign owners before implementation". Nobody has
done so. This work creates an *acceptance ledger*: a machine-readable record
that names, for every gate, the responsible role, the roles that must accept
closure, the evidence that closes it, the roadmap task expected to produce that
evidence, and whether it blocks the E0 development demonstration or public
acceptance. The ledger also separates the settled fact that Polean's own source
is ISC-licensed from the still-open decision about how Polean may be
distributed with its dependencies.

After this change, a contributor can:

1. read `docs/adr-002-first-profile-assurance-boundary.md` and see the exact
   ratified (or rejected) assurance statement, who decided it, and when;
2. read `contracts/acceptance-ledger.json` and see who owns each gate and what
   evidence will close it;
3. run `make design-check` and see the design harness confirm that the
   statement in the ADR, the technical design, and the ledger are identical,
   that the ledger is internally consistent, and which gates currently block E0
   and public acceptance; and
4. run `make design-ledger` and see the ledger checks reject a set of
   deliberately corrupted ledgers, and see the readiness derivation agree with
   an independent oracle over every possible assignment of gate states.

This work does not implement any checker behaviour, prove any theorem, pin any
tool, or close any gate other than the assurance-statement gate. It changes no
Rust code.

## Constraints

These are hard invariants. Violating one requires escalation, not a workaround.

- Do not fabricate human decisions. The sponsor's and reviewer's acceptance or
  rejection must come from an explicit statement by the user (or by a person
  the user names) in this conversation or in a GitHub comment on the pull
  request. The implementer must never write `accepted` or `rejected` on their
  behalf, never infer acceptance from silence, plan approval, a green CI run,
  or a merged pull request, and never fill a role holder with a guessed name.
- Keep the four assurance claims separate (AGENTS.md): admission, model proof,
  source-to-model translation, and runtime correspondence. No new wording may
  imply that the frontend is proved, that OPA or Regorus are proved to
  correspond to the model, or that the theorem authenticates attributes or
  certifies an application.
- Do not change the six claim identifiers or their definitions, the admitted
  atoms, the request domain, the exit codes, or the outcome names in
  `contracts/`. The only contract changes this plan authorizes are the additive
  `attribute_provenance` assurance field described in
  `Interfaces and dependencies` and the two new ledger files.
- Do not modify `src/`, `tests/`, `Cargo.toml`, `Cargo.lock`, `lean/`, or
  `examples/`. This roadmap item has no Rust or Lean deliverable.
- Do not close, waive, or weaken any gate other than `assurance-statement`. A
  gate whose evidence does not yet exist stays `open`.
- Do not record an owner as a named individual unless the user names that
  person for that role. Roles are the unit of allocation (ToR §9: "Owners below
  denote roles to allocate, not named commitments").
- The Python design harness owns design-fixture consistency only
  (`docs/repository-layout.md` §Ownership boundaries). Its new ledger checks
  must not describe themselves as proof, release certification, or
  authentication of a decision.
- Use British English with Oxford spelling
  (`docs/documentation-style-guide.md`). Use "licence" for the noun.
- Keep `SHA256SUMS` passing: `sha256sum -c SHA256SUMS` must report every line
  `OK` at each commit.
- Run gates sequentially, never in parallel.

## Tolerances (exception triggers)

- Scope: if the change needs more than 25 files or roughly 1,600 net added
  lines (excluding the regenerated `validation/design-validation.json`), stop
  and escalate.
- Rust: if any file under `src/`, `tests/`, or the Cargo manifests must change,
  stop and escalate.
- Contracts: if any contract change beyond the additive `attribute_provenance`
  field and the two ledger files appears necessary, stop and escalate.
- Wording: if the sponsor or reviewer asks for wording that differs from the
  proposed statement, apply it only if it preserves all four claim boundaries;
  otherwise record the conflict and escalate.
- Dependencies: if the harness needs a Python package beyond
  `validation/requirements.txt` (`hypothesis`, `jsonschema`), stop and escalate.
- Iterations: if a gate still fails after three fix attempts on the same
  cause, stop and escalate with the log path.
- Ratification: if no explicit sponsor and reviewer decision has arrived when
  milestones EP-M1 and EP-M2 are complete, set the status to `BLOCKED`, report
  what is needed, and wait. Do not mark roadmap 1.1.1 or 1.1.2 done.
- Rejection: if the decision is a blocking rejection, record it (EP-M3 still
  completes), set the status to `BLOCKED` for a revise-or-stop direction, and
  do not begin redesign work.

## Risks

- Risk: the same person holds the sponsor and technical-reviewer roles, so the
  "two-party" acceptance is one person's decision. Severity: medium.
  Likelihood: high (the ToR calls Polean a founder-led experiment). Mitigation:
  permit it, but require the ADR's ratification record to say so explicitly;
  the validator reports `independent_review: false` in the summary when both
  holders are equal.
- Risk: the ledger becomes a second copy of the technical design that drifts.
  Severity: medium. Likelihood: medium. Mitigation: the ledger records only
  ownership, closure evidence, and status; the harness binds the statement text
  to the ADR and technical design; technical design §13 points at the ledger
  instead of restating owners.
- Risk: Markdown extraction of the statement from the ADR and the technical
  design is brittle (wrapping, `>` prefixes). Severity: low. Likelihood:
  medium. Mitigation: delimit the statement with HTML comment markers and
  compare after a defined whitespace normalization; test the normalization with
  explicit cases.
- Risk: the harness grows further beyond a comfortable module size
  (`validate_design.py` is already 1,123 lines). Severity: low. Likelihood:
  high if checks are inlined. Mitigation: put ledger logic in a new module,
  `validation/acceptance_ledger.py`, and call it from `run_checks`.
- Risk: the readiness check becomes vacuous because the oracle restates the
  implementation. Severity: medium. Likelihood: medium. Mitigation: write the
  oracle in a different form (counting rather than set comprehension) and kill
  a seeded mutant that treats `rejected` as satisfied.
- Risk: an evidence URL can point anywhere and so "close" a gate with
  unrelated material. Severity: medium. Likelihood: low. Mitigation: accept
  only repository paths that exist or URLs under
  `https://github.com/leynos/polean/`; the residual gap (a URL to an irrelevant
  comment) is recorded in the verification plan.
- Risk: Firecrawl could not be used for external research because the account
  has no credits. Severity: low. Likelihood: certain. Mitigation: dependency
  licence facts were taken from the GitHub API instead (see
  `Surprises & discoveries`); they inform the distribution gate's description
  only and are not a distribution decision.

## Progress

- [x] (2026-10-15 14:10Z) Renamed branch to
  `1-1-ratify-assurance-and-release-gates`.
- [x] (2026-10-15 14:30Z) Reconnaissance with a Wyvern team: build and gate
  machinery, open gates and owners, assurance wording sources.
- [x] (2026-10-15 14:40Z) External fact check of dependency licences via the
  GitHub API (Firecrawl unavailable).
- [x] (2026-10-15 15:00Z) Drafted this ExecPlan.
- [ ] Community-of-experts design review and revision of this plan.
- [ ] Plan approval by the user.
- [ ] EP-M0: baseline gates recorded.
- [ ] EP-M1: assurance statement bound across ADR-002 (proposed), technical
  design, ledger, and result contract.
- [ ] EP-M2: release gates allocated in the acceptance ledger with closure
  evidence and derived blockers.
- [ ] EP-M3: sponsor and reviewer decision recorded.
- [ ] EP-M4: roadmap updated, conformance reconciled, retrospective written.

## Surprises & discoveries

- Observation: the assurance vocabulary is spelled three ways. Evidence:
  `contracts/profile.json` uses prose (`"trusted frontend"`,
  `"differential tests, not proof"`); `contracts/result.schema.json` uses enums
  (`kernel_checked`, `trusted_frontend`, `tested_agreement`);
  `docs/users-guide.md` and `contracts/cli.txt` print
  `model_proved + trusted_frontend + tested_agreement`, which mixes an outcome
  name (`model_proved`) with two assurance-dimension values. Impact: EP-M1
  fixes the mixed phrase to name the outcome and the three dimension values
  separately.
- Observation: the canonical statement in technical design §1 does not mention
  attribute provenance; that exclusion appears only in §2's table, §4.2, ToR
  A3, and the vocabulary. Impact: the roadmap success criterion requires it in
  the ratified wording, so EP-M1 proposes a fourth sentence.
- Observation: `SHA256SUMS` covers `README.md`, `contracts/*`, three design
  documents, `examples/`, and `validation/`, and currently verifies cleanly,
  but nothing in CI checks it. Impact: each milestone refreshes the affected
  lines and runs `sha256sum -c SHA256SUMS`.
- Observation: Regorus has moved on from the inspected 0.12.0. Evidence:
  GitHub API on 2026-10-15 reports latest release `regorus-v0.13.0` and
  `Cargo.toml` `license = "MIT AND Apache-2.0 AND BSD-3-Clause"`; OPA's latest
  release is `v1.21.2` (Apache-2.0); Lean 4's is `v4.35.0` (Apache-2.0).
  Impact: none for this plan; recorded in the `toolchain-pins` and
  `dependency-distribution` gate descriptions as inputs for roadmap 1.2.1 and
  5.2.2, not as decisions.
- Observation: Firecrawl returned "Insufficient credits" for every request.
  Impact: external facts came from `gh api`; prior-art comparisons (ADR
  formats, assurance-case notation) rely on the repository's own conventions.

## Decision log

- Decision: record the acceptance ledger as a versioned JSON contract,
  `contracts/acceptance-ledger.json`, with a JSON Schema, rather than as a
  Markdown table. Rationale: later roadmap tasks (1.2.1–1.2.4, 4.3.2, 5.1.1,
  5.2.2) close these gates, and AGENTS.md requires machine-readable contracts
  to change with trust boundaries; a contract lets the existing design lane
  reject an incomplete or self-contradictory ledger, which a table cannot.
  Alternative considered: a Markdown table in the technical design; rejected
  because nothing would stop a gate being marked closed without evidence.
  Date/Author: 2026-10-15, planning agent.
- Decision: the ledger never stores whether E0 or public acceptance is ready.
  The harness derives the blocking set from gate states and publishes it in
  `validation/design-validation.json`. Rationale: mirrors technical design §8,
  "A malformed report cannot choose its own exit status by setting a JSON
  field." Date/Author: 2026-10-15, planning agent.
- Decision: the harness holds the gate identifiers and role identifiers as
  reviewed constants, as it already does for claim identifiers. Rationale:
  deleting an inconvenient gate from the JSON must fail the check, not shrink
  the requirement. Date/Author: 2026-10-15, planning agent.
- Decision: add a required `attribute_provenance` assurance dimension with the
  single value `assumed` to `contracts/result.schema.json`, and a matching
  prose entry to `contracts/profile.json`. Rationale: the ratified statement
  places provenance outside the theorem; a report that omits that fact would
  let a reader assume otherwise. The field is additive, pre-1.0, has no
  implemented producer or consumer, and holds whether the sponsor accepts or
  rejects A2, because A3 is independent of A2. Compatibility decision: none
  required (no released API, no consumer). Date/Author: 2026-10-15, planning
  agent; subject to expert review.
- Decision: implement ledger checks in Python inside the existing design lane,
  not in Rust. Rationale: phase 1 must "preserve … independent design
  validation rather than rebuilding the scaffold"; the Rust crate is a
  disposable scaffold with no dependencies, and adding `serde_json` and a JSON
  Schema crate to it for a governance document would create the first Rust
  dependency for a non-product reason. Consequence: `rstest`, `rstest-bdd`,
  `googletest`, `pretty_assertions`, `insta`, `proptest`, Kani, and Verus are
  not used; see `Verification plan` for the justification and substitutes.
  Date/Author: 2026-10-15, planning agent.
- Decision: the new harness files follow the sibling scripts' conventions
  (`#!/usr/bin/env python3`, `validation/requirements.txt`, plain `main()`
  returning an exit code) rather than `docs/scripting-standards.md`'s `uv`, PEP
  723, Cyclopts, and pytest defaults. Rationale: the design workflow installs
  `requirements.txt` with `pip` and runs plain scripts; one lane with two
  conventions is worse than a documented local exception. Date/Author:
  2026-10-15, planning agent; subject to expert review.
- Decision: ledger gates are the seven technical-design §13 gates plus
  `attribute-provenance` (ToR OQ3, A3), `design-transcript-provenance` (ToR
  OQ7), and `first-user-review` (ToR SC8, OQ1; technical design §12). OQ6
  (bidirectional authoring) is not a release gate; roadmap 6.1.1 owns it.
  Rationale: the roadmap names "unresolved provenance", which covers both OQ3
  and OQ7, and §12 makes the first user review an exit gate. Date/Author:
  2026-10-15, planning agent.
- Decision: `attribute-provenance` and `design-transcript-provenance` do not
  block public acceptance; they carry `blocks: []` and remain visible residual
  assumptions. Rationale: the ratified statement already excludes attribute
  provenance from the theorem, so closing OQ3 changes what an integrator knows,
  not what Polean claims; ToR §8.3 already excludes the unseen transcript from
  acceptance evidence. The sponsor may override this at ratification.
  Date/Author: 2026-10-15, planning agent; explicitly offered to the sponsor.

## Outcomes & retrospective

Not started. Complete at EP-M4.

## Context and orientation

Polean is at a design-scaffold stage. Nothing here parses Rego, runs Lean, or
calls OPA. The repository contains:

- `docs/terms-of-reference.md` (ToR): the problem brief. §4 defines the roles
  (policy author, technical reviewer, project maintainer, sponsor, production
  enforcement operator). §8.2 lists assumptions A1–A6; A2 says the frontend may
  stay trusted and is resolved when the "sponsor accepts the assurance
  statement or rejects it before release"; A3 says request attributes come from
  trusted application components. §9 lists open questions OQ1–OQ7 with proposed
  owner roles.
- `docs/technical-design.md` (TDD): §1 holds the proposed first public
  assurance statement as a block quote; §2 is the trust-boundary table; §8
  defines outcomes such as `model_proved`; §12 defines experiments E0–E2 and
  the first user review; §13 ends with the "Open gates" sentence this plan
  allocates.
- `docs/context.md`: the vocabulary (model theorem, source binding, runtime
  correspondence, kernel checked, trusted computing base).
- `docs/adr-001-repository-bootstrap.md`: the only ADR. Header bullets
  `Status`, `Date`, `Decision owners`; sections Context, Decision,
  Consequences, Alternatives considered. It records the ISC licence and defers
  toolchain pins to E0.
- `contracts/`: JSON Schemas and the profile. `result.schema.json` has an
  `assurance` object with `logic`, `source_binding`, and
  `runtime_correspondence` enums. `profile.json` has prose `assurance` values
  constrained by `profile.schema.json`.
- `validation/validate_design.py`: the design harness. `Repository` is the
  read-only input boundary; `Workspace` is the only writer; `run_checks()` is
  the composition root and writes `validation/design-validation.json`.
  `load_schemas()` loads every `contracts/*.schema.json`, so a new schema is
  meta-validated automatically and `json_schemas_checked` rises from 5 to 6.
- `validation/test_boundary.py`: fault-injection tests of the harness, run by
  `make design-boundary`, using a `Tampered` repository subclass that rewrites
  one document in memory.
- `Makefile`: `check-fmt`, `typecheck`, `lint`, `test`, `markdownlint`,
  `design-check`, `design-properties`, `design-boundary`, and `all`.
- `.github/workflows/design.yml`: runs the three design targets and fails if
  the tree is dirty afterwards (so a regenerated summary must be committed).
- `SHA256SUMS`: a manual provenance manifest; verify with
  `sha256sum -c SHA256SUMS`.

Terms used in this plan:

- *Assurance statement*: the sentences a successful run is permitted to
  claim. *Ratified* means the sponsor and technical reviewer accepted it.
- *Gate*: a decision or piece of evidence that must exist before a named stage
  (E0 or public acceptance) may proceed.
- *Acceptance ledger*: the contract listing every gate with its owner roles,
  closure evidence, status, and the stages it blocks.
- *Blocker*: a gate that blocks a stage and is not `closed`.

Required references for the implementer, in reading order:
`docs/terms-of-reference.md` (§§1–5, 8–10), `docs/technical-design.md` (§§1–3,
8–9, 12–13), `docs/context.md`, `docs/adr-001-repository-bootstrap.md`,
`docs/roadmap.md` (phase 1 and 5.2.2), `docs/documentation-style-guide.md`,
`docs/developers-guide.md`, `docs/repository-layout.md`,
`validation/README.md`, and `docs/scripting-standards.md` (for the documented
exception). `docs/reliable-testing-in-rust-via-dependency-injection.md`,
`docs/rust-testing-with-rstest-fixtures.md`, `docs/rust-doctest-dry-guide.md`,
and `docs/complexity-antipatterns-and-refactoring-strategies.md` do not apply
to code changed here, because no Rust changes; read the last one before
splitting harness functions. `docs/references.md` does not exist in this
repository; external sources are listed per document.

Skills to load: `execplans` (maintaining this plan), `arch-decision-records`
(ADR-002 shape), `en-gb-oxendict-style` (prose), `python-router` then
`python-data-shapes` (ledger data handling), `rust-router` and
`rust-verification` (only to confirm that no Rust or Verus work is warranted),
`commit-message`, and `pr-creation`. Delegate full gate runs to the
`scrutineer` agent.

## Conformance basis

Upstream artefacts at commit `9a867bd`:

- Terms of reference, draft v0.1 (2026-10-04): A2, A3, A4, A5, A6, OQ1–OQ7,
  SC7, SC8, §4 roles, §8.1 ("No … licensing requirement … has been
  authorized"), §8.3 (dependency tuple).
- Technical design, draft v0.1 (2026-10-04): §1 statement (TDD-STMT), §2
  "Application to request" row (TDD-BND-APP), §8 `model_proved` row (TDD-OUT),
  §10 trust-ledger sentence (TDD-V), §12 exit gates (TDD-EXIT), §13 open gates
  (TDD-GATES).
- ADR-001 (accepted, 2026-10-08): ISC licence; toolchain pin deferred to E0.
- Roadmap tasks RM-1.1.1 and RM-1.1.2; downstream consumers RM-1.2.1–1.2.4,
  RM-4.3.2, RM-5.1.1, RM-5.2.2.
- Governing standards: AGENTS.md, `docs/documentation-style-guide.md`,
  `docs/repository-layout.md` §Ownership boundaries.

Trace links:

```plaintext
TOR-A2, TOR-OQ2 -> TDD-STMT -> RM-1.1.1 -> EP-M1, EP-M3
  -> ADR-002 ratification record; gate assurance-statement; checks O1, O6
TOR-A3, TOR-OQ3 -> TDD-BND-APP -> RM-1.1.1 -> EP-M1
  -> fourth statement sentence; attribute_provenance=assumed; check O2
TDD-OUT -> EP-M1 -> users-guide and cli.txt wording fix; check O2
TDD-GATES, TOR-OQ4, TOR-OQ5, TOR-OQ7, TOR-SC8 -> RM-1.1.2 -> EP-M2 -> ledger gates; checks O3, O4, O7
ADR-001 ISC, TOR-OQ5 -> RM-1.1.2 -> EP-M2 -> ledger settled repository-licence vs gate dependency-distribution; check O5
TDD-EXIT -> RM-1.1.2 -> EP-M2 -> ledger blocks e0/public-acceptance; derived blockers in design-validation.json; check O7
```

No requirement is dropped. ToR OQ6 is traced to RM-6.1.1 and deliberately
excluded from the ledger (see `Decision log`).

## Verification plan

This change introduces data contracts and consistency rules over documents, not
executable product logic. The invariants below are finite and document-shaped.
The right tools are the existing harness's named checks, fault-injection
controls in the `test_boundary.py` style, and one exhaustive enumeration over a
finite state space.

Verus is not warranted. There is no Rust function, no unbounded data structure,
and no contractual business logic in a compiled component. The one quantified
property (readiness derivation) ranges over 3^10 = 59,049 gate-state
assignments, which can be enumerated exhaustively in well under a second, so a
deductive proof would add trusted machinery without adding coverage. Kani and
`proptest` are not used for the same reason: there is no Rust code, and the
finite domain is enumerated completely rather than sampled. `rstest-bdd` is not
used because there is no Rust behaviour to specify; the observable workflow is
`make design-check` and `make design-ledger`, whose expected outputs are given
in `Validation and acceptance`. `insta` snapshots are not used because the only
multi-format output is the committed JSON summary, which the design workflow
already pins by requiring a clean tree.

Axioms relied on, not verified:

- AX1: `jsonschema` 4.26.0 implements Draft 2020-12 validation correctly.
- AX2: a statement recorded as a sponsor's or reviewer's decision was in fact
  made by the person holding that role. The harness checks the *form* of the
  ratification record (both roles named, dated, evidence present); it cannot
  authenticate a person. The evidence link (a PR comment by that GitHub
  account, or a quoted conversation statement committed in the ADR) is the
  audit trail.
- AX3: an evidence path that exists, or a URL under
  `https://github.com/leynos/polean/`, is relevant to the gate it closes.
  Relevance is a reviewer's judgement; the harness checks existence and
  namespace only.

Obligations:

- O1 statement binding. Obligation: the text between
  `<!-- polean:assurance-statement:start -->` and
  `<!-- polean:assurance-statement:end -->` in ADR-002 and in technical design
  §1, after removing block-quote markers and collapsing runs of whitespace to
  one space, equals `assurance_statement.text` in the ledger, and each marker
  pair occurs exactly once per document. Method: named harness check
  `check_statement_binding`, plus fault-injection tests. Artefact:
  `validation/acceptance_ledger.py`, `validation/test_acceptance_ledger.py`.
  Evidence: `make design-check` passes; `make design-ledger` reports the
  controls rejected. Non-vacuity: the witness is the real ADR and TDD; controls
  change one word in the TDD copy, delete the end marker, and duplicate the
  block, and each must be rejected with a diagnostic naming the document. A
  normalization test shows that rewrapping the same words is accepted, so the
  check is not merely byte equality that would fail on reflow.
- O2 dimension vocabulary. Obligation: the ledger's `dimensions` are exactly
  `logic: kernel_checked`, `source_binding: trusted_frontend`,
  `runtime_correspondence: tested_agreement`, `attribute_provenance: assumed`;
  each value is a member of the corresponding enum in
  `contracts/result.schema.json`; `attribute_provenance` is required there with
  the single value `assumed`. Method: harness check plus controls. Non-vacuity:
  controls remove `attribute_provenance` from the schema's `required` list,
  change the ledger's `source_binding` to `proved_translation`, and add a
  second enum value `authenticated`; each is rejected.
- O3 closed vocabularies. Obligation: the ledger's role identifiers equal the
  reviewed constant `LEDGER_ROLES` and its gate identifiers equal
  `LEDGER_GATES`, with no duplicates; every `responsible` and `accepted_by`
  role is a declared role. Method: harness check plus controls. Non-vacuity:
  controls delete `execution-sandbox`, duplicate `toolchain-pins`, and set a
  gate's `responsible` to `release-manager`; each is rejected.
- O4 closure evidence. Obligation: a gate whose `status` is `closed` or
  `rejected` has a `decided_on` date and at least one evidence entry; every
  `path` entry names an existing repository file; every `url` entry starts with
  `https://github.com/leynos/polean/`; an `open` gate has no `decided_on`.
  Method: harness check plus controls. Non-vacuity: the `assurance-statement`
  gate after EP-M3 is the positive witness; controls close `axiom-audit` with
  no evidence, cite `docs/missing.md`, and cite `https://example.com/`; each is
  rejected.
- O5 licence distinction. Obligation: the ledger's `settled.repository_licence`
  has SPDX identifier `ISC`, which equals `Cargo.toml`'s `license` value and
  matches the first line of `LICENSE` (`ISC License`); no gate is named for the
  repository licence; the `dependency-distribution` gate cannot be closed by
  evidence consisting only of `LICENSE`, `Cargo.toml`, or ADR-001. Method:
  harness check plus controls. Non-vacuity: controls change the ledger SPDX to
  `MIT`, and close `dependency-distribution` citing only `LICENSE`; each is
  rejected.
- O6 ratification consistency. Obligation: `assurance_statement.decision` is
  `pending`, `accepted`, or `rejected`; the `assurance-statement` gate status
  is respectively `open`, `closed`, or `rejected`; a non-pending decision lists
  exactly one `sponsor` and one `technical-reviewer` entry, each with a
  non-empty `holder` and an ISO date; a pending decision lists none; and
  ADR-002's `Status` bullet begins with `Proposed`, `Accepted`, or `Rejected`
  to match. Method: harness check plus controls. Non-vacuity: the committed
  ledger is the `pending` witness before EP-M3 and the decided witness after;
  controls set `accepted` with only a sponsor entry, set `accepted` while the
  ADR says `Proposed`, and close the gate while the decision is `pending`; each
  is rejected.
- O7 readiness derivation. Obligation: for each stage `s` in
  `{e0, public-acceptance}`, `blockers(s)` is exactly the set of gates `g` with
  `s` in `g.blocks` and `g.status` not `closed`; `rejected` never satisfies a
  gate; closing a gate never adds a blocker, and reopening one never removes a
  blocker. Method: exhaustive enumeration over every assignment of
  `{open, closed, rejected}` to the ten gates (59,049 states), comparing
  `blockers` with an independently written counting oracle, plus a monotonicity
  check over every single-gate transition from each state. Artefact:
  `validation/test_acceptance_ledger.py`. Evidence: the test prints the number
  of states and transitions checked; the expected counts are 59,049 states and
  59,049 × 10 × 2 = 1,180,980 single-gate changes to another status.
  Non-vacuity: the enumeration includes the all-closed state (empty blockers)
  and the all-open state (every blocking gate listed), asserted explicitly; a
  seeded mutant `blockers` that treats `rejected` as satisfied, and another
  that ignores `blocks`, must each be reported as a mismatch by the oracle
  comparison.
- O8 no self-declared readiness. Obligation: the ledger schema has
  `additionalProperties: false` at every object level, so a ledger cannot add
  `public_acceptance: "ready"` or similar. Method: schema control. Non-vacuity:
  a control adds that property and must be rejected by schema validation.

Every obligation above is checked against the real committed documents by
`make design-check`, and every control runs in `make design-ledger`. The
residual gap is AX2 and AX3: the harness proves the ledger is well formed and
consistent with the documents, not that a decision is authentic or that cited
evidence is sufficient.

## Plan of work

### Stage A: confirm the baseline (EP-M0)

No edits. Run every gate once, sequentially, through `scrutineer`, and record
the log paths in `Artefacts and notes`. If any gate fails on the untouched
branch, stop and report; do not fix unrelated failures inside this plan.

### Stage B and C for EP-M1: bind the assurance statement

Red first. Create `validation/acceptance_ledger.py` with the constants
`LEDGER_ROLES`, `LEDGER_GATES`, `STATEMENT_MARKERS`, `EXPECTED_DIMENSIONS`, and
functions `normalize_statement(text) -> str`,
`extract_statement(markdown, document) -> str`, `check_statement_binding(...)`
and `check_dimensions(...)`. Wire a single call,
`acceptance_ledger.check_ledger(repo, validators)`, into
`validate_design.run_checks()` immediately after `check_result_schema`, and add
`contracts/acceptance-ledger.json` to the documents read by `Fixtures.load()`
or by `check_ledger` through `Repository.read_json` (prefer passing `repo` so
the module reads only through the boundary). Run `make design-check`; it must
fail with a `HarnessIOError` naming `contracts/acceptance-ledger.json`. Create
`validation/test_acceptance_ledger.py` with the O1 and O2 controls and the
normalization cases; it must fail because the module's checks cannot find the
ledger.

Green. Write `contracts/acceptance-ledger.schema.json` and a first
`contracts/acceptance-ledger.json` holding `schema_version`, `profile`, `roles`,
`assurance_statement` (decision `pending`, `decided_by: []`), and `settled`,
and an empty `gates` array. The schema never constrains the number of gates;
the closed gate set is a harness constant enforced by check O3, which EP-M2
adds in its red step. EP-M1 therefore needs no temporary permissiveness. (If
this sequencing proves awkward, land EP-M1 and EP-M2 as one milestone rather
than add a shim; record the choice in `Decision log`.)

Write `docs/adr-002-first-profile-assurance-boundary.md` with header bullets
`Status: Proposed — awaiting sponsor and technical-reviewer ratification.`,
`Date`, and `Decision owners: sponsor and technical reviewer`, and the sections
Context, Decision (including a one-paragraph Y-statement summary and the marked
statement), Consequences, Alternatives considered, and Ratification record
(empty, with the instructions for EP-M3). The proposed statement is:

```plaintext
The admitted policy model satisfies these six requirements for every valid
request. Lean checked the model evidence. The source frontend remains trusted;
OPA and Regorus agree on the recorded conformance cases, not by a refinement
theorem. The theorem takes subject and resource attributes as given: it does
not authenticate them, establish their provenance, or certify how an
application enforces the decision.
```

The ADR's Decision section must also state the machine mapping: a successful
`prove` or `verify` reports outcome `model_proved` with assurance
`logic: kernel_checked`, `source_binding: trusted_frontend`,
`runtime_correspondence: tested_agreement`, `attribute_provenance: assumed`; no
test result may upgrade `trusted_frontend` or `tested_agreement`; and the
statement is the most that release material may claim. Alternatives considered
must include "require checked source correspondence before v0" (ToR A2's
consequence-if-false) and "omit provenance from the statement", each with the
reason it was not proposed.

Update the TDD §1 block quote to the same text between the markers, and add
ADR-002 and the ledger to its Companions list. Update `README.md` lines 30–33
to add the provenance sentence in prose. Replace "model_proved +
trusted_frontend + tested_agreement" in `docs/users-guide.md` and
`contracts/cli.txt` with wording that names outcome `model_proved` and the four
assurance values separately. Add `attribute_provenance` (enum `["assumed"]`,
required) to `contracts/result.schema.json`'s `assurance` object, update
`check_result_schema`'s sample report to include it, add
`"attribute_provenance": "assumed; outside the theorem"` to
`contracts/profile.json` with a matching `const` in
`contracts/profile.schema.json`, and add one `require` line to `check_profile`.
Add the vocabulary entries *Attribute provenance*, *Assurance statement*, and
*Acceptance ledger* to `docs/context.md`. Link ADR-002 from `docs/contents.md`.
Run `make design-check` (regenerates `validation/design-validation.json`;
review the diff) and the controls.

Refactor. Keep each new Python function under roughly 40 lines; keep
`acceptance_ledger.py` free of writes. Refresh `SHA256SUMS` lines for every
changed covered file and add the two new contract files and two new validation
files.

### Stage B and C for EP-M2: allocate the gates

Red. Add the O3–O5, O7, and O8 checks and controls, and the exhaustive
readiness enumeration with its two seeded mutants, to
`validation/test_acceptance_ledger.py`. Add a `make design-ledger` target
(`$(PYTHON) validation/test_acceptance_ledger.py`), add it to `all`, and add a
step to `.github/workflows/design.yml` after "Test the harness boundaries". Run
`make design-check`: it must fail with "Ledger gates mismatch" because the
ledger has no gates.

Green. Populate `gates` with the ten entries in `Interfaces and dependencies`,
all `open`. Implement `derive_blockers(ledger) -> dict[str, list[str]]` and
publish its result in the summary as `"acceptance_blockers"`, alongside
`"acceptance_gates": 10` and `"independent_review": null` (null while pending).
Update TDD §13: keep the decision table, replace the "Open gates" sentence's
"Assign owners before implementation" with a pointer to the ledger as the
authoritative record of owners, closure evidence, and status. Add one sentence
to ToR §9 that the ledger allocates these roles to gates, without editing the
OQ table. Document the ledger and the "how to close a gate" procedure in
`docs/developers-guide.md` (new subsection under "The validation harness" and a
`make design-ledger` row in the targets table), add the ledger to the
`contracts/` row of `docs/repository-layout.md`, add the new checks to the
`validation/README.md` table, and record the scripting-standards exception in
`docs/developers-guide.md`.

Refactor. Re-run the controls and the full gate sequence. Refresh `SHA256SUMS`.

### EP-M3: record the decision (human gate)

Stop and ask the user, in one message, for: the GitHub handle or name holding
the sponsor role; the one holding the technical-reviewer role (stating that the
same person may hold both, and that the ADR will say so); each holder's
decision on the exact statement (accept, or reject with reason); and whether
they accept the proposed non-blocking treatment of `attribute-provenance` and
`design-transcript-provenance`. Point them at the ADR on the draft PR so they
can answer there instead.

When explicit answers arrive, update ADR-002's Status bullet and Ratification
record (role, holder, decision, date, and evidence: either the PR comment URL
or a verbatim quotation of the conversation statement with its date), set
`assurance_statement.decision`, `decided_by`, the gate's `status`,
`decided_on`, and `evidence`. If both holders are the same person, say so in
the ratification record. If the ToR §4 "individual authority not assigned" note
is superseded, update it to point at the ADR. Run the gates; the summary now
shows the `assurance-statement` gate absent from both blocker lists if
accepted, or present as `rejected` if rejected.

### EP-M4: close out

Mark roadmap 1.1.1 and 1.1.2 done (`- [x]`) only if EP-M3 recorded a decision.
Reconcile discoveries with the conformance basis, complete
`Outcomes & retrospective`, set Status to `COMPLETE` (or leave `BLOCKED` after
a rejection), run the full gate sequence, and refresh `SHA256SUMS`.

## Milestones and plateaus

- EP-M0. Outcome: baseline evidence that every gate passes on the untouched
  branch. Requirements: none. Acceptance: logs under `/tmp` cited in
  `Artefacts and notes`. Conformance check: no edits. Recovery: none needed.
  Remaining gaps: everything. Compatibility decision: none.
- EP-M1. Outcome: a proposed ADR-002 whose statement is byte-for-byte (after
  normalization) the statement in TDD §1 and in the ledger, with the result and
  profile contracts carrying `attribute_provenance`. Requirements: TOR-A2,
  TOR-A3, TOR-OQ2 (advanced, not closed), TDD-STMT, TDD-OUT. Acceptance:
  `make design-check` passes and reports 6 schemas; the O1 and O2 controls are
  rejected in `make design-ledger` (or the boundary script if the target lands
  in EP-M2). Conformance check:
  - TDD §1 changed only by the added provenance sentence and markers.
  - Four claim boundaries intact; no wording promotes tests into proof.
  - A2 and A3 unchanged; A3 now reflected in the statement.
  - Only the authorized additive contract field changed.
  - No trust boundary moved; provenance was already outside the theorem.
  - Trace links above still current.
  Recovery: revert the milestone commit; nothing downstream depends on it.
  Remaining gaps: gates, ratification. Compatibility decision: none (no
  released API or consumer of the result schema).
- EP-M2. Outcome: a complete, open-gated ledger whose derived blockers appear
  in the committed summary. Requirements: TDD-GATES, TOR-OQ3–OQ5, TOR-OQ7,
  TOR-SC8, TDD-EXIT, ADR-001 licence. Acceptance: `make design-check` publishes
  `acceptance_blockers` listing nine E0-or-public blockers as in
  `Validation and acceptance`; `make design-ledger` passes all controls and
  prints the enumeration counts. Conformance check:
  - Every §13 gate and each named roadmap category has a responsible role.
  - The ISC licence is a settled fact; distribution is an open gate.
  - No gate is closed.
  - New Make target and CI step are the only build changes.
  - Trace links current.
  Recovery: revert the milestone commit. Remaining gaps: ratification.
  Compatibility decision: none.
- EP-M3. Outcome: ADR-002 accepted or rejected with named holders and
  evidence. Requirements: RM-1.1.1 success criterion; TOR-OQ2 closed or
  blocking-rejected. Acceptance: O6 passes on the decided ledger; the
  ratification record cites evidence. Conformance check: decision text matches
  the user's statement verbatim; no other gate touched. Recovery: if a decision
  was transcribed wrongly, correct it in a new commit that cites the correcting
  statement; never rewrite the evidence. Compatibility decision: none.
- EP-M4. Outcome: roadmap items marked done and the plan closed.
  Requirements: RM-1.1.1, RM-1.1.2. Acceptance: roadmap shows `[x]` for both;
  every gate passes. Compatibility decision: none.

## Concrete steps

Run everything from the worktree root,
`/home/leynos/.lody/repos/github---leynos---polean/worktrees/dfbd7024-6cdc-4f29-b754-2e4e27df7780`.
Prepare the Python environment once (the design lane needs `jsonschema` and
`hypothesis`):

```bash
uv venv /tmp/polean-venv
uv pip install --python /tmp/polean-venv/bin/python -r validation/requirements.txt
```

Gate sequence (delegate to `scrutineer`; never run two at once), each teed to
`/tmp/$ACTION-polean-1-1-ratify-assurance-and-release-gates.out`:

```bash
make check-fmt
make markdownlint
make typecheck
make lint
make test
make design-check PYTHON=/tmp/polean-venv/bin/python
make design-properties PYTHON=/tmp/polean-venv/bin/python
make design-boundary PYTHON=/tmp/polean-venv/bin/python
make design-ledger PYTHON=/tmp/polean-venv/bin/python   # from EP-M2
sha256sum -c SHA256SUMS
git status --porcelain   # must be empty after committing the regenerated summary
```

Refresh a checksum line after editing a covered file:

```bash
sha256sum contracts/result.schema.json   # paste the new line over the old one
```

Expected focused outputs:

```plaintext
$ make design-check PYTHON=/tmp/polean-venv/bin/python   # EP-M1 red
design validation I/O failure: contracts/acceptance-ledger.json: ...
$ make design-check PYTHON=/tmp/polean-venv/bin/python   # EP-M2 red
design validation check failed: Ledger gates mismatch
$ make design-ledger PYTHON=/tmp/polean-venv/bin/python  # EP-M2 green
... 
readiness: 59049 states, 1180980 transitions, 2 seeded mutants rejected
acceptance ledger controls: all <N> rejected as intended
```

Commit after each milestone (and after each red and green step where the tree
is coherent), with imperative subjects such as "Bind the first-profile
assurance statement across ADR and design" and a body stating the assurance
impact.

## Validation and acceptance

Behavioural acceptance after EP-M2:

- `make design-check` exits 0 and its summary contains
  `"json_schemas_checked": 6`, `"acceptance_gates": 10`, and an
  `"acceptance_blockers"` object whose `e0` list is
  `["toolchain-pins", "admission-coverage", "axiom-audit"]` and whose
  `public-acceptance` list contains, in ledger order, `toolchain-pins`,
  `admission-coverage`, `axiom-audit`, `execution-sandbox`, `resource-budgets`,
  `dependency-distribution`, `assurance-statement`, and `first-user-review`.
- `make design-ledger` exits 0, prints the enumeration counts above, and
  lists every control as rejected for its intended reason.
- Editing one word of the statement in `docs/technical-design.md` makes
  `make design-check` exit 1 with a diagnostic naming that file.

After EP-M3 (accepted): `assurance-statement` disappears from
`public-acceptance`; ADR-002's Status reads `Accepted`; the summary's
`independent_review` is `true` or `false` according to the holders. After EP-M3
(rejected): the gate remains in `public-acceptance`, its status is `rejected`,
and the plan Status is `BLOCKED`.

Red-Green-Refactor evidence to record: the EP-M1 red I/O failure, the EP-M2 red
"Ledger gates mismatch", each green run, and each refactor re-run.

Quality criteria: all gates in `Concrete steps` pass; O1–O8 discharged as
described; `sha256sum -c SHA256SUMS` reports only `OK`; Markdown passes
`markdownlint-cli2` and `mdtablefix --check`. Security: no new executable input
path, no network access in the harness.

## Idempotence and recovery

Every harness run is repeatable; `make design-check` rewrites only
`validation/design-validation.json` and the witnesses, and the design workflow
requires the committed copy to match. The ledger and ADR edits are plain text
and revertible per commit. Ratification evidence is append-only: correct a
mistake with a new commit citing the correction, never by rewriting the cited
statement.

## Artefacts and notes

To be filled during implementation with gate log paths and short transcripts.

## Interfaces and dependencies

No new dependencies. Python modules use only the standard library and
`jsonschema`, as the existing harness does.

`contracts/acceptance-ledger.json` (schema
`contracts/acceptance-ledger.schema.json`, `$id`
`polean.acceptance-ledger.v0`), shape:

```json
{
  "schema_version": "polean.acceptance-ledger.v0",
  "profile": "polean.tenant-write.v0",
  "roles": {
    "sponsor": {"duty": "Decides proceed, revise, or stop; owns licence and distribution."},
    "technical-reviewer": {"duty": "Accepts or rejects assurance wording and replay evidence."},
    "maintainer": {"duty": "Produces toolchain, admission, audit, sandbox, and budget evidence."},
    "policy-author": {"duty": "Supplies the real policy and requirement for the first user review."},
    "application-owner": {"duty": "States how subject and resource attributes are authenticated."}
  },
  "settled": {
    "repository_licence": {"spdx": "ISC", "evidence": ["LICENSE", "Cargo.toml", "docs/adr-001-repository-bootstrap.md"]}
  },
  "assurance_statement": {
    "adr": "docs/adr-002-first-profile-assurance-boundary.md",
    "text": "The admitted policy model satisfies ... enforces the decision.",
    "dimensions": {"logic": "kernel_checked", "source_binding": "trusted_frontend",
                   "runtime_correspondence": "tested_agreement", "attribute_provenance": "assumed"},
    "decision": "pending",
    "decided_by": []
  },
  "gates": [
    {"id": "toolchain-pins", "responsible": "maintainer", "accepted_by": ["technical-reviewer"],
     "sources": ["TDD §§11, 13", "ToR §8.3", "ADR-001"], "closing_task": "1.2.1",
     "blocks": ["e0", "public-acceptance"],
     "closure_evidence": "Recorded Rust, Regorus, Lean, and OPA pins with smoke results.",
     "status": "open", "evidence": []}
  ]
}
```

The ten gates (all `open` at EP-M2):

| Gate                           | Responsible       | Accepted by                 | Sources                         | Closing task | Blocks                |
| ------------------------------ | ----------------- | --------------------------- | ------------------------------- | ------------ | --------------------- |
| `toolchain-pins`               | maintainer        | technical-reviewer          | TDD §§11, 13; ToR §8.3; ADR-001 | 1.2.1        | e0, public-acceptance |
| `admission-coverage`           | maintainer        | technical-reviewer          | TDD §§4.1, 6.1, 13; ToR A5      | 1.2.2        | e0, public-acceptance |
| `axiom-audit`                  | maintainer        | technical-reviewer          | TDD §§5, 6.2, 13                | 1.2.3        | e0, public-acceptance |
| `execution-sandbox`            | maintainer        | technical-reviewer          | TDD §§2, 11, 13; ToR A4         | 1.2.4, 4.3.1 | public-acceptance     |
| `resource-budgets`             | maintainer        | technical-reviewer          | TDD §§11, 13; ToR OQ4, A6, SC7  | 4.3.2        | public-acceptance     |
| `dependency-distribution`      | sponsor           | sponsor                     | TDD §13; ToR OQ5, §8.1          | 5.2.2        | public-acceptance     |
| `assurance-statement`          | sponsor           | sponsor, technical-reviewer | TDD §§1, 13; ToR A2, OQ2        | 1.1.1        | public-acceptance     |
| `first-user-review`            | sponsor           | sponsor, technical-reviewer | TDD §12; ToR SC8, OQ1           | 5.1.1        | public-acceptance     |
| `attribute-provenance`         | application-owner | technical-reviewer          | TDD §2; ToR A3, OQ3             | 5.1.1        | (none; residual)      |
| `design-transcript-provenance` | sponsor           | sponsor                     | ToR OQ7, §8.3                   | none         | (none; residual)      |

`execution-sandbox` does not block E0 because technical design §12 lists only
pins, documented source trust, and AST coverage as E0 evidence, and roadmap
1.2.4 says "Unavailable enforcement blocks public acceptance".

`validation/acceptance_ledger.py` public functions (all pure except reads via
`Repository`):

```python
LEDGER_ROLES: tuple[str, ...]
LEDGER_GATES: tuple[str, ...]
STAGES: tuple[str, ...] = ("e0", "public-acceptance")

def normalize_statement(text: str) -> str: ...
def extract_statement(markdown: str, document: str) -> str: ...
def derive_blockers(gates: list[dict[str, Any]]) -> dict[str, list[str]]: ...
def check_ledger(repo: "Repository", validators: dict[str, Draft202012Validator]) -> dict[str, Any]:
    """Run O1–O6 and O8 against committed documents; return summary fields."""
```

`validation/test_acceptance_ledger.py` follows `test_boundary.py`: plain
`test_*` functions, a `Tampered`-style in-memory repository, `expect_rejected`
helpers, and a `main()` returning 0 or 1.

## Revision notes

- 2026-10-15: initial draft from Wyvern reconnaissance. Pending
  community-of-experts review.
