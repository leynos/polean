# Demonstrate declaration-type and transitive-axiom auditing

This ExecPlan is a living document. Keep `Constraints`, `Tolerances`, `Risks`,
`Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan`
current as work proceeds.

Status: DRAFT (revision 1). Implementation must not begin until the user
explicitly approves this plan.

Roadmap item: [1.2.3](../roadmap.md). Branch:
`1-2-3-demonstrate-type-and-axiom-auditing`.

## Purpose / big picture

Polean's first public claim is that Lean checked a model theorem. A compiled
Lean file does not establish that claim. A build succeeds with `sorry`, with
`native_decide`, or with an invented `axiom`. A metaprogram can add a "theorem"
the kernel never saw. A declaration may have the requested name and prove
something else. Technical design §6.2 therefore requires the expected
declaration type, exact requested claim coverage, successful checking, and a
machine-readable audit. Nobody has yet shown that the pinned Lean 4.35.0 makes
this possible without executing submitted code.

This task demonstrates it. When the work is done, a contributor runs
`make toolchain-smoke` and watches a small trusted Lean program,
`polean-audit`, inspect a committed corpus of Lean fixtures. For each fixture
it receives a request that contains only data: the module to inspect, the
theorem roots, the exact proposition each root must state, and the approved
foundational axioms. It returns one JSON report of what it observed. A
test-only Rust settlement kernel reads that report and decides `accepted`,
`rejected` with named findings, or `tool_failure`.

The expected outcomes are observable. An honest theorem and a theorem that uses
all three foundations are accepted. Fixtures that use `sorry`, `native_decide`,
or an undeclared axiom are rejected, including when the axiom is hidden behind
an imported definition. A root with the wrong statement, a root that is itself
an axiom, a forged `propext`, a forged trusted statement, and a forged theorem
that bypassed the kernel are each rejected for their own named reason. A
garbled report is a tool failure, never a rejection and never an acceptance. A
fixture containing an `initialize` block proves the audit did not run its code.

A Verus proof establishes that the settlement kernel accepts exactly when the
report satisfies an independently written specification, for reports of any
length. A proof of `accepted` alone would be satisfied by a kernel that rejects
everything, in the same way that deny-all satisfies the four safety claims
(technical design §3), so the theorem is a characterization in both directions.

This establishes one thing: the audit boundary is usable for the pinned tuple.
It is an input to claim 2 in AGENTS.md (the Polean model satisfies a registered
requirement), and proves no such requirement, because no Polean semantics exist
yet. It says nothing about admission (claim 1), translation (claim 3), or
runtime correspondence (claim 4). Promotion into production code belongs to
2.2.4, and applying the audit to untrusted evidence belongs to 4.1.3.

## Constraints

- Keep the four claims in AGENTS.md separate in every test name, diagnostic,
  contract, and document. An `accepted` settlement is not `model_proved`, and
  its result language must not suggest otherwise.
- Accept no user Lean, Lake files, compiled proof modules, native libraries, or
  plug-ins as verifier input. The audit request is data. No part of it enters a
  Lean executable position: names are decoded into `Name` values and expected
  statements into `Expr` values, and nothing is elaborated, parsed as Lean
  source, or evaluated.
- The audit must not execute code from the modules it inspects. The worker is
  linked without interpreter support and never enables initializer execution.
- Do not add an item to `src/` and do not add a binary target. The guard in
  `tests/documentation_examples/scope.rs` fails when one appears, and 2.1.4
  owns the application edge. The Rust side of this spike is test-only under
  `tests/toolchain/audit/`, as the 1.2.2 spike was under `tests/admission/`.
- The Lean worker is repository-owned trusted code. It lives at
  `lean/Polean/Audit.lean`, the path technical design §6 reserves. Adversarial
  fixtures are compiled into scratch directories and never join the Lake
  package.
- The worker reports observations and carries no verdict. Rust settlement is the
  only acceptance authority. A report cannot choose its own outcome by setting
  a field (technical design §8), and the process exit status is not acceptance
  (technical design §6.2).
- Run tools by explicit digest-named path with a cleared environment and an
  argument array, as `tests/toolchain/smoke/mod.rs` does. Do not resolve any
  tool through `PATH` or elan.
- Do not change a pin: `toolchain-pins.json`, `lean/lean-toolchain`,
  `lean/lake-manifest.json` package content, `rust-toolchain.toml`, or the
  Verus pins. Lake stays free of dependencies.
- Do not use `assume`, `admit`, `external_body`, or any other trusted proof body
  in `verus/`. Do not use `sorry`, `native_decide`, or an `axiom` in
  `lean/Polean/`.
- Do not modify `examples/tenant-write/` or `examples/admission/`.
- Keep Rust modules under 400 lines and functions under 70 lines. Every Rust
  module begins with a module-level comment. `std::fs` remains banned in tests;
  use `tests/toolchain/ambient.rs`.

If satisfying the objective requires violating a constraint, do not proceed.
Record the conflict in `Decision log`, set the status to `BLOCKED`, and
escalate.

## Tolerances (exception triggers)

Exclude this ExecPlan from every count below.

- Scope: stop and escalate if the work needs more than 30 non-fixture files or
  about 2,500 net lines of code and other documentation. Lean fixtures under
  `examples/audit/lean/` do not count, and neither do regenerated validation
  outputs.
- Interface: stop if any public item in `src/` or the proposed CLI in
  `contracts/cli.txt` would have to change.
- Dependencies: the only authorized new dependency is `serde`, pinned `=1.0.229`
  with the `derive` feature under `[dev-dependencies]`. It is already in
  `Cargo.lock` at that version and adds no package. Anything else needs
  approval.
- Design: if any fixture in `Plan of work` cannot be classified as specified by
  the pinned toolchain, or a probe in `Artefacts and notes` does not reproduce,
  stop. Record the deviation, set `BLOCKED`, and wait for acceptance.
- Verification: if the settlement theorem cannot be discharged without a trusted
  body, stop and present the options. Do not weaken the specification to make
  it pass, and do not report the kernel as proved.
- Iterations: stop after three failed attempts on one root cause.
- Resources: stop if the worker exceeds 10 s or 4 GiB resident on any fixture on
  the development host. The design's 2 GiB budget is not enforced by this spike
  (`Risks`), but a large excess would falsify the feasibility claim.
- Ambiguity: stop and present options when a choice materially changes what a
  settlement means.

## Risks

- Risk: the structural statement comparison is too brittle for real claims,
  because elaboration can insert metadata or implicit arguments the data form
  cannot express. Severity: medium. Likelihood: low. Mitigation: the probe
  `Fx.ClaimJob` showed a ground list-literal statement elaborates to bare
  `const` and `app` nodes with no metadata. `AcceptedClaimShape` pins that
  shape, and 2.2.4 controls the fixed template that produces it.
- Risk: replay stops short of what the audit trusts. Severity: high. Likelihood:
  low. Mitigation: replay covers every module not loaded from the toolchain
  sysroot, and `HackImported` is the regression. Toolchain modules are not
  replayed, and this is recorded as trust in the digest-checked archive.
- Risk: the fixed origin table for the foundations drifts on a Lean upgrade.
  Severity: low. Likelihood: medium. Mitigation: it fails closed, because
  `AcceptedFoundations` stops passing. A Lean upgrade is already a tuple change
  needing evidence (ADR 0003).
