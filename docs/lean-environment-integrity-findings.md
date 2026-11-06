# Lean environment integrity findings

Status: research record for roadmap 4.3.2. Nothing here is implemented. Date:
2026-11-06.

## Why this record exists

While planning roadmap 1.2.3, an adversarial review attacked the proposed Lean
audit on the pinned Lean 4.35.0. It found ways to obtain a clean audit result
for something unsound.
[ADR 0006](adrs/adr-0006-separate-proof-integrity-from-operational-integrity.md)
moves defence against them to 4.3.2, because they need Lean that Polean's
generated-module input model (technical design §2.1) excludes. This record
keeps the evidence, so that hardening does not start from nothing.

## Status of the evidence

- The results are author-reported. They come from one-off scratch runs on
  2026-11-05 and 2026-11-06, using a digest-named install of the pinned archive
  in a sibling worktree, with cleared environments. No gate reproduces them.
- The scratch directories were not retained. The fixtures and prototype below
  are the exact sources of the 2026-11-06 re-run.
- The panel's fixtures were recovered from its transcript and re-run. One
  result did not reproduce, and `Results` marks it.
- All fixtures compile from source. Crafted binary `.olean` files are out of
  scope here, as upstream warns they can attack any tool that loads them.

This record establishes that these attacks exist on the pinned tuple, and that
a small prototype rejected those shown. It does not establish that the list is
complete, or that the prototype is a sufficient defence.

## Results

"Proof-integrity worker" is the shape proposed for 1.2.3: replay of
non-toolchain modules, with the toolchain's imports as the base and `LEAN_PATH`
as given. "Hardened prototype" is the listing in `Prototype`. A clean result
here means replay accepted the module, the root is a theorem, and the reported
axioms are empty or foundation names only.

| Fixture           | What it needs                    | Proof-integrity worker                                           | Hardened prototype                        | Regression use  |
| ----------------- | -------------------------------- | ---------------------------------------------------------------- | ----------------------------------------- | --------------- |
| `KernelNameForge` | `prelude` in a job               | Clean: a proof of `False`, no axioms                             | Rejected: `Nat.gcd` already declared      | 4.3.2           |
| `PreludeForge`    | `prelude` in a job               | Clean: its own `propext`, reported by that name                  | Rejected: `propext` already declared      | 4.3.2           |
| `StatementForge`  | `prelude` in a job               | Clean: statement matches by name                                 | Reports a job module behind the statement | 4.3.2           |
| `TrustedSpoof`    | Write access to the search path  | Clean: `Goal` means `True`                                       | Rejected: declaration type mismatch       | 4.3.2           |
| `InitShadow`      | Write access to the search path  | Clean: `Quot.sound` from a forged `Init.Prelude`                 | Rejected: unknown constant `Absurd`       | 4.3.2           |
| `SpoofedName`     | User axiom in a job              | Reports the one component `Quot.sound`, so settlement rejects it | Same                                      | 1.2.3 unit test |
| `ForgeInit`       | `debug.skipKernelTC` in a job    | Rejected by replay                                               | Rejected by replay                        | 1.2.3 corpus    |
| `ForgeImported`   | The same, in an import           | Rejected by replay                                               | Rejected by replay                        | 1.2.3 corpus    |
| `RecForge`        | Environment hack in a job        | Rejected: invalid recursor                                       | Rejected: invalid recursor                | 4.3.2           |
| `DupRootA`        | Duplicate theorem, forged loser  | Clean; forgery never replayed                                    | Same                                      | 4.3.2           |
| `DupRootB`        | Duplicate theorem, forged winner | Rejected by replay                                               | Rejected by replay                        | 4.3.2           |
| `DupRootC`        | Duplicate theorem, axiom loser   | Clean; axiom merged away                                         | Same                                      | 4.3.2           |
| `Bomb`            | `debug.skipKernelTC` in a job    | Accepted in 8.3 s; see below                                     | Not re-run                                | 4.3.1 candidate |

_Table 1: Attacks, and what each worker reported on 2026-11-06._

The `DupRoot` cases are sound as observed. The audit reads the replayed winner,
which is a real proof, or a real absence of an axiom. They matter because
replay does not inspect every copy in the import closure. Replay also skips
unsafe and partial constants, by design.

