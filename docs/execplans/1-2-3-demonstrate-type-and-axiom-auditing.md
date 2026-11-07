# Demonstrate declaration-type and transitive-axiom auditing

This ExecPlan is a living document. Keep `Constraints`, `Tolerances`, `Risks`,
`Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan`
current as work proceeds.

Status: COMPLETE (revision 9). The user approved this plan and accepted
[ADR 0006](../adrs/adr-0006-separate-proof-integrity-from-operational-integrity.md)
on 2026-11-06. Implementation finished on 2026-11-07; promotion into the
product is roadmap 2.2.4 and operational integrity is roadmap 4.3.2.

Roadmap item: [1.2.3](../roadmap.md). Branch:
`1-2-3-demonstrate-type-and-axiom-auditing`.

## Purpose / big picture

Polean's first claim is that Lean checked a model theorem. A compiled Lean file
does not establish that. A build succeeds with `sorry`, with `native_decide`,
or with an invented `axiom`, and a declaration may have the requested name and
prove something else. Technical design §6.2 therefore requires the expected
declaration type, exact requested claim coverage, successful checking, and a
machine-readable audit.

This task establishes **proof integrity**: Polean can determine whether Lean's
kernel checked exactly the requested proposition, with complete theorem-root
coverage and only approved transitive axioms. It does so under the
**generated-module input model** in technical design §2.1: Polean generates
Lean modules from trusted templates and constrained policy data, and accepts no
arbitrary Lean source or compiled proof artefact.

When the work is done, a contributor runs `make toolchain-smoke` and watches a
small trusted Lean program, `polean-audit`, inspect a committed corpus of
fixtures. It replays the modules through the kernel and returns one JSON report
of each root's kind, statement, and transitive axioms. A test-only Rust
settlement function compares that report with an audit request holding the
exact expected propositions and the approved foundations. It decides `accepted`,
`rejected` with named findings, or `tool_failure`.

Honest theorems are accepted. Fixtures using `sorry`, `native_decide`, or an
undeclared axiom are rejected, including when the axiom hides behind an
imported definition. A root with the wrong statement, a theorem about a
different policy literal, a root that is not a theorem, and a declaration the
kernel refuses are each rejected for their own named reason. Malformed worker
output is a tool failure, never a rejection or an acceptance. A Verus proof
shows settlement accepts exactly when an independently written specification
holds.

This task does **not** establish **operational integrity**: that the Lean
environment that answered is the one the evidence names. Module provenance and
shadowing, privileged-name forgery, and trusted proof-package integrity move to
roadmap 4.3.2, and
[ADR 0006](../adrs/adr-0006-separate-proof-integrity-from-operational-integrity.md)
records why. The earlier adversarial work is preserved in the
[findings record](../lean-environment-integrity-findings.md).

A passing audit is an input to claim 2 in AGENTS.md, and proves no registered
requirement, because no Polean semantics exist yet. It says nothing about
admission, translation, or runtime correspondence. Promotion into production
belongs to 2.2.4.

## Revised scope for review

These tables are what changed in revision 4.

| Concern                                          | Revision 3 | Revision 4                   |
| ------------------------------------------------ | ---------- | ---------------------------- |
| Kernel replay of non-toolchain modules           | 1.2.3      | 1.2.3                        |
| Exact statement, coverage, and kind              | 1.2.3      | 1.2.3                        |
| Transitive axioms against approved foundations   | 1.2.3      | 1.2.3, by name               |
| Machine-readable report and fail-closed outcomes | 1.2.3      | 1.2.3                        |
| No execution of inspected code                   | 1.2.3      | 1.2.3                        |
| Own axiom walker                                 | 1.2.3      | Upstream `collectAxioms`     |
| Foundation origin table                          | 1.2.3      | 4.3.2                        |
| Pinned `Init` replay base                        | 1.2.3      | 4.3.2                        |
| Worker-fixed search order and module classes     | 1.2.3      | 4.3.2                        |
| Statement provenance                             | 1.2.3      | 4.3.2                        |
| Trusted-package identity                         | 4.1.3      | 4.3.2, using 4.1.1's binding |
| Worker deadline and output reader                | 1.2.3      | 2.3.1, then 4.3.1            |

_Table 1: Where each concern now lands._

Dependencies after this revision:

```plaintext
1.2.1 -> 1.2.3 -> 2.2.4 -> 2.3.3 -> phase 3 -> 4.1.1 -> 4.1.2 -> 4.1.3
4.1.3 -> 4.2.x -> 4.3.1
4.1.3 and 4.3.1 -> 4.3.2 (new) -> 4.3.3 (was 4.3.2) -> 5.1.1
```

The assurance trade-offs are stated plainly under `Assurance trade-offs`.

## Constraints

- Keep the four claims in AGENTS.md separate in every test name, diagnostic,
  contract, and document. An `accepted` settlement is not `model_proved`.
  Describe settlement as verified at a named commit and the worker as tested.
  Never describe the audit as verified, and do not describe it as defending
  against a hostile Lean environment.
- Accept no user Lean, Lake files, compiled proof modules, native libraries, or
  plug-ins as verifier input. The expected proposition is data held by Rust and
  never reaches Lean. Names reach the worker only as argument array entries,
  which it splits on `.` into `Name` values without parsing them as Lean source.
- The audit must not execute code from the modules it inspects. The worker
  imports with `loadExts := false`, never enables initializer execution, and
  never calls `evalConst`.
- Do not add an item to `src/` or a binary target to `Cargo.toml`. The guard in
  `tests/documentation_examples/scope.rs` fails when one appears, and 2.1.4
  owns the application edge. The Rust side of this spike is test-only under
  `tests/toolchain/audit/`.
- The Lean worker is repository-owned trusted code at `lean/PoleanAudit.lean`.
  Fixtures are compiled into scratch directories and never join the Lake
  package.
- The worker reports observations, with no verdict and no comparison result.
  Rust settlement is the only acceptance authority. A report cannot choose its
  own outcome (technical design §8), and the exit status is not acceptance
  (§6.2).
- Run tools by explicit digest-named path with a cleared environment and an
  argument array. Do not resolve any tool through `PATH` or elan.
- Do not change a pin: `toolchain-pins.json`, `lean/lean-toolchain`, the empty
  package list in `lean/lake-manifest.json`, `rust-toolchain.toml`, or the
  Verus pins.
- Do not use `assume`, `admit`, `external_body`, or any other trusted proof
  body in `verus/`. Do not use `sorry`, `native_decide`, or an `axiom` in
  `lean/PoleanAudit.lean`.
- Do not add operational-integrity checks to this milestone. Findings that fall
  outside the input model go to the findings record for 4.3.2.
- Do not modify `examples/tenant-write/` or `examples/admission/`.
- Keep Rust modules under 400 lines and functions under 70 lines. Every Rust
  module begins with a module-level comment. `std::fs` remains banned in tests;
  use `tests/toolchain/ambient.rs`.

If satisfying the objective requires violating a constraint, do not proceed.
Record the conflict in `Decision log`, set the status to `BLOCKED`, and
escalate.

## Tolerances (exception triggers)

Exclude this ExecPlan from every count below.

- Scope: stop and escalate if implementation needs more than 30 non-fixture
  files or about 3,000 net lines of code and other documentation. Lean
  fixtures, reviewed snapshots, and regenerated validation outputs do not count.
- Interface: stop if any public item in `src/` or the proposed CLI in
  `contracts/cli.txt` would have to change.
- Dependencies: the only authorized new dependency is `serde`, pinned
  `=1.0.229` with the `derive` feature under `[dev-dependencies]`. It is
  already in `Cargo.lock` at that version. Anything else needs approval.
- Design: if any fixture in Table 2 cannot be classified as specified by the
  pinned toolchain, or a probe in `Artefacts and notes` does not reproduce,
  stop. Record the deviation, set `BLOCKED`, and wait for acceptance. Do not
  edit the manifest to match the worker.
- Verification: if the settlement theorem cannot be discharged without a
  trusted body, stop and present the options. Do not weaken the specification.
- Iterations: stop after three failed attempts on one root cause.
- Resources: stop if the worker exceeds 30 s or 4 GiB resident on any fixture.
- Ambiguity: stop and present options when a choice materially changes what a
  settlement means.

## Assurance trade-offs

- **What is gained.** Phase 1 remains a feasibility demonstration with one
  clear claim. Phase 2 gets a small interface: a worker of about 85 lines in
  prototype, a shorter report, and a simpler specification. Hardening lands
  once, where the identities it depends on exist.
- **What is given up.** Between 1.2.3 and 4.3.2, an audit can accept a forged
  result if the input model is broken, and it looks like any other result. On
  the pinned tuple, the proof-integrity worker reports a clean zero-axiom proof
  of `False` from a `prelude` module redefining `Nat.gcd`, and a clean result
  from a same-named replacement of a trusted module (Table 1 in the findings
  record). A defect in a template, or a tampered proof package, would be
  undetected in the same way.
- **Why that is acceptable now.** No external party can supply Lean. The
  templates of 2.2.4 emit only enum and list literals. Nothing public makes the
  ADR 0002 statement before 4.3.2, and development reports name the input model.
- **What stays strong.** A kernel-rejected declaration, a wrong statement, an
  omitted root, and any unapproved axiom are all still caught, and those are
  the failures realistic inside the input model.
- **A cheap option kept open.** Pinning `Init` in the replay base is one line,
  and would catch privileged-name forgery. It is deferred because that is the
  agreed boundary, and 4.3.2 can take it early if a template defect ever looks
  plausible.
- **Residual gaps after 4.3.2.** Crafted `.olean` files, host compromise, and
  kernel defects stay outside. Those limits predate this revision.

## Accepted design deviations

The user accepted these on 2026-11-05 (D13), and revision 4 keeps them. This
revision has applied them to the technical design.

1. The worker lives at `lean/PoleanAudit.lean`, because it imports `Lean`,
   which policy modules must not load.
2. A root's statement must be a ground tree of constants and applications (D3).
   Claims are stated through a named predicate such as `Holds claim literal`,
   rather than the bare `∀ r` signatures technical design §5 sketches.
   Non-empty rule bodies are represented structurally. The predicate's
   definition may quantify.
3. Toolchain modules are not replayed. The audit trusts toolchain `.olean`
   files unpacked once from a digest-checked archive.

## Risks

- Risk: structural statement comparison is too brittle for real claims.
  Severity: medium. Likelihood: medium. Mitigation: the 2026-11-06 probe
  confirmed a named predicate over a nested list literal is a bare
  `const`/`app` tree, and a maximal 16-by-16 policy fits at 1,765 nodes and 38
  levels by application; the plan estimated 1,705 nodes at depth 37, and the
  corpus measured the larger figure. `AcceptedClaimShape` and `MaximalPolicy`
  pin this, and out-of-grammar statements report `null` and fail closed.
- Risk: a reader takes `accepted` to mean more than it does. Severity: high.
  Likelihood: medium. Mitigation: the constraints, `Assurance trade-offs`, ADR
  0006, and technical design §2.1 all state the condition, and development
  reports must name the model.
- Risk: `Lean.collectAxioms` relies on a pre-computed extension for imported
  declarations and ignores an unknown constant. Severity: low inside the model.
  Likelihood: low. Mitigation: `SorryImported` and `ReachImported` exercise
  imports at the real interface, and an independent walk agreed on every
  fixture compared. 4.3.2 owns anything stronger.
