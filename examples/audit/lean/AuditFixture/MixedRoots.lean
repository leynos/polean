import AuditFixture.Goal

/-!
One clean root and one resting on `sorry`.

The request asks for both roots in source order. Settlement must keep per-root
evidence: only root 1 has `sorry_axiom`, and root 0 stays clean. A settlement
that collapsed a report to one verdict would lose that distinction.
-/

namespace AuditFixture.MixedRoots

/-- The first root, which is clean. -/
theorem clean : AuditFixture.Goal := fun _ => rfl

/-- The second root, which uses `sorry`. -/
theorem root : AuditFixture.Goal := by sorry

end AuditFixture.MixedRoots