# Record an enforceable Linux execution-boundary decision

This ExecPlan is a living document. Keep `Constraints`, `Tolerances`, `Risks`,
`Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan`
current as work proceeds.

Status: DRAFT (revision 2). Awaiting approval. Do not begin implementation
until the user explicitly approves this plan.

Roadmap item: [1.2.4](../roadmap.md). Branch:
`1-2-4-enforceable-linux-execution-boundary-decision`.

## Purpose / big picture

Polean will run two external programs on data derived from an untrusted policy:
the pinned Lean toolchain, which checks generated proof modules, and the pinned
Open Policy Agent (OPA) binary, which evaluates the submitted Rego source.
Technical design §11 requires each such child to run offline, inside memory and
output limits, in private storage, and requires cancellation to kill and reap
every process the job started. It also says that missing enforced isolation is
an execution failure for the first public profile, never a silently degraded
mode.

Nobody has yet shown that this boundary can be enforced. This task answers the
question with a bounded probe and records the answer as an architectural
decision record (ADR). The probe runs deliberately hostile child processes, and
the real pinned OPA and Lean binaries, through a test-only Rust supervisor. It
observes from outside whether each control held, and it requires every hostile
child to report what it attempted and the error it received, so that a child
that never ran cannot pass.

When the work is done, a contributor on a host that meets the enforcement
prerequisites runs `make boundary-probe` and sees five properties demonstrated,
each beside a negative control showing that the probe would notice the property
failing:

1. **Offline execution.** A job cannot reach a TCP listener on the host's
   loopback interface, an abstract Unix socket, or a socket file that the probe
   creates on the host, and it inherits no open descriptor beyond its standard
   streams.
2. **Memory limit.** A job that exceeds its memory limit is killed as a group,
   and the supervisor names the memory bound from systemd's record of the
   out-of-memory kill, not from the exit status alone.
3. **Output limit.** A job that floods standard output is stopped once it
   passes the cap, and the supervisor keeps exactly the first cap bytes.
4. **Process-group cancellation.** On a deadline or an explicit cancellation,
   every process in the job is killed, including one that left the process
   group, and the job's control group is confirmed empty before the supervisor
   returns.
5. **Private storage.** A job can write only to a private, size-bounded scratch
   area that is absent from the host's filesystem view and discarded with the
   job. Its root filesystem and inputs are read-only.

The real OPA binary evaluates the baseline policy inside the boundary and
returns the same decisions it returns outside, while an OPA query that calls
`http.send` cannot reach a host listener. The real Lean binary checks the smoke
module inside the 2 GiB and 60 s defaults. On a host that lacks a required
mechanism, or where a canary job shows that the mechanism did not take effect,
the supervisor starts no job and names what is missing.

The decision this task expects to record is: run each child in a systemd user
scope, which supplies cgroup v2 memory, swap, and task limits and a whole-tree
kill, and inside bubblewrap, which supplies user, network, mount, PID, IPC, and
UTS namespaces and an allowlisted, read-only filesystem view. The probe either
confirms or falsifies that hypothesis. Either result is recorded in the ADR. A
falsified hypothesis means public acceptance is blocked until a design revision
is accepted, as the roadmap's success criterion states.

This task changes no user-visible behaviour, because Polean has no command-line
program yet. The probe is test-only, as the 1.2.2 and 1.2.3 spikes were.
Roadmap 4.3.1 promotes the supervisor into the product and enforces the
boundary end to end, and roadmap 4.3.3 measures budgets on an agreed machine. A
passing probe says nothing about admission, the model proof, translation, or
runtime correspondence, the four claims in `AGENTS.md`. It concerns only the
execution environment in which those claims are later checked.

## Context and orientation

### Terms used in this plan

A **namespace** is a Linux kernel feature that gives a process its own view of
one kind of resource. A **user namespace** lets an unprivileged user act as
root inside the namespace without gaining privilege outside it, which is what
lets the other namespaces be created without `sudo`. A **network namespace**
gives the process its own network stack; a fresh one contains only an inactive
loopback interface. A **mount namespace** gives the process its own filesystem
view. A **PID namespace** gives the process its own process-number space; when
its first process (its **init**) dies, the kernel kills every other process in
it.

A **control group (cgroup)** is a kernel grouping of processes to which limits
apply. The host uses **cgroup v2**, the unified hierarchy. Writing `1` to a
cgroup's `cgroup.kill` file kills every process in the group, whatever its
process group or session. A **systemd user scope** is a transient unit that the
user's own systemd manager creates on request (`systemd-run --user --scope`).
It places a command in a new cgroup with limits such as `MemoryMax=`,
`MemorySwapMax=`, and `TasksMax=`. With `OOMPolicy=kill`, an out-of-memory kill
takes the whole group, and systemd records `Result=oom-kill` on the failed
scope.

**Bubblewrap** (`bwrap`) is an unprivileged sandboxing program, used by
Flatpak, that builds namespaces and a filesystem view from command-line
arguments. **Landlock** is a kernel security module that lets an unprivileged
process restrict its own filesystem access.

A **process group** is a set of processes that receive a signal sent to the
group. A child can leave its parent's group, so a process-group kill does not
by itself reach every descendant. An **escapee** in this plan is a probe child
that deliberately leaves the group.

A **canary** is a harmless job run through the full launch chain that reports
the security state it observes from inside, so that the supervisor can confirm
the mechanisms took effect rather than merely accepted their arguments.

A **negative control** is a run of the same probe with one enforcement layer
switched off, which must show the hostile behaviour succeeding. It proves that
the probe can fail, so a pass is not vacuous.

**Verus** is a deductive verifier for Rust, already used here for the admission
recognizer and the audit settlement.

### Repository survey

Polean is a design scaffold with a single Rust package, `polean`, edition 2024,
on the nightly toolchain pinned in `rust-toolchain.toml`. `src/lib.rs` holds no
process code. `Cargo.toml` has no runtime dependencies; its dev-dependencies
include `cap-std`, `googletest`, `insta`, `pretty_assertions`, `proptest`,
`rstest`, `rstest-bdd`, `rstest-bdd-macros`, `serde`, `serde_json`, and `vstd`.
`[lints.rust]` sets `unsafe_code = "forbid"`, so `CommandExt::pre_exec` and raw
`libc` calls are unavailable. `clippy.toml` bans `std::env::var` and its
relatives; `std::env::args` and `std::env::current_exe` are allowed. Test code
uses the capability wrapper `tests/toolchain/ambient.rs` instead of `std::fs`.

`tests/documentation_examples/scope.rs` fails when a binary target appears in
`Cargo.toml`, because roadmap 2.1.4 owns the application edge. A
repository-owned launcher binary is therefore unavailable, and every helper
process must be either a host program or the test binary itself.

The integration binary `tests/toolchain/main.rs` holds every toolchain
observation. Provisioned OPA and Lean tests live in `tests/toolchain/smoke/`
behind `#[ignore = "needs make tools; run make toolchain-smoke"]`, and
`make toolchain-smoke` runs them with the filter `smoke::`.
`tests/toolchain/support/mod.rs` provides `opa_path`, `lean_root`,
`repository_path`, and `baseline_source_path`; `tests/toolchain/scratch.rs`
creates scratch directories under `CARGO_TARGET_TMPDIR`. `#[ignore]` on an
`rstest-bdd` `#[scenario]` already works
(`tests/toolchain/smoke/audit/bdd/mod.rs`).

