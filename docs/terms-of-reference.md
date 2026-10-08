# Polean – terms of reference

**Status:** draft v0.1, for review; no implementation or proof completion claimed.
**Date:** 2026-10-04.
**Audience:** project sponsor, implementers, policy authors, and technical reviewers.
**Companions:** [technical design](technical-design.md),
[vocabulary](context.md), and [draft-pack validation](../validation/README.md).

`[KNOWN]` denotes an explicit project requirement or sourced observation.
`[ASSUMED]` denotes a proposed premise that needs validation. `[OPEN]` denotes an
unresolved matter. Proposed success criteria are acceptance proposals, not
measurements. This brief reconstructs the problem from the existing design
conversation; it does not pretend that customer discovery has already occurred.

## 1. Background and motivation

[KNOWN] The project brief asks for Polean to verify a lightweight but meaningful
security policy through a minimal-subset vertical slice. The preceding discussion
establishes the larger ambition: represent Rego policies in a proof environment,
state independent invariants, translate mechanically, and distinguish mathematical
claims from assumptions about production execution. [P1]

[KNOWN] The immediate example concerns a multi-tenant document service. An
administrator may write within their own tenant. An owner may write an unlocked
resource within their own tenant. Administrator status must not bypass the
tenant boundary. The example gives the first experiment two authorization paths,
a negative security boundary, and legitimate requests that must remain possible.
[P1]

[ASSUMED] The motivating pain is uncertainty during policy changes: examples can
pass while a newly added rule broadens access in an unexamined case. The initial
user wants evidence about an explicitly stated requirement, not another label
saying that a file compiled successfully.

[OPEN] No customer interviews, purchase commitments, operational incident record,
or measured maintenance cost accompany this brief. Polean therefore starts as a
founder-led feasibility experiment. A successful demonstrator establishes a
capability; it does not establish product-market fit.

## 2. Domain

[KNOWN] The domain is authorization policy assurance. Rego is the policy language
used by Open Policy Agent (OPA), and OPA provides policy evaluation and example-
based testing workflows. Rego distinguishes undefined results from Boolean
values. [R1, R2]

[KNOWN] Three questions require separate answers: whether a policy belongs to the
supported language, whether its model satisfies a requirement, and whether the
actual execution path corresponds to that model. The existing conversation makes
that separation a project requirement. [P1]

[ASSUMED] The first useful assurance statement concerns one decision at a time.
The application supplies an authenticated subject, trustworthy resource
attributes, and a requested action. The experiment does not establish how the
application authenticates these facts, changes them over time, or enforces the
returned decision.

The normative vocabulary lives in [context.md](context.md). In particular,
*availability* means a positive authorization requirement, not service uptime.
*Verified* must always identify what was verified and under which assumptions.

[OPEN] No regulatory certification, contractual assurance level, or sector-specific
standard has been selected. The project must not imply compliance certification.

## 3. Market context

The following alternatives define the immediate comparison, not an exhaustive
market survey.

| Alternative | Established capability | Consequence for the experiment |
| --- | --- | --- |
| OPA evaluation and policy tests | Evaluates Rego and runs policy tests. [R1, R2] | Preserve the existing policy workflow. Show what the universal model claim adds rather than replace tests. |
| Regorus | Rust implementation with AST inspection and evaluation APIs. [R3] | Reuse implementation work where it fits, but do not describe implementation agreement as a semantic-preservation proof. |
| Cedar Analysis | Authorization analysis based on a Lean-developed symbolic compiler and automated reasoning. [R4] | Treat this as relevant prior art. Polean must justify its Rego-facing niche rather than claim that authorization verification is novel. |
| Handwritten proof developments or program contracts | The preceding exploration considered direct Lean specifications and SPARK contracts. [P1] | The proposed value is a repeatable source-to-evidence workflow, not the mere existence of a theorem prover. |
| Review, selected examples, or no additional verification | Proposed current alternatives for the initial user; not supported by a market survey. | The demonstrator must make a concrete regression easier to detect and explain. |

