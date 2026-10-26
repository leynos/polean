# Adopt OrthoConfig v0.10.0 for the planned Polean CLI

This ExecPlan is a living document. The sections `Constraints`,
`Tolerances (exception triggers)`, `Risks`, `Progress`,
`Surprises & discoveries`, `Decision log`, `Outcomes & retrospective`,
`Conformance basis`, and `Verification plan` must stay current.

Status: DRAFT

## Purpose / big picture

When Polean's planned command line is implemented, its application boundary
will load user-facing settings with the published `ortho_config` v0.10.0
release. A user will be able to configure the documented presentation defaults
without changing policy semantics, while `verify` will regenerate evidence
without discovering configuration. Configuration errors will map to status 5
and tool failures to status 6. The repository currently has no executable CLI,
so the exact implementation scope must respect roadmap item 2.1.4, which first
introduces `check`.

## Constraints

- Preserve the imported v0.10.0 user's guide and migration guide byte-for-byte.
- Do not implement a Polean command before its roadmap item reaches it. The
  current baseline has no binary target.
- Keep OrthoConfig types at the application/configuration boundary. Reusable
  domain APIs must receive validated settings and must not print or exit.
- Keep `--profile` reserved for the policy profile
  `polean.tenant-write.v0`; do not opt into OrthoConfig named configuration
  profiles under that flag.
- Use only APIs shipped in OrthoConfig v0.10.0. Keep default features enabled;
  this release requires its JSON-backed merge support and TOML support.
- A clean `verify` must not discover configuration or let environment values
  or files change regenerated evidence.
- Map configuration failures to status 5 and tool failures to status 6.
- Preserve the contract's result statuses and the distinction between
  admission, model proof, source translation, and runtime correspondence.
- Do not merge or publish a release. The requested delivery is a pushed branch
  and a draft pull request.

## Tolerances (exception triggers)

- If satisfying the issue requires starting the CLI before roadmap item 2.1.4,
  stop and ask the user to resolve the scope conflict.
- If integration requires putting OrthoConfig in a reusable domain crate,
  stop and present the boundary change before implementing it.
- If the configuration design allows files or environment values to
  influence `verify`, stop and revise the design before coding.
- If implementation changes more than 20 files or 1,000 net lines, stop and
  explain the scope growth.
- If the existing command contract must change beyond the explicitly required
  configuration error mapping, record the proposal and wait for acceptance.
- If an API needed for the integration is absent from the fetched v0.10.0
  commit, stop; do not substitute an unreleased API.

## Risks

- Risk: the issue asks for implementation now while also forbidding an early
  CLI, and the CLI milestone is still pending. Severity: high; likelihood:
  high. Mitigation: obtain a scope decision before production implementation;
  keep the current plan in DRAFT.
- Risk: `polean` is presently a library-only scaffold, so a dependency added to
  its package could leak into reusable code. Severity: high; likelihood:
  medium. Mitigation: keep configuration types in the future app boundary and
  decide package ownership before adding the runtime dependency.
- Risk: OrthoConfig's ordinary parser defaults outrank files and environment.
  Severity: medium; likelihood: medium. Mitigation: use `cli_default_as_absent`
  with the match-aware merge API for any configured field with a Clap default,
  then test the real v0.10.0 API.
- Risk: imported upstream prose may fail Polean's Markdown gates. Severity:
  medium; likelihood: low. Mitigation: do not edit imported bytes; report any
  gate conflict instead of silently changing the source text.
- Risk: CodeGraph semantic queries may be unavailable in this environment.
  Severity: low; likelihood: high. Mitigation: use targeted source and contract
  inspection, and record this as a fallback rather than claiming a semantic
  graph result.
- Risk: hosted CodeRabbit limits can delay review. Severity: medium;
  likelihood: low. Mitigation: run deterministic gates first and follow the
  requested cooldown procedure if the review service reports a rate limit.

## Progress

- [x] (2026-10-26 12:20Z) Read repository instructions and confirmed the
  worktree was clean before the import. The starting branch was
  `session/d082da36`; it was first renamed to `adopt-ortho-config-cli` as
  required, then to the requested branch
  `issue-14-adopt-ortho-config-v0-10-0-for-the-polean-cli`.
- [x] (2026-10-26 12:20Z) Set the Lody session title with the requested command
  and recorded session ID `d082da36-3396-4616-827a-d78143c79f56`.
