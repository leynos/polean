# Produce a complete-module admission feasibility record

This ExecPlan is a living document. Keep `Constraints`, `Tolerances`, `Risks`,
`Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan`
current as work proceeds.

Status: DRAFT (revision 1). Implementation must not begin until the user
explicitly approves this plan.

Roadmap item: [1.2.2](../roadmap.md). Branch:
`1-2-2-complete-module-admission-feasibility-record`.

## Purpose / big picture

Polean's first profile, `polean.tenant-write.v0`, admits a deliberately tiny
subset of Rego: package `authz`, the single import `rego.v1`, one
`default allow := false`, and zero to 16 `allow if { ... }` rules, each holding
one to 16 of six fixed comparisons. The technical design trusts Regorus to
parse the source and requires a Rust adapter to reject everything else. Nobody
has yet shown that the pinned Regorus 0.13.0 exposes enough of a module to make
that rejection complete. Terms-of-reference assumption A5 and ADR 0002 open
question 2 ask exactly this: can the frontend API support complete fail-closed
admission without Polean becoming an unverified parser trusted wholesale?

This task answers the question with evidence rather than argument. When it is
done, a contributor can run `make test` and watch a test-only admission spike
classify a committed corpus of Rego fixtures. The corpus contains one or more
sources for every package, import, default, rule, literal, and expression form
that the Regorus 0.13.0 abstract syntax tree (AST) can represent, plus metadata
annotations, unreachable unsupported syntax, and lexical forms the AST silently
discards. The spike admits the baseline and the six committed mutations,
producing exactly their committed intermediate representation (IR), and rejects
every other fixture with a stable reason code and the byte span of the
offending text. A generated corruption of the AST JSON at every node position
is rejected as a frontend-contract failure. A reviewed `insta` snapshot
records, for every fixture, what Regorus exposed, what the AST walk concluded,
what the bounded source check concluded, and the final outcome.

The written product is `docs/admission-feasibility-record.md`. It gives a
construct-by-construct verdict: enforced through the AST, enforced by a
demonstrated bounded source check, or recorded as an E0 finding that blocks
roadmap item 2.1.2. Reconnaissance already shows that the AST alone is not
enough. Regorus discards comments, so `# METADATA` annotations leave no trace;
it drops parentheses; its lexer appears to treat an embedded NUL byte as end of
input; and it records neither the `if` keyword nor whether a rule body was
braced. The plan therefore pairs the AST walk with a small canonical-token
recognizer that accounts for every source byte, and proves that recognizer's
soundness with Verus.

None of this implements production admission, which is 2.1.2. Admission is not
proved: the four claims in AGENTS.md stay separate, and a passing spike
establishes feasibility of the frontend boundary only. It does not establish a
model theorem, a translation proof, or runtime correspondence.

## Constraints

- Keep the four claims separate (AGENTS.md, ADR 0002). No file, test name,
  diagnostic, or document may describe the spike, a fixture pass, or the Verus
  proof as proving admission, translation, or runtime agreement. The Verus
  theorem is about the recognizer's byte-level decomposition only.
- Preserve the single Rust package, the reserved Lean boundary, and the
  independent Python design lane. Do not change `src/lib.rs`, `lean/`, or
  `validation/`. Do not move Regorus out of `[dev-dependencies]`; that is 2.1.1.
- Do not widen the grammar. A form is admitted only if technical design §4.1
  already admits it. Where the design is ambiguous, choose the narrower
  reading, record it in `Decision log`, and surface it for approval (see Q1–Q5).
- Unsupported syntax fails closed, including syntax in a rule that can never
  fire. Unknown serialized AST variants and unknown fields are errors, never
  ignorable additions.
- Do not use a regular expression to extract meaning from Rego. The bounded
  source check may only confirm that the source bytes equal a token sequence
  rendered from the already-derived IR, separated by permitted whitespace and
  comments.
- Do not change `contracts/profile.json`, `contracts/*.schema.json`, or the
  committed `examples/tenant-write/` sources and IR. New fixtures live in a new
  `examples/admission/` tree, never under `examples/tenant-write/`, so that no
  oracle that loads a directory can merge them into one policy (TDD §6.3).
- Pin by exact identity. Regorus stays at `=0.13.0` with features
  `["ast", "std"]`. Verus is installed only through `prover-tools` from pinned
  `tools/verus/VERSION` and `tools/verus/SHA256SUMS` files.
- Do not bypass compile admission (`RUSTC_WRAPPER`). Use the shared Cargo
  cache. Never use `/tmp` as a build target.
- Follow the repository lint policy: no `unsafe`, no `allow` attributes, every
  module begins with `//!`, modules under 400 lines, functions under 70 lines.
- Use British English with Oxford spelling. Keep `sha256sum -c SHA256SUMS`
  passing; `docs/technical-design.md`, `docs/terms-of-reference.md`, and
  `docs/context.md` are covered files.

## Tolerances (exception triggers)

The ExecPlan file itself is excluded from every count below.

- Scope: stop and escalate if the change exceeds 40 non-fixture files or
  about 3,000 net lines of code and documentation. Fixture `.rego` files under
  `examples/admission/`, `Cargo.lock`, and `insta` snapshots are excluded from
  both counts but must stay under 150 files.
- Dependencies: the authorized new Rust dev-dependency is `proptest`, pinned
  with `=`. Verus is a tool, installed by `prover-tools`, not a Cargo
  dependency. Any other dependency, including `serde` with `derive`, requires
  escalation.
- Interface: no public item may be added to `src/`. If the spike needs
  library code, stop and escalate.
- Design: stop and set `BLOCKED` if a construct can be distinguished neither
  through the AST nor by a bounded source check. Record it as an E0 finding
  that blocks 2.1.2 and ask how to proceed. Do not weaken the profile to make
  the corpus pass.
- Verification: stop if the Verus soundness theorem cannot be proved without
  `assume`, `admit`, `external_body`, or an axiom that restates the theorem.
  Present the open verification condition rather than weakening the statement.
