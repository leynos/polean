import PoleanAudit.Render

/-!
# Bounded rendering and truncation tests

Direct tests of `PoleanAudit.Render.treeBounded` and
`PoleanAudit.Render.truncateUtf8`, built and run by
`tests/toolchain/smoke/audit/render.rs` under `make toolchain-smoke`. The
worker's `main` cannot be imported into a test executable, so the bounded
operations live in their own module and are driven here.

The shared-expression cases are the reason this file exists. The corpus cannot
show that the walk is bounded: every fixture's statement is a tree the kernel
already built, and a guard that summed nodes on the way back up would still
accept every one of them. Here the graph is built by sharing, so the tree it
expands to is exponential while the graph is tiny. The last case, `shared 40`,
expands to 2^41 - 1 nodes if the budget guard is bypassed; the Rust side runs
this executable under a deadline, so such a bypass fails as a timeout instead of
hanging the gate.

The truncation cases pin the contract's message bound from both sides: a
message at the limit is unchanged, a message over it keeps the exact prefix
that fits, and a multi-byte scalar is never split. Two-, three-, and four-byte
scalars are each checked alone and behind an ASCII prefix, at the limit and one
byte across it, so a `charBytes` that undercounts a wide scalar is caught. An
implementation that returns its input, returns a constant, or drops the
scalar-boundary check fails at least one of them.

The name cases pin the component encoding. A name component is either text or a
number, and the contract keeps them distinct: `A.1` as a string component and
`A.1` as a numeric component render differently, so a walk that stringified
every component would still produce a well-formed array while losing the
distinction settlement relies on.

The universe-level cases pin the rendered JSON rather than a node count. A
constant's levels are part of the proposition settlement compares, so a walk
that dropped a successor or treated a non-ground level as ground would still
spend the right number of nodes while reporting the wrong statement. The three
cases assert the exact JSON for zero, for a successor chain, and for a level
the contract cannot render.

Those cases name the boundaries; they cannot show the bounds hold for shapes
nobody wrote down. The rest of the file therefore generates inputs and checks
each against a property that does not restate the implementation.

For `truncateUtf8` the property needs no second implementation. The result must
be a prefix of the input, must fit the limit, and must be at least as long as
every prefix of the input that fits; the last clause states maximality without
consulting `charBytes`. Inputs mix ASCII, two-, three-, and four-byte scalars at
every length up to four, and each is tried at every limit from zero to one byte
past its size.

For `treeBounded` the oracle is a fold over the `Expr` that counts nodes and
depth and decides renderability from the grammar alone, never from the walk's
budget. The walk must accept exactly the trees that fold admits within the
bounds, and must report the fold's count. The trees mix every name and level
list the renderer admits, two forms outside the grammar, and applications of
those leaves up to two deep, each at every depth bound from zero to four and
every node budget from zero to five.
-/

open Lean

/-- A constant leaf. -/
def leaf : Expr := mkConst ``Nat

/-- A right-nested spine of `n` applications: `2 * n + 1` nodes and depth
    `n + 1`, because each application adds a node and a leaf operand. -/
def spine : Nat → Expr
  | 0 => leaf
  | n + 1 => mkApp (spine n) leaf

/-- A doubling chain. `shared (n + 1)` applies `shared n` to itself, so the
    graph holds `n + 1` nodes and the tree it denotes has `2 ^ (n + 1) - 1`. -/
def shared : Nat → Expr
  | 0 => leaf
  | n + 1 =>
    let inner := shared n
    mkApp inner inner

/-- The rendered node count, or `none` when the walk refused the tree. -/
def rendered (depth budget : Nat) (e : Expr) : Option Nat :=
  (treeBounded depth budget e).map (·.2)

/-- A message one byte longer than the contract's diagnostic bound. -/
def oversized : String := String.ofList (List.replicate (maxMessageBytes + 1) 'a')

/-- The rendered JSON of a constant with the given levels. -/
def constantJson (levels : List Nat) : Json :=
  Json.mkObj [("const", nm ``Nat), ("levels", toJson levels)]

/-- Report one failed check and return whether it passed. -/
def check (label : String) (passed : Bool) : IO Bool := do
  if passed then
    return true
  IO.eprintln s!"FAILED: {label}"
  return false