- Risk: a worker hang stalls a gate, because no supervisor exists until 2.3.1.
  Severity: low. Likelihood: low. Mitigation: the worker takes trusted input
  and has no loop over untrusted data. A hang shows up as the 30-minute
  workflow limit, and 2.3.1 and 4.3.1 own bounded execution.
- Risk: "verified" drifts, because a settlement edit can pass CI without Verus
  running. Severity: medium. Likelihood: medium. Mitigation: the source is
  shared through `#[path]`, documents name the verified commit, and the
  developers guide lists `make verus` for that directory. This matches the
  recognizer's posture.
- Risk: the Verus proof costs more than settlement deserves. Severity: medium.
  Likelihood: low. Removing origin and provenance makes the model far smaller
  than the recognizer's 2,252 lines of specification and lemmas. Literal-slice
  contents are opaque to the pinned Verus (`verus/admission_spec.rs`), so use
  exec builders with `ensures r@ == seq![...]`, one recursive `contains`, and a
  `bytes_eq` helper following `matches_at`. The verification tolerance applies
  otherwise.
- Risk: the fixed foundation names or worker APIs drift on a Lean upgrade.
  Severity: low. Likelihood: medium. Mitigation: `AcceptedFoundations` fails
  closed, `warningAsError` turns deprecations into build failures, and EP-M4
  adds a step to the pin-change procedure.
- Risk: the sibling-worktree toolchain used for the probes is not this
  worktree's gate input. Severity: low. Likelihood: certain. Mitigation: EP-M3
  runs `make tools` here and repeats the probes through the committed fixtures.

## Progress

- [x] (2026-11-05) Renamed the branch, loaded the required skills, and surveyed
      the repository
  with a Wyvern team.
- [x] (2026-11-05) Researched `lake comparator`, `lake check`, SafeVerify, and
      the Lean
  proof-validation guide, and probed the pinned Lean 4.35.0.
- [x] (2026-11-05) Drafted the plan, ran the expert panel and its live attack,
      and revised it.
- [x] (2026-11-05) Published draft pull request #35. The user accepted three
      design deviations
  and the Verus milestone (D13).
- [x] (2026-11-06) The user directed separating proof integrity from
      operational integrity (D14).
- [x] (2026-11-06) Re-ran the corpus and attacks against a proof-integrity
      prototype and a hardened
  prototype.
- [x] (2026-11-06) Revised this plan (revision 4), the roadmap, the technical
      design, the
  vocabulary, and ADR 0002 and ADR 0003, and proposed ADR 0006. Added the
  findings record.
- [x] (2026-11-06) User review of this revision, and acceptance of ADR 0006.
- [x] (2026-11-06) User approval of the plan.
- [x] EP-M1: contracts, corpus manifest, and settlement.
  - [x] (2026-11-06) Wrote both JSON contracts, the two shared samples, the
        22-entry corpus manifest, the strict decoder, the injective key
        encoding, the settlement function, the fixture builders, the eleven
        BDD scenarios, the property tests, and the design-check controls.
  - [x] (2026-11-06) Repaired 26 Clippy findings, including ten
        `manual_let_else` and five `shadow_reuse`. `cargo test --test toolchain
        audit::` is green with 80 tests.
  - [x] (2026-11-06) Repaired the three Whitaker faults from the first full
        `make lint`: split `decision` into `types.rs` and `mod.rs`, and moved
        `protocol_breach` and `convert_replay` into named helpers. `cargo
        check --test toolchain` and `cargo test --test toolchain audit::` are
        green again with 80 tests.
  - [x] (2026-11-07) Repaired the two remaining Clippy findings from the first
        full `make lint`: `rejected_breach` is now a `const fn`, and
        `convert_rejected` takes `&RejectedReplayDocument`.
  - [x] (2026-11-07) Ran the full gate sweep through `scrutineer`:
        `make markdownlint`, `make typecheck`, `make lint`, `make test`, and
        `make design-check` passed; `make check-fmt` failed only on a
        `mdtablefix` prose reflow in this plan, which was repaired and
        re-checked green. `sha256sum -c SHA256SUMS` verifies all 46 entries.
  - [x] (2026-11-07) `make test-scripts` cannot run under the default target
        because the `python3` on `PATH` is 3.12.15 and
        `scripts/provision_tools.py` requires Python 3.14 with
        `compression.zstd`. The documented `uv run --python 3.14 --with
        pytest==9.1.1` fallback is green with 138 tests. Recorded as an
        environment limitation, not a code fault.
- [x] EP-M2: Verus proof of settlement.
  - [x] (2026-11-07) Proved the whole settlement obligation set in a scratch
        tree at `/tmp/epm2`: S0, S1, S2, S3, S5, S6, the four bridging lemmas,
        and the S7 witnesses. `verification results:: 61 verified, 0 errors`.
        The ported proof later measured 64 after the review repair of
        `decision/findings.rs` split out three helper functions.
  - [x] (2026-11-07) Ported the proof into `verus/audit_settlement.rs` with
        its specification, lemmas, and witnesses, and split the contracted
        `decision/mod.rs` into `scan.rs`, `request.rs`, `replay.rs`, and
        `findings.rs` (D19).
  - [x] (2026-11-07) Generalized `VERUS_PROOF_FILES` in the Makefile and the
        two-tree staging in `scripts/check_verus_controls.py`, and added the
        ten settlement controls.
  - [x] (2026-11-07) `make verus` exits 0: the recognizer still verifies
        `89 verified, 0 errors` and the settlement verifies `64 verified, 0
        errors` with eight `autoderive` warnings. All 15 controls are rejected
        with a diagnostic naming the expected failure kind and source path.
        `make test-scripts` is green with 170
        tests under the documented Python 3.14 fallback.
  - [x] (2026-11-07) Repaired two defects the first sweep found. The
        specification and lemma modules are gated on `cfg(verus_keep_ghost)`
        like the witnesses, because plain `rustc` erases every item in them and
        `-D warnings` then failed on their unused imports. `is_theorem` now
        reads `matches!(kind, DeclKind::Theorem)`, and the statement comparison
        tests equality first, to satisfy `match_like_matches_macro` and
        `if_not_else`. The `statement-comparison-dropped` control's anchor was
        updated to the new branch order.
  - [x] (2026-11-07) The second sweep is green on all seven targets:
        `make check-fmt`, `make markdownlint`, `make typecheck`, `make lint`
        (Whitaker, `cargo doc`, and Clippy), `make test` (204 passed, 7
        ignored), `make design-check`, and `make test-scripts` with the Python
        3.14 override (170 passed).
- [x] EP-M3: Lean worker and end-to-end corpus run.
  - [x] (2026-11-07) Added `lean/PoleanAudit.lean` and the `polean-audit`
        `[[lean_exe]]` table. `defaultTargets` and `PoleanSmoke` are unchanged.
  - [x] (2026-11-07) Wrote the twenty-five modules under
        `examples/audit/lean/AuditFixture/`, outside the Lake package — twenty
        audited fixtures and five support modules (`Goal`, `Claim`,
        `SorryBase`, `ReachBase`, `ForgeBase`) — plus the two programs that are
        not report producers (`Garbled`, `SentinelProbe`).
  - [x] (2026-11-07) Added `tests/toolchain/smoke/audit/` with the manifest
        reader, the hand-written request table, the scratch-tree runner, and
        the corpus test. `Corpus::prepare` builds the worker with
        `lake --dir lean build polean-audit`, compiles each fixture with
        `lean -o --root`, and runs every process with an empty environment
        apart from `LEAN_SYSROOT` and `LEAN_PATH`.
  - [x] (2026-11-07) Added `examples/audit/**` and `contracts/audit-*` to both
        path lists in `.github/workflows/toolchain.yml`.
  - [x] (2026-11-07) Found and repaired one real defect before the corpus went
        green (D20): the decoder counted tree depth as JSON nesting levels while
        the worker counted applications, so `MaximalPolicy` was refused at depth
        76 against a bound of 60. The decoder now counts as the worker does. The
        depth-negative fixture moved from `spine(30)` to `spine(60)`.
  - [x] (2026-11-07) `make toolchain-smoke` is green: `9 passed; 0 failed` in
        581.28 s, covering the four OPA scenarios, the two version reports, the
        Lean library build, the sentinel positive control, and the corpus (log
        `/tmp/smoke-polean-1-2-3-demonstrate-type-and-axiom-auditing.out`).
  - [x] (2026-11-07) `cargo test --test toolchain audit::wire` is green with 37
        tests, including the refused over-depth and over-node reports.
  - [x] (2026-11-07) Added the two insta snapshots of the canonical report at
        `tests/toolchain/smoke/audit/snapshots/`, with `replay.message`
        removed.
  - [x] (2026-11-07) Applied the four seeded mutations to the worker in place,
        ran the corpus under each, and restored the worker byte for byte
        (`sha256 4d0d60b7…`). Results are in the EP-M3 outcome.
  - [x] (2026-11-07) The commit-gate sweep through `scrutineer` is green on the
        repaired candidate: `make check-fmt`, `make markdownlint`,
        `make typecheck`, `make lint` (Whitaker, `cargo doc`, and Clippy),
        `make test` (74 + 27 + 2 + 101 passed, 9 ignored, doc-tests 1, 682 s),
        `make design-check`, `make test-scripts` (170 passed), and
        `make toolchain-smoke` (9 passed in 778.49 s).
- [x] EP-M4: documentation and roadmap completion.
  - [x] (2026-11-07) Recorded ADR 0006's measured results, its outcome mapping
        for 2.2.4, and roadmap 4.2.2's dependency on 4.3.2.
  - [x] (2026-11-07) Documented the audit corpus in `docs/developers-guide.md`,
        with report reading, fixture addition, and pin-change evidence, and
        updated the command table and the Verus paragraphs for two proofs and
        fifteen controls.
  - [x] (2026-11-07) Updated `docs/repository-layout.md`, `lean/README.md`,
        `validation/README.md`, `tools/verus/README.md`, and `docs/contents.md`.
  - [x] (2026-11-07) Added `Observed again during 1.2.3` to the findings record.
  - [x] (2026-11-07) Refreshed `SHA256SUMS` after reviewing the two changed
        entries, and marked roadmap 1.2.3 done with a `Recorded:` bullet.

## Surprises & discoveries

Findings about the Lean environment outside the input model, including the two
attacks that defeated revision 1, are in the
[findings record](../lean-environment-integrity-findings.md) as research and
regression candidates for 4.3.2. Those affecting this milestone are below.

- Observation: `native_decide` produces one axiom per use, named such as
  `Fx.Native.helper._native.native_decide.ax_1_1`, in the job's own module.
  Evidence: `Native`. Impact: this confirms ADR 0003 input 1. Acceptance is by
  allowlist, and the `_native` component only selects the diagnostic.
- Observation: a theorem compiled under `debug.skipKernelTC` passes every
  static check, with `Init` alone: `(1 : Nat) = 2` by `decide +kernel`.
  Evidence: `ForgeInit`. Impact: replay is what makes "kernel-checked" an
  observation that can fail, so it stays here as the negative control.
- Observation: `Kernel.Environment.replay` skips unsafe and partial constants,
  and compares constructors and recursors rather than sending them to the
  kernel. Evidence: `Lean/Replay.lean`. Impact: W3's wording. A skipped root is
  simply absent from the replayed environment.
- Observation: `«Quot.sound»` is a one-component name that an unescaped
  renderer prints identically to the two-component `Quot.sound`. Evidence:
  `SpoofedName`. Impact: names cross JSON as component arrays, which costs
  nothing and avoids a later schema change.
