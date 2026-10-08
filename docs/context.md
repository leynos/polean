# Polean vocabulary

Status: draft v0.1. Date: 2026-10-04.
Companions: [terms of reference](terms-of-reference.md) and
[technical design](technical-design.md).

This glossary defines the proposed first slice. It does not describe an
implemented or already verified product.

| Term | Meaning |
| --- | --- |
| Polean | The proposed Rego policy checker/prover and its evidence workflow. |
| Policy | A decision specification, not a statement about government or organizational governance. |
| Source policy | The exact bytes of the single Rego module submitted for checking. |
| Profile | An immutable, versioned contract defining admitted syntax, input domain, semantics, entry point, claims, and operational limits. |
| Admitted policy | A source policy that passes parsing, legality checks, and the profile's closed allowlist. Admission does not establish a security property. |
| Policy IR | Polean-owned intermediate representation: an ordered list of non-empty conjunctions of allowed atoms. No arbitrary code or Regorus-private types form part of this contract. |
| Atom | One of six comparisons admitted by `polean.tenant-write.v0`. |
| Request | The seven-field, strictly typed authorization context defined by the request schema. |
| Fact vector | Five Booleans derived from a request: same tenant, write action, administrator role, ownership, and locked state. |
| Abstraction | The function from requests to fact vectors. Its preservation theorem must justify reasoning over 32 vectors instead of unbounded strings. |
| Concretization | A constructor producing a valid request for each fact vector. It supports checked, replayable counterexamples. |
| Invariant | A universal requirement on policy decisions, with an independent definition rather than a formula inferred from the policy under test. |
| Availability claim | A positive requirement that a specified class of legitimate requests receives permission. It prevents deny-all from satisfying the complete contract. |
| Model theorem | A Lean theorem about the policy IR under the pinned Polean semantics. It does not alone establish how OPA executes the source bytes. |
| Source binding | The correspondence between submitted source and the policy IR. The first slice trusts its frontend for this correspondence. |
| Runtime correspondence | Agreement between model behaviour and the production evaluation path. The first slice tests this agreement; it does not prove it. |
| Evidence bundle | Source, IR, claim definitions, toolchain identity, proof-replay material, and conformance observations bound by a manifest. A report or hash alone is not proof. |
| Kernel checked | The pinned Lean environment accepted a theorem of the expected type, with only the approved transitive axiom dependencies. |
| Proved | A particular model claim has accepted proof evidence. The assurance dimensions still identify translation and runtime limitations. |
| Refuted | A checked model witness violates a particular claim. Runtime replay adds evidence about source behaviour but remains a distinct observation. |
| Unknown | The run failed to settle a claim, for example because it exhausted a resource limit. It is neither proof nor refutation. |
| Unsupported | The source uses legal language outside the selected profile. Polean must not silently approximate it. |
| Invalid input | The source, request, claim set, configuration, or transport violates a required contract. |
| Trusted computing base | The components and assumptions whose failure could invalidate a reported assurance claim. Different claims have different trusted computing bases. |
| Vertical slice | A demonstrable user workflow through ingestion, model construction, proof or refutation, reporting, and replay, not an isolated implementation layer. |

An application must establish the provenance of subject and resource attributes.
A theorem about supplied tenant identifiers does not authenticate those
identifiers.
