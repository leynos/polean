# ADR 0003: pin the E0 toolchain tuple

- **Status:** accepted.
- **Date:** 2026-10-21.
- **Decision owners:** project sponsor and maintainers.

## Context

Experiment E0 needs a working tuple of Rust, Regorus, Lean, and OPA before any
policy evidence can be produced. Technical design §11 left the tuple open,
ADR-001 deferred the Rust pin to E0, and roadmap 1.2.1 owns the decision.

Four claims must stay separate (ADR-002): admission, a model theorem,
source-to-model translation, and runtime correspondence. Nothing in this record
upgrades one into another. A tuple that runs is a prerequisite for observing
those claims, not evidence for any of them.

The pins, the provisioning script, and the smoke checks were delivered by the
[1.2.1 ExecPlan](../execplans/1-2-1-pin-a-tested-compatibility-tuple.md). This
record states what was chosen, why, what each artefact's provenance is, and
what the choice leaves open.

## Decision

Pin and smoke-test this tuple on `x86_64-unknown-linux-gnu`:

| Component    | Identity                                                  |
| ------------ | --------------------------------------------------------- |
| Rust         | `nightly-2026-08-27`; `rustc 1.100.0-nightly (bff8e12ff)` |
| Packages     | the committed `Cargo.lock`, 226 packages                  |
| Regorus      | `=0.13.0`; features `["ast", "std"]`                      |
| Lean         | `leanprover/lean4:v4.35.0`; Lake `5.0.0-src+eb6cafb`      |
| Lean archive | `lean-4.35.0-linux.tar.zst`, 594,481,375 bytes            |
| OPA          | `opa_linux_amd64_static` v1.21.2, 64,181,524 bytes        |

The full digests are in `toolchain-pins.json`, which the provisioning script
and the tests read, together with the archive's extraction root
(`lean-4.35.0-linux`). Recording the digests a second time here would create a
second authority.

### Regorus features

The request is `["ast", "std"]`. `ast` supplies `get_ast_as_json`, and `std`
supplies the engine's standard-library surface. Neither `full-opa` nor `arc` is
requested, because the profile admits no built-in calls (technical design §4.1)
and the orchestrator has no use for the runtime virtual machine. That
restriction is why the disabled-group observation is a meaningful check rather
than a formality: under this tuple a policy calling `regex.match` still parses,
and fails only when it evaluates.

Cargo resolves `["ast", "msvc_spectre_libs", "rand", "std"]`. It adds `rand` and
`msvc_spectre_libs` because `std` names both, and it does not report
`full-opa`. `cargo tree -e features -i regorus` records the whole set in the
ExecPlan. The disabled-feature negative check in `tests/toolchain/regorus.rs`
fails if a later change re-enables a built-in group, which `Cargo.lock` alone
cannot show.

### Dependency delta from 0.12.0 to 0.13.0

Both versions were resolved with the same feature request in throwaway
manifests, offline against the shared Cargo cache, so the delta is attributable
to Regorus alone. The 0.12.0 lock holds 53 packages and the 0.13.0 lock 58.

Added: `msvc_spectre_libs_build` 0.1.1, `ohno` 0.5.2, `ohno_macros` 0.5.2,
`ohno_macros_impl` 0.5.2, and `typeid` 1.0.3.

Raised: `msvc_spectre_libs` from 0.1.3 to 0.2.1, and the six Verus crates
(`vstd`, `verus_builtin`, `verus_builtin_macros`, `verus_prettyplease`,
`verus_state_machines_macros`, `verus_syn`) from the 2026-08-23 line to
`0.0.0-2026-09-06-0133`.

`cc` 1.9.0, `find-msvc-tools` 0.1.15, and `shlex` 2.0.1 appear in both
lockfiles under the highest-compatible resolution, so they are not part of this
delta, but their reach changed. In 0.12.0, `cc` was a
`cfg(all(target_os = "windows", target_env = "msvc"))` build-dependency of
`msvc_spectre_libs` and did not build on Linux. In 0.13.0, `msvc_spectre_libs`
depends unconditionally on `msvc_spectre_libs_build`, which depends
unconditionally on `cc` and `ohno`; all eight crates therefore compile on every
target. The build log for the 0.13.0 test run shows `cc`, `find-msvc-tools`,
`shlex`, `msvc_spectre_libs_build`, `msvc_spectre_libs`, `ohno_macros_impl`,
`ohno_macros`, `ohno`, and `typeid` compiling on Linux. The cost is build time,
not behaviour: `msvc_spectre_libs_build::plan` returns an empty plan unless the
target is Windows MSVC (`plan.rs` in 0.1.1), so no linking happens on Linux.

The graph stays small because the orchestration is a scaffold. When 2.1.1 moves
Regorus into `[dependencies]`, feature unification can re-enable groups
silently; the behavioural negative check is the guard.

### Provenance of each digest

