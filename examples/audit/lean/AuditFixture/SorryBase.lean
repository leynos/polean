import AuditFixture.Goal

/-!
A `sorry` behind an imported helper.

`SorryImported` proves the goal from this declaration, so the axiom is reached
through another module in the import closure. This exercises axiom collection
across modules rather than within one.
-/

namespace AuditFixture.SorryBase

/-- The goal, closed by `sorry` in a helper module. -/
theorem hidden : AuditFixture.Goal := by sorry

end AuditFixture.SorryBase