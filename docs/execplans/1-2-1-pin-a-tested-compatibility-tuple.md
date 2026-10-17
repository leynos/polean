# Pin and smoke-test the Rust, Regorus, Lean, and OPA toolchain tuple

This ExecPlan is a living document. Keep `Constraints`, `Tolerances`, `Risks`,
`Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan`
current as work proceeds.

Status: DRAFT (revision 1)

Roadmap item: [1.2.1](../roadmap.md). Branch:
`1-2-1-pin-a-tested-compatibility-tuple`.

## Purpose / big picture

Every later Polean task depends on four external tools behaving as the design
assumes. Regorus is the trusted frontend, Lean checks the model, OPA is the
independent source oracle, and Rust hosts the orchestrator. Today none of them
is pinned or exercised. The technical design (§11) and ADR-001 both leave the
tuple to experiment E0.

This work pins one tuple and shows that it works on Linux x86-64. A contributor
runs `make toolchain-smoke`. It fetches the Lean and OPA archives and checks
their SHA-256 digests against committed pins before extracting anything. It
then shows four things:

1. The pinned Lean toolchain builds a fixed, repository-authored smoke theorem
   with warnings treated as errors.
2. Regorus, built with the recorded features only, parses the baseline
   `examples/tenant-write/policy.rego` and returns its AST as JSON.
3. Regorus and OPA each evaluate `data.authz.allow` on the original baseline
   bytes. For four requests the result is exactly the Boolean that an
   independently written expectation table predicts.
4. Every pin agrees with its single authoritative file.

`make test` keeps running in hosted CI without Lean or OPA. It covers the
Regorus observations and the consistency of the pins.

None of this establishes admission, a model theorem, translation, or runtime
correspondence. A successful parse is not admission: 1.2.2 owns AST coverage
and 2.1.2 owns fail-closed admission. A compiled smoke theorem is not an
audited declaration; 1.2.3 owns type and axiom auditing. Agreement on four
requests is not conformance evidence; 2.3.2 owns typed oracle observations. The
documentation added here says so in those words.

## Constraints

- Keep the four claims separate (AGENTS.md, ADR-002). No text, test name,
  diagnostic, or Make target may describe parsing as admission, compilation as
  kernel-checked evidence, or two-runtime agreement as correspondence.
- Preserve the single Rust package, the reserved `lean/Polean/` module
  layout, and the independent Python design lane. Do not change `src/lib.rs`
  behaviour, `contracts/`, `examples/`, or `validation/`.
- The Lean package added under `lean/` is trusted repository content. Never
  build submitted Lean or Lake configuration. Generated proof jobs are later
  work (2.2.4).
- Pin by exact identity. Regorus uses an `=` version requirement with
  `default-features = false` and an explicit feature list. Lean uses an exact
  `lean-toolchain` string plus the release-archive SHA-256. OPA uses the static
  binary's SHA-256. Nothing is fetched without first checking its digest. No
  tool is found through `PATH` or elan.
- Keep `rust-toolchain.toml` at `nightly-2026-08-27` unless that is
  impossible. Changing it is an escalation (see Tolerances).
- Do not bypass compile admission (`RUSTC_WRAPPER`). Use the shared Cargo
  cache. Do not use `/tmp` as a build target.
- Child processes get explicit executable paths, argument arrays, and a
  cleared environment. Enforced isolation and offline execution are 1.2.4.
- British English with Oxford spelling. Keep `sha256sum -c SHA256SUMS`
  passing whenever a covered file changes.

## Tolerances (exception triggers)

- Scope: more than 30 changed files, or about 1,500 net lines excluding
  `Cargo.lock`, insta snapshots, and `lake-manifest.json`. Stop and escalate.
- Dependencies: authorized Rust additions are `regorus` (dev-dependency),
  `rstest`, `rstest-bdd`, `rstest-bdd-macros`, `googletest`,
  `pretty_assertions`, `insta`, and `serde_json`, all as dev-dependencies.
  Authorized Python additions are `cyclopts` (script) and `pytest` (tests).
  Anything else needs escalation.
- Toolchain: if no Regorus candidate builds on `nightly-2026-08-27`, or Lean
  v4.35.0 cannot build the smoke module, stop. Present the options rather than
  moving a pin.
- Semantics: if either runtime returns anything other than the expected
  Boolean for any of the four requests, stop. That is an E0 finding, not a test
  to adjust.
