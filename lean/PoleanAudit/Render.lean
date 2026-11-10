import Lean

/-!
# Bounded rendering and diagnostic truncation

The audit worker turns a declaration's type into the ground tree the contract
admits, or into JSON `null` when the type is out of grammar or over the bound.
It also truncates a replay diagnostic to the contract's message bound. Both
bounds live here, beside the code that applies them, so they can be tested
without running the worker: `PoleanAuditRenderTests.lean` drives `treeBounded`
and `truncateUtf8` directly. The worker's `main` cannot be imported into a test
executable, so anything a test must call belongs in this module.

The node bound is spent on the way down rather than summed on the way back up.
An `Expr` is a directed acyclic graph, so a shared sub-expression is visited
once per path to it, and a small, deeply shared term would otherwise expand to
an exponential tree before any total was compared with [`maxNodes`]. With a
budget the walk stops as soon as the whole tree would exceed the bound, however
the graph is shared.
-/

open Lean

/-- A name as the contract spells it: one JSON string or number per component. -/
partial def nameJson : Name → List Json
  | .anonymous => []
  | .str p s => nameJson p ++ [Json.str s]
  | .num p n => nameJson p ++ [toJson n]

/-- Serialise a name as an array of components. -/
def nm (n : Name) : Json :=
  Json.arr (nameJson n).toArray

/-- A universe level as a natural number, or `none` when it is not ground. -/
partial def levelNat : Level → Option Nat
  | .zero => some 0
  | .succ l => (· + 1) <$> levelNat l
  | _ => none

/-- The largest ground statement tree the contract admits. -/
def maxNodes : Nat := 4096

/-- The deepest ground statement tree the contract admits. -/
def maxDepth : Nat := 60

/-- Render a ground statement tree, refusing anything out of grammar or over the
    bound.

    `budget` is how many nodes this subtree may still spend, so the node count
    is bounded on the way down rather than summed on the way back up. That
    matters because an `Expr` is a directed acyclic graph: a shared
    sub-expression is visited once per path to it, and a small, deeply shared
    term could otherwise expand to an exponential tree before any total was
    compared with [`maxNodes`]. With the budget the traversal stops as soon as
    the whole tree would exceed the bound, so it costs at most `maxNodes` node
    visits however the graph is shared.

    Both subtractions are `Nat` subtraction, which truncates at zero rather than
    failing. That is safe only because of the guard: `depth` and `budget` are
    positive whenever a child is visited, and if a returned count ever exceeded
    the remaining budget, `budget - 1 - nf` would truncate to zero and the next
    call would refuse. Truncation can therefore only make the walk reject a
    tree, never admit one. -/
partial def treeBounded (depth budget : Nat) (e : Expr) : Option (Json × Nat) :=
  if depth == 0 || budget == 0 then
    none
  else
    match e with
    | .const n ls => do
      let ns ← ls.mapM levelNat
      return (Json.mkObj [("const", nm n), ("levels", toJson ns)], 1)
    | .app f a => do
      let (jf, nf) ← treeBounded (depth - 1) (budget - 1) f
      let (ja, na) ← treeBounded (depth - 1) (budget - 1 - nf) a
      return (Json.mkObj [("app", Json.arr #[jf, ja])], nf + na + 1)
    | _ => none

/-- The statement tree for a declaration's type, or JSON `null`. -/
def statementJson (e : Expr) : Json :=
  match treeBounded maxDepth maxNodes e with
  | some (j, _) => j
  | none => Json.null

/-- The largest diagnostic the contract admits. -/
def maxMessageBytes : Nat := 4096

/-- How many bytes `c` occupies when encoded as UTF-8. -/
def charBytes (c : Char) : Nat :=
  let n := c.toNat
  if n < 0x80 then 1 else if n < 0x800 then 2 else if n < 0x10000 then 3 else 4

/-- Truncate `s` to at most `maxBytes` UTF-8 bytes on a scalar boundary.

    A character that would cross the limit is dropped whole rather than split,
    so the result is always valid UTF-8. The walk stops at the first character
    that does not fit, so the result is a prefix of `s` rather than a selection
    of its characters. -/
def truncateUtf8 (maxBytes : Nat) (s : String) : String :=
  if s.utf8ByteSize ≤ maxBytes then s else
  let rec go (cs : List Char) (used : Nat) (out : List Char) : List Char :=
    match cs with
    | [] => out.reverse
    | c :: rest =>
      let n := charBytes c
      if used + n > maxBytes then out.reverse else go rest (used + n) (c :: out)
  String.ofList (go s.toList 0 [])