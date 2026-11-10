import Lean
import Lean.Replay
import PoleanAudit.Render

/-!
  The Polean audit worker.

  The worker observes one compiled Lean module and the roots a request names,
  and prints one JSON document describing what it saw. It emits no verdict: the
  Rust side settles a report against the request, so the worker cannot choose
  the outcome. Settlement still trusts a report that matches the request, so a
  worker fault that omits an axiom can make an audit pass; acceptance means the
  report says the right thing, not that the report is true. Report truthfulness
  is the worker's to establish and lies outside the settlement proof, which is
  the operational-integrity boundary ADR 0006 records.

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

open Lean

/-- Whether `p` lies under `root`, by whole path components. -/
def under (root p : System.FilePath) : Bool :=
  let r := root.components
  (p.components.take r.length) == r

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

/-- Print the report and exit 0. -/
def emit (doc : Json) : IO UInt32 := do
  IO.println (Json.compress doc)
  return 0

/-- The worker's entry point.

    Marked `unsafe` because `Lean.withImportModules` is: it loads a module's
    compiled `.olean` files into the environment without re-checking them, which
    is precisely the replay this worker observes and reports. Nothing read
    through that boundary is trusted here; the report is data for the Rust
    settlement, which decides. Settlement guarantees that acceptance follows
    from the submitted report and request; it does not make a false report true.
    A worker fault that drops an axiom can therefore produce an accepted audit,
    which is why report truthfulness is a separate claim and why ADR 0006 keeps
    operational integrity outside this worker. Every other step is ordinary
    `IO`. -/
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
            -- Axioms are collected from the *imported* environment, while the
            -- declaration above is read from the replayed one. The two can in
            -- principle disagree, and collecting from the replay would be the
            -- stronger design; `Environment.ofKernelEnv` cannot supply it
            -- because it leaves `checked` empty, so every root would report no
            -- axioms. D22 records the trade-off and
            -- `docs/lean-environment-integrity-findings.md` the evidence; the
            -- substitution is promotion work.
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