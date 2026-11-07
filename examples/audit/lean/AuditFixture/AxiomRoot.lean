import AuditFixture.Goal

/-!
The requested root is itself an axiom.

`Lean.collectAxioms` includes the root, so settlement reports both the kind
finding and an undeclared axiom. The name is not one of the pinned foundations,
so settlement cannot accept it however the statement compares.
-/

namespace AuditFixture.AxiomRoot

/-- An assumed proof of the goal. -/
axiom root : AuditFixture.Goal

end AuditFixture.AxiomRoot