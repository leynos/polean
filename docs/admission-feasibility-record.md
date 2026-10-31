# Admission feasibility record: `polean.tenant-write.v0`

**Status:** evidence record for roadmap item 1.2.2. Admission is exercised by a
test-only spike under `tests/admission/`; no production command admits policies
yet. **Date:** 2026-10-31. **Audience:** project sponsor, implementers, and
technical reviewers. **Companions:**
[technical design](technical-design.md), [terms of reference](terms-of-reference.md),
[ADR 0005][adr-0005], and the
[ExecPlan](execplans/1-2-2-complete-module-admission-feasibility-record.md)
that produced this evidence.

## 1. The question

Terms-of-reference assumption A5 and ADR 0002 open question 2 ask whether the
pinned Regorus 0.13.0 frontend exposes enough of a module to enforce an exact,
fail-closed allowlist, without Polean becoming an unverified Rego parser that
must be trusted wholesale.

The answer in this record applies only to the pinned tuple: Rust as locked,
`regorus =0.13.0` with features `["ast", "std"]`, and the profile
`polean.tenant-write.v0` as committed in
[`contracts/profile.json`](../contracts/profile.json). It says nothing about a
later frontend, a different profile, or hostile sources.

## 2. Method

The spike classifies each source through three checks:

1. **Closed decoding.** The public `Engine::get_ast_as_json` output is decoded
   whole by a Polean-owned tree with fixed tag, key, type, and arity tables.
   Unknown variants, unknown keys, missing keys, wrong types, bad spans, a
   module count other than one, and a source-text mismatch are errors.
2. **Profile walk.** The decoded module is checked for the profile's package,
   import, default, ordering, limits, heads, bodies, literal kinds, modifiers,
   expressions, operators, and registered atoms. This decides what the module
   means.
3. **Canonical-token recognition.** The admitted rule list is rendered, from
   `contracts/profile.json` and never from the walker's own tables, to the
   exact token sequence a canonical source must spell. A bounded kernel checks
   that the source bytes are those tokens separated by permitted trivia, and
   returns every token's start offset as a witness.

A source is admitted only when all three checks agree. The recognizer can only
narrow acceptance: the walk still decides meaning, and the recognizer never
produces intermediate representation (IR).

The corpus holds one fixture for every constructive form in the profile, one
fixture for each excluded construct class, and the baseline plus six mutations.
The manifest in `examples/admission/manifest.json` is a machine-readable
contract: every fixture has an entry, every entry has a fixture, and each entry
names the stage and reason code that must reject it. The 46 AST node kinds the
pinned frontend can produce are all observed by the corpus.

## 3. Result

**Admission through the pinned frontend is feasible.** Every roadmap success
criterion is met, and there are no E0 findings:

| Criterion                                                     | Result                                                                                                                                                |
| ------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1: package, import, and default                              | Enforced by the profile walk; fixtures in `examples/admission/{package,import,default}/`.                                                             |
| S2: every rule and expression                                 | Every one of the 46 node kinds is observed; the closed decoder and walk cover all of them; fixtures in `head/`, `literal/`, `expr/`, and `operator/`. |
| S3: metadata                                                  | Rejected by the recognizer as `trivia.metadata`; fixtures `trivia/metadata-block.rego` and `trivia/metadata-inline.rego`.                             |
| S4: unknown AST variants                                      | Rejected by the closed decoder; 77,884 structural mutants across ten classes are all rejected at `decode`.                                            |
| S5: missing API coverage has a bounded check or an E0 finding | Six AST gaps are closed by the recognizer; the kernel's soundness is proved in Verus. No gap remains open.                                            |
| S6: unreachable syntax is not ignored                         | Unreachable v0 forms are parse errors; `unreachable/v0-body.rego` and `unreachable/v0-partial-set.rego` are refused.                                  |

The manifest's 105 entries split as 21 admitted and 84 rejected: 12 at parse,
65 at the profile walk, and 7 at the recognizer. Nineteen of the 28 reason
codes are reachable from source; the remaining nine describe AST JSON
corruption and are exercised by the mutation sweep and the decoder's
malformed-input test. The observation snapshot records, for every fixture, what
Regorus exposed, what the walk decided, what the recognizer decided, and
whether that matches the manifest. The corpus test reports zero mismatches.

