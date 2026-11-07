/-!
The claim vocabulary the structural-literal fixtures share.

A real Polean claim is a named predicate applied to a policy literal, so the
root's type is a ground `const`/`app` tree even though the predicate's body
quantifies. The vocabulary here is deliberately small and constructor-only:
every leaf is a nullary constant, so the tree is predictable without relying on
how Lean elaborates numeric or string literals.

`Holds` takes the policy as a list of rules. One rule names an effect, a
subject atom, and the actions it covers. `Atom` has sixteen constructors, which
is enough for the maximal fixture to exercise the tree bound.
-/
namespace AuditFixture.Claim

/-- Whether a rule permits or denies an action. -/
inductive Effect where
  /-- The rule permits. -/
  | allow
  /-- The rule denies. -/
  | deny

/-- One distinct policy subject or action. -/
inductive Atom where
  /-- The first atom. -/
  | a00
  /-- The second atom. -/
  | a01
  /-- The third atom. -/
  | a02
  /-- The fourth atom. -/
  | a03
  /-- The fifth atom. -/
  | a04
  /-- The sixth atom. -/
  | a05
  /-- The seventh atom. -/
  | a06
  /-- The eighth atom. -/
  | a07
  /-- The ninth atom. -/
  | a08
  /-- The tenth atom. -/
  | a09
  /-- The eleventh atom. -/
  | a10
  /-- The twelfth atom. -/
  | a11
  /-- The thirteenth atom. -/
  | a12
  /-- The fourteenth atom. -/
  | a13
  /-- The fifteenth atom. -/
  | a14
  /-- The sixteenth atom. -/
  | a15

/-- One rule of a policy. -/
structure Rule where
  /-- Whether the rule permits or denies. -/
  effect : Effect
  /-- The subject the rule is about. -/
  subject : Atom
  /-- The actions the rule covers. -/
  actions : List Atom

/-- The claim predicate: a policy that is not empty. -/
def Holds (rules : List Rule) : Prop := rules.length > 0

end AuditFixture.Claim