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
project must settle two things on which every later task depends.

First, it must decide exactly what a successful Polean run is allowed to claim.
The technical design proposes a three-part answer: Lean checked a theorem about
the policy *model*; the Rego-to-model frontend remains *trusted* rather than
proved; and OPA and Regorus *agree with the model on tested cases* rather than
by a refinement proof. The roadmap also requires the statement to say plainly
that the authenticity and provenance of request attributes lie outside the
theorem. A sponsor and a technical reviewer must accept that exact wording, or
record a blocking rejection, in their own words on GitHub. That decision is
Architecture Decision Record (ADR) 002.

Second, the technical design lists seven "open gates" (toolchain pins,
admission coverage, axiom audit, sandbox, budgets, licence and distribution,
sponsor acceptance) and says "assign owners before implementation". Nobody has
done so. This work creates an *acceptance ledger*: a machine-readable record
naming, for every gate, the responsible role, the roles whose acceptance closes
it, the repository evidence that must exist, the roadmap tasks expected to
produce that evidence, and whether it blocks the E0 development demonstration
or public acceptance. The ledger also records residual assumptions that remain
visible but block nothing, and it separates the settled fact that Polean's own
source is ISC-licensed from the still-open decision about distributing Polean
with its dependencies.

After this change, a contributor can:

1. read `docs/adr-002-first-profile-assurance-boundary.md` and see the exact
   proposed (later ratified or rejected) assurance statement, its SHA-256
   digest, and, once decided, who decided it, when, and where;
2. read `contracts/acceptance-ledger.json` and see who owns each gate and
   residual assumption and what closes it;
3. run `make design-check` and see the design harness confirm that the ADR and
   the technical design carry the same statement, that the ledger is internally
   consistent and no weaker than a reviewed floor, that every recorded
   acceptance is bound to the current statement digest, and which gates
   currently block E0 and public acceptance; and
4. see the same command reject a set of deliberately corrupted ledgers and
   documents (negative controls), so a passing run is not vacuous.

This work does not implement checker behaviour, prove any theorem, pin any
tool, or close any gate other than `assurance-statement`. It changes no Rust
code.

## Constraints

These are hard invariants. Violating one requires escalation, not a workaround.

- Never fabricate, relay, or infer a human decision. A gate acceptance or
  rejection counts only when the role holder posts it themselves as a GitHub
  comment on the ratification pull request, from their own account, using the
  template in `EP-M3`. The implementing agent must never post such a comment
  (even though `gh` authenticates as the repository owner), never merge the
  ratification pull request, never transcribe a decision from the conversation
  into the ledger, and never fill a role holder with a guessed name. Plan
  approval, silence, a green CI run, or a merged pull request is not a decision.
- Keep the four assurance claims separate (AGENTS.md): admission, model proof,
  source-to-model translation, and runtime correspondence. No new wording may
  imply that admission or the frontend is proved, that OPA or Regorus are
  proved to correspond to the model, or that the theorem authenticates
  attributes or certifies an application.
- Do not change the six claim identifiers or definitions, the admitted atoms,
  the request domain, the exit codes, outcome names, or
  `contracts/result.schema.json`. The only contract changes this plan
  authorizes are the additive `assumptions` array in `contracts/profile.json`
  and its schema, and the two new ledger files.
- Do not modify `src/`, `tests/`, `Cargo.toml`, `Cargo.lock`, `lean/`, or
  `examples/`. This roadmap item has no Rust or Lean deliverable.
