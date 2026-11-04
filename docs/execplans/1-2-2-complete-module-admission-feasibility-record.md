# Produce a complete-module admission feasibility record

This ExecPlan is a living document. Keep `Constraints`, `Tolerances`, `Risks`,
`Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan`
current as work proceeds.

Status: APPROVED (revision 5). The user accepted Q1 and Q2 and approved the
plan to begin implementation on 2026-10-29. See `Revision notes` for the
2026-11-02 scope-exception record and corpus reconciliation.

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

This task answers the question with evidence. When it is done, a contributor
runs `make test` and watches a test-only admission spike classify a committed
corpus of Rego fixtures. The corpus holds at least one source for every
package, import, default, rule, literal, and expression form the Regorus 0.13.0
abstract syntax tree (AST) can represent. It also holds metadata annotations,
unsupported syntax in rules that never fire, and lexical forms the AST silently
discards. The spike admits the baseline, the six committed mutations, and one
new hand-written fixture that uses the `locked` atom, producing exactly their
committed intermediate representation (IR). It rejects every other fixture and
names the stage and reason. Every structural corruption of the AST JSON of
every parsed fixture is rejected before any profile decision is made. A reviewed
`insta` snapshot records, for each fixture, what Regorus exposed, what the AST
walk decided, what the bounded source check decided, and the final outcome.

The written product is `docs/admission-feasibility-record.md`. It gives a
construct-by-construct verdict: enforced through the AST, enforced by a
demonstrated bounded source check, or recorded as an E0 finding that blocks
roadmap item 2.1.2. Reconnaissance already shows that the AST alone is not
enough. Regorus discards comments, so `# METADATA` annotations leave no trace.
It drops parentheses. Its lexer treats an embedded NUL byte as end of input. It
records neither the `if` keyword nor whether a rule body was braced. The plan
therefore pairs the AST walk with a small canonical-token recognizer that
accounts for every source byte and returns a checkable witness. A Verus proof
establishes the recognizer's soundness for inputs of any length.

None of this implements production admission. Ingestion and IR types belong to
2.1.1, and production admission with diagnostic source maps belongs to 2.1.2.
Admission is not proved: the four claims in AGENTS.md stay separate, and a
passing spike shows only that the frontend boundary is feasible. It establishes
no model theorem, no translation proof, and no runtime correspondence.

## Constraints

- Keep the four claims separate (AGENTS.md, ADR 0002). No file, test name,
  diagnostic, or document may describe the spike, a fixture pass, or the Verus
  proof as proving admission, translation, or runtime agreement. The Verus
  theorem concerns the recognizer's byte-level decomposition only.
- Preserve the single Rust package, the reserved Lean boundary, and the
  independent Python design lane. Do not change `src/`, `lean/`, or
  `validation/`. Regorus stays in `[dev-dependencies]` until 2.1.1.
- Do not build 2.1.1 or 2.1.2 early. The spike has no size cap or UTF-8 gate,
  no CLI exit mapping, no stable diagnostic messages, and no source-map
  sidecar. Its IR type is a throwaway test type. Only the recognizer kernel and
  its proof are intended for promotion.
- Do not widen the grammar. A form is admitted only if technical design §4.1
  admits it. Where the design is ambiguous, take the narrower reading and
  record it in `Decision log` (D7–D13). D13 is the user's decision on Q1;
  D7–D12 are the planning agent's narrowings, reviewed with the plan.
- Unsupported syntax fails closed, including syntax in a rule that can never
  fire. Unknown serialized AST variants and unknown fields are errors, never
  ignorable additions.
- Do not use a regular expression to extract meaning from Rego. The bounded
  source check only confirms that the source bytes equal a token sequence
  rendered from the already-derived IR, separated by permitted trivia.
- The recognizer's expected tokens are rendered from the `atoms` strings in
  `contracts/profile.json`. The walker's atom patterns are written separately
  by hand. Neither may read the other's table.
- Do not change `contracts/`, `examples/tenant-write/`, or `SHA256SUMS`
  coverage of them. New fixtures live in a new `examples/admission/` tree,
  never under `examples/tenant-write/`, so that no oracle that loads a
  directory can merge them into one policy (TDD §6.3).
- Pin by exact identity. Regorus stays at `=0.13.0` with features
  `["ast", "std"]`. `vstd` is pinned at the exact version already in
  `Cargo.lock`, and the matching Verus release is pinned in `tools/verus/`
  (D14).
- Do not bypass compile admission (`RUSTC_WRAPPER`). Use the shared Cargo
  cache. Never use `/tmp` as a build target.
- Follow the lint policy: no `unsafe`, no `allow` attributes (a scoped
  `#[expect(..., reason = "...")]` is permitted), every module begins with
  `//!`, modules stay under 400 lines, and functions under 70 lines.
- Use British English with Oxford spelling. `sha256sum -c SHA256SUMS` must
  pass at EP-M5; `docs/technical-design.md`, `docs/terms-of-reference.md`, and
  `docs/context.md` are covered files. It already fails on `main` (see
  `Surprises & discoveries`), so EP-M5 refreshes those entries only after
  reviewing the drifted content.

## Tolerances (exception triggers)

The ExecPlan file is excluded from every count below. Fixture data, meaning the
`.rego` files and `manifest.json` under `examples/admission/`, `Cargo.lock`, and
`insta` snapshots, is also excluded from the line and file counts.

- Scope: stop and escalate above 40 non-fixture files or about 4,000 net lines
  of code and documentation. Fixture files must stay under 90.
- Dependencies: the authorized new dev-dependencies are `proptest`, pinned
  with `=`, and `vstd` at exactly the locked `0.0.0-2026-09-06-0133` (D14). If
  the matching Verus release requires a different `vstd`, stop and escalate.
  Any other dependency, including `serde` with `derive`, requires escalation.
- Interface: no item may be added to `src/`. If the spike needs library code,
  stop and escalate.
- Design: stop and set `BLOCKED` if a construct can be distinguished neither
  through the AST nor by the bounded source check. Record it as a proposed E0
  finding that blocks 2.1.2 and ask how to proceed. Do not weaken the profile
  to make the corpus pass.
- Verification: stop if the Verus theorem cannot be proved without `assume`,
  `admit`, `external_body`, or an axiom that restates the theorem. Present the
  open verification condition rather than weakening the statement. Stop too if
  the Verus-annotated kernel cannot satisfy the repository's Clippy policy
  without an `allow`.
- Iterations: stop when a gate still fails after three fix attempts on one
  cause.
- Ambiguity: a newly discovered ambiguity that would admit more than the
  narrowest reading stops work.

## Risks

- Risk: the AST JSON shape differs from the derive-based inventory. A stale
  doctest at `engine.rs:1813` indexes `["refr"]["Var"][1]`, which does not
  match the serialized layout. Severity: medium. Likelihood: low; the committed
  baseline snapshot shows the externally tagged form. Mitigation: EP-M1 records
  the observed node kinds of every fixture before the decoder is written, and
  O1 compares the JSON view with the typed view.
- Risk: a legal Rego form parses to an admitted AST and also renders to the
  same canonical bytes. Severity: high. Likelihood: low. Mitigation: the
  recognizer compares every byte, so such a form must be the canonical tokens
  plus trivia. Any survivor is a Design tolerance breach and becomes an E0
  finding.
- Risk: the recognizer becomes a second parser that must itself be trusted.
  Severity: medium. Likelihood: medium. Mitigation: it never produces IR.
  Admission is the conjunction of the AST mapping and the recognizer, so each
  can only narrow acceptance. Each `Ok` returns a witness that a structurally
  independent checker validates — it re-derives the offsets by direct slicing
  and a separate gap traversal rather than reusing the kernel's loops, though
  it shares the kernel's `has_metadata` helper — and O7 cross-checks the token
  stream against Regorus's own lexer. ADR 0005 records this division of trust.
- Risk: comment text that parsers ignore but editors render misleadingly.
  Examples are a byte-order mark (BOM, U+FEFF) inside a comment, which OPA
  rejects but Regorus skips; U+2028 or U+0085, which editors show as a line
  break; bidirectional controls; and a bare carriage return. Severity: high.
  Likelihood: medium. Mitigation: comment bodies are restricted to printable
  ASCII and tab (D8), with a fixture for each case.
- Risk: Regorus rejects some sources that OPA accepts: a leading BOM, a bare
  carriage return, a line longer than 1,024 columns, more than 20,000 lines,
  and a number literal beyond `f64` range. EP-M1 observed the number case
  failing inside `add_policy` at parse rather than in `get_ast_as_json`; see
  D15, which refutes D12's prediction. Severity: low, because the effect is
  fail-closed. Likelihood: high. Mitigation: fixtures make each divergence
  visible, and the record lists them for 2.1.1 and 2.3.2.
- Risk: the Verus attribute style, `vstd`, or the matching Verus release is
  unavailable or conflicts with the strict Clippy policy. Severity: medium.
  Likelihood: medium. Mitigation: EP-M3 already delivers a witness-checked
  recognizer, so EP-M4 can stop without losing tested soundness. The fallback
  in Q2 records the specification as a 2.1.2 obligation.

## Progress

- [x] (2026-10-29) Renamed the branch to
  `1-2-2-complete-module-admission-feasibility-record`, tracking
  `origin/1-2-2-complete-module-admission-feasibility-record`.
- [x] (2026-10-29) Wyvern reconnaissance: the Regorus 0.13.0 AST inventory,
  parser and lexer behaviour, the repository harness, the governing documents,
  and OPA v1.21.2's metadata rule.
- [x] (2026-10-29) Drafted revision 1 (commit `e88502e`).
- [x] (2026-10-29) Community-of-experts review: red team, Rust frontend and
  contracts, formal methods and alternatives, and proportionality. Verdict:
  proceed with conditions. Revision 2 adopts the conditions; see
  `Revision notes`.
- [x] (2026-10-29) The user accepted Q1 (reject escaped spellings) and Q2
  (prove the recognizer in Verus now); recorded as D13 and D14.
- [x] (2026-10-29) User approval of the plan to begin implementation.
- [x] (2026-10-30) EP-M1: corpus, manifest, and observation snapshot.
  `examples/admission/` holds 93 fixtures and 14 IR files described by 105 JSON
  Lines manifest entries; `tests/admission/` holds the inventory, the strict
  manifest parser, twelve bijection and contract tests, one observed-coverage
  test, and the observation snapshot. All 18 tests pass. The Red O3 stub run is
  recorded under `Artefacts and notes`. No `src/` change,
  `examples/tenant-write/` untouched, no new dependency.
- [x] (2026-10-30) EP-M2: closed AST decoding, profile walk, and inventory
  guard. `tests/admission/ast/` decodes the frontend's AST JSON through fixed
  key and variant tables; `tests/admission/admission/` walks the decoded module
  in D16's precedence; `tests/admission/ast_mutations/` mutates every parsed
  fixture's AST in ten classes. All 37 tests pass: O1 compares 46 kind counts
  per fixture, O2 rejects all 77,884 mutants at stage `decode` (counts per
  class in the new snapshot), and O3 reports zero manifest mismatches, leaving
  exactly the seven recognizer-stage fixtures as `outcome=gap`. The sweep found
  and closed one decoder gap: a literal's own `span` was never read. Because
  the sweep re-serializes valid JSON, `ast.serialize` gains its fixture from a
  decoder test that feeds malformed text (D17). No `src/` change,
  `examples/tenant-write/` untouched, no new dependency.
- [x] (2026-10-30) EP-M3: canonical-token recognizer, witness checker, and
  properties. `tests/admission/recognizer/` renders the canonical tokens from
  `contracts/profile.json`, recognises them with the bounded kernel EP-M4 will
  verify, confirms every accepted offset list with a structurally independent
  witness (direct slicing and a separate gap traversal; it shares the kernel's
  `has_metadata` helper), and cross-checks the tokens against the pinned
  Regorus lexer. Four seeded kernel faults (metadata rule skipped, token after
  a comment, empty trivia, bare carriage return) are each caught by the witness.
  `tests/admission/properties/` checks 64 generated canonical sources across
  six trivia classes with a per-class floor, inserts 32 random strings into a
  comment, and pins four boundary shapes. `tests/admission/features/` binds
  three scenarios, each derived at test time from
  `examples/tenant-write/policy.rego` and its committed rule list: the baseline
  admitted, a leading `METADATA` comment refused at `trivia.metadata`, and a
  parenthesised comparison refused at `token.mismatch`. The whole corpus now
  matches the manifest: O3 reports zero mismatches, so the seven EP-M2 gaps are
  closed and `Comparison::Gap` is deleted. 60 tests pass, and all six
  applicable commit gates (`check-fmt`, `typecheck`, `lint`, `test`,
  `markdownlint`, `design-check`) are green. The only new dependency is
  `proptest = "=1.12.0"`. No `src/` change, `examples/tenant-write/` untouched.
- [x] (2026-10-31) EP-M4: Verus soundness proof of the recognizer kernel.
  `verus/admission_spec.rs` states the decomposition, trivia, metadata, and
  line-feed predicates over `Seq<u8>`; `verus/admission_lemmas.rs` proves the
  scan steps that lift each kernel loop step to that specification;
  `verus/admission_recognizer.rs` pulls the kernel in with `#[path]`, so one
  file is both compiled by Cargo and verified by Verus. `make verus` reports 40
  verified obligations and no errors, with no `assume`, `admit`, or
  `external_body`, and the proof is parametric in the source length (O6). The
  kernel's behaviour is unchanged: every admission test still passes. New files:
  `tools/verus/` (release and Z3 pins), `tools/rust-prover-tools/REF`,
  `scripts/install_verus_z3.py` with its tests, and the `install-verus` and
  `verus` targets. `vstd` is the only new dependency, dev-only and already in
  `Cargo.lock`. Verus is not part of `make all` or CI.
- [x] (2026-10-31) EP-M5: feasibility record, ADR 0005, and documentation.
  `docs/admission-feasibility-record.md` records the question, method, verdict
  per construct class, the excluded-entry mapping, the six AST gaps and their
  closures, the frontend divergences, the assurance boundaries, and what 2.1.1
  and 2.1.2 must carry forward. It finds no E0 issue.
  `docs/adrs/adr-0005-admit-by-ast-mapping-and-canonical-token-recognition.md`
  records the conjunction design. TDD §4.1 gains the D7–D11 and D13 narrowings
  and the exact-spelling rule; §6.1 links the record and ADR 0005; §13's
  Admission coverage row is met; R14 and R15 cite the OPA parser and scanner.
  The ToR resolves A5 for the pinned tuple, ADR 0002 notes open question 2
  answered, roadmap 1.2.2 is ticked, and the developer guide and repository
  layout describe the spike, the corpus, and the on-demand Verus targets.
  `SHA256SUMS` is refreshed for the three covered files whose bytes changed,
  including the drift inherited from `b6acb06`, and every one of its 39 entries
  verifies. All ten gates pass; the evidence table is under
  `Artefacts and notes`.