Every existing subprocess call clears its environment and passes an argument
array. None sets a process group, a memory limit, or an output cap. The only
deadline helper, `bounded` in `tests/toolchain/smoke/audit/render.rs`, kills
only the direct child, reads pipes without a bound, and discards the results of
`kill` and `wait`; it is not a pattern to copy.

The Verus proofs follow one pattern. The executable kernel sits inside a
`verus!` block in a test module, for example
`tests/toolchain/audit/settlement/decision/mod.rs`; its specification, lemmas,
and witnesses live under `verus/` and are reached by `#[path]`; an entry point
such as `verus/audit_settlement.rs` includes the same directory, so there is
one copy of the logic. `make verus` runs every entry in `VERUS_PROOF_FILES` and
then `scripts/check_verus_controls.py`, which applies named mutations and
requires each to fail. Verus is outside `make all` and CI.

ADR 0003 records, as open item 2, that `lake check` needs `bwrap` and that
`bwrap` was absent on the development host, which constrains 1.2.4 and 4.3.2;
ADR 0006 repeats it for `lake comparator`.

### Planning observations on the development host

These observations were made by hand on 2026-11-14, by the planner and by the
review panel. They shape the hypotheses; they are not acceptance evidence, and
the probe re-observes each one.

- **O1.** Rocky Linux 10.2, kernel 6.12.0, systemd 257, util-linux 2.40.2,
  glibc 2.39, x86_64, six cores. Unprivileged user namespaces are permitted.
  The security modules are
  `lockdown,capability,yama,selinux,bpf,landlock,ima,evm`; SELinux is
  permissive and the caller runs as `unconfined_t`. Landlock reports ABI 6.
  Swap is enabled. `kernel.yama.ptrace_scope` is 0 and `dev.tty.legacy_tiocsti`
  is 1. `systemd-oomd` is inactive.
- **O2.** cgroup v2 is mounted and the user manager delegates `cpu`, `memory`,
  and `pids`. The caller itself runs inside a user service, so a new scope is a
  sibling in `app.slice`, not a child of the caller's cgroup. The delegated
  `cgroup.kill` files are writable by the user.
- **O3.** A 300 MiB allocation in a scope with `MemoryMax=64M` and
  `MemorySwapMax=0` was killed with status 137;
  `systemctl --user show -P Result` then reported `oom-kill` and the failed
  unit stayed loaded until `reset-failed`. With the default `OOMPolicy=stop`,
  systemd stops the scope with `SIGTERM` and a 90 s stop timeout, which a job
  ignoring `SIGTERM` could drag out.
- **O4.** `unshare` with `--user --map-root-user --net --mount --pid`,
  `--ipc --uts`, and `--fork --kill-child --mount-proc`, wrapped in
  `setpriv --no-new-privs` with a Landlock rule set allowing writes only
  beneath a private directory, saw three processes and only the loopback
  interface, failed to connect to `1.1.1.1:53` with `Network is unreachable`,
  and could not write to `$HOME` or `/tmp`.
- **O5.** The same composition **connected** to the D-Bus session bus at
  `/run/user/1000/bus`. A network namespace isolates abstract Unix sockets but
  not socket files, and Landlock ABI 6 does not mediate connecting to an
  existing socket file. Through that bus a job could ask the user's systemd
  manager to start a process outside every limit. Offline execution therefore
  needs the filesystem view to hide host sockets.
- **O6.** `bwrap` is not installed. Rocky BaseOS offers `bubblewrap`
  0.10.0-5.el10_2, whose options include `--size`, `--disable-userns`,
  `--die-with-parent`, and `--remount-ro`. Installing it needs `sudo`, which
  the agent cannot use. `crun` 1.30.1 and Podman 5.8.2 are installed.

### Prior art consulted

`network_namespaces(7)` confirms that a network namespace isolates the abstract
Unix socket namespace, which leaves socket files to the filesystem view.
`systemd.resource-control(5)` and `systemd.service(5)` document
`OOMPolicy=kill` (which sets `memory.oom.group`), `RuntimeMaxSec=` on scopes,
and, from systemd 257, `PrivatePIDs=` and the private-namespace settings that a
user manager can apply through `PrivateUsers=`. Ubuntu 24.04 and later restrict
unprivileged user namespaces through AppArmor
(`kernel.apparmor_restrict_unprivileged_userns`), and bubblewrap-based
sandboxes in several projects fail there until the restriction is lifted or a
profile allows `bwrap`. The Go runtime reserves address space at start-up, and
its maintainers describe `RLIMIT_AS` as an outdated way to limit memory (Go
issue 38010), so OPA is not limited through address-space limits. The ADR cites
each source.

### Size history from comparable plans

The previous spikes underestimated their size. The 1.2.1 plan set 30 files and
about 1,200 net lines and reached 36 changed files without an escalation. The
1.2.2 plan ended at 96 non-fixture files and 14,072 net lines against a revised
40 files and 4,000 lines. The 1.2.3 plan set 30 files and 3,000 net lines,
measured 66 files and 9,406 net lines, and the sponsor raised the limit after
the fact; its decision log says "the estimate was wrong, not the scope". The
causes were review-driven repairs, documentation and fixture growth, and
untraced test infrastructure. This task has no fixture corpus, so the budgets
below come from a traced file list with explicit room for two review rounds.

## Conformance basis

Upstream artefacts are taken at `main` commit `ad217c8`:

- [Terms of reference](../terms-of-reference.md): §7 SC6 and SC7; §8.1, where
  the supported-platform matrix remains open; §8.2 A4 and A6.
- [Technical design](../technical-design.md) v0.1: §2, the "Evidence to
  reported result" boundary, whose residual trust includes "filesystem/process
  isolation"; §8, outcomes `unknown` (exit 3) and `tool_error` (exit 6); §10,
  V8; §11, resource control and execution environment; §13, the "Execution
  environment" prerequisite row.
- [Roadmap](../roadmap.md): 1.2.4 and its success criterion; downstream 2.3.1
  (deadline), 4.3.1 (containment), and 4.3.3 (measured budgets).
- [ADR 0002](../adrs/adr-0002-first-profile-assurance-boundary.md),
  [ADR 0003](../adrs/adr-0003-pin-the-e0-toolchain-tuple.md) open item 2, and
  [ADR 0006](../adrs/adr-0006-separate-proof-integrity-from-operational-integrity.md).
- `contracts/profile.json` budgets: `wall_seconds` 60, `memory_bytes`
  2147483648, `result_bytes` 1048576, `concurrent_proof_jobs` 1, and `network`
  false. `contracts/cli.txt` exits 3 and 6.

Trace chain:

```plaintext
TOR-A4, TOR-SC6, TDD §11 offline     -> RM-1.2.4 -> EP-M2, EP-M3 -> AC1
TDD §11 2 GiB memory, TOR-A6         -> RM-1.2.4 -> EP-M2, EP-M3 -> AC2
TDD §11 1 MiB output, TDD §8 bounded -> RM-1.2.4 -> EP-M1, EP-M2 -> AC3
TDD §11 kill and reap, TDD §10 V8    -> RM-1.2.4 -> EP-M2, EP-M3 -> AC4
TDD §11 private job directory        -> RM-1.2.4 -> EP-M3        -> AC5
TDD §11 missing isolation fails      -> RM-1.2.4 -> EP-M1, EP-M3 -> AC6
TDD §§6.3, 11 tools usable inside    -> RM-1.2.4 -> EP-M3        -> AC7
RM-1.2.4 "record a decision"         -> EP-M4 -> ADR 0007, TDD §§2, 11, 13 -> AC8
```