- Risk: the proof of byte-sequence comparisons in Verus takes more effort than
  the kernel deserves. Severity: medium. Likelihood: low. Mitigation: the
  recognizer proof established the pattern. The verification tolerance applies
  otherwise.
- Risk: the provisioned tests become slow or memory-hungry for CI. Severity:
  low. Likelihood: low. Mitigation: the worker builds in about 15 s and runs in
  about 1 s. Only the `import Lean` forgery fixtures reach about 5 s and 3.3
  GiB resident, most of it mapped files. Hosted runners provide more than that.
- Risk: a reader mistakes the demonstration for protection against hostile
  `.olean` files. Severity: medium. Likelihood: medium. Mitigation: the
  upstream guide says a crafted `.olean` can attack any tool that loads it. ADR
  0006 states that the worker reads only modules freshly compiled from fixed
  templates, and technical design §7 already forbids importing submitted
  `.olean` files.
- Risk: the sibling-worktree toolchain used for the probes is not this
  worktree's gate input. Severity: low. Likelihood: certain. Mitigation: the
  first implementation step runs `make tools` here and repeats the probes
  through the committed fixtures.

## Progress

- [x] (2026-11-05) Renamed the branch and loaded the required skills.
- [x] (2026-11-05) Surveyed the Lean boundary, toolchain, Rust tests, Verus
  tooling, and contracts with a Wyvern team.
- [x] (2026-11-05) Researched `lake comparator`, `lake check`, SafeVerify, and
      the
  Lean proof-validation guide.
- [x] (2026-11-05) Probed the pinned Lean 4.35.0 against adversarial fixtures
      and
  a compiled prototype in a scratch directory.
- [x] (2026-11-05) Drafted this plan.
- [ ] Expert panel review and revision.
- [ ] Publish the draft pull request.
- [ ] User approval. Implementation must not start before this.
- [ ] EP-M1: contracts, fixture corpus, and settlement kernel.
- [ ] EP-M2: Verus proof of settlement.
- [ ] EP-M3: Lean worker and end-to-end corpus run.
- [ ] EP-M4: documentation, design reconciliation, and roadmap.

## Surprises & discoveries

- Observation: `Lean.collectAxioms` is not a plain walk in 4.35.0. It reads
  pre-computed axiom lists for imported declarations from an environment
  extension (`exportedAxiomsExt`), and silently ignores a constant it cannot
  find (`| none => pure ()`). Evidence: `src/lean/Lean/Util/CollectAxioms.lean`
  in the pinned archive. Impact: the worker walks the replayed kernel
  environment itself and treats a missing constant as a tool failure. It uses
  `collectAxioms` only as a cross-check.
- Observation: a theorem added through `debug.skipKernelTC` passes every static
  check: kind `theorem`, the correct type, and an empty axiom list. Evidence:
  `Fx.EnvHack` in `Artefacts and notes`. Impact: kernel replay is required
  rather than optional.
- Observation: `leanchecker Fx.HackUser` exits 0 when the forgery sits in an
  imported module, since it replays only the named module. Evidence: same.
  Impact: replay must cover all non-toolchain modules in the import closure.
- Observation: a `prelude` module can declare its own `axiom propext`. It
  reports the approved name, `leanchecker --fresh` accepts it, and it needs no
  other axiom. Evidence: `Fx.PreludeForge`. Impact: a foundation is identified
  by name and origin. A name-only allowlist is unsound.
- Observation: `native_decide` produces one axiom per use, named such as
  `Fx.Native.helper._native.native_decide.ax_1_1`, in the job's own module.
  Evidence: `Fx.Native`. Impact: this confirms ADR 0003 input 1. No name can be
  blocklisted. Acceptance is by allowlist and the `_native` component only
  selects the diagnostic.
- Observation: `«Quot.sound»` is a one-component name that an unescaped renderer
  would print identically to the two-component `Quot.sound`. Evidence:
  `Fx.Spoof`. Impact: names cross the JSON boundary as component arrays, never
  dotted strings.
- Observation: fresh replay of the whole environment costs 91 s for an
  `Init`-only module, above the proposed 60 s end-to-end budget. Replaying
  non-toolchain modules takes under a second. Evidence: `Artefacts and notes`.
  Impact: this spike trusts the digest-checked toolchain `.olean` files, and
  4.1.3 decides whether a release mode should do better.
- Observation: `docs/references.md`, named in the task's required reading, does
  not exist. Evidence: `ls docs/`. Impact: none on design. References stay in
  the technical design's References section, ADR 0003, and this plan.

## Decision log

- D1. Decision: the worker observes and Rust settles. Rationale: technical
  design §8 forbids a report choosing its own outcome. One authority is simpler
  than comparing two verdicts, and it places the contractual logic in a small
  pure function that Verus can prove. Date/Author: 2026-11-05, plan author.
- D2. Decision: build a small repository-owned worker instead of adopting
  `lake comparator` or `lake check`. Rationale: those tools are the upstream
  gold standard for hostile proofs, and they do not fit this boundary now. They
  refuse to start without `bwrap`, which is absent on the development host
  (exit 2). The rejection observed with `--inadvisably-no-sandbox` is the text
  `error: Illegal axiom detected: 'cheat'` and exit 1, with no per-root
  structured result. Each run exports the environment (36 s and 345 MB for
  `Init` alone). Polean's threat differs: it regenerates its modules from data
  rather than judging hostile Lean. SafeVerify shows the replay-then-compare
  shape and has JSON output, but it is an external dependency and no Lean
  4.35.0 support was established. See `Interfaces and dependencies`.
  Date/Author: 2026-11-05, plan author.
- D3. Decision: an expected proposition is a ground `const`/`app` tree in the
  request and is compared with `Expr.equal`. Rationale: the alternatives are
  worse. A named declaration in the job module lets a faulty job define its own
  target. Source text would put data in a Lean executable position. Comparing
  rendered strings is defeated by notation. Comparing up to definitional
  equality accepts more than was asked, and `Fx.Unfolded` shows the difference.
  The probe confirmed that a claim applied to a list literal is already such a
  tree. Date/Author: 2026-11-05, plan author.
- D4. Decision: every constant in the closure of a root's statement must
  originate in the toolchain or a module the request names as trusted.
  Rationale: without it, a `prelude` module can define `Fx.Goal` itself, and a
  structurally equal statement would then mean something else. This is the
  check comparator performs by comparing statement declarations across
  environments, at a far smaller cost. Date/Author: 2026-11-05, plan author.
- D5. Decision: origin is established by where a module's `.olean` was found,
  not by its name. A toolchain module is one loaded from under `LEAN_SYSROOT`.
  Rationale: a scratch `Init/Prelude.olean` otherwise forges a toolchain
  origin. Date/Author: 2026-11-05, plan author.
- D6. Decision: replay every non-toolchain module and walk the environment
  replay returns. Rationale: the audit then inspects what the kernel accepted,
  not a second copy. Date/Author: 2026-11-05, plan author.
- D7. Decision: a request may narrow the foundations but not widen them. The
  pinned table of three name and origin pairs lives in the Rust kernel.
  Rationale: a request that approves `sorryAx` would make the audit
  meaningless, and technical design §6.2 calls for an explicit pinned set.
  Date/Author: 2026-11-05, plan author.
- D8. Decision: put the Rust spike in the existing `toolchain` test binary.
  Rationale: it inherits the helpers, the `smoke::` filter behind
  `make toolchain-smoke`, and the CI path filter, with no new binary, Make
  target, or `dead_code` difficulty. Date/Author: 2026-11-05, plan author.