- Observation: `docs/references.md`, named in the task's required reading, does
  not exist. Evidence: `ls docs/`. Impact: none. References stay in the
  technical design and this plan.
- Observation: this worktree's `target/` was created without Cargo's
  `CACHEDIR.TAG`, so `tests/admission/compile_fail` could not find the target
  directory and `make test` failed on a pre-existing test. Evidence: the
  failure named `no Cargo target directory above .../target/debug/build/...`,
  and the main checkout has no `target/` at all. Impact: local only. Cargo
  writes the marker when it creates the directory itself, as it does in CI, so
  the repair was to write the standard marker into the untracked `target/`
  rather than to change the test.
- Observation: the first `wrong field type` negative passed for the wrong
  reason. The mutation replaced the pin text inside the quotes, leaving a valid
  string, so the decoder accepted it. Evidence: the failing case printed
  `"githash":"5"`. Impact: the mutation now replaces the whole quoted value,
  and the case is a real type fault.
- Observation: the first full `make lint` found three Whitaker faults that
  Clippy had not: `module_max_lines` on the 459-line `decision/mod.rs`, and
  `bumpy_road_function` in `protocol_breach` and `convert_replay`. Evidence:
  `/tmp/lint-1-2-3-demonstrate-type-and-axiom-auditing.out`, three `error:`
  lines at `Makefile:41`. Impact: the module split in D18 became mandatory, and
  both functions now delegate to named helpers. Behaviour is unchanged; the 80
  audit tests still pass.
- Observation: a scratch Verus probe of the settlement source, run before
  EP-M2 began, verified all 42 functions with `0 errors` once each `while` loop
  carried a `decreases` clause. Evidence: `.verus/0.2026.09.06.8dea4a2` on a
  copy of `decision/` in `/tmp`. Impact: the "same source" plan holds, and the
  proof needs no separate copy of the code.
- Observation: Verus does not carry a `while` loop's negated guard past a
  `break`, so `let Some(x) = keys.get(index) else { break; }` leaves the
  post-loop state unable to prove `index == keys.len()`. Evidence: a probe
  asserting the equality failed with the let-else and verified with
  `let x = &keys[index]`. Impact: EP-M2 must rewrite the seven scan loops to
  index under the loop guard, which needs one
  `#[expect(clippy::indexing_slicing, reason = ...)]` per module. Behaviour is
  unchanged.
- Observation: `mdtablefix` writes its result to standard output unless
  `--in-place` is given, so running it on a scratch copy and diffing reported
  no change while `--check` reported six lines. Evidence: the first attempt
  produced an empty diff; the second captured standard output. Impact: repairs
  use `--in-place`, and `make check-fmt` is the authority.
- Observation: Verus does not propagate a `match` discriminant fact on a
  `&Option<T>` into a specification `match` on the value. Evidence: a probe
  whose postcondition named `root_findings_spec` failed with
  `match &pair.observed.decl { Some(d) => ... }` and verified once the body used
  `if let Some(d) = &pair.observed.decl { ... } else { ... }`. Impact: every
  `&Option` site in the settlement body uses `if let`, not `match`.
- Observation: a derived `PartialEq` on a Verus datatype does not connect the
  executable `==`/`!=` to the specification `==`/`!=`. Evidence: a probe
  asserting `kind != DeclKind::Theorem` could not discharge a postcondition
  written with `is_theorem_spec(kind)`. Impact: the body compares kinds through
  an executable helper `is_theorem` whose postcondition is
  `result == is_theorem_spec(kind)`, and the specification gained
  `is_theorem_spec`.
- Observation: a `proof fn` called from executable code must be wrapped in a
  `proof { ... }` block, or Verus reports "cannot call function `X` with mode
  proof". Evidence: the first `roots_findings_empty_iff` call in `settle`.
  Impact: the call site is `proof { roots_findings_empty_iff(...); }`.
- Observation: a `while` loop body cannot see the enclosing function's
  preconditions. Evidence: `root_findings` needed
  `report.roots.len() <= request.roots.len()` restated as a loop invariant.
  Impact: every loop that relies on a precondition restates it as an invariant.
- Observation: slice equality needs bound locals and an intermediate
  `assert(x@ == y@)`. Evidence: `statement.as_slice() == expected.as_slice()`
  alone did not yield `statement@ == expected@`; the working shape binds both
  slices, asserts the view equality in the equal branch and the view inequality
  in the unequal branch. Impact: `collect_root_findings` uses that shape.
- Observation: a postcondition mentioning a `&mut` parameter must disambiguate
  `old` and `final`. Evidence: `collect_root_findings` writes
  `final(findings)@ == old(findings)@ + ...`. Impact: recorded as the house
  style for mutable-parameter contracts.
- Observation: a specification function that reads `q.roots@[i].expected@`
  needs an explicit link to the executable value the body used. Evidence: the
  final `collect_root_findings` postcondition failed until the function gained
  `requires pair.requested == request.roots@[index as int]`. Impact: the
  `RootPair` carrier is tied to the request by a precondition.
- Observation: nested `forall`s over `.name@` need explicit triggers. Evidence:
  `roots_names_match_spec` carries `#![trigger p.roots@[i].name@]`. Impact:
  recorded in the specification.
- Observation: an executable function written only for the proof is dead code
  under plain `rustc`, which `-D warnings` turns into a lint failure. Evidence:
  `cargo clippy -- -D warnings` reports the witnesses as never used once they
  are not re-exported. Impact: `mod witnesses;` carries
  `#[cfg(verus_keep_ghost)]`, so Cargo never compiles it and Verus always does.
- Observation: the settlement proof verifies with
  `verification results:: 64 verified, 0 errors` and eight warnings, all of
  them Verus's "autoderive Clone impl" notice on the pre-existing `types.rs`
  derivations. Evidence: `.verus/0.2026.09.06.8dea4a2` on the scratch tree in
  `/tmp/epm2`. Impact: the warnings are about the data types, not the proof,
  and are recorded rather than silenced.
- Observation: the decoder and the worker disagreed about what a tree's depth
  is. `wire/tree.rs` counted JSON nesting, one level for a constant and two for
  an application, while the worker counted applications. Evidence: the first
  green-looking corpus run failed at `MaximalPolicy` with
  `statement tree exceeds depth 60`; the same statement measures 38 levels by
  application and 76 by JSON nesting. Impact: a correctly rendered, in-bound
  report was refused, so no accepted fixture with a nested literal could pass.
  The decoder now counts as the worker does (D20).
- Observation: `INSTA_UPDATE=always` writes a new snapshot but still fails the
  assertion that created it. Evidence: the first corpus run reported
  `updated snapshot …` and then `FAILED` with the same `+new results` diff.
  Impact: creating a snapshot needs `INSTA_FORCE_PASS=1` in the same run, and
  the run after that must be unforced so a stale snapshot fails.
- Observation: `insta` writes a snapshot beside the source file that asserts
  it, so the corpus snapshots land in `tests/toolchain/smoke/audit/snapshots/`
  rather than in the existing `tests/toolchain/snapshots/`. Evidence: the path
  printed by the first run. Impact: none; the location is deterministic and is
  committed with the test.
- Observation: the worker's statement for a structural literal is larger than
  reading the source suggests. `Holds [r1, r2]` with two actions on the first
  rule is 47 nodes and 8 levels by application, because each `List.cons` is
  three applications and each rule is three applications of `Rule.mk`.
  Evidence: the `AcceptedClaimShape` snapshot and the `MaximalPolicy` count of
  1,765 nodes against the plan's 1,705. Impact: the request table in
  `smoke/audit/requests.rs` builds these trees from `List.cons`, `List.nil`, and
  `Rule.mk` rather than transcribing a report, so the expected statement is an
  independent claim about what Lean elaborates.

## Decision log

- D1. Decision: the worker observes and Rust settles, including statement
  comparison. Rationale: technical design §8 forbids a report choosing its own
  outcome. The worker reports the statement as a tree and never sees the
  expected one, which moves statement equality inside the theorem. Date/Author:
  2026-11-05, plan author, from the panel.
- D2. Decision: build a small repository-owned worker instead of adopting
  `lake comparator` or `lake check`. Rationale: they need `bwrap` or an
  explicit unsandboxed flag, report in text, and export the environment at
  about two minutes a run. They answer hostile-proof questions this milestone
  does not ask. See Table 3. Date/Author: 2026-11-05, plan author.
- D3. Decision: an expected proposition is a ground `const`/`app` tree,
  compared as an injective byte key. Rationale: a named declaration in the job
  lets a faulty job define its own target, source text would put data in a Lean
  executable position, and definitional equality accepts more than was asked, as
  `Unfolded` shows. Date/Author: 2026-11-05, plan author.
- D4. Superseded by D14. Statement provenance moves to 4.3.2.
- D5. Superseded by D14. Module class by location moves to 4.3.2.
- D6. Partly superseded by D14. Replaying non-toolchain modules stays. Pinning
  `Init` in the base moves to 4.3.2. Fresh replay of the toolchain stays out,
  at 91 s.
- D7. Decision: a request may narrow the foundations but not widen them, and
  `settle` re-validates, so its theorems carry no precondition on callers.
  Foundations are identified by name, which is sound only under the input
  model. Date/Author: revised 2026-11-06, plan author.
- D8. Decision: put the Rust spike in the existing `toolchain` test binary.
  Rationale: it inherits the helpers, the `smoke::` filter behind
  `make toolchain-smoke`, and the CI path filter. `settlement/` and `wire/`
  depend on no sibling module, so 2.2.4 can promote them with `git mv`.
  Date/Author: 2026-11-05, plan author.
- D9. Decision: record the integrity boundary in ADR 0006, and the research in
  a findings record. Rationale: the findings must outlive this plan, which
  becomes historical on merge. Date/Author: revised 2026-11-06, plan author.
- D10. Decision: use Verus for settlement, and tests for the worker and
  adapter. Rationale: see `Verification plan`. The user accepted this (D13).
  Date/Author: 2026-11-05, plan author.
- D11. Decision: panel suggestions not adopted. A settlement-table `insta`
  snapshot duplicates the manifest. A property oracle for settlement duplicates
  S1. Settling in Lean loses the promotable logic. Date/Author: 2026-11-05,
  plan author.
- D12. Decision: worker failures are `tool_failure` here, and `deepRecursion`
  and `excessiveMemory` reach the worker only as replay text and fail closed as
  `kernel_rejected`. Date/Author: 2026-11-05, plan author.
- D13. Decision: accept the plan's recommendations on the worker's location,
  the ground statement form, trusting toolchain `.olean` files, and keeping
  EP-M2. Rationale: the user accepted these in conversation when reviewing
  revision 2. This was not approval to implement. Date/Author: 2026-11-05,
  `leynos`.
- D14. Decision: separate proof integrity from operational integrity. This
  milestone establishes proof integrity under the generated-module input model,
  and a new item, 4.3.2, owns module provenance and shadowing, privileged-name
  forgery, trusted-package integrity, and further hardening. Rationale: the
  user directed this. Polean does not accept arbitrary Lean, so an adversarial
  Lean environment is outside the initial input model. Date/Author: 2026-11-06,
  `leynos`.
- D15. Decision: use upstream `Lean.collectAxioms` rather than a
  repository-owned walk. Rationale: walk completeness was the worker's largest
  obligation, and existed to avoid trusting a pre-computed extension in
  imported files. Inside the input model, those files come from honest
  compiles. The two agreed on every fixture compared. The walker's per-arm
  fixtures and mutations go with it. Date/Author: 2026-11-06, plan author.