- Iterations: stop when a gate still fails after three fix attempts on one
  cause.
- Ambiguity: the five open questions below (Q1–Q5) are decided by the user at
  approval. A new ambiguity that changes which forms are admitted stops work.

## Risks

- Risk: the Regorus AST JSON shape differs from the derive-based inventory. A
  stale doctest at `engine.rs:1813` indexes `["refr"]["Var"][1]`, which does
  not match the serialized layout. Severity: medium. Likelihood: low, because
  the committed baseline snapshot already shows the externally tagged form.
  Mitigation: EP-M1 records the observed shape for every fixture before the
  decoder is written.
- Risk: some legal Rego form parses to an AST indistinguishable from an
  admitted one, and also renders to the same canonical tokens. Severity: high.
  Likelihood: low. Mitigation: the recognizer compares every byte, so a form
  can pass only if its bytes are the canonical tokens plus trivia. Any such
  survivor is a Design tolerance breach and becomes an E0 finding.
- Risk: the recognizer becomes a second parser that must itself be trusted.
  Severity: medium. Likelihood: medium. Mitigation: it recognizes, but never
  produces, IR. Admission is the conjunction of the AST mapping and the
  recognizer, so either can only narrow acceptance. Its soundness is proved in
  Verus. ADR 0005 records this division of trust.
- Risk: the Verus mirror drifts from the Rust kernel the tests execute, so the
  proof is about nothing. Severity: high. Likelihood: medium. Mitigation: a
  drift test compares the executable statements of the two copies (O7).
- Risk: Verus installation needs network access and a specific Rust toolchain,
  and may not support the pinned nightly. Severity: medium. Likelihood: medium.
  Mitigation: `prover-tools` installs the toolchain that Verus reports and runs
  the proof outside Cargo. If the release cannot be installed, stop under the
  Verification tolerance.
- Risk: the corpus is large and its labels might simply mirror the spike's
  behaviour. Severity: medium. Likelihood: medium. Mitigation: fixtures and
  their expected outcomes are written before the walker (Red). A provisioned
  OPA check independently confirms the legality label of every fixture (O9).

## Progress

- [x] (2026-10-29) Renamed the branch to
  `1-2-2-complete-module-admission-feasibility-record`, tracking
  `origin/1-2-2-complete-module-admission-feasibility-record`.
- [x] (2026-10-29) Reconnaissance: Regorus 0.13.0 AST inventory, parser and
  lexer behaviour, repository harness, governing documents, and OPA v1.21.2
  metadata recognition.
- [x] (2026-10-29) Drafted this ExecPlan (revision 1).
- [ ] Community-of-experts design review and revision.
- [ ] User approval, including answers to Q1–Q5.
- [ ] EP-M1: corpus, manifest, and observation snapshot (Red).
- [ ] EP-M2: closed AST walk and inventory guard.
- [ ] EP-M3: canonical-token recognizer and generated properties.
- [ ] EP-M4: Verus soundness proof of the recognizer kernel.
- [ ] EP-M5: feasibility record, ADR 0005, and documentation updates.

## Surprises & discoveries

These come from reading the pinned sources during planning. EP-M1 must confirm
each one empirically; until then they are hypotheses.

- Observation: Regorus discards comments in the lexer and has no annotation
  type, so a `# METADATA` block parses cleanly and leaves no trace in the AST.
  Evidence: `regorus-0.13.0/src/lexer.rs:874-877`; no annotation field in
  `src/ast.rs:681-696`. Impact: metadata rejection needs a bounded source
  check, as TDD §6.1 anticipated.
- Observation: OPA v1.21.2 treats a comment as metadata when it starts at
  column 1 and its whitespace-trimmed text begins with `METADATA`. Evidence:
  `v1/ast/parser.go:558-560` at tag `v1.21.2`. Impact: Polean's check must
  reject at least those comments; the plan rejects a strict superset.
- Observation: the Regorus lexer's `peek` returns `'\x00'` both at end of input
  and for a literal U+0000, and comment skipping stops there. Evidence:
  `lexer.rs:457-462`, `lexer.rs:845`, `lexer.rs:1019`. Impact: an embedded NUL
  may silently truncate the module, which would hide trailing syntax.
- Observation: parentheses are dropped from the AST, so `(a == b)` is
  indistinguishable from `a == b`. Evidence: `parser.rs:898-904`. Impact: needs
  the bounded source check.
- Observation: the AST does not record the `if` keyword or whether a body was
  braced; `allow if expr` and `allow if { expr }` differ only in span text.
  Multiple bodies (`allow if {a} {b}`) are accepted even in v1. Evidence:
  `parser.rs:1932-1992`. Impact: the walker requires exactly one body; the
  recognizer requires the braces.
- Observation: Regorus parses as Rego v1 by default even without
  `import rego.v1`, so `Module.rego_v1` is not evidence of the import. Evidence:
  `engine.rs:319`, `engine.rs:1979-1986`. Impact: the walker must inspect
  `imports` directly.
- Observation: `__target__ := "..."` is stored in `Module.target`, outside the
  rule list. Evidence: `parser.rs:2338-2365`. Impact: a declaration that a
  rule-only walk would miss; the walker must reject any present `target`.
- Observation: AST types are not `#[non_exhaustive]`, so an exhaustive `match`
  over the doc-hidden `regorus::unstable` re-exports compiles outside the crate.
  `Expr::OrExpr` exists only under the `rego-extensions` feature, which the
  pinned feature set does not enable. Evidence: `ast.rs:261`, `lib.rs:619-626`,
  `cargo tree -e features -i regorus`. Impact: a compile-time inventory guard
  is available to tests.
- Observation: span `start` and `end` are UTF-8 byte offsets; `col` is
  unreliable (tabs count as four). Evidence: `lexer.rs:310-329`, `459-512`.
  Impact: diagnostics use only byte offsets.

