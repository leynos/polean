/-
An axiom behind an imported definition.

`ReachImported` reaches it through a helper theorem, so the axiom is two
declarations away from the root. The name is outside the pinned table.
-/
import AuditFixture.Goal

namespace AuditFixture.ReachBase

/-- An assumed proof the request does not declare. -/
axiom hidden : AuditFixture.Goal

end AuditFixture.ReachBase