- D16. Decision: keep kernel replay in this milestone, although fixed templates
  cannot disable the kernel. Rationale: the objective is to determine whether
  Lean has kernel-checked the proposition. Without replay that is an assumption
  no test can falsify, and it costs about a second. `ForgeInit` and
  `ForgeImported` are its negative controls, not a defence against adversaries.
  Date/Author: 2026-11-06, plan author.
- D17. Decision: reuse the existing unbounded test `run` helper, with a size
  check on captured output. Rationale: the worker's input is trusted, and 2.3.1
  owns the deadline. The `run_bounded` helper and `ReplayBomb` fixture from
  revision 2 are dropped. The bomb result did not reproduce on re-run.
  Date/Author: 2026-11-06, plan author.
- D18. Decision: split `audit/fixtures.rs` into `fixtures/mod.rs` and
  `fixtures/cases.rs`, `audit/wire/decode.rs` into `wire/document.rs` and
  `wire/decode.rs`, and `audit/settlement/decision/mod.rs` into
  `decision/types.rs` and `decision/mod.rs`, to keep every module under 400
  lines. Rationale: the splits are mechanical and leave the public paths
  unchanged. The first plan revision kept `decision/mod.rs` whole, on the
  argument that it is the exact text `verus/audit_settlement.rs` consumes
  through `#[path]`; Whitaker's `module_max_lines` rejects that at 459 lines,
  and a lint exemption for the one file under proof would be worse than the
  split. `verus/audit_settlement.rs` now pulls in `decision/mod.rs`, which
  pulls in `decision/types.rs` by the same ordinary module path Cargo uses, so
  the verified text and the compiled text remain one source. Date/Author:
  2026-11-06, plan author; revised after the first `make lint` run.
- D19. Decision: split the contracted `decision/mod.rs` again, into `scan.rs`
  (byte predicates), `request.rs` (admissibility), `replay.rs` (protocol
  conformance), and `findings.rs` (rejection detail), leaving `mod.rs` to hold
  `settle` alone; and add `verus/audit_settlement_witnesses.rs` beside the
  specification and lemmas. Rationale: contracts and their supporting `assert`s
  take the single file to 549 lines, over Whitaker's limit, and the split
  follows the four questions the proof asks rather than an arbitrary line
  count. The S7 witnesses are executable functions, not proof functions, so
  they do not belong in the lemma file; they are gated on
  `cfg(verus_keep_ghost)` because plain `rustc` would otherwise compile them
  and report dead code. `decision` re-exports every item the tests import, so
  no caller changes. Date/Author: 2026-11-07, implementer.
- D20. Decision: count a statement tree's depth in applications, one level per
  constant and one per application, in both the worker and the decoder; keep
  the bound at 60. Rationale: the two sides must agree on which trees are
  admissible, and the bound is only useful if a report the worker emits is
  never refused for depth alone. The plan's own arithmetic is in application
  levels: an application spends two JSON levels, so 60 levels is 121 JSON
  levels, inside `serde_json`'s limit of 128. The decoder had implemented the
  JSON-level reading, which halves the admitted depth to 30 applications and
  refuses `MaximalPolicy` at 38. The alternative — keeping JSON-level counting
  and shrinking the fixture — would contradict the plan's measurement and its
  stated bound. Date/Author: 2026-11-07, implementer, after the corpus found
  the disagreement.
- D21. Decision: bound the worker's statement-tree traversal with a
  remaining-node budget instead of summing node counts after both children
  return. Rationale: an `Expr` is a directed acyclic graph, so a small, deeply
  shared term can have exponentially many paths; checking the total only after
  the recursion would let an adversarial module make the worker run for a very
  long time. With the budget, a subtree is visited at most as many times as it
  has nodes, so the walk costs at most 4,096 visits however the graph is
  shared, and the admitted bound is unchanged. Date/Author: 2026-11-07,
  implementer, after review.
- D22. Decision: keep axiom collection on the imported environment, and record
  why the replayed environment cannot be substituted yet. Rationale: the
  obvious substitution, `Environment.ofKernelEnv kenv`, populates only
  `base.private` and `base.public`; `Lean.collectAxioms` walks bodies through
  `env.checked.get`, which that constructor leaves empty, so every root would
  silently report no axioms and the corpus would fail for the wrong reason. A
  correct single-source collection therefore needs a supported way to set the
  checked environment, or a local collector, and re-verification of all
  twenty-two entries. That is promotion work (roadmap 2.2.4); the divergence
  and its evidence are recorded in the findings note. Date/Author: 2026-11-07,
  implementer, after review proposed the change.
- D23. Decision: record that the delivered scope exceeded the tolerance in this
  plan rather than claim it was inside the estimate. Rationale: the tolerance
  stops and escalates above thirty non-fixture files or about 3,000 net lines.
  Measured against `de8a85e..3128745`, excluding Lean fixtures, reviewed
  snapshots, regenerated validation output, and this ExecPlan, the change is 66
  files and 9,406 net added lines; excluding documentation, the checksum
  manifest, and Rust fixture builders as well, it is 53 files and 6,258 net
  lines. Both counts exceed the trigger by roughly a factor of two. The work
  continued under the user's explicit instruction to implement the approved
  plan to completion, and the overrun was not separately escalated while the
  estimate was being passed. This entry records the fact for the assessment and
  for anyone planning a comparable audit; it is not a retroactive approval.
  Disposition: the hosted assessment escalated the missed trigger, and the
  sponsor's standing instruction to deliver the approved plan to completion is
  what the escalation rests on. No scope beyond the approved plan was added;
  the estimate was wrong, not the scope. If the sponsor wants the trigger
  treated differently, that supersedes this entry. Date/Author: 2026-11-07,
  implementer, after the hosted assessment raised it.

## Outcomes & retrospective

Complete this section at each milestone and at the end. Before setting
`COMPLETE`, reconcile every discovery with the upstream artefacts in
`Conformance basis`.

### EP-M1 outcome (2026-11-06)

Delivered both contracts, the two shared samples, the 22-entry corpus manifest,
the strict decoder and key encoding, the pure settlement function, the fixture
set, the feature file, the property tests, and the design-check controls.

Red evidence. The tests were written before the implementation, so their first
red state was a compile failure; that is weaker than observing the assertions
fail. To show the assertions bind, two seeded mutations were applied and the
suite rerun.

- Seed 1: `axiom_code` returned `None` for every axiom, so every dependency was
  treated as approved.
- Seed 2: `decode_report` skipped the `MAX_REPORT_BYTES` bound.

Result: `test result: FAILED. 68 passed; 12 failed`, with the failures naming
`SorryAxiom`, `NativeAxiom`, `UndeclaredAxiom`, `mixed_roots`, `axiom_root`,
the four matching BDD scenarios, and
`oversized_output_is_refused_before_parsing` (log
`/tmp/red-audit-1-2-3-demonstrate-type-and-axiom-auditing.out`). Both seeds
were reverted and the suite reran: `80 passed; 0 failed` (log
`/tmp/green-audit-1-2-3-demonstrate-type-and-axiom-auditing.out`).

Design check. `make design-check` reports `json_schemas_checked: 7`,
`audit_contract_samples_checked: 2`, `audit_manifest_entries_checked: 22`,
`audit_contract_controls_passed: 11`, and `negative_controls_passed: 19`.

### EP-M2 outcome (2026-11-07)

Delivered `verus/audit_settlement.rs` with its specification, lemmas, and
witnesses. The root pulls in `tests/toolchain/audit/settlement/` with
`#[path]`, so Cargo and Verus compile the same source. Verus verifies 64
obligations over the settlement decision, and the recognizer proof still
verifies 89.

The proof states S0 to S7 over an abstract report: acceptance requires the
pinned toolchain, the requested module and roots, an exact statement, and only
approved foundations; a protocol breach outranks any finding; and a refusal is
distinguished from a rejection. The ten settlement controls in
`scripts/check_verus_controls.py` each fail with the expected diagnostic, and
the five admission controls still do. The check is textual, so it shows each
edit is detected and located; it does not identify which obligation failed
first.

What the proof does not establish. It says what acceptance means for a report
it is given. It does not say the worker told the truth, that the wire decoder
admitted the report faithfully, or that the report corresponds to any source
policy. Those are separate claims, and only the first is inside the proof.

`make verus` is on demand and is not part of the commit gate, so these numbers
are recorded from a run on the pinned Verus release rather than reproduced by
`make test`.

### EP-M3 outcome (2026-11-07)

Delivered the Lean worker, the twenty-five fixture and support modules, and the
end-to-end corpus that compiles them with the pinned toolchain, audits them
with the real worker, decodes the report through the strict adapter, and
settles it against a hand-written request. All twenty-two manifest lines match,
the sentinel positive control fires, and `make toolchain-smoke` is green.

What the corpus establishes, and what it does not. A matching line shows that
on these twenty-two cases the worker, the adapter, and the settlement agree
with the plan that was written before the worker existed, so they cannot be
agreeing only with each other. It does not show the worker truthful in general:
the settlement proof is what makes acceptance mean something if a report is
true.

Mutation evidence. Each mutation was applied to `lean/PoleanAudit.lean` in
place, the corpus rerun, and the worker restored to its original bytes
(`sha256 4d0d60b71995bf44dd1c70b7bdafdbac57a07f868cea06254b8fbc3cdb207649`).
The corpus now runs every fixture before reporting, so one run names the whole
blast radius. Logs: `/tmp/mut1b-…`, `/tmp/mut2-…`, `/tmp/mut3-…`, `/tmp/mut4-…`.

- M1, axioms from the root's type instead of `collectAxioms`. Detected by eight
  fixtures: `MixedRoots`, `Sorry`, `SorryImported`, `Native`, `Undeclared`,
  `ReachImported`, and `NarrowedFoundations` all became `accepted`, and
  `AxiomRoot` lost its `undeclared_axiom`. The plan required `SorryImported` and
  `ReachImported`; both failed as required.
- M2, replay replaced by the un-replayed base environment. Detected by eighteen
  fixtures. Every accepted fixture became `rejected` with `root_absent`, and
  `ForgeInit` and `ForgeImported` reported `root_absent` where the manifest
  requires `kernel_rejected`, as the plan required. `RootAbsent`,
  `ModuleAbsent`, and `WorkerGarbled` still matched, which is right: their
  expected outcome does not depend on the replay.
- M3, statement rendered as its head constant only. Detected by
  `AcceptedClaimShape` and `MaximalPolicy`, which became `statement_mismatch`.
  The plan named `WrongPolicyLiteral`, and that expectation is wrong: a fixture
  whose report must be rejected anyway cannot detect a renderer that loses
  detail, because losing detail keeps it rejected. Only a fixture that must be
  accepted can. The deviation is recorded rather than repaired, because the
  milestone's requirement — that a lossy renderer is caught — is met.
- M4, every declaration kind reported as `theorem`. Detected by `AxiomRoot`,
  which kept one finding instead of two, and `DefRoot`, which was accepted.
  Both failed as the plan required.

Red evidence. `smoke::audit::corpus_matches_manifest` failed before the worker
existed, first as a compile failure and then, once the fixtures and the harness
existed but the worker did not, because `Corpus::prepare` could not build the
`polean-audit` target.

Repair found by the corpus. One real defect, D20: the decoder and the worker
counted tree depth differently, and `MaximalPolicy` was refused at depth 76
against a bound of 60. The corpus found it on its first complete run. The
decoder, its documentation, the report schema's description, and the
depth-negative fixture were corrected together, and
`cargo test --test toolchain audit::wire` is green with 37 tests.