## 4. Verdicts by construct class

"Bounded check" below means the canonical-token recognizer. Every verdict is
backed by at least one committed fixture; the snapshot
`tests/admission/snapshots/admission__observations__corpus.snap` is the
point-in-time record of each.

### 4.1 Admitted forms

| Construct                                          | Enforced by                      | Evidence                                                                                      |
| -------------------------------------------------- | -------------------------------- | --------------------------------------------------------------------------------------------- |
| Package `authz`                                    | Profile walk                     | `package/` fixtures; `package/dotted.rego` and `package/other.rego` are rejected.             |
| Import `rego.v1`, unaliased                        | Profile walk                     | `import/` fixtures; aliases, extra imports, and wrong imports are rejected.                   |
| `default allow := false`                           | Profile walk                     | `default/` fixtures; missing, duplicated, assigned, renamed, or `true` defaults are rejected. |
| Zero to 16 `allow if { ... }` rules                | Profile walk                     | `limits/rules-max.rego` admitted; `limits/rules-over.rego` rejected.                          |
| One to 16 comparisons per body                     | Profile walk                     | `limits/body-max.rego` admitted; `limits/body-over.rego` rejected.                            |
| The six registered atoms                           | Profile walk, then bounded check | One fixture per atom under `atom/`, plus `admitted/locked-owner-rule.rego`.                   |
| Exact canonical spelling                           | Bounded check                    | `atom/escaped-spelling.rego` is rejected although the AST carries `"write"`.                  |
| Braced body and the `if` keyword                   | Bounded check                    | `body/unbraced.rego` is rejected although the walk admits it.                                 |
| One comparison per line, no `;`                    | Bounded check                    | `body/semicolon.rego` is rejected; `body/single-line.rego` with one literal is admitted.      |
| Spaces, tabs, blank lines, CRLF, ordinary comments | Bounded check                    | `trivia/{blank-lines,tabs,comment-spoof}.rego` and generated `crlf`.                          |

A comment that merely looks like a body literal is trivia and does not change
meaning (`trivia/comment-spoof.rego` is admitted). A comment with a byte
outside printable ASCII and tab is refused (`trivia/unicode-comment.rego`).

### 4.2 Refused forms

Each row gives the rejecting stage and the reason code that stage reports, as
the wire values the manifest records.

| Construct class                                            | Stage        | Code                                         | Evidence                                                                        |
| ---------------------------------------------------------- | ------------ | -------------------------------------------- | ------------------------------------------------------------------------------- |
| Other packages, imports, rule names                        | `profile`    | `package.shape`, `import.shape`, `rule.head` | `package/other.rego`, `import/other.rego`, `head/other-name.rego`               |
| Extra or duplicated default                                | `profile`    | `default.shape`                              | `default/{missing,two-defaults}.rego`                                           |
| More than one body, `else`                                 | `profile`    | `rule.body`                                  | `body/{two-bodies,else}.rego`                                                   |
| `with` modifier                                            | `profile`    | `with.modifier`                              | `body/with.rego`                                                                |
| Variables, unification, negation, `some`, `every`          | `profile`    | `literal.kind` or `expr.kind`                | `literal/{some-vars,some-in,not-expr,every}.rego`, `operator/assign-eq.rego`    |
| Calls, unary, arithmetic, membership, assignment           | `profile`    | `expr.kind`                                  | `expr/{call,unary,membership,assign}.rego`, `operator/arith-*.rego`             |
| Comparisons other than `==`                                | `profile`    | `operator.kind`                              | `operator/bool-{lt,le,ge,gt,ne}.rego`                                           |
| Collections and comprehensions                             | `profile`    | `atom.unknown`                               | `expr/{array,set,object,*-compr}.rego`                                          |
| Bracketed and otherwise unmatched references               | `profile`    | `atom.unknown`                               | `atom/bracket-reference.rego`, `expr/ref-brack.rego`                            |
| Other literal values and operand orientations              | `profile`    | `atom.unknown`                               | `atom/{other-literal,reversed-operands,swapped-locked,unknown-attribute}.rego`  |
| Reversed rule and default order                            | `profile`    | `order.shape`                                | `order/default-after-rule.rego`                                                 |
| `__target__`                                               | `profile`    | `module.target`                              | `module/target.rego`                                                            |
| Metadata annotation                                        | `recognizer` | `trivia.metadata`                            | `trivia/{metadata-block,metadata-inline}.rego`                                  |
| Redundant parentheses                                      | `recognizer` | `token.mismatch`                             | `trivia/parentheses.rego`                                                       |
| Non-canonical comment bytes                                | `recognizer` | `trivia.byte`                                | `trivia/unicode-comment.rego`                                                   |
| Rego v0 syntax in an unreachable rule                      | `parse`      | `parse.syntax`                               | `unreachable/{v0-body,v0-partial-set}.rego`                                     |
| Malformed modules, imports after a rule, duplicate imports | `parse`      | `parse.syntax`                               | `module/{empty,two-packages,target-after-rules}.rego`, `import/after-rule.rego` |
| Number outside the frontend's range                        | `parse`      | `parse.number`                               | `limits/number-overflow.rego`                                                   |
| Unknown or malformed AST JSON                              | `decode`     | `ast.*`                                      | The mutation sweep and `ast/tests.rs`                                           |

