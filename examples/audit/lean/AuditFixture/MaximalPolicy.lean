import AuditFixture.Claim

/-!
A claim over a maximal policy literal.

Sixteen rules, each covering all sixteen atoms, written out in full. The tree
is well inside the 4,096-node, depth-60 bound, but large enough that a renderer
which truncated the statement, dropped an argument, or lost a cons cell would
produce a different tree. Acceptance here is what pins the bound's headroom.
-/

namespace AuditFixture.MaximalPolicy

/-- The claim that a sixteen-rule policy is non-empty. -/
theorem root :
    AuditFixture.Claim.Holds
      [ { effect := .allow, subject := .a00
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      , { effect := .deny, subject := .a01
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      , { effect := .allow, subject := .a02
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      , { effect := .deny, subject := .a03
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      , { effect := .allow, subject := .a04
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      , { effect := .deny, subject := .a05
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      , { effect := .allow, subject := .a06
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      , { effect := .deny, subject := .a07
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      , { effect := .allow, subject := .a08
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      , { effect := .deny, subject := .a09
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      , { effect := .allow, subject := .a10
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      , { effect := .deny, subject := .a11
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      , { effect := .allow, subject := .a12
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      , { effect := .deny, subject := .a13
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      , { effect := .allow, subject := .a14
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      , { effect := .deny, subject := .a15
        , actions := [.a00, .a01, .a02, .a03, .a04, .a05, .a06, .a07, .a08, .a09, .a10, .a11, .a12, .a13, .a14, .a15] }
      ] := by
  unfold AuditFixture.Claim.Holds
  decide

end AuditFixture.MaximalPolicy
