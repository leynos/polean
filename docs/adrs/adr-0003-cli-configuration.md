# ADR 0003: use OrthoConfig at the planned CLI boundary

- **Status:** proposed.
- **Date:** 2026-10-26.
- **Decision owners:** repository sponsor and maintainers.

## Context

Polean currently provides a Rust library scaffold and a proposed command
contract; it has no command-line binary. Roadmap item 2.1.4 is the first
command milestone, and later milestones add `prove`, `verify`, and
`export-rego`. Issue 14 asks the future CLI to use the published `ortho_config`
0.10.0 release while forbidding an early CLI implementation. This record
defines the configuration boundary for those roadmap items. It does not claim
that the CLI or configuration is implemented.

The CLI contract reserves `--profile` for the policy profile, and gives
configuration failures status 5 while reserving status 6 for tool failures.
Evidence verification must remain hermetic: environment variables and
discovered files cannot alter regenerated evidence.

## Decision

When the roadmap reaches the command implementation, configuration will live in
the application crate at the CLI boundary and use `ortho_config` 0.10.0 with
its default features enabled. The application crate will require Rust 1.89.0 or
later. The existing reusable Polean library crate will not depend on
OrthoConfig or inherit that MSRV floor; it will receive validated domain
settings through its public boundary. Application configuration types derive
`OrthoConfig`; parse DTOs and domain types remain separate.

Only the output `format` setting is configurable. `check` and `prove` default to
`text` and accept `text` or `json`. Their configuration is namespaced in TOML
as `[cmds.check]` and `[cmds.prove]`. The application boundary maps
`POLEAN_CHECK_FORMAT` and `POLEAN_PROVE_FORMAT` to the corresponding command
settings. Command inputs, policy profiles, claims, evidence paths, output
paths, proof choices, and resource limits remain explicit CLI arguments or
fixed by the contract.

The supported configuration files will be TOML. Discovery reads the user file
`$XDG_CONFIG_HOME/polean/config.toml`, falling back to
`$HOME/.config/polean/config.toml` when `XDG_CONFIG_HOME` is unset, followed by
project-local `.polean.toml` in the working directory. Missing optional files
are ignored. Invalid or unreadable selected files, unknown keys, and invalid
values are configuration errors. No legacy environment variables or
configuration files exist in the baseline, so no compatibility aliases are
needed.

The application constructs the two file sources explicitly in user then project
order. OrthoConfig's default discovery is first-wins, and automatic stacked
scopes can contain more candidates than this contract permits; neither default
discovery nor unrestricted scope stacking is sufficient here. Do not enable
system or ancestor candidates, explicit path flags, or environment file
selectors. Reject TOML includes, `extends`, named profiles and unknown keys
before any loader strips them. Use the published source-aware APIs to compose
the selected sources, and test that no extra path participates.

Precedence is built-in defaults, user file, project file, environment, then
explicit CLI values. The default `--format text` is marked as absent for the
OrthoConfig merge, so the parser default cannot override file or environment
values. Environment names are mapped explicitly because the public flat names
`POLEAN_CHECK_FORMAT` and `POLEAN_PROVE_FORMAT` do not mirror the `cmds`
namespace used in TOML.

OrthoConfig named configuration profiles will not be enabled. `--profile`
continues to select a policy profile such as `polean.tenant-write.v0`.

`verify` will not discover configuration files or read configuration
environment variables; evidence regeneration uses explicit evidence, source and
claims under the pinned verification environment. Its format defaults to
`text`, overridden only by an explicit `--format`. `export-rego` will likewise
use explicit command arguments and no discovered configuration. Configuration
errors map to status 5 and tool errors map to status 6. Help and version
requests succeed; parser failures retain the parser's status and output stream.
The proposed CLI is not localized, so help and diagnostics use English.

## Consequences

The design keeps presentation preferences at the application edge without
letting configuration change policy semantics or replay inputs. The application
crate alone takes the dependency and Rust 1.89.0 floor. The configuration
loader will be tested with injected file and environment sources as each
command is implemented at its roadmap milestone.

When implemented, tests must cover all precedence tiers for each configurable
command, explicit source injection, missing optional files, malformed and
unreadable files, invalid format values, unknown keys, parser-default merging,
and the absence of discovery for `verify` and `export-rego`. Binary-level
compatibility tests must use isolated child processes rather than mutate the
test process environment. Configuration failures must be actionable and
distinct from tool failures.

## Alternatives considered

- **Use another configuration crate.** Rejected because issue 14 selects the
  published OrthoConfig release and its source-aware loading fits the planned
  application boundary.
- **Use OrthoConfig named profiles for configuration.** Rejected because the
  existing `--profile` contract selects a policy profile.
- **Configure policy inputs, claims, evidence, or proof choices.** Rejected
  because those values define the assurance request and must remain explicit.
- **Allow configuration discovery in `verify`.** Rejected because replay must
  regenerate evidence from explicit, pinned inputs.
- **Add OrthoConfig to the current Polean crate.** Rejected because it is the
  reusable library scaffold, while configuration belongs to the future
  application crate.

## References

- [CLI contract](../../contracts/cli.txt)
- [Technical design, §8](../technical-design.md#8-cli-outcomes-and-failure-semantics)
- [OrthoConfig v0.10.0 user's guide](../ortho-config-users-guide.md)
- [OrthoConfig v0.10.0 migration guide](../ortho-config-v0-10-0-migration-guide.md)
