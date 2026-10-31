# Architectural decision record (ADR) 0005: admit by AST mapping and canonical-token recognition

## Status

Accepted. Admission is the conjunction of a closed AST decode, a profile walk,
and a canonical-token recognizer whose kernel is verified in Verus.

## Date

2026-10-31.

## Context and problem statement

Terms-of-reference assumption A5 and ADR 0002 open question 2 ask whether the
pinned Regorus 0.13.0 frontend can support complete fail-closed admission of
`polean.tenant-write.v0` without Polean becoming an unverified parser that must
be trusted wholesale.

Regorus exposes a parsed module as JSON through `Engine::get_ast_as_json`, but
the AST is lossy for admission:

- comments are discarded, so an OPA-meaningful `# METADATA` annotation leaves no
  trace;
- redundant parentheses are dropped;
- the AST records neither the `if` keyword nor whether a rule body was braced;
- `;` separators and line structure are not recorded;
- string literals carry decoded values, not the source spelling.

Each of those differences can make two sources with different meanings look
identical to an AST-only adapter. The feasibility spike (roadmap 1.2.2) had to
either close them, widen the accepted language, or record an E0 finding that
blocks roadmap 2.1.2.

## Decision Drivers

- Admission must fail closed on every source the profile does not admit.
- Polean must not become a second Rego parser that must be trusted as a whole.
- The meaning of a policy must be decided in one place, from the AST.
- Any extra check must be small enough to state and verify, not another
  interpreter.
- Unsupported syntax must be refused even in a rule that can never fire.

## Options considered

### Option A: AST mapping only

Map the decoded AST to the profile and reject anything else.

Rejected: the six gaps above are invisible, so a parenthesised comparison, a
`;`-separated body, or a `METADATA` annotation would be admitted silently. This
is the outcome A5 asks about and it fails the requirement.

### Option B: AST mapping with span-anchored residual accounting

Map the AST, then use Regorus spans to check that every source byte not covered
by a node is whitespace.

Rejected: the check trusts Regorus spans, and spans do not cover comments or
the discarded `if` and brace tokens. It also cannot distinguish a comment from
whitespace without its own lexical rules.

### Option C: a Polean-written Rego parser that produces IR

Parse the source directly with a Polean parser and use Regorus only as a second
opinion.

Rejected: this is the wholesale-trust outcome ADR 0002 warns about, duplicates
the frontend, and creates a much larger verification obligation than the one
this task can discharge.

### Option D: AST mapping plus a canonical-token recognizer

Map the decoded AST to IR, render the canonical token sequence from the
profile's atom table, and check that the source bytes are exactly those tokens
separated by permitted trivia. Return each token's start offset as a witness,
confirm it with an independent checker, cross-check the tokens against the
pinned lexer, and prove the kernel's soundness in Verus.

Selected. The recognizer cannot widen acceptance, because it runs only after
the walk has admitted a module and it never produces IR. It closes every known
gap at once, and its kernel is small enough to verify for sources of any length.

### Option E: use `opa parse` as the metadata detector

Run OPA alongside Regorus and refuse when OPA sees a metadata annotation.

Rejected: it moves an independent oracle into the admission path, merges
profile membership with runtime correspondence, and adds a second provisioned
runner to a task that is about the Regorus boundary.

## Decision outcome

Admission is the conjunction of three checks:

1. a closed decoder over `get_ast_as_json`, with fixed tag, key, type, and
   arity tables, where unknown or missing structure is an error;
2. a profile walk that decides package, import, default, order, limits, heads,
   bodies, literal kinds, modifiers, expressions, operators, and atoms;
3. a canonical-token recognizer that renders the admitted rule list, from
   `contracts/profile.json` and never from the walker's own table, and checks
   the source bytes against that token sequence.

The recognizer's expected tokens come from the profile, so an escaped spelling
of a registered atom cannot match. Its trivia rules are those of the plan's
D8–D11 and D13: printable ASCII and tab in comments, `METADATA` refused in any
column, non-empty gaps between tokens, a line feed before each non-first body
literal, braced bodies, and no `;`.

The recognizer's kernel is verified in Verus. The theorem states that an `Ok`
decomposes the source into the rendered tokens with permitted trivia and that
the line-feed flags hold; it says nothing about what Rego means by those bytes.
The proof is in `verus/`, compiled from the same kernel source Cargo tests.

## Consequences

Trust removed: Polean no longer needs to trust that an AST-only adapter saw
every meaningful source distinction. Comments, parentheses, `if`, braces,
separators, and spellings are decided by bytes the verifier accounts for.

Trust retained:

- Regorus parses the source and decides what the AST contains;
- Polean's decoder and walk map that tree to IR;
- Polean's renderer chooses the canonical tokens, and its correctness is tested
  rather than proved;
- the refusal classifier maps an offset to a reason code, and is unverified
  because it only inspects sources the kernel already refused.

Consequences for later work:

- 2.1.1 owns the production source and IR types; the spike's IR is a throwaway
  test type and must not be promoted.
- 2.1.2 owns production admission, diagnostics, and source maps; it must carry
  the conjunction and the closed reason codes forward.
- `make verus` remains an on-demand target outside `make all` and CI, so the
  Verus release and Z3 stay dev-only, pinned, and digest-checked.
- The frontend divergences recorded in the feasibility record belong to source
  oracles (2.3.2) and hostile-source coverage (4.2.3), not to this decision.

## Known risks and limitations

- A rule list that renders wrongly would make the recognizer refuse a valid
  canonical source or accept a non-canonical one. The property tests and the
  lexer cross-check reduce this risk; they do not prove the renderer.
- The kernel theorem does not establish translation. If Regorus assigns the
  tokens a different meaning from Polean's IR, admission still succeeds and the
  model theorem is about the IR. That is the open source-binding claim, and no
  part of this decision discharges it.
- The corpus is a finite set of fixtures. It covers every AST node kind the
  pinned frontend produces, but it is not a proof about arbitrary sources; the
  kernel proof is what makes the bounded check parametric for all inputs.

## References

- [Feasibility record](../admission-feasibility-record.md).
- [ADR 0002](adr-0002-first-profile-assurance-boundary.md) open question 2.
- [Technical design](../technical-design.md) §§4.1, 6.1, and 6.3.
- [ExecPlan](../execplans/1-2-2-complete-module-admission-feasibility-record.md)
  decisions D4, D8–D11, D13, and D14.
