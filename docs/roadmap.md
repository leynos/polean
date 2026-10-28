# Polean roadmap

This roadmap translates the [terms of reference](terms-of-reference.md) and
[technical design](technical-design.md) into implementation work for the local,
source-first `polean.tenant-write.v0` workflow. The goal is to let a policy
author and reviewer check independently stated access requirements against an
exact source revision, obtain a checked counterexample, and replay the
evidence. Polean currently contains a scaffold and design fixtures; every task
below is pending implementation or a new acceptance decision.

The GIST structure maps that goal to testable phase ideas, steps that validate
or falsify each idea, and review-sized tasks with dependencies and observable
success criteria. It promises no dates. Phases 2–4 follow design experiments
E0–E2; phase 5 supplies the user-review gate. The [bootstrap ADR][adr-0001],
[vocabulary](context.md), and [validation record](../validation/README.md)
establish the starting boundary.

Each delivery task includes relevant regression tests, documentation, and
machine-readable contract updates. Keep admission, model proof, trusted source
translation, and tested runtime agreement separate throughout. A passing model
proof neither authenticates request attributes nor certifies application
integration. Failure of a feasibility or acceptance gate requires a recorded
revise-or-stop decision; it does not authorize wider syntax or weaker assurance.

## 1. Establish a feasible closed-profile implementation boundary

Idea: if the selected tools expose complete admission and auditable proof
roots, a small pinned implementation can support the first workflow without a
new policy engine or a general-purpose proving platform.

This phase resolves uncertainties before production feature work. Preserve the
existing single Rust package, reserved Lean boundary, and independent design
validation rather than rebuilding the scaffold.

### 1.1. Ratify the assurance and release gates

Can the intended reviewer accept the designed trust boundary? The answer sets
the release claim and identifies decisions that must block later acceptance.
See technical-design.md §§1–2 and 13; terms-of-reference.md §§6–9.

- [x] 1.1.1. Record the first-profile assurance decision in an ADR.
  - See technical-design.md §§1–3 and 9; terms-of-reference.md §§1–5 and 8.2,
    A2–A3;
    context.md.
  - Success: the sponsor and reviewer accept the exact model-proof,
    trusted-frontend, and tested-agreement wording, with attribute provenance
    explicitly outside the theorem, or record a blocking rejection.
- [x] 1.1.2. Allocate owners and closure evidence for the open gates.
  - Requires 1.1.1.
  - See technical-design.md §13; terms-of-reference.md §§8–10, OQ1–OQ7; and
    [ADR 0001][adr-0001].
  - Success: the technical-design §13 prerequisites table names responsible
    roles for toolchain pins, admission, audit, sandbox, budgets,
    distribution, sponsor review, and unresolved provenance, and separates E0
    needs from public-release needs; the existing ISC licence is distinguished
    from the still-open dependency distribution decision.

### 1.2. Demonstrate that the selected tool boundaries are usable

Can a bounded spike enforce the closed grammar and inspect checked declarations
without executing submitted code? Its result either unlocks E0 or requires an
explicit design revision. See technical-design.md §§4–6 and 11–13.

- [x] 1.2.1. Pin a tested Rust, Regorus, Lean, and OPA compatibility tuple.
  - Requires 1.1.2.
  - See technical-design.md §§6 and 11;
    [ADR 0001][adr-0001] and [ADR 0003][adr-0003];
    repository-layout.md §Ownership boundaries.
  - Success: provisioned tools compile a fixed Lean smoke theorem, parse the
    baseline, and evaluate its source; lockfiles, Regorus features, and OPA
    digest are recorded. Regorus 0.13.0 was tested in process and OPA replay
    was observed; ADR-003 records the tuple and its provenance.
- [ ] 1.2.2. Produce a complete-module admission feasibility record.
  - Requires 1.2.1.
  - See technical-design.md §§4.1 and 6.1; ../contracts/profile.json.
  - Success: fixtures account for package, import, default, every rule and
    expression, metadata, and unknown AST variants. Missing API coverage has a
    demonstrated bounded check or is recorded as an E0 finding that blocks
    2.1.2; unreachable syntax is not ignored.
