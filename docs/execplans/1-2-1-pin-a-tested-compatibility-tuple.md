# Pin and smoke-test the Rust, Regorus, Lean, and OPA toolchain tuple

This ExecPlan is a living document. Keep `Constraints`, `Tolerances`, `Risks`,
`Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan`
current as work proceeds.

Status: COMPLETE (revision 2)

Roadmap item: [1.2.1](../roadmap.md). Branch:
`1-2-1-pin-a-tested-compatibility-tuple`.

## Purpose / big picture

Every later Polean task depends on four external tools behaving as the design
assumes. Regorus is the trusted frontend, Lean checks the model, OPA is the
independent source oracle, and Rust hosts the orchestrator. Before this task,
none of them was pinned or exercised. The technical design (§11) and ADR-001
both left the tuple to experiment E0.

This work pins one tuple and shows that it works on Linux x86-64. A contributor
runs `make toolchain-smoke`. It fetches the Lean release archive and the OPA
static binary, rejects any byte stream whose SHA-256 differs from the committed
pin, and then shows four things:

1. The pinned Lean toolchain builds a fixed, repository-authored smoke library
   with warnings as errors. It reports the expected version.
2. Regorus, built with the recorded features only, parses the baseline
   `examples/tenant-write/policy.rego` and returns its AST as JSON. A snapshot
   records that output.
3. Regorus and OPA each evaluate `data.authz.allow` on the original baseline
   bytes. For four requests the result is exactly the Boolean that an
   independently written expectation predicts.
4. A built-in from a disabled Regorus feature group is unavailable, which shows
   that the recorded feature set is the one actually compiled.

`make test` covers the Regorus observations everywhere, including hosted CI. A
path-filtered CI workflow re-runs the provisioned smoke checks whenever a pin
or the smoke code changes.

None of this establishes admission, a model theorem, translation, or runtime
correspondence:

- A successful parse is not admission. 1.2.2 owns AST coverage, and 2.1.2 owns
  fail-closed admission.
- A compiled smoke theorem is not an audited declaration. 1.2.3 owns the type
  and axiom audit.
- Agreement on four requests is not conformance evidence. 2.3.2 owns typed
  oracle observations.

The documentation added here states each of these limits explicitly.

## Constraints

- Keep the four claims separate (AGENTS.md, ADR-002). No text, test name,
  diagnostic, or Make target may describe parsing as admission, compilation as
  kernel-checked evidence, or two-runtime agreement as correspondence.
- Preserve the single Rust package, the reserved `lean/Polean/` module layout,
  and the independent Python design lane. Do not change `src/lib.rs`,
  `contracts/`, `examples/`, or `validation/`.
- The Lean package under `lean/` is trusted repository content. Never build
  submitted Lean or Lake configuration. Generated proof jobs are 2.2.4.
- Pin by exact identity:
  - Regorus: an `=` requirement with `default-features = false` and an
    explicit feature list.
  - Lean: an exact `lean-toolchain` string, plus the archive SHA-256 and size.
  - OPA: the static binary's SHA-256 and size.

  Nothing fetched is used before its digest matches. No tool is resolved through
  `PATH` or elan.
- Keep `rust-toolchain.toml` at `nightly-2026-08-27` unless that is impossible;
  changing it is an escalation.
- Do not bypass compile admission (`RUSTC_WRAPPER`). Use the shared Cargo
  cache. Never use `/tmp` as a build target.
- Child processes get explicit executable paths, argument arrays, and an empty
  environment (`env_clear()`). Enforced isolation and offline execution are
  1.2.4.
- Use British English with Oxford spelling. Keep `sha256sum -c SHA256SUMS`
  passing whenever a covered file changes.

## Tolerances (exception triggers)

- Scope: more than 30 changed files, or about 1,200 net lines excluding
  `Cargo.lock`, the insta snapshot, and `lake-manifest.json`.
- Dependencies:
  - Authorized Rust dev-dependencies: `regorus`, `rstest`, `rstest-bdd`,
    `rstest-bdd-macros`, `googletest`, `pretty_assertions`, `insta`, and
    `serde_json`.
  - Authorized Python test dependency: `pytest`.
  - The provisioning script uses the standard library only.
  - Any other dependency requires escalation.
- Toolchain: stop and present options, rather than moving a pin, in any of
  these cases:
  - Neither Regorus candidate builds on `nightly-2026-08-27`.
  - Lean v4.35.0 cannot build the smoke library.
  - A pinned digest differs from upstream on recheck.
- Semantics: if either runtime returns anything other than the expected Boolean
  for any of the four requests, stop. That is an E0 finding, not a test to
  adjust.
- Iterations: stop when a gate still fails after three fix attempts on one
  cause.
- Ambiguity: stop on tool behaviour that would force a design change beyond
  recording pins, such as AST JSON lacking information that 1.2.2 needs. Record
  it in the Decision log and ask.

## Risks

- Risk: Regorus 0.13.0, released 2026-10-14, is new and marked breaking. It
  also adds `cc`, `find-msvc-tools`, `shlex`, `ohno`, and `typeid`, and a
  `msvc_spectre_libs` build script that compiles even on Linux.
  - Severity: medium. Likelihood: low.
  - A scratch build on `nightly-2026-08-27` showed byte-identical AST JSON and
    decisions for 0.12.0 and 0.13.0.
  - The two breaking changes (a 32-component path limit and canonical rule
    paths) do not touch `package authz`.
  - Mitigation: test both in EP-M1 and record the dependency delta in ADR-003.
    The default choice is in the Decision log.
- Risk: Regorus requires exact prereleases of `vstd` and the Verus proc-macros.
  - Severity: medium. Likelihood: low.
  - Mitigation: both versions built in the scratch review. EP-M1 re-proves it
    in the repository; the Toolchain tolerance applies.
- Risk: elan downloads toolchains without hashing them
  (`src/elan-dist/src/download.rs:30-37`, `download_and_check`).
  - Severity: high for integrity. Likelihood: certain.
  - Mitigation: the gate never uses elan. Contributors may keep elan for editor
    support, and `lean/lean-toolchain` selects the same version.