## Decision log

- Decision: build the spike as a separate integration-test binary,
  `tests/admission/`, not as library code. Rationale: Regorus is a
  dev-dependency, 2.1.1 owns the IR types and 2.1.2 owns production admission,
  and the roadmap asks for a feasibility record, not a feature. The spike's
  pure kernel is written to be promoted unchanged in 2.1.2. Date/Author:
  2026-10-29, planning agent.
- Decision: walk the public `Engine::get_ast_as_json` output with a closed
  decoder over `serde_json::Value`, and use the doc-hidden typed
  `regorus::unstable` AST only inside tests, as an exhaustive inventory guard.
  Rationale: the JSON route is the documented, feature-gated API and matches
  TDD §6.1 ("unknown serialized variants are errors"); the typed route offers
  compile-time exhaustiveness but carries no stability promise. Using both
  gives runtime fail-closed decoding plus a compile error when a Regorus bump
  adds a variant. The record evaluates both routes for 2.1.2. Date/Author:
  2026-10-29, planning agent.
- Decision: decode with explicit key-set checks over `serde_json::Value`
  rather than adding `serde` with `derive`. Rationale: no new dependency, and
  the closed key sets are visible in the code the record cites. Date/Author:
  2026-10-29, planning agent.
- Decision: admission is the conjunction of the AST walk and a canonical-token
  recognizer. The walker derives the IR; the recognizer renders that IR to its
  canonical token sequence and checks that the source bytes are exactly those
  tokens separated by permitted trivia (ASCII whitespace and non-metadata
  comments). Rationale: it closes every known AST gap (metadata, parentheses,
  NUL truncation, `if`, braces, separators) with one check that accounts for
  every byte, and because the expected tokens come from the IR it also catches
  walker mapping mistakes. It never produces IR, so it can only narrow
  acceptance. Alternatives considered: span-anchored residual accounting, which
  depends on Regorus spans being right; a Polean parser that produces IR, which
  is the wholesale-trust outcome ADR 0002 warns against. Date/Author:
  2026-10-29, planning agent.
- Decision: classify every rejection as `unsupported` (legal Rego outside the
  profile; result status `unsupported`, CLI exit 4), `malformed` (not legal or
  not ingestible; `invalid_input`, exit 5), or `frontend_contract` (the AST
  JSON violates the closed decoder; `tool_error`, exit 6). Rationale: matches
  `contracts/result.schema.json` and `contracts/cli.txt`; an unknown variant
  reflects a frontend change, not a fault in the user's source. 2.1.2 may
  refine the codes. Date/Author: 2026-10-29, planning agent.
- Decision: use Verus for the recognizer kernel only; use tests and
  property tests everywhere else (see `Verification plan`). Date/Author:
  2026-10-29, planning agent.
- Open question Q1 (needs approval): TDD §4.1 says to compare "decoded literal
  identity", which would admit `"write"` as `"write"`. The recognizer as
  planned compares exact canonical spellings and so rejects escaped spellings.
  Recommendation: reject escapes in v0 and amend §4.1 to say "parsed token kind
  and exact canonical spelling". Alternative: decode JSON escapes inside the
  recognizer, which enlarges the Verus proof.
- Open question Q2 (needs approval): reject raw strings (`` `write` ``), whose
  token kind differs from a quoted string. Recommendation: reject.
- Open question Q3 (needs approval): reject `;` as a body separator, the
  braceless `allow if expr` form, and single-line multi-literal bodies; accept
  newline-separated literals inside braces only. Recommendation: reject, as
  these are outside "normal whitespace".
- Open question Q4 (needs approval): reject any comment whose text contains
  the byte sequence `METADATA`, in any column, rather than reproducing OPA's
  column-1 prefix rule exactly. Recommendation: the superset, which is simpler
  and provably covers OPA's rule (O6).
- Open question Q5 (needs approval): treat a UTF-8 byte-order mark, non-ASCII
  whitespace, and C0 control bytes other than tab, line feed, and carriage
  return as rejections. The class (`malformed` versus `unsupported`) follows
  what the pinned OPA reports for each (O9). Recommendation: reject.

## Outcomes & retrospective

Not started. At completion, record the verdict for each construct class, every
E0 finding and whether it blocks 2.1.2, and lessons for 2.1.2.

## Context and orientation

Polean is a design scaffold. `src/lib.rs` exports only a status string. The
technical design (`docs/technical-design.md`) specifies a Rust orchestrator
around a Lean proof package, with Regorus as a trusted Rego frontend and OPA as
an independent oracle. Read §4.1 (admission), §6.1 (frontend), and §13 (the
"Admission coverage" row) before starting.

Terms used here:

- Admission: the decision that a Rego source belongs to the profile. It
  establishes no security property (`docs/context.md`).
- Profile: the immutable contract `polean.tenant-write.v0`, summarized in
  `contracts/profile.json`. Its `excluded` list names 20 construct classes.
- IR (intermediate representation): the ordered list of rules, each an ordered
  list of six atom names, serialized as in `examples/tenant-write/policy.json`.
- AST JSON: the string returned by Regorus's `Engine::get_ast_as_json`, an
  array of `{source, version, ast}` objects whose enums are externally tagged,
  for example `{"BoolExpr": {"span": ..., "op": "Eq", "lhs": ..., "rhs": ...}}`.
- Trivia: bytes between tokens that carry no meaning: ASCII space, tab, line
  feed, carriage return, and comments that run from `#` to the end of the line.
- Canonical tokens: the token sequence the profile renders for an IR, such as
  `package`, `authz`, `import`, `rego.v1`, `default`, `allow`, `:=`, `false`,
  `if`, `{`, `input.subject.tenant`, `==`, `"write"`, and `}`.
- Spike: test-only code that demonstrates feasibility. It is deleted or
  promoted by 2.1.2, never shipped from `src/` by this task.

Relevant existing files:

- `Cargo.toml`: Regorus `=0.13.0` with `default-features = false` and features
  `["ast", "std"]` under `[dev-dependencies]`, plus `rstest`, `rstest-bdd`,
  `insta`, `googletest`, `pretty_assertions`, and `serde_json`. Lints are
  strict; `unwrap`, `expect`, indexing, and `panic` in result functions are
  denied, so helpers return `Result<_, String>`.
- `tests/toolchain/`: the 1.2.1 observations, one integration binary. Its
  `regorus.rs` snapshots the baseline AST JSON in
  `tests/toolchain/snapshots/toolchain__regorus__baseline_ast_json.snap`; its
  `bdd.rs` shows the `rstest-bdd` 0.6.0 pattern (a `ScenarioState` with
  `Slot`s, because a `&mut` world triggers a compiler error); its `smoke/`
  shows the `#[ignore = "needs make tools; ..."]` pattern for provisioned OPA.
- `examples/tenant-write/policy.rego` and `policy.json`, plus six mutations
  under `examples/tenant-write/mutations/*/`. These are independent sentinels:
  their IR was written by hand and is validated by the Python design lane.
- `contracts/result.schema.json`: result statuses `unsupported`,
  `invalid_input`, and `tool_error`, and diagnostics with
  `source_start_byte`/`source_end_byte`. `contracts/cli.txt`: exits 4, 5, 6.
- Regorus sources for reference: run `cargo fetch --locked`, then read
  `~/.cargo/registry/src/*/regorus-0.13.0/src/{ast,parser,lexer,engine}.rs`.

Skills and guides to load: `rust-router`, then `rust-unit-testing` for test
shape, `proptest` for EP-M3, `verus` for EP-M4, and `en-gb-oxendict-style` for
prose. Read `docs/developers-guide.md`,
`docs/rust-testing-with-rstest-fixtures.md`,
`docs/reliable-testing-in-rust-via-dependency-injection.md`,
`docs/rust-doctest-dry-guide.md`,
`docs/complexity-antipatterns-and-refactoring-strategies.md`,
`docs/scripting-standards.md` (only if a script becomes necessary; none is
planned), and `docs/documentation-style-guide.md`. There is no
`docs/references.md`; the technical design's References section (R1–R13) is the
reference list, and EP-M5 adds the OPA parser source to it.

## Conformance basis

Upstream at commit `b6acb06`:

- Terms of reference draft v0.1: G1, N1, SC6, A2, and A5.
- Technical design draft v0.1: §§4.1, 6.1, 6.3, 10 (V1, V8), and the §13
  "Admission coverage" row.
- ADR 0002: decision item 3 (trusted frontend) and open question 2.
- ADR 0003: Regorus `=0.13.0`, features `["ast", "std"]`; the AST JSON is an
  adapter input, not a proof ABI.
- `docs/context.md`: admitted policy, unsupported, invalid input.
- Roadmap RM-1.2.2, whose success criteria are labelled here S1 (package,
  import, default), S2 (every rule and expression), S3 (metadata), S4 (unknown
  AST variants), S5 (missing API coverage has a bounded check or an E0
  finding), and S6 (unreachable syntax is not ignored).

```plaintext
TOR-A5, ADR-0002 OQ2        -> RM-1.2.2 -> EP-M1..M5 -> docs/admission-feasibility-record.md
TDD §4.1 package/import/default -> S1 -> EP-M1, EP-M2 -> corpus classes package/, import/, default/; O3
TDD §4.1 forms, §6.1 every node -> S2 -> EP-M2 -> inventory guard; O1, O3
TDD §4.1 metadata, §6.1 lexical -> S3, S5 -> EP-M3, EP-M4 -> recognizer; O5, O6
TDD §6.1 unknown variants       -> S4 -> EP-M2 -> AST JSON mutation sweep; O2
TDD §4.1 never ignore unreachable -> S6 -> EP-M1, EP-M2 -> corpus class unreachable/; O3
TOR-N1, TOR-SC6, TDD V8         -> EP-M2, EP-M3 -> fail-closed classification; O3, O8
TDD §7 import(render(p)) = p    -> EP-M3 -> render-then-admit property; O4
```

This task does not discharge ingestion (2.1.1), production admission and
diagnostic source maps (2.1.2), or hostile-source coverage of the release
(4.2.3). It answers A5 and ADR 0002 open question 2 for the pinned tuple only.

## Verification plan

The spike introduces three things worth stating precisely: a classification
function from source bytes to outcome, a recognizer whose acceptance implies a
byte-level decomposition, and a coverage relation between the Regorus AST
inventory and the fixture corpus. Their obligations follow, with the method
chosen for each.

### Where Verus is warranted, and where it is not

Verus is warranted for the recognizer kernel (O5, O6). The kernel is a small
pure function over byte sequences of unbounded length. The property is the
soundness of the bounded check that the feasibility record relies on, and a
counterexample would be a silent admission. No finite test set establishes it
for all inputs. The kernel is stable, independent of Regorus, and written to be
promoted unchanged into 2.1.2, so the proof is not thrown away.

Verus is not warranted for the AST walk, which is a finite case analysis over a
closed set of node kinds. Compiler exhaustiveness, a fixture for every kind,
and an exhaustive per-position mutation sweep give stronger evidence for that
structure than a proof over a hand-written mirror of Regorus types would. It is
not warranted for diagnostics formatting, manifest loading, or snapshot
plumbing either. Kani adds nothing that Verus and the property tests do not
already cover, and `cargo-mutants` is not installed and not needed for a spike.

### Proof sketch for the recognizer kernel

The abstract model is independent of Regorus and of the IR types. A source is a
byte sequence `s`. A token is a non-empty byte sequence drawn from the fixed
canonical vocabulary. A token is a word if it begins and ends with an
identifier byte (`[A-Za-z0-9_.]`); `{`, `}`, `==`, `:=`, and quoted strings are
not words.

