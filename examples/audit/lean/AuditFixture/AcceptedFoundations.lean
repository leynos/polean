import AuditFixture.Goal

/-!
A theorem that uses all three approved foundations.

`propext`, `Quot.sound`, and `Classical.choice` are the pinned foundations the
request may approve. The root's axiom set must be exactly these three, and
settlement accepts them because the request lists them.
-/

namespace AuditFixture.AcceptedFoundations

/-- The goal, resting on all three approved foundations. -/
theorem root : AuditFixture.Goal := by
  intro n
  have _propext : True = True :=
    propext (Iff.intro (fun h => h) (fun h => h))
  have _quot : Quot.mk (fun (_ _ : Nat) => True) 0 =
      Quot.mk (fun (_ _ : Nat) => True) 1 :=
    Quot.sound trivial
  have _choice : Nat := Classical.choice ⟨n⟩
  exact rfl

end AuditFixture.AcceptedFoundations