Acceptance items:

- **AC1.** Under the selected boundary, each offline attempt fails with the
  expected error (`ENETUNREACH` for TCP; `ECONNREFUSED` or `ENOENT` for the
  Unix sockets), no host listener records a connection, and the job's open
  descriptors are exactly its standard streams. Under the negative control,
  each connection succeeds.
- **AC2.** Under the selected boundary, both the allocation probe and the
  scratch-filling probe end with `MemoryLimit` read from `Result=oom-kill`, and
  the scope is empty afterwards. With the memory limit lifted, the same
  allocation completes.
- **AC3.** A stream that exceeds its cap yields `OutputLimit`, keeps exactly the
  first cap bytes, and stops the job; a stream within the cap is kept whole.
  Verus proves the capture kernel's invariants for every chunk sequence.
- **AC4.** Cancellation is shown in three cells: under the negative control, a
  process-group kill alone leaves the escapee running; under the negative
  control, a cgroup kill removes it; under the selected boundary, the escapee
  dies even when only the process group is killed, through the PID namespace.
  After every job, deadline or not, the supervisor confirms the cgroup empty.
- **AC5.** Under the selected boundary, the job writes only beneath its scratch
  area; writes to `/`, `/dev/shm`, the inputs, and any host path fail; filling
  the scratch area beyond `--size` fails with `ENOSPC`; and no file from the
  job remains on the host. Under the negative control, the job writes a marker
  into a host directory that the probe owns.
- **AC6.** When a required program, the user manager, delegated controllers, or
  the canary's expected state is missing, the supervisor returns `Unavailable`
  naming every missing control and starts no job. This holds for injected
  missing paths on every host.
- **AC7.** Pinned OPA returns the same decisions for a `true` and a `false`
  baseline request inside the boundary as outside; an OPA query calling
  `http.send` against a host listener reaches nothing; pinned `lean` checks
  `PoleanSmoke.lean` inside the 2 GiB and 60 s defaults.
- **AC8.** ADR 0007 records the decision, the enforcement prerequisites, the
  evidence, the alternatives, and the residual trust; the technical design,
  developers' guide, and roadmap agree with it, and no document claims more
  than the probe showed.

## Constraints

- Keep the four claims in `AGENTS.md` separate. The probe concerns the
  execution environment only. Never describe a passing probe as proving,
  verifying, or certifying a policy, translation, or runtime.
- Do not use `unsafe`, relax `unsafe_code = "forbid"`, or use
  `CommandExt::pre_exec`.
- Do not add a binary target, an item to `src/`, or any dependency. Everything
  is test-only, under `tests/toolchain/boundary/` and `verus/`.
- Launch every program by an explicit absolute path with a cleared environment
  and an argument array. Never build a shell command from a path or data, and
  never resolve a program through `PATH`.
- Do not weaken a control to make a probe pass, and do not report a missing or
  ineffective control as anything other than `Unavailable`. Missing enforced
  isolation is an execution failure, never a degraded mode.
- A probe passes only on positive evidence from the child: a start line carrying
  the run's nonce and the expected error for each attempt. Silence is failure.
- Hostile behaviours run only inside a supervised job. The hostile entry point
  does nothing unless its argument list carries a behaviour token with a nonce,
  so an accidental `--ignored` run is inert. Every hostile behaviour has a
  finite size that the host survives under its negative control: no fork bomb
  and no unbounded allocation.
- Offline probes connect only to listeners the probe creates. Never connect to
  the user's real D-Bus socket or any other real host service.
- Spawn and reap each job on the same thread, because `--die-with-parent`
  fires when the spawning thread exits.
- Reset only the exact unit names the run created; never reset a pattern.
- Do not change a toolchain pin, `toolchain-pins.json`, `lean/`, the Verus
  pins, `examples/`, or `contracts/`.
- Do not use `assume`, `admit`, `external_body`, or another trusted proof body
  in `verus/`.
- Do not change the host's configuration. Installing the distribution's
  bubblewrap is an operator action the user performs.
- Keep Rust modules under 400 lines and functions under 70 lines, with a
  module-level comment on every module and documentation on every item.

Out of scope, with owners: rejecting symlinks, path traversal, and artefact
paths outside the job directory (2.3.1 and 4.3.1); carrying one remaining
deadline across several child invocations (2.3.1, then 4.3.1); naming the
exhausted bound in `contracts/result.schema.json` (4.3.1); measured budgets and
peak memory (4.3.3); seccomp filtering, session keyrings, `AF_VSOCK`, and
kernel resources outside cgroup accounting such as inotify instances (4.3.1
decides). The probe's input snapshots are copied from trusted repository files.

If satisfying the objective requires violating a constraint, stop, record the
conflict in `Decision log`, set the status to `BLOCKED`, and escalate.

## Tolerances (exception triggers)

Exclude this ExecPlan from every count. Count net lines (additions minus
deletions) and files touched against `git merge-base main HEAD` (`ad217c8` at
the start):

```bash
git diff --numstat "$(git merge-base main HEAD)" -- . \
  ':(exclude)docs/execplans/1-2-4-enforceable-linux-execution-boundary-decision.md'
```

Each category is enforced on its own:

- **Code**, meaning Rust under `tests/`, Verus under `verus/`, Python under
  `scripts/`, `.feature` files, and `Makefile` recipes: traced at about 2,700
  net lines in about 26 files. The trace is `boundary/mod.rs`, `capture/mod.rs`,
  `outcome.rs`, `platform.rs`, `canary.rs`, `launch.rs`, `scope.rs`,
  `supervisor.rs`, `hostile/mod.rs` with one module per behaviour family,
  `probe/mod.rs` with `unconfined.rs`, `bubblewrap.rs`, and `tools.rs`,
  `bdd.rs`, and `tests.rs`; three Verus files plus the entry; the
  control-script entries and their test counts; the feature file; and the
  `Makefile` target. Limit: **3,500 net lines and 32 files**.
- **Configuration** (`Cargo.toml`, `Cargo.lock`, `.config/`, `.github/`):
  expected none. Limit: **20 net lines and 2 files**. Any new dependency is an
  exception whatever its size.
- **Documentation** (Markdown outside `docs/execplans/`, and `AGENTS.md`):
  traced at about 600 net lines in 9 files: the ADR, the technical design, the
  developers' guide, the repository layout, the contents page, the roadmap, ADR
  0003 and ADR 0006 pointers, and `AGENTS.md`. Limit: **850 net lines and 10
  files**.
- **Generated:** reviewed `insta` snapshots are excluded from line counts and
  limited to **2 files**.

Other triggers:

- **Platform:** if the selected boundary fails any of AC1 to AC5 or AC7 on the
  probe host, record the falsification in the ADR draft, set the status to
  `BLOCKED`, and escalate with the alternatives in the decision log.
- **Flakiness:** if any probe fails intermittently across five consecutive
  `make boundary-probe` runs, stop and escalate. Do not add retries.
- **Duration:** if `make boundary-probe` takes longer than five minutes after
  the tools are provisioned, escalate before tuning.
- **Proof:** if the capture proof needs more than about 300 lines of
  specification, lemmas, and witnesses, or forces a change to the kernel's
  planned interface, escalate. Do not weaken the specification to fit.