- [x] (2026-11-02) Rebased onto `2ca284c` and repaired the gates it added.
  `main` now runs Whitaker from `make lint` and builds with the standard
  `-Zthreads=8`/mold flags. The replay was clean, but the new lint reported 18
  findings in `tests/admission/`: `std::fs` in three helpers, three bumpy
  functions, four modules without inner docs, one `unwrap_or_else` panic, one
  over-long module, and two conditions with three branches. The repairs use
  `cap_std` capability handles, extract helpers along natural seams, move the
  docs inside their modules, and split the kernel's byte predicates into
  `kernel/scan.rs`, which also brought `crlf_at` out as its own function. The
  proof now reports 41 verified obligations and no errors. Clippy then found
  three consequences of the repairs (`mixed_attributes_style`, `shadow_reuse`,
  `expect_used`, `manual_let_else`), all fixed. All ten gates pass; the
  evidence table is under `Artefacts and notes`.
- [x] (2026-11-02) Repaired the ExecPlan assessment of `1785056`. T2's
  metadata predicate now covers a marker anywhere after the hash before the
  line ending; completeness adds `ws_canonical`, its token-sequence mirrors, and
  `lemma_ws_canonical_unique`; and `kernel::baseline_is_recognised` is a
  verified executable instance whose postcondition is `Ok(baseline_offsets())`,
  with `lemma_baseline_accepts` retained as the separate decomposition
  evidence. Each property test now persists its shrunk case to its own
  regression file, the `nul_in_comment` divergence row matches the revised
  fixture, and the scope arithmetic applies the plan's stated exclusions.
  `make verus` reports 89 verified obligations with 0 errors, and all five
  negative controls are rejected at the named obligation. Details are under
  `Artefacts and notes`.

## Surprises & discoveries

These came from reading the pinned sources during planning. EP-M1 confirmed
each one empirically on 2026-10-30 using the observation snapshot and a
temporary probe harness over the pinned 0.13.0 frontend; the one exception is
the OPA metadata rule, which remains a source reading because D6 excludes OPA
execution here. The probe harness was not committed.

- Observation: Regorus discards comments in its lexer and has no annotation
  type, so a `# METADATA` block parses cleanly and leaves no trace in the AST.
  Evidence: `regorus-0.13.0/src/lexer.rs:874-877`; no annotation field in
  `src/ast.rs:681-696`. Impact: metadata rejection needs a bounded source
  check, as TDD §6.1 anticipated.
- Observation: OPA v1.21.2 treats a comment as metadata when it starts at
  column 1 and its whitespace-trimmed text begins with `METADATA`. Evidence:
  `v1/ast/parser.go:558-560` at tag `v1.21.2`. Impact: Polean must reject at
  least those comments; D9 rejects a strict superset.
- Observation: the Regorus lexer's `peek` returns `'\x00'` both at end of input
  and for a literal U+0000, and comment skipping stops there. Evidence:
  `lexer.rs:457-462`, `845`, `1019`. Impact: an embedded NUL may silently
  truncate the module and hide trailing syntax.
- Observation: parentheses are dropped from the AST. Evidence:
  `parser.rs:898-904`. Impact: needs the bounded source check.
- Observation: the AST records neither the `if` keyword nor whether a body was
  braced. Multiple bodies (`allow if {a} {b}`) are accepted even in v1, and
  `allow if { 1, 2 }` may parse as a set-literal body. Evidence:
  `parser.rs:1913-1992`. Impact: the walker requires exactly one body of
  comparisons, and the recognizer requires the braces.
- Observation: Regorus parses as Rego v1 by default even without
  `import rego.v1`, so `Module.rego_v1` is not evidence of the import. Evidence:
  `engine.rs:319`, `engine.rs:1979-1986`. Impact: the walker must inspect
  `imports` directly.
- Observation: `__target__ := "..."` is stored in `Module.target`, outside the
  rule list, as a bare string with no span. Evidence: `parser.rs:2338-2365`,
  `ast.rs:689`. Impact: a rule-only walk would miss it, and some rejections
  have no source span. Confirmed, with an addition: the JSON route exposes it as
  `ast.target` (`"target": "rego"`), so the profile walk can see it without a
  source scan. Placement is constrained: it must follow the imports and precede
  the first rule, including `default`; before an import the parser reports
  ``unexpected keyword `import` ``.
- Observation: AST types are not `#[non_exhaustive]`, so an exhaustive `match`
  compiles outside the crate. Fields are public, `NodeRef` dereferences to the
  node, and `Expr`'s `Drop` implementation only forbids moving out, so matching
  through a reference works. `Expr::OrExpr` exists only under
  `rego-extensions`, which the pinned feature set does not enable. Evidence:
  `ast.rs:45-75`, `261`, `304`; `lib.rs:619-626`. Impact: a compile-time
  inventory guard is available to tests through `Engine::get_modules`
  (`engine.rs:919-921`).
- Observation: span `start` and `end` are UTF-8 byte offsets; `col` is
  unreliable. String-token spans exclude their quotes. Evidence:
  `lexer.rs:310-329`, `459-512`, `832-836`. Impact: the spike reports byte
  offsets only.
