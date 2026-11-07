/-
The positive control for `InitSentinel`.

Importing the module runs its initializer, so `lean --run` on this file must
create the sentinel. Without this control, a fixture whose initializer never
runs would pass the audit for the wrong reason.
-/
import AuditFixture.InitSentinel

def main : IO UInt32 := return 0