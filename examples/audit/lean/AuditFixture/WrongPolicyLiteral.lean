import AuditFixture.Claim

/-!
The same claim shape over a different policy literal.

Both the request and the root use `Claim.Holds`, but the literals differ: the
request expects two rules, the root states one. Only the statement comparison
can tell them apart, so this isolates structural comparison from the head
constant.
-/

namespace AuditFixture.WrongPolicyLiteral

/-- The claim that a one-rule policy is non-empty. -/
theorem root :
    AuditFixture.Claim.Holds
      [ { effect := .deny, subject := .a15, actions := [.a15] } ] := by
  unfold AuditFixture.Claim.Holds
  decide

end AuditFixture.WrongPolicyLiteral