- Observation: Regorus rejects a bare carriage return ("\r must be followed by
  \n"), a leading BOM, and lines over 1,024 columns; OPA accepts all three.
  Evidence: `lexer.rs:862-867`, `1061`; `utils/limits/length.rs:22`; OPA
  `v1/ast/internal/scanner/scanner.go:71-73`, `611`. Impact: fail-closed
  divergences, listed in the record.
- Observation: `Cargo.lock` already contains `vstd`, `verus_builtin`, and
  `verus_builtin_macros` at `0.0.0-2026-09-06-0133`, pulled in by Regorus. Its
  `src/number.rs` uses `#[verus_verify]`, `#[verus_spec]`, and `proof!` in code
  that plain `rustc` compiles, because ghost code is erased unless
  `cfg(verus_keep_ghost)` is set. Impact: the recognizer kernel can be written
  once and both compiled by Cargo and checked by Verus (Q2).
- Observation: `sha256sum -c SHA256SUMS` already fails on `main` at
  `b6acb06` for `contracts/cli.txt` and `docs/technical-design.md`. Both
  changed in that commit, and no Make target or CI workflow runs the check.
  Evidence: the check run on this branch before any covered file was edited.
  Impact: EP-M5 reviews the drift and refreshes those entries alongside its own
  edits. This plan does not certify content it has not reviewed.
- Observation: every one of the 46 `NodeKind` variants is reachable from the
  v1 parser, so O1 has no unreachable set to document. `ExprUnary` needs a
  statement-initial unary or a parenthesised one, and `HeadSet` needs a
  `contains` head. Evidence: `examples/admission/expr/unary.rego` observes
  `ExprUnary=1` and `examples/admission/head/set.rego` observes `HeadSet=1`;
  `observations::tests::the_corpus_observes_the_whole_inventory` asserts the
  union is the whole inventory. Impact: the declared-coverage test can be the
  whole inventory, and the observed-coverage test makes a regression visible.
- Observation: `-x` is unary only at the start of a statement or inside
  parentheses; after a variable it is parsed as binary subtraction continuing
  the previous statement. Evidence: the probe's `unary-var` and `unary-parens`
  cases; `examples/admission/expr/unary.rego` is the committed form. Impact:
  the recognizer's canonical rendering must keep `-` adjacent to its operand,
  and the fixture set must not assume `a -b` is two statements.
- Observation: line width is checked on tokens, not on physical lines. A
  1,100-column comment line parses; a 1,100-character string token fails with
  `line exceeds maximum column width of 1024`. Evidence: a temporary probe case
  with a 1,100-column comment line, and `generated/over_long_line` for the
  token limit; the committed `trivia/` fixtures stay well inside it. Impact:
  the record states the narrower limit.
- Observation: two `import rego.v1` lines are rejected at parse with
  `this import shadows previous import of …`, so they cannot be a profile-stage
  fixture. Two distinct `data.*` imports parse and reach the profile walk.
  Evidence: `examples/admission/import/two-imports.rego`. Impact: the
  import-shape fixture uses distinct imports; the duplicate-import rejection
  belongs to the frontend, not the profile.
- Observation: at the frontend level, a literal U+0000 inside a comment
  truncates the module (the package declaration is reported missing), a leading
  BOM is an invalid character, a bare carriage return is rejected unless
  followed by line feed, and CRLF line endings parse. Evidence: the five
  `generated/` fixtures in the snapshot. Impact: each is a fail-closed
  divergence from OPA, and the recognizer's trivia rule already excludes them.
- Observation: `with`, metadata comments, and parentheses leave no node-kind
  trace; `allow if {}` parses as a single `ExprObject` literal; v0 rule bodies
  are parse errors while `set_rego_v0(false)` is in force. Evidence: the probe's
  `with`, `metadata-*`, `parens`, `empty-body` and `v0-*` cases, and the
  matching corpus fixtures. Impact: these are precisely the classes the
  recognizer must decide.
- Observation (EP-M2, 2026-10-30): the mutation sweep found a real closure gap
  in the first decoder draft. Every `Literal` variant declares a `span`, but
  the decoder read only the arm-specific fields, so a mutated or missing span
  on the literal wrapper was invisible while the same span on an expression was
  rejected. Evidence: 2,466 of 77,884 mutants were admitted before
  `decode_body::literal` began validating the wrapper span; every mutant is
  rejected afterwards. Impact: a closed table is not a closed walk. The sweep,
  not a review of the table, is what showed the unread field, and a later table
  edit that declares a field the decoder never reads will be caught the same
  way.
- Observation (EP-M2, 2026-10-30): the D16 precedence holds for every corpus
  fixture. Array, set, and object literals and the three comprehension forms
  render no canonical text, so they reach `atom.unknown` rather than
  `expr.kind`, exactly as D16's closing clause says; calls, unary, binary,
  arithmetic, assignment, and membership expressions reach `expr.kind`. The
  manifest fixtures `expr/` state both outcomes. Impact: no code or manifest
  change was needed, and D16 is confirmed rather than amended.
- Observation (EP-M2, 2026-10-30): the repository denies `allow_attributes` and
  `allow_attributes_without_reason` while `clippy.toml` lowers the argument and
  complexity thresholds, so a generator built from free functions with long
  parameter lists cannot pass `make lint` without suppression. Evidence: the
  first lint run reported 37 diagnostics, all in the new test modules, and the
  fix was structural: `Collector` records mutations through one `Expect` value
  carrying the class, reason code, and description. Impact: new test
  scaffolding is shaped by the lint configuration from the start; a helper that
  needs five arguments is a signal to bundle its data.
- Observation (EP-M2, 2026-10-30): the mutation sweep cannot reach
  `ast.serialize`, because it applies changes to a parsed value and
  re-serializes that value. Every mutant is therefore well-formed JSON, and
  only text the decoder receives from outside can fail to parse. Evidence: no
  mutant carries `ast.serialize`, and `ast/tests.rs` needs a direct call to
  reach that branch. Impact: D17 is amended so the ninth `ast.*` code has a
  named fixture (a malformed-JSON decoder test) instead of an unproven claim
  that the sweep covers it. The general lesson: an oracle built from parsed
  values cannot exercise the parse step; that step needs its own input.
- Observation (EP-M3, 2026-10-30): the pinned Regorus lexer excludes a string
  token's quotes from its span, so `"write"` appears in the lexer as `write`,
  while the canonical token keeps the quotes the source must spell. Evidence:
  the first O7 run failed on 13 corpus fixtures and one generated case with
  ``the lexer produced `write` where the canonical tokens have `"write"` ``.
  Impact: the cross-check normalises the canonical token by stripping a matched
  leading and trailing quote pair before comparing. This is a comparison rule,
  not a change to the recognizer: the source still has to spell the quotes.
- Observation (EP-M3, 2026-10-30): the D10 rule is not expressible as a token
  property. A token that follows a comment on the same line has the right text
  at a gap that contains no line feed, so only the gap's content distinguishes
  it. Evidence: the seeded "comment ends at whitespace" fault returns the
  correct offsets for `package #c authz`, and the witness refuses the gap. The
  kernel and the witness both therefore require a comment to be followed by its
  line feed inside the gap, and allow a comment to end at the end of the source
  only when no token follows.
- Observation (EP-M3, 2026-10-30): a source-mask fault is a weaker test than it
  looks unless the faulty kernel is first shown to accept the source. The first
  draft of the metadata and comment faults blanked the token text itself, so
  the faulty kernel stopped before producing offsets and the test proved
  nothing. Evidence:
  `the faulty kernel must recognise the source, stopped at 18` and
  `the comment holds the byte 0x0d`. Impact: `witness_refusal` panics when the
  injected kernel refuses the source, so a mask that breaks the source cannot
  masquerade as a caught fault.
- Observation (EP-M4, 2026-10-31): Verus does not accept a recursive spec
  function as a quantifier trigger. The flags invariant originally wrote
  `#[trigger] gap_has_newline(...)`; Verus discarded it, fell back to another
  term, and reported "recommendation not met" on every push into the offset
  vector. Evidence: the failing run and the warning text at the trigger
  position; a minimal probe showed the same. Impact: the flag property moved
  behind an opaque `flags_prefix` predicate, with `lemma_flags_prefix_push`
  carrying one token's step and a `#[trigger] p[j] as int` term. The same
  pattern (opaque predicate plus a push lemma) is what makes the per-token
  invariant affordable.
- Observation (EP-M4, 2026-10-31): the first `gap_has_newline` was an
  existential, and Verus instantiated it 5,180 times for 78% of the profile's
  quantifier cost; the `recognize` loop then exceeded the default rlimit at
  2,000. Evidence: `--profile` output on the kernel. Impact: the predicate is
  now a recursive scan, and `trivia` is opaque, so the loop verifies at the
  default rlimit with no rlimit failures.
- Observation (EP-M4, 2026-10-31): a `requires` clause is not available inside
  a loop body unless the loop invariant restates it, and `at + 8 <= to` is a
  genuine overflow hazard rather than a proof gap: `at` may approach
  `usize::MAX`. Evidence: the `contains_metadata` counterexample and the failed
  invariant. Impact: the scan counts down with `remaining` and states
  `at as int + remaining as int == to as int`, so neither `+` nor `-` appears
  in the loop condition; `lemma_slice_len_fits` supplies the `src.len()` term
  that vstd's broadcast length axiom needs before any addition can be proved in
  range.
- Observation (EP-M4, 2026-10-31): Verus warns that `==>` inside
  `assert forall ... by { }` does not assume the antecedent in the body, and
  the assertion fails without the assumption. Evidence: the failing
  `lemma_flags_prefix_push` and the compiler's own recommendation to use
  `implies`. Impact: guarded bodies in an `assert forall` use `implies`.
- Observation (EP-M4, 2026-10-31): `prover-tools verus run` cannot drive the
  pinned release. It parses `Toolchain:` with a pattern anchored at the start
  of a line, while the pinned `verus --version` indents that line by two
  spaces, so every run fails before verification; the pinned `verus install` is
  unaffected. Evidence: the target's output and the pattern in
  `rust_prover_tools/versions.py`. Impact: the `verus` target runs the
  installed, digest-checked binary directly (D21), and the deviation is
  recorded rather than hidden.
- Observation (EP-M4, 2026-10-31): the plain `rustc` build sees the
  `#[cfg(verus_keep_ghost)]` imports as an unexpected configuration, and Verus
  sees the specification items as used only under that same configuration.
  Evidence: `cargo check` warnings before the change, and the `dead_code`
  warning for an unreferenced constant. Impact: Cargo declares
  `check-cfg = ["cfg(verus_keep_ghost)"]`, and the spec imports carry
  `#[cfg(verus_keep_ghost)]`, so both builds are warning-free without an
  `allow` attribute.
- Observation (EP-M5, 2026-10-31): mapping the twenty unexercised manifest
  entries to the construct that refuses each one exposed a construct with no
  dedicated fixture. A `data` reference inside a rule body is refused by the
  same atom lookup that `atom/unknown-attribute.rego` exercises, so the mapping
  is sound, but a reader cannot confirm the refusal from the corpus alone.
  Evidence: section 5 of the feasibility record. Impact: recorded as future
  work for 2.1.1 rather than widened now, because adding a fixture here would
  change the corpus snapshot that EP-M2 already accepted.
- Observation (EP-M5, 2026-10-31): the divergence table can only cite the OPA
  frontend by source reading. Decision D6 forbids executing OPA in this task,
  so the OPA column of section 7 records what the pinned `parser.go` and
  `scanner.go` do, not what an observed run did. Evidence: a throwaway local
  probe produced matching results but is deliberately excluded from the record.
  Impact: the column is labelled as source reading, and 2.3.2 owns the executed
  comparison; a probe run must not be mistaken for recorded evidence.
- Observation (EP-M5, 2026-10-31): the pre-existing `SHA256SUMS` drift is a
  manifest-maintenance gap, not a content problem. `contracts/cli.txt` changed
  in `b6acb06` and its digest was never recomputed, and
  `docs/technical-design.md` changed in the same commit. Evidence: no working
  tree difference for `contracts/cli.txt`, and `git show b6acb06` names it.
  Impact: EP-M5 recomputes the digest for the bytes now committed, which
  records those bytes without certifying the base commit's content, and
  `sha256sum -c SHA256SUMS` passes for every covered file.
- Observation (rebase, 2026-11-02): `main` now runs Whitaker from `make lint`,
  so the replayed candidate acquired a gate it had never been measured against.
  Evidence: 18 findings in `tests/admission/` on the first run after the
  replay. Impact: the repairs are mechanical rather than semantic, but the
  kernel split into `kernel/scan.rs` also improved the proof boundary by giving
  `crlf_at` its own contract, and the obligation count rose from 40 to
  41. The repairs are part of the EP-M5 milestone, not deferred, because a red
  commit gate on the branch blocks delivery.

## Decision log

- D1: build the spike as a separate integration-test binary,
  `tests/admission/`, sharing no modules with `tests/toolchain/`. Rationale:
  Regorus is a dev-dependency; 2.1.1 owns IR types and 2.1.2 owns production
  admission; sharing modules between test binaries risks `dead_code` under the
  denied `allow` policy. Date/Author: 2026-10-29, planning agent.
- D2: decode the public `Engine::get_ast_as_json` output with a closed decoder
  over `serde_json::Value`, driven by one table of node tag to permitted keys
  and value types. Decode the whole tree first, then apply the profile; a
  decoding failure anywhere takes precedence over a profile rejection.
  Rationale: the JSON route is the documented, feature-gated API and matches
  TDD §6.1. Decoding before deciding prevents a contract fault deep in the tree
  from being reported as an ordinary unsupported construct. No new dependency
  is needed. Date/Author: 2026-10-29, planning agent; precedence added after
  expert review.
- D3: use the doc-hidden typed AST from `Engine::get_modules`, after the same
  `add_policy` call, only inside tests as an exhaustive inventory guard.
  Rationale: compile-time exhaustiveness without depending on an unstable API
  for admission itself. Using the engine's own modules means the typed view and
  the JSON view describe the same parse. Date/Author: 2026-10-29.
- D4: admission is the conjunction of the AST walk and a canonical-token
  recognizer. The walker derives the IR. The recognizer renders that IR to its
  canonical tokens from `contracts/profile.json` and checks that the source
  bytes are exactly those tokens separated by permitted trivia. It returns the
  start offset of each token as a witness. Rationale: one check closes every
  known AST gap: metadata, parentheses, NUL truncation, `if`, braces, and
  separators. Because its tokens come from the IR through an independent atom
  table, it also catches walker mapping mistakes, and it can only narrow
  acceptance. Alternatives rejected:
  - span-anchored residual accounting, which trusts Regorus spans;
  - a Polean parser that produces IR, the wholesale-trust outcome ADR 0002
    warns against;
  - `opa parse` as the metadata detector, which moves the independent oracle
    into the admission path and merges claims 1 and 4.
  Date/Author: 2026-10-29.
- D5: the spike's outcome is
  `Admitted(ir) | Rejected { stage, code, span }`, with a closed `ReasonCode`
  enum and an optional byte span. It has no exit or status mapping. The record
  proposes a mapping for 2.1.2: profile and recognizer rejections become
  `unsupported` (exit 4); parse failures become `invalid_input` (exit 5);
  decoding and serialization failures become `tool_error` (exit 6). Rationale:
  avoids building 2.1.2's diagnostics contract in a spike. A closed code enum
  still lets the coverage test show every code has a fixture. Date/Author:
  2026-10-29, after expert review.
- D6: no OPA execution in this task. Rationale: OPA's view of the corpus
  belongs to the source oracles (2.3.2) and hostile-source coverage (4.2.3).
  Running it here would need a second provisioned runner, and its legality
  labels do not affect whether Regorus-based admission is feasible. The record
  cites OPA source for each known divergence. Date/Author: 2026-10-29, after
  expert review.
- D7: reject raw strings (`` `write` ``); their token kind differs from a
  quoted string. Narrowest reading.
- D8: comment bodies may contain only bytes `0x20`–`0x7E` and tab. Whitespace
  is space, tab, line feed, and carriage return only when immediately followed
  by line feed. Reject a BOM anywhere, non-ASCII bytes outside string tokens,
  and every other control byte. Rationale: closes the comment-display spoofing
  and OPA-BOM false admission the red team found, and simplifies the proof.
- D9: reject any comment whose text contains the bytes `METADATA`, in any
  column. Rationale: a strict superset of OPA's rule that needs no model of
  Go's trimming.
- D10: require non-empty trivia between every pair of adjacent tokens, a
  line feed before each body literal, braced bodies, and no `;` separators.
  This rejects `allow if{`, `allow if expr`, and single-line multi-literal
  bodies. A single-literal body on one line, `allow if { a == b }`, is
  admitted. Rationale: removes every possible token merge (`=`+`=`, `:`+`=`,
  word+word) from the lexical axiom X3, at no cost to canonical sources.
- D11: canonical order is package, import, default, then rules. A `default`
  placed after a rule is rejected, although §4.1 does not order it. Narrowest
  reading.
- D12: classify a number literal that makes `get_ast_as_json` fail as a
  frontend failure (stage `serialize`), not as malformed source.
- D13 (Q1, accepted by the user on 2026-10-29): reject escaped spellings.
  EP-M5 amends TDD §4.1 to read "parsed token kind and exact canonical
  spelling", and a fixture under `atom/` covers an escaped `"write"`. The
  question as put to the user follows.
- Q1: TDD §4.1 says to compare "decoded literal
  identity", which would admit `"wr\u0069te"` as `"write"`. The recognizer as
  planned compares exact canonical spellings and so rejects escaped spellings.
  Recommendation: reject escapes in v0, and amend §4.1 to say "parsed token
  kind and exact canonical spelling". The `atoms` strings in
  `contracts/profile.json` are unchanged and keep their exact-spelling meaning.
  Alternative: decode JSON escapes inside the recognizer, which enlarges the
  kernel and its proof.
- D14 (Q2, accepted by the user on 2026-10-29): prove the recognizer in this
  task. EP-M4 is in scope, `vstd` is authorized at the locked version, and the
  fallback below applies only if the Verification tolerance is breached and the
  user then agrees to it. The question as put to the user follows.
- Q2: prove the recognizer in this task by writing
  its kernel once in the Verus attribute style, with `vstd` added as a direct
  dev-dependency at the already-locked version `0.0.0-2026-09-06-0133`, and the
  matching Verus release pinned through `prover-tools`.
  - Recommendation: accept. The kernel is small, independent of Regorus and
    the IR, and the one artefact intended for promotion to 2.1.2. A
    single-source kernel means the proof covers the code the tests execute,
    with no mirror to drift.
  - Fallback: defer the proof to 2.1.2, keep the witness-checked recognizer,
    and record the specification and lemma statements in the record as a
    2.1.2 obligation. Under the fallback EP-M4 is dropped, and the record says
    the recognizer is "tested, not proved".
- D15 (2026-10-30, EP-M1): D12 is refuted. A number literal beyond `f64`
  range fails inside `Engine::add_policy` with
  `invalid number. serde_json cannot parse number: out of range`, so it is a
  parse failure, not a serialization failure. `limits/number-overflow.rego`
  expects `parse` / `parse.number`. `Stage::Serialize` and `ast.serialize` stay
  in the closed sets because a frontend or feature change could still fail
  there; a decoder unit test feeds malformed text for that branch (see D17).
  The corpus explicitly never expects an adapter stage (enforced by a test).
- D16 (2026-10-30, EP-M1): the reason-code mapping is fixed by the manifest
  and is implemented in EP-M2 in this precedence order: package shape, import
  shape, `ast.target`, default shape, order shape, rule limit, rule head, rule
  body, body limit, literal kind, `with` modifier, expression kind, operator
  kind, then an unmatched comparison is `atom.unknown`. A non-`Compare`
  statement is `expr.kind`; a comparison with an operator other than `==` is
  `operator.kind`; a `==` whose sides include a call, unary, binary,
  arithmetic, assignment, or membership expression is also `expr.kind`; only a
  comparison over variables, references, and literal values can reach
  `atom.unknown`. Rationale: a single precedence makes each fixture's expected
  code unambiguous and keeps the more specific diagnosis from being masked by a
  generic one. The manifest is the executable statement of this decision; EP-M2
  changes it only with a recorded reason.
- D17 (2026-10-30, EP-M1; amended EP-M2): the `ast.*` codes are fixtures of
  the EP-M2 mutation sweep, not of the source corpus, because no source can make
  `serde_json` produce a malformed AST. `ReasonCode::AST_CODES` names the nine
  codes, and a corpus test asserts the source corpus uses exactly `ALL` minus
  `AST_CODES`. This keeps the coverage test honest without inventing a source
  fixture that cannot exist. Amendment: eight of the nine are emitted by the
  sweep; `ast.serialize` is not, because every mutant is re-serialized as valid
  JSON. `ast/tests.rs` therefore feeds malformed text (`{`, `not json`, the
  empty string) directly to the decoder and asserts `ast.serialize`, so the
  ninth code has a fixture rather than only a reachable branch.
- D18 (2026-10-30, EP-M1): `module.target` is decided at the profile stage
  from the decoded `ast.target`, not from a source scan. The JSON route exposes
  the value (`"target": "rego"`), so no additional input is needed. Placement
  errors (before an import, after a rule) are frontend parse failures and are
  reported as `parse.syntax`.
- D19 (2026-10-30, EP-M2): the decoder lives in `tests/admission/ast/` and the
  profile walk in `tests/admission/admission/`, not in the `decode/` and
  `profile/` directories sketched under `Interfaces and dependencies`.
  Rationale: `admission::classify` and `admission::admit` are the entry points,
  so the walk belongs beside them, and `ast::decode` names the decoder more
  precisely than a bare `decode`. The layout rule is unchanged: a directory
  with tests holds `mod.rs` plus `tests.rs`, and every file stays under 400
  lines. Four files crossed that limit during implementation and were split:
  `ast/schema/` separates the closed-object readers, the scalar readers, and
  the span readers; `ast/expr_variants.rs` holds the expression variant table
  apart from its dispatch; `admission/walk/` separates the module checks from
  the statement checks; and `ast_mutations/generate/` separates the classes and
  their application from the shape discovery.
- D20 (2026-10-30, EP-M3): the O7 cross-check strips one matched pair of
  quotes from a canonical string token before comparing it with the pinned
  lexer, because Regorus excludes a string's quotes from its token span. The
  recognizer itself is unchanged: the source must still spell the quotes.
  Separately, a `Refusal::Harness` from the recognizer cannot occur after a
  successful walk, because the walk rejects an unregistered atom first; it is
  nevertheless reported as a profile `atom.unknown` rejection with no
  recognizer trace, so a defect in the walk fails closed rather than admitting
  the module. Rationale: a comparison rule for evidence must not weaken the
  property it is evidence for, and an unreachable branch must still have a
  defined, failing outcome.
- D21 (2026-10-31, EP-M4): `make verus` invokes the installed Verus binary
  directly, after `make install-verus` has downloaded it through `prover-tools`
  and checked it against `tools/verus/SHA256SUMS`. It does not call
  `prover-tools verus run`, because that command parses the pinned release's
  `--version` output and cannot read its indented `Toolchain:` line. Rationale:
  the release and the installer stay pinned and digest-checked; only the
  launcher changes, and the failure is deterministic and recorded. The target
  still fails closed: it refuses to run when the binary is absent and names
  `make install-verus`.

## Outcomes & retrospective

Completed 2026-10-31.

- Verdict: admission through the pinned frontend is feasible.
  `docs/admission-feasibility-record.md` states the verdict for every construct
  class, with the fixture or test that demonstrates it.
- E0 findings: none. No construct was distinguishable by neither check, so the
  Design tolerance was not triggered and 2.1.2 is not blocked.
- Evidence: the corpus has 106 manifest entries — 21 admitted and 85 rejected
  (11 at parse, 66 at the profile walk, 8 at the recognizer). All 46 AST node
  kinds are observed, and all 79,181 mutants across ten classes are rejected at
  `decode` or earlier.
- Verification: `make verus` reported 40 verified obligations and no errors at
  commit `14d29d9`, and 41 after the post-rebase repair split `crlf_at` into
  its own function, with no `assume`, `admit`, or `external_body`.
- Lessons for 2.1.2:
  - Admission must stay a conjunction. The recognizer is the only check that
    sees comments, parentheses, `if`, braces, separators, and spelling, and it
    must render from the profile rather than from the walker's table.
  - Verified beats assumed where the property is universal: the kernel proof
    removed the need to trust an unbounded scan, but it did not remove trust in
    the renderer, the walk, or Regorus. State that split wherever the result is
    reported.
  - A closed table is not a closed walk. The mutation sweep, not review, found
    the decoder's unread literal span; keep the sweep at the boundary where a
    new field would reappear.
  - Fixtures a committed file cannot carry (BOM, NUL, bare carriage return,
    over-long token) belong in generated sources, not in `.rego` files that
    tooling might rewrite.
  - Verus cost lay in invariant shape rather than in the kernel: opaque
    predicates, recursive scans instead of existentials, and countdowns instead
    of arithmetic near `usize::MAX`. Budget for that, not for lemma count.
  - The prover launcher could not read the pinned release's version output
    (D21), so the target runs the installed, digest-checked binary. Pin-driven
    wrappers still need an end-to-end run before they are documented.
  - The record's excluded-entry table now has a dedicated fixture for every
    row, including `data` references inside a rule body
    (`expr/data-reference.rego`). One divergence column is still taken from
    source reading rather than execution; that remains future work for 2.1.1
    and 2.3.2 rather than widened here.

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
- Token: one of the fixed canonical strings: `package`, `authz`, `import`,
  `rego.v1`, `default`, `allow`, `:=`, `false`, `true`, `if`, `{`, `}`, `==`,
  the four dotted references, and `"write"` and `"admin"` with their quotes.
- Trivia: the bytes between tokens: whitespace (D8) and comments from `#` to
  the end of the line.
- Witness: the start offset of every token in the source. Given the witness,
  checking the decomposition is a simple slice comparison.
- Spike: test-only code that demonstrates feasibility. 2.1.2 deletes it or
  promotes it; this task never ships it from `src/`.

Relevant existing files:

- `Cargo.toml`: Regorus `=0.13.0` with `default-features = false` and features
  `["ast", "std"]` under `[dev-dependencies]`, plus `rstest`, `rstest-bdd`,
  `insta`, `googletest`, `pretty_assertions`, and `serde_json`. The lints deny
  `unwrap`, `expect`, indexing, and `panic` in result functions, so helpers
  return `Result<_, String>`.
- `tests/toolchain/`: the 1.2.1 observations. `regorus.rs` snapshots the
  baseline AST JSON in
  `tests/toolchain/snapshots/toolchain__regorus__baseline_ast_json.snap`.
  `bdd.rs` shows the `rstest-bdd` 0.6.0 pattern: a `ScenarioState` with
  `Slot`s, a scoped `#[expect(unused_braces)]`, and an `include_str!` of the
  feature file so Cargo recompiles when it changes.
- `examples/tenant-write/policy.rego` and `policy.json`, plus six mutations
  under `examples/tenant-write/mutations/*/`. They are independent sentinels.
  Their IR was written by hand and is validated by the Python design lane. None
  of them uses the `locked` atom.
- `contracts/profile.json`: the six `atoms` strings the renderer uses.
- Regorus sources for reference: run `cargo fetch --locked`, then read
  `~/.cargo/registry/src/*/regorus-0.13.0/src/{ast,parser,lexer,engine}.rs`.

Skills and guides to load: `rust-router`, then `rust-unit-testing` for test
shape, `proptest` for EP-M3, `verus` for EP-M4, and `en-gb-oxendict-style` for
prose. Read these before starting:

- `docs/developers-guide.md`
- `docs/rust-testing-with-rstest-fixtures.md`
- `docs/reliable-testing-in-rust-via-dependency-injection.md`
- `docs/rust-doctest-dry-guide.md`
- `docs/complexity-antipatterns-and-refactoring-strategies.md`
- `docs/documentation-style-guide.md`

`docs/scripting-standards.md` applies only if a script becomes necessary; none
is planned. There is no `docs/references.md`. The technical design's References
section (R1–R13) is the reference list, and EP-M5 adds the OPA parser source to
it.

## Conformance basis

Upstream at commit `b6acb06`:

- Terms of reference draft v0.1: G1, N1, SC6, A2, and A5.
- Technical design draft v0.1: §§4.1, 6.1, 6.3, 7, 10 (V1, V8), and the §13
  "Admission coverage" row.
- ADR 0002: decision item 3 (trusted frontend) and open question 2.
- ADR 0003: Regorus `=0.13.0`, features `["ast", "std"]`. Its AST JSON is an
  adapter input, not a proof ABI.
- `docs/context.md`: admitted policy, unsupported, and invalid input.
- Roadmap RM-1.2.2. Its success criteria are labelled here:
  - S1: package, import, and default;
  - S2: every rule and expression;
  - S3: metadata;
  - S4: unknown AST variants;
  - S5: missing API coverage has a bounded check or an E0 finding;
  - S6: unreachable syntax is not ignored.

```plaintext
TOR-A5, ADR-0002 OQ2               -> RM-1.2.2 -> EP-M1..M5 -> docs/admission-feasibility-record.md
TDD §4.1 package/import/default    -> S1 -> EP-M1, EP-M2 -> corpus package/, import/, default/; O3
TDD §4.1 forms, §6.1 every node    -> S2 -> EP-M2 -> inventory guard; O1, O3
TDD §6.1 unknown variants          -> S4 -> EP-M2 -> AST JSON mutation sweep; O2
TDD §4.1 never ignore unreachable  -> S6 -> EP-M1, EP-M2 -> corpus unreachable/; O3
TDD §4.1 metadata, §6.1 lexical    -> S3, S5 -> EP-M3 -> recognizer; O4, O5, O7
TDD §6.1 bounded check is sound    -> S5 -> EP-M4 -> verus/admission_recognizer.rs; O6
TDD §7 import(render(p)) = p       -> EP-M3 -> render-then-admit property; O5
TOR-N1, TOR-SC6, TDD V8            -> EP-M2, EP-M3 -> fail-closed outcomes; O2, O3
```

This task does not discharge ingestion (2.1.1), production admission and
diagnostic source maps (2.1.2), source-oracle agreement (2.3.2), or
hostile-source coverage of the release (4.2.3). It answers A5 and ADR 0002 open
question 2 for the pinned tuple only.

## Verification plan

The spike introduces three things worth stating precisely:

- a classification from source text to outcome;
- a recognizer whose acceptance implies a byte-level decomposition, with a
  witness; and
- a coverage relation between the Regorus AST inventory and the fixture
  corpus.

### Where Verus is warranted, and where it is not

Verus is warranted for the recognizer kernel (O6), as accepted in D14. The
kernel is a small pure function over byte sequences of unbounded length. Its
property is the soundness of the bounded check the record relies on, and a
counterexample would be a silent admission. No finite test set establishes it
for all inputs. The kernel does not depend on Regorus or the IR. Its interface
(bytes and token slices in, offsets out) survives promotion to 2.1.2, so the
proof is not thrown away.

Verus is not warranted for the AST walk, which is a finite case analysis over a
closed set of node kinds. Compiler exhaustiveness, a fixture for every kind,
and an exhaustive mutation sweep give stronger evidence for that structure than
a proof over a hand-written model of Regorus types would. Nor is it warranted
for manifest loading, snapshot rendering, or other plumbing. Kani is not used:
realistic bounds cannot hold even `package authz`, and the witness checker
already checks every execution. `cargo-mutants` is not installed; named seeded
faults (O5) serve the same purpose here.

### Proof sketch for the recognizer kernel

The abstract model is independent of Regorus and of the IR types. A source is a
byte sequence `s`. Tokens are non-empty byte sequences.

```plaintext
spec ws_at(s, i)       := s[i] ∈ {0x20, 0x09, 0x0A}
                          ∨ (s[i] = 0x0D ∧ i + 1 < |s| ∧ s[i+1] = 0x0A)
spec body_byte(b)      := b = 0x09 ∨ (0x20 ≤ b ≤ 0x7E)
spec trivia(s, a, b, last) :=
     every byte in s[a..b] is classified, from left to right, as Ws, Hash,
     or Body, where:
       Ws   at i requires ws_at(s, i), and if the previous byte is Hash or
            Body, s[i] must be 0x0A or the 0x0D of a CR LF pair;
       Hash at i requires s[i] = '#';
       Body at i requires body_byte(s[i]) and a previous byte of Hash or Body;
       no run of Hash Body* spells "METADATA" in its Body bytes;
       and unless last holds, the byte before b is not Hash or Body.
spec decomposes_at(s, k[0..n), p[0..n)) :=
     (∀ j. |k_j| > 0)
   ∧ (∀ j. s[p_j .. p_j + |k_j|] = k_j)
   ∧ trivia(s, 0, p_0, n = 0)
   ∧ (∀ 0 < j < n. p_{j-1} + |k_{j-1}| < p_j
                   ∧ trivia(s, p_{j-1} + |k_{j-1}|, p_j, false))
   ∧ (n > 0 → trivia(s, p_{n-1} + |k_{n-1}|, |s|, true))
```

With `n = 0` the whole source must be trivia. The "not Hash or Body before b"
condition forbids a token on the same line as a preceding comment, because Rego
would read that token as part of the comment. Strictly increasing offsets with
a gap give non-empty trivia between tokens (D10). The line-feed-before-literal
rule of D10 is not part of the kernel: it is a property of the token list, so
the renderer encodes it as a required line feed in the gap before each
literal's first token. The kernel then takes a per-gap flag, `needs_newline`,
which strengthens the gap's `trivia` with "contains 0x0A".

Executable contract:

```plaintext
exec recognize(src: &[u8], toks: &[&[u8]], needs_newline: &[bool])
     -> Result<Vec<usize>, usize>
  requires ∀ j. |toks[j]| > 0, |needs_newline| = |toks|
  ensures  Ok(p)  ⇒ |p| = |toks| ∧ decomposes_at(src@, toks@, p@)
                     ∧ ∀ j. needs_newline[j] ⇒ the gap before p_j holds 0x0A
           Err(i) ⇒ i ≤ |src|
```

Lemma hypotheses, to be refined once the verification conditions appear:

1. `lemma_skip_trivia_sound`: if the trivia skipper moves from offset `i` to
   `j`, then `trivia(s, i, j, j = |s|)` holds. If it stopped before the end,
   `s[j]` is not whitespace or `#`, and the previous classified byte is not
   inside a comment.
2. `lemma_decomposition_extends`: a decomposition of the first `m` tokens,
   extended by a trivia gap and the next token, is a decomposition of the first
   `m + 1` tokens.
3. `lemma_no_hidden_bytes` (T1): if `decomposes_at(s, k, p)` and every token
   byte lies in `0x20`–`0x7E`, then every byte of `s` is printable ASCII, tab,
   line feed, or a carriage return that precedes a line feed. In particular `s`
   contains no NUL, no BOM, and no bidirectional control.
4. `lemma_no_metadata_outside_tokens` (T2): if `decomposes_at(s, k, p)` and no
   token contains `#`, then no `#` in `s` is followed on the same line by the
   bytes `METADATA`. With axiom X2 this shows that OPA would find no metadata
   annotation in an accepted source, whatever the column.
5. `theorem_recognize_sound`: the executable contract above, by loop
   invariant over the token index, using lemmas 1 and 2.

T1 and T2 show that the specification excludes things: a `trivia` that accepted
every byte would fail them. Completeness means that canonical renderings with
arbitrary permitted trivia are accepted. That is an availability property,
checked by property tests (O5) rather than proved. The proof makes no claim
about how Rego interprets the tokens; that is axiom X3.

### Obligations

- O1, inventory completeness. Every variant of the Regorus 0.13.0 types
  `Rule` (2), `RuleHead` (3), `Literal` (5), `Expr` (21), `BoolOp` (6),
  `AssignOp` (2), `BinOp` (2), and `ArithOp` (5) maps to a distinct Polean-owned
  `NodeKind`. The kinds observed across the corpus equal `NodeKind::ALL`.
  - Method:
    - an exhaustive `match`, without a wildcard arm, from each typed enum,
      reached through `&*node`, to `NodeKind`;
    - an exhaustive `NodeKind` to ordinal `match`, with a test that `ALL`
      covers every ordinal exactly once;
    - per fixture, the multiset of node kinds seen by the JSON decoder equals
      the multiset seen by the typed walk.
    Clippy's pedantic `match_same_arms`, under `-D warnings`, rejects two
    variants mapped to one kind.
  - Artefact: `tests/admission/inventory/`.
  - Evidence: `make test`. A Regorus bump that adds a variant fails to
    compile.
  - Non-vacuity: deleting the only fixture for `Expr::Membership` makes the
    equality fail. So does adding a `NodeKind` with no fixture.
- O2, closed decoding. For the AST JSON of every fixture that Regorus parses,
  each mutation below is rejected with stage `decode`, before any profile
  decision. The mutations are applied at every applicable position:
  - unknown variant tag on an externally tagged object;
  - unknown string at a unit-enum position (`op`);
  - unknown key on any object;
  - deletion of any key Regorus always emits, including keys whose value is
    `null` (`assign`, `key`), which are required, not optional;
  - wrong type by fixed substitution: null to bool, bool or number to string,
    string to number, array to `{}`, object to `[]`;
  - wrong arity for tuple arrays (`RefDot.field`, `Object.fields` items);
  - span ranges with `start > end`, `end` beyond the source, or an offset off
    a UTF-8 boundary;
  - for the keys Regorus may omit (`with_mods`, `target`, `as`), presence with
    an empty or `null` value.
  The decoder also checks that the array holds exactly one module and that
  `source.contents` equals the submitted text.
  - Method: an exhaustive structural sweep. The mutation generator walks the
    JSON shape and does not reuse the decoder's key table.
  - Artefact: `tests/admission/ast_mutations/`.
  - Evidence: `make test`; the test prints mutant counts per class.
  - Non-vacuity: every class yields at least one mutant, and the unmutated
    JSON of each admitted fixture is admitted. Duplicate keys cannot be seen
    through `serde_json::Value`; that is out of scope (X5).
- O3, corpus outcomes. Every fixture yields its manifest outcome:
  - admitted fixtures yield their hand-written IR;
  - rejected fixtures yield the stated stage and `ReasonCode`, with any span
    inside the source.
  Every `ReasonCode` variant has at least one fixture.
  - Method: one data-driven test that walks `examples/admission/manifest.json`
    and reports every mismatch together. The manifest and feature file are
    included with `include_str!` so Cargo tracks them. A bijection test checks
    that manifest entries and `.rego` files correspond.
  - Artefact: `tests/admission/corpus/`.
  - Non-vacuity: the corpus contains admitted fixtures, including the new
    `locked` fixture with hand-written IR, and rejections at every stage.
- O4, witness soundness on every execution. For each `Ok(p)` the recognizer
  returns, in the corpus and in every property case, a checker independent of
  the kernel's loop structure confirms `decomposes_at` by direct slicing and
  classification.
  - Method: a deliberately simple checker, written without reference to the
    kernel's loop structure. The independence is structural: the checker
    re-derives offsets by direct slicing and a separate gap traversal, but it
    does not duplicate the kernel's byte classification, so the shared
    `has_metadata` helper is the one rule both sides apply.
  - Artefact: `tests/admission/recognizer/witness.rs`.
  - Non-vacuity: these seeded faults must each be caught by a named test:
    - a kernel that skips the `METADATA` check;
    - a kernel that lets a token follow a comment on the same line;
    - a kernel that accepts empty trivia between tokens;
    - a kernel that accepts a bare carriage return.
    Each fault is exercised by passing a faulty kernel function to the same
    test harness (dependency injection, not a feature flag).
- O5, faithful mapping and completeness on generated canonical sources. For
  every IR with 0–16 rules of 1–16 atoms, rendered with generated permitted
  trivia, admission returns exactly that IR. Trivia includes spaces, tabs, CR
  LF, blank lines, and comments without `METADATA`. Inserting text without
  `METADATA` into an existing comment never changes the outcome.
  - Method: `proptest` over structured values (an IR plus trivia choices), so
    shrinking preserves syntax validity. Sources are assembled from the atom
    strings in `contracts/profile.json` and literal fragments of
    `examples/tenant-write/policy.rego`, never from `canonical_tokens`. The
    empty IR, the 16-by-16 maximum, 17 rules, and 17 atoms are pinned as
    `rstest` cases, because random generation reaches them unreliably. The
    property counts trivia classes and asserts a floor for each.
  - Artefact: `tests/admission/properties/`.
  - Non-vacuity: a seeded fault that swaps the renderer's `locked` and
    `unlocked` strings fails both this property and O3. A test asserts that
    each rendered atom string equals `profile.atoms[name]`.
- O6, recognizer soundness, `theorem_recognize_sound` with lemmas T1 and T2
  (D14).
  - Method: Verus over the single-source kernel.
  - Artefact: `tests/admission/recognizer/kernel/mod.rs`, with the byte-level
    predicates in `kernel/scan.rs`, the trivia scan in `kernel/trivia.rs`, and
    the executable baseline instance in `kernel/baseline.rs`; the specification
    is `verus/admission_spec.rs` with the completeness contract in
    `verus/admission_spec_completeness.rs`, and the whole is verified through
    the crate root `verus/admission_recognizer.rs`, which contains only a
    `#[path]` module declaration.
  - Evidence: `make verus` reports zero errors. Before the proof is complete,
    the run shows the open postcondition, which is the Red state.
  - Non-vacuity: the antecedent is inhabited. The verified executable
    instance `kernel::baseline_is_recognised` requires its arguments to be the
    concrete baseline source, tokens, and flags, and its postcondition
    establishes `Ok(baseline_offsets())`, so it is the successful result the
    specification admits rather than a fact about an unconstrained witness.
    `lemma_baseline_is_canonical` supplies the canonical offsets that the
    instance needs, and `lemma_baseline_accepts` remains separate evidence that
    the decomposition into tokens and trivia is the intended one. T1 and T2
    show that the specification excludes NUL, BOM, and metadata. `grep` confirms
    that `assume`, `admit`, and `external_body` are absent.
- O7, lexical cross-check of axiom X3. For every source the recognizer
  accepts, Regorus's own token stream (`regorus::unstable::Lexer`) matches the
  canonical tokens. Matching is modulo Regorus's splitting of dotted references
  and its exclusion of string quotes from string tokens.
  - Method: an assertion inside the O3 and O5 harnesses.
  - Non-vacuity: generated sources include adversarial trivia (a token after
    `#`, a NUL inside a comment, `//`) which the recognizer must reject before
    the comparison is reached; a named test checks that.

### External axioms, not verified here

- X1: Regorus 0.13.0 parses admitted sources into the AST its source code
  describes (ADR 0002, trusted frontend). The recognizer and O7 reduce this
  trust but do not remove it.
- X2: OPA v1.21.2 treats a comment as metadata only if a `#` is followed, on
  the same line, by text whose trimmed form starts with `METADATA`
  (`v1/ast/parser.go:558-560`). Any trimming yields a contiguous part of the
  line, so D9's containment test is a superset.
- X3: a Rego lexer, given a source that `decomposes_at` the canonical tokens
  with D10's separation, produces exactly those tokens. O7 checks this for
  Regorus on every accepted source; OPA's agreement is left to 2.3.2.
- X4: the Verus release and `vstd` are sound, and ghost
  erasure leaves the executable kernel unchanged.
- X5: `serde_json` parses Regorus's AST JSON faithfully, and duplicate keys,
  which `Value` cannot represent, do not occur in Regorus's output.

Residual gap: the record establishes feasibility for the pinned tuple only. It
proves nothing about Rego semantics, Regorus's parser, or the IR's meaning. It
does not check OPA's parse of the corpus, and a Regorus or OPA upgrade repeats
the exercise.

## Plan of work

### Stage A: confirm and propose (no code)

The expert review is complete. The user accepted Q1 and Q2 and approved the
plan on 2026-10-29, so proceed to Stage B.

### Stage B: corpus and observations (EP-M1)

Create `examples/admission/`. Put one `.rego` file per construct in these
directories:

- `package/` and `import/`;
- `default/` and `head/`;
- `body/` and `literal/`;
- `expr/` and `operator/`;
- `atom/` (reversed operands, other literal values, bracket references);
- `limits/` and `order/`;
- `trivia/` (metadata with valid YAML, parentheses, comment spoofing, CR, and
  line feeds in every gap);
- `unreachable/` and `module/`.

Each fixture is the baseline with one deliberate difference, and its first line
is a comment naming that difference. Add `admitted/locked-owner-rule.rego` with
a hand-written `admitted/locked-owner-rule.json`, so that `locked` appears in
independently written IR.

`examples/admission/manifest.json` holds one object per line. Each object gives
the fixture path, its `covers` kind names, and its expected outcome. The
outcome is either `admitted` with the path of an IR file, or `rejected` with a
stage and a reason code. The manifest also references the seven committed
`examples/tenant-write/` sources by path, rather than copying them. Some
fixtures need bytes that a `.rego` file cannot carry safely: an embedded NUL, a
BOM, a bare carriage return, or an over-long line. These are generated by a
named function in the test and listed in the manifest by name.

Write `tests/admission/main.rs` and the bijection and observation tests. The
observation snapshot records what Regorus itself does with each fixture: parse
success or failure, whether serialization succeeds, and the multiset of node
kinds. It is the empirical record of what the AST exposes, and confirms or
refutes each item in `Surprises & discoveries`. The committed EP-M1 state is
green. To show the Red stage, run the O3 corpus test locally against a stub
classifier that rejects everything, and record its failure in
`Artefacts and notes`; do not commit the stub.

### Stage C: implementation with verification (EP-M2, EP-M3, EP-M4)

EP-M2 writes the table-driven closed decoder, the profile walk (one small
function per node kind), the typed inventory guard, and the mutation sweep.
After it, every fixture that the AST can decide passes O3. The fixtures that
only the source check can catch are visible in the snapshot as `walk=admitted`.
That is the demonstrated gap, kept as evidence. They are marked
`stage: recognizer` in the manifest. Until EP-M3 adds that stage, O3 requires
exactly this set to be admitted by the walk and fails on any other outcome, so
the gap can neither grow nor shrink unnoticed. EP-M3 removes the allowance.

EP-M3 writes the token renderer and the recognizer kernel, wires admission as
walker then recognizer, and adds the witness checker, the seeded faults, the
properties, the lexer cross-check, and three BDD scenarios. After it, the whole
corpus passes.

EP-M4 does four things:

1. Adds `vstd` and the Verus pins, and the `install-verus` and `verus` Makefile
   targets.
2. Annotates the kernel with its specification and an empty proof. This is the
   Red state: Verus reports the failing postcondition.
3. Adds lemmas and loop invariants until Verus reports success.
4. Confirms that `make lint` still passes on the annotated kernel.

### Stage D: record and documentation (EP-M5)

Write the feasibility record and ADR 0005, amend the design documents, tick the
roadmap, and run every gate.

## Milestones and plateaus

- EP-M1, corpus and observations.
  - End state: the corpus, the manifest, and the observation snapshot are
    committed. The bijection test and the snapshot pass, and the Red O3 run is
    recorded.
  - Requirements: S1, S2, S3, and S6 now have fixtures.
  - Evidence: `make check-fmt`, `make lint`, and `make test`, plus the Red
    transcript.
  - Conformance: no `src/` change, `examples/tenant-write/` untouched, and no
    new dependency.
  - Recovery: revert the commit.
  - Remaining gaps: classification.
  - Compatibility: none.
- EP-M2, closed AST walk.
  - End state: O1, O2, and O3 pass. Only the fixtures marked
    `stage: recognizer` are still expected failures.
  - Requirements: S1, S2, S4, and S6.
  - Evidence: `make check-fmt`, `make lint`, and `make test`; the snapshot is
    reviewed.
  - Conformance: the decoder uses `serde_json::Value` only; no new dependency.
  - Recovery: revert; EP-M1 remains coherent.
  - Remaining gaps: S3 and S5.
  - Compatibility: none.
- EP-M3, recognizer and properties.
  - End state: the whole corpus is classified as the manifest states, and O4,
    O5, O7, and the BDD scenarios pass.
  - Requirements: S3 and S5, with tested soundness.
  - Evidence: `make check-fmt`, `make lint`, and `make test`.
  - Conformance: `proptest` is the only new dependency, and the recognizer
    never produces IR.
  - Recovery: revert; EP-M2 remains coherent.
  - Remaining gaps: the unbounded soundness proof.
  - Compatibility: none.
- EP-M4, Verus soundness proof (D14).
  - End state: `make verus` proves O6 with no `assume`, `admit`, or
    `external_body`; the kernel is unchanged in behaviour; every Cargo gate
    passes. `make verus` is an on-demand target, outside `make all` and CI.
  - Requirements: S5, with the bounded check proved sound.
  - Evidence: `make install-verus`, `make verus`, `make check-fmt`,
    `make lint`, and `make test`.
  - Conformance: `vstd` at the locked version is the only new dependency.
    Verus is pinned in `tools/verus/` and `tools/rust-prover-tools/REF`,
    outside the E0 tuple in `toolchain-pins.json`.
  - Recovery: revert; EP-M3 remains coherent, with tested but unproved
    soundness.
  - Remaining gaps: documentation.
  - Compatibility: none.
- EP-M5, record and documentation.
  - End state: the record, ADR 0005, and the amended documents agree with the
    code; roadmap 1.2.2 is ticked; every gate passes.
  - Requirements: S1–S6; A5 and ADR 0002 open question 2 answered.
  - Evidence: every gate in `Validation and acceptance`, including
    `sha256sum -c SHA256SUMS`.
  - Conformance: no edited sentence widens one of the four claims.
  - Recovery: revert documentation commits individually.
  - Compatibility: none.

Suppose EP-M2 or EP-M3 finds a construct that neither check can distinguish.
The plan then stops under the Design tolerance. With the user's agreement,
EP-M5 records the construct as an E0 finding that blocks 2.1.2. That is a valid
outcome of a feasibility task, not a failure to complete it.

## Concrete steps

Run every command from the worktree root. Tee each gate's output to a log file.
Read the log rather than re-running a gate, and run gates one at a time. The
logs are:

```plaintext
/tmp/check-fmt-polean-1-2-2-complete-module-admission-feasibility-record.out
/tmp/lint-polean-1-2-2-complete-module-admission-feasibility-record.out
/tmp/test-polean-1-2-2-complete-module-admission-feasibility-record.out
/tmp/verus-polean-1-2-2-complete-module-admission-feasibility-record.out
```

The commands, in order:

```bash
cargo fetch --locked
make check-fmt 2>&1 | tee /tmp/check-fmt-polean-1-2-2-complete-module-admission-feasibility-record.out
make lint 2>&1 | tee /tmp/lint-polean-1-2-2-complete-module-admission-feasibility-record.out
make test 2>&1 | tee /tmp/test-polean-1-2-2-complete-module-admission-feasibility-record.out
cargo insta review
make install-verus
make verus 2>&1 | tee /tmp/verus-polean-1-2-2-complete-module-admission-feasibility-record.out
make markdownlint
make design-check
sha256sum -c SHA256SUMS
```

The expected tail of a successful `make verus` run:

```plaintext
verification results:: N verified, 0 errors
```

Commit after each milestone with an imperative subject. Explain the reason and
the assurance impact in the body.

## Validation and acceptance

The behaviour to observe:

- `make test` passes. `admission::corpus` reports zero mismatches across all
  manifest entries. `admission::ast_mutations` prints mutant counts per class.
  The properties report their trivia-class counts.
- The snapshot `tests/admission/snapshots/admission__observations__corpus.snap`
  has one line per fixture, for example:

  ```plaintext
  trivia/metadata-block.rego  regorus=parsed  walk=admitted  recognizer=rejected trivia.metadata [0, 10)  outcome=rejected
  ```

- `make verus` reports zero errors.
- `docs/admission-feasibility-record.md` lists every construct class with its
  verdict and the fixture or test that demonstrates it.

Red-Green-Refactor evidence:

- Red:
  - EP-M1: the uncommitted stub classifier fails O3 for every non-admitted
    fixture.
  - EP-M3: each seeded kernel fault fails its named test.
  - EP-M4: Verus reports the unproved postcondition of `recognize`.
- Green: the committed code passes the same commands.
- Refactor: rerun `make check-fmt`, `make lint`, `make test`, and
  `make verus`.

The BDD specification lives at
`tests/admission/features/admission_feasibility.feature`. It sits outside
`tests/features/` so that editing it does not trigger the provisioned toolchain
workflow:

```plaintext
Feature: Complete-module admission feasibility for polean.tenant-write.v0

  Scenario: The baseline is admitted with its committed IR
    Given the baseline tenant-write source
    When the admission spike classifies it
    Then it is admitted with the IR in examples/tenant-write/policy.json

  Scenario: A metadata annotation is rejected although Regorus discards it
    Given the baseline source preceded by a METADATA comment block
    When the admission spike classifies it
    Then the AST walk alone admits it
    And it is rejected by the recognizer with code trivia.metadata

  Scenario: A parenthesized comparison is caught by the bounded source check
    Given the baseline source with one comparison wrapped in parentheses
    When the admission spike classifies it
    Then the AST walk alone admits it
    And it is rejected by the recognizer with code token.mismatch
```

No end-to-end test is added. The task introduces no command, persisted format,
or network boundary.

Quality criteria:

- Tests: `make test` passes, and so do O1–O5 and O7.
- Verification: `make verus` discharges O6 with no trusted shortcuts.
- Lint and format: `make check-fmt`, `make lint`, `make typecheck`, and
  `make markdownlint` pass.
- Documentation: `make design-check` passes, and `sha256sum -c SHA256SUMS`
  passes after refreshing the covered files.

## Idempotence and recovery

Every test is deterministic. If `proptest` ever finds a failure, the shrunk
case is persisted to that property's own regression file under
`tests/admission/properties/` and is committed with the repair; a later run
replays it before generating new cases. `make install-verus` installs into an
untracked, version-named directory and can be rerun.

If `insta` reports a changed snapshot, review it with `cargo insta review`. A
change means the frontend or the spike changed what it exposes, and must be
explained in `Surprises & discoveries`. Each milestone is one or more commits
that can be reverted independently.

## Artefacts and notes

### EP-M1 Red transcript (2026-10-30)

An uncommitted `tests/admission/stub_o3.rs` classified every source as
`rejected at profile with atom.unknown` and ran the O3 shape: walk the
manifest, compare each fixture's outcome with its expectation, and report every
disagreement in one failure. `cargo test --test admission stub_o3::` reported:

```text
running 1 test
test stub_o3::corpus_outcomes ... FAILED
...
87 fixture(s) disagree with the manifest:
...
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 18 filtered out
```

All 21 admitted fixtures disagreed, together with 66 of the 84 rejected
fixtures. The other 18 are the fixtures whose expected outcome is `profile` /
`atom.unknown`, which a constant reject-everything classifier matches by
accident; that is why the manifest uses distinct codes and why a test requires
a fixture for every code. The log is
`/tmp/red-o3-polean-1-2-2-complete-module-admission-feasibility-record.out`.
The stub and its `mod` declaration were deleted before the commit.

### EP-M1 observation snapshot summary (2026-10-30)

`tests/admission/snapshots/admission__observations__corpus.snap` has one line
for each of the 105 manifest entries. The frontend parsed 93 and rejected 12.
The manifest expects 21 admissions, 65 profile rejections, 7 recognizer
rejections, and 12 parse rejections, so every fixture that a later stage must
decide currently parses. The union of observed node kinds is all 46 inventory
variants. The rejections are the five generated fixtures (leading BOM, bare
carriage return, NUL in a comment, over-long token; CRLF parses),
`module/empty.rego`, the two v0 bodies, `limits/number-overflow.rego`, the two
`__target__` placement errors, `package/missing.rego`, and
`import/after-rule.rego`.

### EP-M2 observation snapshot summary (2026-10-30)

`tests/admission/snapshots/admission__observations__corpus.snap` now records
`walk=` and `outcome=` on each of its 105 lines. Admission admits 28 fixtures,
rejects 65 at the profile stage, and records no decision for the 12 that the
frontend rejects at parse. Every outcome is `match` except the seven
`outcome=gap` lines, which are exactly the fixtures the manifest marks
`stage: recognizer`: `atom/escaped-spelling`, `body/semicolon`, `body/unbraced`,
`trivia/metadata-block`, `trivia/metadata-inline`, `trivia/parentheses`, and
`trivia/unicode-comment`. Those seven are admitted by the walk: the
demonstrated EP-M2 gap, and the set O3 requires until EP-M3. The profile
rejections split as 18 `atom.unknown`, 13 `expr.kind`, 6 `rule.head`, 5
`import.shape`, 5 `operator.kind`, 5 `default.shape`, 4 `literal.kind`, 2
`package.shape`, 2 `rule.body`, and one each of `with.modifier`, `rule.limit`,
`order.shape`, `module.target`, and `body.limit`.

### EP-M2 mutation sweep (2026-10-30)

`tests/admission/snapshots/admission__ast_mutations__counts.snap` records the
sweep over the 93 fixtures the frontend parses: 77,884 mutants, every one
rejected at stage `decode` with the code its class names.

```plaintext
unknown-tag        mutants=4774
unknown-operator   mutants=320
unknown-key        mutants=9406
missing-key        mutants=25995
wrong-type         mutants=28385
wrong-arity        mutants=612
span               mutants=7392
optional-present   mutants=814
module-count       mutants=93
source-mismatch    mutants=93
total     mutants=77884
```

The sweep also requires the unmutated AST of every admitted fixture to be
admitted, so a decoder that rejected everything would not pass.

### EP-M3 observation snapshot summary (2026-10-30)

`tests/admission/snapshots/admission__observations__corpus.snap` now records
`walk=`, `recognizer=`, and `outcome=` on each of its 105 lines, and all 105
outcomes are `match`. The walk admits 28 fixtures. Of those, the recognizer
accepts 21 and refuses seven: `atom/escaped-spelling` and `body/semicolon` and
`body/unbraced` at `token.mismatch`, `trivia/metadata-block` and
`trivia/metadata-inline` at `trivia.metadata` (span `[0, 1)`, the leading `#`),
`trivia/parentheses` at `token.mismatch`, and `trivia/unicode-comment` at
`trivia.byte` (span `[47, 49)`, both bytes of `é`). The 65 profile rejections
and the 12 parse rejections are unchanged from EP-M2. Every recognizer span is
inside its source.

### EP-M3 property evidence (2026-10-30)

`properties::tests::every_generated_canonical_source_is_admitted` generates 64
cases with a deterministic `TestRunner` and requires each of the six trivia
classes — space, tab, newline, blank line, comment, and CRLF — to appear at
least four times across the generated gaps; the run passes, so every class
exceeds the floor. Each generated source must be admitted as exactly its own
rule list, confirmed by the witness, and agreed by the pinned lexer.
`inserting_text_into_a_comment_does_not_change_the_outcome` performs 32 random
insertions into a comment body and requires the same rule list each time.
`pinned_boundary_shapes` admits the empty rule list and the 16×16 maximum, and
requires `rule.limit` for 17 rules and `body.limit` for 17 literals.

### EP-M3 seeded-fault transcript (2026-10-30)

Each fault is a real kernel function with the kernel's signature, injected
through `witness_refusal`. The harness first requires the faulty kernel to
accept the source, then requires the witness to refuse the offsets it returns.
The four faults and the witness's message:

```plaintext
metadata rule skipped        the gap before token 0: the comment spells METADATA
comment ends at whitespace   the gap before token 1: a comment is not followed by a line feed
empty trivia                 the gap before token 1 is empty
bare carriage return         the gap before token 1: byte 0x0d at offset 24 is not trivia
```

### EP-M3 feasibility scenarios (2026-10-30)

`tests/admission/features/admission_feasibility.feature` states the three
scenarios the milestone's acceptance names, and `tests/admission/bdd.rs`
derives every scenario from the committed baseline at test time rather than
from a small fixture:

- the baseline `examples/tenant-write/policy.rego` is admitted with exactly the
  rule list in `examples/tenant-write/policy.json`;
- the same source preceded by a `# METADATA` comment block is admitted by the
  walk and refused by the recognizer at `trivia.metadata`;
- the same source with one comparison wrapped in parentheses is admitted by the
  walk and refused by the recognizer at `token.mismatch`.

The second and third scenarios are the feasibility claim in miniature: each
source is written in the profile's syntax, so the AST walk cannot tell it from
the baseline, and only the source bytes separate them. All three pass.

### EP-M3 gate evidence (2026-10-30)

`make check-fmt`, `make typecheck`, `make lint`, `make test`,
`make markdownlint`, and `make design-check` all pass on the candidate that
introduces the recognizer, properties, and scenarios: `make test` reports the
admission target's 60 tests, including the three scenarios; `make lint` is
clean under `cargo doc` with `-D warnings` and
`cargo clippy --all-targets --all-features -- -D warnings`. `make test-scripts`
and `make toolchain-smoke` are not applicable: the change touches no pin,
`scripts/`, `lean/`, or `tests/toolchain/` path.

### EP-M4 Red transcript (2026-10-31)

The Red state is the kernel's contract with the specification attached and no
proof steps. A scratch copy of the four Verus inputs under
`/tmp/verus-red-probe/` had every `proof { … }` block removed and kept every
contract, invariant, and assertion; the probe was not committed.
`verus --multiple-errors 40 verus/admission_recognizer.rs` reported:

```plaintext
error: possible arithmetic underflow/overflow
   --> verus/../tests/admission/recognizer/kernel.rs:116:25
    |
116 |         if byte_at(src, at + j) != metadata_byte(j) {
    |                         ^^^^^^

error: possible arithmetic underflow/overflow
   --> verus/../tests/admission/recognizer/kernel.rs:187:15
    |
187 |     } else if at + 1 < src.len() {
    |               ^^^^^^

error: invariant not satisfied before loop
   --> verus/../tests/admission/recognizer/kernel.rs:302:13
    |
302 |             trivia(src@, at as int, index as int, last),

error: invariant not satisfied at end of loop body
   --> verus/../tests/admission/recognizer/kernel.rs:376:13
    |
376 |             flags_prefix(src@, toks@, offsets@, needs_newline@, index as int),

verification results:: 34 verified, 6 errors
```

The six errors are the arithmetic obligations and the loop invariants that
carry the postcondition. Verus reports the first unmet obligation at the loop
that needs it, so the empty proof fails there rather than at the function's
closing `ensures`. The log is `/tmp/verus-epm4-red.out`.

### EP-M4 Green transcript (2026-10-31)

With the lemmas and invariants in place, `make verus` reports:

```plaintext
VERUS_Z3_PATH="…/.verus/z3/z3" "…/.verus/0.2026.09.06.8dea4a2/verus/verus" "verus/admission_recognizer.rs"
verification results:: 40 verified, 0 errors
```

The kernel contains no `assume`, `admit`, or `external_body`. The log is
`/tmp/verus-polean-1-2-2-complete-module-admission-feasibility-record.out`.
`make test-scripts` passes 70 tests with the Python 3.14 interpreter, including
24 new tests for `scripts/install_verus_z3.py`, run through
`uv run --python 3.14 --with pytest==9.1.1 python -m pytest scripts/tests`
after the repository's default interpreter reported that it lacks
`compression.zstd`.

### EP-M4 gate evidence (2026-10-31)

All eight applicable gates pass on the candidate that adds the Verus inputs,
the pins, and the Z3 installer, run sequentially by the scrutineer against the
final working tree:

| Gate                   | Result                                                                                 |
| ---------------------- | -------------------------------------------------------------------------------------- |
| `make check-fmt`       | `cargo fmt --all -- --check` clean; `mdtablefix --check` reports 30 files unchanged    |
| `make typecheck`       | `cargo check --all-targets --all-features` with `-D warnings`, no diagnostics          |
| `make lint`            | `cargo doc` with `-D warnings` and Clippy clean                                        |
| `make test`            | admission 60, documentation-example 27, stub 2, toolchain 18 with 7 ignored, doctest 1 |
| `make markdownlint`    | 31 files, 0 issues                                                                     |
| `make design-check`    | 5 schemas, 6 witnesses, 19 negative controls, 2 boundary seams                         |
| `make test-scripts`    | 70 passed under Python 3.14 (substituted interpreter)                                  |
| `make toolchain-smoke` | 7 smoke tests pass, 18 filtered out                                                    |

`make verus` is not part of the commit gates and is covered by the Green
transcript above. The logs are
`/tmp/<gate>-polean-1-2-2-complete-module-admission-feasibility-record.out`.

### EP-M5 gate evidence (2026-10-31)

All ten gates pass on the pre-rebase candidate that adds the record, ADR 0005,
the amended documents, and the refreshed `SHA256SUMS`, run sequentially by the
scrutineer against the working tree at `16dd183`. The post-rebase, post-repair
evidence is recorded in “EP-M5 gate evidence after the rebase and Whitaker
repair” below.

| Gate                   | Result                                                                                          |
| ---------------------- | ----------------------------------------------------------------------------------------------- |
| `make check-fmt`       | `cargo fmt --all -- --check` clean; `mdtablefix --check` reports 32 unchanged                   |
| `make typecheck`       | `cargo check --all-targets --all-features` with `-D warnings`, no diagnostics                   |
| `make lint`            | `cargo doc` with `-D warnings` and Clippy clean                                                 |
| `make test`            | 108 passed, 0 failed, 7 ignored: admission 60, doc-examples 27, stub 2, toolchain 18, doctest 1 |
| `make markdownlint`    | 33 files, 0 issues                                                                              |
| `make design-check`    | 5 schemas, 7 source headers, 224 comparisons, 19 negative controls, 2 seams                     |
| `make test-scripts`    | 70 passed under Python 3.14 via `uv run --with pytest==9.1.1`                                   |
| `make toolchain-smoke` | 7 passed, 0 failed, from the provisioned `.polean/tools`                                        |
| `make verus`           | 40 verified, 0 errors                                                                           |
| `sha256sum -c`         | 39 entries, every line `OK`                                                                     |

The standalone `python3.14` on this host has no `pytest`, so
`make test-scripts` cannot start under `PYTHON=<that interpreter>`. The run
above uses the `uv run --python 3.14 --with pytest==9.1.1` invocation recorded
in the developer guide, which leaves no `uv.lock` or `.venv` behind. The failed
precondition is kept at
`/tmp/test-scripts-polean-1-2-2-complete-module-admission-feasibility-record-pre-uv.out`
rather than overwritten. Logs use the branch slug
`polean-1-2-2-complete-module-admission-feasibility-record`.

### EP-M5 gate evidence after the rebase and Whitaker repair (2026-11-02)

The candidate was replayed onto `2ca284c`, which adds the Whitaker lint to
`make lint` and the standard `-Zthreads=8`/mold build flags. The replay itself
was clean; the repairs it forced are recorded in the 2026-11-02 Progress entry.
All ten gates then pass, run sequentially by the scrutineer against the working
tree:

| Gate                   | Result                                                                                                |
| ---------------------- | ----------------------------------------------------------------------------------------------------- |
| `make check-fmt`       | `cargo fmt --all -- --check` clean; `mdtablefix --check` reports 32 files unchanged                   |
| `make typecheck`       | `cargo check --all-targets --all-features` with `-D warnings`, no diagnostics                         |
| `make lint`            | `lint-whitaker`, `cargo doc -D warnings`, and `cargo clippy -D warnings` all clean                    |
| `make test`            | 110 passed, 0 failed, 7 ignored: admission 60, doc-examples 27, stub 2, toolchain 20, doctest 1       |
| `make markdownlint`    | 33 files, 0 issues                                                                                    |
| `make design-check`    | unchanged from the pre-rebase run: 5 schemas, 7 source headers, 224 comparisons, 19 controls, 2 seams |
| `make test-scripts`    | 76 passed under Python 3.14.7 and pytest 9.1.1 via the documented `uv run` invocation                 |
| `make toolchain-smoke` | 7 passed, 0 failed, with `PYTHON=/home/leynos/.local/bin/python3.14`                                  |
| `make verus`           | 41 verified, 0 errors                                                                                 |
| `sha256sum -c`         | 39 entries, every line `OK`                                                                           |

Two gates depend on the interpreter present on the host rather than on the
candidate. `make test-scripts` still fails to start under the default
`python3`, which is 3.12 and has no `compression.zstd`; the run above uses the
`uv run --python 3.14 --with pytest==9.1.1` command from the developer guide.
The failing pre-run is preserved at
`/tmp/test-scripts-polean-1-2-2-complete-module-admission-feasibility-record-default-python3.out`,
so the interpreter condition is recorded rather than erased.
`make toolchain-smoke` needs the same 3.14 interpreter, and this host has one at
`/home/leynos/.local/bin/python3.14` with the smoke test's dependencies but no
`pytest`, which is why the two gates take different overrides. Neither
condition is a regression of this candidate.

Logs use the branch slug
`polean-1-2-2-complete-module-admission-feasibility-record`.

### Assessment repair and corpus reconciliation (2026-11-02)

The ExecPlan assessment of `10aec6f` raised five in-scope findings. All five
are repaired on the candidate:

1. The walker's six atom mappings are hand-written in
   `tests/admission/admission/walk/body.rs` and no longer read the profile's
   atom table; the recognizer renderer stays contract-derived, and a separate
   renderer-fault test covers the `locked`/`unlocked` swap.
2. O6's T1 and T2 source-wide exclusions and the concrete accepting proof are
   in `verus/admission_spec.rs` and `verus/admission_lemmas.rs`, with their
   token-byte and no-`#` conditions stated in `tools/verus/README.md`.
3. The generated `nul_in_comment` fixture now places the NUL inside a trailing
   comment after an admissible prefix, so the frontend parses and admits the
   prefix while the recognizer refuses the submitted bytes. O7's named
   adversarial refusals (token after `#`, NUL in a comment, `//`) are separate
   normal-kernel tests.
4. `scripts/install_verus_z3.py` validates a cached solver against the current
   pin and reinstalls or rejects a mismatch, with changed-pin and invalid-cache
   regression tests.
5. The properties run through the shrinking runner, the trivia floor counts
   emitted gaps, and `expr/data-reference.rego` supplies the dedicated rule-body
   `data` fixture.

The proof assessment of the same head raised four further findings. Three are
repaired by the work above: O6's T1, T2, and concrete baseline (finding 1), the
README's token conditions (finding 2), and the solver-cache identity check
(finding 4). The fourth asked for verifier-negative controls, which are now
`scripts/check_verus_controls.py`, run by `make verus-controls` and by
`make verus`:

- three body controls remove one kernel rule each — the `METADATA` scan, the
  non-empty gap rule, and the line-feed rule — with the specification
  unchanged, and require Verus to reject the result at that rule's obligation;
- two specification controls weaken `body_byte` to admit NUL and drop
  `comment_ok`'s metadata exclusion, with the kernel unchanged, and require
  Verus to reject T1's `lemma_comment_body_permitted` and T2's
  `lemma_comment_no_metadata` respectively.

Each control runs on a scratch copy, requires the unmodified copy to verify
first, and requires its anchor to occur exactly once, so a drifted source fails
closed rather than replacing nothing. At the assessed head the proof verified
59 functions with 0 errors, and all five controls were rejected at the named
obligation.

Both the repair and the lemma split landed as `804a0fb`, which was then rebased
onto `05edbba`; the replay was clean, and the only replayed difference is the
target's `mdtablefix` 0.6.1 pin in `docs/developers-guide.md`. The rebased head
`03a965d` passed every gate: formatting, lint, typecheck, Markdown lint, tests,
design checks, the script suite (114 tests), toolchain smoke, the checksum
manifest, and `make verus` with 59 verified, 0 errors, and all five controls
rejected at the named obligations.

The corpus therefore changed after the EP-M1 to EP-M3 summaries above, which
record the snapshots as they stood on 2026-10-30. The committed corpus now has
106 manifest entries: 21 admitted and 85 rejected, split as 11 at parse, 66 at
the profile walk, and 8 at the recognizer. The frontend parses 95 fixtures, and
the mutation sweep records 79,181 mutants across the same ten classes. The
observation snapshot has 106 lines and every outcome is `match`.

Scope exception: applying this plan's tolerances exclusions to the rebased base
`05edbba` and the head that carries the assessment repair leaves 96 non-fixture
changed files and 14,072 net added lines (14,137 insertions and 65 deletions),
and the corpus holds 94 `.rego` fixtures. At the earlier assessed head
`1785056` the same calculation gives 91 files and 12,670 net added lines
(12,735 insertions and 65 deletions); the repair added five files and 1,402
lines. The tolerances are 40 files, about 4,000 lines, and 90 fixtures, so all
three are exceeded. The user approved the plan at revision 4 and, on
2026-11-02, directed the work to continue through assessment repair to merge;
that instruction is the recorded escalation approval for these three
exceedances. No further escalation is outstanding.

### ExecPlan assessment repair after `1785056` (2026-11-02)

The ExecPlan assessment `polean-17-execplan-20261102-1785056` raised four
findings against the head. All four are repaired on the candidate:

1. T2's metadata exclusion. `annotates_metadata` recognised only the adjacent
   `#METADATA`, so `# METADATA` and a marker later in a comment escaped the
   theorem, even though the executable scanner and `comment_ok` exclude them.
   The predicate is now stated over marker positions after the hash and before
   the line ending, and the exclusion is proved for it under the same no-`#`
   token condition. The whitespace-prefixed case is part of the specification
   test.
2. The executable accepting instance. `lemma_baseline_accepts` proves the
   decomposition but no return value, and the contract is one-way, so an
   always-refusing kernel would satisfy it. `lemma_baseline_is_canonical` now
   proves the concrete baseline is a canonical whitespace-only decomposition,
   and `kernel::baseline_is_recognised` is an executable function whose
   postcondition is `Ok(baseline_offsets())`; the concrete baseline test calls
   it. `lemma_baseline_accepts` stays as the separate decomposition evidence.
   The completeness contract this rests on adds `ws_canonical` and
   `admits_canonical` over slices, their `Seq<Seq<u8>>` mirrors
   `ws_canonical_t` and `admits_canonical_t`, the extent and no-marker scans,
   `lemma_ws_canonical_unique`, and the transport lemma
   `lemma_ws_canonical_t_to_slices`.
3. Property-test failure persistence. The runner set
   `failure_persistence: None`, so a shrunk case could not outlive the run.
   Each property now names its own regression file under
   `tests/admission/properties/`, and `Idempotence and recovery` describes that
   behaviour instead of a mechanism the runner lacked.
4. The evidence record. The `nul_in_comment` row now says that the frontend
   reads the NUL as end of input and admits the prefix, with the recognizer
   refusing the submitted bytes; and the scope arithmetic above now applies
   this plan's stated exclusions to the requested base and head, giving 96
   files and 14,072 net added lines at the repair head and retaining the
   earlier 91-file, 12,670-line totals as the `1785056` measurement.

`make verus` reports 89 verified obligations and no errors on this head, and
all five negative controls are rejected at the named obligation. The count rose
from 59 because the completeness contract and the executable instance add their
own obligations.

The added proof material pushed four modules past Whitaker's 400-line limit, so
the repair also split them along their existing seams: the completeness
contract moved to `verus/admission_spec_completeness.rs`, the concrete scans
and transport lemma to `verus/admission_lemmas/concrete.rs`, the kernel's
trivia scan to `kernel/trivia.rs`, and the executable instance to
`kernel/baseline.rs`. No proof statement changed, and the controls' anchor for
the metadata rule moved with the function it edits. The witness description is
now stated as structural independence: the checker re-derives offsets by direct
slicing and a separate gap traversal, but shares the kernel's `has_metadata`
helper, so it is not a second implementation of every classification rule. The
proof assessment's fuel correction is accepted: the scan specifications take no
semantic fuel parameter, their recursion terminates by `decreases`, and
`reveal_with_fuel` controls unfolding during the concrete proof rather than
imposing an execution bound.

### Gate evidence for the assessment repair (2026-11-03)

All ten commit gates pass on the candidate that carries the repair, run
sequentially against `1785056` plus the repair changes:

| Gate                      | Result                                         |
| :------------------------ | :--------------------------------------------- |
| `make check-fmt`          | Clean; 32 files unchanged.                     |
| `make typecheck`          | No diagnostics.                                |
| `make lint`               | `cargo doc`, Clippy, and Whitaker clean.       |
| `make test`               | 118 passed, 7 ignored, 0 failed.               |
| `make markdownlint`       | 33 files, 0 issues.                            |
| `make design-check`       | Passed.                                        |
| `make test-scripts`       | 114 passed.                                    |
| `make toolchain-smoke`    | 7 passed, 20 filtered out.                     |
| `make verus`              | 89 verified, 0 errors; five controls rejected. |
| `sha256sum -c SHA256SUMS` | 39 entries, all OK.                            |

The five rejected controls are `body:metadata-rule-dropped`,
`body:empty-gap-rule-dropped`, `body:line-feed-rule-dropped`,
`spec:body-byte-admits-nul`, and `spec:comment-ok-admits-metadata`. Each is
rejected at the obligation named in the control script, and the metadata
control's anchor is `kernel/trivia.rs` after the module split.

### CodeRabbit review repair (2026-11-03)

CodeRabbit's review of `e76e879` raised 22 inline findings. All 22 are applied
in one repair commit; none was rebutted. Three of them change behaviour rather
than wording, and each tightens a check that previously accepted more than the
contract allows:

- `tests/admission/recognizer/lexer.rs`: `cross_check` now records the first
  lexer index of each canonical token and refuses a token that spans more than
  one lexer token unless it contains `.`. Before this, a lexer that split `==`
  or `:=` into two symbols was accepted on the strength of the concatenated
  text, so the cross-check could not distinguish a correct lexer from one that
  tokenized differently. Only a dotted reference may legitimately span several
  lexer tokens.
- `tests/admission/corpus/mod.rs`: `ir_rules` now reads the hand-written IR
  closed. The root must be an object whose keys are exactly `profile` and
  `rules`, `profile` must equal the `id` in `contracts/profile.json`, and any
  other key or a non-object root is refused. The reviewer's literal suggestion
  was to require `rules` to be the only key; that would reject every committed
  IR file, because each is `{"profile": ..., "rules": [...]}`, so the check
  enforces the shape the corpus actually has. A misspelt key can no longer
  leave part of the expected rule list silently unchecked.
- `Makefile`: `PROVER_TOOLS_REF` is filtered and validated inside `awk`
  (`length($$2) == 40 && $$2 !~ /[^0-9a-f]/`) and the guard tests the result
  for emptiness, so an unvalidated value is never interpolated into a shell
  command.

The remaining findings are hardening and consistency:
`scripts/check_verus_controls.py` resolves the selected `--verus` and `--z3`
paths before the `is_file` check, so a relative argument still works when
`run_verus` runs with `cwd=scratch`; `scripts/install_verus_z3.py` narrows the
archive's `platform` group to `[A-Za-z0-9._-]+`, so a name containing `/` or
`..` is refused before it can build a path or URL;
`scripts/tests/test_check_verus_controls.py` gains a diagnostic message on
every assertion, modern type hints, a `fake_run` that snapshots each control
path so the test proves the baseline is restored before each control, and
fixtures that exercise the non-zero error count rather than the missing-summary
branch; `scripts/tests/test_install_verus_z3.py` checks the committed pin
through `read_pin()` and uses an always-expired deadline.

The rest are documentation corrections. `tests/admission/support.rs` is new: it
holds the `manifest()`, `profile()`, and `source(entry)` helpers that eight
test modules had each copied privately, and those modules now import them with
their assertions unchanged. `tools/verus/README.md` names `Z3_SHA256SUMS` and
the installer in its opening paragraph, and "Bumping the pin" gains a step that
compares the new release's `source/tools/get-z3.sh` Z3 version with
`Z3_SHA256SUMS`. The remaining edits correct spelling to Oxford `-ize` forms
and replace "the frontend's JSON spelling" with "serde_json's normalized text"
in the AST decoder, which is what the code actually reads.

Every gate was rerun on the repair commit, in repository order. The logs are
`/tmp/<gate>-polean-1-2-2-complete-module-admission-feasibility-record.out`.

| Gate                      | Result                                                                                  |
| :------------------------ | :-------------------------------------------------------------------------------------- |
| `make check-fmt`          | Clean; 32 files unchanged.                                                              |
| `make typecheck`          | No diagnostics.                                                                         |
| `make lint`               | `cargo doc`, Clippy, and Whitaker clean.                                                |
| `make test`               | 119 passed, 7 ignored, 0 failed.                                                        |
| `make markdownlint`       | 33 files, 0 issues.                                                                     |
| `make design-check`       | 5 schemas, 6 witnesses, 224 comparisons, 19 controls, 2 seams.                          |
| `make test-scripts`       | 115 passed under Python 3.14.7 and pytest 9.1.1 via the documented `uv run` invocation. |
| `make toolchain-smoke`    | 7 passed, 20 filtered out.                                                              |
| `make verus`              | 89 verified, 0 errors; five controls rejected.                                          |
| `sha256sum -c SHA256SUMS` | 39 entries, all OK.                                                                     |

`make test-scripts` cannot start on this host, because neither the default
`python3` nor the installed 3.14 interpreter has `pytest`. The substitute above
is the invocation documented in the developer guide, and it runs the same test
files with the same interpreter version.

### Pre-merge check reconciliation (2026-11-03)

CodeRabbit's assessment of `01d422b` reports two errors and one warning, and
separates five findings across them. All five are repaired here; none is
deferred, and no repair weakens a test or a trust boundary.

- **Unit Architecture, the Rust comparison** was correct and its repair is
  retained. `admission::compare` is typed as a pure comparison, but its
  admitted branch called `corpus::ir_rules`, which reads and parses the fixture
  IR; an unreadable fixture therefore surfaced as a comparison mismatch.
  `Expected::Admitted` carries the parsed `rules`, filled once by
  `corpus::entries()` at the boundary that already owns corpus I/O, and
  `compare_admitted` only compares slices. The regression test that supplies an
  unusable IR path and still requires a match remains.
- **Unit Architecture, the controls runner.**
  `scripts/check_verus_controls.py` called the module-level `run_verus`, and
  its orchestration test replaced that global with `monkeypatch`. `check()` now
  takes a `runner` argument typed as
  `Runner = Callable[[Path, Path, Path], subprocess.CompletedProcess[str]]`, and
  `main()` supplies the production `run_verus`. The orchestration test passes
  its recorder instead of patching anything, so it exercises the real control
  sequence while observing every verifier call.
- **Unit Architecture, the installer's transport and clock.**
  `scripts/install_verus_z3.py` called `urllib.request.urlopen` and
  `time.monotonic` directly, so the size bound and the deadline could only be
  exercised by patching the standard library. `download()` now takes an
  `opener: Opener` and a `clock: Clock`; `fetch_from_release()` composes
  `open_release` with `time.monotonic`, and `install()` keeps `fetch`
  defaulting to it. The download tests inject a prepared response stream and a
  stepping clock, so a size or deadline failure is asserted directly.
- **Testing (Unit And Behavioural).** New command-boundary tests, with no change
  to the behaviour they observe: `scripts/tests/test_install_verus_z3_cli.py`
  runs the installer's `main` against a loopback HTTP release server for the
  successful install, malformed pin, download failure, digest failure, and
  cached-reuse scenarios; `scripts/tests/test_check_verus_controls_cli.py` runs
  the controls CLI against a stand-in verifier for clean orchestration, missing
  tools, a broken baseline, an accepted mutation, and an unrelated rejection,
  and it asserts the working directory each verifier run receives, so the
  staged root is shown to be the script's own scratch tree rather than the
  source repository; `scripts/tests/test_verus_make_targets.py` checks the
  `install-verus`, `verus-proof`, `verus-controls`, and aggregate `verus`
  recipes in an isolated repository layout, including the complete pinned
  launcher: `uv tool run --from` over the prover-tools source at the committed
  ref, the setup diagnostic for an absent and for a non-executable verifier,
  and the malformed-pin diagnostic. Together they cover `main()` error
  conversion and exit status, successful CLI orchestration, and Makefile
  argument forwarding, which are the three gaps the row named. The synthetic
  release fixtures they share live in `scripts/tests/z3_archive.py`.
- **Testing (Compile-Time / Ui).** `tests/admission/compile_fail/` adds a
  compiler harness with no new dependency. It copies the repository beside the
  build's target directory and compiles the copy's `admission` test binary
  three times: unmodified, which must compile and is the control against an
  environment failure; with the `Expr::Null` arm removed from
  `inventory/walk.rs` and the `ExprKind::Null` arm removed from `ast/count.rs`,
  which must stop the build with `E0004` naming both files; and with each arm
  replaced by a catch-all, which must compile and so shows the failure came
  from exhaustiveness checking alone. Removing an arm is the diagnostic a new,
  unhandled variant produces, and the protected matches are the real ones
  rather than a toy enum. A wildcard left in either match makes the second
  phase compile, so the guard fails for exactly the weakening it exists to
  detect. Every filesystem operation goes through `cap_std::fs::Dir` handles
  opened from an ambient authority at the repository and at the target
  directory, because Whitaker's `no_std_fs_operations` denies `std::fs` in test
  modules as well as in `src/`.
- **Issues.** #30 and #31 record the same two coverage gaps as non-blocking
  follow-ups. The coverage the issues describe is implemented here, but neither
  issue is closed by this change: both remain open while the delivery is
  unreviewed, and are closed through the issues workflow only after the pull
  request lands. No duplicate issue is created.

All ten gates were rerun on this repair head, sequentially and in repository
order. The logs are
`/tmp/<gate>-polean-1-2-2-complete-module-admission-feasibility-record.out`.

| Gate                      | Result                                                                                  |
| :------------------------ | :-------------------------------------------------------------------------------------- |
| `make check-fmt`          | Clean; 32 files unchanged.                                                              |
| `make typecheck`          | No diagnostics.                                                                         |
| `make lint`               | `cargo doc`, Clippy, and Whitaker clean.                                                |
| `make test`               | 122 passed, 7 ignored, 0 failed; the admission binary contributes 72 of the passes.     |
| `make markdownlint`       | 33 files, 0 issues.                                                                     |
| `make design-check`       | 5 schemas, 6 witnesses, 224 comparisons, 19 controls, 2 seams.                          |
| `make test-scripts`       | 134 passed under Python 3.14.7 and pytest 9.1.1 via the documented `uv run` invocation. |
| `make toolchain-smoke`    | 7 passed, 20 filtered out.                                                              |
| `make verus`              | 89 verified, 0 errors; five controls rejected.                                          |
| `sha256sum -c SHA256SUMS` | 39 entries, all OK.                                                                     |

`make test` now includes the compile-fail guard, which builds a copy of the
repository three times and accounts for most of the admission binary's 129 s.
The guard's first version called `std::fs` and was rejected by Whitaker's
`no_std_fs_operations`; its second read `CARGO_TARGET_DIR`, which `clippy.toml`
disallows, and left four Clippy diagnostics. The committed version derives the
target directory from `current_exe` and `CACHEDIR.TAG` and touches the
filesystem only through `cap_std::fs::Dir`.

### Pre-merge check reconciliation, second round (2026-11-04)

CodeRabbit's source-level assessment of `5c47b57` accepted Testing (Unit And
Behavioural), Unit Architecture, and Testing (Compile-Time / Ui) by inspection,
and kept Testing (Overall) partly unresolved. Every remaining objection is
repaired here; none claimed that a recipe currently misbehaves.

