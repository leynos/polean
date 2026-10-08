# Polean – technical design

**Status:** draft v0.1. All components and proof obligations below are
proposed. **Date:** 2026-10-04. **Audience:** Rust and Lean implementers,
security reviewers, and maintainers. **Scope:** one local, source-first
verification workflow for a closed Rego profile.

**Companions:**

- [Terms of reference](terms-of-reference.md)
- [Vocabulary](context.md)
- [Bootstrap ADR](adr-001-repository-bootstrap.md)
- [Contracts](../contracts/profile.json)
- [Draft-pack validation](../validation/README.md)

## 1. Decision and scope

Implement a small Rust orchestrator around a pinned Lean proof package, with
Regorus as an explicitly trusted frontend and OPA as the independent source-
evaluation reference. Keep the policy as data. Do not transpile its evaluator
from Lean to Rust in the first slice.

The first profile admits six specific comparisons, conjunction within a rule,
and disjunction between rules. Five Boolean facts describe every distinction
these policies can observe. Prove that abstraction correct in Lean, then check
all 32 fact vectors with a proof-producing finite checker. This avoids an SMT
solver, arbitrary theorem synthesis, a general Rego semantics, and an
application platform while preserving a meaningful universal model claim.

The first public assurance statement is:

> The admitted policy model satisfies these six requirements for every valid
> request. Lean checked the model evidence. The source frontend remains trusted;
> OPA and Regorus agree on the recorded conformance cases, not by a refinement
> theorem.

An unqualified statement that Polean has verified production Rego execution is
not an acceptable substitute. This boundary implements ToR goals G1–G6, subject
to acceptance of assumption A2.

Full Rego coverage, arbitrary user invariants, temporal properties, hosted
jobs, and a new execution engine remain outside this design. The exact
dependency tuple and measured performance remain entry gates, not invented
current facts.

## 2. Threat model and trust boundaries

Protect the integrity of the claim, the source-to-evidence binding, and the
host running the tools. Assume a submitted policy, evidence directory,
diagnostic string, or cached artefact may be malformed or adversarial. A
contributor may try to obtain a green result by weakening the claim, dropping a
source rule, introducing unsupported syntax, substituting another policy,
supplying an unproved theorem, or exhausting resources.

The first release does not accept user Lean files, tactics, Lake configuration,
native libraries, plug-ins, or compiled proof modules. A job contains bounded
data, not executable project configuration. Treat generated and submitted proof
files differently: the verifier regenerates its own modules from admitted data.

| Boundary                    | First-slice guarantee                                                                                              | Remaining trust or limitation                                                                                         |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- |
| Rego bytes to Polean IR     | Closed profile check, source spans, deterministic mapping, round-trip checks, and independent runtime comparisons. | Regorus parser and Rust mapping remain trusted. No formal source-preservation theorem.                                |
| IR to checked model theorem | Pinned semantics, exact expected proposition, literal policy data, kernel acceptance, and axiom audit.             | Lean kernel, approved logical foundations, trusted proof-package identity, and integrity of the checking environment. |
| Model to OPA/Regorus        | Replay original source against independent requests and compare typed results.                                     | Testing is not a production-runtime refinement proof.                                                                 |
| Evidence to reported result | Fresh reconstruction, binding checks, semantic result validation, and non-zero failure outcomes.                   | Rust orchestrator, serialization, filesystem/process isolation, and hashing assumptions.                              |
| Application to request      | Explicit typed ingress contract.                                                                                   | Attribute authenticity, application enforcement, and later changes of state are outside the theorem.                  |

Trust changes with the claim. A correct Lean theorem can survive a faulty
source translator while saying nothing useful about the submitted source. A
faulty reporter can misdescribe even a correct theorem. Both boundaries remain
visible.

## 3. Validation policy and independent requirements

The baseline is
[examples/tenant-write/policy.rego](../examples/tenant-write/policy.rego):

```rego
package authz
import rego.v1

default allow := false

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
    input.subject.role == "admin"
}

allow if {
    input.subject.tenant == input.resource.tenant
    input.action == "write"
    input.subject.id == input.resource.owner
    input.resource.locked == false
}
```

The intended decision is `T ∧ W ∧ (A ∨ (O ∧ ¬L))`, where T means same tenant, W
means write action, A means administrator, O means ownership, and L means
locked. This formula defines the required behaviour independently of the source
being checked. Never generate the requirements by inspecting that source.

