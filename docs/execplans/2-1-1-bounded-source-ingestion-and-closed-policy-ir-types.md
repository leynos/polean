# Implement bounded source ingestion and closed policy IR types

This ExecPlan (execution plan) is a living document. Keep `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan`
current as work proceeds.

Status: DRAFT (revision 1). Awaiting design review and user approval.
Implementation must not begin until the user approves this plan.

Roadmap item: [2.1.1](../roadmap.md). Branch:
`2-1-1-bounded-source-ingestion-and-closed-policy-ir-types`.

## Purpose / big picture

Polean checks one deliberately tiny dialect of Rego, the profile
`polean.tenant-write.v0`. A policy in that profile is a list of zero to 16
`allow` rules. Each rule is a conjunction of one to 16 _atoms_, and an atom is
one of six fixed comparisons such as `same_tenant`
(`input.subject.tenant == input.resource.tenant`). Polean's _policy
intermediate representation_ (IR) is exactly that structure: an ordered list of
non-empty, ordered lists of atoms, with nothing else in it. The Lean model,
the evidence bundle, and every later roadmap item consume the IR rather than
Rego.

Today the crate under `src/` is a scaffold. Nothing in production code reads a
policy, bounds its size, checks its encoding, or represents the IR. The 1.2.2
admission spike under `tests/admission/` used a throwaway `Vec<Vec<String>>`
and read sources without a cap, and its design record says that type must not
be promoted. The committed IR fixtures, such as
`examples/tenant-write/policy.json`, are pretty-printed JSON; nothing produces
the compact _canonical bytes_ that technical design §7 says evidence must bind.

After this work, a library caller can do three things, each with a closed,
typed failure:

1. Ingest a policy source from any `std::io::Read` or byte vector. Ingestion
   reads at most 65,537 bytes, rejects anything longer than 65,536 bytes
   without consuming the rest of the stream, rejects malformed UTF-8 with the
   offset of the first bad byte, and otherwise returns the exact bytes
   unchanged as a `PolicySource`.
2. Build or decode a `Policy`. The types make an empty rule body, a 17th rule,
   a 17th atom, or an unknown atom unrepresentable. The JSON decoder accepts
   the committed pretty-printed fixtures, and rejects duplicate keys, unknown
   fields, a missing field, a different profile, an unknown atom, an empty
   body, and every bound violation, each with its own error variant.
3. Encode a `Policy` to canonical bytes:
   `{"profile":"polean.tenant-write.v0","rules":[...]}` followed by one line
   feed, with no other whitespace and with rule and atom order preserved. Zero
   rules encode as `"rules":[]`. A Verus proof establishes that the executable
   encoder produces exactly the specified bytes and that distinct policies
   never share canonical bytes. Hashing canonical bytes, which roadmap 4.1.1
   will do, therefore identifies exactly one policy.

For example, the baseline fixture becomes:

```plaintext
{"profile":"polean.tenant-write.v0","rules":[["same_tenant","write_action","admin_role"],["same_tenant","write_action","owns_resource","unlocked"]]}
```

followed by a single line feed.

None of this admits Rego. Turning source into IR is roadmap 2.1.2, which will
consume `PolicySource` and produce `Policy`. Nothing here proves that a source
belongs to the profile, that a model claim holds, that translation is correct,
or that a runtime agrees with the model. The Verus theorem concerns the byte
encoding of IR values only.

## Constraints

- Keep the four claims in `AGENTS.md` separate. No type name, error message,
  test name, or document may describe ingestion, decoding, or canonical
  encoding as admission, proof of a policy, translation, or runtime agreement.
  The Verus theorem is stated as a property of the canonical byte encoding.
- Production code lives in one Rust package under `src/`, following technical
  design §6's module names: `src/profile.rs`, `src/source/`, and `src/ir/`.
  `self_named_module_files` is denied, so directories use `mod.rs`.
- The IR holds only the six atoms in rule order. No byte spans, source text,
  comments, or Regorus types may enter it (technical design §6.1); 2.1.2 owns
  the diagnostic sidecar.
- Unsupported or malformed data fails closed. There is no lenient mode, no
  coercion, no last-key-wins, no truncation, and no ignored field.
- Library code must not print, exit, read environment variables, touch the
  filesystem, or depend on OrthoConfig (technical design §8; Whitaker's
  `no_std_fs_operations`; `clippy.toml` `disallowed-methods`). Ingestion takes
  `impl Read`; opening files is the CLI's job in 2.1.4.
- Do not change `contracts/`, `examples/`, `validation/`, or `lean/`. The
  pretty-printed fixtures remain the decoder's inputs; this plan does not
  rewrite them into canonical form.
- Do not change the admission spike in `tests/admission/`. Its throwaway IR
  remains test-only and 2.1.2 retires it.
- Authorized dependency changes are exactly these, all at the versions already
  in `Cargo.lock`: move `serde` (`=1.0.229`, feature `derive`) and
  `serde_json` (`=1.0.154`) to `[dependencies]`; and, if the user accepts Q1,
  move `vstd` (`=0.0.0-2026-09-06-0133`) to `[dependencies]`. Each stays in
  `[dev-dependencies]` only if still needed there. Regorus stays a
  dev-dependency until 2.1.2.
- Verus proofs must not use `assume`, `admit`, `external_body`,
  `external_fn_specification`, any other trusted proof body, or a new axiom.
  Unproved statements may live in this plan, not in code.
- Follow the lint policy: no `unsafe`; no `allow` attributes (a scoped
  `#[expect(..., reason = "...")]` is permitted); every module starts with
  `//!`; modules stay under 400 lines and functions under 70 lines; public
  items carry Rustdoc with an example where useful.
- `make verus` stays on demand, outside `make all` and CI, as ADR 0005 records.
- Use British English with Oxford spelling. Refresh `SHA256SUMS` only for
  covered files this plan changes (`docs/technical-design.md`, and
  `docs/context.md` if its glossary changes), after reviewing the change.
- Do not bypass compile admission (`RUSTC_WRAPPER`), do not create an isolated
  Cargo cache, and do not use `/tmp` as a build target.
- Implementation begins only after the user approves this plan.

## Tolerances (exception triggers)

Exclude this ExecPlan from every count, including when it is updated in the
same change. Exclude reviewed `insta` snapshots (`**/*.snap`) and proptest
regression files (`**/*.proptest-regressions`).

Measure net lines (additions minus deletions) against the merge base with
`main` at `ad217c8`, per category, using:

```bash
git diff --numstat ad217c8...HEAD -- . \
  ':(exclude)docs/execplans/**' ':(exclude)**/*.snap' \
  ':(exclude)**/*.proptest-regressions'
```

Classify whole files by path:

- Code: `src/**`, `tests/**` (including `.feature` files), `verus/**`,
  `scripts/**`, and `Makefile`.
- Configuration: `Cargo.toml`, `Cargo.lock`, `.github/**`, `.config/**`, and
  `SHA256SUMS`.
- Documentation: `docs/**` except ExecPlans, `README.md`, and
  `tools/verus/README.md`.

Limits, each enforced independently:

- Code: stop and escalate above 7,000 net lines.
- Configuration: stop and escalate above 80 net lines.
- Documentation: stop and escalate above 1,100 net lines.
- Files: stop and escalate above 45 changed non-ExecPlan files, snapshots
  excluded.
- Interface: stop if a public item beyond those named in `Interfaces and
  dependencies` is needed, or if `contracts/` would have to change.
- Dependencies: stop if any dependency other than those authorized in
  `Constraints` is needed, or if a pinned version would have to change.
- Verification: stop if a Verus theorem in `Verification plan` cannot be
  proved without a forbidden construct, or can only be proved in a weaker
  form. Present the open verification condition instead.
- Design: stop and set `BLOCKED` if the serde-based decoder cannot
  distinguish a required rejection class, or if technical design §7 turns out
  to require behaviour this plan contradicts.
- Iterations: stop when a gate still fails after three fix attempts on one
  cause.
- Ambiguity: stop on a newly found ambiguity whose readings would accept
  different inputs.

The estimate behind these limits is in `Context and orientation`.

## Risks