- [x] (2026-10-26 12:20Z) Confirmed baseline commit
  `c0c60055c7264f662ce35cfb7a7c1fb518037423` and compiler
  `rustc 1.100.0-nightly (bff8e12ff 2026-08-26)`, host
  `x86_64-unknown-linux-gnu`, LLVM `23.1.0`. The repository pins
  `nightly-2026-08-27`.
- [x] (2026-10-26 12:20Z) Fetched `ortho-config` tag `v0.10.0` over SSH. Its
  tag object `996e5a1c9f22ace4f55bf5dcc196d2c0524262bf` peels to
  `38812c563e3953a490e3efc202609f37e21a0517`, matching the surveyed resolution.
- [x] (2026-10-26 12:20Z) Imported the two guides from the peeled immutable
  commit and byte-compared both imports against `git show` output. Their
  SHA-256 values are recorded in `docs/ortho-config-provenance.md`.
- [x] (2026-10-26 12:20Z) Recorded the baseline CLI inventory: Cargo metadata
  reports one library target and two test targets, no binary, no dependencies,
  `publish = false`, and no declared `rust-version`. The CLI contract is
  proposed, has no environment or configuration contract, and declares result
  statuses 0 and 2–8. No CLI-specific tests or executable callers exist in this
  checkout; local repository search found only the proposed documentation
  examples, not CI or Makefile invocations.
- [x] (2026-10-26 12:20Z) Checked GitHub issue #14 (open), found no PR for the
  requested head branch, and found no remote branch with that name. The local
  branch can track
  `origin/issue-14-adopt-ortho-config-v0-10-0-for-the-polean-cli` after the
  requested SSH push creates it.
- [x] (2026-10-26 12:20Z) Used Firecrawl to inspect the versioned crates.io
  release page. It confirmed the `0.10.0` release; the immutable release
  manifest supplies the authoritative `rust-version = "1.89.0"` and default
  feature set (`serde_json`, `toml`).
- [x] (2026-10-26 12:20Z) Reindexed CodeGraph successfully. Its entry-point and
  symbol-search calls did not return within several minutes and were cancelled;
  structural inventory below uses direct repository metadata and targeted
  searches instead.
- [x] (2026-10-26 12:25Z) Docs-only `make check-fmt` and `make markdownlint`
  passed sequentially after formatting the authored ExecPlan/provenance tables
  and wrapping one overlong trace-map line. The final run linted 25 Markdown
  files with zero errors. The scrutineer report records both full log paths
  under `/tmp`.
- [x] (2026-10-26 12:25Z) The first docs gate found table reflow in the new
  authored documents, and the next found one MD013 line-length issue. Both were
  corrected without touching either imported guide.
- [x] (2026-10-26 12:38Z) CodeRabbit reviewed the imported-document milestone
  and reported seven minor documentation findings, with no high or medium
  concerns. Corrected the authored-plan wording and worktree path; recorded
  upstream-only guide links and the scope of upstream example tests in the
  provenance note without editing imported text. The first follow-up format
  check found table reflow; after formatting only the two changed authored
  files, `make check-fmt` and `make markdownlint` passed sequentially (25
  Markdown files, zero lint errors). Review log:
  `/tmp/coderabbit-d082da36-3396-4616-827a-d78143c79f56-issue-14-adopt-ortho-config-v0-10-0-for-the-polean-cli.out`.
- [ ] (pending) Resolve the scope question and approve this ExecPlan before
  production implementation, as required by the ExecPlans workflow.
- [ ] Record the configuration decision in the technical design, ADR, roadmap,
  user guide, and developer guide.
- [ ] Implement only the approved integration scope and its regression tests.
- [ ] Run deterministic gates sequentially, request CodeRabbit review, clear
  actionable concerns, commit, push, and create the draft PR.

## Surprises & discoveries

- The supplied context describes the current CLI statuses as 0 and 2–6, but
  `contracts/cli.txt` also specifies invalid-evidence status 7 and
  conformance-mismatch status 8. Preserve the repository contract's full set.
- There is no CLI target to adopt the dependency into yet. The first command
  milestone is roadmap 2.1.4 (`check`); roadmap 4.2.2 later completes `prove`
  and `verify`. Both are unchecked at this baseline.