- Do not close, waive, or weaken any gate other than `assurance-statement`.
- Roles, not individuals, are the unit of allocation (ToR §9: "Owners below
  denote roles to allocate, not named commitments"). A holder appears only in
  an acceptance entry, and only as the GitHub login that posted the comment.
- The Python design harness owns design-fixture consistency only
  (`docs/repository-layout.md` §Ownership boundaries). Its ledger checks must
  not describe themselves as proof, release certification, or authentication of
  a person.
- Use British English with Oxford spelling
  (`docs/documentation-style-guide.md`); "licence" is the noun.
- Keep `SHA256SUMS` passing at every commit.
- Run gates sequentially, never in parallel.

## Tolerances (exception triggers)

- Scope: more than 28 files or roughly 1,500 net added lines (excluding the
  regenerated `validation/design-validation.json`): stop and escalate.
- Rust: any change under `src/`, `tests/`, or the Cargo manifests: stop.
- Contracts: any contract change beyond those authorized above: stop.
- Wording: if a role holder asks for different wording, apply it only if it
  preserves all four claim boundaries, then restart ratification against the
  new digest; otherwise record the conflict and escalate.
- Dependencies: any Python package beyond `validation/requirements.txt`:
  stop.
- Iterations: a gate still failing after three fix attempts on one cause:
  stop and escalate with the log path.
- Ambiguous decisions: a comment that does not match the template, names a
  different digest, or is posted by an account other than the role holder is
  not a decision. Ask once for a templated comment; do not record anything.
- Residual override: if the sponsor asks for `attribute-provenance` or
  `design-transcript-provenance` to block a stage, stop, move it to `gates`
  with the requested `blocks`, update the floor constant, the expected blocker
  lists in `Validation and acceptance`, and this plan, and obtain approval of
  the changed expectations before continuing.
- No decision: once the implementation pull request (EP-M1 and EP-M2) is
  ready, hand it over for review and merge by a human with ADR-002 at
  `Proposed`; set this plan to `BLOCKED` awaiting ratification. Do not tick the
  roadmap.
- Rejection: record it (EP-M3 completes), set this plan to `BLOCKED` for a
  revise-or-stop direction, and begin no redesign work.
- Summary keys: changing the meaning of an existing key in
  `validation/design-validation.json`: stop and escalate.

## Risks

- Risk: an accepted statement is later edited in every copy at once, so CI
  stays green while release material quotes unratified words. Severity: high.
  Likelihood: medium. Mitigation: every acceptance records the SHA-256 digest
  of the normalized statement it accepted; the harness recomputes the digest
  from the ADR and fails if any acceptance names another digest, so any wording
  change returns the gate to `open`.
- Risk: later tasks (1.2.1, 4.3.2) let an agent close a gate by citing
  repository files. Severity: high. Likelihood: medium. Mitigation: closing any
  gate requires one templated acceptance comment per `accepted_by` role, posted
  by the holder; the developers' guide documents the closure procedure and the
  anti-fabrication rule so later ExecPlans inherit it.
- Risk: a gate is silently weakened by emptying its `blocks` or trimming its
  `accepted_by`. Severity: high. Likelihood: low. Mitigation: a reviewed
  constant `GATE_FLOOR` in the harness sets the minimum `blocks` and
  `accepted_by` for every gate; the ledger must be a superset.
- Risk: one person holds both the sponsor and technical-reviewer roles, so
  the two-party acceptance is one decision. Severity: medium. Likelihood: high
  (ToR calls Polean founder-led). Mitigation: permitted, but the harness derives
  `independent_review` from normalized GitHub logins and requires ADR-002's
  Status line to read
  `Accepted — single holder; no independent technical review` in that case;
  release material must repeat the disclosure.
- Risk: "ledger" already means two other things. Technical design §10 has a
  "trust ledger" and §12 and roadmap 5.1.1 and 5.2.1 have an "assurance
  ledger", both meaning the per-run record of residual trust. Severity: medium.
  Likelihood: high. Mitigation: `docs/context.md` defines all three terms and
  states that the acceptance ledger tracks release-gate ownership, not run
  evidence.
- Risk: Markdown formatting (`mdtablefix --wrap --ellipsis`) reflows block
  quotes and rewrites `...`, breaking marker extraction. Severity: low.
  Likelihood: medium. Mitigation: markers sit outside the block quote on their
  own lines with blank lines around them; extraction is substring-based; only
  line-anchored `^\s*>\s?` prefixes are stripped; the statement must contain no
  `...`.
- Risk: a ratification pull request left open indefinitely conflicts with
  every change to `main`. Severity: medium. Likelihood: medium. Mitigation:
  split delivery into an implementation pull request that merges at `Proposed`,
  and a small ratification pull request.
- Risk: Firecrawl could not be used for external research (no credits).
  Severity: low. Likelihood: certain. Mitigation: dependency licence facts came
  from the GitHub API; they describe the distribution gate only.

## Progress

- [x] (2026-10-15 14:10Z) Renamed branch to
  `1-1-ratify-assurance-and-release-gates`.
- [x] (2026-10-15 14:30Z) Reconnaissance with a Wyvern team: build and gate
  machinery, open gates and owners, assurance wording sources.
- [x] (2026-10-15 14:40Z) External fact check of dependency licences via the
  GitHub API (Firecrawl unavailable).
- [x] (2026-10-15 15:00Z) Drafted this ExecPlan.
- [x] (2026-10-15 16:10Z) Community-of-experts review (six lenses across three
  reviewers); revised the plan (see `Revision notes`).
- [x] (2026-10-15 17:30Z) Firecrawl research once credits were restored:
  confirmed Regorus 0.13.0 metadata from the crates.io API, GitHub comment
  edit-history behaviour, and GitHub's self-approval rule; revised the
  ratification procedure (see `Revision notes`).
- [ ] Plan approval by the user.
- [ ] EP-M0: baseline gates recorded.
- [ ] EP-M1: assurance statement bound across ADR-002 (`Proposed`), technical
  design, ledger, and profile.
- [ ] EP-M2: release gates and residuals allocated with closure rules, floor,
  and derived blockers; implementation pull request ready for human merge.
- [ ] EP-M3: sponsor and technical-reviewer decisions recorded from their
  comments on the ratification pull request.
- [ ] EP-M4: roadmap updated, conformance reconciled, retrospective written.

## Surprises & discoveries

- Observation: the assurance vocabulary is spelled three ways.
  `contracts/profile.json` uses prose (`"trusted frontend"`,
  `"differential tests, not proof"`); `contracts/result.schema.json` uses enums
  (`kernel_checked`, `trusted_frontend`, `tested_agreement`);
  `docs/users-guide.md` and `contracts/cli.txt` print
  `model_proved + trusted_frontend + tested_agreement`, mixing the outcome name
  `model_proved` with two dimension values. Impact: EP-M1 corrects the mixed
  phrase.
- Observation: the canonical statement in technical design §1 omits attribute
  provenance; the exclusion appears only in §2's table, §4.2, ToR A3, and the
  vocabulary. Impact: EP-M1 proposes a fourth sentence.
- Observation: the request has three top-level parts, `subject`, `resource`,
  and `action` (`contracts/request.schema.json`), but ToR §2 and
  `docs/context.md` speak only of subject and resource attributes. Impact: the
  proposed sentence names all three; `docs/context.md` is aligned.
- Observation: `SHA256SUMS` covers `README.md`, `contracts/*`, three design
  documents, `examples/`, and `validation/` (including the regenerated
  summary). It verifies cleanly today, but nothing documents or checks it.
  Impact: EP-M2 adds a `make checksums-check` target, a design-workflow step,
  and a developers' guide paragraph.
- Observation: Regorus has moved on from the inspected 0.12.0. The crates.io
  API (`/api/v1/crates/regorus/0.13.0`, fetched 2026-10-15) reports 0.13.0
  published on 2026-10-14 with licence expression
  `MIT AND Apache-2.0 AND BSD-3-Clause`; default features `full-opa`, `arc`, and
  `rvm`; an opt-in `ast` feature; and a `std` feature that enables `vstd/std`
  (the Verus standard library) as a dependency. The GitHub API reports OPA
  `v1.21.2` and Lean 4 `v4.35.0`, both Apache-2.0. Impact: recorded in the
  `toolchain-pins` and `dependency-distribution` gate descriptions as inputs to
  roadmap 1.2.1 and 5.2.2, not as decisions. The three-licence expression and
  the transitive `vstd` dependency are exactly the facts the sponsor's
  distribution decision must cover.
- Observation: Firecrawl's model-based extraction from the docs.rs page
  reported Regorus 0.13.0's licence as `Apache-2.0`, contradicting the registry
  metadata. Evidence: the same day's crates.io API response above. Impact:
  dependency facts that feed a gate must come from a registry API or the
  package manifest, never from summarized page extraction; the developers'
  guide states this in "Closing an acceptance gate".
- Observation: GitHub comments are mutable. Anyone with read access can view
  a comment's edit history, but the author and anyone with write access can
  delete a revision's content (the editor and time remain visible), and
  comments can be deleted outright. Evidence: GitHub Docs, "Tracking changes in
  a comment", fetched 2026-10-15. Impact: a comment URL alone is not a stable
  decision record. EP-M3 copies each comment's verbatim body, author login,
  creation and update times, and identifier from the GitHub API into the
  ledger, and the harness parses the stored body against the template
  (obligation O6).
- Observation: "Pull request authors cannot approve their own pull
  requests." Evidence: GitHub Docs, "Approving a pull request with required
  reviews", fetched 2026-10-15. Impact: the agent opens pull requests as
  `leynos`, who is the likely sponsor, so a formal "Approve" review is
  unavailable to that holder. The plan uses templated conversation comments
  (`#issuecomment-` anchors), which any collaborator can post, rather than
  review approvals.
- Observation: every phase-2 (E0) task depends transitively on 1.1.2 and
  therefore on 1.1.1, and 2.3.1 requires 1.2.4. Impact: `assurance-statement`
  and `execution-sandbox` block E0 as well as public acceptance.

## Decision log

- Decision: record the acceptance ledger as a versioned JSON contract with a
  JSON Schema, checked by the existing design lane. Rationale: later tasks
  (1.2.1–1.2.4, 4.3.2, 5.1.1, 5.2.2) close these gates, and AGENTS.md requires
  machine-readable contracts to track trust boundaries; a contract lets the
  harness reject a gate closed without acceptances, which a Markdown table
  cannot. Alternatives: a Markdown-only ledger in ADR-002 (half the files, but
  no enforcement of closure rules; viable only if the sponsor prefers prose);
  Rust ledger types tested with `rstest` and `proptest` (rejected: first crate
  dependencies for a non-product reason, against phase 1's "preserve …
  independent design validation", and roadmap 4.2.1 checks a single run's
  gates, not release governance, so no code would be reused); a separate
  ADR-003 for allocation (rejected: allocation is governance state, not an
  architectural decision). Date/Author: 2026-10-15, planning agent after expert
  review.
- Decision: the ledger never stores readiness. The harness derives the
  blocker set and publishes it in `validation/design-validation.json`.
  Rationale: technical design §8, "A malformed report cannot choose its own
  exit status by setting a JSON field." Date/Author: 2026-10-15.
- Decision: gate identifiers, residual identifiers, role identifiers, and a
  per-gate floor of `blocks` and `accepted_by` are reviewed harness constants,
  as `CLAIM_IDS` already is. Rationale: deleting or weakening an inconvenient
  gate must fail the check. Date/Author: 2026-10-15.
- Decision: ADR-002 is the authoritative copy of the statement. Technical
  design §1 carries a marker-bound copy. The ledger stores only the ADR path
  and the statement's SHA-256 digest; each acceptance repeats the digest it
  accepted. `README.md` and `docs/users-guide.md` link to ADR-002 rather than
  restating it. Rationale: two bound copies and a digest are enough; a third
  free-text copy would drift. Date/Author: 2026-10-15.
- Decision: attribute provenance becomes a profile-level `assumptions` array
  in `contracts/profile.json`
  (`["attribute_provenance", "application_enforcement", "state_changes"]`, from
  technical design §2's "Application to request" row), not a result field.
  Rationale: the value never varies between runs, so it is a property of the
  profile, and a fifth result "dimension" would blur the four-claim separation.
  Forward obligation recorded in ADR-002: roadmap 4.2.1 and 4.2.2 must render
  the profile's assumptions in every report and test that no producer omits
  them. Compatibility decision: none (no released API, no consumer).
  Date/Author: 2026-10-15, after expert review rejected the draft's
  `attribute_provenance: assumed` result field.
- Decision: ledger negative controls run inside `make design-check` through
  `validate_design.negative_controls`, not in a new script or Make target.
  Rationale: content controls already live there; `test_boundary.py` is for
  orchestration and storage seams. Date/Author: 2026-10-15.
- Decision: the readiness derivation is verified by a 12-row truth table
  (three statuses × four `blocks` shapes), golden blocker lists over the
  committed ledger, and two seeded mutants, not by enumerating every
  assignment. Rationale: blocker membership depends only on each gate's own
  status and `blocks`, so the per-gate table is exhaustive for the function;
  the draft's 59,049-state enumeration re-checked the same rule at exponential
  cost. Date/Author: 2026-10-15.
- Decision: the new Python module follows its siblings' conventions
  (`validation/requirements.txt`, standard library plus `jsonschema`, no CLI)
  rather than `docs/scripting-standards.md`'s `uv`, PEP 723, Cyclopts, and
  pytest defaults. Rationale: it is an imported module of an existing lane, not
  a new script. Date/Author: 2026-10-15.
- Decision: eight gates (the seven technical design §13 gates plus
  `first-user-review` from §12 and ToR SC8) and two residual assumptions
  (`attribute-provenance`, ToR OQ3 and A3; `design-transcript-provenance`, ToR
  OQ7). OQ6 is not a release gate; roadmap 6.1.1 owns it. Rationale: the
  roadmap's "unresolved provenance" covers OQ3 and OQ7; the ratified statement
  already excludes attribute provenance from the theorem and ToR §8.3 already
  excludes the unseen transcript from acceptance evidence, so neither changes
  what Polean may claim. Residuals stay visible in the published summary. The
  sponsor may override (see `Tolerances`). Date/Author: 2026-10-15.
- Decision: a gate blocks E0 if and only if a phase-2 roadmap task depends
  transitively on its closing task. Rationale: one stated rule replaces the
  draft's inconsistent reading of technical design §12. Result: E0 blockers are
  `toolchain-pins` (1.2.1), `admission-coverage` (1.2.2), `axiom-audit` (1.2.3),
  `execution-sandbox` (1.2.4), and `assurance-statement` (1.1.1). Date/Author:
  2026-10-15.
- Decision: deliver in two pull requests. The implementation pull request
  (EP-M1, EP-M2) merges with ADR-002 `Proposed` and claims nothing it has not
  earned. A small ratification pull request (EP-M3, EP-M4) carries the holders'
  comments and the recorded decision and is merged by a human. Rationale: makes
  "approved but never ratified" a coherent stopping point and keeps the
  decision trail on one small diff. Date/Author: 2026-10-15.
- Decision: `checksums-check` joins the design workflow. Rationale: the
  manifest covers files this work and every later gate closure change; an
  unchecked manifest drifts. Date/Author: 2026-10-15.

## Outcomes & retrospective

Not started. Complete at EP-M4.

## Context and orientation

Polean is at a design-scaffold stage: nothing parses Rego, runs Lean, or calls
OPA. The relevant parts of the repository are:

- `docs/terms-of-reference.md` (ToR): §4 defines roles (policy author,
  technical reviewer, project maintainer, sponsor, production enforcement
  operator). §8.2 lists assumptions A1–A6: A2 says the frontend may stay
  trusted and is resolved when the "sponsor accepts the assurance statement or
  rejects it before release"; A3 says request attributes come from trusted
  application components. §9 lists open questions OQ1–OQ7 with proposed owner
  roles; OQ3's owner is "Application owner".
- `docs/technical-design.md` (TDD): §1 holds the proposed first public
  assurance statement as a block quote; §2 is the trust-boundary table; §8
  defines outcomes such as `model_proved`; §10 mentions the "trust ledger"; §12
  defines experiments E0–E2 and the first user review; §13 ends with the "Open
  gates" sentence this plan allocates.
- `docs/context.md`: the vocabulary (model theorem, source binding, runtime
  correspondence, kernel checked, trusted computing base).
- `docs/adr-001-repository-bootstrap.md`: the only ADR. Header bullets in the
  form `- **Status:** …`, `- **Date:** …`, `- **Decision owners:** …`; sections
  Context, Decision, Consequences, Alternatives considered. It records the ISC
  licence and defers toolchain pins to E0.
- `contracts/`: JSON Schemas and the profile. `profile.json`'s `assurance`
  object holds prose constants fixed by `profile.schema.json`.
- `validation/validate_design.py`: the design harness. `Repository` is the
  read-only input boundary; `Workspace` is the only writer; `run_checks()` is
  the composition root and writes `validation/design-validation.json`;
  `negative_controls()` splices control lists from `request_controls`,
  `contract_controls`, and `witness_controls`; `load_schemas()` loads every
  `contracts/*.schema.json`, so a new schema is meta-validated automatically and
  `json_schemas_checked` rises from 5 to 6.
- `Makefile`: `check-fmt`, `markdownlint`, `typecheck`, `lint`, `test`,
  `design-check`, `design-properties`, `design-boundary`, and `all`.
- `.github/workflows/design.yml`: runs the three design targets and fails if
  the tree is dirty afterwards.
- `SHA256SUMS`: a provenance manifest; `sha256sum -c SHA256SUMS` verifies it.

Terms used here:

- *Assurance statement*: the sentences a successful run may claim; *ratified*
  means accepted by the sponsor and technical reviewer under the template.
- *Statement digest*: SHA-256 of the statement after normalization (strip
  line-anchored `>` prefixes, collapse whitespace runs to one space, trim).
- *Gate*: a decision or evidence that must exist before a stage (E0 or public
  acceptance) may proceed. *Blocker*: a gate that blocks a stage and is not
  `closed`.
- *Residual assumption*: an owned, visible assumption that blocks no stage.
- *Acceptance ledger*: the contract listing gates, residuals, owner roles,
  closure rules, and recorded acceptances. Distinct from the *assurance ledger*
  or *trust ledger* of technical design §§10 and 12, which describe a run's
  residual trust.
- *Acceptance comment*: a GitHub comment by a role holder on the ratification
  pull request, in the form
  `<role>: ACCEPT|REJECT <gate> sha256:<digest-or-none> [reason]`.

Required references, in reading order: `docs/terms-of-reference.md` (§§1–5,
8–10), `docs/technical-design.md` (§§1–3, 8–10, 12–13), `docs/context.md`,
`docs/adr-001-repository-bootstrap.md`, `docs/roadmap.md` (phases 1, 2, and 5),
`docs/documentation-style-guide.md`, `docs/developers-guide.md`,
`docs/repository-layout.md`, `validation/README.md`, and
`docs/scripting-standards.md` (for the documented exception). Read
`docs/complexity-antipatterns-and-refactoring-strategies.md` before shaping the
harness functions. `docs/reliable-testing-in-rust-via-dependency-injection.md`,
`docs/rust-testing-with-rstest-fixtures.md`, and
`docs/rust-doctest-dry-guide.md` do not apply, because no Rust changes; they
apply to the forward obligation at roadmap 4.2.1. `docs/references.md` does not
exist; each document lists its own sources.

Skills to load: `execplans` (maintaining this plan), `arch-decision-records`
(ADR-002), `en-gb-oxendict-style` (prose), `python-router` then
`python-data-shapes` (ledger data handling), `rust-router` and
`rust-verification` (to confirm that no Rust, Kani, or Verus work is warranted),
`commit-message`, and `pr-creation`. Delegate full gate runs to `scrutineer`.

## Conformance basis

Upstream artefacts at commit `9a867bd`:

- Terms of reference, draft v0.1 (2026-10-04): A2, A3, A4, A5, A6, OQ1–OQ7,
  SC7, SC8, §4 roles, §8.1 (no licensing requirement authorized), §8.3
  (dependency tuple).
- Technical design, draft v0.1 (2026-10-04): §1 statement (TDD-STMT); §2
  "Application to request" row (TDD-BND-APP); §8 `model_proved` row (TDD-OUT);
  §10 trust ledger (TDD-V); §12 exit gates (TDD-EXIT); §13 open gates
  (TDD-GATES).
- ADR-001 (accepted, 2026-10-08): ISC licence; toolchain pin deferred to E0.
- Roadmap: RM-1.1.1 and RM-1.1.2; consumers RM-1.2.1–1.2.4, RM-4.2.1,
  RM-4.2.2, RM-4.3.1, RM-4.3.2, RM-5.1.1, RM-5.2.2.
- Governing standards: AGENTS.md, `docs/documentation-style-guide.md`,
  `docs/repository-layout.md` §Ownership boundaries.

Trace links:

```plaintext
TOR-A2, TOR-OQ2 -> TDD-STMT -> RM-1.1.1 -> EP-M1, EP-M3
  -> ADR-002 ratification record; gate assurance-statement; O1, O6
TOR-A3, TOR-OQ3 -> TDD-BND-APP -> RM-1.1.1 -> EP-M1
  -> fourth statement sentence; profile assumptions; O2
TOR-OQ3, TOR-OQ7 -> RM-1.1.2 -> EP-M2 -> ledger residuals; O3
TDD-OUT -> EP-M1 -> users-guide and cli.txt wording fix; O2
TDD-GATES, TOR-OQ4, TOR-OQ5, TOR-SC8 -> RM-1.1.2 -> EP-M2
  -> ledger gates and floor; O3, O4
ADR-001 ISC, TOR-OQ5 -> RM-1.1.2 -> EP-M2
  -> settled repository_licence vs gate dependency-distribution; O5
TDD-EXIT, roadmap phase-2 dependencies -> RM-1.1.2 -> EP-M2
  -> derived blockers in design-validation.json; O7
```

ToR OQ6 traces to RM-6.1.1 and is deliberately outside the ledger.

## Verification plan

This change introduces data contracts and consistency rules over documents, not
executable product logic. The invariants are finite and document-shaped. The
right tools are named harness checks, negative controls in the existing
`negative_controls` style, and an exhaustive per-gate truth table.

Verus is not warranted: there is no Rust function, no unbounded data structure,
and no contractual business logic in a compiled component. Kani and `proptest`
are not used for the same reason. Hypothesis is available in this Python lane
but is unnecessary for a 12-row domain that is enumerated completely.
`rstest-bdd` and `insta` are not used because there is no Rust behaviour; the
observable workflow is `make design-check`, whose expected outputs are pinned
by the committed summary and the design workflow's clean-tree check. The Rust
testing stack (`rstest`, `googletest`, `pretty_assertions`, `insta`,
`proptest`) becomes a forward obligation at roadmap 4.2.1, which must assert
that every report carries the profile's assumptions; ADR-002 records that
obligation.

Axioms relied on, not verified:

- AX1: `jsonschema` 4.26.0 implements Draft 2020-12 correctly.
- AX2: a GitHub comment recorded in an acceptance was posted by the login
  recorded with it, that login belongs to the role holder, and the comment had
  not been edited when it was copied (`created_at` equals `updated_at` in the
  API response). The harness checks the URL's shape, the login's form, and that
  the stored verbatim body parses to the recorded role, decision, gate, and
  digest; it does not query GitHub or authenticate a person. Reviewers confirm
  by opening the link. A later edit or deletion of the comment is visible on
  GitHub but not to the offline harness; the stored body remains the record.
- AX3: a cited repository artefact is relevant to the gate it supports.
  Relevance is a reviewer's judgement; the harness checks existence only.

Obligations:

- O1 statement binding. The text between
  `<!-- polean:assurance-statement:start -->` and
  `<!-- polean:assurance-statement:end -->` occurs exactly once in ADR-002 and
  once in technical design §1; after normalization the two are equal, contain no
  `...`, and their SHA-256 equals `assurance_statement.sha256` in the ledger.
  Method: named check plus controls. Artefact:
  `validation/acceptance_ledger.py`. Non-vacuity: the real ADR and TDD are the
  witnesses; controls change one word in the TDD copy, delete an end marker,
  duplicate the block, and alter the ledger digest, and each is rejected naming
  the document; a rewrap-only change is accepted, showing the comparison is not
  raw byte equality.
- O2 vocabulary alignment. The ledger's `dimensions` equal
  `{logic: kernel_checked, source_binding: trusted_frontend,
  runtime_correspondence: tested_agreement}`;
  each value is in the matching enum of `contracts/result.schema.json`; the
  ledger's `assumptions` equal `contracts/profile.json`'s `assumptions`, which
  the profile schema fixes and requires. Non-vacuity: controls change
  `source_binding` to `proved_translation`, drop `attribute_provenance` from
  the profile, and remove `assumptions` from the profile schema's `required`
  list; each is rejected.
- O3 closed vocabularies and floor. Role, gate, and residual identifiers equal
  the reviewed constants `LEDGER_ROLES`, `LEDGER_GATES`, and
  `LEDGER_RESIDUALS`, without duplicates; every role reference is declared;
  each gate's `blocks` and `accepted_by` are supersets of `GATE_FLOOR`.
  Non-vacuity: controls delete `execution-sandbox`, duplicate `toolchain-pins`,
  set a `responsible` to `release-manager`, empty `axiom-audit`'s `blocks`, and
  drop `technical-reviewer` from `toolchain-pins`'s `accepted_by`; each is
  rejected.
- O4 closure rules. An `open` gate has no acceptances and no `decided_on`. A
  `closed` gate has a `decided_on` date, at least one existing repository
  artefact that is not a subset of any `settled` entry's evidence, and exactly
  one `ACCEPT` acceptance per `accepted_by` role. A `rejected` gate has a
  `decided_on` date and at least one `REJECT` acceptance with a reason. Every
  acceptance has a role in the gate's `accepted_by`, a holder matching
  `^[a-z0-9](?:[a-z0-9-]{0,38})$` that does not end in `[bot]`, an ISO date,
  and a comment URL matching
  `^https://github\.com/leynos/polean/pull/[0-9]+#issuecomment-[0-9]+$` (the
  prefix lives in one constant). Non-vacuity: after EP-M3 the
  `assurance-statement` gate is the positive witness; before EP-M3 a
  constructed closed gate in the controls serves. Controls close `axiom-audit`
  with no acceptances, with only one of two roles, with a `docs/missing.md`
  artefact, with only `LICENSE` as artefact, with an `https://example.com/`
  URL, and with holder `github-actions[bot]`; each is rejected.
- O5 licence distinction. `settled.repository_licence.spdx` is `ISC`, equals
  `Cargo.toml`'s `license`, and `LICENSE` begins `ISC License`; no gate or
  residual concerns the repository licence; `dependency-distribution` is a gate
  owned by `sponsor`. Non-vacuity: a control changes the SPDX to `MIT` and is
  rejected; O4's settled-subset control covers closure by `LICENSE` alone.
- O6 ratification binding. Every acceptance on `assurance-statement` carries
  `statement_sha256` equal to the current digest; ADR-002's `- **Status:** …`
  line begins `Proposed` when the gate is `open`, `Accepted` when `closed`, and
  `Rejected` when `rejected`; when the sponsor and technical-reviewer holders
  are equal, an `Accepted` status reads
  `Accepted — single holder; no independent technical review`. The summary
  publishes `independent_review` as `null` while open and a Boolean otherwise.
  Every acceptance's `comment_body`, trimmed, is one line of the form
  `<role>: <ACCEPT|REJECT> <gate> sha256:<64 hex digits or none> [reason]`, and
  its parts equal the entry's `role`, `decision`, gate, `statement_sha256`, and
  `reason`; `comment_created_at` equals `comment_updated_at`. Non-vacuity: the
  committed `Proposed` ledger is the pending witness; controls add an
  acceptance with a stale digest, set the gate `closed` while the ADR says
  `Proposed`, use equal holders without the single-holder wording, store a body
  naming a different role or digest than the entry, and store unequal creation
  and update times; each is rejected. Scheduled in EP-M2's red step.
- O7 readiness derivation. For each stage `s` in `{e0, public-acceptance}`,
  `blockers(s)` lists, in ledger order, the gates with `s` in `blocks` and
  status other than `closed`; `rejected` never satisfies a gate; residuals
  never block. Method: a 12-row truth table over status × `blocks` shape for a
  single gate, golden lists for the committed ledger, and two seeded mutants
  (one treats `rejected` as satisfied, one ignores `blocks`) that the truth
  table must reject. Non-vacuity: the table includes rows that block and rows
  that do not for both stages; the mutants prove it can fail.
- O8 no self-declared state. The ledger schema sets
  `additionalProperties: false` on every object, constrains stage names to an
  enum, and constrains status to `open`, `closed`, `rejected`. Non-vacuity: a
  control adds `"public_acceptance": "ready"` and is rejected by schema
  validation.

Every obligation is checked against committed documents by `make design-check`,
and every control runs in the same command. The residual gap is AX2 and AX3:
the harness shows the ledger is well formed and bound to the current statement,
not that a person is who they claim or that evidence is sufficient.

## Plan of work

### Stage A: confirm the baseline (EP-M0)

No edits. Run every gate once, sequentially, through `scrutineer`, and record
log paths in `Artefacts and notes`. If a gate fails on the untouched branch,
stop and report.

### Stage B and C for EP-M1: bind the assurance statement

Red. Create `validation/acceptance_ledger.py` with constants
`STATEMENT_MARKERS`, `EXPECTED_DIMENSIONS`, `EXPECTED_ASSUMPTIONS`, and
functions `normalize_statement`, `extract_statement`, `statement_digest`,
`check_statement_binding`, and `check_vocabulary`, plus `ledger_controls()`
returning control tuples in the shape `negative_controls` already consumes. Wire
`check_ledger(repo, validators)` into `validate_design.run_checks()` after
`check_result_schema`, and splice `ledger_controls()` into `negative_controls`.
The module reads only through `Repository`. Run `make design-check`; it must
fail with `cannot read contracts/acceptance-ledger.json`.

Green. Write `contracts/acceptance-ledger.schema.json` (no `$id`, matching
siblings) and `contracts/acceptance-ledger.json` holding `schema_version`,
`profile`, `roles`, `settled`, `assurance_statement`, an empty `gates` array,
and an empty `residuals` array. The schema never constrains how many gates
exist; the closed set is a harness constant checked by O3, which EP-M2 adds.

Write `docs/adr-002-first-profile-assurance-boundary.md` with header bullets
`- **Status:** Proposed — awaiting sponsor and technical-reviewer ratification.`,
`- **Date:** …`, and `- **Decision owners:** sponsor and technical reviewer.`,
and sections Context, Decision, Consequences, Alternatives considered, and
Ratification record. The Decision section contains a one-paragraph Y-statement
summary, then the marked statement on its own lines outside any block quote:

```plaintext
The admitted policy model satisfies the six registered tenant-write
requirements for every valid request. Lean checked the model evidence. The
source frontend remains trusted; OPA and Regorus agree with the model on the
recorded conformance cases, not by a refinement theorem. The model theorem
takes the request's subject, resource, and action attributes as given: it
does not authenticate them, establish their provenance, or certify how an
application enforces the decision.
```

The Decision section then states: admission is performed by the trusted
frontend, `check` establishes admission only, and admission is not proved; a
successful `prove` or `verify` reports outcome `model_proved` with assurance
`logic: kernel_checked`, `source_binding: trusted_frontend`, and
`runtime_correspondence: tested_agreement`, under the profile's `assumptions`;
no test result upgrades `trusted_frontend` or `tested_agreement`; and the
statement is the most that release material may claim. Consequences record the
forward obligation on roadmap 4.2.1 and 4.2.2 and the disclosure duty when one
person holds both roles. Alternatives considered cover "require checked source
correspondence before v0" (A2's consequence-if-false), "omit provenance from
the statement", and "a result field rather than profile assumptions". The
Ratification record explains the acceptance-comment template and is otherwise
empty.

Update the TDD §1 statement to the same marked text (markers outside the quote)
and add ADR-002 and the ledger to its Companions. Update the TDD §8
`model_proved` row to mention the profile's assumptions. In `README.md` lines
30–33 and `docs/users-guide.md`, replace restated wording with a short pointer
to ADR-002 and correct `model_proved + trusted_frontend + tested_agreement` to
name the outcome and the three dimension values separately; make the same
correction in `contracts/cli.txt`. Add `assumptions` to
`contracts/profile.json` and to `contracts/profile.schema.json` (a `const`
array and an entry in the top-level `required` list), and add one `require`
line to `check_profile`. In `docs/context.md`, add *Assurance statement*,
*Attribute provenance*, *Acceptance ledger*, and *Assurance ledger* (covering
the TDD's "trust ledger"), and change the closing provenance sentence to name
subject, resource, and action attributes. Link ADR-002 from `docs/contents.md`.
Update `validation/README.md` ("Five schemas" becomes six; new rows for the
ledger checks). Run `make design-check`; review the regenerated summary diff.

Refactor. Keep each new function under roughly 40 lines; keep the module free
of writes; refresh `SHA256SUMS` for every changed covered file and add the new
contract and validation files.

### Stage B and C for EP-M2: allocate gates and residuals

Red. Add O3–O8 checks, the `GATE_FLOOR`, `LEDGER_GATES`, `LEDGER_RESIDUALS`, and
`LEDGER_ROLES` constants, the readiness truth table with its two seeded
mutants, and their controls. Run `make design-check`; it must fail with
`Ledger gates mismatch`.

Green. Populate `gates` and `residuals` as specified in
`Interfaces and dependencies`, all `open`. Implement
`derive_blockers(gates) -> dict[str, list[str]]` and publish `acceptance_gates`
(8), `acceptance_blockers`, `acceptance_residuals`, and `independent_review`
(`null`) in the summary. Add a `checksums-check` Make target
(`sha256sum -c --quiet SHA256SUMS`), add it to `all`, and add a design-workflow
step after the clean-tree check.

Documentation. In TDD §13, keep the decision table and replace "Assign owners
before implementation" with a pointer to the ledger as the authoritative record
of owners, closure rules, and status. Add one sentence to ToR §9 that the
ledger allocates these roles to gates and residuals, leaving the OQ table
unchanged. In `docs/developers-guide.md`, add a subsection "Closing an
acceptance gate" (the acceptance-comment template, the anti-fabrication rule,
the digest rule, the floor, the verbatim API-copy rule, the rule that
dependency facts come from registry APIs or manifests rather than summarized
page extraction, and that agents never post acceptance comments or merge
ratification pull requests), a `checksums-check` row, a paragraph on
`SHA256SUMS`, and the scripting-standards exception. In
`docs/repository-layout.md`, extend the `contracts/` row and explain that the
ledger is governance state kept there so its schema is meta-validated with the
others.

Refactor and hand-over. Run the full sequence, refresh `SHA256SUMS`, push, and
mark the implementation pull request ready for review. A human merges it with
ADR-002 at `Proposed`.

### EP-M3: record the decisions (ratification pull request)

From an up-to-date `main`, create branch `1-1-ratify-adr-002` and open a draft
pull request whose description quotes the statement, its digest, and the
template. Ask the user, in one message, to arrange for the sponsor and the
technical reviewer each to post, from their own GitHub accounts:

```plaintext
sponsor: ACCEPT assurance-statement sha256:<digest>
technical-reviewer: ACCEPT assurance-statement sha256:<digest>
```

or `REJECT … sha256:<digest> <reason>`, and to say whether the residual
treatment of the two provenance items stands. Point out that one person may
hold both roles and that the ADR will disclose it.

Use conversation comments, not review approvals: GitHub does not let a pull
request's author approve it, and the agent opens the pull request as `leynos`.

When conforming comments exist, fetch each one with
`gh api repos/leynos/polean/issues/comments/<id>` and copy, without paraphrase,
its `body`, `user.login`, `created_at`, `updated_at`, `id`, and `html_url` into
an acceptance entry, together with the role, decision, digest, and reason
parsed from the body. Reject (do not record) a comment whose `updated_at`
differs from `created_at`, whose author is not the person the user named for
that role, or whose body does not match the template; ask for a fresh comment
instead. Then set the gate's status and `decided_on`, update ADR-002's Status
line and Ratification record (linking and quoting each comment), and, if
superseded, ToR §4's "individual authority not assigned" note. Run the gates.
Never post, edit, or paraphrase a decision comment.

### EP-M4: close out (same ratification pull request)

If the gate is `closed`, tick roadmap 1.1.1 and 1.1.2. If it is `rejected`,
leave both unticked, annotate 1.1.1 as "rejected on DATE; revise-or-stop
pending (ADR-002)", and set this plan to `BLOCKED`. Reconcile discoveries with
the conformance basis, complete `Outcomes & retrospective`, run the full
sequence, refresh `SHA256SUMS`, and hand the pull request to a human to merge.

## Milestones and plateaus

- EP-M0. Outcome: every gate passes on the untouched branch. Acceptance: logs
  cited in `Artefacts and notes`. Compatibility decision: none.
- EP-M1. Outcome: ADR-002 `Proposed`; the statement is bound across ADR-002,
  TDD §1, and the ledger digest; the profile carries `assumptions`.
  Requirements: TOR-A2, TOR-A3, TOR-OQ2 (advanced), TDD-STMT, TDD-OUT.
  Acceptance: `make design-check` passes, reports `json_schemas_checked: 6`,
  and the O1 and O2 controls are counted in `negative_controls_passed`.
  Conformance check:
  - TDD §1 changed only by the statement revision and markers.
  - Four claim boundaries intact; no wording promotes tests into proof.
  - A2 and A3 unchanged; A3 now visible in the statement.
  - Only the authorized profile field and ledger files changed in contracts.
  - No trust boundary moved; provenance was already outside the theorem.
  - Trace links current.
  Recovery: revert the milestone commits. Remaining gaps: gates, residuals,
  ratification. Compatibility decision: none.
- EP-M2. Outcome: a complete, open-gated ledger with floor, closure rules,
  residuals, derived blockers, and a checked `SHA256SUMS`; implementation pull
  request ready for human merge. Requirements: TDD-GATES, TOR-OQ3–OQ5, TOR-OQ7,
  TOR-SC8, TDD-EXIT, ADR-001 licence. Acceptance: the outputs in
  `Validation and acceptance`. Conformance check:
  - Every §13 gate and each named roadmap category has a responsible role.
  - The ISC licence is settled; distribution is an open gate.
  - No gate is closed.
  - The `checksums-check` target and workflow step are the only build
    changes.
  - Trace links current.
  Recovery: revert the milestone commits. Remaining gaps: ratification.
  Compatibility decision: none.
- EP-M3. Outcome: the gate is `closed` or `rejected`, each acceptance bound
  to a holder's own comment and the current digest. Requirements: RM-1.1.1
  success criterion; TOR-OQ2 closed or blocking-rejected. Acceptance: O4 and O6
  pass on the decided ledger. Conformance check: no other gate changed; every
  recorded field is copied from a linked comment. Recovery: correct a
  mistranscription in a new commit citing the comment; never edit the comment
  trail. Compatibility decision: none.
- EP-M4. Outcome: roadmap reflects the decision; plan closed or blocked.
  Requirements: RM-1.1.1, RM-1.1.2. Acceptance: roadmap state as described;
  every gate passes. Compatibility decision: none.

## Concrete steps

Run everything from the worktree root. Prepare the Python environment once:

```bash
uv venv /tmp/polean-venv
uv pip install --python /tmp/polean-venv/bin/python -r validation/requirements.txt
```

Gate sequence (delegate to `scrutineer`; never run two at once), each teed to
`/tmp/$ACTION-polean-$(git branch --show-current).out`:

```bash
make check-fmt
make markdownlint
make typecheck
make lint
make test
make design-check PYTHON=/tmp/polean-venv/bin/python
make design-properties PYTHON=/tmp/polean-venv/bin/python
make design-boundary PYTHON=/tmp/polean-venv/bin/python
make checksums-check            # from EP-M2; before then: sha256sum -c SHA256SUMS
git status --porcelain          # empty once the regenerated summary is committed
```

Refresh one checksum line after editing a covered file, then re-run the check:

```bash
sha256sum contracts/profile.json   # replace the old line with this output
```

Compute the statement digest once ADR-002 exists. Make the digest-mismatch
diagnostic print the digest computed from ADR-002, so the first green run can
copy it into the ledger after a reviewer confirms the ADR text is the intended
statement.

Expected focused outputs:

```plaintext
$ make design-check PYTHON=/tmp/polean-venv/bin/python   # EP-M1 red
design validation I/O failure: cannot read contracts/acceptance-ledger.json: ...
$ make design-check PYTHON=/tmp/polean-venv/bin/python   # EP-M2 red
design validation check failed: Ledger gates mismatch
```

Commit after each red and green step where the tree is coherent, with
imperative subjects (for example "Bind the first-profile assurance statement
across ADR and design") and bodies stating the assurance impact.

## Validation and acceptance

After EP-M2, `make design-check` exits 0 and its summary contains:

```json
{
  "json_schemas_checked": 6,
  "acceptance_gates": 8,
  "acceptance_blockers": {
    "e0": ["toolchain-pins", "admission-coverage", "axiom-audit",
           "execution-sandbox", "assurance-statement"],
    "public-acceptance": ["toolchain-pins", "admission-coverage",
                          "axiom-audit", "execution-sandbox",
                          "resource-budgets", "dependency-distribution",
                          "assurance-statement", "first-user-review"]
  },
  "acceptance_residuals": ["attribute-provenance",
                           "design-transcript-provenance"],
  "independent_review": null
}
```

`negative_controls_passed` rises from 19 by the number of ledger controls
(record the exact figure in `Artefacts and notes`). Editing one word of the
statement in `docs/technical-design.md` makes `make design-check` exit 1 with a
diagnostic naming that file. `make checksums-check` exits 0.

After EP-M3 with acceptance: `assurance-statement` leaves both blocker lists;
ADR-002's Status begins `Accepted`; `independent_review` is a Boolean. After a
rejection: the gate stays in both lists with status `rejected`, and this plan is
`BLOCKED`.

Red-Green-Refactor evidence to record: the EP-M1 red I/O failure, the EP-M2 red
gate mismatch, each green run, and each refactor re-run.

Quality criteria: every command in `Concrete steps` passes; O1–O8 discharged as
described; Markdown passes `markdownlint-cli2` and `mdtablefix --check`.
Security: no new executable input path, no network access in the harness.

## Idempotence and recovery

Every harness run is repeatable; `make design-check` rewrites only the summary
and witnesses, and the design workflow requires the committed copy to match.
Ledger and ADR edits are plain text and revertible per commit. Acceptance
comments are the decision trail: correct a mistranscription with a new commit
citing the comment, and never ask a holder to edit an old comment; ask for a
new one.

## Artefacts and notes

To be filled during implementation with gate log paths, the statement digest,
the control count, and short transcripts.

## Interfaces and dependencies

No new dependencies: the standard library and `jsonschema`, as now.

`contracts/acceptance-ledger.json` shape (abridged; the schema constrains every
object with `additionalProperties: false`):

```json
{
  "schema_version": "polean.acceptance-ledger.v0",
  "profile": "polean.tenant-write.v0",
  "roles": {
    "sponsor": {"duty": "Decides proceed, revise, or stop; owns distribution."},
    "technical-reviewer": {"duty": "Accepts wording, evidence, and replay."},
    "maintainer": {"duty": "Produces pin, admission, audit, sandbox, budget evidence."},
    "policy-author": {"duty": "Supplies the real policy for the first user review."},
    "application-owner": {"duty": "States how request attributes are authenticated."}
  },
  "settled": {
    "repository_licence": {
      "spdx": "ISC",
      "evidence": ["LICENSE", "Cargo.toml", "docs/adr-001-repository-bootstrap.md"]
    }
  },
  "assurance_statement": {
    "adr": "docs/adr-002-first-profile-assurance-boundary.md",
    "sha256": "<digest>",
    "dimensions": {"logic": "kernel_checked", "source_binding": "trusted_frontend",
                   "runtime_correspondence": "tested_agreement"},
    "assumptions": ["attribute_provenance", "application_enforcement", "state_changes"]
  },
  "gates": [
    {
      "id": "toolchain-pins",
      "responsible": "maintainer",
      "accepted_by": ["technical-reviewer"],
      "sources": [{"doc": "docs/technical-design.md", "section": "§§11, 13"}],
      "closing_tasks": ["1.2.1"],
      "blocks": ["e0", "public-acceptance"],
      "closure_evidence": "Recorded Rust, Regorus, Lean, and OPA pins with smoke results.",
      "status": "open",
      "artefacts": [],
      "acceptances": []
    }
  ],
  "residuals": [
    {
      "id": "attribute-provenance",
      "responsible": "application-owner",
      "sources": [{"doc": "docs/terms-of-reference.md", "section": "A3, OQ3"}],
      "closing_tasks": ["5.1.1"],
      "resolution": "Document the integration's attribute provenance and ingress contract.",
      "status": "open"
    }
  ]
}
```

An acceptance entry is
`{"role", "holder", "decision": "ACCEPT"|"REJECT", "date", "comment_url",
"comment_id", "comment_body", "comment_created_at", "comment_updated_at",
"statement_sha256"?, "reason"?}`,
with `statement_sha256` required on `assurance-statement` and `reason`
required on `REJECT`. `holder` is the comment's `user.login`, lower-cased.
`comment_body` is the verbatim API `body`, which O6 parses against the template.
`closing_tasks` entries match `^[1-6](\.[0-9]+){2}$` and may be empty.

Gates (all `open` at EP-M2). The floor equals these `blocks` and `accepted_by`
values:

| Gate                      | Responsible | Accepted by                 | Sources                         | Closing tasks | Blocks                |
| ------------------------- | ----------- | --------------------------- | ------------------------------- | ------------- | --------------------- |
| `toolchain-pins`          | maintainer  | technical-reviewer          | TDD §§11, 13; ToR §8.3; ADR-001 | 1.2.1         | e0, public-acceptance |
| `admission-coverage`      | maintainer  | technical-reviewer          | TDD §§4.1, 6.1, 13; ToR A5      | 1.2.2         | e0, public-acceptance |
| `axiom-audit`             | maintainer  | technical-reviewer          | TDD §§5, 6.2, 13                | 1.2.3         | e0, public-acceptance |
| `execution-sandbox`       | maintainer  | technical-reviewer          | TDD §§2, 11, 13; ToR A4         | 1.2.4, 4.3.1  | e0, public-acceptance |
| `resource-budgets`        | maintainer  | technical-reviewer          | TDD §§11, 13; ToR OQ4, A6, SC7  | 4.3.2         | public-acceptance     |
| `dependency-distribution` | sponsor     | sponsor                     | TDD §13; ToR OQ5, §8.1          | 5.2.2         | public-acceptance     |
| `assurance-statement`     | sponsor     | sponsor, technical-reviewer | TDD §§1, 13; ToR A2, OQ2        | 1.1.1         | e0, public-acceptance |
| `first-user-review`       | sponsor     | sponsor, technical-reviewer | TDD §12; ToR SC8, OQ1           | 5.1.1         | public-acceptance     |

Residuals (all `open`, block nothing):

| Residual                       | Responsible       | Sources                | Closing tasks | Resolution                                                              |
| ------------------------------ | ----------------- | ---------------------- | ------------- | ----------------------------------------------------------------------- |
| `attribute-provenance`         | application-owner | TDD §2; ToR A3, OQ3    | 5.1.1         | Integration's attribute provenance and ingress contract recorded.       |
| `design-transcript-provenance` | sponsor           | ToR OQ7, §8.3; TDD R11 | none          | Transcript compared with sourced decisions, or unavailability recorded. |

`application-owner` follows ToR OQ3's owner; `docs/context.md` notes that it
corresponds to the "production enforcement operator" of ToR §4 for a given
integration.

`validation/acceptance_ledger.py` (pure; reads only through `Repository`):

```python
LEDGER_ROLES: tuple[str, ...]
LEDGER_GATES: tuple[str, ...]
LEDGER_RESIDUALS: tuple[str, ...]
GATE_FLOOR: dict[str, tuple[frozenset[str], frozenset[str]]]  # blocks, accepted_by
STAGES: tuple[str, ...] = ("e0", "public-acceptance")
EVIDENCE_URL_PATTERN: str

def normalize_statement(text: str) -> str: ...
def extract_statement(markdown: str, document: str) -> str: ...
def statement_digest(statement: str) -> str: ...
def derive_blockers(gates: list[dict[str, Any]]) -> dict[str, list[str]]: ...
def check_ledger(repo: Repository, validators: dict[str, Draft202012Validator]) -> dict[str, Any]:
    """Run O1–O8 against committed documents; return summary fields."""
def ledger_controls(repo: Repository, validators: dict[str, Draft202012Validator]
                    ) -> list[tuple[str, bool, bool]]: ...
```

## Revision notes

- 2026-10-15: initial draft from Wyvern reconnaissance.
- 2026-10-15: revised after a community-of-experts review (structure,
  contracts, failure modes, viability, alternatives, cost). Changes: bound
  every acceptance to the statement digest and to a holder-posted, templated
  GitHub comment, and forbade agents from posting or merging; generalized
  acceptances to all gates; added `GATE_FLOOR`; made `assurance-statement` and
  `execution-sandbox` block E0 under one stated rule; split provenance items
  into non-blocking residuals; replaced the result-schema field with profile
  `assumptions`; named subject, resource, and action attributes and "the model
  theorem" in the statement; made ADR-002 the authoritative copy with a digest
  in the ledger; folded controls into `design-check` and replaced the
  59,049-state enumeration with a 12-row truth table and mutants; added
  `closing_tasks` lists, structured sources, stage enums, and a
  `checksums-check` gate; split delivery into implementation and ratification
  pull requests; corrected counts and the expected I/O diagnostic. Effect on
  remaining work: fewer files, no new Make target beyond `checksums-check`,
  stronger closure rules.
- 2026-10-15: revised after Firecrawl research became available. Confirmed
  Regorus 0.13.0's licence expression, features, and transitive `vstd`
  dependency from the crates.io API, and recorded that page-extraction
  summaries are not acceptable gate evidence. Because GitHub comments can be
  edited or have revisions deleted, and pull request authors cannot approve
  their own pull requests, acceptances now store the verbatim comment body,
  identifier, and timestamps fetched from the GitHub API; O6 parses the stored
  body against the template and rejects edited comments. Effect on remaining
  work: four more acceptance fields, one more O6 control family, and a
  deterministic copying step in EP-M3; no change to milestones or file count.