- Risk: the Verus proof of canonical uniqueness is harder than estimated,
  because it reasons about the concatenation of nested sequences. Severity:
  medium. Likelihood: medium. Mitigation: the proof sketch below reduces it to
  a left-inverse spec decoder over a prefix-free token set, which is the shape
  Vest-style serializer proofs use. EP-M2 delivers a fully tested encoder, so
  EP-M3 can stop without losing tested behaviour.
- Risk: production code inside `verus!` blocks conflicts with the strict lint
  policy or with Whitaker. Severity: medium. Likelihood: low, because the
  1.2.2 recognizer kernel already passes `make lint` as test code. Mitigation:
  EP-M2 lints the kernel before any proof is written; a lint conflict that
  needs an `allow` is an escalation.
- Risk: serde's derived or hand-written visitors silently accept a duplicate
  key or unknown field. Severity: high. Likelihood: low; serde 1.0.229 raises
  `duplicate_field` in derived struct visitors
  (`serde_derive-1.0.229/src/de/struct_.rs:269`). Mitigation: the decoder
  detects multiplicity itself, so no rejection depends on derive behaviour;
  red tests show each rejection failing before the code exists.
- Risk: serde_json accepts input that an RFC 8259 reader would reject, or the
  reverse. Severity: medium. Likelihood: low. Mitigation: the whole document
  is UTF-8-checked by `std` first; tests cover a leading byte-order mark
  (BOM), trailing content, unpaired surrogate escapes, control characters in
  strings, and nesting depth. serde_json is treated as an axiom, not verified.
- Risk: canonical bytes diverge from the IR the Python harness and Lean will
  use. Severity: medium. Likelihood: low. Mitigation: a test encodes every
  committed IR fixture and compares it with an independent serde_json compact
  rendering, and an `insta` snapshot records the canonical bytes for review.
- Risk: making `vstd` a production dependency adds a pre-release crate to the
  shipped dependency graph. Severity: medium. Likelihood: certain if Q1 is
  accepted. Mitigation: ADR 0007 records the decision; `verus!` erases ghost
  code under plain `rustc`, and the crate is already locked and built by
  every test run.
- Risk: estimates are low again. Severity: medium. Likelihood: medium; both
  previous plans overran their scope tolerances threefold. Mitigation: these
  budgets start from an itemized survey and carry contingency; a breach still
  stops work.

## Progress

- [x] (2026-11-14) Renamed the branch to
  `2-1-1-bounded-source-ingestion-and-closed-policy-ir-types`; the remote
  branch does not exist yet, so tracking is set at the first push.
- [x] (2026-11-14) Wyvern reconnaissance: admission spike and IR fixtures;
  Verus wiring and gates; prior ExecPlans and governing documents; contracts,
  outcomes, and downstream consumers.
- [x] (2026-11-14) Checked serde and serde_json behaviour in the locked
  sources, and Vest as prior art for verified serializers.
- [x] (2026-11-14) Drafted revision 1.
- [ ] Community-of-experts design review, and revision.
- [ ] User decision on Q1, and user approval of the plan.
- [ ] `EP-M1`: profile constants and bounded source ingestion.
- [ ] `EP-M2`: closed IR types, canonical encoder, and strict decoder.
- [ ] `EP-M3`: Verus proof of the canonical encoding.
- [ ] `EP-M4`: documentation, ADR 0007, and delivery closure.

## Surprises & discoveries

- Observation: the committed IR fixtures are pretty-printed, not canonical.
  Evidence: `examples/tenant-write/policy.json` uses two-space indentation;
  `validation/validate_design.py` writes JSON with `json.dumps(indent=2)`.
  Impact: the decoder must accept any JSON whitespace, and canonical form is a
  property of the encoder's output (D3).
- Observation: no code reads `source_bytes` or `input_bytes` from
  `contracts/profile.json`, and nothing tests the 65,536-byte boundary.
  Evidence: repository-wide search for `source_bytes`, `input_bytes`, and
  `65536`. Impact: EP-M1 adds the first enforcement and a contract-drift test.
- Observation: `scripts/check_verus_controls.py` checks that each proof
  verifies and that each negative control is rejected, but nothing scans proof
  sources for `assume`, `admit`, or `external_body`. Evidence: wyvern review of
  the script. Impact: EP-M3 adds a forbidden-construct scan over every proof
  tree, so the prohibition in `Constraints` is enforced rather than promised.
- Observation: registering a Verus entry point takes three uncoordinated
  edits: `VERUS_PROOF_FILES` in the `Makefile`, a `ProofTree` in
  `scripts/check_verus_controls.py`, and a hard-coded list in
  `scripts/tests/test_verus_make_targets.py`. Impact: EP-M3 updates all three
  and adds a test that the Makefile list and the script's trees agree.
- Observation: serde_json's string ignorer, used for values of unknown keys,
  does not validate UTF-8 inside a byte slice. Evidence:
  `serde_json-1.0.154/src/read.rs`. Impact: the decoder validates the whole
  document with `std::str::from_utf8` before parsing (D6).

## Decision log

- D1. Decision: put production code in `src/profile.rs`, `src/source/`, and
  `src/ir/`, matching technical design §6's module names. Rationale: one
  package, modules rather than crates, as §6 and ADR 0001 require. `profile`
  holds the constants that `contracts/profile.json` fixes, so `source` and
  `ir` do not each restate them. Date/Author: 2026-11-14, planning agent.
- D2. Decision: ingestion accepts at most 65,536 bytes inclusive, reads at
  most 65,537 bytes from a stream, and checks size before UTF-8. Rationale:
  `profile.json` sets `source_bytes` to 65,536 and the design says decoding
  rejects "larger" documents, so 65,536 is accepted. Reading one byte past the
  cap is the least work that distinguishes "at the cap" from "over it" without
  trusting a length hint. Checking size first means an over-long stream is
  never decoded, and a multi-byte character split by the cap reports
  `TooLarge`, not a spurious encoding error. Date/Author: 2026-11-14, planning
  agent.
- D3. Decision: the decoder accepts any JSON layout and either key order; the
  encoder alone defines canonical bytes. Rationale: the committed fixtures are
  pretty-printed, and technical design §7 specifies the canonical form as an
  output rule while separately requiring duplicate keys and unknown fields to
  fail. A canonical-only decoder would reject every committed fixture. A
  caller that must know whether bytes are canonical compares them with
  `to_canonical_bytes` of the decoded policy; that check is correct because
  the encoder is proved injective (L-CANON-INJECTIVE) and the round trip is
  tested (INV-ROUND-TRIP). No such public helper is added until roadmap 4.1.3
  needs one. Date/Author: 2026-11-14, planning agent.
- D4. Decision: ingestion preserves bytes exactly. It does not strip a BOM,
  normalize line endings, or reject NUL. Rationale: all three are valid UTF-8,
  technical design §7 requires the original source to be preserved, and the
  1.2.2 admission corpus already rejects a leading BOM at parse and a NUL in
  the recognizer. Rejecting them here would duplicate admission rules in a
  layer that has no reason codes for them. Date/Author: 2026-11-14, planning
  agent.
- D5. Decision: decode JSON with serde_json, using a hand-written
  `Deserialize` visitor for the top-level object that records every key it
  sees, and validate the result in plain Rust. Rationale: the visitor makes
  duplicate-key and unknown-field detection repository code with closed error
  variants, rather than derive behaviour reported through serde's
  string-typed custom errors. serde_json still owns JSON syntax, escapes,
  unpaired surrogates, trailing content, and nesting depth, which are the
  hard parts of RFC 8259 and are widely exercised. The strongest alternative,
  a hand-written JSON parser verified in Verus, would prove more but would
  make Polean own a JSON grammar the product does not otherwise need; the
  canonical encoder, which Polean does own, is verified instead. Date/Author:
  2026-11-14, planning agent.
- D6. Decision: the IR decoder enforces a 65,536-byte document cap and
  validates the whole document as UTF-8 before parsing. Rationale: technical
  design §11 sets 64 KiB as the input budget. The largest valid canonical
  document is about 4.2 KiB (L-CANON-LENGTH), so the cap never rejects a
  valid policy in any reasonable layout, while bounding parse work. Whole-
  document validation closes the serde_json ignorer gap noted in `Surprises &
  discoveries`. Date/Author: 2026-11-14, planning agent.