- **Malformed pin.** `test_malformed_pin_stops_every_target` replaces the
  isolated layout's `tools/rust-prover-tools/REF` with `ref: not-a-commit`,
  runs the aggregate `verus` target, and requires a non-zero result, the
  `PROVER_TOOLS_REF missing or malformed` diagnostic naming that file, and an
  empty call log. The installer's malformed-checksum test protects a different
  boundary and is not offered in its place.
- **Non-executable verifier.**
  `test_non_executable_verus_is_a_named_setup_error` is parameterised over
  `verus-proof` and `verus-controls`, places the pinned verifier with mode
  `0o644`, and requires the installation diagnostic and an empty call log. A
  regression from `test -x` to `test -f` would run the stand-in and fail that
  log assertion.
- **Complete launcher arguments.** `test_install_verus_calls_both_installers`
  now asserts the whole `uv` argument list: `tool run --from` over the
  prover-tools source at the committed ref, then `prover-tools verus install`
  with `--repo-root .`, so an unpinned or installed-copy launcher is caught.
- **Working directory.** The controls CLI stand-in records `Path.cwd()`. The
  orchestration test requires every baseline and control run to share one root,
  that root to be the script's `polean-verus-controls-` scratch tree, and that
  it lies outside the source repository.
- **Issue status.** #30 and #31 are open and stay open until the delivery is
  accepted; neither the ExecPlan nor the PR description says this change closes
  them.