**OPA.** The v1.21.2 GitHub release reports `"immutable": true`, and the
attestations API returns an attestation for
`sha256:830ab11f61ffec760b5e19db0d7ad02bf037e13f5ef4d665faeaf17632177527`.
`gh release verify-asset` resolved the tag to
`sha1:3f7474e6b3c631821d8aac25ca444f7e75a56483`, loaded the attestation, and
succeeded. The OPA pin is therefore attested and immutable.

**Lean.** The v4.35.0 GitHub release reports `"immutable": false`, and the
attestations API returns 404 both for the release and for the archive digest.
The archive digest matches the `digest` field GitHub reports for the asset, so
it is **trust on first use**: it records what GitHub served on 2026-10-21. An
upstream re-upload fails closed against the committed digest, and nothing
establishes that upstream built the archive from tag `eb6cafb`.

**Lean package metadata.** `lean/lake-manifest.json` is generated by the pinned
Lake and committed. `lean/lean-toolchain` names the exact release, so a
contributor's elan, if any, selects the same toolchain the gate does.

### Why Lean v4.35.0, and why v4.34.1 is the floor

v4.35.0, released 2026-10-12, was the newest stable release at pin time.
v4.36.0-rc1 exists and is a prerelease, so it is not a candidate for a gate.

The floor is v4.34.1. Three runtime fixes that could otherwise reach undefined
behaviour in the kernel, and in the worst case a proof of `False`, landed in
v4.34.0 and were backported to v4.34.1: #15241 (deletion cascades releasing
frozen objects), #15288 (reference sharing between threads), and #15289
(maximal sharing panicking past `INT_MAX` references). Earlier kernel-forgery
fixes are in v4.33.1 and later. v4.35.0 contains all of them.