- D7. Decision: accept JSON escapes in atom names and the profile string, so
  `"same_tenant"` decodes to `same_tenant`. Rationale: the IR is defined
  over JSON string values, the canonical encoder never emits an escape, and
  equality is then on decoded values. This differs deliberately from 1.2.2's
  Q1, which rejected escaped spellings in _Rego source_ because the recognizer
  compares source bytes; IR decoding has no byte-level contract. Flagged for
  design review. Date/Author: 2026-11-14, planning agent.
- D8. Decision: the decoder reports the first failure under a fixed
  precedence: size, then UTF-8, then JSON syntax and JSON types, then key
  multiplicity (duplicate and unknown keys in document order, then missing
  keys), then profile, then rule count, then each rule in order (empty body,
  atom count, then each atom in order). Rationale: one deterministic answer per
  input keeps diagnostics and tests stable. Date/Author: 2026-11-14, planning
  agent.
- D9. Decision: duplicate atoms within a body, and duplicate rules, are valid
  IR and are preserved. Rationale: `policy.schema.json` has no `uniqueItems`,
  technical design §5 says v0 performs no optimizer transformations, and Rego
  admits a repeated comparison. Date/Author: 2026-11-14, planning agent.
- D10. Decision: verify the canonical encoder in Verus; cover ingestion,
  decoding, and type invariants with tests. Rationale: see `Formal
  verification decision`. Date/Author: 2026-11-14, planning agent.
- D11. Decision: write the canonical encoder as a kernel inside `verus!` in
  `src/ir/canonical/`, `#[path]`-included by `verus/policy_canonical.rs`, so
  the executed encoder and the verified encoder are one source. Rationale:
  this is the 1.2.2 pattern (`verus/admission_recognizer.rs`), reversed in
  direction, and it is the only way to prove the shipped code rather than a
  copy. It requires `vstd` in `[dependencies]`; see Q1. Date/Author:
  2026-11-14, planning agent, pending Q1.
- D12. Decision: the users' guide documents the library in prose and points to
  Rustdoc, whose examples are executed as doctests by `make test`; it adds no
  fenced example. Rationale: every fenced example in `docs/users-guide.md`
  needs a tested-example marker and an `ExecutionScope` classification in
  `tests/documentation_examples/scope.rs`, and none of the existing scopes
  executes Rust. Doctests already execute. Date/Author: 2026-11-14, planning
  agent.
- D13. Decision: propose, for 2.1.4 to adopt, that `SourceError::TooLarge` and
  `SourceError::InvalidUtf8` map to `invalid_input` (exit 5) and
  `SourceError::Read` to `tool_error` (exit 6). Rationale: `contracts/cli.txt`
  gives exit 5 to "malformed source" and exit 6 to tool and dependency
  failures. This plan adds no CLI and no outcome mapping code; the proposal is
  recorded in ADR 0007 so 2.1.4 does not have to rediscover it. Date/Author:
  2026-11-14, planning agent.

### Open questions for the user

- Q1. May `vstd` become a production dependency so the canonical encoder can
  be verified where it ships? Recommended: yes. The recognizer kernel that
  2.1.2 must promote is written the same way, so the dependency arrives in
  2.1.2 regardless; taking it now lets ADR 0007 decide it once. The
  alternative keeps the encoder in plain Rust and proves only a separate
  specification, linking it to the shipped encoder by tests; that proves less
  and duplicates the encoder.

## Outcomes & retrospective

Not started. Complete at each milestone and at delivery.

## Context and orientation

Base revision inspected: `ad217c8` on `main` ("Demonstrate declaration-type and
transitive-axiom auditing (1.2.3)"). Line numbers below are from that
revision; relocate them with `grep -n`.

### Current repository state

`src/lib.rs` is a scaffold exporting `STATUS` and `project_name()`, checked by
`tests/stub.rs`. `Cargo.toml` has an empty `[dependencies]` table; serde,
serde_json, proptest, rstest, rstest-bdd, googletest, pretty_assertions, insta,
cap-std, Regorus, and vstd are dev-dependencies pinned with `=`. The lint
policy in `Cargo.toml` denies, among others, `indexing_slicing`,
`unwrap_used`, `expect_used`, `missing_const_for_fn`, `must_use_candidate`,
`shadow_*`, `missing_docs`, `missing_docs_in_private_items`, and
`self_named_module_files`, and forbids `unsafe_code`. `clippy.toml` caps
cognitive complexity at 9 and function length at 70 lines and disallows
environment access. Whitaker (`make lint-whitaker`, run by `make lint`) adds
`no_std_fs_operations`; tests reach files through `cap-std`, as
`tests/toolchain/ambient.rs` does.

`contracts/profile.json` fixes the profile id `polean.tenant-write.v0`, rule
limits (`min` 0, `max` 16, `body_min` 1, `body_max` 16), the atom names in the
order `same_tenant`, `write_action`, `admin_role`, `owns_resource`, `locked`,
`unlocked`, and `limits.source_bytes` and `limits.input_bytes` of 65,536.
`contracts/policy.schema.json` describes the IR: an object with exactly
`profile` (the constant id) and `rules` (at most 16 arrays of 1 to 16 atom
names). Technical design §7 (`docs/technical-design.md:479-482`) defines
canonical bytes: key order `profile` then `rules`, compact JSON, no incidental
whitespace, preserved array order, one final line feed, fixed ASCII
identifiers, and failure on duplicate keys and unknown fields.

The 1.2.2 spike (`tests/admission/`) holds the only IR-like code. Its
`Ir = Vec<Vec<String>>` (`tests/admission/admission/walk/mod.rs:35`) is
throwaway (ADR 0005, `docs/admission-feasibility-record.md` §12). Its
recognizer kernel (`tests/admission/recognizer/kernel/`) is the precedent for
sharing a kernel with Verus: the kernel body sits in `verus! { ... }`,
`#[path]` pulls in `verus/admission_spec.rs` and `verus/admission_lemmas.rs`,
ghost-only imports are gated on `#[cfg(verus_keep_ghost)]` (declared in
`Cargo.toml` `[lints.rust]`), and `verus/admission_recognizer.rs` includes the
kernel with `#[path]` and an empty `main`. `tests/toolchain/audit/wire/` is the
precedent for strict serde decoding with byte caps checked before parsing.

### Verus wiring

`make install-verus` installs the Verus release pinned in `tools/verus/` and
Z3 through `rust-prover-tools` (`tools/rust-prover-tools/REF`). `make verus`
runs `verus-proof` over `VERUS_PROOF_FILES` (`Makefile:122`), then
`verus-controls`, which runs `scripts/check_verus_controls.py`. That script
stages each `ProofTree` into scratch, requires the unmodified tree to verify,
then applies each negative control and requires Verus to reject it with named
output. `scripts/tests/test_verus_make_targets.py:200` hard-codes the proof
file list. None of this runs in CI.

### Gates

Commit gates (`AGENTS.md`): `make check-fmt`, `make typecheck`, `make lint`
(including `interrogate` and `lint-whitaker`), `make test`, `make test-scripts`,
`make design-check`, `make markdownlint`, and `make toolchain-smoke` when a pin,
`scripts/`, `lean/`, or `tests/toolchain/` changes. This plan changes
`Cargo.toml` and `scripts/`, so `make test-scripts` and `make toolchain-smoke`
are in scope; `.github/workflows/toolchain.yml` is path-filtered on
`Cargo.toml`, `Cargo.lock`, `scripts/**`, `verus/**`, and
`tests/admission/recognizer/kernel/**`. Run gates sequentially, through `tee`
to `/tmp/<gate>-polean-2-1-1-bounded-source-ingestion-and-closed-policy-ir-types.out`,
and delegate full gate runs to the `scrutineer` agent.

### Downstream consumers

- 2.1.2 turns a `PolicySource` into a `Policy`, keeping byte spans in a
  sidecar.
- 2.1.3 decodes requests under the same 65,536-byte cap and the same
  duplicate-key discipline; it reuses `source`'s bounded reader (see
  `Interfaces and dependencies`).
- 2.2.1 and 2.2.4 mirror `Atom` and `Policy` as Lean enum and list literals;
  the IR must contain no free strings.
- 2.3.2 runs OPA and Regorus over the exact `PolicySource` bytes.
- 4.1.1 and 4.1.3 bind and re-derive canonical IR bytes.

