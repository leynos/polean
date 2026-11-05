# Demonstrate declaration-type and transitive-axiom auditing

This ExecPlan is a living document. Keep `Constraints`, `Tolerances`, `Risks`,
`Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan`
current as work proceeds.

Status: DRAFT (revision 3). The user accepted the design decisions in
`Accepted design deviations` and D10 on 2026-11-05 (D13). Implementation must
not begin until the user explicitly approves this plan.

Roadmap item: [1.2.3](../roadmap.md). Branch:
`1-2-3-demonstrate-type-and-axiom-auditing`.

## Purpose / big picture

Polean's first public claim is that Lean checked a model theorem. A compiled
Lean file does not establish that claim. A build succeeds with `sorry`, with
`native_decide`, or with an invented `axiom`. A metaprogram can add a "theorem"
the kernel never saw. A declaration may have the requested name and prove
something else. A module can even redefine a name the kernel accelerates and
derive `False` with no axiom at all. Technical design §6.2 therefore requires
the expected declaration type, exact requested claim coverage, successful
checking, and a machine-readable audit. Nobody has yet shown that the pinned
Lean 4.35.0 makes this possible without executing submitted code.

This task demonstrates it. When the work is done, a contributor runs
`make toolchain-smoke` and watches a small trusted Lean program,
`polean-audit`, inspect a committed corpus of Lean fixtures. It is told which
module and theorem roots to inspect, and returns one JSON report of what it
observed: each root's kind, its statement as a data tree, and every axiom it
depends on with that axiom's origin. A test-only Rust settlement function
compares the report with an audit request, which holds the exact proposition
each root must state and the approved foundational axioms. It decides
`accepted`, `rejected` with named findings, or `tool_failure`.

The expected outcomes are observable. An honest theorem and a theorem that uses
all three foundations are accepted. Fixtures that use `sorry`, `native_decide`,
or an undeclared axiom are rejected, including when the axiom is hidden behind
an imported definition. A root with the wrong statement, or about a different
policy literal, or that is itself an axiom, is rejected for its own named
reason. Forged foundations, forged trusted modules, kernel-accelerated name
forgeries, and theorems that bypassed the kernel are rejected. A garbled report
is a tool failure, never a rejection and never an acceptance. A fixture
containing an `initialize` block shows the audit did not run its code, while a
positive control shows that same block does run when initializers are enabled.

A Verus proof establishes that settlement accepts exactly when the request and
report satisfy an independently written specification, for reports of any
length. A proof of acceptance alone would be satisfied by a function that
rejects everything, in the same way that deny-all satisfies the four safety
claims (technical design §3), so the theorem is a characterization in both
directions.

This establishes one thing: the audit boundary is usable for the pinned tuple.
It is an input to claim 2 in AGENTS.md (the Polean model satisfies a registered
requirement), and proves no such requirement, because no Polean semantics exist
yet. It says nothing about admission (claim 1), translation (claim 3), or
runtime correspondence (claim 4). Promotion into production code belongs to
2.2.4, and applying the audit to untrusted evidence belongs to 4.1.3.

## Constraints

- Keep the four claims in AGENTS.md separate in every test name, diagnostic,
  contract, and document. An `accepted` settlement is not `model_proved`, and
  its result language must not suggest otherwise. Describe settlement as
  verified at a named commit, and the worker as tested. Never describe the
  audit as verified.
- Accept no user Lean, Lake files, compiled proof modules, native libraries, or
  plug-ins as verifier input. The expected proposition is data held by Rust. It
  never reaches Lean. Module and root names reach the worker only as argument
  array entries, which the worker splits on `.` into `Name` values without
  parsing them as Lean source.
- The audit must not execute code from the modules it inspects. The worker
  imports with `loadExts := false`, never enables initializer execution, and
  never calls `evalConst`.
- Do not add an item to `src/` and do not add a binary target to `Cargo.toml`.
  The guard in `tests/documentation_examples/scope.rs` fails when one appears,
  and 2.1.4 owns the application edge. The Rust side of this spike is test-only
  under `tests/toolchain/audit/`, as the 1.2.2 spike was under
  `tests/admission/`.
- The Lean worker is repository-owned trusted code at `lean/PoleanAudit.lean`.
  Adversarial fixtures are compiled into scratch directories and never join the
  Lake package.
- The worker reports observations and carries no verdict and no comparison
  result. Rust settlement is the only acceptance authority. A report cannot
  choose its own outcome by setting a field (technical design §8), and the
  process exit status is not acceptance (technical design §6.2).
- Run tools by explicit digest-named path with a cleared environment and an
  argument array. Do not resolve any tool through `PATH` or elan. The worker
  fixes its own search path and does not consult `LEAN_PATH`.
- Do not change a pin: `toolchain-pins.json`, `lean/lean-toolchain`, the empty
  package list in `lean/lake-manifest.json`, `rust-toolchain.toml`, or the
  Verus pins. Lake stays free of dependencies.
- Do not use `assume`, `admit`, `external_body`, or any other trusted proof
  body in `verus/`. Do not use `sorry`, `native_decide`, or an `axiom` in
  `lean/PoleanAudit.lean`.
- Do not modify `examples/tenant-write/` or `examples/admission/`.
- Keep Rust modules under 400 lines and functions under 70 lines. Every Rust
  module begins with a module-level comment. `std::fs` remains banned in tests;
  use `tests/toolchain/ambient.rs`.

If satisfying the objective requires violating a constraint, do not proceed.
Record the conflict in `Decision log`, set the status to `BLOCKED`, and
escalate.

## Tolerances (exception triggers)

Exclude this ExecPlan from every count below.

- Scope: stop and escalate if the work needs more than 45 non-fixture files or
  about 4,000 net lines of code and other documentation. Lean fixtures under
  `examples/audit/lean/`, reviewed snapshots, and regenerated validation
  outputs do not count. The panel's count of the file list and the recognizer
  precedent, at 3,171 lines of kernel and proof, set these figures.
- Interface: stop if any public item in `src/` or the proposed CLI in
  `contracts/cli.txt` would have to change.
- Dependencies: the only authorized new dependency is `serde`, pinned
  `=1.0.229` with the `derive` feature under `[dev-dependencies]`. It is
  already in `Cargo.lock` at that version and adds no package. Anything else
  needs approval.
- Design: if any fixture in Table 1 cannot be classified as specified by the
  pinned toolchain, or a probe in `Artefacts and notes` does not reproduce,
  stop. Record the deviation, set `BLOCKED`, and wait for acceptance. Do not
  edit the manifest to match the worker.
- Verification: if the settlement theorem cannot be discharged without a
  trusted body, stop and present the options. Do not weaken the specification
  to make it pass, and do not report settlement as proved.
- Iterations: stop after three failed attempts on one root cause.
- Resources: stop if the worker exceeds 30 s or 4 GiB resident on any fixture
  other than `ReplayBomb`. The design's 2 GiB budget is not enforced by this
  spike, but a large excess would falsify the feasibility claim.
- Ambiguity: stop and present options when a choice materially changes what a
  settlement means.

## Accepted design deviations

The user accepted these changes to the current design on 2026-11-05 (D13).
EP-M4 records them upstream, and no upstream document is amended before then.

1. Technical design §6 reserves `lean/Polean/Audit.lean`. The worker moves to
   `lean/PoleanAudit.lean`, because it imports `Lean`, and a future umbrella
   import of `Polean` would pull about 3.3 GiB resident into every job against
   a 2 GiB budget.
2. A root's statement must be a ground tree of constants and applications (D3).
   This constrains 2.2.1 and 2.2.3. Claims must be stated through a named
   predicate such as `Holds claim literal`, rather than the bare `∀ r`
   signatures technical design §5 sketches. Non-empty rule bodies must be
   represented structurally, with no subtype proof inside the literal. The
   named predicate may contain the quantifier in its definition.
3. Toolchain modules are not replayed (D6). The audit trusts toolchain `.olean`
   files unpacked once from a digest-checked archive. This goes beyond the
   "Lean kernel" wording of ADR 0002 item 2, so EP-M4 adds a dated note there.

## Risks

- Risk: structural statement comparison is too brittle for real claims, because
  elaboration can insert metadata or terms the tree cannot express. Severity:
  medium. Likelihood: medium. Mitigation: the probe confirmed that a named
  predicate applied to a nested list literal is a bare `const`/`app` tree with
  no metadata. `AcceptedClaimShape` and `MaximalPolicy` pin that shape, and
  Deviation 2 records the constraint on 2.2.1 and 2.2.3. Out-of-grammar
  statements report `null` and fail closed.