- [ ] 1.2.3. Demonstrate declaration-type and transitive-axiom auditing.
  - Requires 1.2.1.
  - See technical-design.md §§5 and 6.2.
  - Success: a pinned audit interface accepts an expected proposition and
    approved foundations, rejects `sorryAx`, native-computation trust axioms,
    and undeclared assumptions, and emits machine-readable results.
- [ ] 1.2.4. Record an enforceable Linux execution-boundary decision.
  - Requires 1.2.1.
  - See technical-design.md §§2 and 11; terms-of-reference.md §8.2, A4–A6.
  - Success: a bounded probe demonstrates offline execution, memory and output
    limits, process-group cancellation, and private storage on the intended
    platform. Unavailable enforcement blocks public acceptance.

## 2. Detect a tenant-boundary regression from source through one workflow

Idea: if the baseline and weakened administrator rule pass through one exact
frontend and checked finite model, Polean can explain a real cross-tenant
regression without requiring hand-written proof code per policy.

This is E0, a development demonstration, not a public v0 release. Its one-claim
workflow must not masquerade as successful public `prove` or `verify`, whose
contracts require all six claims and the remaining gates.

### 2.1. Admit source and requests without losing their meaning

Can the production ingress retain every policy distinction and reject data
outside the declared domain? The result supplies trustworthy inputs for the
first source-bound demonstration. See technical-design.md §§4–6.

- [ ] 2.1.1. Implement bounded source ingestion and closed policy IR types.
  - Requires 1.2.1 and 1.2.2.
  - See technical-design.md §§4.1, 5, and 7;
    ../contracts/policy.schema.json; repository-layout.md.
  - Success: UTF-8 and the 64 KiB source cap are enforced; canonical bytes use
    the specified key order and final LF, preserve rule/atom order, allow zero
    rules, and reject empty bodies, duplicate keys, and unknown fields.
- [ ] 2.1.2. Implement complete Regorus admission with diagnostic source maps.
  - Requires 2.1.1.
  - See technical-design.md §§4.1 and 6.1; ../contracts/profile.json.
  - Success: all six comparisons map by parsed identity; unsupported forms,
    metadata, reverse operands, and unknown nodes fail closed with byte spans.
    Diagnostic maps remain separate from the semantic IR.
- [ ] 2.1.3. Implement the strict typed request decoder.
  - Requires 2.1.1.
  - See technical-design.md §4.2; ../contracts/request.schema.json.
  - Success: the exact seven fields, scalar-sequence equality, and byte cap
    are enforced. Duplicate keys, extra fields, coercions, malformed UTF-8,
    nulls, and unpaired surrogates fail; cross-tenant requests remain valid.
- [ ] 2.1.4. Deliver `check` with admission-only text and JSON diagnostics.
  - Requires 2.1.2.
  - See technical-design.md §8; ../contracts/cli.txt; users-guide.md.
  - Success: the baseline is admitted, unsupported and invalid source have
    distinct exits, and neither output format claims an invariant proof. Build
    the application configuration edge on `ortho_config` v0.10.0 as specified
    in technical-design.md §8 and ADR 0004. Test `check` defaults, both TOML
    layers, injected environment, explicit CLI precedence, missing optional files,
    malformed and unreadable files, invalid values and unknown keys;
    configuration errors exit 5. Keep reusable libraries independent of OrthoConfig.

### 2.2. Produce checked tenant-isolation evidence from admitted data

Can finite checking establish a claim over all valid requests and construct a
real violating request? These tasks establish the E0 model obligations rather
than substituting Python enumeration for a universal proof. See
technical-design.md §§3, 5–6, and 10, V1–V5.

- [ ] 2.2.1. Implement Lean policy semantics and exact request abstraction.
  - Requires 2.1.1.
  - See technical-design.md §5 and §10, V1–V2.
  - Success: typed requests, all six atoms, conjunction/disjunction semantics,
    and `abstraction_preserves` compile and are kernel checked; `unlocked`
    is the complement of locked, never an independent sixth fact.