- D9. Decision: record the outcome in ADR 0006, with no separate feasibility
  record. Rationale: 1.2.2's record catalogued 106 constructs. This corpus is
  small enough for the manifest and snapshot to carry it, and project guidance
  favours one ADR plus small amendments. Date/Author: 2026-11-05, plan author.
- D10. Decision: use Verus for settlement only. The Lean worker and JSON
  adapter are covered by the end-to-end corpus, property tests, and seeded
  mutations. Kani is not used. Rationale: see `Verification plan`. Date/Author:
  2026-11-05, plan author.

## Outcomes & retrospective

Nothing has been implemented. Complete this section at each milestone and at
the end. Before setting `COMPLETE`, reconcile every discovery with the upstream
artefacts in `Conformance basis`.

## Context and orientation

Polean is a design scaffold. `src/lib.rs` holds a project name and a status
string. Later roadmap items build a Rust orchestrator around a pinned Lean
proof package. The policy under check is expressed as data, and generated Lean
modules contain only enum and list literals inside fixed templates.

Some terms recur throughout this plan.

- A **declaration** is a named entry in a Lean environment, such as a theorem
  or an axiom. A **theorem root** is a declaration the audit is asked about.
- A **statement** is a theorem's type: the proposition it proves.
- An **axiom** is a declaration accepted without proof. Its **transitive
  closure** is every axiom reachable by following constants through types and
  values.
- A **foundation** is one of `propext`, `Quot.sound`, and `Classical.choice`,
  the three axioms of Lean's logic that ADR 0002 trusts.
- The **kernel** is Lean's small trusted checker. **Replay** sends stored
  declarations through the kernel again, as `leanchecker` does.
- An **`.olean` file** is Lean's compiled module format. Loading one maps it
  into memory, so a crafted file is an attack on the loader itself.
- **Settlement** is the Rust decision over a request and a report. Its three
  outcomes are `accepted`, `rejected`, and `tool_failure`.

The relevant files are these. `lean/` is a Lake package whose only library,
`PoleanSmoke`, is a compile-time probe. `lean/lakefile.toml` sets
`warningAsError = true`, so fixtures with `sorry` cannot be built there.
`toolchain-pins.json` records the Lean archive, which
`scripts/provision_tools.py` installs at `.polean/tools/lean/<sha256>/` after a
digest check. `make tools` provisions it. `make toolchain-smoke` provisions and
then runs the `#[ignore]` tests matched by `smoke::` in the `toolchain` test
binary (`tests/toolchain/main.rs`). `tests/toolchain/smoke/mod.rs` shows how a
tool is run with `env_clear()`, and how an absent tool fails rather than
skipping.

`contracts/` holds the JSON Schema contracts. `validation/validate_design.py`
discovers every `contracts/*.schema.json` through `load_schemas` and records
`json_schemas_checked` in `validation/design-validation.json`. `SHA256SUMS`
lists the design pack. No gate checks it, but plans refresh it by hand.

`verus/admission_recognizer.rs` shows the proof pattern. A kernel written once
under `tests/admission/recognizer/kernel/` is compiled by Cargo as test code
and pulled into Verus with `#[path]`, so the executed and verified code cannot
drift. `scripts/check_verus_controls.py` requires Verus to reject seeded faults.
`make verus` runs both, and is on demand rather than a commit gate.

Consult these before starting:

- `docs/technical-design.md` §§2, 5, 6.2, 8, 10, and 13.
- `docs/terms-of-reference.md`, `docs/context.md`, and
  `docs/developers-guide.md`.
- ADR 0002 for the trust statement and ADR 0003 "Inputs for 1.2.3".
- `docs/reliable-testing-in-rust-via-dependency-injection.md`,
  `docs/rust-testing-with-rstest-fixtures.md`, and
  `docs/rust-doctest-dry-guide.md`.
- `docs/complexity-antipatterns-and-refactoring-strategies.md`.
- `docs/scripting-standards.md` for the changes to `scripts/`.
- `docs/documentation-style-guide.md` for the ADR and for assurance language.

Load the skills `rust-router`, then `rust-unit-testing`, `rust-errors`,
`proptest`, and `verus` as each facet arises. Use `codegraph-mcp` for
navigation, `execplans` for this document, `en-gb-oxendict-style` for prose,
`arch-decision-records` for the ADR, `addressing-whitaker-findings` for lint
findings, and `sdlc-implementation` to drive the milestones.

## Conformance basis

Upstream artefacts at commit `de8a85ece8340fb7ab57794b405c16fe86db2bee`:

- `docs/terms-of-reference.md`: goals G5 and G6, success criterion SC6,
  assumption A6.
- `docs/technical-design.md`, draft v0.1: §2 (the IR-to-theorem trust row), §5
  (no `native_decide`, no generated axioms), §6 (`lean/Polean/Audit.lean`),
  §6.2, §8 (outcomes), §10 (V4 and V8), and §13 (the axiom-audit row).
- `docs/roadmap.md`, item 1.2.3, whose success criterion is quoted in `Purpose`.
- ADR 0002 item 2 (what the proof trusts) and ADR 0003 "Inputs for 1.2.3".
- `contracts/cli.txt`: exit 6 and exit 7, unchanged by this work.

Trace links:

```plaintext
1.2.3 "pinned audit interface"
  -> TD §6.2, ADR 0003 tuple -> polean-audit, audit contracts -> EP-M3
  -> smoke::audit::corpus_matches_manifest
1.2.3 "accepts an expected proposition and approved foundations"
  -> TD §6.2 -> S1 -> EP-M1, EP-M2
  -> AcceptedGround, AcceptedFoundations, lemma_witness_accepted
1.2.3 "rejects sorryAx"
  -> TD §6.2, ADR 0002 item 2 -> S3 -> EP-M2, EP-M3 -> Sorry, SorryImported
1.2.3 "native-computation trust axioms"
  -> TD §5, ADR 0003 input 1 -> S3 -> EP-M2, EP-M3 -> Native
1.2.3 "undeclared assumptions"
  -> TD §6.2 -> S4, W2 -> EP-M2, EP-M3 -> Undeclared, Reach*, NarrowedFoundations
1.2.3 "machine-readable results"
  -> TD §6.2, §8 -> contracts/audit-report.schema.json -> EP-M1
  -> design-check, wire tests
TD §6.2 "expected declaration type"
  -> W4, W5 -> EP-M3 -> WrongStatement, Unfolded, AxiomRoot, DefRoot
TD §6.2 "exact requested claim coverage"
  -> S2 -> EP-M1, EP-M2 -> coverage unit tests, settle proof
TD §6.2 "malformed, or contradictory worker output is a tool failure"
  -> S2, W1 -> EP-M1 -> wire tests, feature scenario
TD §2 "kernel acceptance" -> W3 -> EP-M3 -> EnvHack, HackImported
```

## Verification plan

### What is being claimed

Three layers with different evidence meet at the JSON report.

1. **Settlement** is a pure function, so Verus can prove it for all inputs.
2. **The wire adapter** is strict JSON decoding, tested by unit and property
   tests.
3. **The worker** is Lean `IO` code over toolchain internals. It carries no
   proof. It is tested against a corpus at the real interface, with seeded
   mutations showing the corpus detects faults.

Verus proves settlement correct given a truthful report. Nothing proves the
report truthful: that rests on the corpus and the axioms below. Do not describe
the audit as verified. Describe the settlement kernel as verified and the
worker as tested.

