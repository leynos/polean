# User guide

Polean does not yet provide an executable checker. This guide records the
proposed first workflow so reviews can distinguish the designed interface from
current repository capability.

## Proposed workflow

The technical design proposes these commands:

<!-- tested-example: guide-proposed-commands -->
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

## Planned configuration

The future command-line application is planned to use `ortho_config` 0.10.0 for
presentation format only. Configuration is not implemented yet. When the
commands are delivered, `check` and `prove` will accept `--format text|json`,
with `text` as the default. The policy input, policy profile, claims, evidence
directory, output paths, and proof choices remain explicit command-line
arguments.

The planned TOML files use separate command namespaces. For example,
`[cmds.check]` contains `format = "json"`, while `[cmds.prove]` can contain
`format = "text"`. Includes, `extends`, and named configuration profiles are
not supported; unknown keys are rejected.

Discovery will read the user file at `$XDG_CONFIG_HOME/polean/config.toml`, or
`$HOME/.config/polean/config.toml` when `XDG_CONFIG_HOME` is unset, followed by
the project file `.polean.toml` in the working directory. A missing optional
file will be ignored. Files with invalid TOML, unreadable selected paths,
unknown keys, or unsupported values will be reported as configuration errors.

Environment overrides use `POLEAN_CHECK_FORMAT` and `POLEAN_PROVE_FORMAT`. The
application boundary will map these names to the matching command namespace.
Precedence will be built-in defaults, user file, project file, environment,
then explicit command-line values. The parser's `text` default will count as
absent for merging, so it does not hide a file or environment value.

Configuration profiles are not planned. `--profile` remains the policy profile
selector, including `polean.tenant-write.v0`. `verify` will not discover files
or read configuration environment variables: it will regenerate evidence from
its explicit inputs and pinned environment. Its format defaults to `text` and
can be changed only with `--format`. `export-rego` will also use explicit
arguments only. Configuration failures will use status 5; tool failures will
remain status 6. Help and version requests will exit 0 on stdout; command-line
parse failures will exit 2 on stderr. The CLI is not localized, so its proposed
help and diagnostics use English text.

## Intended outcomes

The logical outcome of a proof request is proved, refuted, or unknown.
Unsupported syntax, invalid input, tool failure, invalid evidence, and runtime
conformance mismatch remain separate operational outcomes, and
[`contracts/cli.txt`](../contracts/cli.txt) fixes the exit status for each:

| Status | Meaning                                                      |
| ------ | ------------------------------------------------------------ |
| 0      | All command-specific gates succeeded.                        |
| 2      | A claim has a checked counterexample, or CLI parsing fails.  |
| 3      | Verification did not settle, including resource exhaustion.  |
| 4      | Legal syntax is outside the admitted profile.                |
| 5      | Malformed source, invalid request, claims, or configuration. |
| 6      | Tool, process, protocol, or required dependency failure.     |
| 7      | Evidence, digest, theorem, or axiom-policy rejection.        |
| 8      | Runtime/model disagreement or a non-Boolean runtime result.  |

For `prove` and `verify`, status 0 additionally requires exactly six proved
claims, checked bindings, axiom approval, and tested runtime agreement. The
reported outcome is `model_proved`, with assurance `kernel_checked`,
`trusted_frontend`, and `tested_agreement`; a successful run does not upgrade
tested runtime agreement to a refinement proof. The theorem takes the request's
subject, resource, and action attributes as given and does not authenticate
them. [ADR 0002](adrs/adr-0002-first-profile-assurance-boundary.md) states
exactly what a passing run proves, trusts, and leaves outside the theorem.

## Available today

The repository supplies the baseline policy, mutations, schemas, and a Python
design check:

<!-- tested-example: guide-design-check -->
```bash
python -m pip install -r validation/requirements.txt
make design-check
```

That check demonstrates fixture consistency and concrete counterexamples. It
does not supply the model theorem described above. See the
[validation record](../validation/README.md) before quoting its results.
