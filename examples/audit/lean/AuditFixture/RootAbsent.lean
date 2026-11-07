/-
The module exists but defines no `root`.

The worker reports `declaration: null` and settlement reports `root_absent`.
An absent root is a completed audit, not a tool failure: the module compiled
and replayed, and the answer is that there is no such declaration.
-/
namespace AuditFixture.RootAbsent

/-- A declaration, so the module is not empty. -/
def present : Nat := 0

end AuditFixture.RootAbsent