### Abstract model first

Define these specifications in `verus/audit_settlement_spec.rs`, independently
of the executable kernel. They are stated here so that they can test the API
before any code exists. Names are compared through an injective byte key
(`Interfaces and dependencies`), so the model treats a name as an opaque byte
sequence.

```text
// The pinned table: exactly three (name, origin module) pairs.
pinned_foundation(n, m) :=
  (n, m) in { (propext, Init.Core), (Quot.sound, Init.Prelude),
              (Classical.choice, Init.Prelude) }

valid_request(q) :=
  q.roots is non-empty
  and root ids and root names are each pairwise distinct
  and every name in q.foundations has a pinned_foundation entry
  and q.module is not in q.trusted_modules

approved(q, a) :=
  a.name in q.foundations and a.toolchain and pinned_foundation(a.name, a.module)

origin_permitted(q, o) := o.toolchain or o.module in q.trusted_modules

root_clean(q, r) :=
  r.found and r.kind == theorem and r.statement_match
  and forall o in r.statement_origins. origin_permitted(q, o)
  and forall a in r.axioms. approved(q, a)

protocol_ok(q, p, pin) :=
  p.toolchain == pin and p.module == q.module
  and no missing constants are reported on any root
  and (p.replay is rejected  implies p.roots is empty)
  and (p.replay is accepted  implies
         |p.roots| == |q.roots|
         and forall i. p.roots[i] answers q.roots[i] by id and name)

acceptable(q, p, pin) :=
  protocol_ok(q, p, pin) and p.replay is accepted
  and forall r in p.roots. root_clean(q, r)
```

Preconditions: `valid_request(q)`. `settle` has none on the report: every input
is total, because a malformed report must produce an outcome rather than an
error.

### Settlement obligations (Verus)

These are hypotheses about the eventual proof structure. Implementation may
split or refine them. Each must end up discharged from these premises with no
trusted body.

- S1 `settle_accepts_exactly`: under `valid_request(q)`, `settle(q, p, pin)` is
  `Accepted` if and only if `acceptable(q, p, pin)`. This is the main theorem.
  Soundness alone is met by a kernel that never accepts, so completeness is
  part of the statement.
- S2 `settle_fails_exactly`: under `valid_request(q)`, `settle(q, p, pin)` is
  `ToolFailure` if and only if not `protocol_ok(q, p, pin)`. A protocol breach
  therefore always takes precedence over findings. It is never reported as a
  policy rejection.
- S3 `forbidden_never_accepted`: under `valid_request(q)`, if any root reports
  an axiom named `sorryAx`, or with a `_native` component, then `settle` is not
  `Accepted`. This follows from S1 together with the fact that no pinned name is
  `sorryAx` or has such a component, which Verus must establish from the table
  rather than assume.
- S4 `undeclared_never_accepted`: under `valid_request(q)`, if any root reports
  an axiom absent from `q.foundations`, or a foundation name whose origin does
  not match its pinned entry, `settle` is not `Accepted`, whatever the name
  looks like.
- S5 `rejection_explains`: when `settle` is `Rejected`, the findings are
  non-empty and each names a requested root, or the replay. No clean root gets
  a finding, so a mixed report keeps its per-root evidence (technical design
  §8).
- S6 `valid_request_excludes_widening`: `validate_request` returns `Ok` if and
  only if `valid_request(q)`, so a request approving `sorryAx`, a `_native`
  name, or an unpinned axiom is refused before any report is considered.
- S7 non-vacuity witnesses, proved on concrete values: `lemma_witness_accepted`
  shows a valid request and report with one root and all three foundations
  settling `Accepted`, so the antecedent of S1 is inhabited.
  `lemma_witness_rejected` and `lemma_witness_tool_failure` show the other two
  outcomes are reachable.

Non-vacuity for the proof as a whole: `scripts/check_verus_controls.py` gains
controls that must each fail at a named obligation.

- Body controls drop the origin comparison from `approved`, stop the coverage
  loop one root early, and report a protocol breach as a rejection.
- Specification controls add `sorryAx` to the pinned table, and weaken
  `origin_permitted` to `true`.

Evidence: `make verus` prints `verification results:: N verified, 0 errors`,
followed by every control rejected. The proof depends on no axiom beyond Verus
and its standard library. Grepping `verus/audit_*` for `assume`, `admit`, and
`external_body` finds nothing. Discharge condition: `make verus` exits 0.

Solver plumbing, trigger selection, and auxiliary sequence lemmas are
deliberately left until implementation exposes the actual verification
conditions.

### Worker obligations (tests at the real interface)

Each is exercised by `smoke::audit::corpus_matches_manifest`, which compiles
the corpus with the pinned `lean`, runs the real worker, settles, and compares
the outcome and findings with the hand-written manifest. Manifest expectations
are written before the worker exists, and are never generated from its output.

- W1 Process protocol. The worker prints exactly one JSON document and exits 0
  if and only if it completed an audit. Otherwise it exits non-zero with
  nothing on standard output. Artefact: `ModuleAbsent` and `InvalidRequest`
  runs, plus adapter tests. Non-vacuity: `ModuleAbsent` exits 1 in the probe.
- W2 Axiom closure. The reported axioms are exactly those reachable through
  types and values, across modules, and through private helpers. Artefact: one
  `Reach*` fixture for each traversal arm (definition value, definition type,
  opaque value, constructor type, theorem body in an imported module), plus
  `PrivateHelper`. The worker also compares its walk with `Lean.collectAxioms`
  and exits non-zero on disagreement. Non-vacuity: seeded mutation M1 below.
- W3 Replay. An accepted replay means the kernel accepted every declaration in
  each non-toolchain module of the import closure. Artefact: `EnvHack`,
  `HackImported`. Non-vacuity: seeded mutation M2 below.
- W4 Statement exactness. `statement_match` is true if and only if the root's
  type is structurally equal to the expected tree. Artefact: `AcceptedGround`,
  `AcceptedClaimShape`, `WrongStatement`, `WrongPolicyLiteral`, `Unfolded`, and
  `WrongLevel`. Non-vacuity: seeded mutation M3 below.
- W5 Kind and existence. `AxiomRoot` and `DefRoot` give `wrong_kind`.
  `RootAbsent` gives `root_absent`.
- W6 Origin. Reported origins distinguish toolchain modules by the location of
  the loaded file. Artefact: `PreludeForge`, `StatementForge`, and
  `InitShadow`. Non-vacuity: seeded mutation M4 below.
- W7 No execution. Auditing `InitSentinel` leaves no sentinel file, while the
  fixture's `initialize` block would write one if run. Non-vacuity: the fixture
  is accepted, so the module really was inspected, and linking without
  interpreter support removes the mechanism for running it.

Seeded mutations are applied once to a scratch copy of the worker. These
results are author-reported in `Outcomes & retrospective`; nothing in the gates
reproduces them.

- M1: skip definition values in the walk, and require `Reach*` to fail.
- M2: remove replay, and require `EnvHack` and `HackImported` to fail.
- M3: compare only the statement's head constant instead of using `Expr.equal`,
  and require `WrongPolicyLiteral` to fail.
- M4: classify toolchain modules by name prefix, and require `InitShadow` to
  fail.

### Adapter obligations (property and unit tests)

- A1 Key injectivity: distinct names give distinct byte keys, across components,
  including the regression pair `["Quot.sound"]` and `["Quot", "sound"]`, and
  string versus numeric components. Method: `proptest` over generated component
  lists, plus named cases.
