# Implement bounded source ingestion and closed policy IR types

This ExecPlan (execution plan) is a living document. Keep `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan`
current as work proceeds.

Status: DRAFT (revision 2). The community-of-experts review has been applied.
Awaiting the user's decisions on Q1 and Q2 and approval of the plan.
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
non-empty, ordered lists of atoms, and nothing else. The Lean model, the
evidence bundle, and every later roadmap item consume the IR rather than Rego.

Today the crate under `src/` is a scaffold. No production code reads a policy,
bounds its size, checks its encoding, or represents the IR. The 1.2.2 admission
spike under `tests/admission/` used a throwaway `Vec<Vec<String>>` and read
sources without a cap, and its design record says that type must not be
promoted. The committed IR fixtures, such as
`examples/tenant-write/policy.json`, are pretty-printed JSON. Nothing produces
the compact _canonical bytes_ that technical design §7 says evidence must bind.

After this work, a library caller can do four things, each with a closed, typed
failure:

1. Ingest a policy source from any `std::io::Read` or byte vector. Ingestion
   reads at most 65,537 bytes and rejects anything longer than 65,536 bytes
   without consuming the rest of the stream. It rejects malformed UTF-8 with
   the offset of the first bad byte, and a NUL byte with its offset. Otherwise
   it returns the exact bytes, unchanged, as a `PolicySource`.
2. Build a `Policy` from atoms. The constructors make an empty rule body, a
   17th rule, or a 17th atom unrepresentable, and an atom can only be one of
   six enum values.
3. Decode a `Policy` from JSON. The decoder accepts the committed
   pretty-printed fixtures. It rejects duplicate keys, unknown fields, missing
   fields, values of the wrong JSON type, escaped spellings, a different
   profile, unknown atoms, empty bodies, and every bound violation, each with
   its own error variant.
4. Encode a `Policy` to canonical bytes,
   `{"profile":"polean.tenant-write.v0","rules":[...]}` followed by one line
   feed. The bytes contain no other whitespace and preserve rule and atom
   order; zero rules encode as `"rules":[]`. A strict reader accepts exactly
   those bytes and nothing else. A Verus proof establishes two facts: the
   shipped encoder produces exactly the specified bytes, and distinct policies
   never share canonical bytes. Hashing canonical bytes, which roadmap 4.1.1
   will do, therefore identifies exactly one policy.

For example, the baseline fixture's canonical bytes are:

```plaintext
{"profile":"polean.tenant-write.v0","rules":[["same_tenant","write_action","admin_role"],["same_tenant","write_action","owns_resource","unlocked"]]}
```

followed by a single line feed.

None of this admits Rego. Turning source into IR is roadmap 2.1.2, which will
consume `PolicySource` and produce `Policy`. Nothing here proves any of the
following: that a source belongs to the profile, that a model claim holds, that
translation is correct, or that a runtime agrees with the model. The Verus
theorem concerns the byte encoding of IR values only.

### Terms used in this plan

- _Kernel_: a small module of executable Rust written inside a `verus! { ... }`
  block. Cargo compiles it normally; Verus verifies the same file.
- _Ghost code_: specifications, proof functions, `requires`, `ensures`, and
  loop invariants. Verus checks them and plain `rustc` erases them, so they
  cost nothing at run time.
- _Refinement_: the property that an executable function returns exactly what
  its specification says.
- _Injective_: a function is injective when distinct inputs always give
  distinct outputs.
- _Prefix-free_: a set of byte strings is prefix-free when no member is the
  beginning of another member.
- _Left inverse_: a function `parse` is a left inverse of `render` when
  `parse(render(x)) = x` for every `x`.
- _Witness_: a concrete value that satisfies a precondition. It shows the
  precondition is not impossible to meet, so a proof under that precondition is
  not vacuous.
- _Negative control_: a deliberately broken copy of the code or specification
  that the verifier or test must reject. Its rejection shows the check can fail.
- _Plateau_: a milestone end state that is coherent, passes every gate, and is
  safe to stop at.

## Constraints

- Keep the four claims in `AGENTS.md` separate. No type name, error message,
  test name, or document may describe ingestion, decoding, or canonical
  encoding as admission, proof of a policy, translation, or runtime agreement.
  The Verus theorem is stated as a property of the canonical byte encoding.
- Production code lives in one Rust package under `src/`, in the modules named
  in `Interfaces and dependencies`. `self_named_module_files` is denied, so
  directories use `mod.rs`.
- The IR holds only the six atoms in rule order. No byte spans, source text,
  comments, or Regorus types may enter it (technical design §6.1); 2.1.2 owns
  the diagnostic sidecar.
- `Atom`, `Rule`, `Policy`, and `CanonicalIr` must not implement serde's
  `Serialize` or `Deserialize`. The only ways to obtain a `Policy` are its
  constructors and the two decoders, so no derive can bypass the bounds. A
  `compile_fail` doctest guards this.
- Unsupported or malformed data fails closed. There is no lenient mode, no
  coercion, no last-key-wins, no truncation, and no ignored field.
- Library code must not print, exit, read environment variables, touch the
  filesystem, or depend on OrthoConfig (technical design §8; Whitaker's
  `no_std_fs_operations`; `clippy.toml` `disallowed-methods`). Ingestion takes
  `impl Read`; opening files is the CLI's job in 2.1.4.
- No diagnostic may carry attacker-chosen text. `DecodeError` holds no
  `serde_json::Error`, message, key, or value, and has no `source()`.
- Do not change `contracts/`, `examples/`, `validation/`, or `lean/`. The
  pretty-printed fixtures remain the decoder's inputs; this plan does not
  rewrite them into canonical form.
- Do not change the admission spike in `tests/admission/`. Its throwaway IR
  remains test-only, and 2.1.2 retires it.
- Authorized dependency changes are exactly these, all at the versions already
  in `Cargo.lock`: move `serde` (`=1.0.229`, feature `derive`) and `serde_json`
  (`=1.0.154`) to `[dependencies]`; and, if the user accepts Q1, move `vstd`
  (`=0.0.0-2026-09-06-0133`) to `[dependencies]`. Each stays in
  `[dev-dependencies]` only if still needed there. Regorus stays a
  dev-dependency until 2.1.2.
- Verus proofs must not use any construct the forbidden-construct scan
  rejects, listed under `EP-M3`. That includes `assume`, `admit`,
  `assume_specification`, `assume_new`, `external_body` and its relatives,
  `exec_allows_no_decreases_clause`, `axiom fn`, and
  `cfg(not(verus_keep_ghost))`. Unproved statements may live in this plan, not
  in code. `make verus` acceptance runs with `VERUS_FLAGS` unset.
- Follow the lint policy: no `unsafe`; no `allow` attributes (a scoped
  `#[expect(..., reason = "...")]` is permitted); every module starts with
  `//!`; modules stay under 400 lines and functions under 70 lines; public
  items carry Rustdoc with an example where useful.
- `make verus` stays on demand, outside `make all`, unless the user accepts
  Q2.
- Use British English with Oxford spelling. Refresh `SHA256SUMS` only for
  covered files this plan changes (`docs/technical-design.md`, and
  `docs/context.md`), after reviewing each change.
- Do not bypass compile admission (`RUSTC_WRAPPER`), do not create an isolated
  Cargo cache, and do not use `/tmp` as a build target.
- Implementation begins only after the user approves this plan.

## Tolerances (exception triggers)

Exclude this ExecPlan from every count, including when it is updated in the
same change. Exclude reviewed `insta` snapshots (`**/*.snap`), proptest
regression files (`**/*.proptest-regressions`), and the generated proof-input
digest file (`verus/verified-inputs.sha256`).

Measure net lines (additions minus deletions) per category against the merge
base with `main` at `ad217c8`:

```bash
git diff --numstat ad217c8...HEAD -- . \
  ':(exclude)docs/execplans/**' ':(exclude)**/*.snap' \
  ':(exclude)**/*.proptest-regressions' \
  ':(exclude)verus/verified-inputs.sha256'
```

Classify whole files by path:

- Code: `src/**`, `tests/**` (including `.feature` files), `verus/**`,
  `scripts/**`, and `Makefile`.
- Configuration: `Cargo.toml`, `Cargo.lock`, `.github/**`, `.config/**`, and
  `SHA256SUMS`.
- Documentation: `docs/**` except ExecPlans, `README.md`, and
  `tools/verus/README.md`.

Limits, each enforced independently. Record a `numstat` measurement in
`Progress` at the end of every milestone.

- Code, overall: stop and escalate above 8,000 net lines.
- Code, per milestone: stop and escalate if EP-M1 exceeds 1,400, EP-M2
  exceeds 3,400, EP-M3 exceeds 2,600 (including the tooling commit), or EP-M4
  exceeds 200.
- Configuration: stop and escalate above 100 net lines.
- Documentation: stop and escalate above 1,300 net lines.
- Files: stop and escalate above 55 changed non-ExecPlan files, excluding
  snapshots.
- Interface: stop if a public item beyond those named in
  `Interfaces and dependencies` is needed, or if `contracts/` would have to
  change.
- Dependencies: stop if any dependency other than those authorized in
  `Constraints` is needed, or if a pinned version would have to change.
- Feasibility: stop at the end of the EP-M2 feasibility step if the kernel
  cannot declare `Atom`, `Rule`, and `Policy` inside `verus!` and also pass
  `make lint` as library code without an `allow`. Present the obstacle and the
  fallback in Q1.
- Verification: stop if a Verus theorem in `Verification plan` cannot be
  proved without a forbidden construct, or can only be proved in a weaker form.
  Present the open verification condition instead.
- Design: stop and set `BLOCKED` if the decoder cannot distinguish a required
  rejection class, or if technical design §7 turns out to require behaviour
  this plan contradicts.
- Iterations: stop when a gate still fails after three fix attempts on one
  cause.
- Ambiguity: stop on a newly found ambiguity whose readings would accept
  different inputs.

The estimate behind these limits is in `Context and orientation`.

## Risks

- Risk: the Verus proof of canonical uniqueness is harder than estimated,
  because it reasons about concatenations of nested sequences. Severity:
  medium. Likelihood: medium. Mitigation: the proof sketch reduces the problem
  to a specification-level left-inverse parser over a prefix-free token set,
  which is the shape Vest-style serializer proofs use. EP-M2 delivers a fully
  tested encoder, so EP-M3 can stop without losing tested behaviour.
- Risk: production code inside `verus!` fails the library lint policy.
  `missing_docs`, `must_use_candidate`, and rustdoc's `-D warnings` apply to
  exported items, which the 1.2.2 test-only kernel never had. Deriving anything
  beyond `Clone`, `Copy`, `Debug`, `PartialEq`, and `Eq` inside `verus!` has no
  precedent, and neither do string or byte-string literals there. Severity:
  medium. Likelihood: medium. Mitigation: EP-M2 opens with a feasibility step
  that lints the skeleton as library code and runs Verus on it. Atom names are
  emitted through constant byte arrays, and the `&'static str` accessor sits
  outside `verus!`.