- **Scope:** if the work needs a design element not described here, such as a
  seccomp filter, a custom init, a pinned bubblewrap build, or a change to the
  technical design beyond §§2, 11, and 13, escalate.

## Risks

- **R1. Bubblewrap is absent.** The user is asked to run
  `sudo dnf install bubblewrap` as part of approving this plan. EP-M1 and EP-M2
  need no `bwrap`; EP-M3 does not start without it.
- **R2. `systemd-run` cannot reach the user manager from a cleared
  environment.** It needs `XDG_RUNTIME_DIR`. Pass exactly
  `XDG_RUNTIME_DIR=/run/user/<uid>`, with the user ID read from the scratch
  directory's metadata through `tests/toolchain/ambient.rs`, and let
  `bwrap --clearenv` empty the job's environment.
- **R3. Timing against systemd.** `systemd-run` joins its scope a moment after
  it starts, and systemd records `Result=` after the processes die. The
  supervisor waits, within a bound, until `/proc/<pid>/cgroup` names the
  expected scope before starting the job's clock, and after reaping polls
  `ActiveState`, within a bound, until the scope is `failed` or gone. A scope
  never seen active, or a bound exceeded, is `ContainmentFailure`.
- **R4. Bubblewrap's behaviour differs from the planned options.** EP-M3 opens
  with `bwrap --version`, the canary, and the argument snapshot. An option
  missing from 0.10.0 is a scope exception.
- **R5. The pinned tools need a host path the allowlist hides.** AC7 exposes
  it. Add a read-only bind only for a named, justified path recorded in the
  decision log; never bind `$HOME` or an ancestor of it, `/run`, or `/tmp`.
- **R6. Self re-execution behaves differently inside the sandbox.** The test
  binary is bound as a single file at `/job/bin/probe` and run with
  `--exact --ignored --test-threads=1 --nocapture -q`; probes match their
  child's nonce lines and ignore harness output.
- **R7. Probes disturb other agents on the shared host.** Every hostile
  behaviour is finite, memory probes use 64 MiB limits against 256 MiB attempts,
  `make boundary-probe` runs with one test thread, unit names carry a per-run
  nonce, and each probe resets its own failed units and removes its scratch
  directories through a drop guard.
- **R8. The prerequisites are narrower than reviewers' machines.** Ubuntu 24.04
  and later restrict user namespaces by default; hosts without a systemd user
  session have no delegated cgroup; an enforcing SELinux policy may confine the
  caller. The canary turns each of these into a named `Unavailable` rather than
  a vacuous pass. A second platform belongs to 4.3.3.
- **R9. File descriptors leak into the job.** Rust opens its own descriptors
  close-on-exec, but the test harness or Cargo's jobserver may hold others, and
  neither `systemd-run` nor `bwrap` closes them. The descriptor probe exposes
  any leak; the ADR records it as residual trust or as a reason to prefer the
  alternative that starts jobs from the service manager.

## Design

### The boundary under test

The supervisor is test-only Rust in `tests/toolchain/boundary/`. For each job
it creates a scratch directory holding a read-only input snapshot, runs the
platform check and the canary, then spawns one host program with a cleared
environment, a new process group (`CommandExt::process_group(0)`), standard
input from `/dev/null`, and both output streams piped. It reads each stream on
its own thread through its own instance of the capture kernel, polls the child
against a single deadline, and tears the job down after every run.

The selected configuration, `Bubblewrap`, launches this chain; each program
replaces the previous one or runs as its only child:

```plaintext
supervisor (test binary, one thread per job)
  -> /usr/bin/systemd-run --user --scope --quiet --unit=polean-probe-<nonce>
       -p MemoryMax=<limit> -p MemorySwapMax=0 -p TasksMax=<n>
       -p OOMPolicy=kill -p KillSignal=SIGKILL
       -p RuntimeMaxSec=<deadline plus grace>
     -> /usr/bin/bwrap --unshare-user --unshare-net --unshare-pid
          --unshare-ipc --unshare-uts --unshare-cgroup --disable-userns
          --die-with-parent --new-session --cap-drop ALL --clearenv
          --ro-bind /usr /usr  (and /bin, /lib, /lib64 as symlinks into /usr)
          --ro-bind <program file or tool directory> <fixed path>
          --ro-bind <input snapshot> /job/in
          --size <scratch bytes> --tmpfs /job/work
          --proc /proc --dev /dev --chdir /job/work
          --setenv <each named variable>
          --remount-ro /
          -- <program> <arguments>
```

`systemd-run --scope` runs the command itself, so the supervisor's direct child
stays the process it spawned. The scope supplies the limits, a group
out-of-memory kill, a backstop deadline that survives the supervisor's death,
and an authoritative whole-tree kill. Bubblewrap supplies the namespaces and an
allowlisted filesystem: only `/usr`, the program, the inputs, a private
size-limited scratch area, and fresh `/proc` and `/dev` exist inside, so
`$HOME`, `/run`, `/tmp`, and every host socket are absent, and the root is
read-only. Pages written to the scratch `tmpfs` are charged to the scope, so
`--size` must leave headroom below `MemoryMax=` for the tools. `--new-session`
with standard input from `/dev/null` closes the terminal-injection path that
`legacy_tiocsti=1` leaves open. No-new-privileges, the dropped capabilities,
and the unmapped real root neutralize the set-user-ID programs visible under
`/usr`, and the PID namespace hides host processes from `ptrace` and signals.

Each stream has its own cap. Standard output, which carries the structured
result, is capped at `result_bytes` (1 MiB). Standard error, which carries
diagnostics, has a separate 64 KiB cap, recorded as a probe choice for 4.3.1 to
confirm. Set-up errors from `systemd-run` and `bwrap` land in standard error
and count against its cap.

**Teardown runs after every job**, whether it ended normally, hit a limit, or
was cancelled. The supervisor:

1. records the scope's cgroup path from `ControlGroup` while the scope is
   active, and snapshots `cgroup.procs`;
2. sends `SIGKILL` to the job's process group through
   `/usr/bin/kill -s KILL -- -<pgid>`;
3. writes `1` to the scope's `cgroup.kill` through `ambient`;
4. reaps the direct child, treating only "already exited" as benign;
5. confirms, within a bound, that `cgroup.events` reports `populated 0` or the
   cgroup has gone, and that every snapshotted host PID is gone or reused;
6. joins the stream readers, which cannot block on a pipe held by a survivor
   because none survives;
7. reads `Result=` with the bounded poll from R3, then resets that exact unit if
   it failed.

If step 5 fails, the outcome is `ContainmentFailure`, whatever else happened.
The ADR states the roadmap's "process-group cancellation" precisely: the
process group is killed, but the cgroup is the authority, because the escapee
probe shows that a process-group kill alone is insufficient without a PID
namespace.

The negative-control configuration, `Unconfined`, keeps the scope, the process
group, and the teardown, but omits bubblewrap. The memory and output negative
controls lift the specific limit, and the cancellation cells skip the cgroup
kill. Probes live under separate paths, `boundary::probe::unconfined::` and
`boundary::probe::bubblewrap::`, so each configuration can run on its own.

### Platform check and canary

The platform check reports five controls, each `Available` or `Missing(reason)`:

1. **Programs:** `/usr/bin/systemd-run`, `/usr/bin/systemctl`,
   `/usr/bin/kill`, and `/usr/bin/bwrap` exist and are executable, and `bwrap`
   reports version 0.10.0 or later.
