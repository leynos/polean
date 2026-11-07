/-
A direct user axiom.

The root is a theorem, its statement is the goal, and the only axiom is
`AuditFixture.Undeclared.assume`, which the request does not approve. The
finding is `undeclared_axiom`.
-/
import AuditFixture.Goal

namespace AuditFixture.Undeclared

/-- An assumed proof the request does not declare. -/
axiom assume : AuditFixture.Goal

/-- The goal, resting on the undeclared assumption. -/
theorem root : AuditFixture.Goal := assume

end AuditFixture.Undeclared