`Bomb` did not reproduce. The panel's run was killed at 25 s, while the same
source completed replay in 8.3 s on re-run, so this record does not establish
that replay is unbounded. `Lean/Replay.lean` calls `addDeclCore 0 0`, which
sets no limit, and 4.3.1's injected hangs remain the right test.

Other interfaces, measured on 2026-11-05:

```plaintext
leanchecker KernelNameForge                rc=0
leanchecker --fresh KernelNameForge        rc=0
leanchecker on a forgery in an import      rc=0, because it replays the named module only
leanchecker --fresh, Init-only module      91.0 s, 476 MiB
leanexport, Init-only module               36.3 s, 872 MiB, 345,696,313-byte NDJSON
leanchecker --from-export                  87.0 s, 516 MiB
lake comparator                            rc=2 without bwrap
lake comparator --inadvisably-no-sandbox   rc=1, text: Illegal axiom detected: 'cheat'
```

The panel also reports that `lake comparator` exits 134 on `KernelNameForge`,
an exporter crash rather than a principled rejection, with the cause
unconfirmed.

## Observations for 4.3.2

- **Privileged names.** The kernel accelerates certain constants by name, and a
  module importing only `Init.Prelude` may define the ones it omits.
  `Lake/CLI/Check.lean` in the pinned archive lists them. Replaying on the
  toolchain's `Init` makes the kernel refuse every redefinition, with no
  blocklist.
- **Search path.** Lean resolves by package root: the first root containing a
  module's top-level directory captures that whole prefix. Placing the sysroot
  first prevents `Init.*` shadowing, and a trusted root placed next gives
  trusted modules a prefix that a job cannot enter.
- **Module class.** Classify by the root a module resolved from, not by its
  name. `InitShadow` forges a toolchain name.
- **Axiom collection.** `Lean.collectAxioms` reads pre-computed lists for
  imported declarations from an environment extension (`exportedAxiomsExt`) and
  ignores a constant it cannot find. It agreed with an independent walk on
  every fixture that was compared. An independent walk over the replayed
  environment would remove the dependence, and fails closed on a missing
  constant.
- **Trusted package.** The prototype trusts its trusted root's contents, so a
  tampered file there is undetected. That needs the identity binding in 4.1.1.
- **Toolchain.** Reuse checks only the `COMPLETE` marker from
  `scripts/provision_tools.py`, not the unpacked files. Replaying them costs 91
  s against a proposed 60 s budget.
- **No execution.** Importing with `loadExts := false` did not run `initialize`
  blocks, while `lean --run` on the same module did. This one stays in the
  1.2.3 corpus.

## Prototype

This is the 2026-11-06 hardened prototype. Removing the four marked hardenings
gives the proof-integrity shape, apart from taking `LEAN_PATH` from the
environment. It reports only the statement's direct constants; 4.3.2 needs the
closure, without duplicates. It is evidence, not a starting implementation.