All ten gates were rerun on this repair head, sequentially and in repository
order, with the logs at
`/tmp/<gate>-polean-1-2-2-complete-module-admission-feasibility-record.out`.
The first pass failed `make check-fmt` because the new prose was reformatted
after the gate had read it; the reflow was corrected and both Markdown gates
were rerun on the final tree.

| Gate                      | Result                                                                                  |
| :------------------------ | :-------------------------------------------------------------------------------------- |
| `make check-fmt`          | Clean; 32 files unchanged.                                                              |
| `make typecheck`          | No diagnostics.                                                                         |
| `make lint`               | `cargo doc`, Clippy, and Whitaker clean.                                                |
| `make test`               | 122 passed, 7 ignored, 0 failed; the admission binary contributes 72 of the passes.     |
| `make markdownlint`       | 33 files, 0 issues.                                                                     |
| `make design-check`       | 5 schemas, 6 witnesses, 224 comparisons, 19 controls, 2 seams.                          |
| `make test-scripts`       | 137 passed under Python 3.14.7 and pytest 9.1.1 via the documented `uv run` invocation. |
| `make toolchain-smoke`    | 7 passed, 20 filtered out.                                                              |
| `make verus`              | 89 verified, 0 errors; five controls rejected.                                          |
| `sha256sum -c SHA256SUMS` | 39 entries, all OK.                                                                     |

