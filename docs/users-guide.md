# User guide

Polean does not yet provide an executable checker. This guide records the
proposed first workflow so reviews can distinguish the designed interface from
current repository capability.

## Proposed workflow

The technical design proposes these commands:

```bash
polean check policy.rego \
  --profile polean.tenant-write.v0 \
  --format json

polean prove policy.rego \
  --claims claims.json \
  --evidence evidence \
  --format json

polean verify evidence \
  --policy policy.rego \
  --claims claims.json \
  --format json

polean export-rego policy-dir \
  --output policy.rego
```

`check` would establish only that a policy belongs to the selected profile.
`prove` would check all six registered requirements and package replayable
evidence. `verify` would reconstruct the admitted model from explicit source
and claims rather than execute submitted Lean or compiled proof modules.
`export-rego` would render the admitted policy IR back to Rego and write it to
the path given by `--output`. The IR is source-neutral, so the rendered text is
not expected to reproduce the submitted file byte for byte; the original source
is preserved separately and remains the artefact the profile admission was
performed against.

## Intended outcomes

The logical outcome of a proof request is proved, refuted, or unknown.
Unsupported syntax, invalid input, tool failure, invalid evidence, and runtime
conformance mismatch remain separate operational outcomes, and
[`contracts/cli.txt`](../contracts/cli.txt) fixes the exit status for each:

| Status | Meaning                                                      |
| ------ | ------------------------------------------------------------ |
| 0      | All command-specific gates succeeded.                        |
| 2      | A requested model claim has a checked counterexample.        |
| 3      | Verification did not settle, including resource exhaustion.  |
| 4      | Legal syntax is outside the admitted profile.                |
| 5      | Malformed source, invalid request, claims, or configuration. |
| 6      | Tool, process, protocol, or required dependency failure.     |
| 7      | Evidence, digest, theorem, or axiom-policy rejection.        |
| 8      | Runtime/model disagreement or a non-Boolean runtime result.  |

For `prove` and `verify`, status 0 additionally requires exactly six proved
claims, checked bindings, axiom approval, and tested runtime agreement. The
reported assurance remains model_proved + trusted_frontend + tested_agreement;
a successful run does not upgrade tested runtime agreement to a refinement
proof.

## Available today

The repository supplies the baseline policy, mutations, schemas, and a Python
design check:

```bash
python -m pip install -r validation/requirements.txt
make design-check
```

That check demonstrates fixture consistency and concrete counterexamples. It
does not supply the model theorem described above. See the
[validation record](../validation/README.md) before quoting its results.
