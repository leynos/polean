/-!
# Toolchain smoke module

A fixed, repository-authored check that the pinned Lean toolchain elaborates
and kernel-reduces a small finite Boolean enumeration with `decide`. It states
nothing about Polean policies.

`#guard_msgs` pins the axiom report for each theorem, so a change in what the
kernel records fails the build. This is a smoke check, not the axiom audit that
1.2.3 owns: it cannot detect axioms in general, `native_decide`, or in-file
option overrides.
-/
namespace PoleanSmoke

/-- Every pair of Booleans, listed explicitly. -/
def boolPairs : List (Bool × Bool) :=
  [(false, false), (false, true), (true, false), (true, true)]

/-- The enumeration covers every pair. -/
theorem boolPairs_complete : ∀ a b : Bool, (a, b) ∈ boolPairs := by
  decide

/-- info: 'PoleanSmoke.boolPairs_complete' depends on axioms: [propext] -/
#guard_msgs in
#print axioms boolPairs_complete

/-- A finite check over the enumeration, settled by kernel reduction. -/
theorem and_comm_on_boolPairs :
    boolPairs.all (fun p => (p.1 && p.2) == (p.2 && p.1)) = true := by
  decide

/-- info: 'PoleanSmoke.and_comm_on_boolPairs' does not depend on any axioms -/
#guard_msgs in
#print axioms and_comm_on_boolPairs

end PoleanSmoke