### Fresh pre-merge assessment request (2026-11-04)

The `CodeRabbit` check run for `0e8ca3e` reports `Review paused`, so that head
has no review coverage and the pre-merge table still describes `5c47b57`. A
focused reconciliation comment for `0e8ca3e` was posted through the manual
comment route (comment `6638773196`), giving an evidence-backed disposition for
each of the four rows and for the issue-status correction, and asking for a
fresh assessment. It does not queue a full review on its own.

A single `@coderabbitai resume` request was queued afterwards, because the head
is ready and its review was skipped rather than rate-limited. The queue
accepted it as `ff67e4a3` with a posting estimate of roughly six hours; posting
the comment is not completion of the review, and the review decision, thread
state, and required checks remain separate facts until the review runs on
`0e8ca3e`.

## Interfaces and dependencies

Everything below is private to the `tests/admission` binary. No `src/` item is
added.

```rust
enum Outcome {
    Admitted(Ir),
    Rejected(Rejection),
}

/// Throwaway test IR; 2.1.1 owns the real type.
struct Ir {
    rules: Vec<Vec<Atom>>,
}

enum Atom {
    SameTenant,
    WriteAction,
    AdminRole,
    OwnsResource,
    Locked,
    Unlocked,
}

struct Rejection {
    stage: Stage,
    code: ReasonCode,
    span: Option<ByteSpan>,
}

enum Stage {
    Parse,
    Serialize,
    Decode,
    Profile,
    Recognizer,
}

/// Closed list; the coverage test requires a fixture for every variant.
enum ReasonCode {
    /* e.g. PackageShape, ImportMissing, DefaultShape, RuleHead, RuleBodies,
    LiteralKind, WithModifier, ExprKind, Operator, AtomUnknown, RuleLimit,
    BodyLimit, ModuleTarget, AstUnknownTag, AstUnknownKey, AstMissingKey,
    AstType, AstArity, AstSpan, TriviaMetadata, TriviaByte, TokenMismatch,
    ... */
}

struct ByteSpan {
    start: usize,
    end: usize,
}

/// Per-stage trace for the snapshot and the BDD scenarios.
struct Classification {
    regorus: Result<(), String>,
    walk: Result<Ir, Rejection>,
    recognizer: Option<Result<Vec<usize>, usize>>,
    outcome: Outcome,
}

fn classify(source: &str) -> Classification;

/// Closed decoder: whole tree first, then the profile mapping.
fn decode(ast_json: &str, source: &str) -> Result<Module, Rejection>;

fn map_profile(module: &Module) -> Result<Ir, Rejection>;

/// Rendered from contracts/profile.json atoms, never from the walker's table.
fn canonical_tokens(ir: &Ir, atoms: &AtomText) -> (Vec<&[u8]>, Vec<bool>);

/// The proved kernel. Ok gives each token's start offset.
fn recognize(src: &[u8], toks: &[&[u8]], needs_newline: &[bool]) -> Result<Vec<usize>, usize>;
```