| Claim ID                | Required property for every valid request r |
| ----------------------- | ------------------------------------------- |
| `tenant-isolation`      | `allow(r) → T(r)`                           |
| `write-only`            | `allow(r) → W(r)`                           |
| `locked-requires-admin` | `allow(r) ∧ L(r) → A(r)`                    |
| `admin-or-owner`        | `allow(r) → A(r) ∨ O(r)`                    |
| `admin-write-available` | `T(r) ∧ W(r) ∧ A(r) → allow(r)`             |
| `owner-write-available` | `T(r) ∧ W(r) ∧ O(r) ∧ ¬L(r) → allow(r)`     |

Together the claims characterize the intended decision exactly. The first four
alone do not: deny-all satisfies them. The last two prevent that vacuity and
ensure that a runtime which denies everything cannot count as a satisfactory
implementation merely because it never grants unauthorized access.

All six claims are mandatory for `prove` and `verify` in v0. The claim manifest
selects known IDs; it cannot supply definitions, omit a requirement, add hidden
preconditions, or replace a requirement with `True`.

## 4. The first semantic profile

The machine-readable summary is [profile.json](../contracts/profile.json).
`polean.tenant-write.v0` means this exact syntax and semantic contract, not
“any simple-looking Rego”. Profile identifiers are immutable.

### 4.1 Admission

Accept exactly one UTF-8 module with package `authz`, the single import
`rego.v1`, exactly one `default allow := false`, and zero to 16 non-default
`allow if { ... }` rules. Each rule contains one to 16 allowed comparisons.
Zero rules represent the valid deny-all mutation. Accept normal whitespace and
ordinary comments; reject metadata annotations rather than silently discard
potentially meaningful annotation content.

Only these expression forms are admitted:

| Source expression                               | IR atom         |
| ----------------------------------------------- | --------------- |
| `input.subject.tenant == input.resource.tenant` | `same_tenant`   |
| `input.action == "write"`                       | `write_action`  |
| `input.subject.role == "admin"`                 | `admin_role`    |
| `input.subject.id == input.resource.owner`      | `owns_resource` |
| `input.resource.locked == true`                 | `locked`        |
| `input.resource.locked == false`                | `unlocked`      |

Compare parsed token/operator and decoded literal identity, not source
substrings. The listed operand orientation and dot-reference paths are part of
v0; reverse operands, bracket references, alternate heads, aliases, or other
literal values remain unsupported even when logically equivalent. This
deliberately limits the frontend proof obligation rather than implying that
those forms are invalid Rego.

Reject variables, unification, `not`, `else`, `with`, quantifiers, additional
imports or modules, other rule names, function and built-in calls, `data`
references inside policies, dynamic references, arithmetic, and collections.
Never ignore a declaration because it appears unreachable. Do not use a regular
expression to extract recognized lines from otherwise unexamined Rego.

Rego distinguishes undefined results and evaluation errors. This profile
obtains a total Boolean model by excluding constructs and request shapes that
require those outcomes, not by globally redefining undefined or error as false.
[R1]

### 4.2 Input domain

[request.schema.json](../contracts/request.schema.json) requires exactly the
nested fields `subject.tenant`, `subject.id`, `subject.role`, `resource.tenant`,
`resource.owner`, `resource.locked`, and `action`. Every field is a string
except `locked`, which is Boolean. Reject extra properties, missing fields,
nulls, coercions, duplicate JSON keys, invalid UTF-8, and unpaired Unicode
surrogates. JSON Schema validates the decoded shape; the decoder must
separately enforce the byte-level and duplicate-key rules.

String comparisons use exact Unicode scalar sequences, with no case folding,
normalization, or alias resolution. The mathematical request type permits all
finite valid strings. Actual input decoding enforces a 64 KiB transport cap and
rejects larger documents before evaluation. The model theorem therefore covers
more requests than the concrete ingress accepts; it makes no availability claim
about requests rejected by that ingress limit.

The theorem assumes the application supplied the correct attributes. It neither
authenticates subjects nor infers that an owner identifier is globally unique.
Missing or malformed requests are `invalid_input`, not evidence that the policy
has denied a valid request. Tenant equality must never become a precondition:
that would assume away the attack we want to detect.