- Risk: Lean kernel soundness defects were found during the 2026 kernel bug
  hunt.
  - Severity: high. Likelihood: low at the pin.
  - The forged-proof fix (PR #14577) and the bug-hunt fixes are in v4.33.1 and
    later. The use-after-free fixes were backported to v4.34.1.
  - Mitigation: pin v4.35.0, the newest stable release containing every known
    fix. Record v4.34.1 as the floor in ADR-003.
- Risk: the Lean asset sits on a mutable, unattested GitHub release, so its
  digest is trust-on-first-use.
  - Severity: medium. Likelihood: low.
  - Mitigation: an upstream re-upload fails closed against the pin. ADR-003
    records the provenance difference from OPA, whose immutable release is
    attested.
- Risk: the provisioned tools need about 3.7 GiB per worktree: a 3.0 GiB
  extracted Lean tree, the 594 MB archive, and the 64 MB OPA binary.
  - Severity: low. Likelihood: certain.
  - Mitigation: document the size. Keep the archive so that re-extraction and
    the CI cache work offline. `rm -rf .polean/tools` is always safe.
- Risk: feature unification silently re-enables Regorus built-in groups when
  2.1.1 moves it into `[dependencies]`.
  - Severity: medium. Likelihood: medium. `Cargo.lock` does not record
    features.
  - Mitigation: a behavioural negative test (O3) fails if a disabled built-in
    group becomes callable.
- Risk: pins rot because nothing re-tests them.
  - Severity: high. Likelihood: high without CI.
  - Mitigation: a path-filtered `toolchain` workflow runs `make test-scripts`
    and `make toolchain-smoke` on changes to pins, `Cargo.*`, `lean/`, the
    script, or the smoke tests. Dependabot bumps are therefore re-tested.
- Risk: the insta snapshot is accepted without being read.
  - Severity: medium. Likelihood: medium.
  - Mitigation: the snapshot test also asserts named structural facts (O5), and
    developers-guide documents snapshot review.

## Progress

- [x] (2026-10-17) Branch renamed. Three Wyvern agents ran repository
  reconnaissance. Firecrawl research covered the current Regorus, OPA, and Lean
  releases, plus cedar-spec's toolchain practice.
- [x] (2026-10-17) Revision 1 drafted and committed.
- [x] (2026-10-17) A four-member expert panel reviewed the draft:
  supply chain, Lean, Rust and Regorus, and assurance boundary with
  alternatives. Two reviewers ran scratch builds and probes. The findings were
  filtered for proportionality and folded into revision 2.
- [x] Plan approved by the user.
- [x] (2026-10-21) EP-M1 Red. `tests/toolchain/{main,support,regorus,bdd}.rs`
  and `tests/features/regorus_baseline.feature` written;
  `cargo test --test toolchain` failed with
  `error[E0432]: unresolved import 'regorus'` in both `bdd.rs` and
  `regorus.rs`, as the plan requires.
- [x] (2026-10-21) EP-M1 Green. Regorus 0.12.0 pinned, tests green, snapshot
  reviewed and accepted, then 0.13.0 pinned; the accepted snapshot is
  unchanged, and all eight tests pass on 0.13.0.
- [x] (2026-10-21) EP-M1 gates, first run: `make typecheck`, `make test`,
  `make markdownlint`, and `make audit` passed; `make check-fmt` failed on
  ExecPlan table reflow and `make lint` failed on seven Clippy diagnostics in
  `tests/toolchain/regorus.rs`.
- [x] (2026-10-21) EP-M1 repair: `make fmt` reflowed the tables.
  `regorus.rs` gained a `Result`-returning `expect_equal` helper (the
  `panic_in_result_fn` requirement), the structural checks were split into
  small functions (`cognitive_complexity`, `too_many_lines`), and shadowed
  bindings were renamed.
- [x] (2026-10-21) EP-M1 milestone gates, second run: `make check-fmt`,
  `make typecheck`, `make lint`, and `make test` all passed sequentially under
  one scrutineer run (2 stub + 8 toolchain + 1 doctest).
- [x] (2026-10-21) EP-M1 milestone CLI review: `coderabbit review --agent`
  completed over the branch against `main`, reviewing `Cargo.toml`,
  `Cargo.lock`, and the ExecPlan. One low-severity finding: `serialisation`
  should be `serialization` under en-GB-oxendict. Fixed in the same file; the
  review reported no other concern.
- [x] (2026-10-21) EP-M2 Red. `scripts/tests/test_provision_tools.py` written
  against a missing module; `make test-scripts` failed with
  `ModuleNotFoundError: No module named 'provision_tools'` (1 collection
  error), as the plan requires.
- [x] (2026-10-21) EP-M2 Green. `toolchain-pins.json`,
  `scripts/provision_tools.py` (standard library only), and the `tools` and
  `test-scripts` targets added. Twelve pytest cases pass, covering O1, O2,
  pin-file validation, reuse, and the corrupt-cache refetch.
- [x] (2026-10-21) EP-M2 provisioning evidence. `make tools` installed OPA
  1.21.2 and Lean 4.35.0 under digest-named directories (3.7 GiB including the
  retained archive). A second run with `socket.connect` denied succeeded in
  0.09 s, so a complete install is reused offline. `gh release verify-asset`
  verified the OPA digest against the v1.21.2 attestation.
- [x] (2026-10-21) EP-M3 Red. The smoke module was run before the Lean package
  existed. The Lean invocation failed with
  `no configuration file with a supported extension`, and the OPA cases were
  not yet runnable without a pin file.
- [x] (2026-10-21) EP-M3 Green. `lean/lean-toolchain`, `lean/lakefile.toml`,
  `lean/PoleanSmoke.lean`, and the pinned lake's `lean/lake-manifest.json` were
  added; `tests/toolchain/smoke.rs` runs OPA 1.21.2 and the pinned Lake under
  `env_clear()`. Both O6 negative controls failed the build as required: a
  `sorry` declaration, and an altered `#guard_msgs` text.
- [x] (2026-10-21) EP-M3 CI. `.github/workflows/toolchain.yml` added with
  path filters, `permissions: contents: read`, every action pinned by SHA, and
  the verified archives cached on `hashFiles('toolchain-pins.json')`.
- [x] (2026-10-21) EP-M2 and EP-M3 gates, first run: nine of eleven passed
  (`check-fmt`, `typecheck`, `lint`, `test`, `test-scripts`, `toolchain-smoke`,
  `audit`, `SHA256SUMS`, `actionlint`). `make markdownlint` failed on 286
  findings, all inside the gitignored Lean distribution under `.polean/tools/`,
  and `make design-check` could not start because the scratch interpreter lacked
  `jsonschema`.
- [x] (2026-10-21) EP-M2 and EP-M3 repair: `.markdownlint-cli2.jsonc` now
  ignores `**/.polean/**` and `**/.lake/**`, which are generated or provisioned
  trees; `validation/requirements.txt` was installed into the scratch
  interpreter. Both gates pass on a direct re-run.
- [x] (2026-10-21) EP-M4 records. `docs/adrs/adr-0003-pin-the-e0-toolchain-tuple.md`
  written and indexed in `docs/contents.md`; technical design §6.1, §6.3, §11,
  §13, and References updated; **Toolchain tuple** added to `docs/context.md`;
  `lean/README.md`, `docs/repository-layout.md`, `docs/developers-guide.md`,
  `README.md`, and `AGENTS.md` updated; roadmap 1.2.1 ticked; `SHA256SUMS`
  refreshed for `README.md`, `docs/context.md`, and `docs/technical-design.md`.
- [x] (2026-10-21) EP-M2, EP-M3, and EP-M4 milestone gates, second run over
  the committed candidate: 13 gates, 11 passing on the first sweep and the two
  prose-and-digest failures repaired before the commit.
  `coderabbit review --agent` then covered the committed scope against base
  `24f3874` and returned eight unique findings, seven repaired and one
  dispositioned as contradicting the plan's required `=` pins. Recorded in
  Surprises and Artefacts.

## Surprises & discoveries

- Observation: the O3 failure point is `eval_rule`, not `add_policy`.
  `add_policy` accepts a source that calls a built-in from the disabled `regex`
  feature group; the "could not find function regex.match" error is raised at
  evaluation.
  - Evidence: a scratch probe against 0.13.0 and the O3 test, which evaluates
    and asserts only `is_err()` on the combined result.
  - Impact: none for the test, which already combines the two steps and never
    asserts the message. A runtime that fails closed at evaluation is enough
    for O3, and the next consumer must not expect `add_policy` to reject such a
    policy.
- Observation: the deny-all mutation parses to a module whose only rule is the
  default, so O5's non-vacuity check discriminates on rule kind rather than on
  module count.
  - Evidence: `deny_all_ast_has_only_a_default_rule` and the accepted snapshot
    for the baseline, which shows `[Default, Spec, Spec]` and statement counts
    `[3, 4]`.
  - Impact: none; recorded so 1.2.2 can rely on the shape.
- Observation: Regorus 0.13.0's AST JSON serialization is byte-identical to
  0.12.0's for the baseline source. The snapshot accepted under 0.12.0 passed
  unchanged after switching the pin to 0.13.0.
  - Evidence: the `=0.13.0` test run in
    `/tmp/green13-test-polean-1-2-1-pin-a-tested-compatibility-tuple.out`.
  - Impact: supports the Decision log's choice of 0.13.0; the AST is not a
    reason to prefer 0.12.0.
- Observation: `rstest` 0.27.0's fixture expansion wraps a single-expression
  body in braces, which `unused_braces` rejects under this repository's lint
  policy. The upstream remedy is a helper attribute macro that is not exported
  by the published crate, and an `#[expect]` on the fixture item is reported as
  unfulfilled; a module-scoped `#![expect(unused_braces, reason = ...)]` is the
  form that holds.
  - Evidence: three `cargo test` runs in `/tmp/green12d-…out`; the fixture body
    is deliberately written multi-line as well.
  - Impact: `tests/toolchain/bdd.rs` carries one module-scoped expectation. If
    a later `rstest` stops adding the braces, the expectation warns rather than
    silently claiming a lint it no longer suppresses.
- Observation: Regorus 0.13.0 was released 2026-10-14. Its feature table
  matches 0.12.0's: the default is `full-opa`, `arc`, and `rvm`, and `ast` is
  opt-in. The licence is `MIT AND Apache-2.0 AND BSD-3-Clause`.
  - Evidence: the crates.io API and the `regorus-v0.13.0` `Cargo.toml`.
  - Impact: both are candidates. The licence feeds the 5.2.2 decision.
- Observation: `std` + `ast` suffice for every API this task uses. The resolved
  set is `["ast", "msvc_spectre_libs", "rand", "std"]`, because `std` enables
  the other two. Regorus already defaults `strict_builtin_errors` to true.
  `eval_rule` returns:
  - `Ok(Value::Bool)` for a defined decision;
  - `Ok(Value::Undefined)` when undefined;
  - `Err` for an invalid rule path, a rule conflict (detected only at
    evaluation), or a strict built-in error.

  `Value::Undefined.to_json_str()` yields the string `"<undefined>"`.
  - Evidence: scratch builds by the Rust reviewer, 2026-10-17.
  - Impact: tests match on `Value` variants and never on JSON text or error
    messages, whose wording changed in 0.13.0.
- Observation: OPA `eval --format json` output shapes:

  | Case                                        | Stdout                                         | Exit |
  | ------------------------------------------- | ---------------------------------------------- | ---- |
  | Defined result                              | `{"result":[{"expressions":[{"value":...}]}]}` | 0    |
  | Undefined, including a misspelt entry point | `{}`                                           | 0    |
  | Conflict, built-in, or parse error          | `{"errors":[{"code":...}]}`                    | 2    |
  | Missing input                               | empty (stderr only)                            | 2    |

  - Without `--strict-builtin-errors`, a built-in error becomes `{}` with
    exit 0.
  - Evidence: the Rust reviewer's probe of the pinned binary.
  - Impact: the smoke test asserts the exact defined shape and passes
    `--strict-builtin-errors`. The table is recorded for 2.3.2, which owns the
    typed classification.
- Observation: Lake without `defaultTargets` prints "Nothing to build" and
  exits 0 even when the module contains `sorry`. A `weak.` option prefix is
  silently ignored, while a misspelt plain option fails. Lake needs no
  environment variables under `env -i`. Setting `HOME` would move its cache
  into the home directory.
  - Evidence: the Lean reviewer's v4.35.0 scratch runs.
  - Impact: the lakefile sets `defaultTargets`, and the test builds the target
    explicitly and checks for its `.olean`. Children get an empty environment.
- Observation: Lean v4.35.0 removed `Lean.ofReduceBool` and
  `Lean.trustCompiler`. `native_decide` asserts per-use axioms named
  `*._native.native_decide.ax_*`. `lake check` exists but needs `bwrap`, which
  is absent on this host. Running `leanchecker` directly needs `PATH`.
  - Evidence: v4.35.0 release notes (PR #14953 and #14990) and scratch runs.
  - Impact: these are inputs for 1.2.3, recorded in ADR-003. No action here.
- Observation: Python 3.14's `tarfile` `data` filter rewrites `/etc/x` to
  `etc/x` instead of rejecting it. The real Lean archive has 17,925 files and
  13 relative symlinks, all under one `lean-4.35.0-linux/` root, and passes the
  filter unchanged with modes intact. Some distribution Python builds lack
  `_zstd`.
  - Evidence: the supply-chain reviewer's probe.
  - Impact: the script checks that every member is under the pinned `root`
    before extracting. It imports `compression.zstd` up front with a clear
    error.
- Observation: OPA v1.21.2 is an immutable, attested release. Lean v4.35.0 is
  neither (`"immutable": false`; the attestations API returns 404). Both pinned
  digests match the GitHub asset API.
  - Impact: ADR-003 records the provenance of each pin separately.
- Observation: `cargo audit` reports one allowed warning, not a vulnerability:
  `basic-toml` 0.1.10 (RUSTSEC-2026-0374, unmaintained), reached through
  `rstest-bdd` → `i18n-embed` → `i18n-embed-impl` → `i18n-config`. It is a
  dev-dependency, and no patched `basic-toml` exists.
  - Evidence: `/tmp/audit-6906cae8-368b-487e-a47f-f2ac511a2316-…out`.
  - Impact: recorded rather than suppressed; 5.2.2 owns the dependency
    disposition for non-dev crates.
- Observation: Python 3.14's `compression.zstd.ZstdCompressor().compress()`
  returned zero bytes on this host, while the module-level
  `compression.zstd.compress(data)` wrote a frame that `tarfile` in `r:zst`
  mode reads. The script uses the module-level function.
  - Evidence: a scratch probe during EP-M2.
  - Impact: a future refactor to the class API would produce an empty
    extraction rather than an error, so the archive test is what catches it.
- Observation: the pinned Lake needs `--dir <package>`; invoked from the
  repository root it reports
  `no configuration file with a supported extension`, because it looks for
  `lakefile.lean` in the working directory.
  - Evidence: EP-M3 Red.
  - Impact: the smoke test passes `--dir` with an absolute path and also sets
    the child's working directory to the repository root.
- Observation: an extracted Lean release contains three upstream `README.md`
  files, so `markdownlint-cli2 "**/*.md"` lints provisioned tool content unless
  it is ignored. No repository-owned Markdown was affected.
  - Evidence: 286 findings, all under `.polean/tools/lean/<digest>/src/lean/`.
  - Impact: `**/.polean/**` and `**/.lake/**` are now ignored. Both are
    generated or provisioned trees, and `.polean/` could not be committed
    anyway. `make fmt` would otherwise rewrite upstream files.
- Observation: the milestone CLI review found eight unique defects that the
  deterministic gates cannot see. Two were functional:
  `tests/toolchain/regorus.rs` passed `"{context}: head"` as a plain `&str`, so
  three failure diagnostics would have printed the literal braces, and
  `urllib.request.urlopen` had no timeout while the run held the exclusive
  tools lock. One was a real coverage gap: the `toolchain` workflow did not
  trigger on `examples/tenant-write/**`, which both smoke tests read. One was a
  repository rule the file breached: `regorus.rs` was 422 lines against the
  400-line limit in `AGENTS.md`.
  - Evidence:
    `/tmp/coderabbit-milestone-polean-1-2-1-pin-a-tested-compatibility-tuple.out`;
    every finding was re-checked against source before repair.
  - Impact: the module was split into `engine.rs` (the shared engine
    configuration and entry point) and `json.rs` (the AST accessors), and a
    regression test now injects `urlopen` to observe the timeout. One finding
    was rejected: caret dev-dependency requirements would contradict the plan's
    required `=` pins, so ADR-003 now records why the exact pins stand.
- Observation: the containment check's leading-component test was not a
  containment proof. `PurePosixPath("root/../escape").parts` is
  `("root", "..", "escape")`, so `root/../escape` passed the `parts[0] == root`
  test; the `data` filter then rewrote it to `escape` inside the staging
  directory, so the member vanished rather than being refused. The second CLI
  pass found this.
  - Evidence: a probe of `PurePosixPath(...).parts` and the new
    `dot-dot-inside-the-root` parametrization of
    `test_install_refuses_an_escaping_member`. Without the `..` check that case
    installs successfully instead of raising.
  - Impact: `_check_archive` now rejects any `..` component and any link whose
    target resolves outside the root. A scan of the real pinned archive found
    18,571 members, 13 symlinks, 0 `..` components, and 0 escaping links, so
    the hardened check still accepts it; a forced re-extraction of the real
    archive through `make tools` confirms that end to end.
- Observation: a hard link's `linkname` is archive-root-relative, while a
  symbolic link's is relative to the member's own directory. The first version
  of `_require_link_inside` resolved both against the member's directory, so
  `root/bin/link -> root/bin/../../etc/passwd` resolved to `root/etc/passwd`
  and passed.
  - Evidence: `test_install_refuses_a_hard_link_resolved_from_the_wrong_base`
    fails with "DID NOT RAISE ArchiveEscape" when the base distinction is
    removed, and passes with it.
  - Impact: the base now depends on `member.issym()`. The pinned Lean archive
    contains no hard links, so this cannot regress on the artefact the
    repository actually provisions; it matters for the general containment
    claim.
- Observation: the second milestone CLI review over `efcdc91` returned nine
  unique findings, none above `minor`, and none a security or correctness
  defect in the pinned path. Six were actioned.
  - `main` caught only `ProvisionError`, so `URLError` and `HTTPError` — both
    `OSError` subclasses — escaped as a traceback on an unreachable network, a
    full disk, or an unwritable directory. It now catches `OSError` too, with a
    test that fails without the change. `main` takes the pin path, tools
    directory, and fetcher as defaults so the test injects them rather than
    patching, consistent with the rest of the script.
  - Two en-GB-oxendict spellings (`neutralized`, `parametrization`), both
    introduced by the previous commit.
  - `.github/workflows/toolchain.yml` now has a `concurrency` group that
    cancels a superseded pull request run but never a `main` push. Two
    concurrent runs would each attempt a 628 MiB provisioning against one
    cache key.
  - `tests/toolchain/bdd.rs`'s module-wide `#![expect(unused_braces)]` is now
    scoped to a private `fixture` submodule that re-exports `state`. Verified
    load-bearing: rewriting the fixture body multi-line makes the expectation
    report itself unfulfilled.
- Observation: the third CLI review over `d8f0c4a` returned three unique
  findings. Two were actioned.
  - The `concurrency` group keyed on `github.head_ref`, which is only the
    source branch name, so two pull requests from forks with the same branch
    name would cancel each other. It is now keyed on
    `github.event.pull_request.number`, which is unique within the repository,
    with `github.ref` for the push case.
  - ADR-003 said "seven dev-dependencies"; `Cargo.toml` lists eight. Corrected.
  - The docstring-style finding repeats the earlier one and is covered by the
    recorded decision above.
- Observation: the root README's contributor detail was trimmed to a signpost,
  with the detail relocated into the developer guide rather than deleted, and
  the ExecPlan added to `docs/contents.md`, which listed neither ExecPlan even
  though ADR-003 links to this one.
- Observation: a `pull_request` path filter is evaluated against a three-dot
  diff from the merge base, not against the latest commit. The last two commits
  of this branch touch only the ExecPlan, yet CI and Design contracts ran on
  both and Toolchain ran on the second. Reasoning about a single commit's paths
  is therefore the wrong model for whether a workflow should have run.
  - Evidence: the six runs listed in Artefacts, checked against GitHub's
    workflow-syntax documentation for `on.<pull_request>.paths`.
  - Impact: the false "triggers no run" claim previously recorded here was
    replaced with the observed runs. The practical consequence is that a
    docs-only commit on an open pull request still re-runs every unfiltered
    workflow, so a superseded Toolchain run is expected and is cancelled by the
    concurrency group rather than being evidence of a path-filter mistake.
- Observation: the hosted completeness assessment found that O7 was checked
  less strictly than it is written. The plan requires `lean --version` to equal
  the exact recorded string, but the test accepted any line starting
  `Lean (version 4.35.0,`, which admits a different commit, platform, or build
  type. The OPA check had the same shape: a substring test accepts
  `Version: 1.21.20` against a `1.21.2` pin.
  - Evidence: the assessment reply
    (`https://github.com/leynos/polean/pull/11#issuecomment-6322631279`), whose
    finding was re-checked against `tests/toolchain/smoke.rs` before repair.
    Two negative controls then confirmed the repair is load-bearing: changing
    one character of the pinned commit, and shortening the OPA version pin,
    each fail the tightened assertion, while the unmodified pins pass all seven
    smoke tests.
  - Impact: the complete Lean identity is now `lean.identity` in
    `toolchain-pins.json`, so the pin file stays the single authority, and the
    provisioner rejects an identity that does not embed the pinned version. The
    OPA and Rego checks compare whole lines. The digest gate always made the
    practical risk small, as the assessment noted; this closes the acceptance
    gap rather than an exposure.
- Observation: three further findings were judged unwarranted and left alone.
  - NumPy-style `Parameters`/`Returns`/`Raises` sections on the
    `provision_tools.py` public functions. No Python in this repository uses
    them (`validation/` has none), and the script's existing prose docstrings
    state the same obligations without a new convention. Introducing the style
    for one file would make the repository less consistent, not more.
  - The `docs/execplans/…` Stage A digest repetition. It is the plan's
    pre-execution observation record: a historical statement of what was
    observed on 2026-10-17 and what would trigger the Toolchain tolerance.
    Rewriting it to point at `toolchain-pins.json` would destroy the record of
    what was checked before the pin existed.
  - A `name:` on the `toolchain-smoke` job. `ci.yml` and `design.yml` name
    neither of their jobs, so adding one here alone is inconsistent, and the
    job identifier already renders in the checks list.
- Observation: the formal CodeRabbit review
  ([5604162108](https://github.com/leynos/polean/pull/11#pullrequestreview-5604162108))
  on `fd992e3` returned `CHANGES_REQUESTED` with five inline comments, while
  the `CodeRabbit` commit status read `success`. The two are separate signals:
  the status reports that a review ran, not what it concluded. Each comment was
  checked against the tree before any change.
  - Repaired, with the finding restated in this record's own terms:
    - `scripts/provision_tools.py` — `download` was the only place the
      `total > limit` guard existed, and no test exercised it. `good_fetcher`
      raises `DownloadTooLarge` for an oversized payload itself, so deleting
      the production guard left the suite green. The new test drives the real
      `download` with an opener that yields one byte past the limit; removing
      the guard makes it fail.
    - `scripts/provision_tools.py` — `_is_complete` answered `False` for any
      `OSError`, so an unreadable `COMPLETE` marker sent `_install_lean` into
      the repair path, which removes the tree and re-downloads the archive. A
      marker that cannot be inspected is an environment failure and now
      propagates; `main` already reports an `OSError` as one line. The new test
      denies `read_text` on the marker and asserts the install survives and no
      Lean URL is fetched. Restoring the `except OSError` makes it fail.
    - `docs/developers-guide.md` — the pin procedure never mentioned
      `lean/lean-toolchain` or the regenerated `lean/lake-manifest.json`. Both
      are now numbered steps, and the manifest step says to commit only when
      the content changed.
    - `docs/technical-design.md` — Regorus 0.13.0 was released on 2026-10-14,
      so R3 cannot have been inspected on 2026-10-04. R3 joins R12 and R13 in
      the exception list, and the `SHA256SUMS` entry was recomputed.
    - `docs/repository-layout.md` — the install path is
      `.polean/tools/<name>/<sha256>/`, as `_install_opa` and `_install_lean`
      both do; the prose omitted the name segment.
    - This record — `synchronisation` was the only surviving `-ise` spelling
      this branch introduced, and the new-files inventory omitted
      `scripts/tests/conftest.py`, `tests/toolchain/engine.rs`, and
      `tests/toolchain/json.rs`.
  - Declined, for the reason already recorded above: the NumPy-docstring
    finding, which repeats the earlier one.
  - The pre-merge table in the same review's walkthrough edit named the same
    two code defects under "Testing (Overall)" and "Unit Architecture", plus
    two warnings. "Developer Documentation" named the incomplete new-files
    inventory, repaired above. "Testing (Compile-Time / Ui)" asked for the raw
    AST snapshot to be replaced with a projection; it is declined for the
    reason already recorded, because O5 requires the raw adapter input and the
    snapshot is the reviewed artefact.
  - Deviation from a suggested remedy: "Unit Architecture" proposed a
    `Result[bool, OSError]` return and propagation through `install_tools`.
    Letting `OSError` propagate achieves the same observable outcome with no
    new type, because `main` already catches it and reports one line. A
    bespoke result type would re-express in-band what Python already carries
    out of band, and would have to be unwrapped at every call site for a
    failure the caller cannot handle differently anyway.
- Note on the reviewer's own suggestion for the guide: it folded the Lean
  selector and the manifest into step 1's prose. Splitting them into their own
  numbered steps states the same requirements in an order a contributor
  follows, so the content was adopted and the suggested shape was not.
- Observation: all five inline dispositions and the four pre-merge rows were
  reconciled on `508151a` without a further code change. Each inline thread was
  answered in place and CodeRabbit marked it resolved: the four repairs as
  confirmed, and the docstring finding withdrawn. On the pre-merge rows it
  recorded both errors as resolved, accepted the `OSError` propagation in place
  of its suggested `Result[bool, OSError]`, and withdrew the AST-projection
  warning while accepting the recorded disposition as complete rather than
  deferred. Its reply also carried an Assessment-ID
  (`PR11-EP-COMPLETE-20261021`), because the question was posted as a tagged
  issue comment; that is a label on the reconciliation, not a second assessment
  request.
  - Evidence: issue comment
    [6323427714](https://github.com/leynos/polean/pull/11#issuecomment-6323427714)
    for the pre-merge rows; inline replies
    [4340430000](https://github.com/leynos/polean/pull/11#discussion_r4340430000),
    [4340428438](https://github.com/leynos/polean/pull/11#discussion_r4340428438),
    [4340429995](https://github.com/leynos/polean/pull/11#discussion_r4340429995),
    [4340431047](https://github.com/leynos/polean/pull/11#discussion_r4340431047),
    and
    [4340436014](https://github.com/leynos/polean/pull/11#discussion_r4340436014).
    CodeRabbit stated its checks were read-only source inspection; it did not
    execute the repository or reproduce the two negative controls, so those
    remain author-reported evidence.
  - Impact: the surviving `CHANGES_REQUESTED` review is the one submitted on
    `fd992e3`. It is not cleared by these replies, because a review's state
    belongs to the review, not to the threads beneath it. A fresh approval is a
    separate step, and the reconciliation reply explicitly did not change
    approval status.
- Hosted runs on `508151a`, observed 2026-10-21: CI (39652081314), Design
  contracts (39652081215), and Toolchain (39652081144) all completed with
  `success`. Sourcery skipped the pull request for exceeding its diff-size
  limit, and the Codex connector reported its usage limit; neither is a
  required check.
- Hosted runs on `4a7e2e9`, observed 2026-10-21: CI (39652830678), Toolchain
  (39652830159), and Design contracts (39652830180) all completed with
  `success`. That commit records this same reconciliation; it changed no
  source, test, workflow, or pin, which is why the three checks were
  re-observed rather than assumed to carry over from `508151a`. The commit that
  adds this paragraph starts a further set of runs; those are recorded in the
  delivery ledger, because writing them back here would start another cycle.

## Decision log

- Decision: pin Regorus 0.13.0, after testing 0.12.0 as well.
  - Rationale: the AST and decisions are identical. 0.13.0 is the current
    release, so Dependabot would otherwise propose it at once, and fixes land
    there. The cost is five more crates and a build script.
  - The owner may prefer 0.12.0's smaller graph. Answering at plan approval
    switches the pin at no other cost.
  - (Agent, 2026-10-17.)
- Decision: request only the Regorus features `std` and `ast`.
  - Rationale: the profile admits no built-in calls (TDD §4.1), and `ast`
    supplies `get_ast_as_json` (TDD §6.1).
  - ADR-003 records the resolved set, including `rand` and
    `msvc_spectre_libs`.
  - (Agent, 2026-10-17.)
- Decision: pin Lean v4.35.0 and the OPA v1.21.2 `opa_linux_amd64_static`
  binary.
  - Rationale: see Risks. (Agent, 2026-10-17.)
- Decision: provision through one standard-library Python 3.14 script, without
  Cyclopts, elan, curl, or shell pipelines.
  - Rationale: the scripting standards require Python for non-trivial scripts.
    The script takes no parameters, so a CLI framework would be unused.
    `tarfile` reads zstd, and the script launches no subprocess.
  - Considered: Make-only provisioning with `curl`, `sha256sum`, and
    `tar --zstd`. It is about 15 lines, but needs host tools, leaves the digest
    gate untested, and cannot parse the JSON pin file that the Rust tests
    read.
  - (Agent, 2026-10-17.)
- Decision: one integration-test binary, `tests/toolchain/main.rs`, with
  modules.
  - Rationale: several binaries sharing `tests/common/` fail `dead_code`
    under the denied-`allow` lint policy. One binary also links Regorus once.
  - `make toolchain-smoke` runs
    `cargo test --locked --test toolchain -- --ignored smoke::`.
  - (Agent, 2026-10-17; from the Rust review.)
- Decision: assert exact decision shapes. Do not build a decision classifier
  (no `DecisionError`).
  - Rationale: a typed classifier is 2.3.2's deliverable. A test-only copy
    would be dead weight when `src/oracle.rs` arrives.
  - (Agent, 2026-10-17; from the boundary review.)
- Decision: drop line-parsing of `Cargo.toml` and `Cargo.lock` (revision 1's
  O3).
  - Rationale: checking a manifest against itself proves nothing. Cargo already
    enforces the lockfile, and `--locked` keeps it so.
  - Replacement: behavioural checks. The disabled-feature test, the Lean and
    OPA version tests, and the Lean version compared with
    `lean/lean-toolchain`.
  - (Agent, 2026-10-17.)
- Decision: BDD scenarios express the Regorus decision behaviour once, as the
  readable specification of the in-process workflow. They do not repeat it in
  an rstest table.
  - The provisioned OPA decisions use `#[rstest] #[ignore]` cases, because
    rstest-bdd filters by tag at expansion and cannot be ignored.
  - (Agent, 2026-10-17.)
- Decision: cache only `.polean/tools/downloads`, keyed on
  `hashFiles('toolchain-pins.json')`.
  - Rationale: the archives are the 630 MB download; the extracted trees are
    3.7 GiB and re-extract in seconds from a verified archive. A pin change
    changes the key, so a stale archive can never satisfy a new pin.
  - Considered: caching the extracted trees too. Their cache entry would be
    several gigabytes and would need its own eviction policy for no measured
    gain.
  - (Agent, 2026-10-21.)
- Decision: add a path-filtered hosted `toolchain` workflow now.
  - Rationale: without it the tuple is "tested once on one machine". Roadmap
    4.3.2 keeps clean-environment replay and resource measurement.
  - This is a small pull-forward of CI coverage the owner may strike.
  - (Agent, 2026-10-17; from the supply-chain review.)
- Decision: keep three smoke requests inline in `tests/toolchain/support.rs`,
  and take the owner case from `examples/tenant-write/request.json`.
  - Rationale: they are tool smoke inputs, not profile fixtures. Adding them to
    `examples/` would widen the design pack that the Python harness owns.
  - (Agent, 2026-10-17.)
- Decision: leave `README.md`, `docs/users-guide.md`, and
  `validation/README.md` unchanged.
  - Rationale: no user-visible behaviour or public API changes. The validation
    record describes the Python lane, which is unchanged.
  - (Agent, 2026-10-17.)
- Decision: do not use Verus, Kani, or proptest in this task; see the
  Verification plan. (Agent, 2026-10-17.)
- Decision: split the Regorus observations into `engine.rs` and `json.rs`
  rather than leaving them in `regorus.rs`.
  - Rationale: `regorus.rs` was 422 lines against the 400-line module limit.
    The split also removes duplicated engine setup and rule-tag matching that
    `bdd.rs` and `regorus.rs` each carried, without changing a single
    observation. The snapshot is unchanged.
  - Considered: merging `rule_kind` and `head_kind` alone. That leaves the
    file over the limit and leaves the duplicated configuration in place.
  - (Agent, 2026-10-21; from the milestone CLI review.)
- Decision: keep the exact `=` dev-dependency pins and record the exception in
  ADR-003.
  - Rationale: the plan requires an `=` pin for each, and the intent statement
    belongs in the requirements rather than being inferred from the lockfile.
  - (Agent, 2026-10-21; disposition of a milestone CLI finding.)

## Outcomes & retrospective

Delivered. The tuple is pinned, provisioned behind a digest gate, and observed
by three lanes: `make test` for Regorus on every host, `make test-scripts` for
the provisioning script, and `make toolchain-smoke` for OPA and Lean.

What the work establishes, in its own terms:

- the pinned Regorus parses the baseline, exposes an AST that a reviewed
  snapshot records, compiles only the requested feature groups, and decides
  four hand-written requests as `T ∧ W ∧ (A ∨ (O ∧ ¬L))` predicts;
- the pinned OPA decides the same four requests the same way, from the same
  source bytes;
- the pinned Lean and Lake build a fixed smoke library and the kernel reports
  the axioms the source asserts;
- the provisioning script installs only digest-matching bytes, rejects path
  escapes and truncated streams, and reuses a complete install offline.

What it does not establish, and must not be read as establishing: any theorem
about a Polean policy, any source-to-model correspondence, any runtime
conformance claim, or any statement about a platform other than
`x86_64-unknown-linux-gnu`. ADR-003 records the boundary; the smoke tests state
it in their own comments.

Lessons worth carrying forward:

- Testing the next candidate before rejecting the current one paid off twice.
  0.12.0 and 0.13.0 produce a byte-identical AST and the same decisions, so the
  upgrade is free; the plan's default was therefore safe, and the record now
  says why rather than leaving a suspicion.
- The dependency delta was the least visible part of the change. Two offline
  lockfile probes and a build-log check settled it; reading `Cargo.toml` alone
  would have missed the unconditional C compiler build chain.
- The hosted workflow is the part that cannot be observed locally. It is
  path-filtered, so a pin change re-tests it, but its first green run remains
  evidence to collect after publication.
- Provisioning is deliberately slow on the first run and instant afterwards.
  Keeping the verified archive in `.polean/tools/downloads` is what makes the
  second run offline-capable, and the CI cache keys on the pin file for the
  same reason.

## Context and orientation

Polean is a design scaffold.

- `src/lib.rs` exports a status string, and `tests/stub.rs` checks it.
- At the start of this work, `Cargo.toml` had no dependencies and a strict
  lint set. The denied lints include `unwrap_used`, `expect_used` outside test
  functions, `print_*`, `indexing_slicing`, `allow_attributes`,
  `panic_in_result_fn`, and `str_to_string`. Docs and assertion messages are
  mandatory.
- `clippy.toml` bans `std::env::var`.
- At the start, `lean/` held only a README, which forbade Lake configuration.
  This task reverses that for repository-owned configuration.
- Python under `validation/` checks design fixtures as a separate lane.
- `SHA256SUMS` is a manually maintained manifest. It covers `README.md`,
  `contracts/`, `docs/context.md`, `docs/technical-design.md`,
  `docs/terms-of-reference.md`, `examples/`, and `validation/`.
- Hosted CI (`.github/workflows/ci.yml`) runs `make check-fmt`, markdownlint,
  `make typecheck`, `make lint`, and `make test`.

Terms used in this plan:

- **Toolchain tuple**: the exact identities a run used. These are the Rust
  toolchain and `Cargo.lock`, the Regorus version and resolved features, the
  Lean toolchain and package manifest, and the OPA binary digest. TDD §7 later
  binds evidence to this tuple (4.1.1), which should hash the binaries it runs
  rather than trust the pin file.
- **Provisioning**: downloading a pinned artefact, checking its size and
  SHA-256 before use, and installing it into a digest-named directory under
  `.polean/tools/`. Git already ignores `.polean/`.
- **Smoke check**: an observation that a tool works on a fixed input. It is
  not admission, not a theorem about Polean, and not conformance evidence.

The baseline is `examples/tenant-write/policy.rego`. It declares package
`authz`, `import rego.v1`, `default allow := false`, and two `allow` rules. The
intended decision is `T ∧ W ∧ (A ∨ (O ∧ ¬L))` (TDD §3). The smoke expectations
come from that formula, not from either runtime.

Skills to load:

- `execplans` and `rust-router`.
- `rust-unit-testing` for rstest, googletest, and insta.
- `arch-supply-chain` for exact pins and `cargo audit`.
- `arch-decision-records` for ADR-003.
- `en-gb-oxendict-style`.
- `codegraph-mcp` for navigation and `firecrawl-mcp` for upstream lookups.

Documents to read before the matching milestone:

- `docs/rust-testing-with-rstest-fixtures.md`
- `docs/reliable-testing-in-rust-via-dependency-injection.md`
- `docs/rust-doctest-dry-guide.md`
- `docs/complexity-antipatterns-and-refactoring-strategies.md`
- `docs/scripting-standards.md`
- `docs/documentation-style-guide.md`

Delegate full gate runs to `scrutineer`.

## Conformance basis

Upstream at commit `24f3874`:

- ToR draft v0.1: A4, G5, G6, SC5, and OQ5.
- Technical design draft v0.1: §§3, 4.1, 6, 6.1, 6.3, 7, 11, and the §13
  "Toolchain tuple" row.
- ADR-001: the deferred Lean pin, and "revisit the Rust pin alongside the
  tuple during E0".
- ADR-002: the assurance boundary.
- repository-layout.md §Ownership boundaries.
- Roadmap RM-1.2.1.

```plaintext
TDD §11 pins, ADR-001 E0 deferral -> RM-1.2.1 -> EP-M1..M3 -> make toolchain-smoke; ADR-003
TDD §6.1 Regorus ast feature      -> RM-1.2.1 -> EP-M1 -> toolchain::regorus AST snapshot
TDD §6.3 v1, strict errors, entry -> RM-1.2.1 -> EP-M1/M3 -> explicit runtime options in both oracles
TDD §13 toolchain row (E0 need)   -> RM-1.2.1 -> EP-M4 -> §13 row and ADR-003 cite the tuple
```

This task supplies a pre-provisioned toolchain. It does not discharge A4
(offline verification), A5 (1.2.2), the axiom policy (1.2.3), enforced
isolation (1.2.4), or SC5 clean-machine replay (4.3.2).

## Verification plan

This task introduces no contractual business logic and no policy semantics. Its
code is provisioning and smoke-test plumbing.

- Verus is not warranted. There is no lemma, and no pure kernel whose
  correctness an unbounded proof would establish. The Polean theorems (V1–V5)
  are Lean obligations in phase 2.
- Kani and proptest are not warranted either. Each check below is a named
  scenario over a fixed artefact, and a generator would only restate the
  implementation.

The obligations follow.

- O1, digest gate: the script installs an artefact only when the downloaded
  byte count and SHA-256 both equal the pin. Otherwise nothing appears at the
  final path, and the script exits non-zero with a remediation message.
  - Method: pytest with a fetcher injected over a temporary directory. The
    fetcher is a parameter of the install function, not a monkeypatch.
  - Artefact: `scripts/tests/test_provision_tools.py`.
  - Evidence: `make test-scripts`.
  - Non-vacuity: a one-byte flip and a truncated stream are both rejected,
    while the correct bytes install. A second run with a fetcher that raises
    succeeds, which proves that a complete install is reused without
    downloading.
- O2, containment: archive members outside the pinned `root/` are refused
  before extraction. Extraction passes `filter="data"` explicitly. A crash
  leaves only a temporary sibling, which the next run deletes while holding
  `.polean/tools/.lock`.
  - Method: pytest with a crafted archive containing a `../escape` member and
    an `/abs` member. Both must be refused, and nothing may appear outside the
    temporary directory.
  - The `data` filter itself is a stdlib axiom; the test checks that the script
    uses it.
- O3, the compiled feature set equals the recorded set: with the `regex`
  group disabled, a policy calling `regex.match` does not evaluate to a
  decision under the pinned features.
  - Method: rstest in `tests/toolchain/regorus.rs`, run by `make test`.
  - Non-vacuity: the same policy without the call does evaluate. The test fails
    if feature unification ever enables `full-opa`.
  - The exact failure point (`add_policy` or `eval_rule`) is recorded in
    Surprises during EP-M1. The test asserts `is_err()` on the combined
    result, never the error text.
- O4, independent expectations: each smoke request carries its fact vector and
  expected decision, written by hand from `T ∧ W ∧ (A ∨ (O ∧ ¬L))`:

  | Request                                                     | Facts        | Expected |
  | ----------------------------------------------------------- | ------------ | -------- |
  | Owner-unlocked write (`examples/tenant-write/request.json`) | T W ¬A O ¬L  | true     |
  | Cross-tenant administrator write                            | ¬T W A ¬O ¬L | false    |
  | Same-tenant administrator write on a locked resource        | T W A ¬O L   | true     |
  | Owner read                                                  | T ¬W ¬A O ¬L | false    |

  - Regorus: the decision must be exactly `Value::Bool(expected)`. Undefined
    or `Err` fails. Checked in BDD scenarios run by `make test`.
  - OPA: exit 0 and stdout parsing to exactly
    `{"result":[{"expressions":[{"value":<expected>,...}]}]}`. That means one
    result, one expression, a JSON Boolean, and no `errors` key. Run with
    `--strict-builtin-errors` and stdin input (`-I`), in ignored rstest cases
    under `make toolchain-smoke`.
  - Non-vacuity: both outcomes occur. The cross-tenant request is the input on
    which the E0 mutation diverges from the baseline. `"false"` or `{}` cannot
    satisfy the Boolean equality.
- O5, adapter-input record: the raw string from `get_ast_as_json` for the exact
  baseline bytes is snapshotted with `insta::assert_snapshot!`. The test also
  asserts, through `serde_json::Value::pointer`:
  - one module, with `source.file == "policy.rego"` and `source.contents`
    equal to the baseline;
  - package `authz` and `ast.rego_v1` true;
  - three rules: one `Default` for `allow`, and two `Spec` rules with head
    `allow`, each with one body whose `query.stmts` have lengths 3 and 4.
  - Non-vacuity: the deny-all mutation's source is checked in the same test
    and must show one `Default` rule and no `Spec`.
  - These are observations of the adapter input. Node-by-node coverage is
    1.2.2.
- O6, a non-vacuous Lean build: `lake build PoleanSmoke` must exit 0, and
  `lean/.lake/build/lib/lean/PoleanSmoke.olean` must then exist.
  - The lakefile sets `defaultTargets = ["PoleanSmoke"]` and a plain
    `leanOptions.warningAsError = true`, with no `weak.` prefix.
  - Each smoke theorem is followed by `#guard_msgs in #print axioms`, with
    the exact expected text. `and_comm_on_boolPairs` reports no axioms;
    `boolPairs_complete` reports `propext`.
  - Negative controls, run once and recorded in Artefacts:
    - appending `theorem bad : False := by sorry` fails the build;
    - changing an expected `#guard_msgs` text fails the build.
  - Limit: this does not detect axioms in general, `native_decide`, or
    in-file option overrides. That audit is 1.2.3.
- O7, version identity: the tools report their pinned identities.
  - `lean --version` equals, character for character, the `lean.identity`
    string in `toolchain-pins.json`:

    ```plaintext
    Lean (version 4.35.0, x86_64-unknown-linux-gnu, commit eb6cafb7290e8bd5f2937522d8d804f58f02bafc, Release)
    ```

    and `lean/lean-toolchain` selects exactly `leanprover/lean4:v4.35.0`.
    The identity lives in the pin file rather than in the test, so the pin
    file stays the single authority; the provisioner rejects an identity that
    does not embed the pinned version.
  - `opa version` reports the whole lines `Version: 1.21.2` and
    `Rego Version: v1`. Whole-line comparison matters: a substring test would
    accept `Version: 1.21.20` against a `1.21.2` pin.
  - Non-vacuity: the provisioning step re-hashes the installed OPA binary on
    every run, so a replaced binary fails before the version test can pass.

External axioms, not verified here:

- SHA-256 collision resistance.
- Regorus, OPA, Lean, and Lake behaving as documented.
- Python's `tarfile` and `fcntl`.
- GitHub serving the published assets.

Residual gap: the digests come from GitHub release metadata. The OPA asset is
additionally checked with `gh release verify-asset` at pin time. Nothing here
shows that upstream built either artefact from its tagged source.

## Plan of work

Stage A, recheck (repeat at the start of EP-M1 and EP-M2):

- Re-read the crates.io entries for Regorus 0.12.0 and 0.13.0.
- Re-read the GitHub asset digests and sizes for `opa_linux_amd64_static`
  v1.21.2 and `lean-4.35.0-linux.tar.zst`.
- Run
  `gh release verify-asset v1.21.2 <downloaded opa> -R open-policy-agent/opa`
  once and record the output.
- Values observed on 2026-10-17:
  - OPA:
    `830ab11f61ffec760b5e19db0d7ad02bf037e13f5ef4d665faeaf17632177527`,
    64,181,524 bytes.
  - Lean:
    `7289597a44543246c9227b8cae501767e095ce17f6d448b95fc4a8dc97268f6d`,
    594,481,375 bytes.
- Any difference triggers the Toolchain tolerance.

Stage B, EP-M1, Regorus in process:

1. Red.
   - Add dev-dependencies, each with an `=` pin at the current release observed
     on 2026-10-17:

     | Crate | Version |
     | --- | --- |
     | `rstest` | 0.27.0 |
     | `googletest` | 0.14.3 |
     | `pretty_assertions` | 1.4.1 |
     | `insta` | 1.49.0 (no extra features) |
     | `serde_json` | 1.0.154 |
     | `rstest-bdd` | 0.6.0 |
     | `rstest-bdd-macros` | 0.6.0 |

   - Create `tests/toolchain/main.rs`, with a crate-level `//!` doc and module
     declarations.
   - Create `tests/toolchain/support.rs`: the baseline and deny-all sources
     via `include_str!`, the O4 request table, and repository-relative paths
     built from `env!("CARGO_MANIFEST_DIR")`.
   - Create `tests/toolchain/regorus.rs` for O3 and O5.
   - Create `tests/toolchain/bdd.rs` for the O4 Regorus scenarios.
   - Create `tests/features/regorus_baseline.feature` (see Interfaces).
   - Run `cargo test --test toolchain`. Expect a compile failure: crate
     `regorus` unresolved.
2. Green.
   - Add this line under `[dev-dependencies]`:

     ```toml
     regorus = { version = "=0.12.0", default-features = false, features = ["ast", "std"] }
     ```

   - Run the tests, then review and accept the snapshot with
     `cargo insta review`.
   - Switch to `=0.13.0`, rerun, and confirm that the snapshot is unchanged.
     Keep the version the Decision log selects.
   - Record `cargo tree -e features -i regorus` in Artefacts.
3. Write each Regorus engine call out explicitly:
   - `Engine::new()`, then `set_rego_v0(false)` and
     `set_strict_builtin_errors(true)`, matching `contracts/profile.json`
     `runtime_options`;
   - `add_policy("policy.rego".to_owned(), baseline.to_owned())`;
   - `set_input_json(request)`;
   - `eval_rule("data.authz.allow".to_owned())`.

   For the lints: BDD step functions and helpers return `Result<(), String>`
   instead of calling `expect`. `#[gtest]` tests use only `verify_that!` and
   `?`. JSON access goes through `.pointer(..)`, never indexing.
4. Refactor. Run `make check-fmt`, `make typecheck`, `make lint`, and
   `make test`. Run `make audit` once and record its output. Commit.

Stage C, EP-M2, pins and provisioning:

1. Red. Write `scripts/tests/test_provision_tools.py`, covering O1, O2,
   pin-file validation, and the offline re-run.
   - Pin-file validation rejects unknown keys and a `root` that does not embed
     the version.
   - Add `scripts/requirements-test.txt` (`pytest==9.1.1`).
   - Add the `test-scripts` target: `$(PYTHON) -m pytest scripts/tests`.
   - Run it and expect a collection error, because the module is missing.
2. Green. Add `toolchain-pins.json`:

   ```json
   {
     "format": 1,
     "platform": "x86_64-unknown-linux-gnu",
     "opa": {
       "version": "1.21.2",
       "url": "https://github.com/open-policy-agent/opa/releases/download/v1.21.2/opa_linux_amd64_static",
       "sha256": "830ab11f61ffec760b5e19db0d7ad02bf037e13f5ef4d665faeaf17632177527",
       "size": 64181524,
       "kind": "binary"
     },
     "lean": {
       "version": "4.35.0",
       "url": "https://github.com/leanprover/lean4/releases/download/v4.35.0/lean-4.35.0-linux.tar.zst",
       "sha256": "7289597a44543246c9227b8cae501767e095ce17f6d448b95fc4a8dc97268f6d",
       "size": 594481375,
       "kind": "tar.zst",
       "root": "lean-4.35.0-linux"
     }
   }
   ```

   Then write `scripts/provision_tools.py`: a uv-style shebang and PEP 723
   header with no dependencies, Python 3.14, standard library only. Under
   `fcntl.flock` on `.polean/tools/.lock` it does the following:
   - It deletes stale temporary siblings.
   - For each tool, a complete install is reused. A complete install is either
     `.polean/tools/<name>/<sha256>/`, containing a `COMPLETE` marker that
     holds the digest, or, for OPA, a binary whose re-hash matches.
   - Otherwise a verified archive in `.polean/tools/downloads/` is extracted.
   - Otherwise the artefact is fetched into a `mkstemp` file. The download
     stops once it exceeds the pinned size. Size and SHA-256 are checked
     before the file is renamed into place.
   - Installation goes through a `mkdtemp` sibling and an atomic rename. The
     OPA binary is made executable before the rename.
   - It prints the installed tool paths.

   Make the downloader a parameter of the install function so that tests inject
   it.
3. Add `tools` (`$(PYTHON) scripts/provision_tools.py`) and `test-scripts` to
   `.PHONY`, each with a `##` help comment. Neither goes into `all`, because
   the first run needs the network and about 3.7 GiB. Run the gates plus
   `make test-scripts` and `make tools`, then rerun `make tools` with the
   network disabled. Commit.

Stage D, EP-M3, Lean package, provisioned smoke checks, and CI:

1. Red. Write `tests/toolchain/smoke.rs`. Every test in it is `#[rstest]` plus
   `#[ignore = "needs make tools; run make toolchain-smoke"]`:
   - `opa_reports_pinned_version`
   - `opa_decides_baseline_source`, the O4 table with
     `opa eval --format json --strict-builtin-errors -d <baseline path> -I
     data.authz.allow` and the request on stdin
   - `lean_reports_pinned_version`
   - `lean_builds_smoke_library`, which runs
     `<lean root>/bin/lake build PoleanSmoke` in `lean/` and then checks for
     the `.olean`

   Tools are located through `toolchain-pins.json` and the digest-named path. A
   missing tool is a failure with a remediation message, never a skip. All
   children use `env_clear()`. Add a `toolchain-smoke` target that runs `tools`
   and then
   `cargo test --locked --all-features --test toolchain -- --ignored smoke::`.
   Run it and expect the two Lean tests to fail because the package and
   toolchain file are missing.
2. Green. Add:
   - `lean/lean-toolchain`, containing `leanprover/lean4:v4.35.0`;
   - `lean/lakefile.toml` (see Interfaces);
   - `lean/PoleanSmoke.lean` (see Interfaces);
   - `lean/lake-manifest.json`, generated by the pinned `lake build`;
   - `.lake/` in `.gitignore`.

   Run the O6 negative controls and record them.
3. Add `.github/workflows/toolchain.yml` with `permissions: contents: read`
   and every action pinned by SHA. Reuse the checkout, setup-rust, and
   setup-python SHAs from the existing workflows, plus `actions/cache` at a
   pinned SHA.
   - Triggers: `pull_request` and `push` to `main`, path-filtered to
     `toolchain-pins.json`, `lean/**`, `Cargo.toml`, `Cargo.lock`,
     `rust-toolchain.toml`, `scripts/**`, `tests/toolchain/**`,
     `tests/features/**`, `Makefile`, and the workflow itself; plus
     `workflow_dispatch`.
   - Steps: `make test-scripts`, then `make toolchain-smoke`, with
     `.polean/tools/downloads` cached on `hashFiles('toolchain-pins.json')`.
   - Timeout: 30 minutes.
4. Run the gates, `make test-scripts`, and `make toolchain-smoke`. Commit.

Stage E, EP-M4, records:

- Write `docs/adrs/adr-0003-pin-the-e0-toolchain-tuple.md`. It records:
  - the tuple, the requested and resolved Regorus features, and the
    dependency delta between 0.12.0 and 0.13.0;
  - each digest's provenance: OPA attested and immutable, Lean
    trust-on-first-use;
  - the Lean v4.35.0 rationale and the v4.34.1 floor;
  - keeping the Rust nightly, which resolves ADR-001's "revisit" consequence;
  - removing elan from the gate, with the source citation;
  - the 1.2.3 inputs: v4.35.0 axiom naming, `lake check` needing `bwrap`, and
    `leanchecker` needing `PATH`;
  - the licences, as inputs to 5.2.2.

  List the ADR in `docs/contents.md`.
- Update `docs/technical-design.md`:
  - §6.1: the tested Regorus version and features.
  - §6.3: the observed OPA output shapes, as input to 2.3.2.
  - §11: replace "Regorus 0.12.0 is an inspected frontend candidate" with the
    tested pin, linking ADR-003.
  - §13 toolchain row: E0 need met by ADR-003 and `make toolchain-smoke`.
  - References: the new upstream sources.
- Add **Toolchain tuple** to `docs/context.md`.
- Update `lean/README.md` and `docs/repository-layout.md`. They must reverse
  the Lake prohibition for repository-owned configuration, keep it for
  submitted configuration, and describe:
  - the non-semantic `PoleanSmoke` library, which 2.2.4 never imports;
  - `toolchain-pins.json`, `scripts/`, `tests/toolchain/`, and
    `.polean/tools/`.
- Update `docs/developers-guide.md` with:
  - prerequisites: Python 3.14 with `_zstd`, `pytest`, `cargo-insta`, network
    access for the first provisioning, and about 3.7 GiB of disk;
  - the new targets and the `toolchain` workflow;
  - how to change a pin: edit the authoritative file, run `make tools`,
    `make toolchain-smoke`, and `cargo insta review`, then update ADR-003;
  - test conventions: one integration binary, `Result`-returning steps and
    helpers, a committed and reviewed snapshot, and ignored provisioned tests.
- Add `make test-scripts` and `make toolchain-smoke` to the gate list in
  `AGENTS.md`.
- Refresh `SHA256SUMS` for the covered files edited: the technical design,
  context, and — once the README's toolchain section was trimmed — `README.md`.
  The manifest covers the root README but not `docs/developers-guide.md`,
  `docs/repository-layout.md`, ADR-003, or this plan, so editing those needs no
  refresh. A forgotten refresh is caught by the `sha256sum -c` gate, which is
  how the README omission surfaced.
- Mark roadmap 1.2.1 `[x]`, run every gate, and set Status to `COMPLETE`.

## Milestones and plateaus

- EP-M1, Regorus in process.
  - End state: a pinned Regorus. `make test` shows the baseline parsed, its AST
    snapshotted, the disabled built-in group unavailable, and four expected
    Boolean decisions.
  - Requirements: TDD §6.1, and §6.3 for Regorus.
  - Evidence: `toolchain::regorus` and `toolchain::bdd`.
  - Design adherence: no `src/` change; authorized dev-dependencies only.
  - Recovery: revert the commit.
  - Compatibility: none.
- EP-M2, provisioning.
  - End state: `make tools` installs digest-checked artefacts, and a second
    run works with the network disabled.
  - Requirements: TDD §11 pins.
  - Evidence: `make test-scripts` and the offline re-run transcript.
  - Design adherence: `.polean/tools/` stays untracked.
  - Recovery: `rm -rf .polean/tools`.
  - Compatibility: none.
- EP-M3, provisioned smoke.
  - End state: `make toolchain-smoke` passes locally and in the `toolchain`
    workflow.
  - Requirements: the RM-1.2.1 success criteria.
  - Evidence: the smoke log, the O6 negative-control transcripts, and the CI
    run URL.
  - Design adherence: Lake builds only repository content, and `lean/Polean/`
    stays reserved.
  - Recovery: revert the commit and delete `lean/.lake/`.
  - Compatibility: none.
- EP-M4, records.
  - End state: ADR-003 and the documents agree with the code, `SHA256SUMS`
    passes, and 1.2.1 is ticked.
  - Requirements: the TDD §13 E0 toolchain need.
  - Evidence: all gates pass, and a read-through confirms that no edited
    sentence widens one of the four claims.
  - Compatibility: none.

## Concrete steps

From the worktree root, run each command in order, teeing its output to
`/tmp/<action>-polean-1-2-1-pin-a-tested-compatibility-tuple.out`:

```bash
make check-fmt
make markdownlint
make typecheck
make lint
make test                      # Regorus tests run; smoke:: tests ignored
make test-scripts PYTHON=<python3.14 venv with scripts/requirements-test.txt>
make tools PYTHON=<same>       # network only on the first run
make toolchain-smoke PYTHON=<same>   # expect 7 passed: 3 single tests + 4 OPA cases
make design-check PYTHON=<venv with validation/requirements.txt>  # expect no diff
sha256sum -c --quiet SHA256SUMS      # expect no output
```

Expected tail of `make toolchain-smoke`:

```plaintext
test smoke::lean_builds_smoke_library ... ok
test smoke::lean_reports_pinned_version ... ok
test smoke::opa_decides_baseline_source::case_1 ... ok
...
test smoke::opa_reports_pinned_version ... ok
test result: ok. 7 passed; 0 failed; 0 ignored
```

## Validation and acceptance

Accepted when all of the following hold:

- A fresh clone passes `make tools && make toolchain-smoke`, and the
  `toolchain` workflow passes on the PR.
- `make tools` re-runs offline.
- `make test` passes in hosted CI without Lean or OPA.
- `Cargo.lock`, `lean/lake-manifest.json`, the Regorus feature list, and both
  digests are committed and named in ADR-003.
- The O6 negative controls are recorded.
- No document says the tuple proves, admits, or conforms.

Record the Red-Green-Refactor evidence in Artefacts:

- EP-M1: the unresolved-crate compile failure.
- EP-M2: the pytest collection error.
- EP-M3: the failing Lean smoke tests.

## Idempotence and recovery

`make tools` is idempotent and serialized by a lock file. It removes stale
temporary files, reuses complete installs, re-hashes the OPA binary, and never
overwrites a completed install. Deleting `.polean/tools/` or `lean/.lake/` is
always safe. Each milestone is one revertable commit, or a small series of them.

## Artefacts and notes

- `cargo tree -e features -i regorus`, 2026-10-21, Regorus 0.13.0:

  ```plaintext
  regorus v0.13.0
  ├── regorus feature "ast"
  │   [dev-dependencies]
  │   └── polean v0.1.0 (...)
  │       └── polean feature "default" (command-line)
  ├── regorus feature "msvc_spectre_libs"
  │   └── regorus feature "std"
  │       [dev-dependencies]
  │       └── polean v0.1.0 (...) (*)
  ├── regorus feature "rand"
  │   └── regorus feature "std" (*)
  └── regorus feature "std" (*)
  ```

  The requested set is `["ast", "std"]`; the resolved set is
  `["ast", "msvc_spectre_libs", "rand", "std"]`, because `std` enables the
  other two. `full-opa` and `arc` are absent, which is what O3 asserts
  behaviourally.

- O3 failure point: `eval_rule`, not `add_policy`. See Surprises.
- EP-M1 Red transcript:
  `/tmp/red-test-polean-1-2-1-pin-a-tested-compatibility-tuple.out`
  (`error[E0432]: unresolved import 'regorus'`).
- EP-M1 Green transcript on 0.13.0:
  `/tmp/green13-test-polean-1-2-1-pin-a-tested-compatibility-tuple.out` (8
  passed). The snapshot accepted under 0.12.0 is unchanged.
- Accepted snapshot:
  `tests/toolchain/snapshots/toolchain__regorus__baseline_ast_json.snap`.
- `make audit`, 2026-10-21: 226 crates scanned; one allowed warning
  (`basic-toml` unmaintained, dev-only). See Surprises.
- `gh release verify-asset v1.21.2 …`, 2026-10-21:

  ```plaintext
  Calculated digest …: sha256:830ab11f…77527
  Resolved tag v1.21.2 to sha1:3f7474e6b3c631821d8aac25ca444f7e75a56483
  Loaded attestation from GitHub API

  ✓ Verification succeeded! … is present in release v1.21.2
  ```

- EP-M2 Red transcript:
  `/tmp/red-test-scripts-polean-1-2-1-pin-a-tested-compatibility-tuple.out`
  (`ModuleNotFoundError: No module named 'provision_tools'`).
- EP-M2 Green: `make test-scripts` → 12 passed.
- EP-M2 provisioning: `make tools` printed both digest-named paths; the
  offline re-run succeeded in 0.09 s with `socket.connect` denied.
- EP-M3 Red: the pinned Lake from the repository root reported
  `no configuration file with a supported extension` before `lean/` existed.
- O6 negative controls, 2026-10-21:
  - `sorry`: `error: PoleanSmoke.lean:37:8: declaration uses 'sorry'`, exit 1.
    Log `/tmp/negcontrol-sorry-…out`.
  - Altered `#guard_msgs`: `❌️ Docstring on '#guard_msgs' does not match
    generated message`, exit 1. Log `/tmp/negcontrol-guardmsgs-…out`.
- Milestone CLI review, 2026-10-21:
  `coderabbit review --agent --committed --base-commit 24f3874c` (CLI 0.9.1)
  inspected all 34 changed files and returned 17 raw findings, 8 unique, with
  no rate limit or execution failure. Log
  `/tmp/coderabbit-milestone-polean-1-2-1-pin-a-tested-compatibility-tuple.out`.
  Dispositions in Surprises; seven repaired, one rejected with its own
  documented-exception alternative adopted in ADR-003.
- Milestone CLI reassessment, 2026-10-21: the same command over the repaired
  candidate (commit `678ecca`) returned no new rate limit or execution failure.
  It confirmed the `..` containment gap and the dead ADR-003 link in
  `lean/README.md`; both are repaired, and the remaining six items are
  low-priority editorial cleanup. Log
  `/tmp/coderabbit-reassess-polean-1-2-1-pin-a-tested-compatibility-tuple.out`.
- Containment repair evidence: `make test-scripts` → 18 passed, including
  `dot-dot-inside-the-root`, the escaping and inside-root link cases, and the
  hard-link base case; a scan of the real pinned archive rejected 0 of 18,571
  members; `make tools` re-extracted it successfully with the hardened check;
  `make toolchain-smoke` → 7 passed. The hard-link test was confirmed to fail
  without the base fix.
- Second milestone CLI review, 2026-10-21, over `efcdc91` (36 files, true merge
  base `24f3874c`): completed in one attempt, 10 raw findings reducing to 9
  unique, all `minor` or `trivial`. Log
  `/tmp/coderabbit-reassess2-polean-1-2-1-pin-a-tested-compatibility-tuple.out`.
  Dispositions in Surprises: six actioned, two rejected with reasons, one (the
  root README move) actioned together with the `docs/contents.md` entry.
- Third milestone CLI review, 2026-10-21, over `d8f0c4a` (36 files): completed
  in one attempt, 4 raw findings reducing to 3 unique. Log
  `/tmp/coderabbit-reassess3-polean-1-2-1-pin-a-tested-compatibility-tuple.out`.
  Two repaired (the fork-collision concurrency key and the ADR dependency
  count); the third repeats the docstring decision above.
- Published head: `2fd7cc07dc4da90c98fe0677a9dab01b65442824`, pushed over SSH
  and verified against `ls-remote`. All three workflows triggered on it:
  - [Toolchain run](https://github.com/leynos/polean/actions/runs/39645020719)
  - [CI run](https://github.com/leynos/polean/actions/runs/39645020530)
  - [Design contracts run](https://github.com/leynos/polean/actions/runs/39645020545)

  All three completed with `success` on `2fd7cc0`.
- Hosted runs on `847ac7c` and `6acfedd`, observed 2026-10-21: CI and Design
  contracts succeeded on both, and the Toolchain run succeeded on `6acfedd`.
  The Toolchain run on `847ac7c` was cancelled, because the concurrency group
  added in `d8f0c4a` cancels a superseded pull request run. That cancellation
  is the rule working, not a failure.
  - Evidence: run IDs 39645380708, 39645380608, 39645380443 (`847ac7c`) and
    39645450791, 39645450768, 39645450549 (`6acfedd`).
  - Correction: an earlier revision of this record claimed these two commits
    "correctly trigger no run" because they touch only the ExecPlan. That was
    wrong. `ci.yml` and `design.yml` declare no path filter, so they run on
    every synchronization; Toolchain's filter is matched by other paths in the
    pull request's three-dot diff from the merge base, not by the latest
    commit's alone. The claim had been reasoned from the two commits without
    checking the run list.
- Final local sweep, 2026-10-21, over `2fd7cc0`: 12 of 12 gates pass, with the
  three repair commits' changes included. `make markdownlint` initially failed
  on three bare URLs this record had written; they are now Markdown links and
  it passes. Logs are `/tmp/final-<gate>-…out`.
- Hosted assessment repair, 2026-10-21: `make check-fmt`, `typecheck`, `lint`,
  `test`, `test-scripts` (20 passed), `markdownlint`, and `toolchain-smoke` (7
  passed) all pass with the tightened O7 assertions. Logs are
  `/tmp/<gate>-…out` for this branch.

- Hosted completeness/correctness assessment, 2026-10-21, on `c38370e`:
  CodeRabbit echoed the Assessment-ID `PR11-EP-COMPLETE-20261021` and both
  SHAs, recorded O7 as satisfied by the repair, retained EP-M1 to EP-M4 as
  delivered, and found no remaining in-scope omission or correctness defect. It
  is a bounded assessment, not a guarantee that no defect exists. The reply is
  issue comment
  [6322701212](https://github.com/leynos/polean/pull/11#issuecomment-6322701212).
  Its first assessment named the O7 gap; this record's earlier revision had
  treated the version-prefix check as sufficient.
- Proof-specific assessment, 2026-10-21: **not applicable**, and recorded
  rather than posted. This task introduces no CrossHair, Kani, Verus, or
  LemmaScript proof artefact, and no proof-bearing code changed. References to
  those tools elsewhere in the repository are prose only: the Verus crate names
  in `Cargo.lock` and ADR-003 describe Regorus's resolved dependency graph and
  its licences, and this record and
  [1.1](1-1-ratify-assurance-and-release-gates.md) discuss why Verus, Kani, and
  property tests are not used here. The only Lean theorems are the two `decide`
  probes in `lean/PoleanSmoke.lean`, whose own module comment states it "states
  nothing about Polean policies". The technical design's
  `abstraction_preserves`, `check_sound`, and `witness_sound` obligations
  remain unproved targets for 1.2.2 and 1.2.3.
- Hosted runs on `c38370e`, observed 2026-10-21: CI (39647060364), Design
  contracts (39647060407), and Toolchain (39647060569) all completed with
  `success`.

`pr-babysitting` owns hosted CI observation, the two hosted CodeRabbit
assessments, and the merge.

## Interfaces and dependencies

There are no public Rust API changes; all new Rust code lives under
`tests/toolchain/`.

New committed files:

- `toolchain-pins.json`
- `scripts/provision_tools.py`
- `scripts/tests/conftest.py`
- `scripts/tests/test_provision_tools.py`
- `scripts/requirements-test.txt`
- `lean/lean-toolchain`
- `lean/lakefile.toml`
- `lean/lake-manifest.json`
- `lean/PoleanSmoke.lean`
- `tests/toolchain/{main,support,engine,json,regorus,bdd,smoke}.rs`
- `tests/features/regorus_baseline.feature`
- `tests/toolchain/snapshots/*.snap`
- `.github/workflows/toolchain.yml`
- `docs/adrs/adr-0003-pin-the-e0-toolchain-tuple.md`

`lean/lakefile.toml`:

```toml
name = "polean"
defaultTargets = ["PoleanSmoke"]

[leanOptions]
warningAsError = true

[[lean_lib]]
name = "PoleanSmoke"
```

`lean/PoleanSmoke.lean`. The Lean reviewer compiled this file on v4.35.0. The
`#guard_msgs` texts are settled by the first build.

```lean
/-!
# Toolchain smoke module

A fixed, repository-authored check that the pinned Lean toolchain elaborates
and kernel-reduces a small finite Boolean enumeration with `decide`. It states
nothing about Polean policies.
-/
namespace PoleanSmoke

/-- Every pair of Booleans, listed explicitly. -/
def boolPairs : List (Bool × Bool) :=
  [(false, false), (false, true), (true, false), (true, true)]

/-- The enumeration covers every pair. -/
theorem boolPairs_complete : ∀ a b : Bool, (a, b) ∈ boolPairs := by
  decide

/-- info: 'PoleanSmoke.boolPairs_complete' depends on axioms: [propext] -/
#guard_msgs in
#print axioms boolPairs_complete

/-- A finite check over the enumeration, settled by kernel reduction. -/
theorem and_comm_on_boolPairs :
    boolPairs.all (fun p => (p.1 && p.2) == (p.2 && p.1)) = true := by
  decide

/-- info: 'PoleanSmoke.and_comm_on_boolPairs' does not depend on any axioms -/
#guard_msgs in
#print axioms and_comm_on_boolPairs

end PoleanSmoke
```

`tests/features/regorus_baseline.feature`:

```gherkin
Feature: Pinned Regorus decides the baseline source

  Background:
    Given the baseline tenant-write source loaded into Regorus with Rego v1 and strict built-in errors

  Scenario: An owner may write an unlocked resource in their tenant
    Given the owner-unlocked write request
    When Regorus evaluates data.authz.allow
    Then the decision is exactly true

  Scenario: A cross-tenant administrator write is denied
    Given the cross-tenant administrator write request
    When Regorus evaluates data.authz.allow
    Then the decision is exactly false

  Scenario: An administrator may write a locked resource in their tenant
    Given the same-tenant administrator locked write request
    When Regorus evaluates data.authz.allow
    Then the decision is exactly true

  Scenario: An owner read is denied
    Given the owner read request
    When Regorus evaluates data.authz.allow
    Then the decision is exactly false
```

## Revision notes

- Revision 1 (2026-10-17): initial draft from reconnaissance and upstream
  research.
- Revision 2 (2026-10-17): incorporated the expert-panel review.
  - Removed:
    - the test-only `DecisionError` classifier, which is 2.3.2's work; exact
      shape assertions replace it;
    - line-parsing of the Cargo manifests; behavioural feature and version
      checks replace it;
    - Cyclopts and the README edit.
  - Fixed:
    - a vacuous Lake build: `defaultTargets`, an explicit target, an `.olean`
      check, and `#guard_msgs` axiom pins;
    - the Lean rationale and the disk estimate;
    - the absolute-path containment test, now a `root` check;
    - provisioning concurrency: a lock file, temporary siblings, and size
      pins;
    - the multi-binary `dead_code` failure, now one test binary.
  - Reworded the A4 mapping, the cross-tenant claim, and "conformance" in
    milestone checks.
  - Added a path-filtered `toolchain` CI workflow and OPA provenance through
    `gh release verify-asset`.
  - Recorded the reviewers' Regorus and OPA observations as discoveries.
  - Effect on the remaining work: fewer moving parts, the same success
    criteria.