`Module` here is the spike's decoded view of the AST, not Regorus's type. A
recognizer `Err(i)` with `i < len` becomes the span `[i, i + 1)` at a character
boundary; `i == len` becomes the empty span `[len, len)`.

The module layout puts each directory with tests under `mod.rs` plus
`tests.rs`, because `self_named_module_files` is denied, and keeps every file
under 400 lines:

- `main.rs` and `outcome.rs`;
- `corpus/`, `inventory/`, and `observations/`;
- `decode/`, holding the key table and a generic closed-object check;
- `profile/`;
- `recognizer/`, with `kernel/` (`mod.rs` and `scan.rs`), `render.rs`, and
  `witness.rs`;
- `ast_mutations/` and `properties/`;
- `bdd.rs`.

Dependencies:

- Existing: Regorus `=0.13.0`, `serde_json`, `rstest`, `rstest-bdd`,
  `rstest-bdd-macros`, `insta`, `googletest`, and `pretty_assertions`.
- New: `proptest` (`=`-pinned, dev-only).
- Added by D14:
  - `vstd = "=0.0.0-2026-09-06-0133"` (dev-only, already in `Cargo.lock`);
  - Verus through `prover-tools`, pinned by `tools/verus/VERSION`,
    `tools/verus/SHA256SUMS`, and `tools/rust-prover-tools/REF`;
  - Makefile targets `install-verus` and `verus`, following the `verus`
    skill's on-ramp.