- [ ] 2.2.2. Implement fact coverage and concrete witness construction.
  - Requires 2.2.1 and 2.1.3.
  - See technical-design.md §5 and §10, V3.
  - Success: Lean checks that `allFacts` covers every five-Boolean value and
    `α(γ(f)) = f`; serialized witnesses satisfy the strict request boundary.
- [ ] 2.2.3. Implement the tenant-isolation checker and witness soundness.
  - Requires 2.2.2.
  - See technical-design.md §§3 and 5; §10, V4–V5.
  - Success: kernel-reduced checking plus soundness proves baseline tenant
    isolation; the cross-tenant mutation has a checked concrete refutation.
    Neither `native_decide` nor unchecked solver verdicts are accepted.
- [ ] 2.2.4. Implement fixed-data Lean jobs and worker acceptance.
  - Requires 2.2.3 and 1.2.3.
  - See technical-design.md §§2 and 6.2; repository-layout.md.
  - Success: only enum/list literals enter trusted templates; acceptance
    checks exact policy propositions, requested roots, and transitive axioms.
    Missing, contradictory, or malformed worker output is a tool failure.

### 2.3. Join proof, source replay, and diagnostics in the E0 demonstration

Does one invocation retain source identity from ingestion to observed witness
replay? Completion decides whether the architecture merits the six-claim slice.
See technical-design.md §§6, 8, 11–12, E0.

- [ ] 2.3.1. Implement the bounded synchronous job supervisor.
  - Requires 2.1.1.
  - See technical-design.md §§2 and 11.
  - Success: one supervised local proof job uses immutable snapshots, a
    cleared environment, explicit executables, and argument arrays on trusted
    repository inputs; one remaining deadline governs children, which are
    killed and reaped. Enforced isolation follows in 4.3.1.
- [ ] 2.3.2. Implement typed OPA and Regorus source observations.
  - Requires 2.3.1 and 2.1.3.
  - See technical-design.md §6.3 and §10, V5 and V10.
  - Success: each oracle consumes the same original module at
    `data.authz.allow`, with pinned options. False is preserved; undefined,
    errors, multiple/non-Boolean decisions, and malformed output remain
    distinct.
- [ ] 2.3.3. Wire the source-bound E0 development command and report.
  - Requires 2.1.2, 2.2.4, and 2.3.2.
  - See technical-design.md §§8 and 12, E0; terms-of-reference.md §7, SC2.
  - Success: one command proves baseline tenant isolation, checks the weakened
    rule's witness, and replays it in both source runtimes without editing proof
    scripts. The report records exact source identity and remaining trust.

## 3. Check the complete safety and positive-access contract

Idea: if the same finite model settles all six independent requirements and all
mutations, useful access and security boundaries can coexist in the narrow
profile without a stronger solver or a wider grammar.

This is E1. It extends the E0 workflow to reject deny-all as an incomplete
policy contract and to validate the full typed request boundary. Public release
still depends on replay and operational acceptance in later phases.

### 3.1. Extend the same workflow to every registered requirement

Do the independent claims characterize the intended authorization decision and
survive source-driven execution? The answer determines whether E1 satisfies
V1–V6 without changing the five-fact abstraction. See technical-design.md §§3,
5, 10, and 12, E1.

- [ ] 3.1.1. Register the complete Lean claim family and sound checker.
  - Requires 2.3.3.
  - See technical-design.md §§3 and 5; §10, V4–V6.
  - Success: all four safety and two availability definitions are independent
    of the frontend; checker and witness soundness cover every ID, and Lean
    checks that the six claims characterize the intended decision.
- [ ] 3.1.2. Enforce complete claim manifests and per-claim settlement.
  - Requires 3.1.1.
  - See technical-design.md §§3 and 8; ../contracts/claims.schema.json;
    ../contracts/result.schema.json.
  - Success: all six unique known IDs are mandatory; reordering normalizes
    deterministically, while omissions, duplicates, extra IDs, and submitted
    definitions fail. Partial results survive without an aggregate success.