- A2 Strict decoding: unknown fields, duplicate keys, wrong JSON types,
  trailing content, an unknown `schema` value, and oversized output are all
  `tool_failure`. Method: `rstest` tables, with `proptest` mutating a valid
  report (deleting, renaming, and retyping fields) and asserting the outcome is
  never `Accepted` for any mutation that alters a checked field.
- A3 Settlement over generated reports agrees with an independently written
  test oracle. This does not duplicate S1. It exercises the adapter and the
  compiled binary on the host, which Verus does not. Generator acceptance is
  measured, and each of the three outcomes must be reached at least once.

Why not other methods: Kani would bound what Verus already proves without a
bound, and it cannot look into Lean. A state-machine model checker has nothing
to explore, as settlement has no state and the worker has no concurrency this
plan owns. A Lean proof of the worker would need a model of `Environment` that
the toolchain does not expose.

### Axioms relied upon

These are taken as given and not verified.

- The Lean kernel accepts only well-typed declarations, and
  `Kernel.Environment.replay` sends declarations to it as `leanchecker` does.
- `Expr.getUsedConstants` returns every constant in an expression, and
  `Expr.equal` decides structural equality.
- Toolchain `.olean` files under the digest-checked install match the archive.
  They are not replayed.
- Loading an `.olean` file does not run its initializers unless the host
  enables that. `InitSentinel` exercises this at the interface.
- Generated modules are compiled freshly by the pinned `lean` and are not
  crafted binary files. This is the boundary technical design §7 already draws.
- `serde_json`, with derived `deny_unknown_fields` decoding, rejects duplicate
  and unknown fields. Tests exercise this at the interface.
- Verus, Z3, and the release pins in `tools/verus/` are sound.

Residual gaps: no theorem connects the worker's report to the environment. The
audit trusts the toolchain library rather than replaying it. It says nothing
about what a statement means, since that is the definition of the Polean atoms
(ADR 0002 item 2).

## Plan of work

### Stage A: propose (done in this plan)

No repository code changes. Reconnaissance, research, and probes are recorded
above, and the panel's revisions are in `Revision notes`.

### Stage B: red

Write the contracts, the manifest, and the feature file first. Add the kernel's
types with a `settle` that returns `ToolFailure` for everything, so unit tests
compile and fail for the expected reason. Add the provisioned corpus test,
which fails because `polean-audit` is absent.

### Stage C: implement and verify together

Write the specification in `verus/` before the kernel body, and develop each
loop with its invariant. Implement the worker after settlement is proved, so
the proved contract pressure-tests the report format while it is still cheap to
change.

### Stage D: refactor, document, and validate

Remove the scaffolding, reconcile the design, record outcomes, and run all
gates.

## Milestones and plateaus

Each milestone runs `make check-fmt`, `make lint`, and `make test`,
sequentially, with output teed to
`/tmp/$ACTION-polean-1-2-3-demonstrate-type-and-axiom-auditing.out`. Commit
after each gated change.

### EP-M1: contracts, corpus manifest, and settlement kernel

Outcome: a contract and a tested pure kernel exist, and no Lean code has
changed. Any consumer can decide a recorded report.

Work: add `contracts/audit-request.schema.json` and
`contracts/audit-report.schema.json`. Add `examples/audit/sample-request.json`,
`examples/audit/sample-report.json`, and `examples/audit/manifest.json`, the
last being JSON Lines with one hand-written line per fixture. Extend
`validation/validate_design.py` with `check_audit_contracts`, holding the
samples and negative controls. Add `serde` to `Cargo.toml`. Add
`tests/toolchain/audit/`: `mod.rs`, `wire/` (decoding and key encoding),
`settlement/kernel/` (the pure kernel), `properties/`, `tests.rs`, and
`bdd.rs`, with the feature in `tests/features/axiom_audit.feature`.

Requirements: machine-readable results, malformed output, coverage.

Acceptance: `make test` runs the kernel and wire tests, each red before its
implementation. `make design-check` reports `json_schemas_checked: 7`.

Conformance check:

- The report has no verdict field.
- No item is added to `src/`.
- The only new dependency is the authorized `serde`.
- The four claims stay separate in every name.
- `SHA256SUMS` is refreshed for contracts and validation outputs.

Recovery: revert the milestone's commits. Nothing depends on them yet.

Remaining gaps: the kernel is tested but not proved, and no real report exists.

Compatibility decision: none. These are new contracts with no consumer.

### EP-M2: Verus proof of settlement

Outcome: S1–S7 are discharged for the same source Cargo compiles.

Work: add `verus/audit_settlement_spec.rs`, `verus/audit_settlement_lemmas.rs`,
and `verus/audit_settlement.rs`, which pulls in the kernel through `#[path]`.
Change the Makefile so `verus-proof` runs a list of entry points
(`VERUS_PROOF_FILES`, both files), keeping the recognizer proof unchanged.
Generalize `scripts/check_verus_controls.py` from one hard-coded kernel
directory and proof file to a list of proof trees, adding the controls in
`Verification plan`, and update `scripts/tests/`.

Requirements: accepts, rejects, undeclared, exact coverage.

Acceptance: `make verus` exits 0, with both proofs reporting `0 errors` and
every control rejected at its named obligation. `make test-scripts` passes. Then
`make check-fmt`, `make lint`, and `make test`.

Conformance check:

- No trusted proof body exists.
- The specification does not reference the kernel.
- The recognizer's verified count is unchanged.
- `make verus` stays on demand, and outside `make all` and CI.

Recovery: the kernel is unchanged in behaviour by this milestone, so reverting
the proof leaves EP-M1 valid. If the verification tolerance is reached, stop
and present options.

Remaining gaps: the report's truthfulness is not addressed by this proof.

Compatibility decision: none.

### EP-M3: Lean worker and end-to-end corpus

Outcome: `make toolchain-smoke` runs the real worker over the whole corpus, and
each settlement matches the manifest.

Work: run `make tools` in this worktree. Add `lean/Polean/Audit.lean`. Add
`[[lean_exe]]` named `polean-audit` with root `Polean.Audit` to
`lean/lakefile.toml`, with no `supportInterpreter`, and add it to
`defaultTargets`. Add the fixtures under `examples/audit/lean/AuditFixture/`.
Add `tests/toolchain/smoke/audit/`, which builds the worker with
`lake --dir lean build polean-audit`, compiles fixtures with `lean -o` into a
scratch `LEAN_PATH`, runs the worker with only `LEAN_SYSROOT` and `LEAN_PATH`
set, and bounds standard output at 1 MiB. Add `insta` snapshots: one line per
fixture for the settlement, and the canonical report for `AcceptedFoundations`
and `Native`. Add `contracts/audit-*` and `examples/audit/**` to
`.github/workflows/toolchain.yml`. Apply the seeded mutations M1–M4 to a
scratch copy and record the results.

The corpus is hand-written and each line has the expected outcome below.
`Finding` and reason names are contractual.