/-- Every string of exactly `n` characters over `alphabet`. -/
def stringsOfLength (alphabet : List Char) : Nat → List String
  | 0 => [""]
  | n + 1 => (stringsOfLength alphabet n).flatMap fun s => alphabet.map fun c => s.push c

/-- Every string of at most `n` characters over `alphabet`. -/
def stringsUpTo (alphabet : List Char) (n : Nat) : List String :=
  (List.range (n + 1)).flatMap fun k => stringsOfLength alphabet k

/-- Every prefix of `s`, longest last. -/
def prefixes (s : String) : List String :=
  let cs := s.toList
  (List.range (cs.length + 1)).map fun k => String.ofList (cs.take k)

/-- The truncation property: a prefix, inside the limit, and maximal.

    Maximality is stated against the prefixes themselves — no fitting prefix is
    longer than the result — so the property owes nothing to `charBytes`. -/
def truncationHolds (s : String) (limit : Nat) : Bool :=
  let r := truncateUtf8 limit s
  let isPrefix := r.toList.isPrefixOf s.toList
  let withinLimit := r.utf8ByteSize ≤ limit
  let maximal := (prefixes s).all fun p =>
    p.utf8ByteSize > limit || p.toList.length ≤ r.toList.length
  isPrefix && withinLimit && maximal

/-- Inputs mixing ASCII, two-, three-, and four-byte scalars up to length four,
    plus a few interleavings of all four widths. -/
def truncationInputs : List String :=
  stringsUpTo ['a', 'é', '€', '😀'] 4 ++ ["a€😀é", "é€😀a", "😀a€"]

/-- Every generated truncation instance, labelled by its input and limit. -/
def truncationCases : List (String × Bool) :=
  truncationInputs.flatMap fun s =>
    (List.range (s.utf8ByteSize + 2)).map fun limit =>
      (s!"truncating {repr s} to {limit} bytes keeps the longest fitting prefix",
        truncationHolds s limit)

/-- Whether a universe level is one the contract can render. -/
def groundLevel : Level → Bool
  | .zero => true
  | .succ l => groundLevel l
  | _ => false

/-- A tree's node count and depth, or `none` when a node is outside the grammar.

    This is the oracle: it folds over the `Expr` and never consults the walk's
    budget. A shared expression counts once per path, which is the tree the
    contract talks about. -/
def treeShape : Expr → Option (Nat × Nat)
  | .const _ ls => if ls.all groundLevel then some (1, 1) else none
  | .app f a =>
    match treeShape f, treeShape a with
    | some (nf, df), some (na, da) => some (nf + na + 1, max df da + 1)
    | _, _ => none
  | _ => none

/-- The walk's expected answer: the node count exactly when the tree is
    renderable and inside both bounds. -/
def expectedRender (depth budget : Nat) (e : Expr) : Option Nat :=
  match treeShape e with
  | none => none
  | some (nodes, level) => if nodes ≤ budget && level ≤ depth then some nodes else none

/-- The walk accepts exactly the trees inside both bounds and reports the
    oracle's count. -/
def treeHolds (depth budget : Nat) (e : Expr) : Bool :=
  rendered depth budget e == expectedRender depth budget e

/-- Names a statement tree can carry. -/
def sampleNames : List Name :=
  [``Nat, .str (.str .anonymous "AuditFixture") "Goal", .num .anonymous 3]