- Iterations: a gate still failing after three fix attempts on one cause.
- Ambiguity: a tool behaviour that would force a design-document change
  beyond recording pins, such as Regorus AST JSON lacking fields that 1.2.2
  will need. Record it in the Decision log and ask.

## Risks

- Risk: Regorus 0.13.0, released 2026-10-14, is new and marked breaking.
  Severity medium; likelihood low. Mitigation: build-test 0.12.0 (the candidate
  named in the design) and 0.13.0. The upstream compare shows that the
  `get_ast_as_json` body and the AST serde types are unchanged. The two
  breaking changes, a 32-component path limit and canonical rule paths, do not
  touch `package authz`. Pin the newest passing release so that 1.2.2 does not
  start on a superseded version.
- Risk: Regorus requires an exact prerelease of `vstd` and Verus proc-macros,
  which could fail to compile on a newer nightly. Severity medium; likelihood
  low. Mitigation: this is the first build step (EP-M1 red to green). The
  Toolchain tolerance applies.
- Risk: elan downloads toolchains without checking any hash (upstream
  `download_and_check` only downloads). Severity high for integrity; likelihood
  certain. Mitigation: the gate never uses elan. It fetches the Lean release
  archive directly and checks the pinned SHA-256 before extraction.
  Contributors may still use elan for editor support; the `lean-toolchain` file
  selects the same version.
- Risk: Lean kernels up to v4.31.0 accepted a forged proof (oss-security,
  2026-08-02). The full fix is only in v4.35.0. Severity high; likelihood low
  at v4.35.0. Mitigation: pin v4.35.0, released 2026-10-12, and record the
  reason in ADR-003.
- Risk: Lake, run with a cleared environment, may still need `HOME`, a
  writable cache, or `git`. Severity low; likelihood medium. Mitigation: start
  from an empty environment and add only the variables that a recorded failure
  demonstrates. Record each one in Surprises.
- Risk: provisioned tools use about 1.5 GiB per worktree under
  `.polean/tools/`. Severity low; likelihood certain. Mitigation: digest-named
  directories make re-runs offline and idempotent. Document the size.
- Risk: rstest-bdd filters scenarios by tag at macro expansion and may not
  carry `#[ignore]`. Severity low; likelihood medium. Mitigation: BDD scenarios
  cover the in-process Regorus workflow, which always runs. The
  provisioned-tool smoke checks are ordinary `#[ignore]`d rstest cases that
  `make toolchain-smoke` runs explicitly.
- Risk: the insta snapshot of the AST JSON could be accepted without anyone
  reading it. Severity medium; likelihood medium. Mitigation: the snapshot test
  also asserts named structural facts (see Verification plan), and snapshot
  review is a documented convention.

## Progress

- [x] (2026-10-17) Branch renamed. Reconnaissance by three Wyvern agents,
  plus Firecrawl research into the current Regorus, OPA, and Lean releases and
  into cedar-spec's toolchain practice.
- [x] (2026-10-17) Draft revision 1 written.
- [ ] Expert-panel design review; findings filtered and folded into the
  draft.
- [ ] Plan approved by the user.
- [ ] EP-M1: Regorus pinned; in-process parse and evaluation tests green.
- [ ] EP-M2: pin record and digest-checked provisioning script, with pytest
  tests.
- [ ] EP-M3: Lean smoke package and provisioned OPA/Lean smoke tests;
  `make toolchain-smoke` green.
- [ ] EP-M4: ADR-003, design and guide updates, `SHA256SUMS` refresh, and
  roadmap 1.2.1 marked done.

## Surprises & discoveries

- Observation: Regorus 0.13.0 was released on 2026-10-14. Its feature table
  matches 0.12.0's: default `full-opa`, `arc`, `rvm`, with `ast` opt-in. Its
  licence is `MIT AND Apache-2.0 AND BSD-3-Clause`. Evidence: the crates.io API
  and the `regorus-v0.13.0` `Cargo.toml`. Impact: both releases are candidates,
  and the licence feeds the 5.2.2 distribution decision.
