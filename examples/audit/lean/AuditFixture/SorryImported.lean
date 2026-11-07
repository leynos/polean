/-
The goal proved from an imported `sorry`.

The root's own proof term is an application of `SorryBase.hidden`, so the
finding is still `sorry_axiom` on root 0. A worker that inspected only the
root's module would miss it.
-/
import AuditFixture.SorryBase

namespace AuditFixture.SorryImported

/-- The goal, resting on an imported `sorry`. -/
theorem root : AuditFixture.Goal := AuditFixture.SorryBase.hidden

end AuditFixture.SorryImported