/-
  The Polean audit worker.

  The worker observes one compiled Lean module and the roots a request names,
  and prints one JSON document describing what it saw. It emits no verdict: the
  Rust side settles a report against the request, so a worker bug can make a
  report wrong but cannot make an audit pass.

  Input is positional:

      polean-audit <module> <root> [<root> ...]

  The search path comes from `LEAN_SYSROOT`, falling back to the compiled-in
  sysroot, plus whatever `LEAN_PATH` adds. A module is *toolchain* when its
  `.olean` resolved under the sysroot's `lib/lean`; only non-toolchain modules
  are replayed and only they appear in `replay.modules`.

  Everything that can go wrong before the report is printed exits non-zero with
  standard output empty. A replay rejection is a *completed* audit, not a
  worker failure, so it is printed as a report with `roots: []`.
-/
import Lean
import Lean.Replay

open Lean

/-- Whether `p` lies under `root`, by whole path components. -/
def under (root p : System.FilePath) : Bool :=
  let r := root.components
  (p.components.take r.length) == r

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
    visits however the graph is shared. -/
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

/-- The contract's spelling of a declaration kind. -/
def kindOf : ConstantInfo → String
  | .thmInfo _ => "theorem"
  | .axiomInfo _ => "axiom"
  | .defnInfo _ => "definition"
  | .opaqueInfo _ => "opaque"
  | .quotInfo _ => "quotient"
  | .ctorInfo _ => "constructor"
  | .recInfo _ => "recursor"
  | .inductInfo _ => "inductive"

/-- Split a dotted argument into a name. -/
def parseName (s : String) : Name :=
  (s.splitOn ".").foldl Name.mkStr .anonymous

/-- How many bytes `c` occupies when encoded as UTF-8. -/
def charBytes (c : Char) : Nat :=
  let n := c.toNat
  if n < 0x80 then 1 else if n < 0x800 then 2 else if n < 0x10000 then 3 else 4

/-- Truncate `s` to at most `maxBytes` UTF-8 bytes on a scalar boundary. -/
def truncateUtf8 (maxBytes : Nat) (s : String) : String :=
  if s.utf8ByteSize ≤ maxBytes then s else
  let rec go (cs : List Char) (used : Nat) (out : List Char) : List Char :=
    match cs with
    | [] => out.reverse
    | c :: rest =>
      let n := charBytes c
      if used + n > maxBytes then out.reverse else go rest (used + n) (c :: out)
  String.ofList (go s.toList 0 [])

/-- The largest diagnostic the contract admits. -/
def maxMessageBytes : Nat := 4096

/-- Print the report and exit 0. -/
def emit (doc : Json) : IO UInt32 := do
  IO.println (Json.compress doc)
  return 0

unsafe def main (args : List String) : IO UInt32 := do
  let modS :: root0 :: rootRest := args
    | throw (IO.userError "usage: polean-audit <module> <root>...")
  let rootS := root0 :: rootRest
  let sysroot ← findSysroot
  initSearchPath sysroot
  let lib := sysroot / "lib" / "lean"
  let m := parseName modS
  withImportModules #[{module := m}] {} fun env => do
    let names := env.header.moduleNames
    let mut toolchain : NameSet := {}
    for mn in names do
      if under lib (← findOLean mn) then
        toolchain := toolchain.insert mn
    let modOf (n : Name) : Option Name :=
      (env.getModuleIdxFor? n).bind fun i => names[i.toNat]?
    -- `Init` is always in the replay base, so the kernel refuses to redefine a
    -- toolchain name that it accelerates.
    let mut base : Array Import := #[{module := `Init}]
    for (mn, md) in names.zip env.header.moduleData do
      if !toolchain.contains mn then
        for i in md.imports do
          if toolchain.contains i.module && !base.any (·.module == i.module) then
            base := base.push {module := i.module}
    let mut new : Std.HashMap Name ConstantInfo := {}
    for (n, ci) in env.constants.map₁.toList do
      if let some o := modOf n then
        if !toolchain.contains o then
          new := new.insert n ci
    let modules := names.filter (!toolchain.contains ·) |>.map nm
    let header :=
      [("schema_version", Json.str "polean.audit-report.v0"),
       ("githash", Json.str Lean.githash),
       ("module", nm m)]
    withImportModules base {} fun baseEnv => do
      match (← (baseEnv.toKernelEnv.replay new).toBaseIO) with
      | .error e =>
        emit <| Json.mkObj <| header ++
          [("replay", Json.mkObj
             [("status", Json.str "rejected"),
              ("message", Json.str (truncateUtf8 maxMessageBytes (toString e)))]),
           ("roots", Json.arr #[])]
      | .ok kenv =>
        let mut roots := #[]
        for r in rootS do
          let rn := parseName r
          match kenv.find? rn with
          | none =>
            roots := roots.push
              (Json.mkObj [("name", nm rn), ("declaration", Json.null)])
          | some ci =>
            let (axs, _) ← (Lean.collectAxioms rn : CoreM (Array Name)).toIO
              {fileName := "", fileMap := default} {env := env}
            let sorted := axs.qsort Name.lt
            roots := roots.push <| Json.mkObj
              [("name", nm rn),
               ("declaration", Json.mkObj
                 [("kind", Json.str (kindOf ci)),
                  ("statement", statementJson ci.type),
                  ("axioms", Json.arr (sorted.map nm))])]
        emit <| Json.mkObj <| header ++
          [("replay", Json.mkObj
             [("status", Json.str "accepted"), ("modules", Json.arr modules)]),
           ("roots", Json.arr roots)]