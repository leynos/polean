import PoleanAudit.Render

/-!
# Renderer bound tests

Direct tests of `PoleanAudit.Render.treeBounded`, built and run by
`tests/toolchain/smoke/audit/render.rs` under `make toolchain-smoke`. The
worker's `main` cannot be imported into a test executable, so the renderer and
its bounds live in their own module and are driven here.

The shared-expression cases are the reason this file exists. The corpus cannot
show that the walk is bounded: every fixture's statement is a tree the kernel
already built, and a guard that summed nodes on the way back up would still
accept every one of them. Here the graph is built by sharing, so the tree it
expands to is exponential while the graph is tiny. The last case, `shared 40`,
expands to 2^41 - 1 nodes if the budget guard is bypassed; the Rust side runs
this executable under a deadline, so such a bypass fails as a timeout instead of
hanging the gate.
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

/-- Report one failed check and return whether it passed. -/
def check (label : String) (passed : Bool) : IO Bool := do
  if passed then
    return true
  IO.eprintln s!"FAILED: {label}"
  return false

/-- Run every check, then exit non-zero if any failed. -/
def main : IO UInt32 := do
  let cases : List (String × Bool) :=
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
        rendered maxDepth maxNodes (shared 40) == none) ]
  let mut failures := 0
  for (label, passed) in cases do
    unless (← check label passed) do
      failures := failures + 1
  if failures == 0 then
    IO.println s!"renderer: {cases.length} checks passed"
    return 0
  IO.eprintln s!"renderer: {failures} of {cases.length} checks failed"
  return 1