```plaintext
spec ws(b)          := b ∈ {0x20, 0x09, 0x0A, 0x0D}
spec comment_byte(b):= b = 0x09 ∨ b = 0x0D ∨ (0x20 ≤ b ∧ b ≠ 0x7F)
spec comment(c)     := c[0] = '#' ∧ ∀ i ≥ 1. comment_byte(c[i])
                       ∧ ¬ contains(c, "METADATA")
spec trivia(t, last):= t splits into pieces, each a single ws byte or a
                       comment, where every comment is immediately followed
                       by 0x0A inside t, except that when last holds the
                       final piece may be a comment ending at the end of t
spec separated(a, t, b) := word(a) ∧ word(b) → |t| > 0
spec decomposes(s, k[1..n]) :=
     ∃ t[0..n]. s = t0 ++ k1 ++ t1 ++ … ++ kn ++ tn
              ∧ ∀ i < n. trivia(t_i, false) ∧ trivia(t_n, true)
              ∧ ∀ 1 ≤ i < n. separated(k_i, t_i, k_{i+1})
```

The comment rule forbids a token on the same line as a preceding comment,
because Rego would read that token as part of the comment. The separation rule
forbids `falseallow` being read as two tokens.

Executable contract:

```plaintext
exec recognize(src: &[u8], toks: &[&[u8]]) -> Result<(), usize>
  ensures Ok(())  ⇒ decomposes(src@, toks@)
          Err(i)  ⇒ i ≤ src.len()
```

Lemma hypotheses, to be refined when the verification conditions appear:

1. `lemma_skip_trivia_sound`: if the executable trivia skipper moves from
   offset `i` to `j`, then `src[i..j]` satisfies `trivia(_, j = len)`, and if
   it stopped before the end, `src[j]` is neither whitespace nor `#`.
2. `lemma_decomposition_extends`: a decomposition of a prefix, extended by
   trivia and the next expected token, is a decomposition of the longer prefix
   with one more token.
3. `lemma_metadata_superset`: for every comment body `c` and every subrange
   `trim(c)` of `c`, if `trim(c)` starts with `METADATA` then
   `contains(c, "METADATA")`. With the axiom that Go's `bytes.TrimSpace`
   returns a contiguous subrange of its input, this shows that every comment
   OPA treats as metadata fails `comment`, whatever its column.
4. `theorem_recognize_sound`: the executable contract above, by loop
   invariant over the token index, using 1 and 2.

Completeness, meaning that canonical renderings with arbitrary permitted trivia
are accepted, is an availability property. It is checked by property tests (O4)
rather than proved. The proof makes no claim about how Rego interprets the
tokens; that is the lexical axiom X3 below.

### Obligations

- O1, inventory completeness. Every variant of the Regorus 0.13.0 types
  `Rule` (2), `RuleHead` (3), `Literal` (5), `Expr` (21), `BoolOp` (6),
  `AssignOp` (2), `BinOp` (2), and `ArithOp` (5) has a disposition in the
  coverage vocabulary and at least one fixture whose parsed AST contains it.
  - Method: an exhaustive `match`, without a wildcard arm, from each typed
    `regorus::unstable` enum to a Polean-owned kind name, plus a test that the
    kinds observed across the corpus include every name in the vocabulary.
  - Artefact: `tests/admission/inventory.rs`, `tests/admission/coverage.rs`.
  - Evidence: `make test`. A Regorus bump that adds a variant fails to
    compile.
  - Non-vacuity: deleting the only fixture for `Expr::Membership` makes the
    witness test fail; adding a kind name with no fixture fails it too.
- O2, closed decoding. Any unknown enum tag, unknown object key, missing
  required key, or wrong JSON type anywhere in AST JSON is rejected as
  `frontend_contract` with code `ast.*`.
  - Method: an exhaustive sweep. For the AST JSON of every admitted fixture,
    apply each of four mutations at every object and tagged-enum position, and
    classify each result.
  - Artefact: `tests/admission/ast_mutations.rs`.
  - Evidence: `make test`; the test reports the number of mutants per class.
  - Non-vacuity: every class produces at least one mutant per admitted
    fixture, and the unmutated JSON is admitted. Omitted keys that Regorus
    legitimately skips (`with_mods`, `target`, `as`) are handled as absent
    defaults, and their presence is a separate rejection.
- O3, fail-closed classification over the corpus. Every fixture yields the
  outcome its manifest entry states. Admitted fixtures yield the stated IR.
  Rejected fixtures yield the stated class and code, and a span `[start, end)`
  with `start ≤ end ≤ len` whose text equals the manifest's `span_text`.
  - Method: an `rstest` table driven by `examples/admission/manifest.json`,
    plus a bijection test between manifest entries and `.rego` files.
  - Artefact: `tests/admission/corpus.rs`.
  - Non-vacuity: the corpus contains admitted fixtures (the baseline and six
    mutations), and every class in the vocabulary has a rejected fixture.
- O4, faithful mapping and completeness on generated canonical sources.
  For every IR with 0–16 rules of 1–16 atoms, rendered canonically with
  generated permitted trivia (spaces, tabs, CRLF, blank lines, comments without
  `METADATA`), admission returns exactly that IR.
  - Method: `proptest` over structured values (IR plus trivia choices), so
    shrinking preserves syntax validity.
  - Artefact: `tests/admission/properties.rs`.
  - Non-vacuity: the generator's classification counts show the empty IR, the
    16-by-16 maximum, and comment-bearing trivia; a seeded fault (swapping the
    rendered `unlocked` atom for `locked`) fails the property.
- O5, recognizer soundness. `theorem_recognize_sound` above.
  - Method: Verus.
  - Artefact: `verus/admission_recognizer.rs`.
  - Evidence: `make verus` reports zero errors. Before the proof is complete,
    the run shows the open postcondition, which is the Red state.
  - Non-vacuity: the antecedent is inhabited, shown by a proof that
    `recognize` accepts one concrete baseline token sequence with its trivia.
    The proof imports no axiom except X4, and `assume`, `admit`, and
    `external_body` are absent (checked by `grep` in the milestone).