### Budget survey

Planned files and estimated net lines of code:

- `src/lib.rs` (+20), `src/profile.rs` (90), `src/source/mod.rs` (220),
  `src/source/tests.rs` (260), `src/ir/mod.rs` (260), `src/ir/tests.rs` (240),
  `src/ir/canonical/mod.rs` (220, the `verus!` kernel), `src/ir/decode/mod.rs`
  (260), `src/ir/decode/tests.rs` (320).
- `verus/policy_canonical.rs` (25), `verus/policy_canonical_spec.rs` (160),
  `verus/policy_canonical_lemmas.rs` (450), `verus/policy_canonical_witnesses.rs`
  (60).
- `tests/ingestion/main.rs` (40), `tests/ingestion/bdd.rs` (220),
  `tests/ingestion/properties/` (380), `tests/ingestion/contracts.rs` (130),
  `tests/ingestion/fixtures.rs` (130), `tests/ingestion/support.rs` (90),
  `tests/ingestion/features/policy_ingestion.feature` (60).
- `scripts/check_verus_controls.py` (+170), `scripts/tests/` (+180),
  `Makefile` (+3).

That is about 4,650 lines. The two comparable plans overran: 1.2.2 set about
4,000 lines and 40 files and delivered 14,072 lines in 96 files, recorded
without a stated cause beyond assessment repair; 1.2.3 set about 3,000 lines
and 30 files and delivered 9,406 lines in 66 files, which its D23 attributes to
a wrong estimate rather than scope growth, and the sponsor raised its trigger
to 15,000 lines. Both overruns were dominated by tests, proof lemmas, and
review-round repairs, which this survey now itemizes. The code limit of 7,000
lines is the survey plus 50 per cent contingency for proof lemmas and review
repairs. The 45-file limit is the 26 planned files plus documentation, plus
contingency for module splits forced by the 400-line rule.

Configuration: `Cargo.toml` (about +8 −4), `Cargo.lock` (expected 0, because
locked packages do not record dependency kind), `toolchain.yml` (+2 path
filters), `SHA256SUMS` (in-place digest changes). The 80-line limit allows for
lockfile churn if Cargo reorders entries.

Documentation: ADR 0007 (180), `docs/technical-design.md` (+90),
`docs/developers-guide.md` (+170), `docs/users-guide.md` (+50),
`docs/repository-layout.md` (+25), `docs/contents.md` (+8), `docs/context.md`
(+8), `docs/roadmap.md` (1), `tools/verus/README.md` (+60). That is about 590
lines; the 1,100-line limit allows for review-driven clarification.

### Signposts

Read before implementing: `AGENTS.md`; `docs/technical-design.md` §§4–7 and
§11; `docs/admission-feasibility-record.md` §§11–12; ADR 0002, ADR 0005;
`docs/developers-guide.md` ("Admission spike", "Tests", "Documented examples");
`docs/rust-testing-with-rstest-fixtures.md`;
`docs/reliable-testing-in-rust-via-dependency-injection.md` (inject the reader;
do not touch the filesystem in the library); `docs/rust-doctest-dry-guide.md`;
`docs/complexity-antipatterns-and-refactoring-strategies.md`;
`docs/scripting-standards.md` (for `scripts/` changes); and
`tools/verus/README.md`. Load these skills: `rust-router`, then
`rust-types-and-apis` (newtypes and closed enums), `rust-errors` (closed error
enums without `thiserror`), `rust-unit-testing` (rstest, googletest,
pretty_assertions, insta), `proptest`, `verus`, `addressing-whitaker-findings`,
and `execplans` to keep this plan current.

## Conformance basis

Upstream artefacts inspected at `ad217c8`:

- Terms of reference, draft v0.1 (2026-10-04): G1 (state exactly which source
  forms and requests are covered), N1 (explicit rejection, not approximation),
  SC6 (unsupported or malformed input never yields success), A2 (trusted
  frontend), A6 (resource envelope).
- Technical design, draft v0.1 (2026-10-04, amended by 1.2.2 and 1.2.3): §4.1
  (one UTF-8 module; zero to 16 rules of one to 16 atoms), §4.2 (64 KiB
  transport cap), §5 (closed `Atom`, non-empty bodies, ordered rules, no
  optimizer), §6 (module layout), §6.1 (IR holds only profile id and atom
  arrays; spans in a sidecar), §7 (canonical bytes), §11 (64 KiB budgets), and
  obligations V7, V8, and V9 in §10.
- ADR 0001 (one package), ADR 0002 (assurance boundary), ADR 0003 (exact pins),
  ADR 0005 (2.1.1 owns production source and IR types; the spike IR is not
  promoted; Verus stays on demand).
- `contracts/profile.json`, `contracts/policy.schema.json`, `contracts/cli.txt`.
- Roadmap item RM-2.1.1 and its success criteria.

Requirements derived for this plan:

- REQ-SRC-CAP: a source longer than 65,536 bytes is rejected, and ingestion
  never reads more than 65,537 bytes from a stream.
- REQ-SRC-UTF8: a source that is not valid UTF-8 is rejected with the offset
  of the first invalid byte.
- REQ-SRC-EXACT: an accepted source's bytes are exactly the input bytes.
- REQ-IR-CLOSED: an IR atom is one of six enum values; the IR holds no strings,
  spans, or source data.
- REQ-IR-BOUNDS: a policy has 0 to 16 rules and each rule 1 to 16 atoms, by
  construction.
- REQ-IR-ORDER: rule order and atom order are preserved by construction,
  decoding, and encoding.
- REQ-CANON-FORM: canonical bytes are `{"profile":"<id>","rules":<rules>}` plus
  one line feed, compact, with the specified key order.
- REQ-CANON-UNIQUE: distinct policies have distinct canonical bytes, so a
  digest of the bytes identifies one policy.
- REQ-DECODE-STRICT: decoding rejects duplicate keys, unknown fields, missing
  fields, a different profile, unknown atoms, empty bodies, bound violations,
  malformed JSON, malformed UTF-8, and oversized documents.
- REQ-CONTRACT-AGREE: the Rust constants and atom names equal those in
  `contracts/profile.json` and `contracts/policy.schema.json`.

Gaps at the base revision:

- GAP-NO-PROD: no production source or IR type exists.
- GAP-NO-CAP: nothing enforces or tests the source cap.
- GAP-NO-CANON: nothing produces canonical IR bytes.
- GAP-VERUS-SCAN: Verus controls do not reject forbidden proof constructs.

Trace links:

```plaintext
TOR-SC6, TDD-4.2, TDD-11 -> REQ-SRC-CAP, REQ-SRC-UTF8 -> EP-M1 -> tests::ingestion::source_*, src/source/tests.rs
TDD-7 -> REQ-SRC-EXACT -> EP-M1 -> INV-SRC-EXACT property
TDD-5, TDD-6.1 -> REQ-IR-CLOSED, REQ-IR-BOUNDS, REQ-IR-ORDER -> EP-M2 -> src/ir/tests.rs, INV-ROUND-TRIP
TDD-7 -> REQ-CANON-FORM, REQ-CANON-UNIQUE -> EP-M2, EP-M3 -> fixtures snapshot, L-CANON-REFINES, L-CANON-INJECTIVE
TDD-7, TOR-N1 -> REQ-DECODE-STRICT -> EP-M2 -> src/ir/decode/tests.rs, INV-DECODE-REJECTS
profile.json, policy.schema.json -> REQ-CONTRACT-AGREE -> EP-M1, EP-M2 -> tests::ingestion::contracts
GAP-VERUS-SCAN -> EP-M3 -> scripts/tests forbidden-construct cases
RM-2.1.1 -> EP-M4 -> roadmap entry marked done
```

## Verification plan

### Abstract model

Let `A` be the six-element set of atoms, in profile order. A _policy_ is a
finite sequence of rules, and a rule is a finite sequence over `A`. A policy
`p` is _well formed_, written `WF(p)`, when `|p| ≤ 16` and every rule `r` in
`p` has `1 ≤ |r| ≤ 16`.

Let `name : A → Bytes` give each atom's ASCII name, and let `q(a)` be `name(a)`
wrapped in double quotes. For a sequence `xs` and separator `s`, `join(xs, s)`
concatenates the elements with `s` between neighbours. Define:

```plaintext
body(r)   = "[" ++ join(map(q, r), ",") ++ "]"
rules(p)  = "[" ++ join(map(body, p), ",") ++ "]"
render(p) = "{\"profile\":\"polean.tenant-write.v0\",\"rules\":" ++ rules(p) ++ "}\n"
```

`render` is the specification of canonical bytes. It is total over all
policies, including ill-formed ones, so the uniqueness theorem below does not
depend on `WF`.

Source ingestion is modelled as a function from a byte stream `s` to a result:

```plaintext
ingest(s) = TooLarge                     if |s| > 65,536
          = InvalidUtf8(k)               if |s| ≤ 65,536 and s is not UTF-8,
                                           k = the longest valid UTF-8 prefix
          = Ok(s)                        otherwise
```

with the side condition that at most `min(|s|, 65,537)` bytes are consumed.
Read errors other than `Interrupted` are returned as `Read`.

Decoding is modelled as `decode : Bytes → Result<Policy, Error>`, required to
satisfy `decode(render(p)) = Ok(p)` for every `WF(p)`, `decode(b) = Ok(p) ⇒
WF(p)`, and the rejection classes in REQ-DECODE-STRICT, under the D8
precedence.

### Invariants and lemmas

- INV-SRC-CAP: `ingest` returns `TooLarge` exactly when the stream holds more
  than 65,536 bytes, and never consumes more than 65,537 bytes. (REQ-SRC-CAP)
- INV-SRC-UTF8: within the cap, `ingest` returns `InvalidUtf8(k)` exactly when
  the bytes are not UTF-8, with `k` equal to `std`'s `valid_up_to`.
  (REQ-SRC-UTF8)
- INV-SRC-EXACT: `ingest(s) = Ok(t)` implies `t = s` byte for byte.
  (REQ-SRC-EXACT)
- INV-SRC-CHUNKING: the result of ingesting a stream does not depend on how
  the reader splits it into reads, nor on interleaved `Interrupted` errors.
  (REQ-SRC-CAP, REQ-SRC-EXACT)
- INV-IR-WF: every `Policy` value satisfies `WF`; the constructors return an
  error, never a value, otherwise. (REQ-IR-BOUNDS)
- L-CANON-REFINES: for every `Policy` value `p`, the executable encoder
  returns exactly `render(p)`. (REQ-CANON-FORM)
- L-CANON-INJECTIVE: for all policies `p` and `p'`, `render(p) = render(p')`
  implies `p = p'`. (REQ-CANON-UNIQUE, REQ-IR-ORDER)
- L-CANON-SHAPE: `render(p)` begins with
  `{"profile":"polean.tenant-write.v0","rules":[`, ends with `]}` and one line
  feed, contains no other line feed, and contains no space, tab, or carriage
  return. (REQ-CANON-FORM)
- L-CANON-LENGTH: `WF(p)` implies `|render(p)|` is at most the computed
  maximum (about 4,200 bytes; the proof fixes the exact constant), which is
  below the decoder's 65,536-byte cap. (Links REQ-CANON-FORM to D6.)
- INV-ROUND-TRIP: `decode(render(p)) = Ok(p)` for every `WF(p)`, and
  `decode(b) = Ok(p)` for every JSON layout `b` of `p` (any whitespace, either
  key order, escaped spellings). (REQ-IR-ORDER, REQ-DECODE-STRICT)
- INV-DECODE-REJECTS: each class in REQ-DECODE-STRICT produces its own error
  variant under the D8 precedence, and never `Ok`. (REQ-DECODE-STRICT)
- INV-CONTRACT: the constants in `src/profile.rs` and `Atom`'s names and order
  equal `contracts/profile.json` and `contracts/policy.schema.json`.
  (REQ-CONTRACT-AGREE)

### Proof sketch for the canonical encoding

These are hypotheses about the proof's shape, not commitments to its
decomposition. Implementation may merge, split, or replace them.

L-CANON-INJECTIVE is the substantive theorem. The intended route is a
specification-level decoder `parse` with `parse(render(p)) = Some(p)` for all
`p`; injectivity follows at once, since `render(p) = render(p')` gives
`Some(p) = Some(p')`. The decoder need not be executable. Supporting
hypotheses:

1. Prefix-freeness of atom tokens: for atoms `a ≠ b`, `q(a)` is not a prefix
   of `q(b) ++ t` for any `t`. This holds because the six names begin with
   distinct letters (`s`, `w`, `a`, `o`, `l`, `u`), and a closing quote ends
   each token.
2. Body decoding: for any rule `r` and any suffix `t`,
   `parseBody(body(r) ++ t) = Some((r, t))`, by induction on `r`, using (1)
   and the fact that `,` and `]` cannot begin an atom token.
3. Rule-list decoding: for any `p` and suffix `t`,
   `parseRules(rules(p) ++ t) = Some((p, t))`, by induction on `p`, using (2)
   and the fact that a body begins with `[` while the list's terminator is `]`.
4. Envelope: `render` is a fixed prefix, `rules(p)`, and a fixed suffix, so
   `parse` strips the prefix, applies (3), and requires the suffix to remain.

L-CANON-REFINES is proved by loop invariants in the kernel: after the encoder
has written rules `0..i`, its buffer equals the prefix of `render(p)` built
from those rules; likewise within a body. L-CANON-SHAPE follows from the
definition and a lemma that `rules(p)` contains only bytes from the atom names
and `[],"`. L-CANON-LENGTH follows by bounding `|body(r)|` by
`2 + 16 · 15 + 15` and summing.

The exact atom names are part of the specification. The executable
`Atom::name` sits in the kernel with an `ensures` tying it to the
specification's `name`, and INV-CONTRACT ties both to the contract, so a drift
between the proof's names and the profile's names fails a test.

### Axioms (trusted, not verified here)

- AXIOM-STD-UTF8: `std::str::from_utf8` accepts exactly well-formed UTF-8 and
  reports `valid_up_to` and `error_len` as documented. INV-SRC-UTF8 relies on
  it.
- AXIOM-STD-READ: `Read::take` and `Read::read_to_end` read no more than the
  limit, retry on `Interrupted`, and stop at end of stream, as documented.
  INV-SRC-CAP relies on it; a counting reader exercises it.
- AXIOM-SERDE-JSON: serde_json 1.0.154 parses RFC 8259 JSON, rejects trailing
  content, unpaired surrogate escapes when decoding `String`, and control
  characters in strings, and enforces a nesting limit of 128. INV-ROUND-TRIP
  and INV-DECODE-REJECTS rely on it; named tests exercise each behaviour.
- AXIOM-VERUS: Verus at the pinned release, its `vstd` library, and the
  pinned Z3 are sound, and `verus!` erases ghost code under plain `rustc`
  without changing executable code. L-CANON-* rely on it.

### Methods per obligation

- INV-SRC-CAP. Method: parameterized boundary tests plus a property test.
  Domain: lengths 0, 1, 65,535, 65,536, 65,537, 65,538, and an infinite
  stream (`std::io::repeat`); generated lengths weighted around the cap.
  Artefact: `src/source/tests.rs`, `tests/ingestion/properties/source.rs`.
  Evidence: `make test`; red first, because `PolicySource` does not exist.
  Non-vacuity: a counting reader records bytes consumed, and the infinite
  stream proves the bound without trusting length; a seeded fault that raises
  the read limit to 65,538 must fail the consumption test.
- INV-SRC-UTF8. Method: `rstest` table plus property test against
  `std::str::from_utf8` as oracle. Domain: truncated sequences, overlong
  encodings, encoded surrogates (`ED A0 80`), code points above U+10FFFF,
  stray continuation bytes, a multi-byte character split exactly at 65,536,
  and random byte vectors. Artefact: as above. Non-vacuity: generator
  classification shows both valid and invalid inputs on both sides of the cap;
  a seeded fault that skips validation must fail.
- INV-SRC-EXACT and INV-SRC-CHUNKING. Method: property test. Domain: random
  valid sources up to the cap, delivered by a reader that returns random chunk
  sizes and injects `Interrupted`. Oracle: `from_bytes` on the same bytes.
  Non-vacuity: the generator must produce at least one chunk of size 1 and one
  `Interrupted` per run (asserted); a seeded fault that drops a chunk must
  fail.