## 5. The profile's excluded entries

Each `excluded` entry in `contracts/profile.json` maps to a reason code or is
not applicable to source admission. The mapping is part of the record, not only
of the code, so a future profile change must revisit it.

| Profile exclusion     | Reason code                                                               | Evidence                                                                                                                          |
| --------------------- | ------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Other packages        | `package.shape`                                                           | `package/other.rego`                                                                                                              |
| Other imports         | `import.shape`                                                            | `import/other.rego`                                                                                                               |
| Additional modules    | `ast.module_count` at the decode stage; a second package is a parse error | `module/two-packages.rego`; decoder module-count test                                                                             |
| Metadata annotations  | `trivia.metadata`                                                         | `trivia/metadata-*.rego`                                                                                                          |
| Other rules           | `rule.head`                                                               | `head/other-name.rego`                                                                                                            |
| Variables             | `literal.kind` or `atom.unknown`                                          | `expr/var.rego`, `literal/some-vars.rego`                                                                                         |
| Unification           | `expr.kind`                                                               | `operator/assign-eq.rego`                                                                                                         |
| Negation              | `literal.kind`                                                            | `literal/not-expr.rego`                                                                                                           |
| `else`                | `rule.body`                                                               | `body/else.rego`                                                                                                                  |
| `with`                | `with.modifier`                                                           | `body/with.rego`                                                                                                                  |
| `some`                | `literal.kind`                                                            | `literal/some-in.rego`, `literal/some-vars.rego`                                                                                  |
| `every`               | `literal.kind`                                                            | `literal/every.rego`                                                                                                              |
| Built-in calls        | `expr.kind`                                                               | `expr/call.rego`                                                                                                                  |
| Functions             | `rule.head`                                                               | `head/function.rego`                                                                                                              |
| `data` references     | `atom.unknown`                                                            | No dedicated fixture; the atom lookup rejects any reference that is not a registered atom, as `atom/unknown-attribute.rego` shows |
| Dynamic references    | `atom.unknown`                                                            | `expr/ref-brack.rego`, `atom/bracket-reference.rego`                                                                              |
| Numeric expressions   | `expr.kind` or `operator.kind`                                            | `operator/arith-*.rego`, `operator/bool-*.rego`                                                                                   |
| Collections           | `atom.unknown`                                                            | `expr/{array,set,object,*-compr}.rego`                                                                                            |
| Arbitrary Lean source | Not applicable to source admission                                        | The verifier's template boundary, not source syntax                                                                               |
| Custom invariants     | Not applicable to source admission                                        | A claims-set construct, not source syntax                                                                                         |

## 6. What the AST does not expose, and how each gap is closed

Reconnaissance found six properties the AST discards. Each is closed by the
bounded check; none needed a Polean parser.

