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
```

`check` would establish only that a policy belongs to the selected profile.
`prove` would check all six registered requirements and package replayable
evidence. `verify` would reconstruct the admitted model from explicit source
and claims rather than execute submitted Lean or compiled proof modules.

The intended logical outcomes are proved, refuted, and unknown. Unsupported
syntax, invalid input, tool failure, invalid evidence, and runtime conformance
mismatch remain separate operational outcomes.

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