- INV-IR-WF. Method: `rstest` tables at 0, 1, 16, and 17 rules and at 0, 1,
  16, and 17 atoms, plus a property over constructor inputs. Artefact:
  `src/ir/tests.rs`. Non-vacuity: both acceptance and rejection occur at each
  bound.
- L-CANON-REFINES, L-CANON-INJECTIVE, L-CANON-SHAPE, L-CANON-LENGTH. Method:
  Verus proof over the shipped kernel. Domain: all policies (injectivity,
  shape) and all well-formed policies (refinement, length). Artefacts:
  `src/ir/canonical/mod.rs`, `verus/policy_canonical_spec.rs`,
  `verus/policy_canonical_lemmas.rs`, `verus/policy_canonical.rs`. Evidence:
  `make verus` reports 0 errors for every entry point and every control
  rejected. Non-vacuity: `verus/policy_canonical_witnesses.rs` proves
  `render` of the empty policy and of the baseline policy equal concrete
  literal byte sequences, which shows `WF` is inhabited and the specification
  is the intended one. Negative controls in `scripts/check_verus_controls.py`
  must each be rejected: (a) the kernel omits the comma between atoms, so
  refinement fails; (b) the kernel omits the final line feed, so refinement
  fails; (c) the specification gives `unlocked` the name `locked`, so
  injectivity fails; (d) the specification drops a body's brackets, so
  `[[a],[b]]` and `[[a,b]]` collide and injectivity fails. Control (c) and (d)
  show that the injectivity proof depends on the token set and the brackets,
  not on a vacuous premise. The new forbidden-construct scan must reject a
  scratch copy with an inserted `assume(false)`.
- Canonical bytes, independent oracle. Method: property and fixture tests that
  compare `to_canonical_bytes` with serde_json's compact serialization of a
  test-only struct with fields `profile` then `rules`, plus a line feed. This
  is independent of the encoder and checks the same `render` relation in
  ordinary tests, so a proof that verifies the wrong specification would still
  meet a failing test. Artefact: `tests/ingestion/properties/canonical.rs`,
  `tests/ingestion/fixtures.rs`, and an `insta` snapshot of the canonical
  bytes of every committed IR fixture.
- INV-ROUND-TRIP. Method: property test. Domain: well-formed policies across
  the classes empty policy, 16 rules, 16-atom bodies, every atom, repeated
  atoms, and repeated rules; layouts with random whitespace (space, tab, line
  feed, carriage return) between tokens, swapped key order, and atom names
  with random `\u` escapes. Oracle: the generated policy value. Non-vacuity: a
  deterministic coverage test asserts that a fixed-seed run reaches every class
  above.
- INV-DECODE-REJECTS. Method: `rstest` table with one named row per class, and
  property tests that mutate a valid document into each class. Classes:
  duplicate `profile`, duplicate `rules`, unknown key (with each JSON value
  kind), missing `profile`, missing `rules`, wrong profile string, profile of
  the wrong JSON type, unknown atom, atom of the wrong JSON type, empty body,
  17 rules, 17 atoms, top-level array or scalar, trailing content, truncation,
  leading BOM, invalid UTF-8 anywhere (including inside an unknown key's
  value), unpaired surrogate escapes, and a document over 65,536 bytes.
  Artefact: `src/ir/decode/tests.rs`, `tests/ingestion/properties/decode.rs`.
  Non-vacuity: every mutation starts from a document that decodes `Ok`, which
  the property asserts before mutating; seeded faults that disable each check
  must make its row fail.
- INV-CONTRACT. Method: unit tests that read `contracts/profile.json` and
  `contracts/policy.schema.json` through `cap-std` and compare. Artefact:
  `tests/ingestion/contracts.rs`. Non-vacuity: each comparison names the
  contract field it reads, and a test fails if a field is absent rather than
  defaulting.

### Formal verification decision

Verus is warranted for the canonical encoding and nowhere else in this item.
Canonical bytes are the identity that evidence binding (roadmap 4.1.1) and
re-derivation (4.1.3) hash and compare. If two policies shared canonical
bytes, a binding would silently cover a policy that was never checked; if the
encoder emitted bytes other than the specified form, independent tools could
not reproduce the binding. Both properties quantify over every policy and every
ordering, the encoder is a small pure kernel, and the token set makes a clean
inductive argument, which is the profile `rust-router` gives for Verus.

Kani adds nothing here: the input space is up to 6^256 orderings, too large for
meaningful bounds, and Verus covers it without a bound. Ingestion is a thin
composition of `std` primitives whose behaviour is an axiom; boundary tests
and properties with a counting reader test the repository-owned logic at its
real interface. The decoder's hard parts belong to serde_json, which is not
ours to verify (AXIOM-SERDE-JSON); its repository-owned logic is key
multiplicity and value validation, which named tests and mutation properties
cover. Type invariants hold by construction behind private fields, and
compile-checked constructors leave nothing for a prover to add. Diagnostics
formatting gets tests only.

The proof does not establish that decoding inverts encoding for the real
serde-based decoder; INV-ROUND-TRIP tests that. It does not establish
anything about Rego, admission, or the Lean model.

### Suggested BDD features

Adopted: `tests/ingestion/features/policy_ingestion.feature`, bound by
`tests/ingestion/bdd.rs` with `rstest-bdd` 0.6.0, following the slot-state
pattern in `tests/admission/bdd.rs`. It sits outside `tests/features/` so the
toolchain workflow's path filter does not run on every change to it. Run with
`cargo nextest run --test ingestion` or `make test`.

```gherkin
Feature: Policy sources and IR are ingested without losing their meaning

  Scenario: A source at the size cap is accepted unchanged
    Given a valid UTF-8 policy source of exactly 65536 bytes
    When the source is ingested from a stream
    Then ingestion succeeds
    And the ingested bytes equal the original bytes

  Scenario: A source one byte over the cap is rejected without reading on
    Given an unbounded stream of policy bytes
    When the source is ingested from the stream
    Then ingestion fails because the source is too large
    And no more than 65537 bytes were read from the stream

  Scenario: Malformed UTF-8 is rejected with its offset
    Given a policy source whose byte at offset 10 starts an invalid sequence
    When the source is ingested from bytes
    Then ingestion fails with invalid UTF-8 at offset 10

  Scenario: The committed baseline IR encodes to canonical bytes
    Given the committed IR fixture "examples/tenant-write/policy.json"
    When the fixture is decoded and encoded canonically
    Then the canonical bytes are compact, keyed profile then rules, and end with one line feed
    And the rules appear in their committed order

  Scenario: A policy with zero rules is valid
    Given the committed IR fixture "examples/tenant-write/mutations/deny-all/policy.json"
    When the fixture is decoded and encoded canonically
    Then the canonical bytes end with "\"rules\":[]}" and one line feed

  Scenario Outline: Malformed IR documents are rejected
    Given the baseline IR document altered by "<alteration>"
    When the document is decoded
    Then decoding fails with "<error>"

    Examples:
      | alteration          | error         |
      | empty first body    | empty body    |
      | duplicate profile   | duplicate key |
      | added field comment | unknown field |
      | unknown atom        | unknown atom  |
```

Scenarios map to EP-M1 (the first three) and EP-M2 (the rest), and to
REQ-SRC-CAP, REQ-SRC-UTF8, REQ-SRC-EXACT, REQ-CANON-FORM, REQ-IR-BOUNDS, and
REQ-DECODE-STRICT. The feature file is added in its milestone before the step
bindings, so its first run fails.

## Plan of work

The work is four milestones. Each ends with `make check-fmt`, `make lint`, and
`make test` passing, plus the milestone's extra gates, and is committed in
small steps: red test, green implementation, refactor.

### `EP-M1` profile constants and bounded source ingestion

Add `src/profile.rs` with `PROFILE_ID`, `SOURCE_BYTES_MAX` (65,536),
`RULES_MAX` (16), `BODY_ATOMS_MIN` (1), and `BODY_ATOMS_MAX` (16), each
documented with its contract field. Add `src/source/mod.rs` with
`PolicySource`, `SourceError`, `PolicySource::from_bytes`, and
`PolicySource::read_from`, and a crate-private `read_bounded(reader, limit)`
that `read_from` uses and 2.1.3 will reuse. Implement `Display` and
`std::error::Error` for `SourceError` by hand; `Read` carries the
`std::io::Error` as its `source()`. Declare both modules in `src/lib.rs` and
update its crate documentation to say what now exists, keeping `STATUS` and
`project_name` so `tests/stub.rs` stays valid.