2. **User manager:** `systemctl --user is-system-running` answers through the
   cleared environment.
3. **Delegation:** the user manager's cgroup lists `memory` and `pids` in
   `cgroup.subtree_control`.
4. **Canary isolation:** a canary run through the full chain prints the run's
   nonce and reports `NoNewPrivs: 1`, `CapEff: 0`, a single loopback interface,
   `ENETUNREACH` for an outbound connect, and only standard descriptors. A
   `bwrap` set-up failure, such as AppArmor refusing the user namespace, lands
   here with its message.
5. **Canary limits:** while the canary's scope is active, `memory.max`,
   `memory.swap.max`, and `pids.max` read back the requested values. systemd
   accepts limits it cannot apply, so this read-back is the only evidence that
   they apply.

The verdict is `Ready` only when all five are `Available`. Otherwise it is
`Unavailable`, listing every missing control, and no job starts. The canary is
itself a hostile-entry behaviour, so it is the same test-binary code path as
the probes.

### Hostile behaviours

The hostile entry point is one ignored test, `boundary::hostile::entry`, whose
exact name is built from `module_path!()` and pinned by a unit test. The
supervisor runs the test binary with
`--exact --ignored boundary::hostile::entry polean-hostile=<behaviour>:<nonce>
--test-threads=1 --nocapture -q`.
libtest treats the extra token as another filter that matches nothing, and the
entry reads it through `std::env::args()`. Without the token the entry returns
immediately, under any runner.

