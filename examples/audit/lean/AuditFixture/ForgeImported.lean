import AuditFixture.ForgeBase
import AuditFixture.Goal

/-!
The same forgery, in an imported module.

The audited module's own declarations are honest. Replay still rejects, because
it covers every non-toolchain module in the import closure, not just the job.
This is the fixture that separates replaying the closure from replaying the job.
-/

namespace AuditFixture.ForgeImported

/-- An honest root whose import holds a forgery. -/
theorem root : AuditFixture.Goal := fun _ => rfl

end AuditFixture.ForgeImported