| Gap             | What is lost                                       | Closure                                                                            | Fixture                                                    |
| --------------- | -------------------------------------------------- | ---------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| Comments        | Regorus's lexer discards them entirely             | Trivia must be whitespace or a permitted comment; comment bytes are restricted     | `trivia/comment-spoof.rego`, `trivia/unicode-comment.rego` |
| Metadata        | A `# METADATA` comment leaves no AST trace         | Any comment body containing `METADATA` is refused outright                         | `trivia/metadata-block.rego`                               |
| Parentheses     | The parser drops them                              | Source bytes must equal the rendered token sequence                                | `trivia/parentheses.rego`                                  |
| `if` and braces | Neither is recorded                                | The rendered tokens include `if` and `{` … `}`                                     | `body/unbraced.rego`                                       |
| Separators      | `;` and line structure are not recorded            | Each non-first literal must begin after a line feed, and separators are not trivia | `body/semicolon.rego`                                      |
| Token spelling  | The AST holds decoded values, not source spellings | Tokens are compared as exact canonical bytes                                       | `atom/escaped-spelling.rego`                               |

The recognizer's rules are those of D8–D11 and D13: trivia is a space, a tab, a
line feed, a carriage return immediately before a line feed, or a comment;
comment bodies are tab and bytes `0x20`–`0x7E`; every pair of adjacent tokens
is separated by non-empty trivia; the gap before each non-first body literal
holds a line feed; and escaped spellings are refused. These are narrowings of
§4.1, approved with the plan, not widenings.

## 7. Frontend divergences

These are fail-closed differences between the pinned frontend and OPA 1.21.2.
They are recorded for 2.1.1 and 2.3.2; they do not weaken admission, because
each refuses a source that OPA might accept.

| Shape                              | Regorus 0.13.0                                                 | OPA 1.21.2        | Evidence                         |
| ---------------------------------- | -------------------------------------------------------------- | ----------------- | -------------------------------- |
| Leading byte-order mark            | Parse error                                                    | Accepted          | `generated/leading_bom`          |
| Bare carriage return               | Parse error unless followed by a line feed                     | Accepted          | `generated/bare_carriage_return` |
| Line over 1,024 columns            | Parse error on a token that wide; comment width is not checked | Accepted          | `generated/over_long_line`       |
| Embedded NUL in a comment          | Truncates the module; the package is reported missing          | Rejected          | `generated/nul_in_comment`       |
| CRLF line endings                  | Accepted; treated as trivia                                    | Accepted          | `generated/crlf`                 |
| Number beyond the frontend's range | Rejected inside `add_policy`                                   | Accepted at parse | `limits/number-overflow.rego`    |

OPA was not executed by the spike (D6); the OPA column comes from reading the
pinned sources at the cited points, as the plan requires. OPA legality belongs
to the source-oracle work (2.3.2) and hostile-source coverage (4.2.3).

## 8. Verification evidence

The recognizer kernel is verified in Verus. The kernel is compiled twice — by
Cargo as test code and by Verus through `#[path]` — so the proved function is
the one the spike calls.

| Property        | Statement                                                                                                                           | Where                                      |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ |
| Decomposition   | An `Ok` from `recognize` places each token's exact bytes at its offset, with permitted trivia in every gap and after the last token | `verus/admission_spec.rs`, `decomposes_at` |
| Line-feed flags | Every token whose flag is set follows a gap containing a line feed                                                                  | `flags_hold`                               |
| Metadata        | A comment body containing `METADATA` is never accepted                                                                              | `comment_ok`, `metadata_in`                |
| Refusal         | An `Err` offset is never past the end of the source                                                                                 | `recognize`'s postcondition                |
| Scope           | The property holds for sources of any length; no bound on token count                                                               | O6                                         |

`make verus` reports 40 verified obligations and no errors at commit `16dd183`.
The proof contains no `assume`, `admit`, or `external_body`. The Red transcript
in the ExecPlan shows the same contracts and invariants with the proof steps
removed: Verus then reports 34 verified and 6 errors, the arithmetic
obligations and the invariants that carry the postcondition.

The theorem is about the kernel only. It does not establish that:

- the renderer produces the right tokens for a rule list — that is tested, not
  proved;
- the AST walk maps the frontend's tree faithfully — that is a closed case
  analysis, tested and mutation-checked, not proved;