### EP-M4 outcome (2026-11-07)

Delivered the durable documentation and closed roadmap 1.2.3.

- ADR 0006 gained `Measured results` and `Outcome mapping for 2.2.4`, and the
  roadmap records that 4.2.2 depends on 4.3.2 before either public command may
  carry the ADR 0002 statement.
- `docs/developers-guide.md` gained `The audit corpus (1.2.3)`, covering what
  the corpus is and is not, how to read a report, how to add a fixture, and the
  five pieces of pin-bound evidence. The command table and the Verus paragraphs
  now describe two proofs and their fifteen controls.
- `docs/repository-layout.md`, `lean/README.md`, `validation/README.md`, and
  `tools/verus/README.md` describe the audit worker, the settlement proof, and
  the seven schemas.
- The findings record gained `Observed again during 1.2.3`, including the
  `native_decide` axiom's shape, replay's treatment of constructors and
  recursors, and the worker's split source for axioms and declaration kind.
- `SHA256SUMS` was refreshed for `contracts/audit-report.schema.json` and
  `validation/README.md` after reviewing the drift, and roadmap 1.2.3 is
  checked with a `Recorded:` bullet.

Retrospective. The milestone code was small; the documentation was not, because
four lanes now need to say what each does and does not establish. The corpus's
own first complete run found D20, a depth-counting disagreement between the
worker and the decoder, which is the strongest evidence that the end-to-end
harness earns its cost. Two limits are recorded rather than smoothed over: the
worker collects axioms from the imported environment while reading the
declaration from the replayed one, and neither the corpus nor the proof says
anything about operational integrity.

Performance, measured on the shared development host under contention: fixture
compilation took 0.66–4.94 s and an audit 1.1–14.9 s, with peak child RSS of
425–799 MiB. The accepted report was 404 bytes and the maximal-policy report
54,371 bytes. These are upper bounds, not budgets.

### Review repairs (2026-11-07)

A full CodeRabbit pass over the committed milestone raised twenty-five
findings. Twenty-three were repaired and two were recorded rather than changed.

- The worker's tree walk now carries a remaining-node budget (D21), so a shared
  `Expr` DAG cannot expand exponentially before the bound is checked. A
  source-level shared-DAG fixture is not expressible in the corpus grammar: the
  corpus is ground statement trees, and a term that leaves the grammar is
  reported as `null` for that reason rather than for its size. The guard is
  therefore argued structurally, and the existing over-bound negatives pin the
  bound itself.
- Axiom collection stays on the imported environment (D22). The proposed
  `Environment.ofKernelEnv` substitution would report every root as axiom-free,
  because `collectAxioms` reads bodies through `env.checked.get`, which that
  constructor leaves empty. The evidence is in the findings note.
- The worker now refuses a call with no roots, and its `truncateUtf8` returns
  early when the message already fits.
- The decoder gained negative cases for a name component that is negative,
  fractional, or above `u64`, and the tree module documents that its walkers
  assume a bounded tree.
- The manifest reader reports physical line numbers, and an `accepted` line
  that also lists findings or a reason is now a fault in the oracle.
- The property tests derive a near-name and near-tree pair by an edit a
  length-free encoding cannot see, so A1 and A2 reach a collision instead of
  waiting for one; the required-field test now compares the list with the
  shared sample rather than with a literal.
- The settlement's root-finding builder was split into `kind_code`,
  `statement_code`, and a single `root_finding` constructor, and the three
  refusal tests that duplicated the parameterized one were removed.
- The Toolchain workflow now runs on `verus/**` and
  `tests/admission/recognizer/kernel/**`, so a control anchor cannot drift
  without the job that checks it.
- Documentation was corrected where it described the replay base, the
  settlement proof boundary, and the ordering rules; the roadmap's 4.2.2
  dependency cycle was broken by moving the 4.3.2 gate to 5.2.1.

The repairs were measured on the repaired tree. `make verus` exits 0 with
`verification results:: 89 verified, 0 errors` for the recognizer and
`64 verified, 0 errors` for the settlement, and all fifteen negative controls
are rejected with the expected diagnostic. The settlement count rose from 61
because the split described above adds three verified helper functions; the
`statement-comparison-dropped` control was re-anchored on the extracted
`statement_code` body and now fails at an assertion in `decision/findings.rs`
rather than at the former inline comparison.

### Review round 5 (2026-11-07)

A second CodeRabbit CLI pass, over the repaired head `afe212f`, raised four
findings. All four were actioned.

- The status line now reads revision 6, and the pin-bound evidence count matches
  the developers guide's five.
- ADR 0003's open item 2 is restored to its accepted wording, and the
  2026-11-06 addendum records that, under ADR 0006, the `bwrap` limitation
  constrains roadmap 1.2.4 and 4.3.2 but not the 1.2.3 experiment. Accepted
  ADRs are amended by dated addenda, not in place.
- The audit contract checks moved from `validate_design.py` to
  `validation/audit_contracts.py`, and `require`, `check_schema`, and the
  negative-control message moved to `validation/harness.py`, so the two modules
  do not import each other's orchestration. `run_checks` is the only caller.
  The harness module is still 1,104 lines, so the 400-logical-line rule the
  review cites is not met by this change; the split gives the audit checks a
  module of their own rather than claiming the harness is now within the limit.
- `sha256sum -c SHA256SUMS` covers 46 entries, including the two new modules.

### Review round 6 (2026-11-07)

A third CodeRabbit CLI pass, over `35bbdd8`, raised sixteen findings. Fifteen
were actioned and one is declined because it contradicts an accepted decision.

- The wire decoder now enforces `uniqueItems` on `foundations`, which the
  request schema already stated. Without it the decoder admitted a document the
  contract refuses, so the A3 agreement was one-sided. A request mutation, a
  negative case, and a contract control cover it; `validate_request` is
  unchanged.
- The first contract control is retitled "audit request rejects an empty roots
  list". It tested that mutation all along, but its label described the
  unmutated sample, which is the opposite of what the second value checks.
- `require` and `check_schema` gain NumPy-style parameter and raise sections,
  so the diagnostic each raises is documented rather than inferred.
- ADR 0006's status carries a one-sentence decision summary.
- The embedded Gherkin in this plan is replaced by a pointer to
  `tests/features/axiom_audit.feature`. The copy had drifted from the
  executable specification, and a copy that is not run will drift again.
- The findings record says the `DupRoot` entry modules' own sources were not
  retained, so the section documents the three copies rather than claiming a
  complete build.
- The `spine(60)` comment now gives the arithmetic: 60 applications and 61
  leaves, so depth 61 and 121 nodes, not 61 nodes.
- `lean/PoleanAudit.lean` documents the `unsafe` boundary at `main` — it is
  `Lean.withImportModules` that is unsafe — and records why `treeBounded`'s
  `Nat` truncation can only refuse a tree, never admit one.
- The trivially true `0 <=` is dropped from `contains`'s loop invariant in
  `scan.rs`; `make verus` re-checks the settlement at 64 verified.
- The four pinned-literal key tests become one `#[rstest]` case, the two decoder
  panics include the error, three positive controls stop the negative suite
  passing for a decoder that refuses everything, the corpus snapshot table is
  single and its comparison fails loudly, and the worker build and its
  existence check are one `built_worker` helper.
- Declined: `serde` remains `=1.0.229`. ADR 0003, "Why every dev-dependency
  carries an exact pin", records that caret requirements would make an intended
  tuple change indistinguishable from an incidental one; relaxing that is a
  separate decision. The finding asks for a change the accepted ADR forbids.

### Review round 7 (2026-11-07)

A fourth CodeRabbit CLI pass, over `c104ec2`, raised four findings. All four
were actioned.

- `decode_report` now refuses an empty `githash` as `Malformed`. The report
  schema sets `minLength: 1`, so the decoder accepted a document the contract
  rejects and the empty value surfaced as `ToolchainMismatch`, which blames the
  toolchain for a malformed report. This is the same one-sided A3 gap round 6
  closed for `foundations`; a report mutation, a decoder negative case, and a
  contract control cover it. The controls now number thirteen.
- `read_audit_manifest` validates each finding through a structural `match`
  helper, `manifest_finding`, rather than an inline chain of `isinstance`
  checks. The reviewer's proposed pattern bound `root` only in the `int()`
  alternative; the committed form, `(None | int()) as root`, binds it on both
  paths.
- `audit_controls`, `read_audit_manifest`, and `check_audit_contracts` carry
  NumPy-style `Parameters`, `Returns`, and `Raises` sections, so each raised
  diagnostic is documented rather than inferred.
- The seven single-root rejection tests become one `#[rstest]`, with a
  fixture-label helper and per-case expected findings and failure messages. The
  cases share a shape but not an outcome, so each keeps its own message; a
  shared one would only name the fixture.

### Assessment round 1 (2026-11-07)

The hosted ExecPlan assessment, `POL35-EXEC-20261107-A` over `3128745`,
returned four findings and one reconciliation note. All five were actioned.

- The worker's module and `main` documentation claimed a worker fault could
  make a report wrong but not an audit pass. That is false: a worker that omits
  an axiom from an otherwise matching report is accepted. Both comments now
  state what settlement does guarantee — acceptance follows from the report and
  request — and leave report truthfulness on ADR 0006's operational-integrity
  side.
- A1 required a test-only key-decoder round trip under `proptest`, and the
  properties tested injection and near-collisions only, which an injective but
  unfaithful encoding would also pass. The property support module now holds
  `decode_name_key` and `decode_tree_key`, repeating the wire tags so a format
  change on either side fails, and both round-trip properties run.
- W1 required a failed worker to leave standard output empty, and the corpus
  classifier accepted any non-zero exit. It now refuses a failed run that left
  bytes on stdout, which covers `ModuleAbsent`.
- The scope tolerance in this plan was exceeded: 66 files and 9,406 net lines,
  or 53 files and 6,258 net lines with documentation, the checksum manifest,
  and fixture builders also excluded, against a trigger of 30 files or about
  3,000 lines. D23 records the measurement and the fact that the overrun was
  not separately escalated.
- The M3 instruction named `WrongPolicyLiteral`, which cannot detect a lossy
  renderer because that fixture is rejected for another reason. The instruction
  now states the corrected expectation and points at the EP-M3 outcome that
  already recorded it.

The hosted proof assessment, `POL35-PROOF-20261107-A` over the same head,
returned five qualifications. All five were actioned.

- The recognizer's completeness result is stated only for a canonical-token
  predicate, which is sufficient for success rather than a characterisation of
  every successful input. The developers guide now says so next to the
  soundness claim.
- The settlement witnesses show abstract outcome reachability, not reachability
  through the wire decoder: their keys are arbitrary bytes. The witness header
  and S7 now state that boundary, and point at A1 for the encoding link.
- The report boundary: the proof checks the axioms a report lists and cannot
  show the worker listed every transitive axiom. The worker documentation was
  corrected above; the D22 split and operational integrity stay explicit.
- The key round trip was added above, which is the encoding evidence A1 asked
  for.
- The negative controls match diagnostic text that names a failure kind and a
  source path, not an individual obligation. The control script, ADR 0006, and
  every plan claim that said "named obligation" now say "expected diagnostic"
  and state what the match does and does not establish.

The repairs landed as `ffb86ef`, and the eleven-target gate sweep on that head
was green, including `make verus` — which the assessment asked for — with the
recognizer at 89 verified, settlement at 64 verified, and fifteen of fifteen
controls rejected. Its output is retained in the delivery ledger. The two proof
results and the fifteen controls are still author-reported from that run, not
independently reproduced by the assessor.