v4.35.0 also removes `Lean.reduceBool`, `Lean.reduceNat`, `Lean.ofReduceBool`,
`Lean.ofReduceNat`, and the `Lean.trustCompiler` axiom (#14953). Because
`Lean.trustCompiler` was referenced from the definitions of the two `reduce`
functions, it looked used in every environment that imported them, which made
whole-environment axiom checks less meaningful. Its removal is a prerequisite
for the audit work in 1.2.3, not a claim that the audit is done.

### Keeping the Rust nightly

ADR-001 required the project to revisit the Rust pin alongside the tuple. It is
kept, and the consequence is now resolved rather than deferred: the tuple was
tested on `nightly-2026-08-27`, and the pin remains nightly because
`.rustfmt.toml` sets `unstable_features = true` and uses options such as
`imports_granularity`, `group_imports`, and `format_strings`. Stable rustfmt
ignores those options with a warning, so `make check-fmt` would stop enforcing
the committed formatting. No `#![feature]` attribute occurs in `src/` or
`tests/`, so nothing else depends on the nightly channel; moving to stable is a
formatting-policy decision, not a code change, and is out of scope here.

### Removing elan from the gate

The gate never invokes elan. It runs the binary under
`.polean/tools/lean/<sha256>/bin/`, located from the pin file, after the
archive's digest has been checked. Elan's download path cannot substitute:
`DownloadCfg::download_and_check` (`src/elan-dist/src/download.rs:30-36` at
elan commit `9605b24b08cf68b78dce0014b6fa01be1f8e2cee`) parses the URL, creates
a temporary file, and streams the download; the file contains no digest
computation or comparison. Contributors may keep elan for editor support,
because `lean/lean-toolchain` selects the same version, but its result is not
trusted as a gate input.

### Inputs for 1.2.3

The declaration-type and transitive-axiom audit needs these observations,
recorded here so they are not rediscovered:

1. `native_decide` asserts per-use axioms named
   `*._native.native_decide.ax_*`. With `Lean.ofReduceBool` and
   `Lean.trustCompiler` gone there is no single trust axiom to match; an audit
   must reject the per-use names, or require that they never occur.
2. `lake check` and `lake comparator` ship with v4.35.0 (#14990, #15145). They
   build and export the default targets, replay the export through the kernel,
   and fail on any axiom outside the standard set (`propext`, `Quot.sound`,
   `Classical.choice`). `--paranoid` additionally runs the bundled `lean4lean`,
   `nanoda`, `con-leche`, and `con-ron` checkers.
3. `lake check` needs `bwrap` to sandbox the code it checks, and reports that
   it is missing when bubblewrap is absent. `bwrap` is absent on the current
   development host.
4. `leanchecker` needs `lean` on `PATH`: under `env -i` it fails with
   `could not execute external process 'lean'`. A clean-environment audit must
   supply the digest-named `bin` directory, or the flag that names the checker
   explicitly.

None of this is implemented. 1.2.3 owns it.

### Licences, as inputs to 5.2.2

5.2.2 owns the dependency-distribution decision. This is the input:

- **Rust toolchain:** dual MIT / Apache-2.0.
- **Regorus 0.13.0:** `MIT AND Apache-2.0 AND BSD-3-Clause`. Its resolved
  graph adds MIT or MIT/Apache-2.0 crates, plus the Verus crates (`vstd`,
  `verus_builtin`, and `verus_state_machines_macros` are MIT;
  `verus_prettyplease` and `verus_syn` are MIT OR Apache-2.0).
- **OPA 1.21.2:** Apache-2.0.
- **Lean 4.35.0 release bundle:** Lean is Apache-2.0, but the archive is not
  single-licence. Its `LICENSES` file records bundled third-party components,
  including LLVM under Apache-2.0 with LLVM exceptions and CaDiCaL under MIT.
- **Polean itself:** ISC, settled by ADR-001.

## Consequences

A contributor with network access runs `make tools` once and then
`make toolchain-smoke`; the second command reuses the verified archives without
the network. `make test` keeps the Regorus observations on every host, and the
path-filtered `toolchain` workflow re-runs the provisioned checks when a pin,
the script, the Lean package, or the smoke tests change. Dependabot bumps are
therefore re-tested rather than assumed.

The tuple costs about 3.7 GiB per worktree and one conditional CI job. It does
not make Lean or OPA available to `make test`, and it does not widen what a
passing run proves. The smoke module `PoleanSmoke` is repository-authored
scaffolding; 2.2.4 never imports it, and no theorem about Polean policies
exists yet.

Three open items stay visible:

1. The Lean digest is trust on first use until upstream attests its releases.
2. `lake check` is unusable on hosts without `bwrap`, which constrains the
   1.2.3 experiment and the eventual verifier environment.
3. Regorus's AST remains an adapter input, not a stable proof ABI. A Regorus
   upgrade is a tuple change and needs the same evidence.

## Alternatives considered

**Regorus 0.12.0.** It has five fewer crates, no unconditional C compiler build
chain, and produced a byte-identical AST snapshot under both versions. It is
rejected because 0.13.0 is the current release, so Dependabot would propose it
immediately, and because the two breaking changes (a 32-component path limit
and canonical rule paths) do not touch `package authz`. The owner could prefer
the smaller graph; the plan recorded that choice, and no such preference was
expressed.

**Lean v4.34.1.** It contains the kernel fixes and would be cheaper to support.
It is rejected because it lacks `lake check` and the removal of
`Lean.trustCompiler`, both of which 1.2.3 would otherwise have to build itself.

**Lean v4.36.0-rc1.** A prerelease is not a gate input.

**The upstream Lean release manifest with elan.** Elan downloads without a
digest, so the gate would have to trust TLS and a mutable release; that is the
risk this record removes.

**Stable Rust.** Nothing in the code needs nightly, but the formatting policy
does. Changing it is a separate decision.

## Acceptance

2026-10-21: recorded during implementation of the
[1.2.1 ExecPlan](../execplans/1-2-1-pin-a-tested-compatibility-tuple.md), which
`leynos` approved on 2026-10-17. The plan named Lean v4.35.0 and OPA v1.21.2,
and defaulted Regorus to 0.13.0 unless the owner chose otherwise at approval;
no such choice was recorded, so approving the plan accepted this tuple. Polean
is founder-led, and `leynos` holds the sponsor and technical-reviewer
responsibilities, so one acceptance covers both. Correct the record if the
choice no longer holds.

## References

Inspected on 2026-10-21.

- **A3-R1.**
  [Regorus 0.13.0 engine API](https://docs.rs/regorus/0.13.0/regorus/struct.Engine.html):
  `add_policy`, `get_ast_as_json`, and the feature table.
- **A3-R2.**
  [Regorus 0.13.0 changelog](https://github.com/microsoft/regorus/blob/regorus-v0.13.0/CHANGELOG.md):
  the breaking path changes and the 0.12.0 baseline.
- **A3-R3.**
  [Lean 4.35.0 release notes](https://lean-lang.org/doc/reference/latest/releases/v4.35.0/):
  `lake check`, `vcgen`, the sandbox requirement, and #14953.
- **A3-R4.**
  [Lean 4.34.1 release notes](https://lean-lang.org/doc/reference/latest/releases/v4.34.1/):
  the three backported runtime fixes.
- **A3-R5.**
  [Lean v4.35.0 release](https://github.com/leanprover/lean4/releases/tag/v4.35.0):
  the archive, its size, and its commit `eb6cafb`.
- **A3-R6.**
  [OPA v1.21.2 release](https://github.com/open-policy-agent/opa/releases/tag/v1.21.2):
  the immutable, attested static binary.
- **A3-R7.**
  [elan `download.rs` at 9605b24](https://github.com/leanprover/elan/blob/9605b24b08cf68b78dce0014b6fa01be1f8e2cee/src/elan-dist/src/download.rs):
  `download_and_check` without a digest check.