- `Cargo.toml` has no dependency or `rust-version`; the package is a
  non-published library scaffold. `cargo metadata --no-deps` confirms there is
  no binary target.
- The `ortho_config` v0.10.0 manifest declares Rust 1.89.0 and default features
  `serde_json` and `toml`. Firecrawl's crates.io scrape did not expose MSRV, so
  the release manifest, not that scrape, is the evidence for the floor.
- The migration guide's section 6 identifies v0.10.0 features including
  injected sources, inferred Clap defaults, and the Cargo external-subcommand
  helper. The user's guide marks per-field localized defaults and later
  agent-native linting as unavailable/planned; this design will not rely on
  them.
- The command contract names `polean`, not `cargo polean`, so no Cargo external
  subcommand wrapper is currently part of the proposed CLI shape.
- The imported guides byte-compare exactly to their immutable source paths.

## Decision log

- Decision: use `docs/ortho-config-users-guide.md` and
  `docs/ortho-config-v0-10-0-migration-guide.md`, with provenance in a separate
  file. Rationale: this follows repository naming and preserves the exact
  release-specific source. Date: 2026-10-26.
- Decision: do not add OrthoConfig to `Cargo.toml` during reconnaissance.
  Rationale: there is no application binary yet, and the issue forbids starting
  the CLI early solely to adopt configuration. The appropriate code milestone
  depends on the user's scope resolution and approved plan. Date: 2026-10-26.
- Decision pending: whether to defer executable configuration code to roadmap
  2.1.4, add a testable configuration module without a binary now, or promote
  the CLI milestone. The user has been asked to resolve this because the
  alternatives materially change the implementation and dependency boundary.

## Outcomes & retrospective

Preflight and immutable documentation import are complete, and the docs-only
formatting and Markdown gates passed. The task is not complete: no integration
decision has been approved, no runtime code or configuration dependency has
been added, and no CodeRabbit review, push, or draft PR has run. This
distinction prevents a documentation import from being reported as a working
CLI adoption.

## Context and orientation

The baseline checkout is commit `c0c60055c7264f662ce35cfb7a7c1fb518037423` on
`origin/main`. `Cargo.toml` defines one Rust 2024 package named `polean`, sets
`publish = false`, and currently produces only `src/lib.rs`; its two test
targets cover the scaffold and documentation examples. `rust-toolchain.toml`
pins `nightly-2026-08-27`. The test inventory contains no CLI process or
configuration tests.

The command surface is a proposal in `contracts/cli.txt` and
`docs/technical-design.md` §8: `check`, `prove`, `verify`, and `export-rego`.
`--profile` identifies a policy profile. The contract distinguishes model
refutation (2), unknown (3), unsupported syntax (4), invalid input or
configuration (5), tool failure (6), invalid evidence (7), and conformance
mismatch (8). Roadmap 2.1.4 is the first command (`check`); 4.2.2 owns the
complete `prove` and `verify` surface. Existing `Makefile` and CI workflows do
not invoke the proposed commands. A scan of the local repository clones found
no external executable caller.

The imported OrthoConfig docs describe only v0.10.0. The release tag and its
peeled commit are recorded in `docs/ortho-config-provenance.md`. OrthoConfig's
public release manifest sets `rust-version = "1.89.0"`, with default TOML and
`serde_json` features. No configuration library is currently named in the
Polean roadmap.

## Conformance basis

- Terms of reference draft v0.1: G5 (bind evidence to source, requirements,
  domain, semantic version, and relevant runtime configuration), G6 (reviewer
  replay), SC4–SC6, and non-goals N1–N6.
- Technical design draft v0.1: §2 trust boundaries, §6 ownership, §8 CLI and
  outcomes, and §10 verification obligations.
- ADR 0002: admission, model proof, translation, and runtime correspondence
  must remain distinct.
- `contracts/cli.txt`: proposed command/flag surface and exit statuses 0, 2–8.
- Roadmap 2.1.4: first `check` command; 4.2.2: public `prove`/`verify`
  contract; 4.2.3: hostile-evidence and CLI interaction tests.

Trace links:

```plaintext
ToR G5, G6, SC4-SC6 -> TDD §8 -> CLI contract -> Roadmap 2.1.4 / 4.2.2 ->
  configuration-source tests and hermetic verify test
ADR 0002 assurance boundary -> TDD §2 and §8 -> Roadmap 4.2.2 / 4.2.3 -> status and output-boundary tests
Baseline gap: no executable CLI -> Roadmap 2.1.4 -> first application configuration loader -> binary-level behavior tests
```

