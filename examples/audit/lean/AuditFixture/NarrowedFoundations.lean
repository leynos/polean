import AuditFixture.Goal

/-!
The goal resting on a foundation the request does not list.

The root uses `propext`, and the corpus request approves only `Quot.sound` and
`Classical.choice`. Narrowing the request is the point: a request may approve
fewer foundations than the job uses, and settlement then rejects the surplus
as an undeclared axiom.
-/

namespace AuditFixture.NarrowedFoundations

/-- The goal, resting on `propext`. -/
theorem root : AuditFixture.Goal := by
  intro n
  have _propext : True = True :=
    propext (Iff.intro (fun h => h) (fun h => h))
  exact rfl

end AuditFixture.NarrowedFoundations