- Risk: replay stops short of what the audit trusts. Severity: high.
  Likelihood: low. Mitigation: replay covers every module not resolved under
  the sysroot, on a base that always contains the sysroot's `Init`.
  `ForgeImported` and `KernelNameForge` are the regressions.
- Risk: the worker hangs or floods its output after a Lean upgrade, and no
  supervisor exists until 2.3.1. Severity: medium. Likelihood: medium.
  Mitigation: a small test helper, `run_bounded`, imposes a deadline with kill
  and reap, and reads at most a fixed number of bytes. `ReplayBomb` exercises
  it. A hang is `tool_failure`, not a stuck gate.
- Risk: "verified" drifts, because a settlement edit can pass CI without Verus
  running. Severity: medium. Likelihood: medium. Mitigation: the source is
  shared through `#[path]`, documents name the commit that was verified, and
  `make verus` is listed in the developers guide's checklist for that
  directory. CI does not run it, matching the recognizer's posture.
- Risk: the Verus proof takes more effort than settlement deserves. Severity:
  medium. Likelihood: medium. The recognizer's specification and lemmas are
  2,252 lines. Literal-slice contents are opaque to the pinned Verus
  (`verus/admission_spec.rs`), and nested forall-exists membership is a likely
  trouble spot. Mitigation: use exec builders with `ensures r@ == seq![...]`,
  one recursive `contains`, a `bytes_eq` helper following `matches_at`, opaque
  per-level predicates as with `trivia`, and ghost model types with `View`. The
  verification tolerance applies otherwise.
- Risk: the provisioned tests grow slow for CI. Severity: low. Likelihood: low.
  Mitigation: every fixture is `Init`-only now that forgeries need no
  `import Lean`. One sequential test runs the corpus. Expect 2–3.5 minutes
  locally under load and 4–7 minutes in CI, against a 30-minute workflow limit.
  Real replay cost at 2.2.4 scale is unmeasured.
- Risk: a reader mistakes the demonstration for protection against hostile
  `.olean` files. Severity: medium. Likelihood: medium. Mitigation: the
  upstream guide says a crafted `.olean` can attack any tool that loads it. ADR
  0006 states that the worker reads only modules freshly compiled from fixed
  templates, and technical design §7 already forbids importing submitted
  `.olean` files.
- Risk: the fixed origin table and worker APIs drift on a Lean upgrade.
  Severity: low. Likelihood: medium. Mitigation: `AcceptedFoundations` and
  `PrivateHelper` fail closed, `warningAsError` turns deprecations into build
  failures, and EP-M4 adds a step to the pin-change procedure in
  `docs/developers-guide.md`.
- Risk: the sibling-worktree toolchain used for the probes is not this
  worktree's gate input. Severity: low. Likelihood: certain. Mitigation: the
  first implementation step runs `make tools` here and repeats the probes
  through the committed fixtures.

## Progress

- [x] (2026-11-05) Renamed the branch and loaded the required skills.
- [x] (2026-11-05) Surveyed the Lean boundary, toolchain, Rust tests, Verus
      tooling, and
  contracts with a Wyvern team.
- [x] (2026-11-05) Researched `lake comparator`, `lake check`, SafeVerify, and
      the Lean
  proof-validation guide.
- [x] (2026-11-05) Probed the pinned Lean 4.35.0 against adversarial fixtures
      and a
  compiled prototype in a scratch directory.
- [x] (2026-11-05) Drafted this plan (revision 1).
- [x] (2026-11-05) Expert panel review, including a live adversarial attack on
      the design.
- [x] (2026-11-05) Reproduced the two attacks that defeated revision 1,
      prototyped the
  fix, and revised this plan (revision 2).
- [x] (2026-11-05) Published draft pull request #35.
- [x] (2026-11-05) The user accepted the three design deviations and keeping the
  Verus milestone (D13).
- [ ] User approval of the plan. Implementation must not start before this.
- [ ] EP-M1: contracts, corpus manifest, and settlement.
- [ ] EP-M2: Verus proof of settlement.
- [ ] EP-M3: Lean worker and end-to-end corpus run.
- [ ] EP-M4: documentation, design reconciliation, and roadmap.

## Surprises & discoveries

- Observation: a zero-axiom proof of `False` passes a name-only allowlist and
  whole environment replay. A `prelude` module importing only `Init.Prelude`
  may define `Nat.gcd := 0`. The kernel's accelerated arithmetic keys on the
  name, so `Nat.gcd 6 4` reduces to `2` by acceleration and `0` by unfolding,
  giving `2 = 0`. Evidence: `KernelNameForge` under `Artefacts and notes`: the
  unfixed prototype reports `"axioms":[],"replay":"accepted"`, and
  `leanchecker --fresh` exits 0. Impact: this defeated revision 1. Replay now
  always runs on the sysroot's `Init`, so the kernel refuses the redefinition
  (`constant has already been declared 'Nat.gcd'`). `lake check` guards the
  same names, listed in `Lake/CLI/Check.lean`.
- Observation: a trusted module identified by name can be replaced. A
  same-named module defining `Goal := True` was accepted by revision 1.
  Evidence: the panel's attack, reproduced as `TrustedSpoof`. Impact: this
  defeated revision 1. Trust now follows the directory a module resolved from,
  which the worker controls.
- Observation: Lean's search path works by package root. The first root
  containing a module's top-level directory captures all modules with that
  prefix, so `Fx.Good` is not found in a later root once an earlier one has
  `Fx/`. Evidence: the first run of the fixed prototype failed for every
  fixture. Impact: this helps the design. Trusted and job modules use different
  top-level prefixes, and placing the sysroot first prevents any `Init.*`
  shadowing.
- Observation: `Lean.collectAxioms` is not a plain walk in 4.35.0. It reads
  pre-computed axiom lists for imported declarations from an environment
  extension (`exportedAxiomsExt`), and silently ignores a constant it cannot
  find. Evidence: `src/lean/Lean/Util/CollectAxioms.lean`. Impact: the worker
  walks the replayed environment itself, exits non-zero on a missing constant,
  and uses `collectAxioms` only as a cross-check.
- Observation: a theorem added through `debug.skipKernelTC` passes every static
  check, and this needs no `import Lean`:
  `set_option debug.skipKernelTC true in` over `by decide +kernel` proves
  `(1 : Nat) = 2` under `Init` alone. Evidence: `ForgeInit`. Impact: replay is
  required, and the forgery fixtures are cheap.
- Observation: `leanchecker Fx.HackUser` exits 0 when the forgery sits in an
  imported module, since it replays only the named module. Evidence:
  `Artefacts and notes`. Impact: replay covers all non-toolchain modules in the
  import closure.
- Observation: `native_decide` produces one axiom per use, named such as
  `Fx.Native.helper._native.native_decide.ax_1_1`, in the job's own module.
  Evidence: `Native`. Impact: this confirms ADR 0003 input 1. Acceptance is by
  allowlist, and the `_native` component only selects the diagnostic.
- Observation: `«Quot.sound»` is a one-component name that an unescaped
  renderer would print identically to the two-component `Quot.sound`. Evidence:
  `Spoof`. Impact: names cross the JSON boundary as component arrays.
- Observation: `Kernel.Environment.replay` skips unsafe and partial constants,
  compares constructors and recursors rather than sending them to the kernel,
  and sees only the winner when a constant is duplicated across modules.
  Evidence: `Lean/Replay.lean`, and the panel's duplicate-root runs. Impact:
  W3's wording narrows. This stays sound because the walk reads the replayed
  environment, so skipped constants are simply absent.
- Observation: `replay` has no heartbeat limit, and a kernel bomb behind
  `debug.skipKernelTC` ran until killed at 25 s. Evidence: the panel's `Bomb`
  run. Impact: the runner has a deadline.
- Observation: fresh replay of the whole environment costs 91 s for an
  `Init`-only module, above the proposed 60 s end-to-end budget. Evidence:
  `Artefacts and notes`. Impact: see Deviation 3. 4.1.3 decides whether a
  release mode should do better.
- Observation: `docs/references.md`, named in the task's required reading, does
  not exist. Evidence: `ls docs/`. Impact: none on design. References stay in
  the technical design's References section, ADR 0003, and this plan.

## Decision log