- [ ] 3.1.3. Extend the development workflow to all baseline and mutation
      claims.
  - Requires 3.1.2.
  - See technical-design.md §10.1 and §12, E1;
    terms-of-reference.md §7, SC1–SC3; ../validation/README.md.
  - Success: the baseline proves all six claims; all six mutations fail the
    specified requirements, including both availability claims for deny-all,
    with checked witnesses and independent oracle replay over all 32 vectors.

### 3.2. Exercise input distinctions and conservative source round trips

Can real inputs and canonical export reuse the same model without changing
source or claim meaning? These checks inform reviewer replay while keeping
source translation explicitly trusted. See technical-design.md §§4.2, 7, and 10.

- [ ] 3.2.1. Implement canonical Rego rendering and `export-rego`.
  - Requires 3.1.3.
  - See technical-design.md §§7–8; ../contracts/cli.txt; §10, V9–V10.
  - Success: re-import yields identical ordered IR for generated valid policies;
    original and canonical source agree with the model in both oracles. Export
    preserves original bytes separately and never claims proved equivalence.
- [ ] 3.2.2. Deliver the E1 ingress and source-conformance E2E matrix.
  - Requires 3.2.1 and 2.1.3.
  - See technical-design.md §§4.2 and 10.1; developers-guide.md §Tests.
  - Success: empty, escaped, composed/decomposed Unicode, mismatched, and long
    valid strings exercise decoding through oracle replay; unsupported syntax
    in unreachable rules and malformed `"false"` values never pass. Independent
    sentinels and valid-profile shrinking avoid shared expected-output errors.

## 4. Let reviewers reconstruct evidence across an untrusted boundary

Idea: if verification reconstructs policy and proof jobs from explicit inputs,
a reviewer can detect stale or substituted evidence without trusting submitted
executable proof artefacts or the author's report.

This is E2. It adds binding, fresh reconstruction, and release-grade failure
semantics to the complete contract. Required oracles remain enabled; neither a
valid JSON schema nor a successful worker exit is sufficient for acceptance.

### 4.1. Bind evidence to the full claim and execution identity

Which identities must match before a prior report is relevant to submitted
source? The answer supplies the replay protocol and prevents identical IR from
hiding changed source bytes. See technical-design.md §§2 and 7–8.

- [ ] 4.1.1. Version the evidence manifest and binding contract.
  - Requires 3.2.2.
  - See technical-design.md §7 and §10, V7; terms-of-reference.md §7, SC4.
  - Success: a documented schema binds source, canonical IR, normalized claims,
    profile, request schema, proof package, axiom policy, orchestrator build,
    lockfiles/features, executable identities, entry point, and runtime options.
- [ ] 4.1.2. Implement bounded evidence packaging and safe storage.
  - Requires 4.1.1 and 2.3.1.
  - See technical-design.md §§7 and 11.
  - Success: the proposed bundle retains source, model, claims, witnesses,
    observations, and result; incomplete publication cannot look complete.
    Symlinks, traversal, out-of-directory artefacts, and oversized data fail.
- [ ] 4.1.3. Implement fresh `verify` against explicit source and claims.
  - Requires 4.1.2 and 3.1.2.
  - See technical-design.md §§6.2–8; §10, V7; ../contracts/cli.txt.
  - Success: verification snapshots explicit inputs, reconstructs admission,
    compares bindings, regenerates fixed jobs, rechecks roots/axioms, and reruns
    required observations. Submitted Lean, Lake, `.olean`, native libraries,
    and plug-ins are never executed or imported.

### 4.2. Make settled verdicts depend on every required gate

Can the public command distinguish refutation from incomplete work and broken
execution? The outcome informs release readiness and report-consumer trust. See
technical-design.md §8 and §10, V8.