- Risk: serde_json reports type faults when it meets them, so a single typed
  pass cannot give a stable precedence or a variant per fault. Severity:
  medium. Likelihood: certain for a single-pass design. Mitigation: D8's
  two-pass decoder settles syntax first. The typed pass then classifies values
  through repository visitors that never raise serde errors.
- Risk: the proof quietly stops covering the shipped encoder. The kernel moves
  into production `src/`, which later items will edit, while `make verus` is
  not a commit gate. Severity: high. Likelihood: medium. Mitigation: a digest
  manifest of every proof input is written only by a successful `make verus`.
  `make test-scripts`, which CI runs when those paths change, fails if any
  input differs from the manifest. Q2 asks whether a CI job should also run
  `make verus`.
- Risk: canonical bytes diverge from what the Python harness and Lean will
  use. Severity: medium. Likelihood: low. Mitigation: a test encodes every
  committed IR fixture and compares it with an independent serde_json compact
  rendering. An `insta` snapshot records the canonical bytes for review.
- Risk: making `vstd` a production dependency adds pre-release crates to the
  shipped dependency graph. Severity: medium. Likelihood: certain if Q1 is
  accepted. Mitigation: ADR 0007 records the decision and amends ADR 0005's
  "dev-only" consequence. `verus!` erases ghost code under plain `rustc`, and
  every test run already builds the crates.
- Risk: a reader misbehaves. It may return `Ok(0)` before the true end,
  return `Interrupted` forever, report more bytes than it was given, or fail
  after a partial read. Severity: medium. Likelihood: low. Mitigation: a
  failure after a partial read is a tested `Read` error, never a truncated
  success. Rustdoc documents the other three as the caller's contract: `Ok(0)`
  is end of stream, `Interrupted` is retried, and a lying reader makes `std`
  panic. 2.1.4 compares the length read with the file's metadata length.
- Risk: estimates are low again. Severity: medium. Likelihood: medium; both
  previous plans overran their scope tolerances by two to three and a half
  times. Mitigation: per-milestone budgets with measurements at each plateau; a
  breach stops work.

## Progress

- [x] (2026-11-14) Renamed the branch to
  `2-1-1-bounded-source-ingestion-and-closed-policy-ir-types`; the remote
  branch does not exist yet, so tracking is set at the first push.
- [x] (2026-11-14) Wyvern reconnaissance: admission spike and IR fixtures;
  Verus wiring and gates; prior ExecPlans and governing documents; contracts,
  outcomes, and downstream consumers.
- [x] (2026-11-14) Checked serde and serde_json behaviour in the locked
  sources, and Vest as prior art for verified serializers.
- [x] (2026-11-14) Drafted revision 1 (commit `ec9d2ad`).
- [x] (2026-11-14) Community-of-experts review: API and contracts, formal
  methods and alternatives, red team and pre-mortem, and proportionality and
  process. Verdict: revise. Revision 2 adopts the conditions; see
  `Revision notes`.
- [ ] User decisions on Q1 and Q2, and user approval of the plan.
- [ ] `EP-M1`: profile constants and bounded source ingestion.
- [ ] `EP-M2`: closed IR types, canonical encoder, and strict decoders.
- [ ] `EP-M3`: proof tooling, then the Verus proof of the canonical encoding.
- [ ] `EP-M4`: documentation, ADR 0007, and delivery closure.

## Surprises & discoveries

- Observation: the committed IR fixtures are pretty-printed, not canonical.
  Evidence: `examples/tenant-write/policy.json` uses two-space indentation;
  `validation/validate_design.py` writes JSON with `json.dumps(indent=2)`.
  Impact: the general decoder must accept any JSON whitespace; canonical form
  is a property of the encoder's output and of the strict reader (D3).
- Observation: no code reads `source_bytes` or `input_bytes` from
  `contracts/profile.json`, and nothing tests the 65,536-byte boundary.
  Evidence: repository-wide search. Impact: EP-M1 adds the first enforcement
  and a contract-drift test.
- Observation: `scripts/check_verus_controls.py` checks that each proof
  verifies and that each negative control is rejected, but it does not scan
  proof sources for forbidden constructs. Its `expects` substrings are matched
  over the whole output, so they cannot attribute a rejection to one
  obligation. Evidence: the script's `Control` docstring
  (`scripts/check_verus_controls.py:93-97`). Impact: EP-M3 adds a scan and
  isolates the injectivity lemmas in their own file, so a control can name it.
- Observation: registering a Verus entry point takes three uncoordinated
  edits: `VERUS_PROOF_FILES` in the `Makefile`, a `ProofTree` in the script,
  and a hard-coded list at `scripts/tests/test_verus_make_targets.py:200`.
  Impact: EP-M3 updates all three and adds an agreement test.
- Observation: serde_json 1.0.154 does not validate UTF-8 or surrogate escapes
  inside values it ignores, and its ignorer does not limit nesting depth.
  Evidence: `serde_json-1.0.154/src/read.rs:1076-1093`, `src/de.rs:1102-1214`.
  Impact: the decoder validates the whole document as UTF-8 first (D6). Tests
  place surrogate faults in keys, the profile, and atoms, and use a deeply
  nested unknown value to show that nesting is bounded by the size cap.
- Observation: serde_json stops at the first wrong-type value without reading
  the rest of the document, so syntax and type faults surface in document
  order. Evidence: `serde_json-1.0.154/src/de.rs:269`. Impact: D8 uses two
  passes.
- Observation: `docs/references.md`, named in the task's required reading,
  does not exist. The technical design's `References` section (R1–R18) serves
  that role. Impact: none; signposted below.
- Observation: technical design §6 names `src/frontend.rs`, not
  `src/source/`. Impact: D1 records the departure, and EP-M4 updates §6.

## Decision log

- D1. Decision: put production code in `src/profile.rs`, `src/bounded.rs`,
  `src/source/`, and `src/ir/`. Technical design §6 lists
  `src/{cli,profile,frontend,ir,...}.rs`. `profile` and `ir` match it. `source`
  is a deliberate departure: ingestion is not the frontend, which is 2.1.2's
  Regorus adapter, and naming it `frontend` would blur the boundary between
  ingestion and admission. `bounded` holds the size-capped reader that `source`
  uses now and 2.1.3's request decoder will use. EP-M4 updates §6. Date/Author:
  2026-11-14, planning agent, revised after review.
- D2. Decision: ingestion accepts at most 65,536 bytes inclusive and reads at
  most 65,537 bytes from a stream. It checks size, then UTF-8, then NUL.
  Rationale: `profile.json` sets `source_bytes` to 65,536, and the design says
  decoding rejects "larger" documents, so 65,536 bytes is accepted. Reading one
  byte past the cap is the least work that distinguishes "at the cap" from
  "over it" without trusting a length hint. The bound holds at the `Read`
  interface; a buffered reader beneath it may read further ahead. Checking size
  first means an over-long stream is never decoded, and a multi-byte character
  split by the cap reports `TooLarge`, not an encoding error. Date/Author:
  2026-11-14, planning agent.
- D3. Decision: provide two decoders. `Policy::from_json_slice` accepts any
  JSON whitespace and either key order. `Policy::from_canonical_bytes` accepts
  only canonical bytes and returns `NonCanonical` for any other valid encoding.
  Evidence paths (roadmap 4.1.x) must use the strict reader. Rationale: the
  committed fixtures are pretty-printed, so a canonical-only decoder cannot
  read them. A lenient decoder at an evidence boundary would let one policy
  have many bundle digests, which is pre-mortem scenario B. The strict reader
  decodes, then compares the input with the policy's canonical bytes. Equality
  accepts exactly the canonical encodings of well-formed policies, because
  L-CANON-REFINES says the encoder emits `render`. Every such encoding is
  accepted, by INV-ROUND-TRIP. Date/Author: 2026-11-14, planning agent, revised
  after review.
- D4. Decision: ingestion preserves bytes exactly. It does not strip a
  byte-order mark (BOM) or normalize line endings. It rejects NUL. Rationale:
  NUL is the one fail-open divergence in the admission feasibility record.
  Regorus's lexer treats NUL as end of input and would admit the prefix, while
  a hash would cover the whole source. Today only the throwaway spike's
  recognizer refuses it. No admitted source can contain NUL, and rejecting it
  does not alter the bytes kept. A BOM and a bare carriage return already fail
  closed in Regorus and in the recognizer respectively, and rejecting them here
  would duplicate admission rules without their reason codes. ADR 0007 records,
  as an entry requirement for 2.1.2, that the feasibility corpus's BOM, NUL,
  and bare carriage return rows must stay rejected. Date/Author: 2026-11-14,
  planning agent, revised after review.