## 5. Semantic core and exact finite abstraction

Use a closed `Atom` enumeration, non-empty rule bodies, and an ordered list of
rules. Interpret a body as conjunction and the rule list as disjunction, with
false for no matching rule. Retain rule and atom order in the IR and source
map; v0 performs no optimizer transformations.
[policy.schema.json](../contracts/policy.schema.json) defines the serialized
form.

Define two evaluators in the Lean package:

- `evalRequest : Policy → Request → Bool` reads the actual typed fields.
- `evalFacts : Policy → Facts → Bool` reads five Boolean facts.

`Facts` contains T, W, A, O, and L. `unlocked` evaluates to `!L`; it is not an
independent sixth fact. Let `α : Request → Facts` compute those comparisons.
The essential theorem is:

```text
abstraction_preserves:
  ∀ p r, evalRequest p r = evalFacts p (α r)
```

Prove it by atom cases followed by structural induction over bodies and rules.
A separate theorem connects the executable model to its declarative
conjunction/disjunction semantics. Lean's inductive definitions and
kernel-checked theorem terms provide the machinery; neither theorem exists
merely because the types compile. [R5]

Define `allFacts` as the Cartesian product of five Booleans. Prove that every
`Facts` value occurs. Define `γ : Facts → Request`, choosing two tenant names,
two subject identifiers, `admin` versus `member`, and `write` versus `read`.
Prove `α(γ(f)) = f`. This establishes that every abstract counterexample has a
real request witness; the abstraction does not invent impossible combinations.

For a fixed registered claim i:

```text
check(p, i) = allFacts.all (fun f => claim(i, f, evalFacts p f))

check_sound:
  check(p, i) = true → ∀ r, Claim(i, p, r)

witness_sound:
  check finds f → ¬ Claim(i, p, γ(f))
```

Prove soundness through enumeration coverage and `abstraction_preserves`. Prove
witness soundness through `α(γ(f)) = f`. The six claims depend only on the same
facts, which makes this reduction exact.

**Enumerating 32 hand-picked requests is not the universal proof.** The
universal claim comes from the proved abstraction and coverage lemmas. Their
applicability ends as soon as a new profile can observe something outside these
five facts. Numbers, different string constants, and additional references
require a new profile and a revised proof, not another unchecked atom handler.

Use ordinary kernel-reduced computation, for example a small `decide` proof of
the finite checker result followed by `check_sound`. Do not use
`native_decide`, unchecked external solver verdicts, or generated axioms. All
theorem interfaces here are design signatures, not compiled source supplied
with this draft.

## 6. Architecture and ownership

Figure 1 shows the two independent paths from the same source snapshot.

```text
                    exact Rego source bytes
                       /              \
        Regorus parse + profile       pinned OPA
                adapter               eval source
                   |                     |
             Polean policy IR            | source observations
                   |                     |
          fixed Lean data module         |
                   |                     |
         model proof / checked witness --+-- Regorus source replay
                   |
       evidence binding + assurance report
```

**Figure 1: model proof and runtime observations share source identity, not a
claim of proven runtime equivalence.** OPA consumes the original source, never
only code re-emitted by the same adapter being tested.

Start with one Rust package and one Lean package. Use modules rather than six
separately published crates:

```text
src/{cli,profile,frontend,ir,lean,oracle,evidence,process}.rs
lean/Polean/{Syntax,Semantics,Abstract,Claims,Check,Audit}.lean
examples/tenant-write/
contracts/
```

The Rust package owns process supervision, bounded ingestion, Regorus
adaptation, source maps, serialization, OPA invocation, and evidence packaging.
The Lean package owns semantic definitions, claims, abstraction lemmas, finite
checker soundness, and theorem audits. A Python script in this draft validates
the design fixtures; Python is not part of the proposed production checker.

### 6.1 Frontend

Regorus 0.12.0 exposes `add_policy` and `get_ast_as_json`, the latter behind its
`ast` feature. Its AST JSON is an adapter input, not Polean's stable proof
ABI. Pin the exact crate, enabled features, and adapter fixtures. [R3]