Each behaviour prints `polean-hostile start <nonce>` and then one line per
attempt with its target and resulting error. The families are: canary; offline
(TCP to a host loopback listener, an abstract socket, a socket file under the
probe's host directory passed as an argument, and a descriptor listing); memory
(allocate and touch 256 MiB; fill the scratch area); output (write 2 MiB to
standard output); escapee (start a grandchild in a new process group that
sleeps, then sleep); and storage (write to the scratch area, `/`, `/dev/shm`,
`/job/in`, and a host directory path passed as an argument, then fill the
scratch area beyond `--size`).

### Hypotheses

- **H1.** Under `Bubblewrap`, AC1 to AC5 hold on the probe host.
- **H2.** A network namespace alone does not block socket files (O5); the
  allowlisted view does.
- **H3.** Under `Unconfined`, a process-group kill leaves the escapee running
  and a cgroup kill removes it; under `Bubblewrap`, the PID namespace removes
  it even after a process-group kill alone.
- **H4.** Pinned OPA and Lean work inside `Bubblewrap` within the defaults,
  needing no host path beyond `/usr`, their own directories, and their inputs.
- **H5.** When any control is missing or ineffective, the check reports
  `Unavailable` before any job starts.

A falsified hypothesis is recorded in `Surprises & discoveries`, the decision
log, and the ADR. A falsified H1, H4, or H5 triggers the platform tolerance.

### Observations, outcomes, and the proposed mapping

The probe's pure function `classify` turns the supervisor's observations into
an outcome: `Completed` (exit 0, no bound fired, cgroup empty); `ChildFailed`;
one or more of `MemoryLimit`, `OutputLimit`, `Deadline`, and `Cancelled`, with
every bound that fired listed; `ContainmentFailure`, which outranks all others;
and `Unavailable` from the platform check. These are the probe's observation
categories, not a product taxonomy.

The ADR proposes, for 4.3.1 to decide, that a named bound maps to `unknown`
(exit 3), which `contracts/cli.txt` already describes as "not settled,
including resource exhaustion", and that `Unavailable`, `ContainmentFailure`,
and an unexpected child failure map to `tool_error` (exit 6). An explicit user
cancellation has no contract outcome today, and the ADR says so.

### Enforcement prerequisites

The ADR names the enforcement prerequisites, not a supported platform: Linux
with cgroup v2; a systemd user manager reachable through `XDG_RUNTIME_DIR` that
delegates the `memory` and `pids` controllers; `systemd-oomd` either inactive
or confirmed not to have caused a reported kill; bubblewrap 0.10.0 or later
from the distribution; and a host policy (AppArmor, SELinux, or sysctl) that
lets bubblewrap create its namespaces, which the canary checks. The tested
instance is the O1 host after bubblewrap is installed. The ADR states that
Ubuntu 24.04 and later, with their default AppArmor restriction, fail closed.
Hosted CI is not an enforcement platform and does not run `make boundary-probe`.

This does not settle the terms of reference's open supported-platform matrix
(§8.1) or the agreed measurement machine (OQ4, roadmap 4.3.3).

## Verification plan

### Obligations and methods

The kernel and systemd enforce memory, namespaces, and the filesystem view; the
supervisor itself enforces only the output caps, the deadline, and teardown.
The obligations are a small pure kernel Polean owns, finite decision logic, and
external behaviour that only real processes can demonstrate.

**V-CAP (capture kernel, Verus).** The kernel consumes the chunks of one
stream. Its state is the kept bytes `kept` and a flag `tripped`, with the cap
fixed at construction. For every finite sequence of chunks:

1. **I1, bound:** `|kept| <= cap`.
2. **I2, prefix:** `kept` equals the first `min(|s|, cap)` bytes of the
   concatenated stream `s`.
3. **I3, trip:** `tripped` holds exactly when `|s| > cap`.
4. **I4, monotone:** once `tripped` holds it stays, and `kept` never changes
   again.

Method: Verus, because I2 and I3 quantify over chunk sequences of unbounded
length, the kernel is small, stable, and pure, and it is the one limit the
supervisor enforces rather than delegates. Artefacts: the kernel in
`tests/toolchain/boundary/capture/mod.rs` inside `verus!`;
`verus/boundary_capture_spec.rs`, `verus/boundary_capture_lemmas.rs`,
`verus/boundary_capture_witnesses.rs`; and the entry
`verus/boundary_capture.rs`, added to `VERUS_PROOF_FILES`. Command:
`make verus`. Discharge: zero errors, no trusted bodies, and every new control
rejected.

**V-CLS (classification, enumeration).** `classify` never returns `Completed`
when a bound fired, cancellation was requested, or the cgroup was not empty; it
returns `ContainmentFailure` whenever the cgroup was not empty after teardown;
and it lists every bound that fired. The input is a finite product of Booleans
and a fixed set of exit forms. Method: an `rstest` test that enumerates every
combination and asserts those three properties, plus named precedence cases.
Enumerating a finite domain is complete, so Verus would add ceremony without
rigour.

**V-AVL (fail-closed verdict, enumeration and injection).** The verdict is
`Ready` exactly when all five controls are `Available`, and otherwise lists
every missing control. Method: enumeration of all 32 combinations, and named
tests that inject missing program paths and a failing canary report into the
real check. These run under `make test` on any host, because they start no job.

**V-ARGV (launcher arguments, snapshot and matchers).** The `Bubblewrap`
argument vector for a canonical job built from fixed synthetic paths is an
`insta` snapshot reviewed by hand. Matchers assert that every isolating option
is present, that `--size` immediately precedes `--tmpfs`, that `--remount-ro /`
is last before `--`, and that nothing binds `/`, `/run`, `/tmp`, `/home`, or
any ancestor of the user's home, and that no `--bind` or `--dev-bind` appears.

**E1 to E5 (enforcement, end-to-end probes).** Each property is demonstrated by
a hostile behaviour under `Bubblewrap` beside its negative control, as AC1 to
AC5 describe, with the three cancellation cells of AC4. Method: ignored
integration tests run by `make boundary-probe`, with behavioural scenarios
bound to the main outcomes. These are the only evidence for the external
behaviour.

**E6 (real tools).** AC7, by ignored integration tests that reuse the smoke
tests' OPA invocation inside and outside the boundary, and run
`lean /job/in/PoleanSmoke.lean` with `LEAN_SYSROOT` set and the file bound
read-only. `PoleanSmoke.lean` imports nothing and pins its axiom reports with
`#guard_msgs`, so a successful exit is meaningful and no `.lake` directory is
written.

### External assumptions (axioms)

1. The kernel isolates network, mount, PID, IPC, and UTS resources per
   namespace as `namespaces(7)` documents, including abstract Unix sockets.
2. Killing a PID namespace's init kills every process in that namespace.
3. cgroup v2 enforces `memory.max`, `memory.swap.max`, and `pids.max`; charges
   `tmpfs` pages to the writer's cgroup; kills every member on `cgroup.kill`;
   and kills the whole group on out-of-memory when `memory.oom.group` is set.
4. systemd applies the requested scope properties, records `Result=oom-kill`,
   and keeps a failed scope loaded until reset.
5. Bubblewrap 0.10.0 implements its documented options.
6. `SIGKILL` cannot be caught, and a pipe reaches end-of-file once every write
   end is closed.

The canary's read-back and isolation report, and every probe, exercise these
assumptions on the probe host. A kernel, systemd, or bubblewrap defect beyond
that reach is residual trust recorded in the ADR and in technical design §2.

### Non-vacuity

- **V-CAP:** the witnesses construct a stream under the cap, one landing exactly
  on it, and one crossing it inside a chunk, so no precondition excludes the
  interesting cases. Four controls in `scripts/check_verus_controls.py` must
  each be rejected: keeping the whole chunk instead of clamping it (I1);
  keeping nothing from the chunk that crosses the cap (I2, and I4 if a later
  chunk is then kept); tripping when the stream reaches the cap rather than
  passes it (I3); and a specification without the prefix clause, whose witness
  lemma must then fail. The pytest count assertions in
  `scripts/tests/test_check_verus_controls.py` grow to match.
- **V-CAP, executable side:** a `proptest` property, using the deterministic
  `TestRunner` pattern in `tests/admission/properties/tests.rs`, splits random
  streams into random chunks with the cap drawn relative to the stream length,
  so overflow, exact fit, and underflow all occur by construction, and compares
  the kept bytes with `stream[..min(len, cap)]`.
- **V-CLS and V-AVL:** the enumeration asserts that every outcome and both
  verdicts occur. A deliberate mutation letting a fired bound return
  `Completed` must fail the test; it is applied by hand once in EP-M1 and the
  result recorded in `Surprises & discoveries`.
- **E1 to E5:** each probe requires the child's start line with the nonce and
  the expected error for each attempt, and its negative control must succeed in
  the same run. The canary's read-back means a limit systemd silently ignored
  becomes `Unavailable`, not a pass.
- **E6:** inside-boundary OPA decisions are compared with outside-boundary ones
  for both a `true` and a `false` request, so a constant result cannot pass.

### Proof sketch for V-CAP

The model state is `(cap, kept, tripped)` with initial state
`(cap, [], false)`. Feeding a chunk `c` takes `k = min(|c|, cap - |kept|)` and
produces `kept' = kept + c[0..k]` and
`tripped' = tripped || |c| > cap - |kept|`. Before the first trip, the bytes
seen equal `|kept|`, so the kernel needs no separate counter of bytes seen, and
no arithmetic can overflow.

The per-step contract of `feed` is
`kept' == (kept + c).take(min(|kept| + |c|, cap))` together with the trip
update. The whole-stream specification is that, for chunks `c1 … cn` with
concatenation `s`, `kept == s.take(min(|s|, cap))` and `tripped == (|s| > cap)`.

The lemmas expected to carry the proof are:

1. **Prefix extension:** for sequences `s` and `c` and a cap `n`,
   `(s + c).take(min(|s| + |c|, n)) == s.take(min(|s|, n)) + c.take(k)` with
   `k = min(|c|, n - min(|s|, n))`. This is the inductive step for I2.
2. **Trip characterization:** `tripped` after the step holds exactly when
   `|s| + |c| > n`, given that it held exactly when `|s| > n` before. This
   gives I3.
3. **Stability after trip:** once `|s| > n`, `k` is zero for every later chunk,
   which gives I4.
4. **Fold:** an induction over the chunk sequence lifting the per-step
   contract to the whole-stream specification.

These statements are hypotheses about the proof's shape. The interface is
`Capture::new(cap)` and `feed(&mut self, chunk: &[u8]) -> Fed`, where `Fed`
reports the number of bytes kept from this chunk and whether this call tripped
the cap; the reader thread signals the supervisor on the first trip. Each call
is one model step. If verification shows a lemma unnecessary or needing
decomposition, record the change in the decision log; do not weaken the
specification.

## Test strategy

Unit tests use `rstest` fixtures and tables, `googletest` matchers
(`assert_that!` with `elements_are!`, `contains_substring`, and similar), and
`pretty_assertions` for structural comparisons. This is the first use of
`googletest` in the repository; keep it to matcher assertions inside `rstest`
tests that return `Result<(), String>`, as elsewhere:

- `boundary::capture`: named cases for an empty stream, an exact fit, one byte
  over, a crossing inside a chunk, and many empty chunks; the `proptest`
  property.
- `boundary::outcome`: the V-CLS and V-AVL enumerations and named precedence
  cases, such as a deadline and a memory kill together, and a clean exit with a
  surviving process.
- `boundary::launch`: the V-ARGV snapshot and matchers.
- `boundary::platform`: injected missing programs and a failing canary report.
- `boundary::hostile`: the entry point's exact name, and that it is inert
  without a token.

End-to-end probes are ignored tests run by `make boundary-probe`, which depends
on `make tools` for E6 and runs with one test thread. Behavioural scenarios use
`rstest-bdd`; their bindings live under `boundary::probe::` so the target's
filter selects them. The feature file
`tests/features/execution_boundary.feature` is authoritative; keep this copy in
step with it:

```gherkin
Feature: Enforced Linux execution boundary
  Polean runs each external tool under an enforced boundary, or not at all.

  Scenario: A job cannot reach host listeners or inherit host descriptors
    Given the platform check reports every control available
    And the probe listens on a loopback port, an abstract socket, and a socket file
    When a hostile job tries to connect to each of them
    Then every attempt fails inside the job with the expected error
    And no listener records a connection
    And the job holds only its standard descriptors

  Scenario: A job that exceeds its memory limit is killed as a group
    Given the platform check reports every control available
    When a job allocates 256 MiB under a 64 MiB limit
    Then the outcome names the memory limit
    And the job's control group is empty

  Scenario: A job that floods its output is stopped at the cap
    Given the platform check reports every control available
    When a job writes 2 MiB to standard output under a 1 MiB cap
    Then the outcome names the output limit
    And exactly the first 1 MiB of the output is kept

  Scenario: A deadline reaches a process that left the process group
    Given the platform check reports every control available
    And a job has started a grandchild in a new process group
    When the job's deadline expires
    Then the outcome names the deadline
    And the job's control group is empty

  Scenario: A job writes only to its private scratch area
    Given the platform check reports every control available
    When a job writes to its scratch area, the root, its inputs, and a host path
    Then only the scratch write succeeds
    And no file from the job remains on the host

  Scenario: Missing enforcement starts no job
    Given bubblewrap is not at its configured path
    When a job is submitted
    Then the outcome is unavailable and names bubblewrap
    And no job is started
```

The last scenario runs under `make test` on every host; the others are bound to
ignored tests. Where a step needs the negative control, its definition runs the
control in the same scenario and asserts that it succeeded.

`insta` is used only for the reviewed argument vector. The probe's property
matrix is pasted into the ADR from the run log rather than snapshotted, because
it records one host. `kani` is not used: the finite logic is enumerated, and
the one unbounded property is proved in Verus.

## Milestones

### EP-M1: pure kernels, proof, and fail-closed verdict

Requirements: AC3 (kernel), AC6 (verdict), V-CAP, V-CLS, V-AVL, and V-ARGV. End
state: `tests/toolchain/boundary/` exists with `mod.rs`, `capture/mod.rs`,
`outcome.rs`, `platform.rs`, `launch.rs`, `hostile/mod.rs` (the inert entry
point only), and `tests.rs`; `tests/toolchain/main.rs` declares
`mod boundary;`; the Verus entry, specification, lemmas, witnesses, and four
controls exist, and `VERUS_PROOF_FILES` includes `verus/boundary_capture.rs`.
Nothing starts a sandboxed process.

Work test first: add the named cases and enumerations, run
`cargo test --locked --all-features --test toolchain boundary::` and watch them
fail for the missing behaviour, then implement. Write the Verus specification
and lemma statements before the kernel body.

Acceptance: `make check-fmt`, `make typecheck`, `make lint`, `make test`, and
`make verus` pass; `make test-scripts` passes under Python 3.14; the `classify`
mutation check is recorded. Conformance check: no binary target, dependency, or
`unsafe`; the kernel interface matches the proof sketch. Recovery: revert the
milestone's commits. Remaining gaps: no real process yet.

### EP-M2: supervisor, scope, canary, and negative controls (prototyping)

Requirements: AC4's `Unconfined` cells, the `Unconfined` negative controls for
AC1, AC2, AC3, and AC5, and AC6 end to end except the canary's isolation
fields. End state: `scope.rs`, `supervisor.rs`, `canary.rs`, the hostile
behaviour modules, and `probe/unconfined.rs` exist; `make boundary-probe`
exists, runs
`cargo test --locked --all-features --test toolchain -- --ignored
--test-threads=1 boundary::probe::`,
and depends on `tools`. With the filter `boundary::probe::unconfined::`, every
negative control shows its hostile behaviour succeeding, the escapee survives a
process-group kill and dies on the cgroup kill, and teardown confirms every
cgroup empty.

This milestone is labelled prototyping because it settles the supervisor's
shape. Keep it test-only and additive.

Acceptance: the M1 gates, plus the `unconfined` probes green on the O1 host.
Conformance check: only safe `std`, explicit paths, and cleared environments;
no host configuration changed. Recovery: revert the milestone's commits.
Remaining gaps: the selected configuration is untested.

### EP-M3: the selected boundary and the real tools

Precondition: bubblewrap is installed on the probe host. If it is not, stop,
set the status to `BLOCKED`, and report.

Requirements: AC1, AC2, AC4, AC5, AC6 (canary), and AC7 under `Bubblewrap`, and
H1 to H5. End state: `probe/bubblewrap.rs`, `probe/tools.rs`, `bdd.rs`, and the
feature file exist; every `Bubblewrap` probe passes beside its negative
control; the canary reports the expected isolation; the OPA and Lean runs pass;
the scenarios are bound.

Acceptance: the M1 gates, `make toolchain-smoke`, and five consecutive green
`make boundary-probe` runs with their durations recorded. Conformance check:
every extra read-only bind is named in the decision log, and nothing binds
`$HOME`, an ancestor of it, `/run`, or `/tmp`. Recovery: revert to the M2
plateau. Remaining gaps: one host only.

### EP-M4: the decision record and documentation

Requirements: AC8. End state:

- `docs/adrs/adr-0007-enforce-the-execution-boundary-with-a-user-scope-and-bubblewrap.md`
  follows ADR 0006's shape: Status (`Proposed` until the user accepts it in
  conversation), Date, Context and problem statement, Decision drivers,
  Decision, Consequences, Observed results (the property matrix and single-run
  tool timings, labelled as observations rather than 4.3.3 measurements),
  Alternatives considered, Known risks and limitations, Outstanding decisions,
  Acceptance, and References. The alternatives compare concretely: a systemd
  user service with private namespaces (no install, no inherited descriptors,
  but a denylist filesystem view); rootless `crun` with a repository-owned OCI
  configuration (declarative and installed, but with container state and slower
  start-up); the util-linux composition (O4 and O5 with the exact command);
  Landlock alone; `RLIMIT_AS`; Podman; and a repository-owned launcher. The
  constraints for 4.3.1 include spawning and reaping on one thread, the stream
  caps, and the proposed outcome mapping.
