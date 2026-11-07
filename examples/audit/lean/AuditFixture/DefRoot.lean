/-
The requested root is a definition, not a theorem.

The statement matches and no axiom is reachable, so `wrong_kind` is the only
finding. A definition is checked by the kernel but is not a proof, and the
audit accepts only theorem roots.
-/
import AuditFixture.Goal

namespace AuditFixture.DefRoot

-- The definition is the point: settlement must report `wrong_kind`, so the
-- linter's advice to write a theorem instead is deliberately not taken.
set_option linter.defProp false

/-- The goal as a definition rather than a theorem. -/
def root : AuditFixture.Goal := fun _ => rfl

end AuditFixture.DefRoot