| Fixture               | Purpose                                      | Settlement                                   |
| --------------------- | -------------------------------------------- | -------------------------------------------- |
| `AcceptedGround`      | Honest theorem with no axiom                 | `accepted`                                   |
| `AcceptedFoundations` | All three foundations with pinned origins    | `accepted`                                   |
| `AcceptedClaimShape`  | Claim applied to a nested list literal       | `accepted`                                   |
| `InitSentinel`        | `initialize` writes a sentinel if ever run   | `accepted`, and no sentinel file             |
| `MixedRoots`          | One clean root and one with `sorry`          | `rejected`: `sorry_axiom` on one root only   |
| `Sorry`               | Direct `sorry`                               | `rejected`: `sorry_axiom`                    |
| `SorryImported`       | `sorry` behind an imported helper            | `rejected`: `sorry_axiom`                    |
| `SorryMacro`          | `sorry` hidden behind a macro                | `rejected`: `sorry_axiom`                    |
| `Native`              | `native_decide` in a helper                  | `rejected`: `native_axiom`                   |
| `Undeclared`          | A direct user axiom                          | `rejected`: `undeclared_axiom`               |
| `ReachDefValue`       | Axiom reached through a definition value     | `rejected`: `undeclared_axiom`               |
| `ReachDefType`        | Axiom reached through a definition type      | `rejected`: `undeclared_axiom`               |
| `ReachOpaque`         | Axiom reached through an opaque value        | `rejected`: `undeclared_axiom`               |
| `ReachCtorType`       | Axiom reached through a constructor type     | `rejected`: `undeclared_axiom`               |
| `ReachImported`       | Axiom in an imported module's theorem        | `rejected`: `undeclared_axiom`               |
| `PrivateHelper`       | Axiom behind a private helper in a `module`  | `rejected`: `undeclared_axiom`               |
| `SpoofedName`         | `axiom «Quot.sound»`                         | `rejected`: `undeclared_axiom`               |
| `NarrowedFoundations` | Uses foundations the request does not list   | `rejected`: `undeclared_axiom`               |
| `PreludeForge`        | `prelude` module with its own `propext`      | `rejected`: `forged_foundation`, plus origin |
| `InitShadow`          | Scratch `Init/Prelude` forging `Quot.sound`  | `rejected`: `forged_foundation`, plus origin |
| `StatementForge`      | `prelude` module defining the expected name  | `rejected`: `statement_origin` only          |
| `WrongStatement`      | Different proposition under the right name   | `rejected`: `statement_mismatch`             |
| `WrongPolicyLiteral`  | Same claim, about a different list literal   | `rejected`: `statement_mismatch`             |
| `Unfolded`            | Definitionally equal, structurally different | `rejected`: `statement_mismatch`             |
| `WrongLevel`          | Universe level differs from the request      | `rejected`: `statement_mismatch`             |
| `AxiomRoot`           | Root is itself an axiom                      | `rejected`: `wrong_kind`, `undeclared_axiom` |
| `DefRoot`             | Root is a definition                         | `rejected`: `wrong_kind`                     |
| `RootAbsent`          | Named root does not exist                    | `rejected`: `root_absent`                    |
| `EnvHack`             | Forgery through `debug.skipKernelTC`         | `rejected`: `kernel_rejected`                |
| `HackImported`        | The same forgery in an imported module       | `rejected`: `kernel_rejected`                |
| `ModuleAbsent`        | Module file does not exist                   | `tool_failure`: `worker_failed`              |
| `InvalidRequest`      | Foundation name outside the pinned table     | Refused before any worker runs               |

_Table 1: The audit corpus and the settlement each entry must produce._

Requirements: pinned interface, all rejection classes, no execution of
submitted code.

Acceptance: `make toolchain-smoke` passes, including the existing OPA and Lean
smoke tests. `smoke::audit::corpus_matches_manifest` fails before the worker
exists. Then `make check-fmt`, `make lint`, and `make test`.

Conformance check:

- The request stays data.
- The worker emits no verdict.
- Fixtures stay outside the Lake package.
- `make test` needs no provisioned tool.
- `PoleanSmoke` is unchanged.

Recovery: delete `lean/Polean/` and the `[[lean_exe]]` table to return to
EP-M2. If a fixture behaves differently from the table, the design tolerance
applies. Do not edit the manifest to match.

Remaining gaps: promotion to production and generated jobs are 2.2.4. Enforced
isolation is 1.2.4 and 4.3.1. Untrusted evidence is 4.1.3.

Compatibility decision: none.

### EP-M4: documentation, design reconciliation, and roadmap

Outcome: durable documents record the decision, and the roadmap entry is done.

Work:

- Add `docs/adrs/adr-0006-audit-lean-roots-by-observation-and-settlement.md`,
  with the decisions D1–D7, the alternatives, what the audit establishes and
  does not, the measured costs, and the proposed mapping of `rejected` to
  `invalid_evidence` (exit 7) and `tool_failure` to `tool_error` (exit 6),
  which 2.2.4 implements.
- Amend technical design §2 (the IR-to-theorem trust row), §6.2 (observation
  and settlement, replay scope, statement data form, foundation origin), §13
  (the axiom-audit row), and its References. Lean upgrades, comparator, and
  SafeVerify are cited there.
- Add a dated note to ADR 0003 resolving its 1.2.3 inputs and its `bwrap` open
  item.
- Update `docs/developers-guide.md` (what the audit is, how to add a fixture,
  how to read a report, `make verus` covering two proofs),
  `docs/repository-layout.md`, `lean/README.md`, `validation/README.md` (seven
  schemas), `tools/verus/README.md`, `docs/contents.md`, and `docs/context.md`
  if a new term needs defining.
- Refresh `SHA256SUMS` after reviewing the drift.
- Mark roadmap 1.2.3 as done, with a `Recorded:` bullet stating what ran.

`docs/users-guide.md` does not change, because no library or command behaviour
changes: there is still no executable checker. Record that decision here rather
than adding text about an internal tool.

Acceptance: `make markdownlint`, `make design-check`, and
`sha256sum -c SHA256SUMS` pass. All gates in `Validation and acceptance` pass.
Then set `Status: COMPLETE`.

Conformance check: every discovery in `Surprises & discoveries` is reconciled
upstream.

Recovery: documentation commits are independently revertible.

Remaining gaps: as listed under EP-M3.

Compatibility decision: none.

## Concrete steps

Run everything from the worktree root. Tee long output, for example:

```bash
make check-fmt 2>&1 | tee /tmp/check-fmt-polean-1-2-3-demonstrate-type-and-axiom-auditing.out
make lint 2>&1 | tee /tmp/lint-polean-1-2-3-demonstrate-type-and-axiom-auditing.out
make test 2>&1 | tee /tmp/test-polean-1-2-3-demonstrate-type-and-axiom-auditing.out
```

Python 3.14 is needed by `make tools`, `make test-scripts`, and
`make toolchain-smoke`. On the development host, select it with
`PYTHON="uv run --python 3.14 --with pytest==9.1.1 python"` as
`docs/developers-guide.md` describes, or an equivalent.

Provision the tools once, and then run the proof or the corpus:

```bash
make tools
make install-verus
make verus 2>&1 | tee /tmp/verus-polean-1-2-3-demonstrate-type-and-axiom-auditing.out
make toolchain-smoke 2>&1 | tee /tmp/smoke-polean-1-2-3-demonstrate-type-and-axiom-auditing.out
```

Expected tail of the proof run:

```plaintext
verification results:: N verified, 0 errors
```

To run the worker by hand on a compiled fixture directory:

```bash
env -i LEAN_SYSROOT="$PWD/.polean/tools/lean/$SHA" LEAN_PATH="$SCRATCH" \
  lean/.lake/build/bin/polean-audit "$SCRATCH/request.json"
```

## Validation and acceptance

The behavioural specification is `tests/features/axiom_audit.feature`:

```gherkin
Feature: Settle a Lean audit report against an expected proposition and approved foundations

  Background:
    Given a request for one theorem root with its expected statement and all pinned foundations

  Scenario: A clean report is accepted
    Given a report whose root is a theorem with the expected statement and only approved axioms
    When the report is settled
    Then the settlement is accepted

  Scenario: A root that depends on sorryAx is rejected
    Given a report whose root depends on sorryAx
    When the report is settled
    Then the settlement is rejected with finding sorry_axiom

  Scenario: A root that depends on a native computation axiom is rejected
    Given a report whose root depends on an axiom with a _native name component
    When the report is settled
    Then the settlement is rejected with finding native_axiom

  Scenario: A root that depends on an undeclared assumption is rejected
    Given a report whose root depends on an axiom outside the request
    When the report is settled
    Then the settlement is rejected with finding undeclared_axiom

  Scenario: A foundation name from the wrong origin is rejected
    Given a report whose root depends on propext from a module outside the toolchain
    When the report is settled
    Then the settlement is rejected with finding forged_foundation

  Scenario: A statement that differs from the request is rejected
    Given a report whose root statement does not match
    When the report is settled
    Then the settlement is rejected with finding statement_mismatch

  Scenario: A kernel replay rejection is rejected
    Given a report whose replay was rejected by the kernel
    When the report is settled
    Then the settlement is rejected with finding kernel_rejected

  Scenario: A report that omits a requested root is a tool failure
    Given a report that answers no requested root despite an accepted replay
    When the report is settled
    Then the settlement is a tool failure

  Scenario: Garbled worker output is a tool failure
    Given worker output that is not a single audit report
    When the output is decoded and settled
    Then the settlement is a tool failure
```

Red-Green-Refactor evidence to record for each milestone:

- Red: `make test` fails for `audit::` because `settle` returns `ToolFailure`
  for every report; `make toolchain-smoke` fails because `polean-audit` is not
  built.
- Green: the same commands pass, with the corpus matching the manifest.
- Refactor: the same commands, then every gate below.

Testing layers:

- Unit tests with `rstest` and `googletest` or `pretty_assertions` cover each
  finding, the precedence rules, each `validate_request` refusal, and the key
  encoding.
- Behavioural tests with `rstest-bdd` run the feature above against recorded
  reports.
- Property tests with `proptest` cover A1–A3.
- Snapshot tests with `insta` hold the per-fixture settlement table and two
  canonical reports, so format drift is reviewed.
- End-to-end tests are the provisioned corpus at the real process boundary.
- Verus covers S1–S7. Tests continue to cover the adapter, the compiled binary,
  and everything outside the proof.

Quality criteria:

- Tests: `make test` and `make toolchain-smoke` pass, with no new `#[ignore]`
  outside `smoke::`.
- Verification: `make verus` exits 0, with S1–S7 discharged and all controls
  rejected.
- Lint and types: `make check-fmt`, `make typecheck`, and `make lint`, which
  runs Whitaker.
- Scripts and design: `make test-scripts`, `make design-check`,
  `make design-properties`, `make design-boundary`, `make markdownlint`, and
  `sha256sum -c SHA256SUMS`.
- Performance: record worker wall time and peak memory on `AcceptedGround`.
  This is an observation, not a budget claim. SC7 measurement remains 4.3.2.
- Security: the corpus rows for forgery, spoofing, and execution pass. The
  seeded mutations are recorded as author-reported.

## Idempotence and recovery

Every step can be repeated. `make tools` is digest-checked and reuses verified
artefacts. Fixtures compile into fresh scratch directories per test run, so
nothing accumulates under `lean/`. `lean/.lake/` is ignored.
`make design-check` regenerates its outputs deterministically, and `design.yml`
requires them to be clean. If a Verus control's anchor drifts, the script fails
closed: update the anchor deliberately, in the same commit as the source edit.

The fixtures that forge theorems use `import Lean`, which is slow. If the
provisioned test is interrupted, rerun it. It holds no state.

## Artefacts and notes

These observations were made against the pinned archive on 2026-11-05 in a
scratch directory, using a sibling worktree's digest-named install. EP-M3
reproduces them through the committed corpus. All worker runs used a cleared
environment with only `LEAN_SYSROOT` and `LEAN_PATH`.

```plaintext
Good            replay accepted  theorem  axioms []
GoodFoundations replay accepted  theorem  Quot.sound@Init.Prelude Classical.choice@Init.Prelude propext@Init.Core
Sorry           replay accepted  theorem  sorryAx@Init.Prelude
Native          replay accepted  theorem  Fx.Native.helper._native.native_decide.ax_1_1@Fx.Native
Custom          replay accepted  theorem  Fx.Custom.assume@Fx.Custom
Transitive      replay accepted  theorem  Fx.TransitiveBase.hidden@Fx.TransitiveBase
PrivateModule   replay accepted  theorem  _private.Fx.PrivateModule.0.Fx.PrivateModule.secret@Fx.PrivateModule
Spoof           replay accepted  theorem  «Quot.sound»@Fx.Spoof
PreludeForge    replay accepted  theorem  propext@Fx.PreludeForge
AxiomRoot       replay accepted  axiom    Fx.AxiomRoot.root@Fx.AxiomRoot
InitSentinel    replay accepted  theorem  axioms []   (no sentinel written at compile or audit)
EnvHack         replay REJECTED  (kernel) declaration type mismatch, 'Fx.EnvHack.root' has type True
HackUser        replay REJECTED  (kernel) declaration type mismatch, 'Fx.HackBase.forged' has type True
Fx.Nope         exit 1           object file '.../Fx/Nope.olean' of module Fx.Nope does not exist
```

The same fixtures against other interfaces:

```plaintext
leanchecker Fx.EnvHack          rc=1  rejected
leanchecker Fx.HackUser         rc=0  forgery in an import is not replayed
leanchecker --fresh Fx.HackUser rc=1  131.8 s, 1323 MiB
leanchecker --fresh Fx.Good     rc=0  91.0 s, 476 MiB
leanchecker --fresh Fx.PreludeForge rc=0  accepted: a forged foundation is well typed
leanexport Fx.Good              rc=0  36.3 s, 872 MiB, 345,696,313-byte NDJSON
leanchecker --from-export       rc=0  87.0 s, 516 MiB
lake comparator                 rc=2  bwrap missing
lake comparator --inadvisably-no-sandbox  rc=1  error: Illegal axiom detected: 'cheat'
```

Costs of the compiled prototype:

```plaintext
lake build (empty environment, no interpreter support)  15.3 s, binary 118,675,648 bytes
audit, Init-only job                                    about 1 s, about 800 MiB resident
audit, job importing Lean                               about 5 s, about 3.3 GiB resident
without LEAN_SYSROOT                                    exit 1: could not execute external process 'lean'
```

A ground claim statement elaborates with no metadata, confirming D3:

```plaintext
Lean.Expr.app (Lean.Expr.app (Lean.Expr.const `Fx.Holds []) (Lean.Expr.const `Fx.ClaimId.iso []))
  (Lean.Expr.app (Lean.Expr.app (Lean.Expr.app (Lean.Expr.const `List.cons [Lean.Level.zero]) ...