- D1. Decision: the worker observes and Rust settles, and that includes
  statement comparison. Rationale: technical design §8 forbids a report
  choosing its own outcome. Revision 1's `statement_match` was a worker verdict
  that also left the proof silent on the roadmap's first-named check. The
  worker now reports the statement as a tree and never sees the expected one,
  which removes tree decoding and the JSON request from Lean, and moves
  statement equality inside the theorem. Date/Author: 2026-11-05, plan author,
  from the panel.
- D2. Decision: build a small repository-owned worker instead of adopting
  `lake comparator` or `lake check`. Rationale: those tools are the upstream
  gold standard for hostile proofs, and do not fit this boundary now. They
  refuse to start without `bwrap`, absent on the development host (exit 2). The
  rejection seen with `--inadvisably-no-sandbox` is the text
  `error: Illegal axiom detected: 'cheat'` and exit 1, with no per-root
  structured result. Each run exports the environment (36 s and 345 MB for
  `Init` alone). Polean's threat also differs: it regenerates its modules from
  data rather than judging hostile Lean. See Table 2. Date/Author: 2026-11-05,
  plan author.
- D3. Decision: an expected proposition is a ground `const`/`app` tree,
  compared as an injective byte key. Rationale: a named declaration in the job
  lets a faulty job define its own target. Source text would put data in a Lean
  executable position. Comparing rendered strings depends on the renderer.
  Comparing up to definitional equality accepts more than was asked, as
  `Unfolded` shows. An expected module could not catch `WrongPolicyLiteral`.
  Date/Author: 2026-11-05, plan author.
- D4. Decision: every non-toolchain module reachable from a root's statement
  must have resolved from the trusted root. Rationale: without it, a job can
  define the statement's constants itself, as `StatementForge` does. The panel
  proposed deferring this to 2.2.4, and proposed digests of trusted modules.
  Location is smaller than either: the request loses its module list, and 4.1.3
  binds the trusted package's identity. Date/Author: 2026-11-05, plan author,
  from the panel.
- D5. Decision: a module's class is the search root it resolved from. The
  worker's search order is sysroot, then trusted root, then job root, and it
  ignores `LEAN_PATH`. Rationale: name-based origin is forged by `InitShadow`
  and `TrustedSpoof`. Date/Author: 2026-11-05, plan author.
- D6. Decision: replay every non-toolchain module on a base that always
  includes the sysroot's `Init`, and walk the environment replay returns.
  Rationale: pinning `Init` makes the kernel itself reject every redefinition
  of a toolchain name. That covers `KernelNameForge`, `PreludeForge`, and
  `InitShadow` with no blocklist. Walking the returned environment inspects
  what the kernel accepted. Fresh replay costs 91 s, so 4.1.3 owns any stronger
  mode. Date/Author: 2026-11-05, plan author.
- D7. Decision: a request may narrow the foundations but not widen them, and
  `settle` re-validates, so its theorems carry no precondition on callers. The
  table of three name and origin pairs lives in the settlement code. Rationale:
  a request approving `sorryAx` would make the audit meaningless, and technical
  design §6.2 calls for a pinned set. Date/Author: 2026-11-05, plan author.
- D8. Decision: put the Rust spike in the existing `toolchain` test binary.
  Rationale: it inherits the helpers, the `smoke::` filter behind
  `make toolchain-smoke`, and the CI path filter. `settlement/` and `wire/`
  depend on no sibling module, so 2.2.4 can promote them with `git mv`.
  Date/Author: 2026-11-05, plan author.
- D9. Decision: record the outcome in ADR 0006, with no separate feasibility
  record. Rationale: 1.2.2's record catalogued 106 constructs. This corpus is
  small enough for its manifest to carry it. Date/Author: 2026-11-05, plan
  author.
- D10. Decision: use Verus for settlement, and tests for the worker and
  adapter. Rationale: see `Verification plan`. One reviewer recommended
  deferring EP-M2, because the roadmap does not require it and the worker stays
  unproved. This plan keeps it: the task instructs Verus for selected
  contractual logic, settlement survives into 2.2.4 and 4.1.3, and after D1 the
  theorem covers the roadmap's named checks. The user accepted this (D13).
  Date/Author: 2026-11-05, plan author.
- D11. Decision: panel suggestions not adopted. A settlement-table `insta`
  snapshot duplicates the manifest, so only canonical reports are snapshotted.
  A property oracle for settlement duplicates S1, so it is removed. `lean4lean`
  and `nanoda` as a second opinion are left to 4.1.3, as neither is established
  here. The adversary's explicit shadow-list finding is replaced by D6, which
  needs no list. Settling in Lean with a Lean proof loses the promotable logic
  and makes Rust accept a worker-chosen outcome. Date/Author: 2026-11-05, plan
  author.
- D12. Decision: worker resource exhaustion and deadline kills are
  `tool_failure` in this spike. `deepRecursion` and `excessiveMemory` reach the
  worker only as replay text, and fail closed as `kernel_rejected`. ADR 0006
  proposes that 2.2.4 maps deadline kills to `unknown` (exit 3) under technical
  design §8. Date/Author: 2026-11-05, plan author.
- D13. Decision: accept the plan's recommendations on four decisions: the worker
  moves to `lean/PoleanAudit.lean`, statements take the ground tree form that
  constrains 2.2.1 and 2.2.3, toolchain `.olean` files are trusted rather than
  replayed, and EP-M2 stays in scope. Rationale: the user accepted these
  recommendations in conversation when reviewing revision 2. This records those
  decisions only. It is not approval to begin implementation. Date/Author:
  2026-11-05, `leynos`.

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
- The **Lean kernel** is Lean's small trusted checker. **Replay** sends stored
  declarations through it again, as `leanchecker` does.
- An **`.olean` file** is Lean's compiled module format. Loading one maps it
  into memory, so a crafted file is an attack on the loader itself.
- The **sysroot** is the digest-named Lean install. The **trusted root** holds
  repository-owned statement modules. The **job root** holds the module being
  audited.
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

`verus/admission_recognizer.rs` shows the proof pattern. Source written once
under `tests/admission/recognizer/kernel/` is compiled by Cargo as test code
and pulled into Verus with `#[path]`, so the executed and verified code are the
same files. `scripts/check_verus_controls.py` requires Verus to reject seeded
faults. `make verus` runs both, and is on demand rather than a commit gate.

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

The task also names `docs/references.md`, which does not exist. Use the sources
above.

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
  (no `native_decide`, no generated axioms), §6 (the Lean layout), §6.2, §8
  (outcomes), §10 (V4 and V8), and §13 (the axiom-audit row).
- `docs/roadmap.md`, item 1.2.3, whose success criterion is quoted in `Purpose`.
- ADR 0002 item 2 (what the proof trusts) and ADR 0003 "Inputs for 1.2.3".
- `contracts/cli.txt`: exit 6 and exit 7, unchanged by this work.

Trace links:

```plaintext
1.2.3 "pinned audit interface"
  -> TD §6.2, ADR 0003 tuple -> polean-audit, audit contracts -> EP-M3
  -> smoke::audit::corpus_matches_manifest
1.2.3 "accepts an expected proposition and approved foundations"
  -> TD §6.2 -> S1 -> EP-M1, EP-M2 -> AcceptedGround, AcceptedFoundations, S7
1.2.3 "rejects sorryAx"
  -> TD §6.2, ADR 0002 item 2 -> S3 -> EP-M2, EP-M3 -> Sorry, SorryImported
1.2.3 "native-computation trust axioms"
  -> TD §5, ADR 0003 input 1 -> S3, A1 -> EP-M2, EP-M3 -> Native
1.2.3 "undeclared assumptions"
  -> TD §6.2 -> S3, W2 -> EP-M2, EP-M3 -> Undeclared, Reach*, NarrowedFoundations
1.2.3 "machine-readable results"
  -> TD §6.2, §8 -> contracts/audit-report.schema.json -> EP-M1 -> design-check, wire tests
TD §6.2 "expected declaration type"
  -> S1, W4, W5 -> EP-M2, EP-M3 -> WrongStatement, WrongPolicyLiteral, AxiomRoot
TD §6.2 "exact requested claim coverage"
  -> S2 -> EP-M1, EP-M2 -> coverage unit tests, settlement proof
TD §6.2 "malformed, or contradictory worker output is a tool failure"
  -> S2, A2, W1 -> EP-M1 -> wire tests, feature scenarios, WorkerGarbled
TD §2 "kernel acceptance"
  -> W3 -> EP-M3 -> ForgeInit, ForgeImported, KernelNameForge
```

## Verification plan

### What is being claimed

Three layers with different evidence meet at the JSON report.