- O6, metadata superset. `lemma_metadata_superset` above.
  - Method: Verus, plus a parameterized test over OPA's own examples (column
    1, indented, and `#METADATA` without a space).
  - Non-vacuity: a comment containing `METADATA` mid-line is rejected, and a
    comment containing `metadata` in lower case is accepted (it is not in the
    superset by design and OPA ignores it).
- O7, the proof is about the executed code. The executable statements of
  `recognize` and its helpers in `tests/admission/recognizer/kernel.rs` equal
  those in `verus/admission_recognizer.rs` once ghost clauses are removed.
  - Method: a test that strips `requires`, `ensures`, `invariant`,
    `decreases`, and `proof { ... }` blocks from the Verus file and compares
    the remaining function bodies.
  - Non-vacuity: editing one comparison in the kernel makes it fail.
- O8, inserted unsupported syntax is rejected anywhere. Inserting any entry
  from a catalogue of unsupported fragments (one per excluded class) at any
  literal, rule, or top-level position of a generated valid policy yields a
  rejection.
  - Method: `proptest` with a structured insertion position.
  - Non-vacuity: insertion of an admitted atom at a literal position is
    admitted, so the property does not pass merely because insertion breaks
    parsing.
- O9, legality labels are independent. Every fixture labelled `unsupported`
  or `admitted` is accepted by the pinned OPA's parser, and every fixture
  labelled `malformed` is rejected by it.
  - Method: an ignored provisioned test that runs
    `opa parse --format json --v1-compatible` on each fixture, with
    `env_clear()` and an argument array.
  - Artefact: `tests/admission/smoke.rs`, run by `make toolchain-smoke`.
  - Non-vacuity: the corpus contains fixtures of both labels.

### External axioms, not verified here

- X1: Regorus 0.13.0 parses the admitted fixtures into the AST its source
  describes (ADR 0002, trusted frontend). The recognizer reduces this trust but
  does not remove it.
- X2: OPA v1.21.2 recognizes metadata exactly as `IsMetadataComment` at
  `v1/ast/parser.go:558-560`.
- X3: a Rego lexer, given a source that `decomposes` into canonical tokens,
  produces exactly those tokens. O9 and the Regorus parse of every admitted
  fixture exercise this boundary; neither proves it.
- X4: Go's `bytes.TrimSpace` returns a contiguous subrange of its input.
- X5: `serde_json` reports duplicate-free objects as Regorus serialized them;
  Regorus's AST JSON is produced by its `Serialize` derives and the custom
  `Value` serializer.

Residual gap: the record establishes feasibility for the pinned tuple only. It
proves nothing about Rego semantics, about Regorus's parser, or about the IR's
meaning, and a Regorus upgrade repeats the whole exercise.

## Plan of work

### Stage A: confirm and propose (no code)

Complete the community-of-experts review, revise this plan, and obtain user
approval and answers to Q1–Q5. Stop here until approval.

### Stage B: corpus and Red tests (EP-M1)

Create `examples/admission/` with one `.rego` file per construct, grouped in
directories `package/`, `import/`, `default/`, `head/`, `body/`, `literal/`,
`expr/`, `operator/`, `atom/`, `limits/`, `trivia/`, `unreachable/`, and
`module/`. Each fixture is the baseline with one deliberate difference, and
starts with a one-line comment stating that difference. The manifest
`examples/admission/manifest.json` lists every fixture with its `covers` labels
and its expected outcome. It also references the seven committed
`examples/tenant-write/` sources as admitted fixtures, pointing at their
committed `policy.json` files rather than copying them. Fixtures that need
bytes JSON cannot carry (NUL, invalid UTF-8, a byte-order mark, a 64 KiB
overrun) are generated in the test and listed in the manifest by name.

The coverage vocabulary is the union of the inventory kind names, the 20
`excluded` entries of `contracts/profile.json` (two of which, "arbitrary Lean
source" and "custom invariants", are marked not applicable to source
admission), the RM-1.2.2 categories, and the lexical classes found in
reconnaissance.

Write `tests/admission/main.rs` and the corpus, coverage, and observation
tests. The classifier is a stub returning a `not implemented` rejection, so O3
fails for every fixture (Red). The observation snapshot records what Regorus
itself does with each fixture: parse success or error, the set of node kinds,
and module count. This snapshot is the empirical record of what the AST
exposes, and confirms or refutes each item in `Surprises & discoveries`.

### Stage C: implementation with verification (EP-M2, EP-M3, EP-M4)

EP-M2 writes the closed decoder and walker, one small function per node kind,
and the typed inventory guard. After it, every AST-detectable fixture passes,
and the fixtures that only the source check can catch (metadata, parentheses,
NUL, braceless body, separators) are recorded in the snapshot as "AST walk
admitted". That is the demonstrated gap, kept as evidence rather than hidden.

EP-M3 writes the canonical-token renderer and the recognizer kernel, wires
admission as walker-then-recognizer, and adds the `proptest` properties and the
BDD scenarios. After it, the whole corpus passes.

EP-M4 installs Verus through `prover-tools`, writes the specification first
with the theorem statements and an empty proof body (Red: Verus reports the
failing postcondition), then the lemmas and loop invariants until Verus reports
success, and finally the drift test.

### Stage D: record and documentation (EP-M5)

Write the feasibility record and ADR 0005, amend the design documents, tick the
roadmap, and run every gate.

## Milestones and plateaus