Documentation changes in EP-M5:

- `docs/admission-feasibility-record.md` (new). It covers:
  - the question and the method;
  - the verdict per construct class, with links to fixtures and tests;
  - a table mapping each `excluded` entry of `contracts/profile.json` to its
    reason codes, or to "not applicable to source admission";
  - the AST gaps and the bounded check that closes each;
  - the Regorus and OPA divergences;
  - E0 findings;
  - the proposed status and exit mapping for 2.1.2;
  - the commit at which `make verus` passed;
  - what 2.1.1 and 2.1.2 must carry forward.
- `docs/adrs/adr-0005-admit-by-ast-mapping-and-canonical-token-recognition.md`
  (new, short). It records the conjunction design, the trust it removes and
  retains, and the alternatives rejected.
- `docs/technical-design.md`:
  - §4.1: the narrowings D7–D11 and the Q1 answer;
  - §6.1: about three sentences plus links to the record and ADR 0005;
  - §13: the "Admission coverage" row;
  - References: the OPA parser and scanner sources.
- `docs/terms-of-reference.md`: the A5 resolution evidence cites the record.
- `docs/adrs/adr-0002-first-profile-assurance-boundary.md`: a dated note that
  open question 2 is answered by the record.
- `docs/roadmap.md`: tick 1.2.2.
- `docs/developers-guide.md`:
  - the admission spike;
  - the corpus and manifest conventions;
  - `make install-verus` and `make verus`;
  - what the proof does and does not cover.
- `docs/repository-layout.md`:
  - rows for `tests/admission/`, `examples/admission/`, `verus/`, and
    `tools/`;
  - the `tests/` row amended to include feasibility spikes;
  - a note that prover tools are dev-only and outside the E0 tuple.
- `docs/contents.md`: the record, ADR 0005, and this plan.
- `docs/users-guide.md`: no change, because nothing user-visible changes. The
  record says so.
- `SHA256SUMS`: refreshed for every covered file edited, including the two
  entries that already drift on `main`. `examples/admission/` is not added.

## Revision notes

- Revision 1 (2026-10-29): initial draft, based on Wyvern reconnaissance of
  the Regorus 0.13.0 sources, the repository harness, the governing documents,
  and OPA's metadata recognition.
- Revision 2 (2026-10-29): adopts the community-of-experts conditions.
  - Red team:
    - restricted comment bytes to printable ASCII, closing a BOM-in-comment
      false admission (D8);
    - rendered tokens from `contracts/profile.json` and added an independent
      `locked` fixture;
    - added fixtures for the Regorus divergences.
  - Formal methods:
    - required non-empty tokens;
    - replaced the word-separation rule with non-empty trivia everywhere;
    - allowed carriage return only before line feed;
    - replaced the trivial metadata lemma with T1 and T2;
    - adopted a witness-returning recognizer with an independent checker;
    - added a Regorus lexer cross-check;
    - adopted the single-source Verus kernel, replacing the mirror and its
      drift test.
  - Rust and contracts:
    - decoding now runs before the profile, with precedence;
    - the mutation sweep covers every parsed fixture, with defined classes;
    - the inventory guard uses equality and ordinals;
    - added the `Classification` trace and optional spans;
    - corpus reporting is data-driven;
    - adjusted the size budget.
  - Proportionality:
    - removed ingestion checks, the exit taxonomy, OPA legality runs (O9),
      and the insertion property (O8);
    - decided Q2–Q5 of revision 1 narrowly as D7–D11;
    - asked the user only about the escaped-spelling narrowing (Q1) and the
      Verus timing and dependency (Q2);
    - committed a green EP-M1 rather than an ignored test;
    - moved the feature file out of the provisioned workflow's path filter.
  Remaining work is unchanged in shape: approval, then EP-M1 to EP-M5.
- Revision 3 (2026-10-29): records the user's acceptance of Q1 and Q2 as D13
  and D14. It authorizes `vstd` at the locked version, and puts EP-M4 firmly in
  scope. It also restores the literal `\u0069` escape in the Q1 example, which
  had been decoded when the file was written. The plan still awaits explicit
  approval to begin EP-M1.
- Revision 4 (2026-10-29): records the user's approval to begin
  implementation and marks the plan `APPROVED`. No plan content changed.
- Revision 5 (2026-11-02): records the rebase onto `2ca284c` and the gate
  repairs it required. The milestone scope, the four assurance claims, and the
  verdict are unchanged; the kernel's module layout, the obligation count, and
  the evidence tables are updated to the replayed candidate.