The frontend must account for the complete module: package, imports, default,
all rule heads and bodies, expressions, and any semantically relevant source
metadata. Its initial feasibility gate must demonstrate that the API preserves
enough information to enforce the allowlist. Unsupported nodes fail closed with
byte spans. Unknown serialized variants are errors, not ignorable additions.

If AST inspection loses a property needed for admission, use a bounded source
check or a documented upstream API rather than assert completeness. Metadata
rejection, for example, may need a lexical check. If the adapter cannot enforce
complete admission, block the milestone; do not quietly widen the trust claim.

Polean's semantic IR contains only the profile ID and arrays of six ASCII atom
names. Keep diagnostic byte spans in a separate sidecar so they cannot change
the formal meaning of a policy.

### 6.2 Lean worker

Rust emits literal enum constructors and list structure into a fixed trusted
module template. Source comments, user names, imports, and code never enter a
Lean executable position. The worker checks all requested roots against the
exact policy literal and fixed claim definitions.

Audit transitive axioms for the theorem roots and their semantic dependencies.
Allow only an explicit pinned foundational set, initially `propext`,
`Quot.sound`, and `Classical.choice`; minimize actual use where possible. Reject
`sorryAx`, native-computation trust axioms, and every undeclared assumption.
Lean documents the difference between these foundations and additional axioms
introduced by proof shortcuts. [R6]

A process exit code or the presence of a theorem name is not acceptance.
Require the expected declaration type, exact requested claim coverage,
successful checking, and a machine-readable audit. Missing, malformed, or
contradictory worker output is a tool failure.

### 6.3 Runtime oracles

Run OPA against `data.authz.allow` with only the exact source snapshot. Run
Regorus against the same bytes. Do not load the example directory recursively:
its mutation fixtures share a package and must never merge into one policy. Pin
Rego v1 and explicitly align runtime error options; Regorus documents a
built-in-error default that differs from OPA. The profile excludes built-in
calls, but recording this option prevents a future silent semantic expansion. [
R3, R7]

Preserve the result variants `true`, `false`, `undefined`, `error`, and
malformed transport output. On a valid admitted request, this profile expects
precisely one Boolean decision. Undefined, non-Boolean, multiple, or error
results produce `conformance_mismatch`, not a Boolean coercion. An OPA process
can exit normally while its query result is false; inspect the returned value,
not truthiness or process success. [R7]

## 7. Bidirectional conversion and evidence

The reverse path prints the six atoms into canonical Rego. Its first-slice
obligations are structural round-trip checks and differential execution:

```text
import(render(p)) = p
OPA(original, r) = OPA(render(import(original)), r)
```

These are test obligations for the initial Rust frontend and printer. They are
not described as proved source-equivalence theorems. Preserve the original
source separately; canonical formatting may change comments and bytes.

Canonical IR bytes use the fixed key order `profile`, then `rules`, compact
JSON, no incidental whitespace, preserved array order, and one final LF.
Identifiers are fixed ASCII strings. Duplicate keys and unknown fields fail.
This deliberately avoids needing a general-purpose canonicalization standard.

Bind an evidence bundle to exact source bytes, canonical IR bytes, the
normalized claim set, the profile, input schema, proof package, toolchain
tuple, selected entry point, and runtime options. Hash the complete relevant
artefacts, not just a human-readable version string. Include the orchestrator
build identity, Regorus dependency/features, OPA binary identity, Lean
toolchain and package lockfiles, and the axiom policy in that tuple.

A proposed bundle contains:

```text
manifest.json
source/policy.rego
model/policy.json
claims.json
profile.json
request.schema.json
proof/                 # diagnostic/reproduction material, never blindly executed
witnesses/             # concrete JSON requests, when present
conformance.json
result.json
```

Verification requires explicit `--policy` and `--claims` arguments. It
snapshots those inputs, reconstructs the model, compares bindings, and
regenerates the fixed proof job in a clean pinned environment. Never import
submitted `.olean` files or execute submitted `.lean` files to check a bundle.
For this tiny checker, recomputing the proof is preferable to designing a
portable proof-object format. Trusted toolchain libraries may be cached;
job-supplied compiled artefacts may not.

Call this **replayable proof evidence**, not a self-authenticating security
certificate. Hashes bind identity; they do not prove semantic correspondence or
identify an author. A signature service and public certificate registry are
outside v0.

## 8. CLI, outcomes, and failure semantics

