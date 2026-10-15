# ADR 002: first-profile assurance boundary

- **Status:** accepted.
- **Date:** 2026-10-15.
- **Decision owners:** project sponsor.

## Context

Polean's value depends on saying exactly what a successful run establishes and
nothing more. The [technical design](technical-design.md) proposed a first
public assurance statement, and the [terms of reference](terms-of-reference.md)
made its acceptance an open question (OQ2) resting on two assumptions: that the
source-to-model frontend may remain explicitly trusted in the first experiment
(A2), and that request attributes come from trusted application components (A3).

The proposed statement left attribute provenance implicit, and it named only
some of what the proof trusts. This record fixes the statement and states each
assurance boundary separately: admission, model proof, source-to-model
translation, and runtime correspondence. No theorem described here has been
proved yet; the obligations remain V1–V10 in technical design §10.

## Decision

A successful `prove` or `verify` for `polean.tenant-write.v0` may claim no more
than this statement, which technical design §1 repeats verbatim:

> The admitted policy model satisfies the six registered tenant-write
> requirements for every valid request. Lean checked the model evidence. The
> source frontend remains trusted; OPA and Regorus agree with the model on the
> recorded conformance cases, not by a refinement theorem. The model theorem
> takes the request's subject, resource, and action attributes as given: it
> does not authenticate them, establish their provenance, or certify how an
> application enforces the decision.

Each boundary means the following.

1. **What Lean proves.** For the admitted IR `p` and each of the six claims,
   the claim holds under Polean's pinned semantics for every request in the
   declared domain. The proof route is the abstraction lemma
   (`evalRequest p r = evalFacts p (α r)`), fact coverage, and kernel-reduced
   finite checking (V2–V4). None of these is proved yet.
2. **What that proof trusts.** The Lean kernel; the foundational axioms
   `propext`, `Classical.choice`, and `Quot.sound` only, with no `sorryAx` and
   no native-computation axioms; the integrity of the pinned proof package; and
   Polean's own Lean definitions of the six atoms. Those definitions are the
   specification of what each Rego comparison means, and they are not proved
   against Rego. This is the assumption most at risk of being smuggled into the
   trusted computing base, so this record names it explicitly.
3. **What the trusted frontend assumes.** Regorus parses the source, and
   Polean's Rust adapter maps the AST to IR faithfully and rejects everything
   outside the profile. Admission is performed by this frontend; `check`
   establishes admission only, and admission is not proved. There is no
   source-preservation theorem. Spans, round-trip checks, and differential
   tests reduce this trust but do not remove it (ToR A2).
4. **What differential tests establish.** On the recorded conformance cases,
   OPA and Regorus return the model's decision for the original source. They
   prove neither refinement direction. In particular, transferring a safety
   claim needs `RuntimeAllows(source, r) → ModelAllows(ir, r)`, which remains
   unproved (technical design §9).
5. **What lies outside the theorem.** The authenticity and provenance of the
   subject, resource, and action attributes; application enforcement; later
   state changes; and host integrity (ToR A3, technical design §2).
6. **What a run may report.** Outcome `model_proved` with assurance
   `kernel_checked`, `trusted_frontend`, and `tested_agreement`. No test result
   upgrades the latter two, and release material may not claim more than the
   statement.

## Consequences

Result reporting (roadmap 4.2.1) must render this boundary, including attribute
provenance, in every report. Release material and user documentation quote or
link this record rather than paraphrasing a stronger claim.

The following open questions stay visible until E0 and E1 answer them. They are
recorded as questions, not gates:

1. Do Polean's Lean atom definitions mean what OPA means for the admitted forms?
   Today, only differential sampling (V1, V10) tests this.
2. Can Regorus's AST support complete fail-closed admission without Polean
   becoming an unverified parser that must be trusted wholesale (A5)?
3. Can the finite check run by kernel reduction alone, with no native
   computation, within the resource envelope (A6), so that proof-search code
   stays outside the trusted computing base?

Technical design §13 lists the remaining prerequisites for E0 and for public
release.

## Alternatives considered

Requiring checked source correspondence before v0 would remove the trusted
frontend from the claim. It is A2's consequence if false: a smaller checked
parser or translation witness would have to come first. The first experiment
does not yet know whether admission is even enforceable from the selected
frontend, so this would front-load the hardest assurance work before the
workflow has shown any value.

Omitting provenance from the statement would leave the most common misreading —
that a policy proof says something about who the subject is — uncorrected in
the one sentence most likely to be quoted.

## Acceptance

2026-10-15: the project sponsor (`leynos`) accepted this record as written, in
conversation, when approving the
[1.1 ExecPlan](execplans/1-1-ratify-assurance-and-release-gates.md).
