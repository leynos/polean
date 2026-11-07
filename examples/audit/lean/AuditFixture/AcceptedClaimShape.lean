/-
A named claim over a structural policy literal.

The root's type is `Claim.Holds` applied to a two-rule policy written out in
full. It is a ground `const`/`app` tree: no binder, literal, or variable form.
The settlement accepts it because the request carries exactly this tree.
-/
import AuditFixture.Claim

namespace AuditFixture.AcceptedClaimShape

/-- The claim that a two-rule policy is non-empty. -/
theorem root :
    AuditFixture.Claim.Holds
      [ { effect := .allow, subject := .a00, actions := [.a00, .a01] }
      , { effect := .deny, subject := .a01, actions := [.a02] } ] := by
  unfold AuditFixture.Claim.Holds
  decide

end AuditFixture.AcceptedClaimShape