1. **Settlement** is a pure function, so Verus can prove it for all inputs.
2. **The wire adapter** is strict JSON decoding and key encoding, tested by
   unit and property tests.
3. **The worker** is Lean `IO` code over toolchain internals. It carries no
   proof. It is tested against a corpus at the real interface, with seeded
   mutations showing the corpus detects faults.

Verus proves settlement correct given a truthful report. Nothing proves the
report truthful: that rests on the corpus and the axioms below.

### Abstract model first

Define these specifications in `verus/audit_settlement_spec.rs`, independently
of the executable code. They are stated here so they can test the API before
any code exists. Keys are byte sequences built by `wire::name_key` and
`wire::tree_key`, and are opaque to the model. Literals such as `K_propext` are
concrete byte sequences.

```text
Axiom   := { name: Key, module: Key, native: bool }   // native selects a diagnostic only
Decl    := { kind, statement: Option<Key>, statement_modules: Seq<Key>, axioms: Seq<Axiom> }
RootObs := { name: Key, decl: Option<Decl> }
Replay  := Accepted(trusted: Seq<Key>, job: Seq<Key>) | Rejected
Report  := { githash, module: Key, replay: Replay, roots: Seq<RootObs> }
Request := { module: Key, foundations: Seq<Key>, roots: Seq<{ name: Key, expected: Key }> }

pinned(n, m)   := (n, m) in { (K_propext, M_core), (K_quot, M_prelude), (K_choice, M_prelude) }
pinned_name(n) := n in { K_propext, K_quot, K_choice }

valid_request(q) := |q.roots| > 0
  and root names are pairwise distinct
  and forall f in q.foundations. pinned_name(f)

// ms is trusted ++ job. A module outside ms resolved under the sysroot.
protocol_ok(q, p, pin) := p.githash == pin and p.module == q.module and match p.replay {
  Rejected => |p.roots| == 0,
  Accepted(t, j) => q.module in j
    and |p.roots| == |q.roots|
    and forall i. p.roots[i].name == q.roots[i].name
    and forall i, d. p.roots[i].decl == Some(d) implies
          forall m in d.statement_modules. m in t ++ j }

approved(q, ms, a) := a.name in q.foundations and pinned(a.name, a.module) and a.module not in ms

root_clean(q, t, ms, i, o) := o.decl == Some(d) and d.kind == theorem
  and d.statement == Some(q.roots[i].expected)
  and forall m in d.statement_modules. m in t
  and forall a in d.axioms. approved(q, ms, a)

acceptable(q, p, pin) := valid_request(q) and protocol_ok(q, p, pin)
  and p.replay == Accepted(t, j)
  and forall i. root_clean(q, t, t ++ j, i, p.roots[i])

axiom_code(q, ms, a) :=
  if approved(q, ms, a) then None
  else if a.name == K_sorry then Some(SorryAxiom)
  else if a.native then Some(NativeAxiom)
  else Some(UndeclaredAxiom)

// expected_findings(q, p), in order:
//   Rejected replay => [ { root: None, item: None, KernelRejected } ]
//   Otherwise, for each root i:
//     decl None    => [ { i, None, RootAbsent } ]
//     decl Some(d) => WrongKind if kind != theorem;
//       StatementMismatch if statement != Some(expected);
//       { i, k, StatementOrigin } for each statement module k outside t;
//       { i, j, axiom_code } for each unapproved axiom j.
```

A forged foundation has no code of its own. `approved` still checks origin, so
a report naming `propext` from a replayed module settles `undeclared_axiom`. D6
means the real worker cannot produce that report, so the corpus reaches those
fixtures as `kernel_rejected`, and unit tests cover the settlement rule.

### Settlement obligations (Verus)

Each is a hypothesis about the eventual proof structure. Implementation may
split or refine them. `settle` returns `Result<Settlement, RequestRefusal>` and
carries no precondition, because the adapter is unverified and could not
discharge one.

- S0 `settle_refuses_exactly`: `settle` is `Err` if and only if not
  `valid_request(q)`.
- S1 `settle_accepts_exactly`: `settle` is `Ok(Accepted)` if and only if
  `acceptable(q, p, pin)`. This is the main theorem. It covers the expected
  statement, the kind, coverage, statement provenance, and the foundations.
- S2 `settle_fails_exactly`: `settle` is `Ok(ToolFailure)` if and only if
  `valid_request(q)` and not `protocol_ok(q, p, pin)`. A protocol breach
  therefore always takes precedence over findings. S0, S1, and S2 leave
  `Rejected` as exactly the remainder.
- S3 `accepted_axioms_closed_world`: `Ok(Accepted)` implies every reported
  axiom's name and module form one of the three literal pairs, and its name is
  not `K_sorry`. This is the roadmap's rejection criterion as a closed-world
  statement. Exclusion of `_native` and undeclared names follows by being
  outside the table; A1 connects that to real name components.
- S5 `rejected_findings_exact`: under `valid_request` and `protocol_ok`, and not
  `acceptable`, `settle` returns `Rejected(expected_findings(q, p))`, and root
  `i` has a finding if and only if it is not `root_clean`. A first-finding-only
  implementation fails this, and a mixed report keeps its per-root evidence
  (technical design §8).
- S6 `validate_request` is `Ok` if and only if `valid_request(q)`.
- S7 witnesses over ghost values show `Ok(Accepted)`, `Ok(Rejected)`,
  `Ok(ToolFailure)`, and `Err` are each reachable, so no antecedent above is
  vacuous.

Non-vacuity for the proof as a whole: `scripts/check_verus_controls.py` gains
controls that must each fail at a named obligation.

- Body controls: always reject (the deny-all argument); drop the
  `replay accepted` check, since empty roots make `forall` vacuous; compare no
  statement key; drop the name comparison from coverage; stop the root loop,
  the module loop, and the axiom loop one early; remove one table entry; ignore
  `q.foundations`; report a protocol breach as a rejection.
- Specification controls: add `K_sorry` to the pinned table, which must fail
  S3; weaken `valid_request` to `false`, which must fail the `Accepted`
  witness; weaken the origin clause of `approved` to `true`.

Evidence: `make verus` prints `verification results:: N verified, 0 errors` for
both proof entry points, followed by every control rejected. Grepping
`verus/audit_*` for `assume`, `admit`, and `external_body` finds nothing.
Discharge condition: `make verus` exits 0, recorded with the commit it ran on.

Solver plumbing, trigger selection, and auxiliary sequence lemmas are
deliberately left until implementation exposes the actual verification
conditions.

### Worker obligations (tests at the real interface)

Each is exercised by `smoke::audit::corpus_matches_manifest`, which compiles
the corpus with the pinned `lean`, runs the real worker, settles, and compares
the outcome and findings with the hand-written manifest. Manifest expectations
are written before the worker exists, and are never generated from its output.
Evidence: `make toolchain-smoke`. Discharge condition: every line matches.
Bound: the committed corpus and pinned toolchain.

- W1 Process protocol. The worker prints exactly one JSON document and exits 0
  if and only if it completed an audit. Otherwise it exits non-zero with
  nothing on standard output. A signal death has no exit code and is
  `tool_failure`. Artefact: `ModuleAbsent`, `JobUnderTrustedPrefix`, and
  `WorkerGarbled` runs. Non-vacuity: `ModuleAbsent` exits 1 in the probe.
- W2 Axiom closure. The reported axioms are exactly those reachable through
  types and values, across modules, and through private helpers. Artefact: one
  `Reach*` fixture for each traversal arm (definition value, definition type,
  opaque value, constructor type, theorem body in an imported module), plus
  `PrivateHelper`. The worker also compares its walk with `Lean.collectAxioms`,
  and exits non-zero on disagreement. Non-vacuity: M1.
- W3 Replay. An accepted replay means the kernel accepted every retained
  declaration in each non-toolchain module of the import closure, on a base
  containing the sysroot's `Init`. Unsafe and partial constants, and merged
  duplicates, are not retained, so they are not in the environment the walk
  reads. Artefact: `ForgeInit`, `ForgeImported`, `KernelNameForge`,
  `PreludeForge`, and `UnsafeRoot`. Non-vacuity: M2 and M4.
- W4 Statement fidelity. `statement` is the root's type rendered without loss,
  or `null` when out of grammar or over the bound. Artefact: `AcceptedGround`,
  `AcceptedClaimShape`, `MaximalPolicy`, `WrongStatement`, `WrongPolicyLiteral`,
  `Unfolded`, and `WrongLevel`. Non-vacuity: M3.
