/-
A declaration the frontend wrote without asking the kernel.

The forged theorem is in the audited module itself. Replay reads every retained
constant in every non-toolchain module and rejects the job, so the settlement
is `kernel_rejected` whatever the declaration claims.
-/
import AuditFixture.Goal
import Lean
open Lean Elab Command

namespace AuditFixture.ForgeInit

run_cmd liftCoreM do
  let decl := Declaration.thmDecl {
    name := `AuditFixture.ForgeInit.root
    levelParams := []
    type := mkConst `AuditFixture.Goal
    value := mkConst ``True.intro }
  withOptions (fun options => debug.skipKernelTC.set options true) (addDecl decl)

end AuditFixture.ForgeInit