## Verification plan

No application invariant or theorem is introduced during the documentation
import. The imported-document obligation is exact byte preservation: compare
each destination's bytes with
`git show 38812c563e3953a490e3efc202609f37e21a0517:<upstream-path>` and compare
the recorded SHA-256. A mismatch fails this obligation. A witness for each
imported file is its full byte array and checksum.

For any approved runtime integration, verify the following obligations against
the actual OrthoConfig 0.10.0 interface:

- `CFG-PRECEDENCE`: for each configuration-enabled command, defaults are
  overridden by TOML, then injected environment, then explicit CLI values. Use
  finite table tests with distinct `text`/`json` values at each tier, `MapEnv`,
  explicit file sources, and the `_with_sources` APIs. The test is non-vacuous
  because each tier supplies a value different from all lower tiers; removing a
  merge tier must fail the expected assertion.
- `CFG-PROFILE-SEPARATION`: configuration cannot change `--profile` from its
  policy-profile meaning. A test supplies a configuration key/profile table and
  proves it is rejected or has no effect, according to the approved unknown-key
  rule. A valid policy-profile flag is the positive witness.
- `CFG-VERIFY-HERMETIC`: changing config files and configuration environment
  variables cannot change the model regenerated by `verify`. Test the actual
  verifier boundary in an isolated child process with two conflicting config
  sources and compare regenerated output digests. The negative control must
  demonstrate that a deliberately config-aware verifier would be detected.
- `CFG-FAILURE-CLASS`: malformed, unreadable, invalid-value, and explicitly
  missing config paths map to 5; a missing required external tool maps to 6;
  absence of optional discovered files leaves defaults in force. Test each
  boundary separately and assert status, stdout, and stderr.
- `CLI-DISPLAY-AND-PARSE`: help and version are successful display requests;
  invalid command-line syntax retains Clap's own status and stream; config
  failures are reported at the application boundary. Child-process tests
  exercise the real binary without mutating the parent process environment.
- `CLI-NO-CONFIG-DEFAULTS`: the first implemented command retains its approved
  no-config flags, defaults, output, status, and streams. Since no CLI exists
  at baseline, the red test must be based on the proposed contract and the
  explicit default chosen in the approved design; there is no legacy CLI
  snapshot to preserve.
- `CLI-BOUNDARY`: the application maps OrthoConfig errors to public outcomes;
  domain/library APIs receive validated Polean settings and never print or
  terminate the process. Verify through a focused boundary test and linted
  module structure.

No formal proof is appropriate: OrthoConfig loading is infrastructure
composition, and the required claims are observable command behavior. The
third-party library's documented interface is an axiom; repository-owned source
selection, precedence, error mapping, and hermeticity are tested against the
real v0.10.0 release. The full repository gates are run by the exclusive
`scrutineer` gate runner, sequentially, before each requested CodeRabbit review.

## Plan of work

### EP-M1: Import the immutable v0.10.0 guides and record the baseline

The coherent end state is two byte-exact imported guides, separate provenance,
an updated documentation index, and a recorded CLI baseline. This milestone has
passed its byte comparisons and docs-only gates; its requested CodeRabbit
review remains pending.

### EP-M2: Decide and document the configuration boundary

After the user resolves the scope question and approves the plan, record the
decision in `docs/technical-design.md` §8 and a new sequential ADR. Update the
first applicable roadmap command item (2.1.4) and the later public command
items (4.2.2 and 4.2.3), as well as `docs/users-guide.md` and
`docs/developers-guide.md`. The design must state configurable versus CLI-only
settings, TOML discovery and file names, environment prefix and selectors,
precedence, per-command namespaces, status mapping, localization, profile
behavior, and whether Cargo's wrapper is needed. End-state evidence is
consistent cross-links and no claim that a CLI exists before its code.

### EP-M3: Implement the approved configuration spine

If the approved scope keeps the CLI deferred, do not add a binary or runtime
dependency; encode the implementation and tests as explicit acceptance criteria
of the planned CLI milestone, and do not mark this issue's runtime adoption
complete. If the user approves an isolated configuration module or promoted CLI
work, add red tests first, implement only the selected boundary, and keep
reusable domain types independent of OrthoConfig. Tests must cover every
precedence tier, source injection, unhappy paths, error mapping, and the
`verify` hermetic boundary.