```lean
import Lean
import Lean.Replay
open Lean

def under (root p : System.FilePath) : Bool :=
  let r := root.components; (p.components.take r.length) == r

partial def nameJson : Name → List Json
  | .anonymous => []
  | .str p s => nameJson p ++ [Json.str s]
  | .num p n => nameJson p ++ [toJson n]

def nm (n : Name) : Json := Json.arr (nameJson n).toArray

partial def levelNat : Level → Option Nat
  | .zero => some 0
  | .succ l => (· + 1) <$> levelNat l
  | _ => none

partial def tree : Expr → Option Json
  | .const n ls => do
      let ns ← ls.mapM levelNat
      return Json.mkObj [("const", nm n), ("levels", toJson ns)]
  | .app f a => do return Json.mkObj [("app", Json.arr #[← tree f, ← tree a])]
  | _ => none

def kindOf : ConstantInfo → String
  | .thmInfo _ => "theorem" | .axiomInfo _ => "axiom" | .defnInfo _ => "definition"
  | .opaqueInfo _ => "opaque" | .quotInfo _ => "quotient" | .ctorInfo _ => "constructor"
  | .recInfo _ => "recursor" | .inductInfo _ => "inductive"

def parseName (s : String) : Name :=
  (s.splitOn ".").foldl Name.mkStr .anonymous

unsafe def main (args : List String) : IO UInt32 := do
  let trusted :: job :: modS :: rootS := args
    | throw (IO.userError "usage: <trusted> <job> <module> <root>...")
  let sysroot ← findSysroot
  let lib := sysroot / "lib" / "lean"
  -- Hardening 1: fix the search order. LEAN_PATH is never consulted, and the
  -- first root holding a top-level directory captures that whole prefix.
  searchPathRef.set [lib, trusted, job]
  let m := parseName modS
  withImportModules #[{module := m}] {} fun env => do
    let names := env.header.moduleNames
    let mut toolchain : NameSet := {}
    for mn in names do
      if under lib (← findOLean mn) then toolchain := toolchain.insert mn
    let modOf (n : Name) := (env.getModuleIdxFor? n).bind fun i => names[i.toNat]?
    -- Hardening 2: `Init` is always in the replay base, so the kernel refuses to
    -- redefine any toolchain name, including the ones it accelerates.
    let mut base : Array Import := #[{module := `Init}]
    for (mn, md) in names.zip env.header.moduleData do
      if !toolchain.contains mn then
        for i in md.imports do
          if toolchain.contains i.module && !base.any (·.module == i.module) then
            base := base.push {module := i.module}
    let mut new : Std.HashMap Name ConstantInfo := {}
    for (n, ci) in env.constants.map₁.toList do
      if let some o := modOf n then
        if !toolchain.contains o then new := new.insert n ci
    -- Hardening 3: class a module by the root it resolved from, not by its name.
    let mut trustedMods : NameSet := {}
    for mn in names do
      if under trusted (← findOLean mn) then trustedMods := trustedMods.insert mn
    let modules := names.filter (!toolchain.contains ·) |>.map fun mn =>
      Json.mkObj [("name", nm mn), ("trusted", toJson (trustedMods.contains mn))]
    let header := [("schema_version", Json.str "polean.audit-report.v0"),
      ("githash", Json.str Lean.githash), ("module", nm m)]
    withImportModules base {} fun baseEnv => do
      match (← (baseEnv.toKernelEnv.replay new).toBaseIO) with
      | .error e =>
        IO.println <| Json.compress <| Json.mkObj <| header ++
          [("replay", Json.mkObj [("status", "rejected"), ("message", Json.str ((toString e).take 200).toString)]),
           ("roots", Json.arr #[])]
        return 0
      | .ok kenv =>
        let mut roots := #[]
        for r in rootS do
          let rn := parseName r
          match kenv.find? rn with
          | none => roots := roots.push (Json.mkObj [("name", nm rn), ("declaration", Json.null)])
          | some ci =>
            let (axs, _) ← (Lean.collectAxioms rn : CoreM (Array Name)).toIO
              {fileName := "", fileMap := default} {env := env}
            let sorted := axs.qsort Name.lt
            -- Hardening 4: report the non-toolchain modules the statement rests on.
            let statementModules := ci.type.getUsedConstants.filterMap modOf
              |>.filter (!toolchain.contains ·) |>.map nm
            roots := roots.push <| Json.mkObj [("name", nm rn), ("declaration", Json.mkObj [
              ("kind", Json.str (kindOf ci)),
              ("statement", (tree ci.type).getD Json.null),
              ("statement_modules", Json.arr statementModules),
              ("axioms", Json.arr (sorted.map nm))])]
        IO.println <| Json.compress <| Json.mkObj <| header ++
          [("replay", Json.mkObj [("status", "accepted"), ("modules", Json.arr modules)]),
           ("roots", Json.arr roots)]
        return 0
```

## Regression candidates

### Privileged-name forgery

```lean
prelude
import Init.Prelude
def Nat.gcd (_ _ : Nat) : Nat := 0
theorem Fx.KernelNameForge.unfolded (a b : Nat) : Eq (Nat.gcd a b) 0 := rfl
theorem Fx.KernelNameForge.accel : Eq (Nat.gcd 6 4) 2 := rfl
theorem Fx.KernelNameForge.two_eq_zero : Eq (2 : Nat) 0 :=
  Eq.trans (Eq.symm Fx.KernelNameForge.accel) (Fx.KernelNameForge.unfolded 6 4)
theorem Fx.KernelNameForge.root : False :=
  @Eq.rec Nat 2
    (fun n _ => Nat.rec (motive := fun _ => Prop) False (fun _ _ => True) n)
    True.intro 0 Fx.KernelNameForge.two_eq_zero
```

### Forged foundation

```lean
prelude
inductive Fx.Absurd : Prop
def Fx.PreludeForge.Goal : Prop := Fx.Absurd
axiom propext : Fx.PreludeForge.Goal
theorem Fx.PreludeForge.root : Fx.PreludeForge.Goal := propext
```

### Forged statement

Audit with the expected statement `Fx.Goal`.

```lean
prelude
inductive Fx.Truth : Prop where | intro
def Fx.Goal : Prop := Fx.Truth
theorem Fx.StatementForge.root : Fx.Goal := Fx.Truth.intro
```

### Shadowed trusted module

Compile both against each other, with the real `Fx.Expected` in the trusted
root. The job's `Fx/Expected.olean` must not be loaded.

```lean
def Fx.Goal : Prop := True
```

```lean
import Fx.Expected
theorem Fx.TrustedSpoof.root : Fx.Goal := trivial
```

### Shadowed toolchain module

```lean
prelude
inductive Absurd : Prop
axiom Quot.sound : Absurd
```

```lean
prelude
import Init.Prelude
theorem Fx.InitShadow.root : Absurd := Quot.sound
```

### Duplicate declarations

`DupRootA` imports the forged and the honest copy, in that order. `DupRootB`
reverses the order, and `DupRootC` imports an axiom copy first. Compile each
copy separately.

```lean
import Fx.Expected
import Lean
open Lean Elab Command
run_cmd liftCoreM do
  let decl := Declaration.thmDecl {
    name := `Fx.Dup.t, levelParams := [], type := mkConst `Fx.Goal, value := mkConst ``True.intro }
  withOptions (fun o => debug.skipKernelTC.set o true) (addDecl decl)
```

```lean
import Fx.Expected
theorem Fx.Dup.t : Fx.Goal := by intro a b; cases a <;> cases b <;> rfl
```

```lean
import Fx.Expected
axiom Fx.Dup.t : Fx.Goal
```

### Forged recursor

```lean
import Fx.Expected
import Lean
open Lean Elab Command
inductive Fx.RecForge.I : Prop | mk
unsafe def go : CoreM Unit := do
  let env ← getEnv
  let n := mkPrivateNameCore `Lean.Environment `Lean.Environment.lakeAdd
  let f ← ofExcept (env.evalConst (Environment → ConstantInfo → Environment) {} n)
  let some (.recInfo r) := env.find? `Fx.RecForge.I.rec | throwError "no rec"
  setEnv (f env (.recInfo { r with k := !r.k }))
  let e' ← getEnv
  logInfo m!"forged: {(e'.find? `Fx.RecForge.I.rec).map (fun | .recInfo r => r.k | _ => false)} was {r.k}"