- W5 Kind and existence. `AxiomRoot` and `DefRoot` give `wrong_kind`.
  `RootAbsent` and `UnsafeRoot` give `root_absent` where replay retains
  nothing. Non-vacuity: M7.
- W6 Module class. Modules are classified by the root they resolved from, and
  the search order holds. Artefact: `StatementForge`, `TrustedSpoof`,
  `InitShadow`, and `JobUnderTrustedPrefix`. Non-vacuity: M6.
- W7 No execution. Auditing `InitSentinel` leaves no sentinel file. Positive
  control: `smoke::audit::sentinel_fires_when_initializers_run` runs
  `lean --run` on a file importing that module and requires the sentinel, so a
  broken fixture cannot pass. Non-vacuity: M5.

Seeded mutations are applied once to a scratch copy of the worker. These
results are author-reported in `Outcomes & retrospective`; nothing in the gates
reproduces them.

- M1: skip each traversal arm in turn, with the `collectAxioms` cross-check
  disabled, and require the matching `Reach*` fixture to fail.
- M2: remove replay, and require `ForgeInit` and `ForgeImported` to fail.
- M3: render only the statement's head constant, and require
  `WrongPolicyLiteral` to fail.
- M4: do not force `Init` into the base, and require `KernelNameForge` and
  `PreludeForge` to fail.
- M5: enable initializer execution, and require `InitSentinel` to fail.
- M6: search the job root before the trusted root, and require `TrustedSpoof`
  to fail.
- M7: report every declaration's kind as `theorem`, and require `AxiomRoot` and
  `DefRoot` to fail.

### Adapter obligations (property and unit tests)

Evidence for these is `make test`, and discharge is every test passing on every
host.

- A1 Key injectivity and classification. Keys are injective across name and
  tree structure. Method: a test-only decoder round trip under `proptest`,
  since random pairs rarely collide, plus named cases: `["Quot.sound"]` against
  `["Quot", "sound"]`, and string against numeric components. A seeded removal
  of the length prefix must fail. The `native` flag is set exactly when a
  component begins `_native`. `name_key` reproduces the settlement table's
  literal bytes, which connects the proof to real names.
- A2 Strict decoding. Unknown fields, duplicate keys, a missing nullable field,
  wrong JSON types, trailing content, an unknown `schema_version`, integers
  above `u64`, and oversized output each give exact `tool_failure`. Method:
  `rstest` tables, with `proptest` mutating a valid report by deleting,
  renaming, and retyping fields. Non-vacuity: generator classification must
  show each mutation class occurred.
- A3 Contract agreement. The Python design harness and the Rust decoder consume
  the same sample files, so schema and decoder cannot drift unseen. Method:
  named tests on both sides.

Why not other methods: Kani would bound what Verus already proves without a
bound, and cannot look into Lean. A state-machine model checker has nothing to
explore, as settlement has no state and the worker has no concurrency this plan
owns. A Lean proof of the worker would need a model of `Environment` that the
toolchain does not expose.

### Axioms relied upon

These are taken as given and not verified.

- The Lean kernel accepts only well-typed declarations, provided the names it
  treats as privileged denote their toolchain definitions. D6 establishes that
  proviso.
- `Kernel.Environment.replay` sends retained declarations to the kernel and
  checks constructors and recursors against those it generates.
- `Expr.getUsedConstants` returns every constant in an expression.
- Toolchain `.olean` files match the archive. They are unpacked once from a
  digest-checked archive, and later reuse checks only the `COMPLETE` marker.
  They are not replayed.
- Loading an `.olean` file with `loadExts := false` runs no initializers.
  `InitSentinel` and its positive control exercise this.
- Modules are compiled freshly by the pinned `lean` and are not crafted binary
  files, which is the boundary technical design §7 draws. Compiling a job
  executes its metaprograms, so compilation itself is trusted input, as
  technical design §2 requires. `ForgeInit` relies on this.
- The contents of the trusted root are authentic. This is the "integrity of the
  pinned proof package" in ADR 0002 item 2, and 4.1.3 binds it.
- `serde` with `deny_unknown_fields` rejects duplicate and unknown fields.
  Tests exercise this at the interface.
- Verus, Z3, and the release pins in `tools/verus/` are sound.

Residual gaps: no theorem connects the worker's report to the environment. The
audit trusts the toolchain library rather than replaying it. It says nothing
about what a statement means, since that is the definition of the Polean atoms
(ADR 0002 item 2).

## Plan of work

### Stage A: propose (done in this plan)

No repository code changes beyond this plan. Reconnaissance, research, probes,
and the panel's review are recorded here.

### Stage B: red

Write the contracts, the manifest, and the feature file first. Add settlement's
types with a `settle` that returns `ToolFailure` for every report, so unit
tests compile and fail for the expected reason. Add the provisioned corpus
test, which fails because `polean-audit` is absent.

### Stage C: implement and verify together

Write the specification in `verus/` before the body of `settle`, and develop
each loop with its invariant. Implement the worker after settlement is proved,
so the proved contract pressure-tests the report format while it is still cheap
to change.

### Stage D: refactor, document, and validate

Remove the scaffolding, reconcile the design, record outcomes, and run all
gates.

## Milestones and plateaus

Each milestone runs `make check-fmt`, `make lint`, and `make test`
sequentially, with output teed to
`/tmp/$ACTION-polean-1-2-3-demonstrate-type-and-axiom-auditing.out`. Commit
after each gated change.

### EP-M1: contracts, corpus manifest, and settlement

Outcome: a contract and a tested pure settlement function exist, and no Lean
code has changed. Any consumer can decide a recorded report.

Work: add `contracts/audit-request.schema.json` and
`contracts/audit-report.schema.json`. Add `examples/audit/sample-request.json`,
`examples/audit/sample-report.json`, and `examples/audit/manifest.json`, the
last being JSON Lines with one hand-written line per corpus entry. Extend
`validation/validate_design.py` with `check_audit_contracts`, holding the
samples and negative controls. Add `serde` to `Cargo.toml`. Add
`tests/toolchain/audit/`: `mod.rs`, `wire/` (decoding and key encoding),
`settlement/` with `settlement/decision/` (the pure function), `properties/`,
`tests.rs`, and `bdd.rs`, with the feature in
`tests/features/axiom_audit.feature`. Widen the description at
`tests/toolchain/main.rs:1` and `docs/repository-layout.md:14`.

Requirements: machine-readable results, malformed output, coverage.

Acceptance: `make test` runs the settlement and wire tests, each red before its
implementation. `make design-check` reports `json_schemas_checked: 7`.

Conformance check:

- The report holds no verdict or comparison result.
- `settlement/` and `wire/` use nothing from sibling modules.
- No item is added to `src/`.
- The only new dependency is the authorized `serde`.
- The four claims stay separate in every name.
- `SHA256SUMS` is refreshed for contracts and validation outputs.

Recovery: revert the milestone's commits. Nothing depends on them yet.

Remaining gaps: settlement is tested but not proved, and no real report exists.

Compatibility decision: none. These are new contracts with no consumer.

### EP-M2: Verus proof of settlement

Outcome: S0–S7 are discharged for the same source Cargo compiles.

Work: add `verus/audit_settlement_spec.rs`, `verus/audit_settlement_lemmas.rs`,
and `verus/audit_settlement.rs`, which pulls in `settlement/decision/` through
`#[path]`. Change the Makefile so `verus-proof` runs a list of entry points
(`VERUS_PROOF_FILES`), keeping the recognizer proof unchanged. Generalize
`scripts/check_verus_controls.py` from one hard-coded kernel directory and
proof file (`:50-52`) to a list of proof trees, add the controls in
`Verification plan`, and update `scripts/tests/`, including the pinned CLI
assertions in `test_check_verus_controls_cli.py`.

Requirements: accepts, rejects, undeclared, exact coverage.

Acceptance: `make verus` exits 0, with both proofs reporting `0 errors` and
every control rejected at its named obligation. `make test-scripts` passes. Then
`make check-fmt`, `make lint`, and `make test`.

Conformance check:

- No trusted proof body exists.
- The specification does not reference the executable code.
- The recognizer's verified count is unchanged.
- `make verus` stays on demand, and outside `make all` and CI.

Recovery: settlement's behaviour is unchanged by this milestone, so reverting
the proof leaves EP-M1 valid. If the verification tolerance is reached, stop
and present options.

Remaining gaps: the report's truthfulness is not addressed by this proof.

Compatibility decision: none.

### EP-M3: Lean worker and end-to-end corpus