- Observation: Lean v4.35.0 removed `Lean.ofReduceBool` and
  `Lean.trustCompiler`. Each `native_decide` now introduces its own
  `*._native.native_decide.ax_*` axiom. The release also adds `lake check` and
  the `leanchecker` integration. Evidence: v4.35.0 release notes (PR #14953).
  Impact: 1.2.3's rejection list must match the new names. Note it in ADR-003
  for 1.2.3; do not act on it here.
- Observation: OPA v1.21.2 (2026-10-12) is an immutable GitHub release.
  `gh release verify` succeeds against its attestation. Each binary ships with a
  `sha256sum`-format `.sha256` file. Lean releases have neither a checksum
  file nor an attestation; only the GitHub API asset digest exists. Impact:
  record each artefact's digest provenance separately in ADR-003.

## Decision log

- Decision: test both Regorus 0.12.0 and 0.13.0, and pin the newest release
  that passes EP-M1. Rationale: the roadmap names 0.12.0 as a candidate, not a
  choice. A one-step trial is cheap, and starting 1.2.2 on a superseded release
  would waste its AST fixtures. (Agent, 2026-10-17; subject to plan approval.)
- Decision: enable only the Regorus features `std` and `ast`. Rationale: the
  profile admits no built-in calls (TDD §4.1). `ast` supplies `get_ast_as_json`
  (TDD §6.1). Disabling the `full-opa` built-in groups, `arc`, and `rvm`
  shrinks the trusted frontend. If the build shows that another feature is
  needed, record why. (Agent, 2026-10-17.)
- Decision: pin Lean v4.35.0 and OPA v1.21.2 `opa_linux_amd64_static`.
  Rationale: Lean v4.35.0 is the first release with both kernel-soundness
  fixes, and OPA v1.21.2 is the latest stable immutable release; see Risks.
  (Agent, 2026-10-17.)
- Decision: provision through a stdlib-plus-Cyclopts Python script and do not
  use elan, curl, or shell pipelines. Rationale: the scripting standards
  require Python for non-trivial scripts. Python 3.14's `tarfile` reads zstd
  archives, confirmed locally with `compression.zstd`, and applies the `data`
  extraction filter by default. Elan checks no digest. The script launches no
  subprocess, so `cuprum` is not needed. (Agent, 2026-10-17.)
- Decision: Regorus is a dev-dependency in this task. Rationale: no
  production module uses it until 2.1.1/2.1.2. Moving the same `=` pin to
  `[dependencies]` then is a one-line change, and `Cargo.lock` records it
  either way. (Agent, 2026-10-17.)
- Decision: the docs/users-guide.md text stays unchanged. Rationale: there is
  no change in user-visible behaviour or public API, since Polean still
  provides no executable checker. (Agent, 2026-10-17.)
- Decision: Verus, Kani, and proptest are not used in this task; see
  Verification plan. (Agent, 2026-10-17.)

## Outcomes & retrospective

Not started.

## Context and orientation

Polean is a design scaffold. `src/lib.rs` exports a status string only.
`tests/stub.rs` checks it. `Cargo.toml` has no dependencies and a strict lint
set: no `unwrap`, `expect` outside tests, or `print*`; mandatory docs and
assertion messages; and `clippy.toml` bans `std::env::var`. `lean/` contains
only a README that reserves the directory. Python under `validation/` checks
design fixtures and is a separate lane. `SHA256SUMS` is a manually maintained
manifest covering `README.md`, `contracts/`, `docs/context.md`,
`docs/technical-design.md`, `docs/terms-of-reference.md`, `examples/`, and
`validation/`. Hosted CI (`.github/workflows/ci.yml`) runs `make check-fmt`,
markdownlint, `make typecheck`, `make lint`, and `make test` with no Lean or
OPA.

Terms used here:

- **Toolchain tuple**: the exact Rust toolchain and `Cargo.lock`, Regorus
  version and features, Lean toolchain and package manifest, and OPA binary
  digest that a run used. TDD §7 later binds a hash of this tuple into evidence
  (4.1.1).
- **Provisioning**: downloading a pinned archive, checking its SHA-256 before
  use, and extracting it into a digest-named directory under `.polean/tools/`.
  That path is already ignored by Git.
- **Smoke check**: an observation that a tool works for a fixed input. It is
  not admission, a theorem about Polean, or conformance evidence.

The baseline policy is `examples/tenant-write/policy.rego`: package `authz`,
`import rego.v1`, `default allow := false`, and two `allow` rules. The intended
decision is `T ∧ W ∧ (A ∨ (O ∧ ¬L))` (TDD §3). The smoke checks use that
formula, not either runtime, to state expected results.

Skills to load: `execplans`, `rust-router`, then `rust-unit-testing` (rstest,
googletest, insta), `arch-supply-chain` (exact pins and audit),
`arch-decision-records` (ADR-003), `en-gb-oxendict-style`, and `nextest` only
if a test filter misbehaves. Use `codegraph-mcp` for navigation and
`firecrawl-mcp` for any further upstream lookups. Read
`docs/rust-testing-with-rstest-fixtures.md`,
`docs/reliable-testing-in-rust-via-dependency-injection.md`,
`docs/rust-doctest-dry-guide.md`,
`docs/complexity-antipatterns-and-refactoring-strategies.md`,
`docs/scripting-standards.md`, and `docs/documentation-style-guide.md` before
the corresponding milestone. Delegate full gate runs to `scrutineer`.

## Conformance basis

Upstream at commit `24f3874`: ToR draft v0.1 (A4, A5, G5, G6, SC5, OQ5);
technical design draft v0.1 (§§3, 4.1, 6, 6.1, 6.3, 7, 11, 13 "Toolchain tuple"
row); ADR-001 (deferred Lean pin; "revisit the Rust pin alongside the tuple
during E0"); ADR-002 (assurance boundary); repository-layout.md §Ownership
boundaries; roadmap RM-1.2.1.

```plaintext
TDD §11 pins, ADR-001 E0 deferral -> RM-1.2.1 -> EP-M1..M3 -> make toolchain-smoke; ADR-003
TDD §6.1 Regorus ast feature      -> RM-1.2.1 -> EP-M1 -> tests/regorus_tuple.rs AST snapshot
TDD §6.3 v1, strict errors, entry -> RM-1.2.1 -> EP-M1/M3 -> runtime option assertions
ToR A4 offline after provisioning -> RM-1.2.1 -> EP-M2 -> second provisioning run makes no request
TDD §13 toolchain row (E0 need)   -> RM-1.2.1 -> EP-M4 -> §13 row and ADR-003 cite the tuple
```

Not discharged here: A5 (1.2.2), the axiom policy (1.2.3), enforced isolation
(1.2.4), and SC5 clean-machine replay (4.3.2). This task supplies the pins
those tasks use.

## Verification plan

This task introduces no contractual business logic and no policy semantics. Its
code is provisioning and smoke-test plumbing. Verus is not warranted: there is
no lemma or pure kernel whose correctness an unbounded proof would establish.
The Polean theorems (V1–V5) are Lean obligations for phase 2. Kani and proptest
are not warranted either. The classification helper below is a finite partition
over JSON shapes, and a named rstest table covers every class. A generator
would only restate the implementation. The obligations and their checks follow.

- Obligation O1, digest gate: the provisioning script installs an artefact
  only if the SHA-256 of the downloaded bytes equals the pinned digest. On a
  mismatch it leaves no file at the final path and exits non-zero. Method:
  pytest unit tests using a fetcher injected over a local temporary directory.
  Artefact: `scripts/tests/test_provision_tools.py`. Non-vacuity: one case
  flips a single byte and must be rejected; another case passes correct bytes
  and must install, so the predicate is reachable both ways. Evidence:
  `make test-scripts`.
- Obligation O2, extraction containment: archive members cannot be written
  outside the digest-named directory, and an interrupted extraction is never
  visible as complete. Method: pytest with a crafted archive containing a
  `../escape` member and an absolute-path member, plus a case interrupted
  before the atomic rename. The `tarfile` `data` filter is an axiom (stdlib
  behaviour). The test checks that the script actually applies it. Negative
  control: the traversal archive must raise and leave nothing behind.
- Obligation O3, single-source pins: `lean/lean-toolchain` names the same
  version as `toolchain-pins.json`. The pinned archive URL embeds that version.
  The Regorus requirement in `Cargo.toml` is an exact `=` pin with
  `default-features = false` and features exactly `["ast", "std"]`.
  `Cargo.lock` contains exactly one `regorus` package at that version. Method:
  rstest cases in `tests/toolchain_pins.rs`, run by `make test`, that parse the
  files with `serde_json` and line-oriented reads (no TOML crate). Non-vacuity:
  each check runs a second time on an in-test mutated copy, such as a changed
  version string, which must be rejected.
- Obligation O4, strict decision observation: a smoke check counts a runtime
  result as a decision only when it is exactly one Boolean. `false` is kept as
  `false`. An empty or undefined result, multiple results, the string `"false"`,
  `null`, a non-Boolean, a malformed document, or a non-zero OPA exit each
  produce a distinct error. Method: a named rstest table over
  `tests/common/decision.rs::observe_opa_json` and `observe_regorus_value`.
  Non-vacuity: every error class has a row, and the table includes both `true`
  and `false` successes.
- Obligation O5, independent expectations: the expected decision for each
  smoke request is written by hand from `T ∧ W ∧ (A ∨ (O ∧ ¬L))` and annotated
  with its fact vector. Requests: owner-unlocked write (true), cross-tenant
  administrator write (false), same-tenant administrator write on a locked
  resource (true), and owner read (false). Method: the same table drives Regorus
  (`make test`) and OPA (`make toolchain-smoke`). Neither runtime's output
  feeds the expectations. Non-vacuity: the set contains both outcomes, and the
  cross-tenant case is exactly the regression that E0 targets.
- Obligation O6, non-vacuous Lean smoke: building the smoke library fails if
  the module contains `sorry` or any other warning. Lake runs with
  `warningAsError = true` and the build must exit 0. Method: the provisioned
  smoke test runs the pinned `lake build`. Negative control, run once and
  recorded in Artefacts: temporarily append `theorem bad : False := by sorry`,
  observe the build fail, then revert. This is not an axiom or declaration
  audit, which 1.2.3 owns.
- Obligation O7, adapter-input record: the Regorus AST JSON for the exact
  baseline bytes is snapshotted with insta. The test also asserts that the JSON
  has exactly one module, package path `authz`, one `rego.v1` import, and three
  `allow` rules (one default and two bodies, the latter with three and four
  expressions). Method: insta plus googletest matchers. Non-vacuity: the
  structural assertions fail on the deny-all mutation's source, which is
  checked in the same test.

External axioms, not verified here: SHA-256 collision resistance; the
correctness of Regorus parsing and evaluation; OPA evaluation; the Lean kernel
and Lake; Python's `tarfile` filter; GitHub serving the published assets.
Residual gap: digests are taken from GitHub release metadata, and for OPA the
release attestation is also checked once at pin time. Nothing here proves that
upstream built the binaries from the tagged source.

## Plan of work

Stage A, confirm without code (already done in research; re-run at the start of
EP-M1 and EP-M2): re-read the crates.io versions of the two Regorus candidates.
Re-read the OPA `.sha256` asset and the GitHub asset digests for
`opa_linux_amd64_static` (v1.21.2) and `lean-4.35.0-linux.tar.zst`. Run
`gh release verify v1.21.2 -R open-policy-agent/opa` once and record the
output. Values observed on 2026-10-17: OPA
`830ab11f61ffec760b5e19db0d7ad02bf037e13f5ef4d665faeaf17632177527`; Lean
`7289597a44543246c9227b8cae501767e095ce17f6d448b95fc4a8dc97268f6d`. If either
differs on recheck, stop; that is a supply-chain surprise.

Stage B, EP-M1, Regorus in process:

1. Red. Add dev-dependencies `rstest`, `googletest`, `pretty_assertions`,
   `insta` (feature `json`), `serde_json`, `rstest-bdd`, and
   `rstest-bdd-macros`. Each gets a `=` pin at the current release (observed
   2026-10-17: 0.27.0, 0.14.3, 1.4.1, 1.49.0, 1.0.154, 0.6.0). Write
   `tests/common/mod.rs` (fixtures: baseline bytes via `include_str!`, the O5
   request table, pin-file paths from `env!("CARGO_MANIFEST_DIR")`) and
   `tests/common/decision.rs` (O4 helpers). Write `tests/regorus_tuple.rs` with
   the O4 table, the O5 Regorus cases, and the O7 snapshot test. Write
   `tests/features/toolchain_tuple.feature` with its bindings in
   `tests/toolchain_tuple_bdd.rs`. Run `cargo test --test regorus_tuple`.
   Expect compile failure: crate `regorus` unresolved.
2. Green. Add this line under `[dev-dependencies]`:

   ```toml
   regorus = { version = "=0.12.0", default-features = false, features = ["ast", "std"] }
   ```

   Build and run the tests. Accept the reviewed snapshot with
   `cargo insta review`. Repeat with `=0.13.0` and compare the snapshots:
   identical output is expected; any diff is a Surprise. Keep the newest
   release that passes. Record `cargo tree -e features -i regorus` in Artefacts.
3. Write `tests/toolchain_pins.rs` covering the Regorus half of O3.
4. Refactor; then run `make check-fmt`, `make typecheck`, `make lint`, and
   `make test`. Run `make audit` once and record the result, since it needs the
   network. Commit.

The Regorus engine is configured explicitly even where the value matches the
default: `set_rego_v0(false)` and `set_strict_builtin_errors(true)`, matching
`contracts/profile.json` `runtime_options`. The policy is added with
`add_policy("policy.rego", baseline)`. Inputs come from `set_input_json`.
`eval_rule("data.authz.allow")` returns the observation that
`observe_regorus_value` classifies.

Stage C, EP-M2, pins and provisioning:

1. Red. Write `scripts/tests/test_provision_tools.py` (O1, O2, pin-file
   parsing, and idempotent offline re-run: a second call with a fetcher that
   raises must succeed without fetching). Run
   `uv run --with pytest==<current> pytest scripts/tests`. Expect an import
   failure.
2. Green. Write `scripts/provision_tools.py`, a uv script with Python 3.14,
   PEP 723 metadata, and Cyclopts. It reads `toolchain-pins.json`, which
   strictly rejects unknown keys and has entries `opa` and `lean`, each with
   `version`, `url`, `sha256`, and `kind` (`binary` or `tar.zst`). It downloads
   to `.partial` files under `.polean/tools/downloads/` and checks SHA-256. It
   installs to `.polean/tools/<name>/<sha256>/` through a temporary sibling
   directory and an atomic rename, marks the OPA binary executable, and prints
   the installed paths. Network access happens only when the verified archive
   is absent.
3. Add Make targets `tools` (runs the script) and `test-scripts` (pytest),
   both in `.PHONY` with `##` help. Add `test-scripts` to `all`. Add the Lean
   half of O3 to `tests/toolchain_pins.rs` once `lean/lean-toolchain` exists;
   this is done in EP-M3 if ordering requires.
4. Gates as before plus `make test-scripts`; commit.

Stage D, EP-M3, Lean package and provisioned smoke tests:

1. Red. Write `tests/toolchain_smoke.rs` with
   `#[ignore = "needs make tools; run make toolchain-smoke"]` tests:
   - `opa_reports_pinned_version`, which checks `opa version` for
     `Version: 1.21.2` and `Rego Version: v1`.
   - `opa_evaluates_baseline_source`, which runs the O5 table through
     `opa eval --format json --strict-builtin-errors -d <baseline path> -i
     <request file> data.authz.allow`, where each request is written into a
     per-test `target/`-relative scratch directory.
   - `lean_builds_smoke_theorem`, which runs the pinned `lake build` in
     `lean/`.

   Each test locates its tool through `toolchain-pins.json` and the
   digest-named path, and fails, rather than skipping, when the tool is absent.
   Add `make toolchain-smoke`: `tools`, then
   `cargo test --all-features --test toolchain_smoke -- --ignored`. Run it
   before the Lean package exists. Expect the Lean test to fail for the missing
   package and the OPA tests to pass.
2. Green. Add `lean/lean-toolchain` (`leanprover/lean4:v4.35.0`) and
   `lean/lakefile.toml`. The lakefile declares package `polean`, one `lean_lib`
   named `PoleanSmoke`, no dependencies, and
   `leanOptions.warningAsError = true`. Add `lean/PoleanSmoke.lean` and generate
   `lean/lake-manifest.json` with the pinned `lake`, never by hand. Add
   `.lake/` to `.gitignore`. The proposed module text follows; its exact
   wording is settled by the first successful build:

   ```lean
   /-!
   # Toolchain smoke module

   A fixed, repository-authored check that the pinned Lean toolchain
   elaborates and kernel-reduces a small finite Boolean enumeration with
   `decide`. It states nothing about Polean policies.
   -/
   namespace PoleanSmoke

   /-- Every pair of Booleans, listed explicitly. -/
   def boolPairs : List (Bool × Bool) :=
     [(false, false), (false, true), (true, false), (true, true)]

   /-- The enumeration covers every pair. -/
   theorem boolPairs_complete : ∀ a b : Bool, (a, b) ∈ boolPairs := by
     decide

   /-- A finite check over the enumeration, settled by kernel reduction. -/
   theorem and_comm_on_boolPairs :
       boolPairs.all (fun p => (p.1 && p.2) == (p.2 && p.1)) = true := by
     decide

   end PoleanSmoke
   ```

   The shape mirrors TDD §5 in miniature, coverage plus a decided finite check,
   so that a toolchain unable to reduce such a check is caught now. Run the O6
   negative control.
3. Complete O3's Lean half. Then run the gates, `make test-scripts`, and
   `make toolchain-smoke`, and commit.

Child processes use `std::process::Command` with `env_clear()`. Only variables
demonstrated to be needed are added back, each documented beside the call.
Every invocation uses an argument array and the explicit tool path.

Stage E, EP-M4, documentation and closure:

- Write `docs/adr-003-pin-the-e0-toolchain-tuple.md`. It records the tuple,
  feature choice, digest provenance, the reason for Lean v4.35.0, retention of
  the Rust nightly (resolving ADR-001's "revisit" consequence), the removal of
  elan from the gate, and licences as inputs to 5.2.2. List it in
  `docs/contents.md`.
- In `docs/technical-design.md`:
  - §6.1 records the tested Regorus version and features.
  - §11 replaces "Regorus 0.12.0 is an inspected frontend candidate" with the
    tested pin and links ADR-003.
  - The §13 toolchain row shows that the E0 need is met by ADR-003 and
    `make toolchain-smoke`.
  - Add references for the new upstream sources.
- In `docs/context.md`, add **Toolchain tuple**.
- In `lean/README.md` and `docs/repository-layout.md`, describe the trusted
  smoke package, `toolchain-pins.json`, `scripts/`, the new tests, and
  `.polean/tools/`. Distinguish repository-owned Lake configuration from
  forbidden submitted configuration.
- In `docs/developers-guide.md`, cover:
  - prerequisites (uv, Python 3.14, network for the first provisioning, and
    about 1.5 GiB of disk);
  - the new targets;
  - how to change a pin (edit the authoritative file, run `make tools`,
    `make toolchain-smoke`, and `cargo insta review`, and update ADR-003);
  - the test-stack conventions (rstest, googletest, insta snapshots
    committed and reviewed, `#[ignore]`d provisioned tests).
- In `AGENTS.md`, add `make test-scripts` and `make toolchain-smoke` to the
  gate list.
- In `README.md`, add one status sentence: the tuple is pinned and
  smoke-tested, while admission, proofs, and conformance remain unimplemented.
- Leave `validation/README.md` unchanged. Its "Checks not performed" section
  records the drafting run and is still true of the Python lane.
- Refresh `SHA256SUMS` for the covered files edited. Mark roadmap 1.2.1
  `[x]`. Run every gate and set Status to `COMPLETE`.

## Milestones and plateaus

- EP-M1, Regorus in process. The repository builds with a pinned Regorus.
  `make test` shows the baseline parsed, its AST recorded, and four expected
  Boolean decisions. Requirements: TDD §6.1 and §6.3 (Regorus side). Evidence:
  `tests/regorus_tuple.rs`, `tests/toolchain_tuple_bdd.rs`, and the Regorus
  half of `tests/toolchain_pins.rs`.
  - Conformance: no `src/` change; the only dependencies are dev-dependencies
    from the authorized list.
  - Recovery: revert the commit.
  - Remaining: OPA, Lean, provisioning.
  - Compatibility decision: none.
- EP-M2, provisioning. `make tools` installs digest-checked OPA and Lean
  artefacts, and a second run is offline. Requirements: TDD §11 pins, ToR A4
  (after provisioning). Evidence: `make test-scripts`.
  - Conformance: `.polean/tools/` remains untracked, and no trust boundary
    changes.
  - Recovery: `rm -rf .polean/tools` is always safe.
  - Compatibility decision: none.
- EP-M3, provisioned smoke. `make toolchain-smoke` passes: the OPA version,
  the four OPA decisions, and the Lean smoke build. Requirements: the RM-1.2.1
  success criteria. Evidence: the `tests/toolchain_smoke.rs` run log, plus the
  O6 negative-control transcript.
  - Conformance: Lake builds only repository content, and `lean/Polean/`
    stays reserved.
  - Recovery: revert; delete `lean/.lake/`.
  - Compatibility decision: none.
- EP-M4, records. ADR-003 and the documentation agree with the code;
  `SHA256SUMS` passes; 1.2.1 is ticked. Requirements: the TDD §13 E0 toolchain
  need. Evidence: all gates, plus a reading check that no edited sentence
  widens one of the four claims.
  - Compatibility decision: none.

## Concrete steps

From the worktree root, sequentially, each step teed to
`/tmp/<action>-polean-1-2-1-pin-a-tested-compatibility-tuple.out`:

```bash
make check-fmt
make markdownlint
make typecheck
make lint
make test                      # Regorus and pin tests run; smoke tests ignored
make test-scripts              # from EP-M2
make tools                     # from EP-M2; network only on first run
make toolchain-smoke           # from EP-M3; expect 3 passed, 0 ignored
make design-check PYTHON=<venv with validation/requirements.txt>  # expect no diff
sha256sum -c --quiet SHA256SUMS   # expect no output
```

Expected tail of `make toolchain-smoke`:

```plaintext
running 3 tests
test lean_builds_smoke_theorem ... ok
test opa_evaluates_baseline_source ... ok
test opa_reports_pinned_version ... ok
test result: ok. 3 passed; 0 failed; 0 ignored
```

## Validation and acceptance

The task is accepted when:

- A fresh clone can run `make tools && make toolchain-smoke`, and every test
  passes.
- Re-running `make tools` with the network disabled succeeds.
- `make test` passes in hosted CI without Lean or OPA.
- `Cargo.lock`, `lean/lake-manifest.json`, the Regorus feature list, and the
  OPA and Lean digests are committed and named in ADR-003.
- The O6 negative control is recorded.
- No document says the tuple proves, admits, or conforms.

Red-Green-Refactor evidence for each milestone is recorded in Artefacts:

- the unresolved-crate compile failure (EP-M1);
- the pytest import failure (EP-M2);
- the missing-package Lean failure (EP-M3).

## Idempotence and recovery

`make tools` is idempotent. It re-hashes downloads, skips existing digest-named
installs, and never overwrites a completed install. Deleting `.polean/tools/` or
`lean/.lake/` is always safe. Each milestone is a single revertable commit, or
a small series of them.

## Artefacts and notes

To be filled during implementation: the `cargo tree -e features -i regorus`
output, the `make audit` result, the `gh release verify` output, the Red-stage
transcripts, the O6 negative-control transcript, and any variables Lake needed.

## Interfaces and dependencies

- No public Rust API changes. All new Rust code lives under `tests/`.
- New committed files:
  - `toolchain-pins.json`
  - `scripts/provision_tools.py`
  - `scripts/tests/test_provision_tools.py`
  - `lean/lean-toolchain`
  - `lean/lakefile.toml`
  - `lean/lake-manifest.json`
  - `lean/PoleanSmoke.lean`
  - `tests/common/{mod,decision}.rs`
  - `tests/{regorus_tuple,toolchain_pins,toolchain_smoke,toolchain_tuple_bdd}.rs`
  - `tests/features/toolchain_tuple.feature`
  - `tests/snapshots/*.snap`
  - `docs/adr-003-pin-the-e0-toolchain-tuple.md`
- The test helpers in `tests/common/decision.rs`:

```rust
/// Why a runtime result is not exactly one Boolean decision.
#[derive(Debug, PartialEq, Eq)]
pub enum DecisionError {
    Undefined,
    Multiple(usize),
    NotBoolean(String),
    Malformed(String),
    ToolFailed { status: Option<i32> },
    Evaluation(String),
}

pub fn observe_opa_json(stdout: &str, exit: Option<i32>) -> Result<bool, DecisionError>;
pub fn observe_regorus_value<E: core::fmt::Display>(result: Result<regorus::Value, E>) -> Result<bool, DecisionError>;
```

The feature file, `tests/features/toolchain_tuple.feature`:

```gherkin
Feature: Pinned Regorus observes the baseline source

  Scenario: The baseline parses into one authz module
    Given the baseline tenant-write source
    When Regorus parses it with the pinned features
    Then the AST names package authz with three allow rules

  Scenario: An owner may write an unlocked resource in their tenant
    Given the baseline tenant-write source
    And the owner-unlocked write request
    When Regorus evaluates data.authz.allow
    Then the decision is exactly true

  Scenario: A cross-tenant administrator write is denied
    Given the baseline tenant-write source
    And the cross-tenant administrator write request
    When Regorus evaluates data.authz.allow
    Then the decision is exactly false
```

## Revision notes

- Revision 1 (2026-10-17): initial draft from reconnaissance and upstream
  research.