[cli.txt](../contracts/cli.txt) and
[result.schema.json](../contracts/result.schema.json) define the draft
interface.

```bash
polean check policy.rego --profile polean.tenant-write.v0 --format json
polean prove policy.rego --claims claims.json --evidence evidence --format json
polean verify evidence --policy policy.rego --claims claims.json --format json
polean export-rego evidence --output canonical.rego
```

These are proposed commands, not a runnable interface in this draft pack.
`check` establishes admission only. `prove` and `verify` require all six
claims, fresh evidence checking, and required oracle observations. They cannot
succeed with the oracles silently disabled.

| Outcome                | Exit | Meaning                                                                                                                                                 |
| ---------------------- | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `model_proved`         | 0    | All model claims checked; bindings and conformance gates passed. Translation still reads `trusted_frontend`, runtime correspondence `tested_agreement`. |
| `refuted`              | 2    | At least one claim has a checked concrete witness; preserve per-claim results.                                                                          |
| `unknown`              | 3    | No settled verdict because a controlled limit or proof obligation remained unresolved.                                                                  |
| `unsupported`          | 4    | Legal source or requested capability lies outside the profile.                                                                                          |
| `invalid_input`        | 5    | Invalid syntax, JSON, request, claim set, or configuration.                                                                                             |
| `tool_error`           | 6    | Missing dependency, worker crash, protocol failure, or failed required invocation.                                                                      |
| `invalid_evidence`     | 7    | Binding, theorem, axiom policy, or artefact validation failed.                                                                                          |
| `conformance_mismatch` | 8    | An oracle disagreed with the model or returned an inadmissible outcome.                                                                                 |

An aggregate failure never erases useful checked per-claim evidence. If an
oracle mismatch accompanies a model counterexample, retain the counterexample
but give the aggregate conformance failure precedence. Otherwise prefer invalid
evidence, tool failure, or invalid input over an incomplete logical verdict.
The result consumer validates these combinations explicitly.

Schema validation alone cannot establish acceptance. Enforce exactly the six
unique expected IDs; all must be proved for exit zero. Verify non-null binding
identities, allowed theorem roots, witness containment, assurance consistency,
and the absence of failed required gates. A malformed report cannot choose its
own exit status by setting a JSON field.

Write one structured result to stdout and bounded diagnostics to stderr. Escape
untrusted text and keep raw witness inputs out of routine logs. Evidence export
is explicit. Atom-to-source maps explain a failed claim without inventing a
unique causal rule when several rules authorize the witness.

## 9. Lessons from the Lean-to-Rust transpiler

The supplied shared conversation was inaccessible. A recovered summary
identified `auser/lean4-prod` and its correspondence concerns. This draft
rechecked its README and relevant generated-shift source rather than treating
the earlier analysis as a fresh audit of the entire repository. [R8–R10]

The README describes lowering through Lean's compiler normal form, erasing
proofs, and representing natural numbers with bounded Rust integers. The
inspected code includes `checked_shl` lowering. Rust's documentation explicitly
allows a checked left shift to discard high bits: it checks the shift amount,
not lossless preservation of an unbounded natural-number result. [R8–R10]

| Lesson                                                                            | Polean decision and falsification check                                                                                                                 |
| --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A source theorem and successful target compilation do not establish preservation. | Keep Lean as the model proof environment and existing source runtimes as oracles. Reject any release wording claiming a verified Rust policy evaluator. |
| Matching API names can hide semantic differences.                                 | Exclude numbers and arithmetic initially. Any later numeric profile must specify width, overflow, division, and conversions before adding code.         |
| Erasing proof-only preconditions does not enforce them at a foreign interface.    | Validate request shape and provenance assumptions explicitly. Do not place tenant equality in an assumed precondition.                                  |
| Proof-root coverage is useful inventory, not a proof of the translation.          | Check the exact theorem proposition and full source binding; keep frontend trust visible even when every root passes.                                   |
| Shared expected-output generation can reproduce the same error twice.             | Test original Rego in OPA, compare Regorus, use independently written requirements, and retain hand-authored mutations.                                 |
| Internal compiler dependencies introduce another compatibility boundary.          | Use stable surface Lean data modules, pin the toolchain, and avoid LCNF export or runtime code generation in v0.                                        |