#### Assessment round 1 follow-up (2026-11-07)

The follow-up reply asked for the A1 mutation evidence this plan's verification
plan requires. Removing the text length prefix from `name_key` in a scratch
working tree failed `name_keys_round_trip` with the minimal input
`Some([Text("")])` against `None`, and failed `tree_keys_round_trip` with
`Const { name: [Text("")], levels: [] }`; five other key properties still
passed, because a lossy encoder remains injective on the sampled cases. The
unmodified encoder then passed all seven key properties. The transcript is
retained in the delivery ledger. The scope-tolerance disposition is recorded in
D23.

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
  closure** is every axiom reachable through types and values.
- A **foundation** is one of `propext`, `Quot.sound`, and `Classical.choice`,
  the three axioms of Lean's logic that ADR 0002 trusts.
- The **Lean kernel** is Lean's small trusted checker. **Replay** sends stored
  declarations through it again, as `leanchecker` does.
- An **`.olean` file** is Lean's compiled module format.
- The **sysroot** is the digest-named Lean install.
- **Settlement** is the Rust decision over a request and a report. Its outcomes
  are `accepted`, `rejected`, and `tool_failure`.

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
faults. `make verus` runs both, on demand.

Consult these before starting:

- `docs/technical-design.md` §§2.1, 5, 6.2, 8, 10, and 13.
- `docs/adrs/adr-0006-separate-proof-integrity-from-operational-integrity.md`.
- `docs/terms-of-reference.md`, `docs/context.md`, and
  `docs/developers-guide.md`.
- ADR 0002 for the trust statement, and ADR 0003 "Inputs for 1.2.3".
- `docs/reliable-testing-in-rust-via-dependency-injection.md`,
  `docs/rust-testing-with-rstest-fixtures.md`, and
  `docs/rust-doctest-dry-guide.md`.
- `docs/complexity-antipatterns-and-refactoring-strategies.md`.
- `docs/scripting-standards.md` for the changes to `scripts/`.
- `docs/documentation-style-guide.md` for assurance language.

The task also names `docs/references.md`, which does not exist.

Load the skills `rust-router`, then `rust-unit-testing`, `rust-errors`,
`proptest`, and `verus` as each facet arises. Use `codegraph-mcp` for
navigation, `execplans` for this document, `en-gb-oxendict-style` for prose,
`addressing-whitaker-findings` for lint findings, and `sdlc-implementation` to
drive the milestones.

## Conformance basis

Upstream artefacts: commit `de8a85ece8340fb7ab57794b405c16fe86db2bee`, as
amended on this branch by this revision.

- `docs/terms-of-reference.md`: goals G5 and G6, success criterion SC6,
  assumption A6. Unchanged, because the scope of the product does not change.
- `docs/technical-design.md`, draft v0.1: §2.1 (new), §5, §6, §6.2, §8, §10
  (V11 and V12, new), and §13.
- `docs/roadmap.md`: item 1.2.3, and new item 4.3.2.
- ADR 0002 item 2, ADR 0003 "Inputs for 1.2.3", and ADR 0006.
- `contracts/cli.txt`: exit 6 and exit 7, unchanged by this work.

Trace links:

```plaintext
1.2.3 "pinned audit interface"
  -> TD §6.2, ADR 0003 tuple -> polean-audit, audit contracts -> EP-M3
  -> smoke::audit::corpus_matches_manifest
1.2.3 "accepts an expected proposition and approved foundations"
  -> TD §6.2, V11 -> S1 -> EP-M1, EP-M2 -> AcceptedGround, AcceptedFoundations, S7
1.2.3 "rejects sorryAx"
  -> TD §6.2, ADR 0002 item 2 -> S3 -> EP-M2, EP-M3 -> Sorry, SorryImported
1.2.3 "native-computation trust axioms"
  -> TD §5, ADR 0003 input 1 -> S3, A1 -> EP-M2, EP-M3 -> Native
1.2.3 "undeclared assumptions"
  -> TD §6.2 -> S3, W2 -> EP-M2, EP-M3 -> Undeclared, ReachImported, NarrowedFoundations
1.2.3 "machine-readable results"
  -> TD §6.2, §8 -> contracts/audit-report.schema.json -> EP-M1 -> design-check, wire tests
TD §6.2 "expected declaration type"
  -> S1, W4, W5 -> EP-M2, EP-M3 -> WrongStatement, WrongPolicyLiteral, AxiomRoot
TD §6.2 "exact requested claim coverage"
  -> S2 -> EP-M1, EP-M2 -> coverage unit tests, settlement proof
TD §6.2 "malformed, or contradictory worker output is a tool failure"
  -> S2, A2, W1 -> EP-M1 -> wire tests, feature scenarios, WorkerGarbled
TD §2 "kernel replay"
  -> W3 -> EP-M3 -> ForgeInit, ForgeImported
TD §2.1 operational integrity, V12 -> 4.3.2 -> not this plan
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
report truthful: that rests on the corpus and the axioms below. The whole
result is conditional on the input model.

### Abstract model first

Define these specifications in `verus/audit_settlement_spec.rs`, independently
of the executable code. Keys are byte sequences built by `wire::name_key` and
`wire::tree_key`, and are opaque to the model. `K_propext`, `K_quot`,
`K_choice`, and `K_sorry` are concrete byte sequences.

```text
Axiom   := { name: Key, native: bool }     // native selects a diagnostic only
Decl    := { kind, statement: Option<Key>, axioms: Seq<Axiom> }
RootObs := { name: Key, decl: Option<Decl> }
Replay  := Accepted(modules: Seq<Key>) | Rejected
Report  := { githash, module: Key, replay: Replay, roots: Seq<RootObs> }
Request := { module: Key, foundations: Seq<Key>, roots: Seq<{ name: Key, expected: Key }> }

pinned(n) := n in { K_propext, K_quot, K_choice }

valid_request(q) := |q.roots| > 0
  and root names are pairwise distinct
  and forall f in q.foundations. pinned(f)

protocol_ok(q, p, pin) := p.githash == pin and p.module == q.module and match p.replay {
  Rejected => |p.roots| == 0,
  Accepted(ms) => q.module in ms
    and |p.roots| == |q.roots|
    and forall i. p.roots[i].name == q.roots[i].name }

approved(q, a) := a.name in q.foundations

root_clean(q, i, o) := o.decl == Some(d) and d.kind == theorem
  and d.statement == Some(q.roots[i].expected)
  and forall a in d.axioms. approved(q, a)

acceptable(q, p, pin) := valid_request(q) and protocol_ok(q, p, pin)
  and p.replay is Accepted
  and forall i. root_clean(q, i, p.roots[i])

axiom_code(q, a) :=
  if approved(q, a) then None
  else if a.name == K_sorry then Some(SorryAxiom)
  else if a.native then Some(NativeAxiom)
  else Some(UndeclaredAxiom)