### EP-M4: Validate and deliver

Run formatting, Markdown, type, lint, test, design, and boundary gates
sequentially via `scrutineer`; do not ask CodeRabbit to diagnose deterministic
failures. After each coherent implementation plateau, run
`coderabbit review --agent` and resolve actionable concerns before continuing.
Commit each coherent change with an imperative subject and a body explaining
the reason and assurance impact. Verify SSH identity as `leynos`, push the
requested branch, refresh its remote-tracking ref, set its upstream, then
create a draft PR titled with `(#14)`, with `Closes #14` and a terminal
`## References` section linking the Lody session. Do not merge.

## Milestones and plateaus

- **EP-M1 — immutable documentation imported.** Requirements: source
  provenance and release boundary. Acceptance: byte comparisons and hashes
  match, docs index links are valid, Markdown gates pass. Conformance: no
  imported byte changed; the decision is not yet presented as runtime code; no
  dependency or public API changed. Recovery: revert only this documentation
  commit. Remaining gap: configuration design and code. Compatibility decision:
  none.
- **EP-M2 — approved design recorded.** Requirements: CLI contract and
  hermetic replay. Acceptance: technical design, ADR, roadmap, and guides state
  one decision with consistent trace links. Conformance: no source or status
  contract drift outside issue scope; no unapproved API or dependency.
  Recovery: revise the unimplemented design before code. Remaining gap:
  configuration runtime. Compatibility decision: none.
- **EP-M3 — approved configuration path implemented.** Requirements:
  `CFG-PRECEDENCE`, `CFG-PROFILE-SEPARATION`, `CFG-VERIFY-HERMETIC`,
  `CFG-FAILURE-CLASS`, and `CLI-BOUNDARY`. Acceptance: focused tests fail for
  seeded defects and pass for the candidate. Conformance: config stays in the
  application edge; verify is config-free; all result statuses remain as
  contracted. Recovery: revert the implementation commit independently of
  M1/M2. Remaining gap: hosted CI/review. Compatibility decision: none.
- **EP-M4 — delivery candidate validated.** Requirements: all canonical gates,
  reviewed current head, branch tracking, and draft PR. Acceptance: gate and
  review evidence names the exact pushed commit; draft PR links issue 14 and
  the Lody session. Recovery: no merge; amend/review a new candidate. Remaining
  gap: maintainer merge. Compatibility decision: none.

## Concrete steps

Run from the repository root in the candidate worktree.

The source and baseline read-only checks are:

```bash
git rev-parse HEAD
rustc -Vv
cargo metadata --no-deps --format-version 1
sha256sum docs/ortho-config-users-guide.md docs/ortho-config-v0-10-0-migration-guide.md
```

For a byte comparison, run a Python check that reads each destination and
compares it with
`/usr/bin/git show 38812c563e3953a490e3efc202609f37e21a0517:<path>`. Expected
evidence is `BYTE_COMPARE ...: PASS` for both imports.

For docs-only changes, run `make check-fmt` then `make markdownlint` through
`scrutineer`, sequentially. For runtime changes, run these gates sequentially:

```bash
make check-fmt
make markdownlint
make typecheck
make lint
make test
make design-check
make design-properties
make design-boundary
```

Each command must exit zero before the next gate begins. Capture each gate's
output under `/tmp` and retain the exact candidate SHA in the report.
CodeRabbit review starts only after deterministic gates pass.

## Validation and acceptance

The imported-document milestone's byte comparisons and docs-only Markdown gates
are green; its requested CodeRabbit review remains pending. The configuration
design milestone is accepted only after the user approves the selected scope
and the design artifacts agree. Runtime adoption is accepted only when the
selected implementation exists and the tests in the Verification plan pass
against `ortho_config` v0.10.0. A design-only deferral must be reported as
deferred, never as a completed runtime adoption.

Pull-request acceptance requires the exact pushed head to have all local
deterministic gates green, CodeRabbit concerns cleared or explicitly resolved,
issue `#14` named in the PR title, `Closes #14` in the summary, and the Lody
session link in the final `## References` section. The PR remains draft and
unmerged.