The runtime preservation direction matters. To transfer a safety theorem, a
later proof must establish `RuntimeAllows(source, r) → ModelAllows(ir, r)`
under the stated input relation. To transfer the positive availability
requirements, the reverse direction and relevant termination/error behaviour
also matter. A runtime that always denies is safe in the first direction but
fails the product contract. Neither direction is proved by the initial
conformance suite.

## 10. Verification obligations

These are release obligations, not accomplished results.

| ID  | Property                                                                          | Method and scope                                                               | What remains unchecked                                    |
| --- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | --------------------------------------------------------- |
| V1  | Every admitted atom has the specified request meaning.                            | Lean atom-case lemmas; complete profile-adapter fixtures.                      | Rust parsing and source mapping remain trusted.           |
| V2  | `evalRequest p r = evalFacts p (α r)`.                                            | Lean structural induction over every admitted IR policy.                       | Full Rego semantics and actual runtimes.                  |
| V3  | Every fact vector occurs in `allFacts`; `α(γ(f)) = f`.                            | Lean Boolean case analysis.                                                    | JSON decoder correctness and actual attribute provenance. |
| V4  | Finite checking implies each universal model claim.                               | Lean soundness theorem and per-policy kernel-reduced certificate.              | Runtime refinement.                                       |
| V5  | A reported model counterexample violates the named claim.                         | Lean witness proof plus concrete JSON replay in OPA and Regorus.               | General runtime equivalence beyond observations.          |
| V6  | All six baseline requirements hold; deny-all fails availability.                  | Checked model claims and independent truth-table acceptance fixtures.          | Application-level availability or system state changes.   |
| V7  | Evidence names exactly the policy, domain, requirements, and environment checked. | Negative substitution fixtures and clean regeneration against explicit inputs. | Hash collision assumptions and host compromise.           |
| V8  | Unsupported, malformed, incomplete, or timed-out work never passes.               | Adversarial state/result matrix; process-kill and corrupt-output injection.    | General absence of implementation defects.                |
| V9  | Rendering and re-import preserve admitted IR.                                     | Property generation within profile bounds and exact structural comparison.     | Formal parser/printer proof.                              |
| V10 | Original and canonical source agree with model on required cases.                 | Independent OPA/Regorus conformance observations.                              | Universal source-runtime correspondence.                  |

The trust ledger must describe these boundaries even when every test passes.
Test success never upgrades `trusted_frontend` to `proved_translation`.

### 10.1 Counterexamples and interaction coverage

The independent fixture expectations are:

| Mutation                          | Required failing claim                  |
| --------------------------------- | --------------------------------------- |
| Remove administrator tenant guard | Tenant isolation.                       |
| Remove owner lock guard           | Locked writes require an administrator. |
| Remove owner identity guard       | Administrator-or-owner authorization.   |
| Remove administrator action guard | Write-only access.                      |
| Remove all non-default rules      | Both availability requirements.         |
| Remove owner rule                 | Owner-write availability.               |

Exercise all 32 fact vectors for each policy in model validation and both
source oracles. Add variants with empty strings, distinct Unicode sequences,
escaped JSON, mismatched identifiers, and long strings inside limits. These
variants test adapters; they are not additional premises in the abstraction
theorem.

Require targeted combinations rather than a random feature checklist:
unsupported syntax in a rule that never fires; false or missing oracle
decisions on a model allow; unchanged IR with changed source bytes; reordered
or omitted claims; valid evidence with an altered profile; timeout after some
claims prove; and malformed JSON that supplies `"false"` instead of `false`.

Property generation must retain the closed profile and shrink a mismatch while
preserving syntax validity and the disagreement. Oracle inputs and expected
requirements must not come solely from the frontend mapping under test. The
hand-authored source fixtures remain independent sentinels for mapping mistakes.

## 11. Resource control and execution environment

Start with Linux local execution, one proof job at a time, no daemon, database,
remote solver, or mandatory async runtime. A synchronous Rust process
supervisor is sufficient for this workload. Split crates or add a service only
after a second consumer or measured concurrency need creates a concrete
boundary.

Set proposed defaults of 64 KiB source/input, 16 rules, 16 atoms per rule, 60
seconds per end-to-end verification, 2 GiB memory, and 1 MiB structured output.
The profile records these as initial budgets, not proven performance. Propagate
a single remaining deadline through all child invocations; do not restart a
full budget for each oracle input. Kill and reap the process group on
cancellation.