Red: add `src/source/tests.rs` with the boundary table, the UTF-8 table, the
infinite-stream test, and the chunking reader; add
`tests/ingestion/main.rs`, `tests/ingestion/support.rs`,
`tests/ingestion/contracts.rs` (profile constants only), and the first three
scenarios of the feature file. Run
`cargo nextest run --lib source --test ingestion` and record the failure
(unresolved `PolicySource`). Green: implement until they pass. Refactor: keep
`read_bounded` under 30 lines and the module under 400.

Add `tests/ingestion/properties/source.rs` for INV-SRC-UTF8, INV-SRC-EXACT,
and INV-SRC-CHUNKING.

Acceptance: `make check-fmt`, `make lint`, `make test` pass; the new tests ran
red before the implementation and green after.

### `EP-M2` closed IR types, canonical encoder, and strict decoder

If Q1 is accepted, move `vstd` to `[dependencies]`. Move `serde` (with
`derive`) and `serde_json` to `[dependencies]`.

Add `src/ir/mod.rs` with `Atom` (six unit variants in profile order, with
`Atom::ALL` and `Atom::name`), `Rule` (private `Vec<Atom>`, built by
`Rule::new`, exposing `atoms() -> &[Atom]`), `Policy` (private `Vec<Rule>`,
built by `Policy::new`, exposing `rules() -> &[Rule]` and `profile()`), and
`IrError` for constructor failures. `Atom` is declared inside the kernel's
`verus!` block so Verus can reason about it; `src/ir/mod.rs` re-exports it.
Neither enum is `#[non_exhaustive]`: the IR is closed, and adding an atom must
break every exhaustive match.

Add `src/ir/canonical/mod.rs`, the kernel: inside `verus!`, `Atom`,
`Atom::name` with an `ensures` naming the spec, and `encode` over the rule
sequence, written so the EP-M3 loop invariants can be added without changing
executable statements. `Policy::to_canonical_bytes` calls it. Gate ghost-only
imports on `#[cfg(verus_keep_ghost)]` as `tests/admission/recognizer/kernel/`
does. In this milestone the kernel has only executable code and empty
`requires` and `ensures`; the specification modules are added in EP-M3.

Add `src/ir/decode/mod.rs` with `Policy::from_json_slice(&[u8])` and
`DecodeError`: size check, `std::str::from_utf8`, then `serde_json::from_str`
into a private `WireDocument` whose hand-written visitor collects
`(Key, value)` entries in document order (`Key` is `Profile`, `Rules`, or
`Unknown`; unknown values are read as `serde::de::IgnoredAny`), then plain
Rust validation in the D8 precedence. `DecodeError` variants: `TooLarge`,
`InvalidUtf8 { valid_up_to }`, `Json { class, line, column }` with a closed
`JsonErrorClass` (`Syntax`, `Eof`, `Data`), `DuplicateKey(Key)`,
`UnknownField`, `MissingField(Key)`, `WrongProfile`, `TooManyRules { count }`,
`EmptyBody { rule }`, `TooManyAtoms { rule, count }`, and
`UnknownAtom { rule, position }`. No variant echoes an unknown key or value,
so diagnostics cannot carry attacker-chosen text.

Red: add `src/ir/tests.rs`, `src/ir/decode/tests.rs`,
`tests/ingestion/fixtures.rs`, `tests/ingestion/properties/canonical.rs`,
`tests/ingestion/properties/decode.rs`, the IR half of
`tests/ingestion/contracts.rs`, and the remaining scenarios; run them and
record the failures. Green: implement. Refactor: split modules that approach
400 lines. Review and accept the `insta` snapshot of canonical fixture bytes
with `cargo insta review`.

Acceptance: `make check-fmt`, `make lint`, `make test`, and
`make toolchain-smoke` pass (`Cargo.toml` changed). Every committed IR fixture
decodes, and its canonical bytes match the serde_json oracle and the reviewed
snapshot.

### `EP-M3` Verus proof of the canonical encoding

Add `verus/policy_canonical_spec.rs` (the `render` specification and `WF`),
`verus/policy_canonical_lemmas.rs` (the lemmas in the proof sketch),
`verus/policy_canonical_witnesses.rs` (the two concrete witnesses), and
`verus/policy_canonical.rs` (the entry point that `#[path]`-includes
`src/ir/canonical/mod.rs`). The kernel `#[path]`-includes the spec and lemma
files, as the recognizer kernel does. Add the `requires`, `ensures`, and loop
invariants to the kernel.

Add `verus/policy_canonical.rs` to `VERUS_PROOF_FILES` in the `Makefile`. Add
a `ProofTree` and the four controls to `scripts/check_verus_controls.py`, and
add a forbidden-construct scan that fails any tree whose sources contain
`assume(`, `admit(`, `external_body`, `external_fn_specification`, or
`verifier::external`. Update `scripts/tests/test_verus_make_targets.py` for
the new entry point, and add a test that the Makefile's list and the script's
trees name the same entry points. Add `src/ir/canonical/**` to both path
filters in `.github/workflows/toolchain.yml`, so `make test-scripts` runs when
the kernel changes. Document the proof's statement and boundaries in
`tools/verus/README.md`.

Red: before the lemmas exist, `make verus` must fail on the new entry point
with unproved postconditions; record the output. Green: prove. The forbidden
scan must reject a scratch tree with an inserted `assume(false)` in a test.

Acceptance: `make install-verus` (once) and `make verus` exit 0, with every
entry point reporting `0 errors`, the recognizer and settlement counts
unchanged, and every control, old and new, rejected at its named obligation.
Then `make test-scripts`, `make check-fmt`, `make lint`, and `make test` pass,
and `make toolchain-smoke` passes because `scripts/` changed.

### `EP-M4` documentation, ADR 0007, and delivery closure

Write `docs/adrs/adr-0007-verified-canonical-ir-encoding-and-production-
dependencies.md` (final slug chosen at writing time): the decision to verify
the canonical encoder in production code, the production dependencies and why
each is acceptable, the decoder's acceptance rules (D3, D6–D9), and the
proposed outcome mapping (D13). Update `docs/technical-design.md`: §6 to
record the implemented module layout, §6.1 to point to the IR types, §7 to
state D3, D6, D7, and the Verus theorem with its boundary, and §10 to note what
the canonical-encoding proof adds beside V7 and V9. Update
`docs/developers-guide.md`: replace "The Rust scaffold" with the production
modules, describe the shared-kernel pattern for production code, the
bounded-reader re-use strategy for 2.1.3, test placement, and the Verus
wiring checklist. Update `docs/users-guide.md` "Available today" with a prose
description of the library (D12). Update `docs/repository-layout.md` (the
`src/` rows and the `verus/` row), `docs/contents.md` (this plan and ADR 0007),
and `docs/context.md` (glossary entries for _policy source_ and _canonical IR
bytes_). Refresh `SHA256SUMS` for changed covered files. Mark roadmap 2.1.1
done. Search for stale claims with
`grep -rn "scaffold\|not implemented" README.md docs --include=*.md`,
excluding `docs/execplans/`.

Acceptance: `make check-fmt`, `make lint`, `make test`, `make markdownlint`,
`make design-check`, and `sha256sum -c SHA256SUMS` pass.

## Milestones and plateaus

### `EP-M1`

- Outcome: `polean::source` ingests a bounded, UTF-8-checked policy source;
  `polean::profile` exposes the contract's constants.
- Requirements: REQ-SRC-CAP, REQ-SRC-UTF8, REQ-SRC-EXACT, part of
  REQ-CONTRACT-AGREE; GAP-NO-CAP.
- Acceptance: boundary, UTF-8, chunking, and contract tests; three BDD
  scenarios; gates green.
- Conformance check:
  - No filesystem, environment, or printing in `src/`.
  - No dependency change.
  - Bytes preserved exactly (D4).
- Recovery: revert the milestone's commits; nothing depends on them.
- Remaining gaps: no IR yet.
- Compatibility decision: none; new pre-1.0 API.

