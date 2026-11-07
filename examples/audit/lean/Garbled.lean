/-
A program that prints something which is not an audit report.

The smoke runner runs this in place of the worker for the `WorkerGarbled`
corpus entry. It exits 0 with output the report decoder must refuse, so the
settlement is `tool_failure` with reason `malformed_report` rather than
`worker_failed`.
-/
def main : IO UInt32 := do
  IO.print "this is not an audit report\n"
  return 0