Use a private job directory, immutable input snapshots, a cleared environment,
explicit executable paths, a network-disabled sandbox, and bounded
stdout/stderr. Reject symlinks, path traversal, and artefact paths outside that
directory. Spawn argument arrays, never shell-concatenate policy paths. Missing
enforced isolation is an execution failure for the first public profile, not a
silently degraded security mode.

Provision the trusted Lean package and dependencies before jobs. A submitted
Lake file must never control a build. Pin the Rust lockfile, Regorus features,
Lean toolchain/package dependencies, and OPA binary digest. Regorus 0.12.0 is
an inspected frontend candidate, not a tested complete dependency tuple.
Resolve Lean, Rust, and OPA pins in E0 below and record them before accepting
evidence.

Cache trusted build/toolchain artefacts. Defer proof-result caching until the
binding and replay path has passed adversarial checks. Build CI should stay
within a proposed 4 vCPU / 8 GiB envelope; non-build validation should fit 1
vCPU / 2 GiB. Measure warm verification wall time and peak resident memory. No
paid proving service or recurring background runner is required.

## 12. Vertical experiments and exit gates

A standalone syntax library or proof notebook is not an exit gate. Each
experiment adds an observable user workflow. These are scope boundaries, not a
separate full project roadmap.

| Experiment                   | Hypothesis                                                                                    | Demonstrable slice                                                                                                                    | Evidence and stop condition                                                                                                                                                           |
| ---------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| E0: first source-bound claim | A tiny exact frontend and finite model are sufficient for a meaningful source-first workflow. | One development command reads baseline source, proves tenant isolation, and produces a checked witness for the cross-tenant mutation. | Pin a working toolchain tuple; document source trust; inspect full AST coverage. Stop rather than generalize if the frontend cannot enforce admission. E0 is not a public v0 release. |
| E1: complete policy contract | Useful safety and positive requirements fit the same small proof method.                      | The same workflow checks all six claims, all mutations, exact abstraction, and typed request boundaries.                              | V1–V6 and required finite-domain lemmas pass Lean checking. No manual edits to per-policy proof scripts. Keep this scope closed until it works.                                       |
| E2: reviewer replay          | Evidence can remain bound and reproducible across an untrusted artefact boundary.             | A clean invocation verifies explicit source and claim files, reconstructs evidence, and replays OPA/Regorus observations.             | V7–V10, hostile-input matrix, sandbox limits, and measured budgets pass. All ToR success criteria except sponsor adoption have concrete evidence.                                     |
| First user review            | The workflow improves an actual policy review.                                                | Sponsor and reviewer inspect one meaningful change and its assurance ledger.                                                          | Record a proceed/revise/stop decision under SC8. No new language features merely to avoid a negative result.                                                                          |

A subsequent profile requires a named second policy and independently stated
property. A likely experiment may add another role or bounded membership, but
neither is designed or authorized here. Before widening the grammar, prove the
new abstraction or select a different proof strategy. Formal source translation
and production-runtime refinement are separate future assurance increments.

## 13. Alternatives and unresolved decisions

| Decision                 | Selected approach                                                    | Alternative and reason for deferral                                                                                                                               |
| ------------------------ | -------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Orchestration            | Rust, one package with modules.                                      | Python or Go would add another language without removing Lean or the Regorus adapter. No concrete need justifies that boundary.                                   |
| Proof method             | Exact five-fact abstraction with checked finite enumeration.         | General SMT compilation requires certificates or extra solver trust and adds no capability needed by these requirements.                                          |
| Policy representation    | Closed data IR.                                                      | Arbitrary Lean functions lose guaranteed Rego representability and make safe ingestion substantially harder.                                                      |
| Runtime                  | OPA remains the source oracle; Regorus supplies another observation. | A generated Rust evaluator would add a new preservation obligation before the policy workflow has demonstrated value.                                             |
| Input semantics          | Strict typed request domain.                                         | A general JSON/undefined/error semantics would expand the first proof substantially. The boundary remains explicit rather than silently coercing malformed input. |
| Evidence consumption     | Regenerate fixed proofs from admitted data.                          | Importing arbitrary compiled proof artefacts creates an avoidable integrity and execution boundary.                                                               |
| Verification environment | Lean for language/model theorems.                                    | SPARK may suit a later verified imperative runtime, but a second proof tool does not itself establish cross-language correspondence.                              |

