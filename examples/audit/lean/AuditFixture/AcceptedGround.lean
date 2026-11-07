/-
An honest theorem with no axiom. The settlement must accept it.

This is the smallest accepted fixture: the root's type is the constant
`AuditFixture.Goal`, and its proof term mentions no axiom.
-/
import AuditFixture.Goal

namespace AuditFixture.AcceptedGround

/-- The goal, proved by reflexivity. -/
theorem root : AuditFixture.Goal := fun _ => rfl

end AuditFixture.AcceptedGround