/-
A different proposition. The root proves `True`, not `AuditFixture.Goal`.

The declaration is the right kind and has no axioms, so `statement_mismatch`
is the only finding, which isolates the statement comparison.
-/
namespace AuditFixture.WrongStatement

/-- A true proposition that is not the requested one. -/
theorem root : True := True.intro

end AuditFixture.WrongStatement