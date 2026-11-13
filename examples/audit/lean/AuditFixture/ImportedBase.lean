import AuditFixture.Goal
import Std.Data.HashMap

/-!
An honest theorem whose replay needs a toolchain module outside `Init`.

The worker replays the audited modules over a base environment that always
holds `Init` and adds each toolchain module a fixture imports. `Init` does not
import `Std`, so this fixture's `helper` can only be replayed if the base
contains `Std.Data.HashMap`, where `Std.HashMap.Raw.WF.empty` lives. If the
worker dropped those additions the replay would reject the job, and the corpus
would report `kernel_rejected` instead of acceptance.

The fixture relies on no axiom: `helper` is a definitional fact about the
empty hash map, and the root is proved by reflexivity.
-/

namespace AuditFixture.ImportedBase

/-- The empty hash map is well formed, a fact that lives in `Std`. -/
theorem helper {α β : Type} [BEq α] [Hashable α] :
    (∅ : Std.HashMap.Raw α β).WF :=
  Std.HashMap.Raw.WF.empty

/-- The goal, proved without axioms. -/
theorem root : AuditFixture.Goal := fun _ => rfl

end AuditFixture.ImportedBase