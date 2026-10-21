# Developer guide

Polean currently consists of a Rust scaffold and an executable design pack. The
first implementation milestone must preserve the assurance distinctions in the
technical design rather than racing ahead to a broad Rego interpreter.

## Prerequisites

- Git;
- the Rust toolchain pinned by `rust-toolchain.toml`;
- Python 3.11 or later for the design harness;
- `jsonschema` and `hypothesis` from `validation/requirements.txt` for design
  checks;
- `cargo-audit` for `make audit`, which is a periodic check rather than a
  commit gate; and
- for the toolchain targets only: Python 3.14 built with `zstd` support (so
  that `compression.zstd` imports), `pytest` from
  `scripts/requirements-test.txt`, network access for the first provisioning
  run, and about 3.7 GiB of disk under `.polean/tools`.

`make tools` and `make toolchain-smoke` are the only targets that need the 3.14
interpreter; point them at one with `make toolchain-smoke PYTHON=...`.
`cargo-insta` is needed only to review a changed Regorus AST snapshot.

Hosted CI installs Markdown tooling separately. Local contributors may install
`mdtablefix` 0.6.0 or later and `markdownlint-cli2` to reproduce every
formatting and prose gate.

## Common commands

`make help` lists every target with its one-line description. The contributor
targets are:

| Target                   | Effect                                                             |
| ------------------------ | ------------------------------------------------------------------ |
| `make build`             | Build every Rust target and feature.                               |
| `make check-fmt`         | Verify Rust formatting and Markdown table layout; changes nothing. |
| `make fmt`               | Rewrite Rust formatting and Markdown tables in place.              |
| `make typecheck`         | Type-check every Rust target with warnings denied.                 |
| `make lint`              | Build documentation and run Clippy with warnings denied.           |
| `make markdownlint`      | Lint Markdown with the repository configuration.                   |
| `make test`              | Run Rust unit, integration, and documentation tests.               |
| `make test-scripts`      | Test the provisioning script with pytest.                          |
| `make tools`             | Provision the digest-checked OPA and Lean artefacts.               |
| `make toolchain-smoke`   | Provision, then run the ignored OPA and Lean observations.         |
| `make design-check`      | Validate schemas, policy fixtures, abstractions, and witnesses.    |
| `make design-properties` | Search the admitted input domain for model-side counterexamples.   |
| `make design-boundary`   | Test the harness's orchestration and storage boundaries.           |
| `make audit`             | Audit Rust dependencies for known vulnerabilities.                 |
| `make clean`             | Remove Rust and Python build artefacts.                            |
| `make all`               | Run every local commit gate in sequence.                           |

The usual loop is `make check-fmt`, `make typecheck`, `make lint`, `make test`,
`make test-scripts`, `make design-check`, `make design-properties`, and
`make design-boundary`, plus `make toolchain-smoke` when the toolchain tuple is
in scope. `make all` runs the commit gates, but run them individually while
iterating, so a failure is attributable to one gate. Do not run these
concurrently: the build cache serializes better sequentially, and concurrent
gate runs interleave their output.

`make toolchain-smoke` depends on `make tools`, so one command covers
provisioning. The first run downloads 628 MiB; later runs verify whatever is
already installed and use no network. `.polean/tools/` is untracked, and
`rm -rf .polean/tools` is always safe.

`make test-scripts` and `make toolchain-smoke` need the Python 3.14 interpreter
described above. The `toolchain` workflow runs both on every pull request that
touches a pin, the script, the Lean package, the smoke tests, the manifests, the
`Makefile`, or the workflow itself, so an unavailable local interpreter does
not leave the provisioning path unobserved.

`make design-check` may rewrite the committed validation summary and witness
files after verifying them. Review those changes rather than assuming any
regeneration is harmless. `make design-properties` only reads the repository
and writes nothing.

`make design-properties` needs `hypothesis`, which is not a system package.
Hosted CI installs `validation/requirements.txt` first. Locally, either install
that file into the active interpreter or point the target at one that already
has it: `make design-properties PYTHON=/path/to/python`.

`make audit` requires `cargo-audit` and reaches the network to fetch the
advisory database, so it is a periodic dependency check rather than part of
every commit. It is not in `make all` for that reason.

## Changing a toolchain pin

`toolchain-pins.json` is the authoritative identity of the provisioned
artefacts, and [ADR 0003](adrs/adr-0003-pin-the-e0-toolchain-tuple.md) is the record
of why the tuple is what it is. A pin change is a tuple change:

1. Edit `toolchain-pins.json` — the upstream URL, the SHA-256, and the byte
   size — or, for Regorus or Rust, `Cargo.toml` and `rust-toolchain.toml`.
2. Run `make tools`, which must reject any artefact that does not match the new
   pin, then `make toolchain-smoke`.
3. If a Regorus change alters the AST snapshot, review it with
   `cargo insta review` and accept only a difference you can explain.
4. Update ADR-003 with the new tuple, its provenance, and the reason, and
   refresh any prose that repeats the version.

Never edit an installed file under `.polean/tools/` to make an observation
pass: the directory is digest-named and disposable, and the next run replaces
it.

## The Rust scaffold

`src/lib.rs` is disposable. It exports `STATUS`, a human-readable string
describing the current crate, and `project_name()`, a `const fn` returning the
project name. Both exist, so the crate builds, documents, and tests before any
policy machinery exists, and both are `const`, so a caller may use them where a
constant expression is required.

`tests/stub.rs` checks both values at runtime and also assigns them to `const`
items, so a change that made either unavailable at compile time would fail the
build. No part of the scaffold is an assurance claim: it is a placeholder for
the orchestration described in the [technical design](technical-design.md).

The seven source fixtures are generated from their rule-list IR plus a reviewed
module header held in `validate_design.py`, and the design check fails on any
byte difference between the rendered text and the committed file. Edit the rule
list or the header and regenerate; do not hand-edit a `.rego` fixture, because
the check will reject the result.

## The validation harness

`validation/validate_design.py` checks the design fixtures. It separates
reading from publishing, so the checks receive reviewed documents as data
through `Repository` rather than reaching for a path of their own. This is
architectural separation, not enforced filesystem isolation: Python does not
prevent a check from opening a file, and only the checks' signatures and the
boundary tests give evidence that they do not.

- `Repository` is the read-only input boundary. Every method either returns a
  value or raises `HarnessIOError` naming the document, and every path is built
  from a repository-relative name, so a check names a document rather than
  holding a path of its own.
- `Workspace` subclasses `Repository` and is the sole publication boundary,
  adding `write_json` and `remove`. It is a parameter of `run_checks` and
  `main`, so the capability to change the tree appears in the signature rather
  than being implied by a module constant.
- `Fixtures` and `PolicyReview` are in-memory review data. `Fixtures.load()`
  reads every reviewed document once, and `validate_policies()` consumes that
  data and returns the counterexamples it found without writing anything.
- `run_checks()` is the composition root; `publish_witnesses()` persists a
  review's witnesses into the supplied workspace and replays them from there.
- `HarnessIOError` is the single shape for a read or write failure, and a
  document that fails its JSON Schema is reported as one named diagnostic. A
  run that fails a check before publication writes nothing; a failure inside
  publication can leave the witnesses it had already written, but never
  `design-validation.json`.

`validation/property_fixtures.py` states the same model-side obligations as
Hypothesis properties. It requires one explicit `load_admitted()` call, made by
`main`, and passes the result to `Suite`, so a property reads `self.admitted`
rather than module state. Importing the module reads no profile or schema
document, though it does execute the sibling harness to reuse its definitions.

The negative controls in `validate_design.py` use the committed witness set,
including `witnesses/deny-all.json`, as their accepted members, so those files
must exist before a run. A checkout without them fails the run with a named
diagnostic rather than regenerating them: publication happens after the
controls, so it cannot rescue a missing witness.

`validation/test_boundary.py` tests that orchestration and storage boundary
rather than the fixtures, and `make design-boundary` runs it in the design
workflow alongside the other two. It snapshots the witness JSON files and the
summary document — not the whole tree — to observe both storage seams directly,
and calls the real entry points with deliberately altered inputs, which is what
catches a `validate_policies` that returned a constant review.

## Change discipline

Keep functional changes and assurance-claim changes explicit. A parser feature,
new atom, wider input domain, changed failure outcome, or revised trust
boundary must update the relevant contracts and design text before
implementation is reported complete.

Do not add a Rego form merely because Regorus can parse it. Admission requires
a specified model, source mapping, proof strategy, conformance fixtures, and
failure semantics. Unknown syntax fails closed.

Do not accept submitted Lean, Lake, native-library, or compiled proof artefacts
as executable input. The proposed verifier regenerates fixed modules from
admitted data in a pinned environment.

## Tests

Add the smallest test at the boundary where a defect could recur:

- Rust unit and integration tests for orchestration and contracts;
- design-fixture validation for schema and example consistency;
- Lean theorems for semantic, abstraction, checker-soundness, and witness
  claims;