[ASSUMED] Rego compatibility matters more to the initial experiment than support
for multiple policy languages. [OPEN] Whether other Rego users will accept a
narrow profile, and what they will pay for assurance, remain unresolved.

## 4. Users and stakeholders

These roles are proposed from the project brief, not research personas.

| Role | Context and job | What matters | Likely rejection criterion | Status |
| --- | --- | --- | --- | --- |
| Policy author, primary user | Edits a small authorization policy in a repository. | A precise pass, counterexample, or inconclusive result tied to that revision. | A new policy language or manual proof work for every routine edit. | [ASSUMED] |
| Technical reviewer, secondary user | Reviews a change and its supporting evidence. | Exact assumptions, source binding, and independent replay. | An unexplained green badge or proof of a different policy. | [ASSUMED] |
| Project maintainer, stakeholder and operator | Maintains the checker and its semantic boundary. | Small scope, reproducible runs, controlled dependency and CI costs. | A full-language implementation before any user-visible evidence. | [KNOWN] scope requirement; role allocation open. |
| Sponsor | Decides whether the capability warrants further investment. | A defensible proceed, revise, or stop decision. | Unbounded implementation effort without a meaningful demonstration. | [ASSUMED]; individual authority not assigned. |
| Production enforcement operator, non-user of v0 | Owns a live service's authentication and authorization integration. | System-level behaviour. | Any implication that a model theorem certifies an unexamined deployment. | [KNOWN] outside the initial deliverable. |

## 5. Job to be done

[ASSUMED] When changing a repository-managed authorization policy, a policy author
wants to establish that the change preserves named access boundaries and still
permits legitimate work, so they can submit the change with inspectable evidence
rather than rely only on selected examples.

[ASSUMED] When reviewing that change, a technical reviewer wants to reproduce the
claim against the exact source and assumptions, so they can distinguish a policy
regression from unsupported syntax, incomplete analysis, or an implementation
mismatch.

The functional outcome is a settled claim or an honest explanation of why it
remains unsettled. The associated confidence must derive from visible evidence.
The product should not ask reviewers to trust the author of the proof-search
code merely because the report looks authoritative.

## 6. Scope

### 6.1 Goals

The following goals define the first experiment.

| ID | Proposed goal |
| --- | --- |
| G1 | Accept the agreed tenant-write policy and state exactly which source forms and requests the claim covers. |
| G2 | Establish tenant isolation, write-only access, administrator-or-owner authorization, and administrator-only writes to locked resources for every request in the declared domain. |
| G3 | Establish that eligible administrators and owners retain the specified access, so deny-all fails the complete contract. |
| G4 | Produce a concrete, replayable violating request for deliberately weakened policies; distinguish that result from an inconclusive run. |
| G5 | Bind evidence to the source, requirements, domain, semantic version, and relevant execution configuration. |
| G6 | Let a reviewer reproduce the evidence and see the remaining source-translation and runtime assumptions. |

### 6.2 Non-goals

| ID | Explicit boundary |
| --- | --- |
| N1 | Full Rego coverage is a longer-term ambition, not a prerequisite for the experiment. Unsupported legal constructs receive explicit rejection, not approximation. |
| N2 | Arbitrary user-written invariants, unrestricted theorem-prover code, and automatically invented security requirements remain outside v0. |
| N3 | Stateful privilege escalation, identity-provider correctness, external network facts, and application-level enforcement remain separate assurance work. |
| N4 | A replacement production policy engine, compiler, virtual machine, or Lean-to-Rust transpiler is not part of the first deliverable. |
| N5 | A multi-tenant hosted service, graphical authoring environment, plug-in platform, and distributed proving farm remain deferred. |
| N6 | Regulatory certification and unconditional claims about production security remain outside scope. Existing evaluation and operational safeguards remain necessary. |