/-- Level lists, including one the contract cannot render. -/
def sampleLevels : List (List Level) :=
  [[], [.zero], [.succ .zero], [.succ (.succ .zero)], [.param `u]]

/-- Leaves: every name with every level list, plus two forms outside the
    grammar. -/
def sampleLeaves : List Expr :=
  (sampleNames.flatMap fun n => sampleLevels.map fun ls => mkConst n ls)
    ++ [mkSort .zero, mkLambda `x BinderInfo.default leaf leaf]

/-- The leaves applications are built from, so the generated family stays a few
    hundred trees. -/
def buildingLeaves : List Expr :=
  [leaf, mkConst ``Nat [.zero], mkConst (.str .anonymous "A") [],
    mkConst (.num .anonymous 1) [.succ .zero]]

/-- Generated trees: the leaves, and applications of the building leaves up to
    two deep. -/
def sampleTrees : List Expr :=
  sampleLeaves
    ++ (buildingLeaves.flatMap fun f => buildingLeaves.map fun a => mkApp f a)
    ++ (buildingLeaves.flatMap fun f =>
      buildingLeaves.flatMap fun a => buildingLeaves.map fun b => mkApp f (mkApp a b))

/-- Every generated tree instance: each tree under every pair of bounds. -/
def treeCases : List (String × Bool) :=
  sampleTrees.flatMap fun e =>
    (List.range 5).flatMap fun depth =>
      (List.range 6).map fun budget =>
        (s!"the walk matches the oracle on {repr e} at depth {depth} and budget {budget}",
          treeHolds depth budget e)

/-- Run every check, then exit non-zero if any failed. -/
def main : IO UInt32 := do
  let fixed : List (String × Bool) :=
    [ ("a leaf spends one node", rendered maxDepth maxNodes leaf == some 1),
      ("a nine-node spine spends nine nodes",
        rendered maxDepth maxNodes (spine 4) == some 9),
      ("a nine-node spine is refused at budget eight",
        rendered maxDepth 8 (spine 4) == none),
      ("the depth bound admits depth 60",
        rendered 60 maxNodes (spine 59) == some 119),
      ("the depth bound refuses depth 61",
        rendered 60 maxNodes (spine 60) == none),
      ("a shared tree expands to its tree size",
        rendered maxDepth maxNodes (shared 4) == some 31),
      ("a shared tree is refused below its tree size",
        rendered maxDepth 30 (shared 4) == none),
      ("a shared tree is refused above the depth bound",
        rendered 4 maxNodes (shared 4) == none),
      ("a shared tree over the node bound is refused without expanding it",
        rendered maxDepth maxNodes (shared 40) == none),
      ("a zero universe level renders as zero",
        statementJson (mkConst ``Nat [.zero]) == constantJson [0]),
      ("successor levels render their depth",
        statementJson (mkConst ``Nat [.succ (.succ .zero)]) == constantJson [2]),
      ("a non-ground universe level is refused",
        statementJson (mkConst ``Nat [.param `u]) == Json.null),
      ("a text name component renders as a string",
        nm (.str (.str .anonymous "A") "1") == Json.arr #[Json.str "A", Json.str "1"]),
      ("a numeric name component renders as a number",
        nm (.num (.str .anonymous "A") 1) == Json.arr #[Json.str "A", toJson 1]),
      ("a short message is unchanged", truncateUtf8 8 "short" == "short"),
      ("a message at the byte limit is unchanged",
        truncateUtf8 5 "abcde" == "abcde"),
      ("a message over the limit keeps the prefix that fits",
        truncateUtf8 4 "abcde" == "abcd"),
      ("a multi-byte scalar crossing the limit is dropped whole",
        truncateUtf8 2 "aé" == "a"),
      ("a multi-byte scalar ending exactly at the limit is kept",
        truncateUtf8 3 "aéX" == "aé"),
      ("a three-byte scalar ending exactly at the limit is kept",
        truncateUtf8 3 "€" == "€"),
      ("a three-byte scalar crossing the limit is dropped whole",
        truncateUtf8 2 "€" == ""),
      ("a prefixed three-byte scalar ending exactly at the limit is kept",
        truncateUtf8 4 "a€" == "a€"),
      ("a prefixed three-byte scalar crossing the limit keeps the prefix",
        truncateUtf8 3 "a€" == "a"),
      ("a four-byte scalar ending exactly at the limit is kept",
        truncateUtf8 4 "😀" == "😀"),
      ("a four-byte scalar crossing the limit is dropped whole",
        truncateUtf8 3 "😀" == ""),
      ("a prefixed four-byte scalar ending exactly at the limit is kept",
        truncateUtf8 5 "a😀" == "a😀"),
      ("a prefixed four-byte scalar crossing the limit keeps the prefix",
        truncateUtf8 4 "a😀" == "a"),
      ("the contract's bound cuts an oversized message to 4,096 bytes",
        (truncateUtf8 maxMessageBytes oversized).utf8ByteSize == maxMessageBytes) ]
  let cases := fixed ++ truncationCases ++ treeCases
  let mut failures := 0
  for (label, passed) in cases do
    unless (← check label passed) do
      failures := failures + 1
  if failures == 0 then
    IO.println s!"renderer: {cases.length} checks passed"
    return 0
  IO.eprintln s!"renderer: {failures} of {cases.length} checks failed"
  return 1