- `docs/technical-design.md`: §2's "Evidence to reported result" row adds the
  kernel, systemd, and bubblewrap to the remaining trust, leaving the
  first-slice guarantee column unchanged; §11 states the selected mechanism,
  the enforcement prerequisites, the teardown order, and the stream caps; §13's
  "Execution environment" row records that 1.2.4 selected the mechanism and
  showed it enforceable on the probe host, and that 4.3.1 enforces it in the
  product; new references from R19 cover bubblewrap,
  `systemd.resource-control(5)`, the kernel's cgroup v2 documentation,
  `namespaces(7)`, `network_namespaces(7)`, and Go issue 38010; ADR 0007 joins
  the companions.
- `docs/adrs/adr-0003-pin-the-e0-toolchain-tuple.md` and
  `docs/adrs/adr-0006-separate-proof-integrity-from-operational-integrity.md`:
  a dated addendum pointing open item 2 to ADR 0007, where bubblewrap becomes a
  deliberate prerequisite.
- `docs/developers-guide.md`: a section "The execution-boundary probe (1.2.4)"
  covering prerequisites, `make boundary-probe` and its row in the commands
  table, reading the property matrix, adding a hostile behaviour safely, and a
  reuse strategy for 4.3.1, which promotes the supervisor, kernels, and
  launcher and keeps the probes as its acceptance suite.
- `docs/repository-layout.md`, `docs/contents.md`, and the gate list in
  `AGENTS.md` name the new module, ADR, plan, and target.
- `docs/roadmap.md`: 1.2.4 is marked done with a short "Recorded" note once the
  user has accepted the ADR.

`docs/users-guide.md` and `docs/terms-of-reference.md` do not change: no user
behaviour changes, and the probe resolves neither A4 nor A6.

Acceptance: `make check-fmt`, `make lint`, `make test`, `make markdownlint`, and
`make design-check` pass. Conformance check: every link in `Conformance basis`
is current, and no document claims more than the probe showed. Recovery:
documentation commits revert independently.

## Concrete steps

Run commands from the repository root, and log gate output, for example:

```bash
make lint 2>&1 | tee /tmp/lint-polean-1-2-4-enforceable-linux-execution-boundary-decision.out
```

