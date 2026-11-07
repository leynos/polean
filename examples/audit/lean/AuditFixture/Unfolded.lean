/-
A definitionally equal but structurally different statement.

The root's type writes the body of `AuditFixture.Goal` out instead of naming
it. The two propositions are definitionally equal, but a `forall` is outside
the ground grammar, so the worker reports `statement: null` and settlement
fails closed with `statement_mismatch`.
-/
import AuditFixture.Goal

namespace AuditFixture.Unfolded

/-- The goal's body, written out rather than named. -/
theorem root : ∀ n : Nat, n = n := fun _ => rfl

end AuditFixture.Unfolded