#eval go
theorem Fx.RecForge.root : Fx.Goal := by intro a b; cases a <;> cases b <;> rfl
```

### Kernel work

```lean
import Fx.Expected
import Lean
open Lean Elab Command
def Fx.Bomb.ack : Nat → Nat → Nat
  | 0, n => n + 1
  | m+1, 0 => Fx.Bomb.ack m 1
  | m+1, n+1 => Fx.Bomb.ack m (Fx.Bomb.ack (m+1) n)
termination_by m n => (m, n)
def Fx.Bomb.spin : Nat → Bool
  | 0 => true
  | n+1 => Fx.Bomb.spin n && Fx.Bomb.spin n
run_cmd liftCoreM do
  let lit := mkNatLit 200
  let lhs := mkApp (mkConst `Fx.Bomb.spin) lit
  let ty := mkApp3 (mkConst ``Eq [1]) (mkConst ``Bool) lhs (mkConst ``Bool.true)
  let decl := Declaration.thmDecl {
    name := `Fx.Bomb.big, levelParams := [], type := ty,
    value := mkApp2 (mkConst ``Eq.refl [1]) (mkConst ``Bool) (mkConst ``Bool.true) }
  withOptions (fun o => debug.skipKernelTC.set o true) (addDecl decl)
theorem Fx.Bomb.root : Fx.Goal := by intro a b; cases a <;> cases b <;> rfl
```
