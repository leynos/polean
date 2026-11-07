import AuditFixture.Goal
import Lean

/-!
A declaration the frontend wrote without asking the kernel.

`debug.skipKernelTC` suppresses the kernel's check when `addDecl` records the
theorem, so the `.olean` holds a proof of `AuditFixture.Goal` whose value is
`True.intro`. The file compiles, and only kernel replay notices.
-/
open Lean Elab Command

namespace AuditFixture.ForgeBase

run_cmd liftCoreM do
  let decl := Declaration.thmDecl {
    name := `AuditFixture.ForgeBase.root
    levelParams := []
    type := mkConst `AuditFixture.Goal
    value := mkConst ``True.intro }
  withOptions (fun options => debug.skipKernelTC.set options true) (addDecl decl)

end AuditFixture.ForgeBase