### `EP-M2`

- Outcome: `polean::ir` provides closed types, a tested canonical encoder, and
  a strict decoder.
- Requirements: REQ-IR-CLOSED, REQ-IR-BOUNDS, REQ-IR-ORDER, REQ-CANON-FORM
  (tested), REQ-DECODE-STRICT, REQ-CONTRACT-AGREE; GAP-NO-PROD, GAP-NO-CANON.
- Acceptance: unit, property, fixture, snapshot, and BDD tests; gates and
  toolchain smoke green.
- Conformance check:
  - Only the authorized dependencies moved.
  - No spans or strings in the IR.
  - `contracts/` and `examples/` unchanged.
- Recovery: revert to EP-M1.
- Remaining gaps: the encoder is tested but not proved.
- Compatibility decision: none.

### `EP-M3`

- Outcome: L-CANON-REFINES, L-CANON-INJECTIVE, L-CANON-SHAPE, and
  L-CANON-LENGTH are proved over the shipped kernel; forbidden constructs are
  rejected mechanically.
- Requirements: REQ-CANON-FORM, REQ-CANON-UNIQUE; GAP-VERUS-SCAN.
- Acceptance: `make verus` with 0 errors and all controls rejected;
  `make test-scripts`; gates green.
- Conformance check:
  - No trusted proof body or new axiom.
  - Executable kernel statements unchanged from EP-M2 apart from ghost code.
  - Existing proofs' verified counts unchanged.
  - `make verus` still on demand.
- Recovery: the encoder's behaviour is unchanged, so reverting the proof
  leaves EP-M2 valid. If the verification tolerance is reached, stop and
  present the open condition.
- Remaining gaps: the serde-based decoder is tested, not proved.
- Compatibility decision: none.

### `EP-M4`

- Outcome: durable documents describe what exists; roadmap 2.1.1 is done.
- Requirements: all, by documentation; RM-2.1.1.
- Acceptance: documentation gates and checksum manifest pass.
- Conformance check:
  - No document claims admission, proof of a policy, or runtime agreement.
  - ADR 0007 records every dependency decision.
- Recovery: documentation-only; revert or amend.
- Remaining gaps: admission (2.1.2), requests (2.1.3), CLI (2.1.4).
- Compatibility decision: none.

## Concrete steps

Run from the worktree root. Each gate writes a log for later reading:

```bash
BRANCH=2-1-1-bounded-source-ingestion-and-closed-policy-ir-types
make check-fmt 2>&1 | tee /tmp/check-fmt-polean-$BRANCH.out
make lint 2>&1 | tee /tmp/lint-polean-$BRANCH.out
make test 2>&1 | tee /tmp/test-polean-$BRANCH.out
make test-scripts 2>&1 | tee /tmp/test-scripts-polean-$BRANCH.out
make toolchain-smoke 2>&1 | tee /tmp/toolchain-smoke-polean-$BRANCH.out
make verus 2>&1 | tee /tmp/verus-polean-$BRANCH.out
```

Focused loops during red and green:

```bash
cargo nextest run --lib source
cargo nextest run --lib ir
cargo nextest run --test ingestion
cargo insta review
```

Expected `make verus` tail at EP-M3 (counts are illustrative until the proof
exists):

```plaintext
verification results:: N verified, 0 errors   (policy_canonical)
control kernel-drops-atom-comma: rejected as expected
control kernel-drops-final-lf: rejected as expected
control spec-aliases-unlocked: rejected as expected
control spec-drops-body-brackets: rejected as expected
```

## Validation and acceptance

The work is accepted when:

- `PolicySource::read_from` over `std::io::repeat(b'a')` returns `TooLarge`
  after consuming at most 65,537 bytes, and over exactly 65,536 valid bytes
  returns those bytes unchanged.
- `Policy::from_json_slice` on every committed IR fixture returns the fixture's
  rules in order, and `to_canonical_bytes` returns the reviewed snapshot,
  equal to serde_json's compact rendering plus one line feed.
- Each REQ-DECODE-STRICT class yields its named `DecodeError` variant.
- `make verus` proves the four canonical-encoding lemmas with no forbidden
  construct, and all controls are rejected.
- `make check-fmt`, `make lint`, `make test`, `make test-scripts`,
  `make toolchain-smoke`, `make design-check`, `make markdownlint`, and
  `sha256sum -c SHA256SUMS` pass.

## Idempotence and recovery

Every step is re-runnable. Tests read committed fixtures and write nothing
outside Cargo's target directory and `insta`'s pending snapshots. The Verus
controls run on scratch copies and never touch the working tree. If a gate
fails part-way, fix the cause and re-run that gate only, then the remaining
gates in order. Each milestone is committed, so `git revert` of a milestone's
commits restores the previous plateau.

## Artefacts and notes

Reconnaissance evidence for the serde decisions, from the locked sources:

```plaintext
serde_derive-1.0.229/src/de/struct_.rs:269  duplicate_field(...) in derived struct visitors
serde_json-1.0.154/src/read.rs:951-1010     LoneLeadingSurrogateInHexEscape when validating strings
serde_json-1.0.154/src/de.rs:63             remaining_depth: 128
```

Prior art: Vest (Cai et al., USENIX Security 2025) proves parser and
serializer round trips in Verus over combinators; this plan borrows only its
proof shape (a specification-level parser as a left inverse), not the library.

## Interfaces and dependencies

Public items at the end of this plan, all new and pre-1.0:

```rust
pub mod profile {
    pub const PROFILE_ID: &str = "polean.tenant-write.v0";
    pub const SOURCE_BYTES_MAX: usize = 65_536;
    pub const RULES_MAX: usize = 16;
    pub const BODY_ATOMS_MIN: usize = 1;
    pub const BODY_ATOMS_MAX: usize = 16;
}

pub mod source {
    pub struct PolicySource { /* private */ }
    impl PolicySource {
        pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, SourceError>;
        pub fn read_from(reader: impl std::io::Read) -> Result<Self, SourceError>;
        pub fn as_bytes(&self) -> &[u8];
        pub fn as_str(&self) -> &str;
    }
    pub enum SourceError {
        TooLarge { limit: usize },
        InvalidUtf8 { valid_up_to: usize },
        Read(std::io::Error),
    }
}

pub mod ir {
    pub enum Atom { SameTenant, WriteAction, AdminRole, OwnsResource, Locked, Unlocked }
    impl Atom {
        pub const ALL: [Atom; 6];
        pub const fn name(self) -> &'static str;
    }
    pub struct Rule { /* private */ }
    impl Rule {
        pub fn new(atoms: Vec<Atom>) -> Result<Self, IrError>;
        pub fn atoms(&self) -> &[Atom];
    }
    pub struct Policy { /* private */ }
    impl Policy {
        pub fn new(rules: Vec<Rule>) -> Result<Self, IrError>;
        pub fn rules(&self) -> &[Rule];
        pub const fn profile(&self) -> &'static str;
        pub fn to_canonical_bytes(&self) -> Vec<u8>;
        pub fn from_json_slice(bytes: &[u8]) -> Result<Self, DecodeError>;
    }
    pub enum IrError { EmptyBody, TooManyAtoms { count: usize }, TooManyRules { count: usize } }
    pub enum DecodeError { /* variants listed in EP-M2 */ }
    pub enum JsonErrorClass { Syntax, Eof, Data }
    pub enum Key { Profile, Rules }
}
```

Each type derives `Debug`; value types derive `Clone`, `PartialEq`, `Eq`, and
`Hash`; `Atom` also derives `Copy`, `PartialOrd`, and `Ord`. Error types
implement `Display` and `std::error::Error` by hand. Exact signatures may
change during implementation if a lint or Verus requires it, provided no item
is added beyond this list; such a change is recorded in `Decision log`.

Crate-private: `source::read_bounded(reader, limit) -> Result<Vec<u8>,
SourceError>`, reused by 2.1.3's request decoder.

Dependencies after this plan: `serde =1.0.229` (`derive`), `serde_json
=1.0.154`, and, if Q1 is accepted, `vstd =0.0.0-2026-09-06-0133`, all moved
from `[dev-dependencies]` without version changes.

## Revision notes

- Revision 1 (2026-11-14): initial draft from reconnaissance; awaiting design
  review and the user's decision on Q1.