## 7. Success criteria

These proposed acceptance criteria trace directly to the goals. They describe
observable outcomes, not a sequence of implementation tasks.

| ID | Category | Acceptance observation | Goals |
| --- | --- | --- | --- |
| SC1 | User-facing | The baseline policy satisfies all six independently defined requirements without editing proof code. | G1–G3 |
| SC2 | User-facing | Removing the administrator tenant guard produces a checked cross-tenant witness that the unmodified source runtime reproduces. | G2, G4 |
| SC3 | User-facing | Removing the owner lock or identity guard, removing the administrator action guard, and denying everything fail the appropriate requirements. | G2–G4 |
| SC4 | Reviewer-facing | Changing source, requirements, domain, semantic identity, or relevant runtime configuration invalidates previously bound evidence. | G5 |
| SC5 | Reviewer-facing | A reviewer replays the baseline evidence in a clean, pinned environment and sees its precise assurance scope. | G6 |
| SC6 | Operational | Unsupported syntax, malformed evidence, timeout, missing tools, and runtime disagreement never produce a successful verification verdict. | G1, G5, G6 |
| SC7 | Operational | A warmed run completes within an explicitly configured local CPU, memory, and wall-clock budget; failures report the exhausted bound. Initial values belong to the design and require measurement. | G6 |
| SC8 | Strategic | The sponsor reviews one real policy change using the demonstrator and records whether another policy warrants expansion. A toy proof alone does not authorize a larger platform. | G1–G6 |

SC1, SC2, and SC5 must use the same source-to-evidence workflow. Separate scripts
that prove a hand-copied policy while checking unrelated source do not satisfy
these criteria.

## 8. Constraints and assumptions

### 8.1 Hard constraints

[KNOWN] The immediate scope is a minimal but meaningful vertical slice. Preserve
that boundary when implementation convenience suggests a broader framework. [P1]

[KNOWN] The project must expose assumptions and distinguish proof, refutation,
inconclusive analysis, unsupported language, and operational failure. [P1]

[KNOWN] The document templates require evidence-backed design, explicit
verification targets, and a separation between this problem brief and the
technical solution. [T1, T2]

[OPEN] No delivery date, staffing commitment, numerical budget, licensing
requirement, or supported-platform matrix has been authorized. Proposed
engineering defaults do not become external commitments merely by appearing in
the design.

### 8.2 Assumptions

| ID | Assumption | Consequence if false | Resolution evidence |
| --- | --- | --- | --- |
| A1 | One constrained decision policy is enough to test the workflow's usefulness. | Broader syntax work may hide the original experiment's value. | Review an actual baseline and mutation with the intended author. |
| A2 | The source-to-model frontend may remain explicitly trusted in the first experiment. | The first milestone needs a smaller checked parser or translation witness before it can claim useful assurance. | Sponsor accepts the assurance statement or rejects it before release. |
| A3 | Request attributes come from trusted application components and obey the declared domain. | The theorem can hold while the application remains vulnerable. | Review the stated attribute provenance assumptions; do not infer deployment correctness. |
| A4 | Local, offline verification after toolchain provisioning meets the first user's needs. | Distribution, tenancy, or integration requirements may change the threat model. | Complete a clean-machine reviewer replay. |
| A5 | Current frontend APIs can expose enough syntax information to enforce an exact allowlist. | A wrapper, small supported parser, or revised dependency choice becomes necessary. | A bounded frontend feasibility experiment, not a broad parser rewrite. |
| A6 | The small proof domain fits the proposed resource envelope. | Simplify the representation or proof method before increasing the feature surface. | Record timings and memory for baseline, maximum admitted size, and failure cases. |

### 8.3 Dependencies

[KNOWN] The design conversation selects Lean semantics and Rust orchestration as
candidate implementation directions, and identifies Regorus as reusable prior
art. These are downstream design choices, not proof that the problem requires
those particular tools. [P1]