Outcome: `make toolchain-smoke` runs the real worker over the whole corpus, and
each settlement matches the manifest.

Work: run `make tools` in this worktree. Add `lean/PoleanAudit.lean`, and add a
`[[lean_exe]]` table named `polean-audit` with root `PoleanAudit` to
`lean/lakefile.toml`, without `supportInterpreter` and without changing
`defaultTargets`. Add fixtures under
`examples/audit/lean/trusted/AuditTrusted/` and
`examples/audit/lean/job/AuditJob/`, with `InitShadow` and `TrustedSpoof` in
their own scratch roots. Add `tests/toolchain/smoke/audit/`, which builds the
worker with `lake --dir lean build polean-audit`, compiles fixtures with
`lean -o`, and runs the worker through `run_bounded`. Run the corpus in one
sequential test. Add `insta` snapshots of the canonical report for
`AcceptedFoundations` and `Native`, with `message` excluded. Add
`contracts/audit-*` and `examples/audit/**` to
`.github/workflows/toolchain.yml`. Apply the seeded mutations M1–M7 to a
scratch copy and record the results.

`run_bounded` takes a program, arguments, a deadline, and a byte cap. It reads
standard output with `take(cap + 1)` on a thread, polls `try_wait` until the
deadline, then kills and reaps. It returns the exit status, the bytes read, and
whether either bound was reached. It is test-only: 2.3.1 owns the production
supervisor.

The corpus is hand-written, and each line has the expected outcome below.
Finding and reason names are contractual.

| Fixture                 | Purpose                                      | Settlement                                   |
| ----------------------- | -------------------------------------------- | -------------------------------------------- |
| `AcceptedGround`        | Honest theorem with no axiom                 | `accepted`                                   |
| `AcceptedFoundations`   | All three foundations with pinned origins    | `accepted`                                   |
| `AcceptedClaimShape`    | Named claim over a structural non-empty body | `accepted`                                   |
| `MaximalPolicy`         | Claim over a 16-rule, 16-atom literal        | `accepted`, inside the tree bound            |
| `InitSentinel`          | `initialize` writes a sentinel if ever run   | `accepted`, and no sentinel file             |
| `MixedRoots`            | One clean root and one with `sorry`          | `rejected`: `sorry_axiom` on one root only   |
| `Sorry`                 | Direct `sorry`                               | `rejected`: `sorry_axiom`                    |
| `SorryImported`         | `sorry` behind an imported helper            | `rejected`: `sorry_axiom`                    |
| `Native`                | `native_decide` in a helper                  | `rejected`: `native_axiom`                   |
| `Undeclared`            | A direct user axiom                          | `rejected`: `undeclared_axiom`               |
| `ReachDefValue`         | Axiom reached through a definition value     | `rejected`: `undeclared_axiom`               |
| `ReachDefType`          | Axiom reached through a definition type      | `rejected`: `undeclared_axiom`               |
| `ReachOpaque`           | Axiom reached through an opaque value        | `rejected`: `undeclared_axiom`               |
| `ReachCtorType`         | Axiom reached through a constructor type     | `rejected`: `undeclared_axiom`               |
| `ReachImported`         | Axiom in an imported module's theorem        | `rejected`: `undeclared_axiom`               |
| `PrivateHelper`         | Axiom behind a private helper in a `module`  | `rejected`: `undeclared_axiom`               |
| `SpoofedName`           | `axiom «Quot.sound»`                         | `rejected`: `undeclared_axiom`               |
| `NarrowedFoundations`   | Uses foundations the request does not list   | `rejected`: `undeclared_axiom`               |
| `WrongStatement`        | A different proposition                      | `rejected`: `statement_mismatch`             |
| `WrongPolicyLiteral`    | Same claim, about a different list literal   | `rejected`: `statement_mismatch`             |
| `Unfolded`              | Definitionally equal, structurally different | `rejected`: `statement_mismatch`             |
| `WrongLevel`            | Universe level differs from the request      | `rejected`: `statement_mismatch`             |
| `StatementForge`        | Job defines the expected statement itself    | `rejected`: `statement_origin` only          |
| `AxiomRoot`             | Root is itself an axiom                      | `rejected`: `wrong_kind`, `undeclared_axiom` |
| `DefRoot`               | Root is a definition                         | `rejected`: `wrong_kind`                     |
| `RootAbsent`            | Named root does not exist                    | `rejected`: `root_absent`                    |
| `UnsafeRoot`            | `unsafe def`, not retained by replay         | `rejected`: `root_absent`                    |
| `ForgeInit`             | Forgery through `debug.skipKernelTC`         | `rejected`: `kernel_rejected`                |
| `ForgeImported`         | The same forgery in an imported module       | `rejected`: `kernel_rejected`                |
| `KernelNameForge`       | `Nat.gcd := 0`, deriving `False`             | `rejected`: `kernel_rejected`                |
| `PreludeForge`          | `prelude` module with its own `propext`      | `rejected`: `kernel_rejected`                |
| `InitShadow`            | Job root ships a forged `Init/Prelude`       | `rejected`: `kernel_rejected`                |
| `TrustedSpoof`          | Job root ships a same-named trusted module   | `rejected`: `kernel_rejected`                |
| `ReplayBomb`            | Kernel bomb behind `debug.skipKernelTC`      | `tool_failure`: `worker_timeout`             |
| `ModuleAbsent`          | Module file does not exist                   | `tool_failure`: `worker_failed`              |
| `JobUnderTrustedPrefix` | Job module named under the trusted prefix    | `tool_failure`: `worker_failed`              |
| `WorkerGarbled`         | Program printing a non-report                | `tool_failure`: `malformed_report`           |

_Table 1: The audit corpus and the settlement each entry must produce._

Requirements: pinned interface, all rejection classes, no execution of
submitted code.

Acceptance: `make toolchain-smoke` passes, including the existing OPA and Lean
smoke tests. `smoke::audit::corpus_matches_manifest` fails before the worker
exists. Then `make check-fmt`, `make lint`, and `make test`.

Conformance check:

- The worker sees no expected statement and emits no verdict.
- Fixtures stay outside the Lake package.
- `make test` needs no provisioned tool.
- `PoleanSmoke` and `defaultTargets` are unchanged.

Recovery: delete `lean/PoleanAudit.lean` and the `[[lean_exe]]` table to return
to EP-M2. If a fixture behaves differently from the table, the design tolerance
applies.

Remaining gaps: promotion to production and generated jobs are 2.2.4. Enforced
isolation is 1.2.4 and 4.3.1. Untrusted evidence is 4.1.3.

Compatibility decision: none.

### EP-M4: documentation, design reconciliation, and roadmap

Outcome: durable documents record the decision, and the roadmap entry is done.

Work:

- Add `docs/adrs/adr-0006-audit-lean-roots-by-observation-and-settlement.md`,
  with the decisions D1–D7, the alternatives, what the audit establishes and
  does not, the measured costs, the forward constraints on 2.2.1 and 2.2.3, and
  the proposed outcome mapping for 2.2.4. `rejected` maps to `invalid_evidence`
  (exit 7). `tool_failure` maps to `tool_error` (exit 6), except that a
  deadline kill maps to `unknown` (exit 3). A `RequestRefusal` maps to
  `tool_error`, because Rust generates the request, so a refusal is an internal
  defect rather than user input. An `accepted` settlement alone never exits 0.
  Record the unmeasured cost of replaying the trusted package per job against
  the 60 s budget. Record that 2.2.4's six-root walk may need a per-constant
  memo, since a shared `seen` set would misattribute axioms between roots.
- Amend technical design §2 (the IR-to-theorem trust row), §5 and §6
  (`lean/PoleanAudit.lean`, the ground statement form), §6.2 (observation and
  settlement, replay on a pinned `Init`, module class by location), §13 (the
  axiom-audit row), and its References.
- Add dated notes to ADR 0002 item 2 (toolchain `.olean` trust) and ADR 0003
  (resolving its 1.2.3 inputs and its `bwrap` open item).
