/-
A native-computation axiom reached through a helper.

`native_decide` trusts compiled evaluation and records an axiom whose name
contains a `_native` component. The request cannot approve it, so settlement
reports `native_axiom` rather than the generic `undeclared_axiom`, which names
the trust step the user must see.
-/
import AuditFixture.Goal

namespace AuditFixture.Native

/-- A fact closed by compiled evaluation. -/
theorem helper : (2 : Nat) + 2 = 4 := by native_decide

/-- The goal, resting on the native helper. -/
theorem root : AuditFixture.Goal :=
  let _fact : (2 : Nat) + 2 = 4 := helper
  fun _ => rfl

end AuditFixture.Native