- EP-M1, corpus and observations (Red).
  - End state: the corpus, manifest, and observation snapshot are committed.
    The bijection and coverage-vocabulary tests pass; O3 fails for every
    fixture against the stub classifier. To keep `make test` green at the
    commit, the O3 table is marked `#[ignore = "red until EP-M2"]`, and the
    Red run is recorded in `Artefacts and notes` with
    `cargo test --test admission -- --ignored corpus`.
  - Requirements: S1, S2, S3, S6 (fixtures exist).
  - Evidence: `make check-fmt`, `make lint`, `make test`; the Red transcript.
  - Conformance: no `src/` change; `examples/tenant-write/` untouched; no new
    dependency yet.
  - Recovery: revert the commit.
  - Remaining gaps: classification.
  - Compatibility: none.
- EP-M2, closed AST walk.
  - End state: O1, O2, and O3 pass for every fixture the AST can decide. The
    lexical-only fixtures remain on a named list whose observed outcome is
    "AST walk admitted"; that list is asserted, so it cannot silently grow.
  - Requirements: S1, S2, S4, S6.
  - Evidence: `make check-fmt`, `make lint`, `make test`; snapshot reviewed.
  - Conformance: decoder uses `serde_json::Value` only; no new dependency.
  - Recovery: revert; EP-M1 remains a coherent Red state.
  - Remaining gaps: S3, S5.
  - Compatibility: none.
- EP-M3, recognizer and properties.
  - End state: the whole corpus is classified as the manifest states, the
    lexical-only list is empty, O4 and O8 pass, the BDD feature passes.
  - Requirements: S3, S5.
  - Evidence: `make check-fmt`, `make lint`, `make test`.
  - Conformance: `proptest` is the only new dependency; the recognizer never
    produces IR.
  - Recovery: revert; EP-M2 remains coherent.
  - Remaining gaps: the soundness proof.
  - Compatibility: none.
- EP-M4, Verus soundness proof.
  - End state: `make verus` proves O5 and O6 with no `assume`, `admit`, or
    `external_body`, and O7 passes. `make verus` is an on-demand target, not
    part of `make all` or CI.
  - Requirements: S5 (the bounded check is demonstrated and proved sound).
  - Evidence: `make install-verus`, `make verus`, `make check-fmt`,
    `make lint`, `make test`.
  - Conformance: Verus pinned through `tools/verus/` and
    `tools/rust-prover-tools/REF`; no Cargo dependency.
  - Recovery: revert; EP-M3 remains coherent with tested but unproved
    soundness.
  - Remaining gaps: documentation.
  - Compatibility: none.
- EP-M5, record and documentation.
  - End state: the record, ADR 0005, and amended documents agree with the
    code; roadmap 1.2.2 is ticked; every gate passes.
  - Requirements: all of S1–S6; A5 answered.
  - Evidence: every gate in `Validation and acceptance`, including
    `make toolchain-smoke` for O9 and `sha256sum -c SHA256SUMS`.
  - Conformance: no edited sentence widens one of the four claims.
  - Recovery: revert documentation commits individually.
  - Compatibility: none.

If EP-M2 or EP-M3 finds a construct that neither check can distinguish, the
plan stops under the Design tolerance. With the user's agreement, EP-M5 then
records it as an E0 finding that blocks 2.1.2; that is a valid outcome of a
feasibility task, not a failure to complete it.

## Concrete steps

Run every command from the worktree root. Tee each gate to
`/tmp/<action>-polean-1-2-2-complete-module-admission-feasibility-record.out`
and read the log rather than re-running a gate. Run gates sequentially, never
in parallel.

```bash
cargo fetch --locked
make check-fmt 2>&1 | tee /tmp/check-fmt-polean-1-2-2-complete-module-admission-feasibility-record.out
make lint 2>&1 | tee /tmp/lint-polean-1-2-2-complete-module-admission-feasibility-record.out
make test 2>&1 | tee /tmp/test-polean-1-2-2-complete-module-admission-feasibility-record.out
cargo test --test admission -- --ignored corpus
cargo insta review
make install-verus
make verus 2>&1 | tee /tmp/verus-polean-1-2-2-complete-module-admission-feasibility-record.out
make toolchain-smoke 2>&1 | tee /tmp/toolchain-smoke-polean-1-2-2-complete-module-admission-feasibility-record.out
make markdownlint
make design-check
sha256sum -c SHA256SUMS
```

Expected `make verus` tail on success:

```plaintext
verification results:: N verified, 0 errors
```

Commit after each milestone with an imperative subject, explaining the reason
and assurance impact in the body.

## Validation and acceptance

The behaviour to observe, in the user's terms:

- `make test` passes. In its output, `admission::corpus` runs one case per
  manifest entry, `admission::ast_mutations` reports mutant counts per class,
  and the properties report their generated-case classifications.
- The snapshot `tests/admission/snapshots/admission__observations__corpus.snap`
  has one line per fixture, for example:

  ```plaintext
  trivia/metadata-block.rego  regorus=parsed  walk=admitted  recognizer=rejected source.metadata [0, 10) "# METADATA"  outcome=unsupported
  ```

- `make verus` reports zero errors; `make toolchain-smoke` passes, including
  the O9 legality check.
- `docs/admission-feasibility-record.md` lists every construct class with its
  verdict and the fixture or test that demonstrates it.

Red-Green-Refactor evidence:

- Red: at EP-M1, the ignored O3 table fails for every fixture with
  `not implemented`. At EP-M4, Verus reports the unproved postcondition of
  `recognize`.
- Green: the same commands pass after EP-M2/EP-M3 and EP-M4.
- Refactor: rerun `make check-fmt`, `make lint`, `make test`, and `make verus`.

BDD specification, `tests/features/admission_feasibility.feature`:

```plaintext
Feature: Complete-module admission feasibility for polean.tenant-write.v0

  Scenario: The baseline is admitted with its committed IR
    Given the baseline tenant-write source
    When the admission spike classifies it
    Then it is admitted with the IR in examples/tenant-write/policy.json

  Scenario: A metadata annotation is rejected although Regorus discards it
    Given the baseline source preceded by a METADATA comment block
    When the admission spike classifies it
    Then Regorus parses it without error
    And it is rejected as unsupported with code source.metadata

  Scenario: Unsupported syntax in a rule that never fires is rejected
    Given the baseline source with an extra rule containing a not literal
    When the admission spike classifies it
    Then it is rejected as unsupported with code literal.kind

  Scenario: An unknown AST variant is a frontend contract failure
    Given the baseline AST JSON with one Expr tag renamed
    When the admission spike decodes it
    Then it is rejected as frontend_contract with code ast.unknown_variant

  Scenario: A parenthesized comparison is caught by the bounded source check
    Given the baseline source with one comparison wrapped in parentheses
    When the admission spike classifies it
    Then the AST walk alone would admit it
    And it is rejected as unsupported with code source.residual
```

No end-to-end test is added: the task introduces no command, persisted format,
or network boundary. The provisioned O9 check is the only external process.

Quality criteria:

- Tests: `make test` passes; O1–O4, O7, and O8 pass; O9 passes under
  `make toolchain-smoke`.
- Verification: `make verus` discharges O5 and O6 with no trusted shortcuts.
- Lint and format: `make check-fmt`, `make lint`, `make typecheck`, and
  `make markdownlint` pass.
- Documentation: `make design-check` passes and `sha256sum -c SHA256SUMS`
  passes after refreshing covered files.

## Idempotence and recovery

Every test is deterministic; `proptest` regression files are committed under
`tests/admission/` if a failure is ever found. `make install-verus` installs
into an untracked, version-named directory and can be rerun. If `insta` reports
a changed snapshot, review it with `cargo insta review`; a change means the
frontend or the spike changed what it exposes, which must be explained in
`Surprises & discoveries`. Each milestone is one or more commits that can be
reverted independently.

## Artefacts and notes

To be filled during implementation: the EP-M1 Red transcript, the observation
snapshot summary, mutant counts, generator classifications, the Verus Red and
Green transcripts, and the O9 OPA transcript.

## Interfaces and dependencies

Everything below lives under `tests/admission/` and is private to that test
binary. No public `src/` item is added.

```rust
/// Final outcome of classifying one source.
enum Outcome {
    Admitted(PolicyIr),
    Rejected(Rejection),
}

/// Ordered rules of ordered atom names, as in `policy.json`.
struct PolicyIr { rules: Vec<Vec<Atom>> }

enum Atom { SameTenant, WriteAction, AdminRole, OwnsResource, Locked, Unlocked }

struct Rejection {
    class: RejectionClass,
    code: &'static str,   // e.g. "literal.kind", "source.metadata"
    span: ByteSpan,
}

enum RejectionClass { Unsupported, Malformed, FrontendContract }

struct ByteSpan { start: usize, end: usize }

/// Classify source bytes: size and UTF-8 checks, Regorus parse, closed AST
/// walk, then the canonical-token recognizer.
fn classify(source: &[u8]) -> Outcome;

/// Closed decoder and profile mapping over Regorus AST JSON.
fn walk_ast_json(ast_json: &str) -> Result<(PolicyIr, Vec<NodeWitness>), Rejection>;

/// Canonical tokens for an IR, in source order.
fn canonical_tokens(ir: &PolicyIr) -> Vec<&'static [u8]>;

/// The proved kernel: `Ok` only if `src` decomposes into `toks` with
/// permitted trivia; otherwise the offset of the first unaccounted byte.
fn recognize(src: &[u8], toks: &[&[u8]]) -> Result<(), usize>;
```

Module layout, each under 400 lines: `main.rs`, `outcome.rs`, `corpus.rs`,
`coverage.rs`, `inventory.rs`, `walk/{mod,decode,profile}.rs`,
`recognizer/{mod,kernel,render}.rs`, `ast_mutations.rs`, `properties.rs`,
`bdd.rs`, `observations.rs`, and `smoke.rs`. Children with tests use `mod.rs`
plus `tests.rs`, because `self_named_module_files` is denied.

Dependencies: Regorus `=0.13.0` (existing), `serde_json` (existing), `rstest`,
`rstest-bdd`, `rstest-bdd-macros`, `insta`, `googletest`, and
`pretty_assertions` (existing), and `proptest` (new, `=`-pinned, dev-only).
Verus through `prover-tools`, pinned by `tools/verus/VERSION`,
`tools/verus/SHA256SUMS`, and `tools/rust-prover-tools/REF`, with Makefile
targets `install-verus` and `verus` following the `verus` skill's on-ramp.

Documentation changes in EP-M5:

- `docs/admission-feasibility-record.md` (new): the question, the method, the
  verdict per construct class with links to fixtures and tests, the AST gaps
  and the bounded check that closes each, E0 findings, and what 2.1.2 must
  carry forward.
- `docs/adrs/adr-0005-admit-by-ast-mapping-and-canonical-token-recognition.md`
  (new): the conjunction design, the trust it removes and retains, and the
  alternatives considered.
- `docs/technical-design.md`: §4.1 wording for Q1–Q5 as approved; §6.1 a
  summary of findings and a link to the record; §13 the "Admission coverage"
  row; References gains the OPA parser source.
- `docs/terms-of-reference.md`: the A5 resolution evidence cites the record.
- `docs/roadmap.md`: tick 1.2.2.
- `docs/developers-guide.md`: the admission spike, the corpus and manifest
  conventions, `make verus`, and what the proof does and does not cover.
- `docs/repository-layout.md`: rows for `tests/admission/`,
  `examples/admission/`, `verus/`, and `tools/`.
- `docs/contents.md`: the record, ADR 0005, and this plan.
- `docs/users-guide.md`: no change, because nothing user-visible changes; the
  record says so.
- `SHA256SUMS`: refreshed for every covered file edited.

## Revision notes

- Revision 1 (2026-10-29): initial draft from Wyvern reconnaissance of the
  Regorus 0.13.0 sources, the repository harness, the governing documents, and
  OPA's metadata recognition. Awaiting expert review.