- Update `docs/developers-guide.md` (what the audit is, how to add a fixture,
  how to read a report, `make verus` covering two proofs, a pin-change step
  covering the origin table, control anchors, snapshot githash, and the
  worker's Lean APIs), `docs/repository-layout.md`, `lean/README.md`,
  `validation/README.md` (seven schemas), `tools/verus/README.md`,
  `docs/contents.md`, and `docs/context.md` if a new term needs defining.
- Refresh `SHA256SUMS` after reviewing the drift.
- Mark roadmap 1.2.3 as done, with a `Recorded:` bullet stating what ran, and
  add the forward constraints to 2.2.1 and 2.2.3.

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

To run the worker by hand on compiled fixture directories:

```bash
env -i LEAN_SYSROOT="$PWD/.polean/tools/lean/$SHA" lean/.lake/build/bin/polean-audit \
  --trusted "$SCRATCH/trusted" --job "$SCRATCH/job" \
  --module AuditJob.AcceptedGround --root AuditJob.AcceptedGround.root
```

## Validation and acceptance

The behavioural specification is `tests/features/axiom_audit.feature`. It runs
against recorded reports through the decoder and settlement, while the corpus
exercises the real worker.

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

  Scenario: A foundation name from a replayed module is rejected
    Given a report whose root depends on propext from a module the worker replayed
    When the report is settled
    Then the settlement is rejected with finding undeclared_axiom

  Scenario: A statement that differs from the request is rejected
    Given a report whose root statement differs from the expected tree
    When the report is settled
    Then the settlement is rejected with finding statement_mismatch

  Scenario: A kernel replay rejection is rejected
    Given a report whose replay was rejected by the kernel
    When the report is settled
    Then the settlement is rejected with finding kernel_rejected

  Scenario: A mixed report keeps per-root evidence
    Given a report with one clean root and one root depending on sorryAx
    When the report is settled
    Then only the second root has finding sorry_axiom

  Scenario: Narrowed foundations are enforced
    Given a request omitting propext and a report using toolchain propext
    When the report is settled
    Then the settlement is rejected with finding undeclared_axiom

  Scenario: A request that approves sorryAx is refused
    Given a request listing sorryAx as a foundation
    When the request is validated
    Then it is refused before any worker runs

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
  reports. Follow the immutable `Slot` pattern in `tests/toolchain/bdd.rs`,
  which avoids a compiler crash on the pinned nightly.
- Property tests with `proptest` cover A1 and A2.
- Snapshot tests with `insta` hold two canonical reports, so format drift is
  reviewed.
- End-to-end tests are the provisioned corpus at the real process boundary.
  There is no CLI, so no command-line test applies.
- Verus covers S0–S7. Tests continue to cover the adapter, the compiled binary,
  and everything outside the proof.

Quality criteria:

- Tests: `make test` and `make toolchain-smoke` pass, with no new `#[ignore]`
  outside `smoke::`.
- Verification: `make verus` exits 0, with S0–S7 discharged and all controls
  rejected.
- Lint and types: `make check-fmt`, `make typecheck`, and `make lint`, which
  runs Whitaker.
- Scripts and design: `make test-scripts`, `make design-check`,
  `make design-properties`, `make design-boundary`, `make markdownlint`, and
  `sha256sum -c SHA256SUMS`.
- Performance: record fixture compile cost, and worker wall time and peak
  memory on `AcceptedGround` and `MaximalPolicy`. These are observations, not
  budget claims. SC7 measurement remains 4.3.2.
- Security: the corpus rows for forgery, spoofing, and execution pass. The
  seeded mutations are recorded as author-reported.

## Idempotence and recovery

Every step can be repeated. `make tools` is digest-checked and reuses verified
artefacts. Fixtures compile into fresh scratch directories per test run, so
nothing accumulates under `lean/`. `lean/.lake/` is ignored.
`make design-check` regenerates its outputs deterministically, and `design.yml`
requires them to be clean. If a Verus control's anchor drifts, the script fails
closed: update the anchor deliberately, in the same commit as the source edit.
If the provisioned test is interrupted, rerun it. It holds no state.

## Artefacts and notes

These observations were made against the pinned archive on 2026-11-05 in
scratch directories, using a sibling worktree's digest-named install. EP-M3
reproduces them through the committed corpus. All runs used a cleared
environment. Fixture names below are the probes' names.

Revision 1's checks, on the unfixed prototype, with `LEAN_PATH` and name-based
origin:

```plaintext
GcdForge    replay accepted  theorem  axioms []                  root is a proof of False
InitShadow  replay accepted  theorem  Quot.sound@Init.Prelude    a forged toolchain origin
leanchecker Job.GcdForge          rc=0
leanchecker --fresh Job.GcdForge  rc=0
```

The fixed prototype, with its own search path and `Init` pinned in the replay
base:

```plaintext
Good            accepted  theorem  []                 statement_modules [Fx.Expected:trusted]
GoodFoundations accepted  theorem  Quot.sound@Init.Prelude Classical.choice@Init.Prelude propext@Init.Core
ClaimJob        accepted  theorem  []                 statement is a const/app tree
InitSentinel    accepted  theorem  []                 no sentinel; `lean --run` control writes it
Sorry           accepted  theorem  sorryAx@Init.Prelude
Native          accepted  theorem  Fx.Native.helper._native.native_decide.ax_1_1@Job.Native
Custom          accepted  theorem  Fx.Custom.assume@Job.Custom
Transitive      accepted  theorem  Fx.TransitiveBase.hidden@Job.TransitiveBase
PrivateModule   accepted  theorem  _private.Job.PrivateModule.0.Fx.PrivateModule.secret@Job.PrivateModule
Spoof           accepted  theorem  «Quot.sound»@Job.Spoof
AxiomRoot       accepted  axiom    Fx.AxiomRoot.root@Job.AxiomRoot
StatementForge  accepted  theorem  []                 statement_modules [Job.StatementForge:job]
UnsafeRoot      accepted  declaration null
PreludeForge    REJECTED  constant has already been declared 'propext'
GcdForge        REJECTED  constant has already been declared 'Nat.gcd'
LandForge       REJECTED  constant has already been declared 'Nat.land'
InitShadow      REJECTED  unknown constant 'Absurd'
ForgeInit       REJECTED  application type mismatch
ForgeImported   REJECTED  application type mismatch
TrustedSpoof    REJECTED  declaration type mismatch
Fx.TrustedSpoof exit 1    object file '.../trusted/Fx/TrustedSpoof.olean' does not exist
```

In this transcript `accepted` and `REJECTED` are the replay status only.
Settlement then applies the request.

The same class of fixture against other interfaces:

```plaintext
leanchecker Fx.EnvHack          rc=1  rejected
leanchecker Fx.HackUser         rc=0  forgery in an import is not replayed
leanchecker --fresh Fx.HackUser rc=1  131.8 s, 1323 MiB
leanchecker --fresh Fx.Good     rc=0  91.0 s, 476 MiB
leanexport Fx.Good              rc=0  36.3 s, 872 MiB, 345,696,313-byte NDJSON
leanchecker --from-export       rc=0  87.0 s, 516 MiB
lake comparator                 rc=2  bwrap missing
lake comparator --inadvisably-no-sandbox  rc=1  error: Illegal axiom detected: 'cheat'
```

Costs of the compiled prototype:

```plaintext
lake build (empty environment, no interpreter support)  15–53 s, binary 118,675,648 bytes
audit, Init-only job                                    about 1–2 s, about 800 MiB resident
fixture compile, Init-only, under load                  1.9–4.6 s, about 430 MiB
ForgeInit audit                                         6.6 s
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

| Candidate                  | Statement check | Axioms          | Replay            | Structured result | Runs here        | Cost per run    | Trusts toolchain oleans |
| -------------------------- | --------------- | --------------- | ----------------- | ----------------- | ---------------- | --------------- | ----------------------- |
| `#print axioms`            | None            | Transitive      | None              | None              | Yes              | Not measured    | Yes                     |
| `leanchecker`              | None            | None            | Named modules     | None              | Yes              | About 1 s       | Yes                     |
| `leanchecker` and a script | None            | Transitive      | Named modules     | Script's own      | Yes              | Not measured    | Yes                     |
| `lake check`               | None            | Standard set    | Export            | Exit code         | Unsandboxed only | About 2 minutes | No                      |
| `lake comparator`          | Challenge file  | Permitted list  | Export            | Text, exit code   | Unsandboxed only | About 2 minutes | No                      |
| SafeVerify                 | Target file     | Standard set    | Named modules     | JSON              | Not established  | Not measured    | Yes                     |
| `polean-audit` (proposed)  | Data tree       | Name and origin | All non-toolchain | JSON              | Yes, unsandboxed | About 1 s       | Yes                     |

_Table 2: Existing audit interfaces compared with the proposed worker. The two
export-based costs are derived from the measured `leanexport` and
`leanchecker --from-export` runs on an `Init`-only module, not timed directly._

`lake comparator` gives stronger assurance than the worker because it does not
trust toolchain `.olean` files, and it remains the better tool for hostile
Lean. It could become an independent second opinion once 1.2.4 settles the
sandbox. ADR 0006 records this as a deferred option, and not a current
requirement.

### Request contract

`contracts/audit-request.schema.json`, identifier `polean.audit-request.v0`:

```json
{
  "schema_version": "polean.audit-request.v0",
  "module": ["AuditJob", "AcceptedGround"],
  "roots": [
    {
      "name": ["AuditJob", "AcceptedGround", "root"],
      "expected": { "const": ["AuditTrusted", "Goal"], "levels": [] }
    }
  ],
  "foundations": [["propext"], ["Quot", "sound"], ["Classical", "choice"]]
}
```

A statement tree is exactly `{"const": Name, "levels": [Nat]}` or
`{"app": [Tree, Tree]}`. It has no binder, literal, or variable form. A tree is
bounded at 4,096 nodes and depth 60: `serde_json` allows nesting of 127, each
`app` costs two levels, and a maximal policy needs a depth near 40 and about
2,000 nodes, which `MaximalPolicy` confirms. A request `Name` is a non-empty
array of non-empty strings, each matching `[A-Za-z_][A-Za-z0-9_']*`, so it can
be passed as a dotted argument without ambiguity. Levels are small natural
numbers.

### Worker interface

`lean/PoleanAudit.lean` provides the `polean-audit` executable. It imports
`Lean` and `Lean.Replay`, and accepts exactly these arguments, with one or more
`--root`:

```plaintext
polean-audit --trusted <dir> --job <dir> --module <dotted> --root <dotted> ...
```

The sequence is:

1. Read `LEAN_SYSROOT`, and set the search path to sysroot `lib/lean`, then
   trusted root, then job root. `LEAN_PATH` is not consulted.
2. Split each name on `.` into `Name` values, refusing anything outside the
   request grammar.
3. Import the job module with `withImportModules`, which forces
   `loadExts := false`.
4. Classify each imported module by the root its `.olean` resolved under,
   comparing path components.
5. Import `Init` and every toolchain module that a non-toolchain module
   directly imports, as the replay base.
6. Call `Kernel.Environment.replay` on all constants from non-toolchain modules.
7. For each root in argument order, read the constant from the replayed
   environment. Render its type as a tree. Walk its types and values, afresh
   for each root so that findings stay attributable, to collect axioms with
   their origin modules and the non-toolchain modules reachable from the
   statement. Compare the axiom set with `Lean.collectAxioms`.
8. Print one compact JSON line, and exit 0.

Any failure before step 8, including a missing constant during a walk, goes to
standard error and exits non-zero, with standard output empty. A replay
rejection is a completed audit, not a worker failure.

### Report contract

`contracts/audit-report.schema.json`, identifier `polean.audit-report.v0`:

```json
{
  "schema_version": "polean.audit-report.v0",
  "githash": "eb6cafb7290e8bd5f2937522d8d804f58f02bafc",
  "module": ["AuditJob", "AcceptedFoundations"],
  "replay": {
    "status": "accepted",
    "trusted_modules": [["AuditTrusted", "Expected"]],
    "job_modules": [["AuditJob", "AcceptedFoundations"]]
  },
  "roots": [
    {
      "name": ["AuditJob", "AcceptedFoundations", "root"],
      "declaration": {
        "kind": "theorem",
        "statement": { "const": ["AuditTrusted", "Goal"], "levels": [] },
        "statement_modules": [["AuditTrusted", "Expected"]],
        "axioms": [
          { "name": ["Classical", "choice"], "module": ["Init", "Prelude"] }
        ]
      }
    }
  ]
}
```

These rules bind independent implementers.

- A rejected replay is `{"status": "rejected", "message": String}` with
  `roots: []`. `message` is at most 4,096 UTF-8 bytes, truncated on a scalar
  boundary, never parsed, and excluded from snapshots.
- `roots` follows the request's order. `declaration` is `null` when the
  replayed environment has no such constant. It is a required field, and a
  missing field is malformed rather than `null`. `statement` is likewise
  required, and `null` when out of grammar or over the bound.
- `kind` is one of `theorem`, `axiom`, `definition`, `opaque`, `quotient`,
  `constructor`, `recursor`, and `inductive`.
- `axioms` lists every axiom reachable through the root's type and value,
  including the root itself if it is an axiom. A module not listed in `replay`
  is a toolchain module.
- `statement_modules` lists the non-toolchain modules holding a constant
  reachable from the root's type.
- A report `Name` is a non-empty array whose components are strings (possibly
  empty) or integers up to `u64::MAX`, because private names contain numbers.
  `githash` is compared with the commit in the `identity` entry of
  `toolchain-pins.json`.
- Lists are sets. The worker's deterministic order keeps snapshots stable, and
  consumers must not rely on it.
- There is deliberately no `accepted`, `verdict`, `ok`, or `statement_match`
  field. Any shape change means a new schema identifier.
- Enforcement: the schema checks shape. `wire` also checks integer ranges, byte
  bounds, duplicates, trailing content, and tree bounds. Settlement checks
  correspondence.

`Lean.Json.compress` output is safe to decode: in the pinned source, naturals
print in decimal, characters below `0x20` are escaped, and other text is raw
UTF-8.

### The Rust settlement function

In `tests/toolchain/audit/settlement/decision/mod.rs`, pure and with no
`serde`, I/O, or Lean. Keys are `Vec<u8>` length-prefixed, tag-prefixed
encodings built by `wire::name_key` and `wire::tree_key`.

```rust
pub struct AuditRequest { /* module, foundations, roots with name and expected, as keys */ }
pub struct AuditReport { /* githash, module, replay, roots, as keys and closed enums */ }

pub enum Settlement { Accepted, Rejected(Vec<Finding>), ToolFailure(ProtocolBreach) }

pub struct Finding { pub root: Option<usize>, pub item: Option<usize>, pub code: FindingCode }

pub enum FindingCode {
    KernelRejected, RootAbsent, WrongKind, StatementMismatch, StatementOrigin,
    SorryAxiom, NativeAxiom, UndeclaredAxiom,
}

pub enum ProtocolBreach {
    ToolchainMismatch, ModuleMismatch, CoverageMismatch,
    RejectedReplayWithRoots, UnreplayedStatementModule,
}

pub fn validate_request(request: &AuditRequest) -> Result<(), RequestRefusal>;
pub fn settle(request: &AuditRequest, report: &AuditReport, pin: &[u8])
    -> Result<Settlement, RequestRefusal>;
```

The `native` flag and `FindingCode` select a diagnostic only: acceptance rests
on `approved`, so a defect in classification cannot admit an axiom. Process and
decoding failures (`worker_failed`, `worker_timeout`, `output_overflow`, and
`malformed_report`) arise in `tests/toolchain/audit/wire/` and the smoke
runner, before settlement, and are reported as the same `tool_failure` outcome.
Promotion into `src/lean.rs` is 2.2.4's work.

### Dependencies

`serde` `=1.0.229` with `derive`, under `[dev-dependencies]`, gives derived
decoding with `deny_unknown_fields`, which rejects duplicate and unknown fields.
`serde_json::Value` would silently keep the last duplicate. Derived decoding
treats a missing `Option` as `None`, so required nullable fields use a
presence-checking `deserialize_with`, and A2 tests this. No Lake package is
added. The worker uses only the pinned Lean's own `Lean` library.

## Revision notes

- Revision 1 (2026-11-05): initial draft, from reconnaissance, research, and
  probes of the pinned toolchain.
- Revision 2 (2026-11-05): revised after expert panel review, including a live
  attack. Two attacks defeated revision 1: a zero-axiom proof of `False`
  through kernel-accelerated names, and a same-named trusted module. Both were
  reproduced and the fix prototyped. Replay now runs on a pinned `Init`, and
  the worker fixes its own search path and classifies modules by location. The
  worker now reports statement trees and no comparison result, takes arguments
  rather than JSON, and moved to `lean/PoleanAudit.lean`. The specification was
  corrected and made unconditional. A deadline and output bound were added, with
  `ForgeInit` replacing `import Lean` fixtures. The forged-foundation finding
  and several report fields were removed. Remaining work is unchanged in order,
  and larger in EP-M2's control list.
- Revision 3 (2026-11-05): recorded the user's acceptance of the three design
  deviations and of keeping the Verus milestone (D13), and renamed the section
  that lists the deviations. The design and remaining work are unchanged, and
  the plan still awaits approval before implementation.
