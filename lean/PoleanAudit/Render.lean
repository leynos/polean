import Lean

/-!
# Bounded statement rendering

The audit worker turns a declaration's type into the ground tree the contract
admits, or into JSON `null` when the type is out of grammar or over the bound.
The bounds live here, beside the walk that enforces them, so they can be tested
without running the worker: `PoleanAuditRenderTests.lean` drives
`treeBounded` directly.

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