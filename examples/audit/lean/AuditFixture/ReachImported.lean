import AuditFixture.ReachBase

/-!
The goal resting on an axiom reached through an imported definition.

The root applies a helper that applies the axiom, so axiom collection must walk
the transitive value, not just the root's own constants. The finding is
`undeclared_axiom`.
-/

namespace AuditFixture.ReachImported

/-- A helper between the root and the axiom. -/
theorem helper : AuditFixture.Goal := AuditFixture.ReachBase.hidden

/-- The goal, resting on an imported axiom. -/
theorem root : AuditFixture.Goal := helper

end AuditFixture.ReachImported