1. Confirm the branch with `git branch --show-current`.
2. EP-M1:

   ```bash
   cargo test --locked --all-features --test toolchain boundary::
   make check-fmt && make typecheck && make lint && make test
   make verus
   make test-scripts
   ```

   Expected: the new unit tests pass; `make verus` reports
   `verus/boundary_capture.rs` verified with zero errors; `make verus-controls`
   lists the four new controls as rejected.
3. EP-M2:

   ```bash
   cargo test --locked --all-features --test toolchain -- --ignored \
     --test-threads=1 boundary::probe::unconfined::
   ```

   Expected: every negative control passes, and every teardown reports an empty
   cgroup.
4. EP-M3, after the user has run `sudo dnf install bubblewrap`:

   ```bash
   /usr/bin/bwrap --version
   make toolchain-smoke
   for run in 1 2 3 4 5; do make boundary-probe || break; done
   ```

   Expected: `bubblewrap 0.10.0`, and every probe passes in all five runs.
5. EP-M4: write the ADR and documentation, then run the gates listed there.
6. Commit after each green step with an imperative subject and a body that
   explains the reason and the assurance impact.

## Skills and references

Load `execplans` to maintain this plan; `rust-router` first for Rust questions,
then `rust-unit-testing`, `proptest`, `verus`, and `domain-cli-and-daemons` for
process lifecycle and shutdown; `rust-verification` if the method choice is
questioned; `arch-decision-records` for the ADR; and `en-gb-oxendict-style` for
prose. Use `firecrawl-mcp` for further external documentation. The CodeGraph
server was unavailable during planning; use it when it is.

Read first: `docs/technical-design.md` §§2, 8, 10, 11, and 13;
`docs/terms-of-reference.md` §§7 and 8; ADR 0006 as the ADR template;
`docs/developers-guide.md` on tests, Verus, and toolchain observations;
`docs/reliable-testing-in-rust-via-dependency-injection.md` for injecting
program paths and the canary report;
`docs/rust-testing-with-rstest-fixtures.md`; `docs/rust-doctest-dry-guide.md`;
`docs/complexity-antipatterns-and-refactoring-strategies.md` for keeping the
supervisor's functions small; `docs/scripting-standards.md` for the control
script; and `docs/documentation-style-guide.md`. There is no
`docs/references.md`; references live in the technical design's References
section and in each ADR.

## Progress

- [x] 2026-11-14: renamed the branch and set its upstream to
  `origin/1-2-4-enforceable-linux-execution-boundary-decision`.
- [x] 2026-11-14: surveyed the repository, earlier plans, and documents with a
  reconnaissance team; probed the host (O1 to O6); consulted prior art.
- [x] 2026-11-14: drafted revision 1.
- [x] 2026-11-14: expert panel review (sandbox security, Rust testing and
  verification, assurance and proportionality); revision 2 addresses it.
- [ ] User approval, acceptance of D1 and D3, and installation of bubblewrap.
- [ ] EP-M1.
- [ ] EP-M2.
- [ ] EP-M3.
- [ ] EP-M4, and the user's acceptance of ADR 0007.

## Surprises & discoveries

- 2026-11-14: a network namespace with Landlock still reached the D-Bus session
  bus through its socket file (O5). Offline execution needs the filesystem
  view, not only the network namespace.
- 2026-11-14: a repository-owned launcher is ruled out by
  `unsafe_code = "forbid"` and by the binary-target guard, which steered the
  design to host programs and self re-execution.
- 2026-11-14: systemd accepts limits it cannot apply, and a `bwrap` set-up
  failure looks like a child failure, so presence checks alone could make a
  probe pass vacuously. The canary and the limit read-back answer this.

## Decision log

- **D1, 2026-11-14:** compose a systemd user scope with bubblewrap. Rationale:
  bubblewrap builds an allowlisted filesystem view from arguments, closing O5
  without repository-owned mount code; it is mature and already expected by
  `lake check`; it needs no `unsafe` or binary target; and the scope supplies
  the limits and whole-tree kill that bubblewrap lacks. The panel's strongest
  alternatives were a systemd user service with private namespaces (no install,
  but a denylist view) and rootless `crun` (installed and declarative, but with
  container state). Either is the fallback if bubblewrap cannot be installed or
  H1 fails. Needs the user's acceptance with the plan.
- **D2, 2026-11-14:** limit memory through cgroups, not `RLIMIT_AS`, because
  Go reserves address space at start-up and a cgroup limit measures memory
  used. Use `OOMPolicy=kill` and `KillSignal=SIGKILL`, because the default
  `stop` policy waits up to 90 s for a job ignoring `SIGTERM`.
- **D3, 2026-11-14:** prove the capture kernel in Verus, enumerate the finite
  logic, and prove no plumbing. Two panel members recommended deferring the
  proof to 4.3.1, where the kernel becomes product code, to save about 300
  lines. The plan keeps it because the output cap is the one bound the
  supervisor enforces itself, the simplified kernel needs no counter, and the
  1.2.3 settlement proof set the precedent of proving a test-only kernel before
  promotion. Needs the user's acceptance with the plan; if the user prefers to
  defer it, drop V-CAP, the Verus files, and the four controls, and keep the
  `proptest` property and named cases.
- **D4, 2026-11-14:** keep the util-linux composition out of the code; the ADR
  records it with the reproducible command.
- **D5, 2026-11-14:** select hostile behaviours by an argument token, not a
  file, so that negative controls without bubblewrap still run and an accidental
  `--ignored` run stays inert.
- **D6, 2026-11-14:** tear down and confirm every job, not only cancelled ones,
  and confirm emptiness from `cgroup.events`, because a successful scope is
  garbage-collected and a surviving grandchild would otherwise block the pipe
  readers.
- **D7, 2026-11-14:** run `lean` directly on a read-only `PoleanSmoke.lean`
  rather than `lake build`, which would write `.lake/` into the read-only
  repository view.
- **D8, 2026-11-14:** call the result "enforcement prerequisites demonstrated
  on the probe host", not a supported platform, leaving ToR §8.1 and 4.3.3
  open; and record a falsified hypothesis in the ADR rather than only stopping.
- **D9, 2026-11-14:** do not use `bwrap --json-status-fd`. Passing it an extra
  descriptor would need `pre_exec`, and sharing standard error lets the child
  forge status lines. The canary detects set-up failure instead.

## Outcomes & retrospective

Not started.

## Revision notes

- Revision 1, 2026-11-14: initial draft for expert review.
- Revision 2, 2026-11-14: addressed the panel. Hostile behaviours now take an
  argument token, so negative controls can run; teardown runs after every job
  and confirms emptiness from `cgroup.events`; a canary and limit read-back
  make the fail-closed check observe effect, not presence; `OOMPolicy=kill`,
  `KillSignal=SIGKILL`, `RuntimeMaxSec=`, `--remount-ro /`, and a descriptor
  probe were added; cancellation is shown in three cells; the Lean run uses
  `lean` directly; per-stream caps are fixed; the capture kernel lost its
  counter and gained a prefix control; the storage wording no longer claims the
  host cannot see the scratch area; §13 and the platform wording no longer
  widen the claim; the users' guide and terms of reference edits were dropped
  and ADR 0003 and 0006 pointers added; out-of-scope items now name their
  owners. Budgets were re-traced to 32 code files. The remaining work is
  unchanged in order; approval now also asks for D1, D3, and the bubblewrap
  installation.