- [ ] 4.2.1. Implement semantic result validation and aggregate precedence.
  - Requires 4.1.3.
  - See technical-design.md §8; ../contracts/result.schema.json;
    context.md; users-guide.md §Intended outcomes.
  - Success: exact claim coverage, non-null identities, approved theorem roots,
    contained witnesses, and assurance fields determine acceptance; mismatch
    overrides refutation without erasing it, and operational failures cannot
    be hidden by incomplete logical results or a report-supplied exit value.
- [ ] 4.2.2. Expose the complete public `prove` and `verify` command contract.
  - Requires 4.2.1.
  - See technical-design.md §8; ../contracts/cli.txt; users-guide.md.
  - Success: exits 0 and 2–8 match the contract in text and JSON; stdout has
    one bounded result, stderr has escaped diagnostics, and witnesses require
    explicit export. Exit zero requires all six proved claims and every gate,
    with `trusted_frontend` and `tested_agreement` still visible. Apply the
    §8 OrthoConfig presentation settings to `prove`; `verify` and `export-rego`
    do not discover configuration or read its environment variables.
- [ ] 4.2.3. Deliver the hostile-evidence and CLI-interaction E2E suite.
  - Requires 4.2.2.
  - See technical-design.md §10.1, V7–V10; terms-of-reference.md §7, SC4–SC6.
  - Success: substitutions of each bound identity, unchanged IR with changed
    source, altered profiles, omitted claims, corrupt worker output, missing
    tools, missing oracle decisions (always a failure for valid admitted
    requests), false oracle decisions on model-allowed requests, and timeout
    after partial proof all yield the specified non-success outcome across
    command/output combinations. A false decision matching a model denial
    passes conformance. Cover every configuration precedence tier per enabled
    command with injected sources and isolated child processes; verify that
    conflicting files and environment cannot change regenerated evidence.

### 4.3. Demonstrate replay within enforced local budgets

Does clean reviewer execution remain bounded when inputs and subprocesses are
hostile? The answer supplies E2 exit evidence rather than assumed performance.
See technical-design.md §§11–12; terms-of-reference.md §7, SC5–SC7.

- [ ] 4.3.1. Harden supervisor containment and remaining-budget accounting.
  - Requires 4.2.3 and 1.2.4.
  - See technical-design.md §§2 and 11; §10, V8.
  - Success: enforced isolation provides private storage and offline execution;
    injected hangs, output floods, cancellation, child crashes, and storage
    escapes leave no running job children; absent isolation fails.
    The 60-second, 2 GiB, and 1 MiB defaults are enforced end to end, not reset
    per oracle invocation, and exhausted bounds are named in the result.
- [ ] 4.3.2. Deliver a clean-environment replay and resource acceptance harness.
  - Requires 4.3.1.
  - See technical-design.md §§11–12, E2; terms-of-reference.md §8.2, A4–A6.
  - Success: offline replay after provisioning checks baseline and maximum
    admitted policies, measures warm wall time and peak memory, and records
    measured acceptance limits or a revise/stop decision. CI exercises pinned
    Lean, OPA, and Regorus separately from Python design-fixture validation.

## 5. Validate a real policy review before declaring the first release

Idea: if a sponsor and reviewer can use bounded replay to judge a meaningful
policy change, the closed workflow warrants distribution; a toy theorem or
clean CI alone does not demonstrate that value.

This phase closes SC8 and checks that documentation and packaging expose the
same assurance boundary as the implementation. It authorizes no new grammar.

### 5.1. Test the workflow's value with the intended users

Can author and reviewer reach a justified policy decision from the evidence?
Their recorded assessment determines whether to release, revise, or stop. See
technical-design.md §12; terms-of-reference.md §§4–5 and 7, SC8.

- [ ] 5.1.1. Run a sponsor-reviewed policy-change demonstration.
  - Requires 4.3.2.
  - See terms-of-reference.md §§7–9, SC8 and OQ1–OQ4;
    technical-design.md §12, First user review.
  - Success: the intended users inspect a meaningful change and its assurance
    ledger, record a proceed/revise/stop decision, and either name a second
    policy with an independent requirement or explicitly defer expansion.