hasMData? false  githash=eb6cafb7290e8bd5f2937522d8d804f58f02bafc  v=4.35.0
```

References inspected on 2026-11-05:

- [Validating a Lean proof](https://lean-lang.org/doc/reference/latest/ValidatingProofs/):
  the escalating checks, the `_native` naming, and `.olean` attack surface.
- [Lean 4.35.0 release notes](https://lean-lang.org/doc/reference/latest/releases/v4.35.0/):
  `lake check` (#14990), `lake comparator` (#14885, #15146), `--paranoid`
  (#15145), and the removal of `Lean.trustCompiler` (#14953).
- [leanprover/comparator](https://github.com/leanprover/comparator): archived
  and merged into core at v4.35.0; its statement-closure comparison and trust
  assumptions.
- [SafeVerify](https://github.com/GasStationManager/SafeVerify): replay
  followed by checks on the replayed environment, and JSON output.

## Interfaces and dependencies

### Why a new worker

| Candidate                 | Statement check | Axioms          | Replay            | Structured result | Runs here       |
| ------------------------- | --------------- | --------------- | ----------------- | ----------------- | --------------- |
| `#print axioms`           | None            | Name, top level | None              | None              | Yes             |
| `leanchecker`             | None            | None            | Named modules     | None              | Yes             |
| `lake check`              | None            | Standard set    | Export            | Exit code         | Needs `bwrap`   |
| `lake comparator`         | Challenge file  | Permitted list  | Export            | Text, exit code   | Needs `bwrap`   |
| SafeVerify                | Target file     | Standard set    | Named modules     | JSON              | Not established |
| `polean-audit` (proposed) | Data tree       | Name and origin | All non-toolchain | JSON              | Yes             |

_Table 2: Existing audit interfaces compared with the proposed worker._

`lake comparator --paranoid` remains the better tool for hostile Lean and could
become an independent second opinion once 1.2.4 settles the sandbox. ADR 0006
records this as a deferred option. It is not a current requirement.

### Request contract

`contracts/audit-request.schema.json`, identifier `polean.audit-request.v0`:

```json
{
  "schema": "polean.audit-request.v0",
  "module": ["AuditFixture", "AcceptedGround"],
  "trusted_modules": [["AuditFixture", "Expected"]],
  "roots": [
    {
      "id": "goal",
      "name": ["AuditFixture", "AcceptedGround", "root"],
      "expected": { "const": ["AuditFixture", "Goal"], "levels": [] }
    }
  ],
  "foundations": [["propext"], ["Quot", "sound"], ["Classical", "choice"]]
}
```

A statement tree is exactly `{"const": Name, "levels": [Nat]}` or
`{"app": [Tree, Tree]}`. It has no binder, literal, or variable form, and the
schema and worker both refuse anything else. The worker bounds a tree at 4,096
nodes and depth 128. A maximal 16-rule, 16-atom policy literal is about 2,000
nodes at a depth under 64, and a bound exceeded is an invalid request. A `Name`
is a non-empty array whose components are non-empty strings; requests never
contain numeric components. Levels are small natural numbers, since a ground
term needs nothing more.

### Report contract

`contracts/audit-report.schema.json`, identifier `polean.audit-report.v0`:

```json
{
  "schema": "polean.audit-report.v0",
  "toolchain": { "version": "4.35.0", "githash": "eb6cafb7290e8bd5f2937522d8d804f58f02bafc" },
  "module": ["AuditFixture", "AcceptedFoundations"],
  "replay": {
    "status": "accepted",
    "modules": [["AuditFixture", "Expected"], ["AuditFixture", "AcceptedFoundations"]],
    "constants": 4
  },
  "roots": [
    {
      "id": "goal",
      "name": ["AuditFixture", "AcceptedFoundations", "root"],
      "found": true,
      "kind": "theorem",
      "statement_match": true,
      "statement_origins": [{ "module": ["AuditFixture", "Expected"], "toolchain": false }],
      "axioms": [
        { "name": ["Classical", "choice"], "module": ["Init", "Prelude"], "toolchain": true }
      ],
      "missing_constants": []
    }
  ]
}
```

`replay.status` is `accepted` or `rejected`. A rejected replay adds
`declaration` and a bounded `message`, and has `roots: []`. `kind` is one of
`theorem`, `axiom`, `definition`, `opaque`, `quotient`, `constructor`,
`recursor`, and `inductive`, or `null` when `found` is false. In a report name,
a component is a string or a natural number, because private names contain
numbers. `axioms` and `statement_origins` are sorted and contain no duplicates.
There is deliberately no `accepted`, `verdict`, or `ok` field.

### The Lean worker

`lean/Polean/Audit.lean` provides the `polean-audit` executable, whose single
argument is a path to a request. It imports `Lean` and `Lean.Replay`. The
sequence is:

1. Decode the request into `Name` and `Expr` values, refusing out-of-grammar
   data.
2. Import the job module with `withImportModules`, reading module names and
   imports from the header.
3. Classify each module as toolchain if `findOLean` resolves it under
   `LEAN_SYSROOT`.
4. Import the toolchain modules that non-toolchain modules directly import, as
   the replay base.
5. Call `Kernel.Environment.replay` on all constants from non-toolchain modules.
6. For each root, read the constant from the replayed environment, compare its
   type with `Expr.equal`, and walk types and values to collect axioms and
   statement origins, recording any missing constant. Compare the axiom set with
   `Lean.collectAxioms`.
7. Print one compact JSON line, and exit 0.

Any failure before step 7 goes to standard error and exits non-zero, and
standard output stays empty. A replay rejection is a successful audit, not a
failure of the worker.

### The Rust settlement kernel

In `tests/toolchain/audit/settlement/kernel/mod.rs`, pure and with no `serde`,
I/O, or Lean. A name is a `Vec<u8>` holding the length-prefixed, tag-prefixed
encoding of its components, built by `wire::name_key`:

```rust
pub struct Foundation<'a> { pub name: &'a [u8], pub module: &'a [u8] }

pub struct Request { /* module, trusted_modules, roots, foundations as name keys */ }
pub struct Report { /* toolchain, module, replay, roots, as name keys and closed enums */ }

pub enum Settlement { Accepted, Rejected(Vec<Finding>), ToolFailure(ToolFailure) }

pub struct Finding { pub root: Option<usize>, pub code: FindingCode }

pub enum FindingCode {
    KernelRejected, RootAbsent, WrongKind, StatementMismatch, StatementOrigin,
    SorryAxiom, NativeAxiom, ForgedFoundation, UndeclaredAxiom,
}

pub enum ToolFailure {
    ToolchainMismatch, ModuleMismatch, CoverageMismatch, MissingConstant, RejectedReplayWithRoots,
}

pub fn validate_request(request: &Request) -> Result<(), RequestRefusal>;
pub fn settle(request: &Request, report: &Report, pin: &[u8]) -> Settlement;
```

`FindingCode` selects a diagnostic only: acceptance rests on `approved`, so a
defect in classification cannot admit an axiom. The worker-level failure
`worker_failed` and decode failures arise in `tests/toolchain/audit/wire/` and
the smoke runner, and are the same `tool_failure` outcome. Promotion into
`src/lean.rs` is 2.2.4's work. Its exit-status mapping is proposed in ADR 0006.

### Dependencies

`serde` `=1.0.229` with `derive`, under `[dev-dependencies]`, gives derived
decoding with `deny_unknown_fields`, which rejects duplicate and unknown fields.
`serde_json::Value` would silently keep the last duplicate. No Lake package is
added. The worker uses only the pinned Lean's own `Lean` library.

## Revision notes

- Revision 1 (2026-11-05): initial draft, from reconnaissance, research, and
  probes of the pinned toolchain.
