/-
A direct `sorry`.

`sorry` closes the goal with `sorryAx`, which `Lean.collectAxioms` reports.
The finding is `sorry_axiom`, and no request can approve `sorryAx`.
-/
import AuditFixture.Goal

namespace AuditFixture.Sorry

/-- The goal, closed by `sorry`. -/
theorem root : AuditFixture.Goal := by sorry

end AuditFixture.Sorry