### 5.2. Publish only the capability that passed acceptance

Can a new reviewer install and reproduce exactly the accepted workflow? This
step closes distribution and documentation gates before public availability.
See technical-design.md §§11–14; terms-of-reference.md §9, OQ5.

- [ ] 5.2.1. Document the implemented workflow and assurance ledger.
  - Requires 5.1.1.
  - See users-guide.md; developers-guide.md; repository-layout.md;
    documentation-style-guide.md; technical-design.md §§9–10 and 14.
  - Success: guides, contents, layout, contracts, and validation records agree
    on actual commands, enforced limits, V1–V10 evidence, SC1–SC8 closure, and
    residual trust. Python fixture checks retain their distinct status.
- [ ] 5.2.2. Package the pinned local release and dependency provenance.
  - Requires 5.2.1 and 1.1.2.
  - See technical-design.md §§11 and 13; terms-of-reference.md §9, OQ5;
    [ADR 0001][adr-0001].
  - Success: the sponsor's distribution decision covers dependency licences;
    a clean install provisions trusted tools once and reproduces accepted
    replay offline. Release claims remain limited to the closed public profile.

## 6. Evaluate deferred extensions only after the first workflow earns trust

Idea: if the first release is useful and reproducible, specific follow-on needs
can justify new assurance work without destabilizing the closed profile.

These are conditional decision tasks, not commitments to implement full Rego, a
hosted platform, or a replacement evaluator. Each approved extension needs its
own design and delivery scope; rejection is a valid completion outcome.

### 6.1. Decide whether another policy justifies a new profile

Does a named second policy require additional distinctions? Its independent
requirement determines the next abstraction, rather than parser convenience.
See technical-design.md §§5 and 12–13; terms-of-reference.md §9, OQ1 and OQ6.

- [ ] 6.1.1. Record a second-policy and authoring-surface decision.
  - Requires phase 5.
  - See technical-design.md §§4–5 and 12; terms-of-reference.md §§6.2 and 9.
  - Success: a named policy and independent property justify or reject another
    immutable profile and any bidirectional authoring need. New roles,
    membership, numbers, arithmetic, or general JSON require revised semantics,
    abstraction, proofs, fixtures, and conformance before implementation.

### 6.2. Decide whether stronger correspondence is worth its own project

Which unproved boundary matters to the next user? The answer selects a bounded
assurance increment instead of promoting differential tests into a theorem. See
technical-design.md §§2, 9, and 12–13.

- [ ] 6.2.1. Record a formal translation and runtime-refinement decision.
  - Requires phase 5.
  - See technical-design.md §§7 and 9–10; terms-of-reference.md §6.2, N1–N4.
  - Success: any approved project states its source relation and preservation
    directions for both safety and positive availability, including termination
    and errors. General SMT, SPARK, or a generated runtime needs a separate
    rationale and trust analysis; no Lean-to-Rust transpiler is implied.

### 6.3. Decide whether operational expansion has a demonstrated consumer

Do measured use and replay constraints warrant caching or another deployment
boundary? The answer prevents speculative services from consuming core scope.
See technical-design.md §§7, 11, and 13; terms-of-reference.md §6.2.

- [ ] 6.3.1. Record decisions on caching, services, and broader tooling.
  - Requires phase 5.
  - See technical-design.md §§7 and 11; terms-of-reference.md §6.2, N2–N6;
    [ADR 0001][adr-0001] §Decision.
  - Success: proof-result caching is considered only after adversarial replay;
    multiple crates, concurrency, hosted jobs, UI, signatures, registries, and
    bootstrap tooling need measured demand and separate designs. Arbitrary
    submitted proof code, application certification, and stateful security
    claims remain outside this roadmap.

[adr-0001]: adrs/adr-0001-bootstrap-polean-from-the-combobulate-scaffold.md
[adr-0003]: adrs/adr-0003-pin-the-e0-toolchain-tuple.md