[ASSUMED] The project can obtain and pin the necessary toolchains and runtime
binaries, and the intended reviewer can provision them once. The first public
acceptance must resolve the exact compatible dependency tuple.

The provided transpiler discussion informs assurance requirements. Its shared
page could not be retrieved for this draft; a recovered conversation summary and
independently inspected source informed the technical design. No unseen details
of that page serve as acceptance evidence. [P2]

## 9. Open questions

Owners below denote roles to allocate, not named commitments.

| ID | Question | Why it matters | Closure criterion | Proposed owner |
| --- | --- | --- | --- | --- |
| OQ1 | Which real repository policy will follow the demonstrator? | Determines whether the narrow profile solves a useful job. | Select a concrete source policy and independent requirement. | Sponsor / policy author |
| OQ2 | Is model proof plus an explicit trusted frontend sufficient for v0? | Sets the minimum acceptable assurance boundary. | Accept the exact claim wording or require checked source correspondence. | Sponsor / reviewer |
| OQ3 | Who validates the subject and resource attributes? | Prevents policy-level evidence from implying identity correctness. | Document the intended integration's provenance and ingress contract. | Application owner |
| OQ4 | What resource and onboarding budgets matter to the first reviewer? | Turns provisional defaults into useful requirements. | Measure the workflow on an agreed machine and set acceptance limits. | Maintainer |
| OQ5 | What licence and distribution model should the project use? | Affects redistribution and dependency packaging. | Record a sponsor decision after reviewing applicable dependencies. | Sponsor |
| OQ6 | Is future bidirectional authoring essential, or is source-first verification enough? | Determines whether a native policy authoring surface earns its cost. | Demonstrate a user task that needs round-tripping rather than only source inspection. | Policy author / sponsor |
| OQ7 | Can the original shared discussion be made available for a precise provenance check? | Separates recovered lessons from the exact supplied transcript. | Compare an accessible transcript with the sourced design decisions. | Sponsor |

## 10. Handoff

This brief supports a bounded technical experiment with A1–A6 and OQ1–OQ7
visible. It does not authorize full-language support or a production-security
claim. The [technical design](technical-design.md) selects the first semantic
profile, proof strategy, orchestration boundary, and evidence protocol.

The [vocabulary](context.md) supplies the initial context document. Candidates
for later architectural decision records are the assurance boundary, profile
versioning, proof-evidence format, and any eventual decision to replace the
production evaluator. No separate roadmap or ADRs were requested in this draft.

## References

All retrievable external sources were inspected on 2026-10-04.

- **P1.** User-provided Polean design conversation and current drafting request.
  Primary project requirements, not a published implementation claim.
- **P2.** [Referenced Lean-to-Rust discussion](https://chatgpt.com/share/6ac21ad3-64ec-83eb-96a2-7d459bac7164?ogimg=plain).
  Direct retrieval failed. A recovered conversation summary supplied context;
  the technical design identifies the primary sources checked independently.
- **T1.** [df12 terms-of-reference skill](https://github.com/leynos/df12-documentation-skills/blob/main/skills/terms-of-reference-doc/SKILL.md).
  Inspected file blob: `51782c18874ebe67072fb9feffe04785ca9c653d`.
- **T2.** [df12 technical-design skill](https://github.com/leynos/df12-documentation-skills/blob/main/skills/tech-design-doc/SKILL.md),
  with its document anatomy, research protocol, and editing checklist.
- **R1.** [OPA policy language](https://www.openpolicyagent.org/docs/policy-language).
- **R2.** [OPA policy testing](https://www.openpolicyagent.org/docs/policy-testing).
- **R3.** [Regorus 0.12.0 Engine API](https://docs.rs/regorus/0.12.0/regorus/struct.Engine.html).
- **R4.** [Cedar Analysis announcement](https://aws.amazon.com/blogs/opensource/introducing-cedar-analysis-open-source-tools-for-verifying-authorization-policies/).