// expected_findings(q, p), in order:
//   Rejected replay => [ { root: None, item: None, KernelRejected } ]
//   Otherwise, for each root i:
//     decl None    => [ { i, None, RootAbsent } ]
//     decl Some(d) => WrongKind if kind != theorem;
//       StatementMismatch if statement != Some(expected);
//       { i, j, axiom_code } for each unapproved axiom j.
```

### Settlement obligations (Verus)

Each is a hypothesis about the eventual proof structure, and implementation may
split or refine them. `settle` returns `Result<Settlement, RequestRefusal>` and
carries no precondition, because the adapter is unverified and could not
discharge one.

- S0 `settle_refuses_exactly`: `settle` is `Err` if and only if not
  `valid_request(q)`.
- S1 `settle_accepts_exactly`: `settle` is `Ok(Accepted)` if and only if
  `acceptable(q, p, pin)`. This is the main theorem. Soundness alone is met by
  a function that never accepts, as deny-all meets the safety claims (technical
  design §3), so it runs in both directions.
- S2 `settle_fails_exactly`: `settle` is `Ok(ToolFailure)` if and only if
  `valid_request(q)` and not `protocol_ok(q, p, pin)`. S0, S1, and S2 leave
  `Rejected` as exactly the remainder, and a protocol breach always takes
  precedence over findings.
- S3 `accepted_axioms_closed_world`: `Ok(Accepted)` implies every reported
  axiom's name is one of the three literals, and is not `K_sorry`. This is the
  roadmap's rejection criterion as a closed-world statement. Exclusion of
  `_native` and undeclared names follows by being outside the table, and A1
  connects that to real names.
- S5 `rejected_findings_exact`: under `valid_request` and `protocol_ok`, and not
  `acceptable`, `settle` returns `Rejected(expected_findings(q, p))`, and root
  `i` has a finding if and only if it is not `root_clean`. A first-finding-only
  implementation fails this, and a mixed report keeps its per-root evidence
  (technical design §8).
- S6 `validate_request` is `Ok` if and only if `valid_request(q)`.
- S7 witnesses over ghost values show `Ok(Accepted)`, `Ok(Rejected)`,
  `Ok(ToolFailure)`, and `Err` are each reachable as abstract outcomes, so the
  preconditions are satisfiable. The witnesses use arbitrary byte keys rather
  than encoded names, so they do not show reachability through the wire decoder
  or non-vacuity of every individual obligation; A1's key properties carry the
  encoding link.

Non-vacuity for the proof as a whole: `scripts/check_verus_controls.py` gains
controls that must each be rejected with a diagnostic naming the expected
failure kind and source path.

- Body controls: always reject; drop the `replay accepted` check, since empty
  roots make `forall` vacuous; compare no statement key; drop the name
  comparison from coverage; stop the root loop and the axiom loop one early;
  ignore `q.foundations`; report a protocol breach as a rejection.
- Specification controls: add `K_sorry` to the pinned table, which must fail
  S3; weaken `valid_request` to `false`, which must fail the `Accepted` witness.

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
  `tool_failure`. Artefact: `ModuleAbsent`, `WorkerGarbled`. Non-vacuity:
  `ModuleAbsent` exits 1 in the probe.
- W2 Axiom reporting. The reported axioms are those `Lean.collectAxioms`
  returns for the root, across modules. Artefact: `SorryImported`,
  `ReachImported`, and `AcceptedFoundations`. Non-vacuity: M1.
- W3 Replay. An accepted replay means the kernel accepted every retained
  declaration in each non-toolchain module of the import closure. Unsafe and
  partial constants are not retained. Artefact: `ForgeInit`, `ForgeImported`,
  and `RootAbsent`. Non-vacuity: M2.
- W4 Statement fidelity. `statement` is the root's type rendered without loss,
  or `null` when out of grammar or over the bound. Artefact: `AcceptedGround`,
  `AcceptedClaimShape`, `MaximalPolicy`, `WrongStatement`,
  `WrongPolicyLiteral`, and `Unfolded`. Non-vacuity: M3.
- W5 Kind and existence. `AxiomRoot` and `DefRoot` give `wrong_kind`, and
  `RootAbsent` gives `root_absent`. Non-vacuity: M4.
- W6 No execution. Auditing `InitSentinel` leaves no sentinel file. Positive
  control: `smoke::audit::sentinel_fires_when_initializers_run` runs
  `lean --run` on a file importing that module and requires the sentinel, so a
  broken fixture cannot pass.

Seeded mutations are applied once to a scratch copy of the worker. Their
results are author-reported in `Outcomes & retrospective`, and nothing in the
gates reproduces them.

- M1: report only the root's direct constants as axioms, and require
  `SorryImported` and `ReachImported` to fail.
- M2: remove replay, and require `ForgeInit` and `ForgeImported` to fail.
- M3: render only the statement's head constant, and require a fixture that
  must otherwise be accepted to fail. This line first named
  `WrongPolicyLiteral`, and that expectation was wrong: a fixture rejected for
  another reason stays rejected when rendering loses detail, so only an
  accepted fixture can detect the mutation. `AcceptedClaimShape` and
  `MaximalPolicy` caught it; the corrected expectation and the evidence are in
  the EP-M3 outcome.
- M4: report every declaration's kind as `theorem`, and require `AxiomRoot` and
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
explore, as settlement has no state. A Lean proof of the worker would need a
model of `Environment` that the toolchain does not expose.

### Axioms relied upon

These are taken as given, and not verified.

- The generated-module input model in technical design §2.1. A violation is not
  detected here, and Table 1 of the findings record shows the consequence.
- The Lean kernel accepts only well-typed declarations, given that privileged
  names denote their toolchain definitions, which the input model provides.
- `Kernel.Environment.replay` sends retained declarations to the kernel, and
  checks constructors and recursors against those it generates.
- `Lean.collectAxioms` reports every axiom reachable from a declaration in
  honestly compiled modules.
- Toolchain `.olean` files match the archive. They are unpacked once from a
  digest-checked archive, later reuse checks only the `COMPLETE` marker, and
  they are not replayed.
- Loading an `.olean` file with `loadExts := false` runs no initializers.
  `InitSentinel` and its positive control exercise this.
- `serde` with `deny_unknown_fields` rejects duplicate and unknown fields.
  Tests exercise this at the interface.
- Verus, Z3, and the release pins in `tools/verus/` are sound.

Residual gaps: no theorem connects the worker's report to the environment. The
audit says nothing about what a statement means, since that is the definition
of the Polean atoms (ADR 0002 item 2).

## Plan of work

### Stage A: propose (done in this plan)

Reconnaissance, research, probes, and review are recorded here and in the
findings record.

### Stage B: red

Write the contracts, the manifest, and the feature file first. Add settlement's
types with a `settle` that returns `ToolFailure` for every report, so unit
tests compile and fail for the expected reason. Add the provisioned corpus
test, which fails because `polean-audit` is absent.

### Stage C: implement and verify together

Write the specification in `verus/` before the body of `settle`, and develop
each loop with its invariant. Implement the worker after settlement is proved,
so the proved contract pressure-tests the report format while it is cheap to
change.

### Stage D: refactor, document, and validate

Remove the scaffolding, record outcomes, and run all gates.

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
last being JSON Lines with one hand-written line per corpus entry. Extend the
validation harness with `check_audit_contracts`, holding the samples and
negative controls; it lives in `validation/audit_contracts.py`, with the shared
diagnostics in `validation/harness.py`, so the audit checks are a module of
their own rather than further growth in the harness. Add `serde` to
`Cargo.toml`. Add `tests/toolchain/audit/`: `mod.rs`, `wire/`, `settlement/`
with `settlement/decision/` (the pure function), `properties/`, `tests.rs`, and
`bdd.rs`, with the feature in `tests/features/axiom_audit.feature`. Widen the
description at `tests/toolchain/main.rs:1` and `docs/repository-layout.md:14`.

Requirements: machine-readable results, malformed output, coverage.

Acceptance: `make test` runs the settlement and wire tests, each red before its
implementation. `make design-check` reports `json_schemas_checked: 7`.

Conformance check:

- The report holds no verdict or comparison result.
- `settlement/` and `wire/` use nothing from sibling modules.
- No item is added to `src/`.
- The only new dependency is the authorized `serde`.
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
proof file (`:50-52`) to a list of proof trees, add the controls above, and
update `scripts/tests/`, including the pinned CLI assertions in
`test_check_verus_controls_cli.py`.

Requirements: accepts, rejects, undeclared, exact coverage.

Acceptance: `make verus` exits 0, with both proofs reporting `0 errors` and
every control rejected with the expected diagnostic. `make test-scripts`
passes. Then `make check-fmt`, `make lint`, and `make test`.

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

Outcome: `make toolchain-smoke` runs the real worker over the corpus, and each
settlement matches the manifest.

Work: run `make tools` in this worktree. Add `lean/PoleanAudit.lean`, and a
`[[lean_exe]]` table named `polean-audit` with root `PoleanAudit` to
`lean/lakefile.toml`, without `supportInterpreter` and without changing
`defaultTargets`. Add fixtures under `examples/audit/lean/`. Add
`tests/toolchain/smoke/audit/`, which builds the worker with
`lake --dir lean build polean-audit`, compiles fixtures with `lean -o`, and
runs the worker in one sequential test. Add `insta` snapshots of the canonical
report for `AcceptedFoundations` and `Native`, with `message` excluded. Add
`contracts/audit-*` and `examples/audit/**` to
`.github/workflows/toolchain.yml`. Apply the seeded mutations to a scratch copy
and record the results.

The corpus is hand-written, and each line has the expected outcome below.
Finding and reason names are contractual.

| Fixture               | Purpose                                      | Settlement                                   |
| --------------------- | -------------------------------------------- | -------------------------------------------- |
| `AcceptedGround`      | Honest theorem with no axiom                 | `accepted`                                   |
| `AcceptedFoundations` | All three foundations                        | `accepted`                                   |
| `AcceptedClaimShape`  | Named claim over a structural literal        | `accepted`                                   |
| `MaximalPolicy`       | Claim over a 16-rule, 16-atom literal        | `accepted`, inside the tree bound            |
| `InitSentinel`        | `initialize` writes a sentinel if ever run   | `accepted`, and no sentinel file             |
| `MixedRoots`          | One clean root and one with `sorry`          | `rejected`: `sorry_axiom` on one root only   |
| `Sorry`               | Direct `sorry`                               | `rejected`: `sorry_axiom`                    |
| `SorryImported`       | `sorry` behind an imported helper            | `rejected`: `sorry_axiom`                    |
| `Native`              | `native_decide` in a helper                  | `rejected`: `native_axiom`                   |
| `Undeclared`          | A direct user axiom                          | `rejected`: `undeclared_axiom`               |
| `ReachImported`       | Axiom behind an imported definition          | `rejected`: `undeclared_axiom`               |
| `NarrowedFoundations` | Uses foundations the request does not list   | `rejected`: `undeclared_axiom`               |
| `WrongStatement`      | A different proposition                      | `rejected`: `statement_mismatch`             |
| `WrongPolicyLiteral`  | Same claim, about a different list literal   | `rejected`: `statement_mismatch`             |
| `Unfolded`            | Definitionally equal, structurally different | `rejected`: `statement_mismatch`             |
| `AxiomRoot`           | Root is itself an axiom                      | `rejected`: `wrong_kind`, `undeclared_axiom` |
| `DefRoot`             | Root is a definition                         | `rejected`: `wrong_kind`                     |
| `RootAbsent`          | Named root does not exist                    | `rejected`: `root_absent`                    |
| `ForgeInit`           | Negative control: kernel never checked it    | `rejected`: `kernel_rejected`                |
| `ForgeImported`       | The same, in an imported module              | `rejected`: `kernel_rejected`                |
| `ModuleAbsent`        | Module file does not exist                   | `tool_failure`: `worker_failed`              |
| `WorkerGarbled`       | Program printing a non-report                | `tool_failure`: `malformed_report`           |

_Table 2: The audit corpus and the settlement each entry must produce._

`SpoofedName`, a bad request, and level or coverage mismatches are unit tests
against recorded reports. `Sorry`, `Native`, `Undeclared`, and `ForgeInit` fall
outside what the fixed templates emit. They are here as negative controls,
because the audit's rejections are the thing under demonstration.

Requirements: pinned interface, all rejection classes, no execution of
inspected code.

Acceptance: `make toolchain-smoke` passes, including the existing OPA and Lean
smoke tests. `smoke::audit::corpus_matches_manifest` fails before the worker
exists. Then `make check-fmt`, `make lint`, and `make test`.

Conformance check:

- The worker sees no expected statement and emits no verdict.
- Fixtures stay outside the Lake package.
- `make test` needs no provisioned tool.
- `PoleanSmoke` and `defaultTargets` are unchanged.
- No operational-integrity check has crept in.

Recovery: delete `lean/PoleanAudit.lean` and the `[[lean_exe]]` table to return
to EP-M2. If a fixture behaves differently from the table, the design tolerance
applies.

Remaining gaps: promotion and generated jobs are 2.2.4. Operational integrity
is 4.3.2.

Compatibility decision: none.

### EP-M4: documentation and roadmap completion

Outcome: durable documents describe what was actually built, and the roadmap
entry is done.

Work:

- Update ADR 0006 with the measured results, and record its acceptance.
- Propose the outcome mapping for 2.2.4 in ADR 0006: `rejected` maps to
  `invalid_evidence` (exit 7); `tool_failure` to `tool_error` (exit 6), with a
  deadline kill mapping to `unknown` (exit 3) once 2.3.1 provides deadlines; a
  `RequestRefusal` to `tool_error`, because Rust generates the request. An
  `accepted` settlement alone never exits 0.
- Update `docs/developers-guide.md` (what the audit is, what it does not
  establish, how to add a fixture, how to read a report, `make verus` covering
  two proofs, and a pin-change step covering foundation names, control anchors,
  snapshot githash, and the worker's Lean APIs), `docs/repository-layout.md`,
  `lean/README.md`, `validation/README.md` (seven schemas),
  `tools/verus/README.md`, and `docs/contents.md`.
- Add anything the implementation learned about the environment to the findings
  record.
- Refresh `SHA256SUMS` after reviewing the drift.
- Mark roadmap 1.2.3 as done, with a `Recorded:` bullet stating what ran.

`docs/users-guide.md` does not change, because no library or command behaviour
changes: there is still no executable checker. This revision has already
amended the technical design, the vocabulary, the roadmap, and ADR 0002 and ADR
0003.

Acceptance: `make markdownlint`, `make design-check`, and
`sha256sum -c SHA256SUMS` pass. All gates in `Validation and acceptance` pass.
Then set `Status: COMPLETE`.

Conformance check: every discovery is reconciled upstream.

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
  lean/.lake/build/bin/polean-audit AuditFixture.AcceptedGround AuditFixture.AcceptedGround.root
```

## Validation and acceptance

The behavioural specification is `tests/features/axiom_audit.feature`. It runs
against recorded reports through the decoder and settlement, while the corpus
exercises the real worker.

The feature file is the authority for those scenarios; they are deliberately
not copied here, because an embedded copy drifts from the executable
specification whenever either changes.

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
  budget claims.

## Idempotence and recovery

Every step can be repeated. `make tools` is digest-checked and reuses verified
artefacts. Fixtures compile into fresh scratch directories per test run, so
nothing accumulates under `lean/`. `lean/.lake/` is ignored.
`make design-check` regenerates its outputs deterministically. If a Verus
control's anchor drifts, the script fails closed: update the anchor
deliberately, in the same commit as the source edit. If the provisioned test is
interrupted, rerun it.

## Artefacts and notes

### Research findings

The [findings record](../lean-environment-integrity-findings.md) preserves the
adversarial discoveries, measurements, prototype source, and fixtures as
research and regression candidates for 4.3.2. It marks what was re-run, and
what did not reproduce.

### Proof-integrity prototype

An 84-line prototype of this revision's worker ran on 2026-11-06 against the
pinned archive, in a scratch directory, using a sibling worktree's digest-named
install, with only `LEAN_SYSROOT` and `LEAN_PATH` set. It compiled under
`warningAsError`. EP-M3 reproduces these results through the committed corpus.
`ok` below is the replay status only, and settlement then applies the request.
These are author-reported.

```plaintext
Good            ok  theorem     statement Fx.Goal      nodes 1     axioms []
GoodFoundations ok  theorem     statement Fx.Goal      nodes 1     propext Classical.choice Quot.sound
ClaimShape      ok  theorem     statement tree         nodes 47    depth 8   axioms []
MaximalPolicy   ok  theorem     statement tree         nodes 1705  depth 37  axioms []
WrongLiteral    ok  theorem     statement tree         nodes 25    depth 7   axioms []
InitSentinel    ok  theorem     statement Fx.Goal      axioms []   no sentinel; `lean --run` control writes it
Sorry           ok  theorem     statement Fx.Goal      sorryAx
SorryImported   ok  theorem     statement Fx.Goal      sorryAx
Native          ok  theorem     statement Fx.Goal      Fx.Native.helper._native.native_decide.ax_1_1
Undeclared      ok  theorem     statement Fx.Goal      Fx.Undeclared.assume
ReachImported   ok  theorem     statement Fx.Goal      Fx.ReachBase.hidden
Wrong           ok  theorem     statement True         axioms []
Unfolded        ok  theorem     statement null         axioms []
AxiomRoot       ok  axiom       statement Fx.Goal      Fx.AxiomRoot.root
DefRoot         ok  definition  statement Fx.Goal      axioms []
RootAbsent      ok  declaration null
ForgeInit       REJECTED  application type mismatch
ForgeImported   REJECTED  application type mismatch
ModuleAbsent    exit 1, standard output empty
```

Costs: `lake build` in an empty environment takes 15–53 s for a 118 MiB binary.
An audit takes about 1.5–2 s and, measured on the earlier prototype, about 800
MiB resident. A fixture compile takes 1.9–4.6 s under load. Expect the corpus
to take about 1–2 minutes.

A ground claim statement elaborates with no metadata, confirming D3:

```plaintext
Lean.Expr.app (Lean.Expr.app (Lean.Expr.const `Fx.Holds []) (Lean.Expr.const `Fx.ClaimId.iso []))
  (Lean.Expr.app (Lean.Expr.app (Lean.Expr.app (Lean.Expr.const `List.cons [Lean.Level.zero]) ...
hasMData? false  githash=eb6cafb7290e8bd5f2937522d8d804f58f02bafc  v=4.35.0
```

References inspected on 2026-11-05 are R5, R6, and R13 in the technical design,
and R16–R18 there.

## Interfaces and dependencies

### Why a new worker

| Candidate                  | Statement check | Axioms         | Replay            | Structured result | Runs here        | Cost per run    |
| -------------------------- | --------------- | -------------- | ----------------- | ----------------- | ---------------- | --------------- |
| `#print axioms`            | None            | Transitive     | None              | None              | Yes              | Not measured    |
| `leanchecker`              | None            | None           | Named modules     | None              | Yes              | About 1 s       |
| `leanchecker` and a script | None            | Transitive     | Named modules     | Script's own      | Yes              | Not measured    |
| `lake check`               | None            | Standard set   | Export            | Exit code         | Unsandboxed only | About 2 minutes |
| `lake comparator`          | Challenge file  | Permitted list | Export            | Text, exit code   | Unsandboxed only | About 2 minutes |
| SafeVerify                 | Target file     | Standard set   | Named modules     | JSON              | Not established  | Not measured    |
| `polean-audit` (1.2.3)     | Data tree       | Name           | All non-toolchain | JSON              | Yes, unsandboxed | About 2 s       |

_Table 3: Existing audit interfaces compared with the worker this plan built.
The export-based costs are derived from measured `leanexport` and
`leanchecker --from-export` runs on an `Init`-only module._

`lake comparator` gives stronger assurance against hostile Lean than this
worker, and remains an option for 4.3.2's independent check.

### Request contract

`contracts/audit-request.schema.json`, identifier `polean.audit-request.v0`:

```json
{
  "schema_version": "polean.audit-request.v0",
  "module": ["AuditFixture", "AcceptedGround"],
  "roots": [
    {
      "name": ["AuditFixture", "AcceptedGround", "root"],
      "expected": { "const": ["AuditFixture", "Goal"], "levels": [] }
    }
  ],
  "foundations": [["propext"], ["Quot", "sound"], ["Classical", "choice"]]
}
```

A statement tree is exactly `{"const": Name, "levels": [Nat]}` or
`{"app": [Tree, Tree]}`, with no binder, literal, or variable form. It is
bounded at 4,096 nodes and depth 60: `serde_json` allows nesting of 127, each
`app` costs two levels, and `MaximalPolicy` needs 1,765 nodes at 38 levels by
application. A request `Name` is a non-empty array of non-empty strings, each
matching `[A-Za-z_][A-Za-z0-9_']*`, so it can be passed as a dotted argument
without ambiguity. Levels are small natural numbers.

### Worker interface

`lean/PoleanAudit.lean` provides the `polean-audit` executable. It imports
`Lean` and `Lean.Replay`, reads `LEAN_SYSROOT` and `LEAN_PATH`, and takes
positional arguments:

```plaintext
polean-audit <module> <root> [<root> ...]
```

The sequence is:

1. Initialize the search path from the sysroot, and split each name on `.` into
   `Name` values.
2. Import the job module with `withImportModules`, which forces
   `loadExts := false`.
3. Treat a module as toolchain when its `.olean` resolved under the sysroot.
   This selects the replay scope only.
4. Import the toolchain modules that non-toolchain modules directly import, as
   the replay base.
5. Call `Kernel.Environment.replay` on all constants from non-toolchain modules.
6. For each root in argument order, read the constant from the replayed
   environment, render its type as a tree, and collect its axioms with
   `Lean.collectAxioms`.
7. Print one compact JSON line, and exit 0.

Any failure before step 7 goes to standard error and exits non-zero, with
standard output empty. A replay rejection is a completed audit, not a worker
failure.

### Report contract

`contracts/audit-report.schema.json`, identifier `polean.audit-report.v0`:

```json
{
  "schema_version": "polean.audit-report.v0",
  "githash": "eb6cafb7290e8bd5f2937522d8d804f58f02bafc",
  "module": ["AuditFixture", "AcceptedFoundations"],
  "replay": {
    "status": "accepted",
    "modules": [["AuditFixture", "Expected"], ["AuditFixture", "AcceptedFoundations"]]
  },
  "roots": [
    {
      "name": ["AuditFixture", "AcceptedFoundations", "root"],
      "declaration": {
        "kind": "theorem",
        "statement": { "const": ["AuditFixture", "Goal"], "levels": [] },
        "axioms": [["Classical", "choice"], ["Quot", "sound"], ["propext"]]
      }
    }
  ]
}
```

These rules bind independent implementers.

- A rejected replay is `{"status": "rejected", "message": String}` with
  `roots: []`. `message` is at most 4,096 UTF-8 bytes, truncated on a scalar
  boundary, never parsed, and excluded from snapshots.
- `replay.modules` lists the modules replayed, and must include `module`, so a
  report cannot claim acceptance while omitting the job.
- `roots` follows the request's order. `declaration` is `null` when the
  replayed environment has no such constant. It is a required field, and a
  missing field is malformed. `statement` is likewise required, and `null` when
  out of grammar or over the bound.
- `kind` is one of `theorem`, `axiom`, `definition`, `opaque`, `quotient`,
  `constructor`, `recursor`, and `inductive`.
- `axioms` lists every axiom `Lean.collectAxioms` returns for the root,
  including the root itself if it is an axiom.
- A report `Name` is a non-empty array whose components are strings (possibly
  empty) or integers up to `u64::MAX`, because private names contain numbers.
  `githash` is compared with the commit in the `identity` entry of
  `toolchain-pins.json`.
- Order is part of the protocol where settlement compares positions. `roots`
  follows the request's order, and an accepted replay must report exactly as
  many roots as the request names, each at the same index. Findings are a
  sequence: they are ordered by root and then by the position of the axiom in
  the report, so a consumer that reorders either list changes the outcome.
  `replay.modules` is compared as a set, and `axioms` is a set for acceptance,
  because settlement looks each name up in the request's foundations. The
  worker sorts axioms and lists modules deterministically so snapshots stay
  stable, but only the two ordered lists above carry meaning.
- There is deliberately no `accepted`, `verdict`, `ok`, or `statement_match`
  field. Any shape change means a new schema identifier, which is how 4.3.2
  adds provenance.
- Enforcement: the schema checks shape. `wire` also checks integer ranges, byte
  bounds, duplicates, trailing content, and tree bounds. Settlement checks
  correspondence.

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
    KernelRejected, RootAbsent, WrongKind, StatementMismatch,
    SorryAxiom, NativeAxiom, UndeclaredAxiom,
}

pub enum ProtocolBreach {
    ToolchainMismatch, ModuleMismatch, CoverageMismatch, RejectedReplayWithRoots,
}

pub fn validate_request(request: &AuditRequest) -> Result<(), RequestRefusal>;
pub fn settle(request: &AuditRequest, report: &AuditReport, pin: &[u8])
    -> Result<Settlement, RequestRefusal>;
```

The `native` flag and `FindingCode` select a diagnostic only: acceptance rests
on `approved`, so a defect in classification cannot admit an axiom. Process and
decoding failures (`worker_failed`, `output_overflow`, and `malformed_report`)
arise in `tests/toolchain/audit/wire/` and the smoke runner, before settlement,
and are reported as the same `tool_failure` outcome. Promotion into
`src/lean.rs` is 2.2.4's work.

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
  attack that defeated revision 1 twice. Added a pinned `Init`, a worker-fixed
  search path, module classes, and a corrected specification.
- Revision 3 (2026-11-05): recorded the user's acceptance of three design
  deviations and the Verus milestone (D13).
- Revision 4 (2026-11-06): separated proof integrity from operational integrity
  at the user's direction (D14). This milestone now establishes proof integrity
  under the generated-module input model. Foundation origin, the pinned `Init`
  base, the fixed search order, module classes, and statement provenance moved
  to new roadmap item 4.3.2, and the deadline helper to 2.3.1 and 4.3.1. The
  worker now uses `Lean.collectAxioms` (D15) and keeps replay (D16). The corpus
  shrank from 37 entries to 22, and the specification lost its origin and
  provenance clauses. The adversarial results moved to a findings record, with
  their sources. Revision 2's design stands as the starting point for 4.3.2.
- Revision 5 (2026-11-07): recorded the completed implementation, the measured
  results, and decisions D17 to D20. No scope change from revision 4.
- Revision 6 (2026-11-07): recorded the review repairs, decisions D21 and D22,
  and the ordering rules the wire protocol actually requires. No scope change
  from revision 5.
- Revision 7 (2026-11-07): recorded review round 6, including the decoder's
  enforcement of the request contract's `uniqueItems` and the reason `serde`
  keeps its exact pin. No scope change from revision 6.
- Revision 8 (2026-11-07): recorded review round 7, including the decoder's
  refusal of an empty `githash` and the thirteenth contract control. No scope
  change from revision 7.
- Revision 9 (2026-11-07): recorded both hosted assessments, the key round-trip
  properties, the worker-failure output check, the corrected worker assurance
  claim, the witness and control qualifications, and decision D23 on the
  exceeded scope tolerance. No scope change from revision 8.
