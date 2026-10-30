# Produce a complete-module admission feasibility record

This ExecPlan is a living document. Keep `Constraints`, `Tolerances`, `Risks`,
`Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan`
current as work proceeds.

Status: APPROVED (revision 4). The user accepted Q1 and Q2 and approved the
plan to begin implementation on 2026-10-29.

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
  can only narrow acceptance. Each `Ok` returns a witness that an independent
  checker validates, and O7 cross-checks the token stream against Regorus's own
  lexer. ADR 0005 records this division of trust.
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
  verify, confirms every accepted offset list with an independent witness, and
  cross-checks the tokens against the pinned Regorus lexer. Four seeded kernel
  faults (metadata rule skipped, token after a comment, empty trivia, bare
  carriage return) are each caught by the witness.
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
- [ ] EP-M4: Verus soundness proof of the recognizer kernel (D14).
- [ ] EP-M5: feasibility record, ADR 0005, and documentation updates.

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

## Outcomes & retrospective

Not started. At completion, record the verdict for each construct class, every
E0 finding and whether it blocks 2.1.2, the commit at which `make verus`
passed, and lessons for 2.1.2.

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
  returns, in the corpus and in every property case, an independent checker
  confirms `decomposes_at` by direct slicing and classification.
  - Method: a deliberately simple checker, written without reference to the
    kernel's loop structure.
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
  - Artefact: `tests/admission/recognizer/kernel.rs`, verified through the
    crate root `verus/admission_recognizer.rs`, which contains only a
    `#[path]` module declaration.
  - Evidence: `make verus` reports zero errors. Before the proof is complete,
    the run shows the open postcondition, which is the Red state.
  - Non-vacuity: the antecedent is inhabited, shown by a proof that
    `recognize` accepts one concrete baseline token sequence. T1 and T2 show
    that the specification excludes NUL, BOM, and metadata. `grep` confirms
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

Every test is deterministic. If `proptest` ever finds a failure, its regression
files are committed under `tests/admission/`. `make install-verus` installs
into an untracked, version-named directory and can be rerun.

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

Still to be filled in during implementation:

- the Verus Red and Green transcripts.

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
- `recognizer/`, with `kernel.rs`, `render.rs`, and `witness.rs`;
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