- the refusal classifier names the right reason code — it inspects only sources
  the kernel already refused, so a wrong code cannot change an admission;
- Regorus assigns the tokens the meaning Polean reads — that is the translation
  claim, which remains open.

`make verus` is an on-demand target. It is not part of `make all` or CI, so an
ordinary build needs neither the Verus release nor the pinned Z3 solver.

## 9. Assurance boundaries

The four claims stay separate.

1. **Profile membership.** Established by the spike for the corpus and by the
   generated properties, not for arbitrary sources.
2. **Model satisfaction.** Untouched. No Lean theorem was produced or checked by
   this task.
3. **Source-to-model correspondence.** Not proved. Admission is trusted in the
   sense ADR 0002 describes: Regorus parses, and the Polean decoder, walk, and
   renderer decide. The Verus theorem removes one source of doubt, the kernel's
   byte-level check, but it does not turn the frontend into a verified parser.
4. **Runtime correspondence.** Untouched. OPA and Regorus were not compared in
   this task.

A passing corpus, a mutation sweep, or a checked kernel theorem is evidence for
the first claim only. None of them authenticates request attributes, proves a
model invariant, or certifies an application integration.

## 10. E0 findings

None. No construct was found that can be distinguished neither through the AST
nor by the bounded check. The Design tolerance was therefore not triggered, and
this record does not block roadmap item 2.1.2.

The two gaps that remain are future work rather than findings:

- the corpus has no dedicated fixture for a `data` reference inside a rule body;
  the atom lookup rejects it by shape, and `atom/unknown-attribute.rego`
  exercises that lookup, but a direct fixture would make the evidence local;
- OPA was not executed, by design; the divergence table rests on source reading.

Neither affects the admission verdict for the pinned tuple.

## 11. Proposed status and exit mapping for 2.1.2

The spike has no CLI outcome. For production admission, this record proposes
the mapping D5 already fixed, consistent with
[`contracts/cli.txt`](../contracts/cli.txt) and technical design §8:

| Spike outcome                                 | Production outcome                | Exit |
| --------------------------------------------- | --------------------------------- | ---- |
| Walk rejection (`profile.*`)                  | `unsupported`                     | 4    |
| Recognizer rejection (`recognizer.*`)         | `unsupported`                     | 4    |
| Frontend parse rejection (`parse.*`)          | `invalid_input`                   | 5    |
| Decoder or serialization failure (`decode.*`) | `tool_error`                      | 6    |
| Admitted                                      | Continue to the requested command | 0    |

A production implementation must keep the reason codes closed and keep byte
spans in a sidecar, so that diagnostics cannot change the formal meaning of a
policy.

## 12. What 2.1.1 and 2.1.2 must carry forward

- **2.1.1 (ingestion and IR):** the source cap and UTF-8 gate; canonical byte
  order and final line feed; rule and atom order preserved; zero rules allowed;
  empty bodies, duplicate keys, and unknown fields refused. The spike's IR is a
  throwaway test type and must not be promoted.
- **2.1.2 (production admission):** the conjunction of a closed decoder, the
  profile walk, and the canonical-token recognizer; the six reason-code
  families; the D8–D11 and D13 trivia rules; fail-closed handling of every
  unsupported form; and the exit mapping above.
- **Both:** the recognizer kernel and its proof are intended for promotion. The
  renderer, witness, and classifier need their own review, because only the
  kernel is verified.
- **2.3.2:** the OPA divergences in §7, and the differential cases they imply.

## 13. References

- [Technical design](technical-design.md) §§4.1, 6.1, 6.3, 8, and 10.
- [Terms of reference](terms-of-reference.md) A2 and A5.
- [ADR 0002](adrs/adr-0002-first-profile-assurance-boundary.md) open question 2.
- [ADR 0003](adrs/adr-0003-pin-the-e0-toolchain-tuple.md): the pinned tuple.
- [ADR 0005][adr-0005]: the admission design.
- [ExecPlan](execplans/1-2-2-complete-module-admission-feasibility-record.md):
  milestones, decisions, red and green transcripts, and gate evidence.

[adr-0005]: adrs/adr-0005-admit-by-ast-mapping-and-canonical-token-recognition.md
