/-
An `initialize` block that writes a sentinel if it ever runs.

The audit imports the job with `loadExts := false`, so initializers must not
run. The sentinel path is the first `LEAN_PATH` entry, which the smoke runner
sets to the fixture output directory for both the audit and the positive
control. If the audit ran the initializer, the sentinel would appear and the
corpus would fail; `lean --run` on `SentinelProbe` must create it, so a broken
fixture cannot pass by never writing anything.
-/
import AuditFixture.Goal

namespace AuditFixture.InitSentinel

initialize do
  let some path ← IO.getEnv "LEAN_PATH" | pure ()
  let dir := (path.splitOn ":").headD ""
  if dir != "" then
    IO.FS.writeFile (System.FilePath.mk dir / "audit-sentinel") "ran\n"

/-- The goal, proved without touching the sentinel. -/
theorem root : AuditFixture.Goal := fun _ => rfl

end AuditFixture.InitSentinel