/-
The proposition every accepted audit fixture's root proves.

The root's type must render as the ground tree `AuditFixture.Goal`, so the
definition is never unfolded: the audit compares the *statement as written*,
not its body. A fixture that writes the body out instead is `Unfolded`, and is
rejected because a `forall` is outside the ground grammar.
-/
namespace AuditFixture

/-- The proposition every accepted fixture's root proves. -/
def Goal : Prop := ∀ n : Nat, n = n

end AuditFixture