- OPA and Regorus replay tests for observed source behaviour; and
- hostile-input tests for evidence binding and process supervision.

Passing one class of test does not upgrade another assurance dimension. In
particular, differential testing is not a source-to-model proof.

### Documented examples

The style guide requires a marker comment on the line immediately before every
executable fenced example in a user-facing document, and a loader that fails
the build on an unmarked fence, an unterminated fence, a missing identifier, or
a duplicate identifier. `tests/documentation_examples/mod.rs` is that loader,
ported from the reference implementation in `leynos/netsuke`; it is applied to
`README.md` and `docs/users-guide.md` by
`tests/documentation_examples_tests.rs`. Name each marker after the surrounding
section rather than a positional counter, so reordering a document does not
renumber unrelated markers. Indented fences count, so an example inside a list
item is contracted too. Both `CommonMark` fence characters count as well: a
tilde fence needs a marker exactly as a backtick fence does, and each closes
only on a run of its own character. A closing run may be longer than its opener
and may carry trailing spaces or tabs. The loader reads from column zero, so it
measures indentation relative to the file rather than to a containing list or
block quote; an example nested deeper than three spaces therefore escapes the
check instead of failing it. Keep examples top level or indented within that
limit.

The loader places no upper bound on a fence run, so runs longer than any the
sweeps enumerate are accepted. The sweeps in
`tests/documentation_examples/sweep.rs` are bounded regression coverage over
the dimensions the rules turn on — both fence characters, closers equal to and
longer than their opener, opening and closing indentation varied independently
within the supported range, and body and identifier variants — and they are not
a proof about arbitrary input. Describe their coverage by its stated bounds
rather than as exhaustive.

The loader enforces the marker contract only. It returns the fenced text; it
does not execute it, so a passing run is evidence about markers and not about
whether the documented commands succeed.

Because the style guide asks for each example to be executable and exercised,
the repository records the exception instead of leaving it implicit. The
`ExecutionScope` table in `tests/documentation_examples/scope.rs` classifies
every contracted example, and the corpus tests hold that table to the
repository rather than to a comment:

- A `Gate` fence must name only targets the `Makefile` declares. This checks the
  `make` invocations, not any surrounding environment preparation, which the
  workflows perform in their own steps; the `make test` fence in the
  [README](../README.md) also re-enters the gate that loads it, which is why
  the loader classifies rather than executes.
- A `DeferredUnimplemented` fence must still have nothing to run. Today that is
  the `polean` commands in the [user guide](users-guide.md), whose interface
  has no binary target. The test reads `Cargo.toml` and fails once a `[[bin]]`
  is declared, so declaring the binary forces the example to be reclassified
  rather than leaving a stale exemption behind.
- Adding an example without recording its scope fails, and a scope left behind
  after its example is removed fails too.

Treat execution as a separate obligation, and do not describe an example as
tested until something runs it.

### Toolchain observations

These observations follow the conventions below.

- One integration binary, `tests/toolchain/main.rs`, holds every module. A
  separate binary per module would link Regorus more than once, and a module
  used by only one target would be reported as dead code under the denied
  `allow` lint.
- Observations return `Result<(), String>`, and helpers do too, so a failure
  names what was expected, what was found, and which source produced it. Avoid
  panics except where a test's own assertion is the observation.
- The Regorus AST snapshot is committed and reviewed. A diff is a change in
  what the frontend exposes, so review it rather than accepting it.
- Provisioned observations carry `#[ignore]` with the reason, and fail with a
  remediation message when the tool is absent. A silently skipped provisioned
  test would read as a pass.
- A missing tool is not a skip. `make toolchain-smoke` runs the ignored module
  by name, so an absent binary is a failure.

## Documentation

Start with [documentation contents](contents.md). Update the terms of reference
when the problem, scope, users, or acceptance criteria change. Update the
technical design when implementation or assurance mechanisms change. Record
substantive, durable choices as ADRs.

ADRs live in `docs/adrs/` and are named `adr-nnnn-title-slug.md`, where `nnnn`
is a four-digit, zero-padded serial number and `title-slug` is a kebab-case
American Standard Code for Information Interchange (ASCII) rendering of the
title. Proposals that need review before acceptance live in `docs/rfcs/` and
follow the same pattern with an `rfc-` prefix. Number each series sequentially,
never renumber a published record, and update every inbound link when a record
is renamed. The [documentation style guide](documentation-style-guide.md)
specifies the required sections and templates.