- D5. Decision: decode JSON with serde_json, but classify every policy-level
  fault in repository code. The top-level visitor records every key in document
  order. Value visitors return classification enums (for example, "a string
  without escapes", "an escaped string", "not a string") rather than serde
  errors, consuming unexpected values with `IgnoredAny`. Only `deserialize_map`
  is used, never `visit_seq`, so a JSON array cannot masquerade as the object.
  Rationale: closed error variants and a stable precedence need classification
  in our code. serde_json still owns JSON syntax, escapes, unpaired surrogates,
  and trailing content, the hard parts of RFC 8259. The strongest alternative
  is a hand-written JSON parser verified in Verus. It would prove more, but
  Polean would then own a general JSON grammar it does not otherwise need. The
  strict reader plus the verified encoder gives the evidence path a proved
  canonical form without that. Date/Author: 2026-11-14, planning agent, revised
  after review.
- D6. Decision: both decoders cap the document at 65,536 bytes (named
  `IR_DOCUMENT_BYTES_MAX`, equal to `input_bytes`) and validate the whole
  document as UTF-8 before parsing. Rationale: technical design §11 sets 64 KiB
  as the input budget. The largest valid canonical document is 4,175 bytes
  (INV-CANON-LENGTH), so the cap never rejects a valid policy in any reasonable
  layout. Whole-document validation closes the serde_json ignorer gap.
  Date/Author: 2026-11-14, planning agent.
- D7. Decision: reject JSON escapes in keys, the profile string, and atom names
  with `EscapedText`. Rationale: technical design §7 says identifiers are fixed
  ASCII strings, and terms-of-reference N1 prefers explicit rejection to
  approximation. No consumer needs escapes. serde_json calls
  `visit_borrowed_str` for an unescaped string and `visit_str` for one it had
  to unescape, so the visitors can tell the two apart without scanning bytes.
  This matches 1.2.2's Q1 decision for Rego source. Escapes inside unknown
  values are irrelevant, because an unknown key is already a rejection.
  Date/Author: 2026-11-14, planning agent, revised after review (revision 1
  accepted escapes).
- D8. Decision: the decoder works in two passes and reports the first failure
  in this fixed precedence:
  1. size;
  2. UTF-8;
  3. JSON syntax, end of input, and trailing content (the first pass, which
     parses the whole document into `IgnoredAny`);
  4. a top-level value that is not an object;
  5. keys in document order: an escaped key, a duplicate key, or an unknown
     key, whichever comes first;
  6. missing `profile`, then missing `rules`;
  7. `profile`: not a string, escaped, then wrong value;
  8. `rules`: not an array, then more than 16 rules;
  9. each rule in order: not an array, empty, more than 16 atoms, then each
     atom in order (not a string, escaped, unknown name).

  In the typed pass, the value of a repeated key is read as `IgnoredAny`, so
  multiplicity outranks the value's type. Rationale: one deterministic answer
  per input keeps diagnostics and tests stable. Two passes are needed because
  serde_json reports type faults where it meets them
  (`Surprises & discoveries`). Date/Author: 2026-11-14, planning agent, revised
  after review.
- D9. Decision: duplicate atoms within a body, and duplicate rules, are valid
  IR and are preserved. Rationale: `policy.schema.json` has no `uniqueItems`,
  technical design §5 says v0 performs no optimizer transformations, and Rego
  admits a repeated comparison. Date/Author: 2026-11-14, planning agent.
- D10. Decision: verify the canonical encoder in Verus, and cover ingestion,
  decoding, and type invariants with tests. Rationale: see
  `Formal verification decision`. Date/Author: 2026-11-14, planning agent.
- D11. Decision: declare `Atom`, `Rule`, and `Policy` inside `verus!` in a
  self-contained kernel module `src/ir/model/`, together with the canonical
  encoder. The kernel holds the profile identifier, the atom names, and the
  structural tokens as constant byte arrays, which the specification reads
  through their views. Rationale: a `#[path]`-included kernel is compiled as
  its own crate by Verus, so it cannot name types declared elsewhere in `src/`.
  Verified code cannot reach private fields of types Verus cannot see, except
  through the forbidden `external_*` attributes. The settlement kernel
  (`tests/toolchain/audit/settlement/decision/types.rs`) already follows this
  self-contained pattern. Sharing the constants between the specification and
  the encoder lets the injectivity controls change both at once, so only
  injectivity can fail. `Atom::name() -> &'static str` and other conveniences
  sit outside `verus!`, and a test ties them to the kernel's bytes. A test also
  ties the kernel's profile bytes to `profile::PROFILE_ID`. Date/Author:
  2026-11-14, planning agent, revised after review; depends on Q1.
- D12. Decision: the users' guide documents the library in prose and points to
  Rustdoc, whose examples run as doctests in `make test`. It adds no fenced
  example. Rationale: every fenced example in `docs/users-guide.md` needs a
  tested-example marker and an `ExecutionScope` classification in
  `tests/documentation_examples/scope.rs`, and none of the existing scopes
  executes Rust. Doctests already execute. Date/Author: 2026-11-14, planning
  agent.
- D13. Decision: this plan fixes no exit-code mapping. 2.1.4 maps
  `SourceError` and `DecodeError` to outcomes. Rationale: no CLI exists yet,
  and the review noted that an unreadable user-supplied policy may reasonably be
  `invalid_input` rather than `tool_error`. ADR 0007 lists the failure classes
  and leaves the mapping, and any stall deadline for blocking readers
  (technical design §11), to 2.1.4. Date/Author: 2026-11-14, planning agent,
  revised after review.
- D14. Decision: `Policy::to_canonical_bytes` returns `CanonicalIr`, a newtype
  over the bytes. Rationale: roadmap 4.1 binds source bytes and IR bytes
  separately. A distinct type stops a caller from passing one where the other
  is meant. Date/Author: 2026-11-14, planning agent, adopted from review.
- D15. Decision: protect the proof against drift with a digest manifest. On
  success, `make verus` writes `verus/verified-inputs.sha256`, holding the
  SHA-256 of every file in every proof tree. A Python test under
  `make test-scripts` fails if any listed file differs from its digest, or if a
  proof tree holds a file not listed. `.github/workflows/toolchain.yml` runs
  that target when any proof input changes. Rationale: the kernel now ships,
  and 2.1.2 and later items will edit `src/`. Without a tripwire, an edit would
  leave a stale proof that still reads as current. The manifest catches honest
  mistakes; it is not a security control, since anyone can rewrite it. Q2 asks
  about the stronger option. Date/Author: 2026-11-14, planning agent, adopted
  from review.

### Open questions for the user

- Q1. May `vstd` become a production dependency, so that the canonical encoder
  is verified where it ships? Recommended: yes. The cost is these pre-release
  crates in the normal dependency graph: `vstd`, `verus_builtin`,
  `verus_builtin_macros`, `verus_state_machines_macros`, `verus_syn`, and
  `verus_prettyplease`. Their build-time proc-macro dependencies `syn` 3,
  `synstructure`, and `convert_case` come too. All are already locked and built
  by every test run, and `verus!` erases ghost code from the shipped binary.
  ADR 0005's record says the recognizer kernel, written the same way, is meant
  to be promoted in 2.1.2, so the dependency arrives then regardless; taking it
  now lets ADR 0007 decide it once. Alternative B keeps production code free of
  `vstd`: prove injectivity of a specification in `verus/` only, and link the
  plain-Rust encoder to it by the serde_json oracle and the snapshot. Under B,
  refinement is tested rather than proved, the encoder's logic exists twice,
  and the drift manifest covers only the specification. Choose B if 2.1.2 might
  not promote the recognizer kernel into `src/`.
- Q2. Should a path-filtered CI job run `make install-verus` and `make verus`
  when a proof input changes? That would amend ADR 0005's consequence that
  `make verus` stays outside CI. Recommended: not in this item. D15's manifest
  catches an unverified edit at commit time without CI cost. If you want the
  proof re-run in CI, say so, and the job and the ADR amendment join EP-M3.

## Outcomes & retrospective

Not started. Complete at each milestone and at delivery.

## Context and orientation

Base revision inspected: `ad217c8` on `main` ("Demonstrate declaration-type and
transitive-axiom auditing (1.2.3)"). Line numbers below are from that revision;
relocate them with `grep -n`.

### Current repository state

`src/lib.rs` is a scaffold exporting `STATUS` and `project_name()`, which
`tests/stub.rs` checks. `Cargo.toml` has an empty `[dependencies]` table. These
are dev-dependencies pinned with `=`: serde, serde_json, proptest, rstest,
rstest-bdd, googletest, pretty_assertions, insta, cap-std, Regorus, and vstd.
googletest and pretty_assertions are declared but unused. The lint policy in
`Cargo.toml` denies, among others:

- `indexing_slicing`, `unwrap_used`, and `expect_used`;
- `missing_const_for_fn` and `must_use_candidate`;
- the `shadow_*` lints;
- `missing_docs` and `missing_docs_in_private_items`;
- `self_named_module_files`.

It also forbids `unsafe_code`. `clippy.toml` caps cognitive complexity at 9 and
function length at 70 lines, and disallows environment access. Whitaker
(`make lint-whitaker`, run by `make lint`) adds `no_std_fs_operations`; tests
reach files through `cap-std`, as `tests/toolchain/ambient.rs` does.

`contracts/profile.json` fixes these values:

- the profile id, `polean.tenant-write.v0`;
- the rule limits: `min` 0, `max` 16, `body_min` 1, `body_max` 16;
- the atom names, in the order `same_tenant`, `write_action`, `admin_role`,
  `owns_resource`, `locked`, `unlocked`;
- `limits.source_bytes` and `limits.input_bytes`, both 65,536.

`contracts/policy.schema.json` describes the IR: an object with exactly
`profile` (the constant id) and `rules` (at most 16 arrays of 1 to 16 atom
names). Technical design §7 (`docs/technical-design.md:479-482`) defines
canonical bytes:

- key order `profile` then `rules`;
- compact JSON with no incidental whitespace;
- array order preserved;
- one final line feed;
- fixed ASCII identifiers;
- failure on duplicate keys and unknown fields.

The 1.2.2 spike (`tests/admission/`) holds the only IR-like code. Its
`Ir = Vec<Vec<String>>` (`tests/admission/admission/walk/mod.rs:35`) is
throwaway (ADR 0005; `docs/admission-feasibility-record.md` §12). Two kernels
show how to share code with Verus:

- The recognizer kernel (`tests/admission/recognizer/kernel/`) puts its body
  inside `verus! { ... }`. `#[path]` pulls in the specification and lemmas;
  ghost-only imports are gated on `#[cfg(verus_keep_ghost)]`, which
  `Cargo.toml` declares under `[lints.rust]`. `verus/admission_recognizer.rs`
  includes the kernel with `#[path]` and an empty `main`. It emits fixed bytes
  through `match` (`kernel/scan.rs:35-49`), not string literals.
- The settlement kernel (`tests/toolchain/audit/settlement/decision/`)
  declares every type it reasons about inside `verus!` and gates its spec,
  lemma, and witness `mod` declarations on `#[cfg(verus_keep_ghost)]`
  (`mod.rs:52-60`).

`tests/toolchain/audit/wire/` is the precedent for strict serde decoding with
byte caps checked before parsing.

### Verus wiring

`make install-verus` installs the Verus release pinned in `tools/verus/`, and
Z3, through `rust-prover-tools` (`tools/rust-prover-tools/REF`). `make verus`
first runs `verus-proof` over `VERUS_PROOF_FILES` (`Makefile:122`). It then runs
`verus-controls`, which invokes `scripts/check_verus_controls.py`. That script
stages each `ProofTree` into scratch and requires the unmodified tree to
verify. It then applies each negative control and requires Verus to reject it
with named output. `scripts/tests/test_verus_make_targets.py:200` hard-codes
the proof file list. None of this runs in CI.

### Gates

The commit gates (`AGENTS.md`) are:

- `make check-fmt`, `make typecheck`, and `make lint` (which includes
  `interrogate` and `lint-whitaker`);
- `make test`, `make test-scripts`, and `make design-check`;
- `make markdownlint`;
- `make toolchain-smoke`, when a pin, `scripts/`, `lean/`, or
  `tests/toolchain/` changes.

This plan changes `Cargo.toml` and `scripts/`, so `make test-scripts` and
`make toolchain-smoke` are in scope. `.github/workflows/toolchain.yml` is
path-filtered on `Cargo.toml`, `Cargo.lock`, `scripts/**`, `verus/**`, and
`tests/admission/recognizer/kernel/**`.

Run gates sequentially. Pipe each through `tee` to
`/tmp/<gate>-polean-2-1-1-bounded-source-ingestion-and-closed-policy-ir-types.out`,
and delegate full gate runs to the `scrutineer` agent.

### Test conventions for this plan

- Unit tests sit beside the code in `src/**/tests.rs`, declared as
  `#[cfg(test)] mod tests;`.
- Public-API tests, properties, fixtures, and behaviour-driven development
  (BDD) scenarios live in a new integration binary, `tests/ingestion/`. It runs
  no nested Cargo builds, so it needs no nextest group.
- There is no command-line interface (CLI) before 2.1.4. The BDD scenarios
  over the public library API are therefore the externally observable workflow,
  and stand in for an end-to-end test.
- Use `rstest` for fixtures and tables.
- For error variants, use googletest matchers: `expect_that!`, `assert_that!`,
  and `matches_pattern!`. They are needed most for `SourceError`, which cannot
  derive `PartialEq` because it holds `io::Error`. Use `#[gtest]` beneath
  `#[rstest]` where a test uses `expect_that!`.
- For byte and `Policy` equality, use `pretty_assertions::assert_eq`, imported
  by path so it does not collide with googletest's macro.
- `insta` snapshots record two things: the canonical bytes of every committed
  IR fixture, and the `Display` text of every error variant.

### Downstream consumers

- 2.1.2 turns a `PolicySource` into a `Policy`, keeping byte spans in a
  sidecar. It must keep the BOM, NUL, and bare carriage return corpus rows
  rejected (D4).
- 2.1.3 decodes requests under `profile::INPUT_BYTES_MAX` through
  `bounded::read_bounded`, with the same duplicate-key and escape discipline.
- 2.2.1 and 2.2.4 mirror `Atom` and `Policy` as Lean enum and list literals;
  the IR must contain no free strings.
- 2.3.2 runs OPA and Regorus over the exact `PolicySource` bytes, through
  `PolicySource::as_str` or `into_string`.
- 4.1.1 and 4.1.3 bind and re-derive `CanonicalIr`, and read evidence IR with
  `Policy::from_canonical_bytes`.

### Budget survey

Planned files and estimated net lines of code, by milestone:

- EP-M1 (about 930 lines):
  - production: `src/lib.rs` (+25), `src/profile.rs` (90), `src/bounded.rs`
    (90), `src/source/mod.rs` (190);
  - tests: `src/source/tests.rs` (260), `tests/ingestion/main.rs` (40),
    `tests/ingestion/support.rs` (90), `tests/ingestion/contracts.rs` (80),
    the first BDD scenarios and bindings (70).
- EP-M2 (about 2,350 lines):
  - production: `src/ir/mod.rs` (120), `src/ir/model/mod.rs` (230, the
    `verus!` kernel), `src/ir/decode/mod.rs` (260), `src/ir/decode/wire.rs`
    (280, the classifying visitors);
  - tests: `src/ir/tests.rs` (220), `src/ir/decode/tests.rs` (400),
    `tests/ingestion/properties/` (420), `tests/ingestion/fixtures.rs` (130),
    `tests/ingestion/contracts.rs` (+60), the remaining BDD scenarios and
    bindings (230);
  - the feasibility-step skeleton entry point, `verus/policy_canonical.rs`
    (25).
- EP-M3 (about 1,800 lines):
  - tooling commit: the forbidden-construct scan, the agreement test, the
    digest manifest and its check, and their tests (`scripts/`, about 450);
  - `Makefile` (+10);
  - proof: `verus/policy_canonical_spec.rs` (180),
    `verus/policy_canonical_lemmas/` (`mod.rs`, `bodies.rs`, `injective.rs`;
    about 900), `verus/policy_canonical_witnesses.rs` (80), and kernel ghost
    code (+150).
- EP-M4: about 20 lines of code (doctest touch-ups).

That is about 5,100 lines. The two comparable plans overran:

- 1.2.2 set about 4,000 lines and 40 files. It delivered 14,072 net lines in
  96 files, of which 14,824 were code-path lines and 2,253 were in `verus/`.
  The overrun was approved retrospectively on 2026-11-02, with no cause
  recorded beyond assessment repair.
- 1.2.3 set about 3,000 lines and 30 files. It delivered 9,406 net lines in
  66 files, of which 7,048 were code-path lines and 592 were in `verus/`. Its
  D23 attributes the overrun to a wrong estimate rather than scope growth, and
  the sponsor raised its trigger to 15,000 lines.

Both overruns were dominated by tests, proof lemmas, and review-round repairs,
which this survey now itemizes. The 1.2.2 proof, comparable in kind to this
one, needed about 2,250 lines, so the proof estimate here is about 1,300 rather
than revision 1's 700. The overall code limit of 8,000 is the survey plus about
55 per cent contingency. The per-milestone limits give each milestone about 45
per cent. The 55-file limit covers the 34 planned code files, about 12
documentation and configuration files, and splits forced by the 400-line rule.

Configuration:

- `Cargo.toml`: about +8 −4.
- `Cargo.lock`: expected 0, because a lockfile does not record dependency
  kind.
- `toolchain.yml`: +6 path filters.
- `SHA256SUMS`: in-place digest changes.

The 100-line limit allows for lockfile churn if Cargo reorders entries.

Documentation:

- ADR 0007: 200.
- `docs/technical-design.md`: +100.
- `docs/developers-guide.md`: +200.
- `docs/users-guide.md`: +50.
- `docs/repository-layout.md`: +30.
- `docs/contents.md`: +8.
- `docs/context.md`: +10.
- `docs/roadmap.md`: +2.
- `docs/adrs/adr-0005-*.md`: +12, for the amendment banner and section.
- `tools/verus/README.md`: +70.

That is about 680 lines. The 1,300-line limit allows for review-driven
clarification.

### Signposts

Read before implementing:

- `AGENTS.md`.
- `docs/terms-of-reference.md`: G1, N1, SC6, A2, and A6.
- `docs/context.md`: the shared vocabulary.
- `docs/technical-design.md`: §§4–7, §§10–11, and its References section.
  `docs/references.md` does not exist.
- `docs/admission-feasibility-record.md` §§7, 11, and 12.
- ADR 0002 and ADR 0005.
- `docs/developers-guide.md`: "Admission spike", "Tests", and "Documented
  examples".
- `docs/users-guide.md`.
- `docs/documentation-style-guide.md`: ADR amendments and table captions.
- `docs/rust-testing-with-rstest-fixtures.md`.
- `docs/reliable-testing-in-rust-via-dependency-injection.md`: inject the
  reader; do not touch the filesystem in the library.
- `docs/rust-doctest-dry-guide.md`.
- `docs/complexity-antipatterns-and-refactoring-strategies.md`.
- `docs/scripting-standards.md`, for `scripts/` changes.
- `tools/verus/README.md`.

Load these skills:

- `rust-router` first, then `rust-types-and-apis` (newtypes and closed enums)
  and `rust-errors` (closed error enums without `thiserror`).
- `rust-unit-testing` (rstest, googletest, pretty_assertions, insta),
  `proptest`, and `nextest`.
- `rust-verification` and `verus`.
- `addressing-whitaker-findings`.
- `arch-decision-records`, for ADR 0007.
- `en-gb-oxendict`.
- `execplans`, to keep this plan current.

## Conformance basis

Upstream artefacts inspected at `ad217c8`:

- Terms of reference, draft v0.1 (2026-10-04):
  - G1: state exactly which source forms and requests are covered.
  - N1: explicit rejection, not approximation.
  - SC6: unsupported or malformed input never yields success.
  - A2: trusted frontend.
  - A6: resource envelope.
- Technical design, draft v0.1 (2026-10-04, amended by 1.2.2 and 1.2.3):
  - §4.1: one UTF-8 module of zero to 16 rules of one to 16 atoms.
  - §4.2: the 64 KiB transport cap.
  - §5: a closed `Atom`, non-empty bodies, ordered rules, no optimizer.
  - §6: module layout. §6.1: the IR holds only the profile id and atom arrays,
    with spans in a sidecar.
  - §7: canonical bytes.
  - §11: 64 KiB budgets.
  - §10: obligations V7, V8, and V9.
- ADRs:
  - ADR 0001: one package.
  - ADR 0002: the assurance boundary.
  - ADR 0003: exact pins.
  - ADR 0005: 2.1.1 owns the production source and IR types; the spike IR is
    not promoted; Verus stays dev-only and on demand. ADR 0007 amends that
    last consequence if Q1 is accepted.
- Contracts: `contracts/profile.json`, `contracts/policy.schema.json`, and
  `contracts/cli.txt`.
- Roadmap item RM-2.1.1 and its success criteria.

Requirements derived for this plan:

- REQ-SRC-CAP: a source longer than 65,536 bytes is rejected, and ingestion
  never reads more than 65,537 bytes from a stream.
- REQ-SRC-UTF8: a source that is not valid UTF-8 is rejected with the offset
  of the first invalid byte.
- REQ-SRC-NUL: a source containing NUL is rejected with its offset.
- REQ-SRC-EXACT: an accepted source's bytes are exactly the input bytes.
- REQ-IR-CLOSED: an IR atom is one of six enum values; the IR holds no strings,
  spans, or source data.
- REQ-IR-BOUNDS: a policy has 0 to 16 rules and each rule 1 to 16 atoms, by
  construction.
- REQ-IR-ORDER: construction, decoding, and encoding all preserve rule order
  and atom order.
- REQ-CANON-FORM: canonical bytes are `{"profile":"<id>","rules":<rules>}`
  plus one line feed, compact, ASCII, with the specified key order.
- REQ-CANON-UNIQUE: distinct policies have distinct canonical bytes, so a
  digest of the bytes identifies one policy.
- REQ-CANON-STRICT: the strict reader accepts exactly the canonical bytes of
  well-formed policies.
- REQ-DECODE-STRICT: decoding rejects all of the following:
  - duplicate keys, unknown fields, and missing fields;
  - escaped spellings and wrong JSON types;
  - a different profile, unknown atoms, empty bodies, and bound violations;
  - malformed JSON, malformed UTF-8, and oversized documents.
- REQ-CONTRACT-AGREE: the Rust constants and atom names equal those in
  `contracts/profile.json` and `contracts/policy.schema.json`.

Gaps at the base revision:

- GAP-NO-PROD: no production source or IR type exists.
- GAP-NO-CAP: nothing enforces or tests the source cap.
- GAP-NO-CANON: nothing produces or checks canonical IR bytes.
- GAP-VERUS-SCAN: Verus controls do not reject forbidden proof constructs, and
  nothing detects a proof input edited after verification.

Trace links:

```plaintext
TOR-SC6, TDD-4.2, TDD-11 -> REQ-SRC-CAP, REQ-SRC-UTF8, REQ-SRC-NUL -> EP-M1 -> src/source/tests.rs, BDD source scenarios
TDD-7 -> REQ-SRC-EXACT -> EP-M1 -> INV-SRC-EXACT property
TDD-5, TDD-6.1 -> REQ-IR-CLOSED, REQ-IR-BOUNDS, REQ-IR-ORDER -> EP-M2 -> src/ir/tests.rs, INV-ROUND-TRIP
TDD-7 -> REQ-CANON-FORM, REQ-CANON-UNIQUE -> EP-M2, EP-M3 -> fixtures snapshot, L-CANON-REFINES, L-CANON-INJECTIVE
TDD-7, RM-4.1 -> REQ-CANON-STRICT -> EP-M2 -> INV-STRICT-READER property
TDD-7, TOR-N1 -> REQ-DECODE-STRICT -> EP-M2 -> src/ir/decode/tests.rs, INV-DECODE-REJECTS
profile.json, policy.schema.json -> REQ-CONTRACT-AGREE -> EP-M1, EP-M2 -> tests/ingestion/contracts.rs
GAP-VERUS-SCAN -> EP-M3 -> scripts/tests scan and manifest cases
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
policies, including ill-formed ones, so the uniqueness theorem does not depend
on `WF`. The executable `Policy` type maintains `WF` as a type invariant.

Source ingestion is modelled as a function from a byte stream `s` to a result,
taking the first case that applies:

```plaintext
ingest(s) = TooLarge                     if |s| > 65,536
          = InvalidUtf8(k)               if s is not UTF-8; k = longest valid prefix length
          = Nul(k)                       if s contains NUL; k = offset of the first NUL
          = Ok(s)                        otherwise
```

Ingestion consumes at most `min(|s|, 65,537)` bytes. A read error other than
`Interrupted` returns `Read`, even if some bytes have already arrived.

Decoding is modelled as `decode : Bytes → Result<Policy, Error>`. It must
satisfy three conditions:

1. `decode(render(p)) = Ok(p)` for every `WF(p)`;
2. `decode(b) = Ok(p)` implies `WF(p)`;
3. it rejects every class in REQ-DECODE-STRICT, following the D8 precedence.

The strict reader is `strict(b) = decode(b)` when that is `Ok(p)` and
`render(p) = b`, and `NonCanonical` when `decode(b)` is `Ok(p)` but
`render(p) ≠ b`.

### Invariants and lemmas

- INV-SRC-CAP: `ingest` returns `TooLarge` exactly when the stream holds more
  than 65,536 bytes, and never consumes more than 65,537 bytes through the
  `Read` interface. (REQ-SRC-CAP)
- INV-SRC-UTF8 and INV-SRC-NUL: within the cap, `ingest` returns
  `InvalidUtf8(k)` exactly when the bytes are not UTF-8, with `k` equal to
  `std`'s `valid_up_to`. Otherwise it returns `Nul(k)` exactly when a NUL is
  present, at its first offset. (REQ-SRC-UTF8, REQ-SRC-NUL)
- INV-SRC-EXACT: `ingest(s) = Ok(t)` implies `t = s` byte for byte.
  (REQ-SRC-EXACT)
- INV-SRC-CHUNKING: the result of ingesting a stream does not depend on how
  the reader splits it into reads, nor on interleaved `Interrupted` errors.
  (REQ-SRC-CAP, REQ-SRC-EXACT)
- INV-IR-WF: every `Policy` value satisfies `WF`, and the constructors return
  an error, never a value, otherwise. (REQ-IR-BOUNDS)
- L-CANON-REFINES: for every `Policy` value `p`, the executable encoder
  returns exactly `render(p@)`, where `p@` is the policy's view as a sequence
  of atom sequences. No precondition is needed. (REQ-CANON-FORM)
- L-CANON-INJECTIVE: for all `p` and `p'` in `Seq<Seq<Atom>>`,
  `render(p) = render(p')` implies `p = p'`. (REQ-CANON-UNIQUE, REQ-IR-ORDER)
- L-CANON-SHAPE: `render(p)` begins with
  `{"profile":"polean.tenant-write.v0","rules":[` and ends with `]}` and one
  line feed. It contains no other line feed and no space, tab, or carriage
  return, and every byte is below `0x80`. (REQ-CANON-FORM)
- INV-CANON-LENGTH: for every `WF(p)`, `|render(p)| ≤ 4,175`, with equality
  for 16 rules of 16 `owns_resource` atoms. The constant is a 44-byte prefix,
  at most `16 · 257 + 15 = 4,127` bytes of rules plus the two list brackets,
  and the suffix `}` and line feed. (Links REQ-CANON-FORM to D6.)
- INV-ROUND-TRIP: `decode(render(p)) = Ok(p)` for every `WF(p)`, and
  `decode(b) = Ok(p)` for every JSON layout `b` of `p`, meaning any whitespace
  and either key order. (REQ-IR-ORDER, REQ-DECODE-STRICT)
- INV-STRICT-READER: `strict(b) = Ok(p)` exactly when `b = render(p)` for a
  well-formed `p`; every other valid layout gives `NonCanonical`.
  (REQ-CANON-STRICT)
- INV-DECODE-REJECTS: each rejection class yields the variant in the table
  below, and never `Ok`. (REQ-DECODE-STRICT)
- INV-CONTRACT: three sets of values agree with `contracts/profile.json` and
  `contracts/policy.schema.json`: the constants in `src/profile.rs`; `Atom`'s
  names and order; and the kernel's profile and name bytes. (REQ-CONTRACT-AGREE)

Each rejection class maps to one `DecodeError` variant:

| Class                                                | Variant                            |
| ---------------------------------------------------- | ---------------------------------- |
| Document over 65,536 bytes                           | `TooLarge { limit }`               |
| Malformed UTF-8 anywhere                             | `InvalidUtf8 { valid_up_to }`      |
| JSON syntax, BOM, surrogate escape, trailing content | `Json { class: Syntax, .. }`       |
| Truncated document                                   | `Json { class: Eof, .. }`          |
| Top-level array, scalar, or `null`                   | `NotAnObject`                      |
| Escaped key, profile, or atom                        | `EscapedText`                      |
| Repeated `profile` or `rules`                        | `DuplicateField(Field)`            |
| Any other key                                        | `UnknownField`                     |
| Missing `profile` or `rules`                         | `MissingField(Field)`              |
| `profile` not a string                               | `ProfileNotString`                 |
| `profile` not `polean.tenant-write.v0`               | `WrongProfile`                     |
| `rules` not an array                                 | `RulesNotArray`                    |
| More than 16 rules                                   | `TooManyRules { count }`           |
| A rule that is not an array                          | `RuleNotArray { rule }`            |
| An empty rule                                        | `EmptyBody { rule }`               |
| More than 16 atoms in a rule                         | `TooManyAtoms { rule, count }`     |
| An atom that is not a string                         | `AtomNotString { rule, position }` |
| An atom name outside the six                         | `UnknownAtom { rule, position }`   |
| Valid but not canonical (strict reader only)         | `NonCanonical`                     |

_Table 1: Decoder rejection classes and their error variants._

### Proof sketch for the canonical encoding

These are hypotheses about the proof's shape, not commitments to its
decomposition. Implementation may merge, split, or replace them.

L-CANON-INJECTIVE is the substantive theorem. The intended route is a
specification-level parser `parse` with `parse(render(p)) = Some(p)` for every
`p`. Injectivity then follows directly: `render(p) = render(p')` gives
`Some(p) = Some(p')`. The parser need not be executable. Supporting hypotheses:

1. Prefix-freeness of atom names. The six names begin with distinct letters
   (`s`, `w`, `a`, `o`, `l`, `u`), so for atoms `a ≠ b`, `q(a)` and `q(b)`
   differ at their second byte. No name is a prefix of another. The quotes are
   not what makes the set prefix-free.
2. Body decoding: for any rule `r` and any suffix `t`,
   `parseBody(body(r) ++ t) = Some((r, t))`. This goes by induction on `r`,
   using (1) and the fact that neither `,` nor `]` can begin an atom token. The
   empty body `[]` is the base case.
3. Rule-list decoding: for any `p` and suffix `t`,
   `parseRules(rules(p) ++ t) = Some((p, t))`. This goes by induction on `p`,
   using (2) and the fact that a body begins with `[` while the list's
   terminator is `]`. The brackets are what keep `[[a],[b]]` and `[[a,b]]`
   apart.
4. Join bridging: `join(xs.push(x), s) = join(xs, s) ++ s ++ x` for non-empty
   `xs`. The encoder's loops append at the back, while `parse` reads from the
   front, and this lemma connects the two views.
5. Envelope: `render` is a fixed prefix, then `rules(p)`, then a fixed suffix.
   `parse` strips the prefix, applies (3), and requires exactly the suffix to
   remain.

L-CANON-REFINES is proved by loop invariants in the kernel. After the encoder
has written rules `0..i`, its buffer equals the prefix of `render(p@)` built
from those rules. The same holds within a body, using (4). Bytes are appended
with `push` loops over the constant arrays. vstd's `extend_from_slice` promises
only cloned elements, not equality. L-CANON-SHAPE follows from the definition
and a lemma that `rules(p)` contains only bytes from the atom names and `[`,
`]`, `,`, and `"`. Injectivity is stated over views. No proof relies on the
derived `PartialEq`, so its agreement with view equality is not an obligation;
the tests compare `Policy` values, and the views are the proof's only notion of
equality.

The atom names, the profile bytes, and the structural tokens are kernel
constants. The specification reads them through their views, so changing a
constant changes the encoder and the specification together. INV-CONTRACT ties
them to the contract, so drift between the proof's names and the profile's
names fails a test.

### Axioms (trusted, not verified here)

- AXIOM-STD-UTF8: `std::str::from_utf8` accepts exactly well-formed UTF-8 and
  reports `valid_up_to` as documented. INV-SRC-UTF8 relies on it.
- AXIOM-STD-READ: `Read::take` and `Read::read_to_end` behave as documented.
  They read no more than the limit, retry on `Interrupted`, stop at `Ok(0)`,
  and return an error without the partial data. INV-SRC-CAP and
  INV-SRC-CHUNKING rely on this; a counting reader exercises it.
- AXIOM-SERDE-JSON: serde_json 1.0.154 behaves as follows. It parses RFC 8259
  JSON. It rejects trailing content, a leading BOM, and unpaired surrogate
  escapes in strings it decodes, including keys. It calls `visit_borrowed_str`
  exactly when a string from a `&str` input needed no unescaping.
  INV-ROUND-TRIP and INV-DECODE-REJECTS rely on this, and named tests exercise
  each behaviour.
- AXIOM-VERUS: Verus at the pinned release, its `vstd` library, and the pinned
  Z3 are sound. `verus!` erases ghost code under plain `rustc` without changing
  executable code. vstd's own specifications of `Vec` are trusted.
  L-CANON-REFINES, L-CANON-INJECTIVE, and L-CANON-SHAPE rely on this.

### Methods per obligation

- INV-SRC-CAP.
  - Method: parameterized boundary tests.
  - Domain: lengths 0, 1, 65,535, 65,536, 65,537, and 65,538; an infinite
    stream (`std::io::repeat`); and a multi-byte character split at 65,536.
  - Artefacts: `src/source/tests.rs` and the BDD scenarios.
  - Evidence: `make test`. The red step is a compile failure, because
    `PolicySource` does not exist; then each row fails against a stub that
    returns `Ok`.
  - Non-vacuity: a counting reader records the bytes consumed, and the
    infinite stream proves the bound without trusting a length. A seeded
    fault that raises the read limit to 65,538 must fail the consumption
    test. Record the transcript under `Artefacts and notes`.
- INV-SRC-UTF8 and INV-SRC-NUL.
  - Method: `rstest` tables. A property test would compare `from_utf8` with
    itself, so the table carries the evidence.
  - Domain:
    - truncated sequences and overlong encodings;
    - encoded surrogates (`ED A0 80`) and code points above U+10FFFF;
    - stray continuation bytes;
    - NUL at offsets 0, 10, and 65,535;
    - an invalid sequence before a NUL, which must report UTF-8;
    - a BOM, a carriage return, and a line feed, all accepted unchanged.
  - Non-vacuity: seeded faults that skip each check must fail their rows.
- INV-SRC-EXACT and INV-SRC-CHUNKING.
  - Method: property test.
  - Domain: random valid sources up to the cap, delivered by a reader that
    returns random chunk sizes and injects `Interrupted`. A fixed row covers
    100 bytes followed by `Err(Other)`, which must give `Read`.
  - Oracle: `from_bytes` on the same bytes.
  - Non-vacuity: assert that each run produces at least one chunk of size 1
    and one `Interrupted`. A seeded fault that drops a chunk must fail.
- INV-IR-WF.
  - Method: `rstest` tables at 0, 1, 16, and 17 rules, and at 0, 1, 16, and
    17 atoms.
  - Artefact: `src/ir/tests.rs`.
  - Non-vacuity: both acceptance and rejection occur at each bound. A
    `compile_fail` doctest shows that `Policy` has no `Deserialize`
    implementation.
- L-CANON-REFINES, L-CANON-INJECTIVE, and L-CANON-SHAPE.
  - Method: Verus proof over the shipped kernel.
  - Domain: every `Policy` value for refinement, and every `Seq<Seq<Atom>>`
    for injectivity and shape.
  - Artefacts:
    - the kernel, `src/ir/model/mod.rs`;
    - `verus/policy_canonical_spec.rs`;
    - `verus/policy_canonical_lemmas/`, with the injectivity lemmas alone in
      `injective.rs`;
    - `verus/policy_canonical_witnesses.rs`;
    - the entry point, `verus/policy_canonical.rs`.
  - Evidence: `make verus` reports 0 errors for every entry point and every
    control rejected.
  - Non-vacuity, witnesses: the witnesses prove that `render` of the empty
    policy and of the baseline policy equal concrete literal byte sequences.
    They also call the executable encoder on the baseline inside a proof, so
    the specification is the intended one and refinement is exercised on a
    real value.
  - Non-vacuity, controls: each of these must be rejected by Verus.
    - (a) The kernel omits the comma between atoms, with the specification
      unchanged. Refinement must fail, naming the encoder's postcondition.
    - (b) The kernel omits the final line feed. Refinement must fail.
    - (c) The kernel constant for `unlocked`'s name is changed to `locked`'s
      bytes. This is one edit, read by both the encoder and the
      specification, so refinement still holds. Injectivity must fail in
      `injective.rs`.
    - (d) The body bracket constants become empty, changing both encoder and
      specification. `[[a],[b]]` and `[[a,b]]` then collide, and injectivity
      must fail in `injective.rs`.

    Controls (c) and (d) show that the injectivity proof depends on distinct
    names and on the brackets, not on a vacuous premise. They must not be
    satisfiable by a refinement failure. The script requires the
    `injective.rs` path in their expected output, and the plan records
    whether any other obligation was also rejected.
- INV-CANON-LENGTH.
  - Method: unit test of the maximal policy, and a property that
    `to_canonical_bytes().len() ≤ 4,175` for generated well-formed policies.
  - Rationale: length grows with token length, so the all-`owns_resource`
    policy is the maximum.
- Canonical bytes, against an independent oracle.
  - Method: property and fixture tests compare `to_canonical_bytes` with
    serde_json's compact serialization of a test-only struct, fields
    `profile` then `rules`, plus a line feed.
  - Rationale: this oracle is independent of the encoder and checks the same
    `render` relation in ordinary tests. A proof that verified the wrong
    specification would still meet a failing test.
  - Artefacts: `tests/ingestion/properties/canonical.rs`,
    `tests/ingestion/fixtures.rs`, and an `insta` snapshot of the canonical
    bytes of every committed IR fixture.
- INV-ROUND-TRIP and INV-STRICT-READER.
  - Method: property tests.
  - Policy classes: the empty policy, 16 rules, 16-atom bodies, every atom,
    repeated atoms, and repeated rules.
  - Layouts: random whitespace (space, tab, line feed, carriage return)
    between tokens, and swapped key order.
  - Oracle: the generated policy value. The strict reader must accept the
    canonical layout and return `NonCanonical` for every other layout.
  - Non-vacuity: a deterministic fixed-seed coverage test asserts that every
    class above is reached, and that some generated layout is non-canonical.
- INV-DECODE-REJECTS.
  - Method: an `rstest` table with one named row per Table 1 class and per D8
    precedence pair (for example, a duplicate `profile` whose second value is
    a number must give `DuplicateField`). Property tests mutate a valid
    document into each class.
  - Edge rows:
    - an escaped key `"profile"`;
    - an escaped duplicate;
    - the key `"profile\u0000"`;
    - a homoglyph key;
    - `1e999` as the profile;
    - 60,000 nested brackets in an unknown value, which must give
      `UnknownField` without overflowing the stack;
    - deep nesting inside `rules`;
    - unpaired surrogates in a key, in the profile, and in an atom.
  - Artefacts: `src/ir/decode/tests.rs`, and
    `tests/ingestion/properties/decode.rs`.
  - Non-vacuity: every mutation starts from a document the property first
    asserts decodes `Ok`. Seeded faults that disable each check must make its
    row fail.
- INV-CONTRACT.
  - Method: tests that read `contracts/profile.json` and
    `contracts/policy.schema.json` through `cap-std` and compare.
  - Artefact: `tests/ingestion/contracts.rs`.
  - Non-vacuity: each comparison names the contract field it reads, and a
    test fails if the field is absent rather than defaulting.
- Error rendering.
  - Method: an `insta` snapshot of `Display` for one value of every
    `SourceError`, `RuleError`, `PolicyError`, and `DecodeError` variant.
  - Rationale: the snapshot shows no message echoes input text.

### Formal verification decision

Verus is warranted for the canonical encoding and nowhere else in this item.
Canonical bytes are the identity that evidence binding (roadmap 4.1.1) hashes
and re-derivation (4.1.3) compares. Two failures would undermine that:

- if two policies shared canonical bytes, a binding would silently cover a
  policy that was never checked;
- if the encoder emitted bytes other than the specified form, the strict
  reader and independent tools could not reproduce the binding.

Both properties quantify over every policy and every ordering. The encoder is a
small pure kernel, and the token set admits a clean inductive argument. That is
the case the `rust-router` hierarchy reserves for Verus.

The stronger methods were each considered and rejected:

- Kani adds nothing here. The input space is up to 6^256 atom orderings, too
  large for meaningful bounds, and Verus covers it without a bound.
- A Verus-verified canonical-only parser would make the strict reader proved
  rather than tested on its acceptance side. The review ranked it the strongest
  alternative. It costs about 400 more proof lines and a second executable
  parser. D3's decode-and-compare reader gets most of its value: soundness of
  acceptance follows from the equality check and L-CANON-REFINES; only
  completeness rests on the tested round trip.
- Ingestion is a thin composition of `std` primitives whose behaviour is an
  axiom. Boundary tests with a counting reader test the repository-owned logic
  at its real interface.
- The decoder's hard parts belong to serde_json, which is not ours to verify
  (AXIOM-SERDE-JSON). Verifying our visitors would need `assume_specification`
  over serde, which the constraints forbid. Named tests and mutation properties
  cover key multiplicity and value classification.
- Type invariants hold by construction behind private fields.

Diagnostics formatting gets tests only.

The proof does not establish that the real serde-based decoder inverts the
encoder; INV-ROUND-TRIP tests that. It establishes nothing about Rego,
admission, or the Lean model.

### BDD feature

Adopted: `tests/ingestion/features/policy_ingestion.feature`, bound by
`tests/ingestion/bdd.rs` with `rstest-bdd` 0.6.0. The bindings follow the
slot-state pattern in `tests/admission/bdd.rs`. The feature sits outside
`tests/features/`, so the toolchain workflow's path filter does not run on
every change to it. Run it with `cargo nextest run --test ingestion` or
`make test`.

```gherkin
Feature: Policy sources and IR are ingested without losing their meaning

  Scenario: A source at the size cap is accepted unchanged
    Given a valid UTF-8 policy source of exactly 65536 bytes
    When the source is ingested from a stream
    Then ingestion succeeds
    And the ingested bytes equal the original bytes

  Scenario: A source one byte over the cap is rejected
    Given a valid UTF-8 policy source of exactly 65537 bytes
    When the source is ingested from a stream
    Then ingestion fails because the source is too large

  Scenario: An unbounded stream is rejected without reading on
    Given an unbounded stream of policy bytes
    When the source is ingested from the stream
    Then ingestion fails because the source is too large
    And no more than 65537 bytes were read from the stream

  Scenario: Malformed UTF-8 is rejected with its offset
    Given a policy source whose byte at offset 10 starts an invalid sequence
    When the source is ingested from bytes
    Then ingestion fails with invalid UTF-8 at offset 10

  Scenario: A NUL byte is rejected with its offset
    Given a policy source with a NUL byte at offset 20
    When the source is ingested from bytes
    Then ingestion fails with a NUL byte at offset 20

  Scenario: The committed baseline IR encodes to canonical bytes
    Given the committed IR fixture "examples/tenant-write/policy.json"
    When the fixture is decoded and encoded canonically
    Then the canonical bytes are compact, keyed profile then rules, and end with one line feed
    And the rules appear in their committed order
    And the strict reader accepts the canonical bytes
    And the strict reader rejects the committed fixture as non-canonical

  Scenario: A policy with zero rules is valid
    Given the committed IR fixture "examples/tenant-write/mutations/deny-all/policy.json"
    When the fixture is decoded and encoded canonically
    Then the canonical bytes end with "\"rules\":[]}" and one line feed

  Scenario Outline: Malformed IR documents are rejected
    Given the baseline IR document altered by "<alteration>"
    When the document is decoded
    Then decoding fails with "<error>"

    Examples:
      | alteration          | error           |
      | empty first body    | empty body      |
      | duplicate profile   | duplicate field |
      | added field comment | unknown field   |
      | unknown atom        | unknown atom    |
      | escaped atom name   | escaped text    |
```

The first five scenarios belong to EP-M1, and the rest to EP-M2. They trace to
REQ-SRC-CAP, REQ-SRC-UTF8, REQ-SRC-NUL, REQ-SRC-EXACT, REQ-CANON-FORM,
REQ-CANON-STRICT, REQ-IR-BOUNDS, and REQ-DECODE-STRICT. Each milestone adds its
scenarios before their step bindings, so the first run fails.

## Plan of work

The work is four milestones. Each ends with these gates passing:
`make check-fmt`, `make typecheck`, `make lint`, `make test`, and
`make markdownlint`, plus the milestone's extra gates. Record a `numstat`
measurement in `Progress` at each plateau. Commit in small steps (red test,
green implementation, refactor), with imperative subjects and bodies that state
the assurance impact.

### `EP-M1` profile constants and bounded source ingestion

Add `src/profile.rs` with these constants, each documented with its contract
field: `PROFILE_ID`, `SOURCE_BYTES_MAX` and `INPUT_BYTES_MAX` (both 65,536),
`RULES_MAX` (16), `BODY_ATOMS_MIN` (1), and `BODY_ATOMS_MAX` (16).

Add `src/bounded.rs` with `read_bounded(reader, limit)`, which returns
`Result<Vec<u8>, BoundedReadError>`. `BoundedReadError` has the variants
`TooLarge { limit }` and `Read(io::Error)`. The module is public because 2.1.3
reuses it. Its Rustdoc states the reader contract: `Ok(0)` is end of stream,
`Interrupted` is retried, and a reader that reports more bytes than it was
given makes `std` panic.

Add `src/source/mod.rs` with:

- `PolicySource`, with the constructors `from_bytes` and `read_from`;
- the accessors `as_bytes`, `as_str`, and `into_string`;
- `SourceError`, with the variants `TooLarge`, `InvalidUtf8`, `Nul`, and
  `Read`.

Implement `Display` and `std::error::Error` by hand. `Read` exposes the
`io::Error` as its `source()`; its `Display` names a read failure without
quoting the reader's text.

Declare the modules in `src/lib.rs`, and update its crate documentation to say
what now exists. Keep `STATUS` and `project_name`, so `tests/stub.rs` stays
valid; change `STATUS` only if its wording becomes false.

Red: add `src/source/tests.rs` with the boundary, UTF-8, and NUL tables, the
infinite-stream test, and the partial-read error row. Add
`tests/ingestion/main.rs`, `tests/ingestion/support.rs` (the counting and
chunking readers, and `cap-std` fixture loading), and
`tests/ingestion/contracts.rs` (profile constants only). Add the first five
scenarios. Run `cargo nextest run --lib source --test ingestion` and record the
compile failure. Then add stubs that return `Ok` and record the assertion
failures.

Green: implement until the tests pass. Refactor: keep functions short and
modules under 400 lines.

Add `tests/ingestion/properties/source.rs` for INV-SRC-EXACT and
INV-SRC-CHUNKING.

Acceptance: the milestone gates pass, and the new tests ran red before the
implementation and green after.

### `EP-M2` closed IR types, canonical encoder, and strict decoders

Step 1, the feasibility probe. If Q1 is accepted, move `vstd` to
`[dependencies]`; move `serde` (with `derive`) and `serde_json` there too. Add
a skeleton `src/ir/model/mod.rs` inside `verus!` holding:

- `Atom`, `Rule`, and `Policy`, with private fields and `View` implementations;
- constructors with `ensures` on the type invariant;
- the constant byte arrays;
- an `encode` with no `ensures` yet.

Add `src/ir/mod.rs`, re-exporting the types and holding the non-Verus
conveniences. Add the entry point `verus/policy_canonical.rs`. Run `make lint`,
`make test`, and `make verus` on the skeleton. Only the skeleton's trivially
true obligations are checked at this stage.

The probe decides whether the derives, the constants, `View`, and the
library-level lints work together. If they do not, stop under the Feasibility
tolerance. Commit the probe as its own commit.

Step 2, the IR types. Derive `Clone`, `Debug`, `PartialEq`, and `Eq` inside
`verus!`, and `Copy` on `Atom`. Add no `Hash` or `Ord` unless the probe shows
they are free, because no consumer needs them. Outside `verus!`, add:

- `Atom::ALL` and `Atom::name()`;
- `Rule::new`, returning `RuleError`;
- `Policy::new`, returning `PolicyError`;
- the accessors `Rule::atoms` and `Policy::rules`.

Neither enum is `#[non_exhaustive]`. The IR is closed, and adding an atom must
break every exhaustive match.

Step 3, the encoder. Complete `encode` in the kernel with `push` loops over the
constants, written so EP-M3 can add invariants without changing executable
statements. `Policy::to_canonical_bytes` wraps the result in `CanonicalIr`,
which has `as_bytes` and `into_bytes`.

Step 4, the decoders. Add `src/ir/decode/mod.rs` and `src/ir/decode/wire.rs`.
`Policy::from_json_slice` runs in this order:

1. the size check;
2. `std::str::from_utf8`;
3. the syntax pass, `serde_json::from_str::<serde::de::IgnoredAny>`;
4. the typed pass, through the classifying visitors (D5, D7, D8);
5. validation into a `Policy`.

`Policy::from_canonical_bytes` calls `from_json_slice`, then compares the input
with the policy's canonical bytes. Map `serde_json::Category` totally: `Eof` to
`JsonErrorClass::Eof`, and every other category to `Syntax`, with a comment that
`Data` and `Io` cannot arise from an `IgnoredAny` pass over a `&str`.

Red: add `src/ir/tests.rs`, `src/ir/decode/tests.rs`,
`tests/ingestion/fixtures.rs`, `tests/ingestion/properties/canonical.rs`,
`tests/ingestion/properties/decode.rs`, the IR half of
`tests/ingestion/contracts.rs`, the error-display snapshot, and the remaining
scenarios. Run them and record the failures.

Green: implement. Refactor: split any module that approaches 400 lines. Review
and accept the `insta` snapshots with `cargo insta review`.

Acceptance: the milestone gates pass, together with `make toolchain-smoke` and
`make test-scripts`, because `Cargo.toml` changed. Every committed IR fixture
decodes, and its canonical bytes match both the serde_json oracle and the
reviewed snapshot.

### `EP-M3` proof tooling, then the Verus proof of the canonical encoding

Tooling commit, made before any proof work. Extend
`scripts/check_verus_controls.py` and `scripts/tests/`, following
`docs/scripting-standards.md`. Make three additions.

First, a forbidden-construct scan over every proof tree's sources. It strips
`//` comments and fails on any of these patterns:

- `\b(assume|admit)\s*\(`;
- `\bassume_specification\b` and `\bassume_new\b`;
- `\bexternal(_body|_fn_specification|_type_specification|_trait_specification|_trait_extension)?\b`,
  inside a `verifier::` or `verifier(` attribute;
- `exec_allows_no_decreases_clause`;
- `not\s*\(\s*verus_keep_ghost`;
- `\baxiom\s+fn\b`.

Tests must show three things. A scratch tree with an inserted `assume(false)`
is rejected. The existing identifiers `axiom_code_spec` and
`axiom_findings_empty_iff`, and a doc comment mentioning "assume", are not
flagged. Every existing tree passes.

Second, the D15 digest manifest. On success, `make verus` writes
`verus/verified-inputs.sha256`. A test fails if any proof input differs from
the manifest, or if a tree holds a file the manifest does not list.

Third, a test that `VERUS_PROOF_FILES` and the script's trees name the same
entry points.

Also add `verus/policy_canonical.rs` to `VERUS_PROOF_FILES`, add its
`ProofTree` (sources `src/ir/model/` and the `verus/policy_canonical*` files),
and update `scripts/tests/test_verus_make_targets.py`. Add `src/ir/model/**` and
`verus/verified-inputs.sha256` to both path filters in
`.github/workflows/toolchain.yml`. The forbidden-construct scan and the
manifest check are pure Python and run under `make test-scripts`, which CI runs.

Proof. Add three files beside the kernel:

- `verus/policy_canonical_spec.rs`: `render`, `WF`, and the views;
- `verus/policy_canonical_lemmas/`: `mod.rs`, `bodies.rs`, and `injective.rs`;
- `verus/policy_canonical_witnesses.rs`.

The kernel `#[path]`-includes them, gating the `mod` declarations on
`#[cfg(verus_keep_ghost)]` as the settlement kernel does. Add the `ensures` and
loop invariants to the kernel, then add the four controls.

Red: before the lemmas exist, `make verus` must fail on the new entry point
with unproved postconditions; record the output. Green: prove. Then run
`make verus` again to refresh the manifest, and commit the manifest.

Acceptance:

- `make install-verus` (once) and `make verus` exit 0, with `VERUS_FLAGS`
  unset. Every entry point reports `0 errors`, the recognizer and settlement
  verified counts are unchanged, and every control, old and new, is rejected at
  its named obligation.
- `make test-scripts` and `make toolchain-smoke` pass, as do the milestone
  gates.

Record in `Artefacts and notes` whether controls (c) and (d) also rejected any
obligation outside `injective.rs`.

### `EP-M4` documentation, ADR 0007, and delivery closure

Write ADR 0007, `docs/adrs/adr-0007-verify-the-canonical-ir-encoding.md`. It
records:

- the decision to verify the canonical encoder in production code;
- the production dependencies, and why each is acceptable;
- the two decoders and their acceptance rules (D3, D5–D9);
- the NUL decision and the 2.1.2 entry requirement (D4);
- the failure classes left for 2.1.4 to map (D13);
- the drift manifest (D15).

Add an "Amended by ADR 0007" banner and an `## Amendments` section to ADR 0005,
as the documentation style guide prescribes.

Update `docs/technical-design.md`:

- §6: record the implemented module layout, including the `source` departure
  (D1);
- §6.1: point to the IR types;
- §7: state the two decoders, D6, D7, and the Verus theorem with its
  boundary;
- §10: note what the canonical-encoding proof adds beside V7 and V9.

Update `docs/developers-guide.md`. Replace "The Rust scaffold" with the
production modules, and describe:

- the self-contained `verus!` kernel pattern in production code;
- the re-use strategy for `bounded::read_bounded` (2.1.3 must use it, not
  write another reader);
- test placement and the googletest pattern;
- the Verus registration checklist and the drift manifest.

Update `docs/users-guide.md` "Available today" with a prose description of the
library (D12). Update `docs/repository-layout.md`: the `src/` rows, the
`verus/` row, and the `examples/tenant-write/` row, which wrongly calls the
fixtures "canonical IR". Update `docs/contents.md` to link this plan and ADR
0007. Add glossary entries for _policy source_ and _canonical IR bytes_ to
`docs/context.md`. Document the proof's statement and boundaries in
`tools/verus/README.md`. Refresh `SHA256SUMS` for the covered files changed.

Mark roadmap 2.1.1 done, with a "Recorded:" line matching the 1.2.x entries.
Search for stale claims with:

```bash
grep -rn "scaffold\|not implemented\|canonical IR" README.md docs --include=*.md
```

Exclude `docs/execplans/` from that review.

Acceptance: the milestone gates pass, together with `make design-check` and
`sha256sum -c SHA256SUMS`.

## Milestones and plateaus

### `EP-M1`

- Outcome: `polean::source` ingests a bounded, UTF-8-checked, NUL-free policy
  source. `polean::bounded` and `polean::profile` exist.
- Requirements: REQ-SRC-CAP, REQ-SRC-UTF8, REQ-SRC-NUL, REQ-SRC-EXACT, and
  part of REQ-CONTRACT-AGREE; GAP-NO-CAP.
- Acceptance: the boundary, UTF-8, NUL, chunking, and contract tests, the five
  BDD scenarios, and the gates all pass.
- Conformance check:
  - No filesystem, environment, or printing in `src/`.
  - No dependency change.
  - Bytes preserved exactly (D4).
  - Code within its 1,400-line budget.
- Recovery: revert the milestone's commits; nothing depends on them.
- Remaining gaps: no IR yet.
- Compatibility decision: none; this is a new pre-1.0 API.

### `EP-M2`

- Outcome: `polean::ir` provides closed types, a tested canonical encoder, a
  general decoder, and a strict reader.
- Requirements: REQ-IR-CLOSED, REQ-IR-BOUNDS, REQ-IR-ORDER, REQ-CANON-FORM
  (tested), REQ-CANON-STRICT, REQ-DECODE-STRICT, and REQ-CONTRACT-AGREE;
  GAP-NO-PROD and GAP-NO-CANON.
- Acceptance: the feasibility probe passes, and the unit, property, fixture,
  snapshot, and BDD tests pass. The gates and the toolchain smoke pass.
- Conformance check:
  - Only the authorized dependencies moved.
  - No spans, strings, or serde implementations on the IR.
  - `contracts/` and `examples/` unchanged.
  - Code within its 3,400-line budget.
- Recovery: revert to EP-M1. If the probe fails, revert the probe commit only
  and escalate.
- Remaining gaps: the encoder is tested but not proved.
- Compatibility decision: none.

### `EP-M3`

- Outcome: L-CANON-REFINES, L-CANON-INJECTIVE, and L-CANON-SHAPE are proved
  over the shipped kernel. Forbidden constructs and unverified edits are caught
  mechanically.
- Requirements: REQ-CANON-FORM and REQ-CANON-UNIQUE; GAP-VERUS-SCAN.
- Acceptance: `make verus` reports 0 errors and all controls rejected;
  `make test-scripts` and the gates pass.
- Conformance check:
  - No construct the scan forbids.
  - Executable kernel statements unchanged from EP-M2, apart from ghost code.
  - Existing proofs' verified counts unchanged.
  - `make verus` still on demand, unless Q2 says otherwise.
  - Code within its 2,600-line budget.
- Recovery: the encoder's behaviour is unchanged, so reverting the proof
  leaves EP-M2 valid; the tooling commit stands on its own. If the verification
  tolerance is reached, stop and present the open condition.
- Remaining gaps: the serde-based decoder is tested, not proved.
- Compatibility decision: none.

### `EP-M4`

- Outcome: durable documents describe what exists, and roadmap 2.1.1 is done.
- Requirements: all, by documentation; RM-2.1.1.
- Acceptance: the documentation gates and the checksum manifest pass.
- Conformance check:
  - No document claims admission, proof of a policy, or runtime agreement.
  - ADR 0007 records every dependency decision and amends ADR 0005.
- Recovery: documentation only; revert or amend.
- Remaining gaps: admission (2.1.2), requests (2.1.3), and the CLI (2.1.4).
- Compatibility decision: none.

## Concrete steps

Run these from the worktree root. Each gate writes a log for later reading:

```bash
BRANCH=2-1-1-bounded-source-ingestion-and-closed-policy-ir-types
make check-fmt 2>&1 | tee /tmp/check-fmt-polean-$BRANCH.out
make typecheck 2>&1 | tee /tmp/typecheck-polean-$BRANCH.out
make lint 2>&1 | tee /tmp/lint-polean-$BRANCH.out
make test 2>&1 | tee /tmp/test-polean-$BRANCH.out
make markdownlint 2>&1 | tee /tmp/markdownlint-polean-$BRANCH.out
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

Expected `make verus` tail at EP-M3. The counts are placeholders until the
proof exists, and the control names are fixed when the controls are written:

```plaintext
verification results:: N verified, 0 errors   (policy_canonical)
control kernel-drops-atom-comma: rejected as expected
control kernel-drops-final-lf: rejected as expected
control alias-unlocked-name: rejected as expected
control drop-body-brackets: rejected as expected
```

## Validation and acceptance

The work is accepted when all of the following hold:

- Ingestion behaves as follows:
  - `PolicySource::read_from` over `std::io::repeat(b'a')` returns `TooLarge`
    after consuming at most 65,537 bytes;
  - over exactly 65,536 valid bytes, it returns those bytes unchanged;
  - over a source with a NUL at offset 20, it returns `Nul` at offset 20.
- The decoders behave as follows:
  - `Policy::from_json_slice` on every committed IR fixture returns the
    fixture's rules in order;
  - `to_canonical_bytes` returns the reviewed snapshot, equal to serde_json's
    compact rendering plus one line feed;
  - `Policy::from_canonical_bytes` accepts those bytes and rejects the
    pretty-printed fixture as `NonCanonical`;
  - each Table 1 class yields its variant.
- `make verus` proves the three canonical-encoding lemmas with no forbidden
  construct, and all controls are rejected.
- These gates pass: `make check-fmt`, `make typecheck`, `make lint`,
  `make test`, `make test-scripts`, `make toolchain-smoke`, `make design-check`,
  `make markdownlint`, and `sha256sum -c SHA256SUMS`.

## Idempotence and recovery

Every step can be re-run. Tests read committed fixtures and write nothing
outside Cargo's target directory and `insta`'s pending snapshots. The Verus
controls run on scratch copies and never touch the working tree. `make verus`
rewrites the manifest only after every proof and control succeeds. If a gate
fails part-way, fix the cause and re-run that gate, then the remaining gates in
order. Each milestone is committed, so reverting a milestone's commits restores
the previous plateau.

## Artefacts and notes

Reconnaissance evidence for the serde decisions, from the locked sources:

```plaintext
serde_derive-1.0.229/src/de/struct_.rs:269  duplicate_field(...) in derived struct visitors
serde_json-1.0.154/src/read.rs:951-1010     unpaired surrogates rejected when validating decoded strings
serde_json-1.0.154/src/read.rs:1076-1093    escapes in ignored strings are not validated
serde_json-1.0.154/src/de.rs:269            wrong-type values reported where met
serde_json-1.0.154/src/de.rs:1102-1214      ignore_value keeps a heap stack; no depth limit
serde_json-1.0.154/src/de.rs:1836-1846      deserialize_struct also accepts a JSON array
```

Prior art: Vest (Cai et al., USENIX Security 2025) proves parser and serializer
round trips in Verus over combinators. This plan borrows only its proof shape,
a specification-level parser as a left inverse, and not the library.

Seeded-fault transcripts, red-step outputs, `numstat` measurements, and the
control attribution for (c) and (d) are recorded here during implementation.

## Interfaces and dependencies

Public items at the end of this plan, all new and pre-1.0:

```rust
pub mod profile {
    pub const PROFILE_ID: &str = "polean.tenant-write.v0";
    pub const SOURCE_BYTES_MAX: usize = 65_536;
    pub const INPUT_BYTES_MAX: usize = 65_536;
    pub const RULES_MAX: usize = 16;
    pub const BODY_ATOMS_MIN: usize = 1;
    pub const BODY_ATOMS_MAX: usize = 16;
}

pub mod bounded {
    pub fn read_bounded(reader: impl std::io::Read, limit: usize)
        -> Result<Vec<u8>, BoundedReadError>;
    pub enum BoundedReadError { TooLarge { limit: usize }, Read(std::io::Error) }
}

pub mod source {
    pub struct PolicySource { /* private */ }
    impl PolicySource {
        pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, SourceError>;
        pub fn read_from(reader: impl std::io::Read) -> Result<Self, SourceError>;
        pub fn as_bytes(&self) -> &[u8];
        pub fn as_str(&self) -> &str;
        pub fn into_string(self) -> String;
    }
    pub enum SourceError {
        TooLarge { limit: usize },
        InvalidUtf8 { valid_up_to: usize },
        Nul { offset: usize },
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
        pub fn new(atoms: Vec<Atom>) -> Result<Self, RuleError>;
        pub fn atoms(&self) -> &[Atom];
    }
    pub struct Policy { /* private */ }
    impl Policy {
        pub fn new(rules: Vec<Rule>) -> Result<Self, PolicyError>;
        pub fn rules(&self) -> &[Rule];
        pub fn to_canonical_bytes(&self) -> CanonicalIr;
        pub fn from_json_slice(bytes: &[u8]) -> Result<Self, DecodeError>;
        pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, DecodeError>;
    }
    pub struct CanonicalIr { /* private */ }
    impl CanonicalIr {
        pub fn as_bytes(&self) -> &[u8];
        pub fn into_bytes(self) -> Vec<u8>;
    }
    pub const IR_DOCUMENT_BYTES_MAX: usize = 65_536;
    pub enum RuleError { Empty, TooManyAtoms { count: usize } }
    pub enum PolicyError { TooManyRules { count: usize } }
    pub enum DecodeError { /* the variants in Table 1 */ }
    pub enum JsonErrorClass { Syntax, Eof }
    pub enum Field { Profile, Rules }
}
```

Derives are as follows:

- Every type derives `Debug`.
- Value and error types without an `io::Error` also derive `Clone`,
  `PartialEq`, and `Eq`.
- `Atom` also derives `Copy`.

Error types implement `Display` and `std::error::Error` by hand. Exact
signatures may change during implementation if a lint or Verus requires it,
provided no item is added beyond this list; record any such change in
`Decision log`. In particular, `Atom::name` may lose `const` if the probe
requires it.

Dependencies after this plan: `serde =1.0.229` (`derive`) and
`serde_json =1.0.154`, plus `vstd =0.0.0-2026-09-06-0133` if Q1 is accepted.
All three move from `[dev-dependencies]` without version changes.

## Revision notes

- Revision 1 (2026-11-14): initial draft from reconnaissance.
- Revision 2 (2026-11-14): applied the community-of-experts review, whose four
  members each returned "revise".
  - Kernel boundary. Declared `Atom`, `Rule`, and `Policy` in a
    self-contained `verus!` kernel with views and constant byte arrays (D11),
    because a `#[path]`-included kernel cannot see types elsewhere in `src/`.
    Added a feasibility probe and its tolerance at the start of EP-M2.
  - Refinement. Removed `WF` from refinement's precondition.
  - Controls. Rewrote controls (c) and (d) so they change shared constants and
    can fail only through injectivity, and isolated the injectivity lemmas in
    `injective.rs`.
  - Proof sketch. Corrected the prefix-freeness argument, added the join
    bridging lemma and the ASCII clause, and fixed the length at 4,175 bytes
    as a tested invariant rather than a lemma.
  - Proof tooling. Widened the forbidden-construct scan, and added the drift
    manifest (D15) and Q2.
  - Decoder. Replaced the single-pass decoder with a two-pass decoder that
    classifies values in repository visitors (D5, D8), and published Table 1.
    Reversed D7 to reject escapes. Added the strict reader (D3) and the
    `CanonicalIr` newtype (D14).
  - Ingestion. Added NUL rejection at ingestion and the 2.1.2 entry
    requirement (D4). Added the neutral `bounded` module and
    `INPUT_BYTES_MAX`.
  - API. Split `IrError` into `RuleError` and `PolicyError`, renamed `Key` to
    `Field`, and dropped `Policy::profile`. Forbade serde implementations on
    the IR.
  - Scope of decisions. Withdrew the exit-code mapping from this plan (D13).
  - Tests and documentation. Stated the googletest and pretty_assertions
    pattern, the error-display snapshot, and the reader contract. Added
    `make typecheck` and `make markdownlint` to every milestone. Raised and
    split the budgets using measured history. Added definitions, signposts,
    and the missing `docs/references.md` note.