Open gates: exact tested version tuple; complete Regorus admission coverage;
axiom-root audit API; Linux sandbox implementation and availability; measured
resource defaults; licence and distribution choice; and sponsor acceptance of
the first-slice trust statement. Assign owners before implementation; do not
invent an upstream API or silently weaken a gate when a spike fails.

## 14. Draft artefacts and provenance

The adjacent JSON schemas, source fixtures, mutations, and validation script
make the scope executable enough to inspect. They do not implement the Rust
orchestrator, Lean proof package, parser, or a certified translator. The
[validation record](../validation/README.md) states which checks ran and which
proof/runtime checks did not.

This document now lives in the initial `leynos/polean` repository scaffold. The
scaffold supplies the documented paths, contracts, examples, and design
validation fixtures, but it does not implement the proposed Rust orchestrator,
Lean package, frontend, sandbox, or evidence verifier. Repository presence must
not be reported as proof completion.

The technical-design structure follows the df12 security-oriented pattern:
threats and trust boundaries before components, named verification obligations,
external contracts, explicit failure outcomes, and an editing pass. The terms
of reference remains the upstream authority for scope. [T1–T3]

## References

Sources were inspected on 2026-10-04. Moving documentation URLs describe the
inspected API, not a substitute for release pins.

- **R1.**
  [OPA policy language](https://www.openpolicyagent.org/docs/policy-language).
- **R2.**
  [OPA policy testing](https://www.openpolicyagent.org/docs/policy-testing).
- **R3.**
  [Regorus 0.12.0 Engine API](https://docs.rs/regorus/0.12.0/regorus/struct.Engine.html):
  AST feature, source evaluation, and explicit error options.
- **R4.**
  [Cedar Analysis](https://aws.amazon.com/blogs/opensource/introducing-cedar-analysis-open-source-tools-for-verifying-authorization-policies/):
  adjacent authorization-analysis prior art; no dependency in v0.
- **R5.**
  [Lean elaboration and compilation](https://lean-lang.org/doc/reference/latest/Elaboration-and-Compilation/):
  separation of checking and execution.
- **R6.** [Lean axioms](https://lean-lang.org/doc/reference/latest/Axioms/):
  foundational axioms, `sorryAx`, and additional trust from native computation.
- **R7.** [OPA CLI](https://www.openpolicyagent.org/docs/cli):
  explicit evaluation inputs, query output, and runtime options.
- **R8.**
  [lean4-prod README](https://github.com/auser/lean4-prod/blob/main/README.md):
  inspected compiler route, proof erasure, and bounded natural-number contract.
- **R9.**
  [Inspected lean4-prod code-generation source](https://github.com/auser/lean4-prod/blob/991956fe427bb646498fd0912c89b71be71e7beb/rust/prod-codegen/src/lib.rs):
  `checked_shl` lowering, corroborated by source-search excerpts at this commit.
- **R10.**
  [Rust u64 checked shifts](https://doc.rust-lang.org/std/primitive.u64.html#method.checked_shl):
  shift-count checking does not guarantee lossless unbounded arithmetic.
- **R11.**
  [User-supplied shared discussion](https://chatgpt.com/share/6ac21ad3-64ec-83eb-96a2-7d459bac7164?ogimg=plain):
  direct fetch failed. A recovered summary identified lessons; R8–R10 supplied
  independently checked primary evidence for the concrete transpilation issue.
- **T1.**
  [df12 technical-design skill](https://github.com/leynos/df12-documentation-skills/blob/main/skills/tech-design-doc/SKILL.md).
- **T2.**
  [df12 document anatomy](https://github.com/leynos/df12-documentation-skills/blob/main/skills/tech-design-doc/references/document-anatomy.md).
  Inspected blob: `c4b40c97bab445760d846fe5de33e324f44a5f17`.
- **T3.**
  [df12 research protocol](https://github.com/leynos/df12-documentation-skills/blob/main/skills/tech-design-doc/references/research-protocol.md)
  and
  [editing checklist](https://github.com/leynos/df12-documentation-skills/blob/main/skills/tech-design-doc/references/editing-checklist.md).
