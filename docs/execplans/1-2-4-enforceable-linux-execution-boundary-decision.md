# Record an enforceable Linux execution-boundary decision

This ExecPlan is a living document. Keep `Constraints`, `Tolerances`, `Risks`,
`Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan`
current as work proceeds.

Status: DRAFT (revision 1). Awaiting approval. Do not begin implementation
until the user explicitly approves this plan.

Roadmap item: [1.2.4](../roadmap.md). Branch:
`1-2-4-enforceable-linux-execution-boundary-decision`.

## Purpose / big picture

Polean will run two external programs on data derived from an untrusted policy:
the pinned Lean toolchain, which checks generated proof modules, and the pinned
Open Policy Agent (OPA) binary, which evaluates the submitted Rego source.
Technical design §11 requires that each such child runs offline, inside memory
and output limits, in private storage, and that cancellation kills and reaps
every process the job started. It also says that missing enforced isolation is
an execution failure for the first public profile, never a silently degraded
mode.

Nobody has yet shown that this boundary can be enforced on the machine Polean
is meant to run on. This task answers that question with a bounded probe and
records the answer as an architectural decision record (ADR). The probe runs
deliberately hostile child processes, and the real pinned OPA and Lean
binaries, through a test-only Rust supervisor, and observes from outside
whether each control held.

When the work is done, a contributor on the intended platform runs
`make boundary-probe` and sees five properties demonstrated, each beside a
negative control that shows the probe would have noticed the property failing:

1. **Offline execution.** A job cannot reach a TCP listener on the host's
   loopback interface, an abstract Unix socket, or a pathname Unix socket such
   as the user's D-Bus session bus.
2. **Memory limit.** A job that allocates beyond its limit is killed, and the
   supervisor names the memory bound from systemd's record of an
   out-of-memory kill, not from the exit status alone.
3. **Output limit.** A job that floods standard output is stopped once it
   passes the cap, and the supervisor keeps exactly the first cap bytes.
4. **Process-group cancellation.** On a deadline or an explicit cancellation,
   every process in the job is killed, including one that left the process
   group, and nothing in the job is left running when the supervisor returns.
5. **Private storage.** A job can write only to a private, size-bounded scratch
   area that the host cannot see and that disappears with the job. Its inputs
   are read-only, and it cannot create or modify files elsewhere.

The real OPA binary evaluates the baseline policy inside the boundary and
returns the same decisions it returns outside, while a probe query that calls
`http.send` cannot reach the host. The real Lean binary checks the smoke
theorem inside the 2 GiB default. On a host where any required mechanism is
missing, the supervisor refuses to run the job and names what is missing.

The decision this task records is expected to be: run each child under a
systemd user scope, which supplies cgroup v2 memory, swap, and task limits and
a whole-tree kill, and inside bubblewrap, which supplies user, network, mount,
PID, IPC, and UTS namespaces and an allowlisted read-only filesystem view. The
probe either confirms that decision or falsifies it. A falsified hypothesis is a
legitimate outcome: it blocks public acceptance and requires an explicit design
revision, as the roadmap's success criterion states.

This task changes no user-visible behaviour, because Polean has no command-line
program yet. The probe is test-only, as the 1.2.2 and 1.2.3 spikes were.
Roadmap 4.3.1 promotes the supervisor into the product, and roadmap 4.3.3
measures budgets on an agreed machine. A passing probe says nothing about
admission, the model proof, translation, or runtime correspondence, the four
claims in `AGENTS.md`; it concerns only the execution environment those claims
depend on.

## Context and orientation

### Terms used in this plan

A **namespace** is a Linux kernel feature that gives a process its own view of
one kind of resource. A **user namespace** lets an unprivileged user act as
root inside the namespace without gaining privilege outside it, which is what
allows the other namespaces to be created without `sudo`. A **network
namespace** gives the process its own network stack; a fresh one contains only
an inactive loopback interface, so nothing is reachable. A **mount namespace**
gives the process its own filesystem view. A **PID namespace** gives the
process its own process-number space; when the first process in it (its
**init**) dies, the kernel kills every other process in it.

A **control group (cgroup)** is a kernel grouping of processes to which limits
apply. Polean's host uses **cgroup v2**, the unified hierarchy. Writing `1` to
a cgroup's `cgroup.kill` file, or asking systemd to kill a unit, kills every
process in the group regardless of process group or session. A **systemd user
scope** is a transient unit that the user's own systemd manager creates on
request (`systemd-run --user --scope`); it places a command in a new cgroup
with limits such as `MemoryMax=`, `MemorySwapMax=`, and `TasksMax=`. When the
kernel kills a process in a scope for exceeding `MemoryMax=`, systemd marks the
scope failed with `Result=oom-kill`.

**Bubblewrap** (`bwrap`) is an unprivileged sandboxing program, used by Flatpak,
that builds namespaces and a filesystem view from command-line arguments. It
can bind host directories read-only, mount private size-limited `tmpfs`
filesystems, clear the environment, start a new session, drop capabilities, and
die with its parent. **Landlock** is a kernel security module that lets an
unprivileged process restrict its own filesystem access; it is considered here
as an alternative.

A **process group** is a set of processes that receive a signal sent to the
group as a whole. A child can leave its parent's group, so a process-group kill
does not by itself reach every descendant. An **escapee** in this plan is a
probe child that deliberately leaves the group.

A **negative control** is a run of the same probe with one enforcement layer
switched off, which must show the hostile behaviour succeeding. It proves that
the probe can fail, so that a pass is not vacuous.

**Verus** is a deductive verifier for Rust. The repository already uses it for
the admission recognizer and the audit settlement.

### Repository survey

Polean is a design scaffold with a single Rust package, `polean`, edition 2024,
on the nightly toolchain pinned in `rust-toolchain.toml`. `src/lib.rs` holds no
process code. `Cargo.toml` has no runtime dependencies; its dev-dependencies
already include `cap-std`, `googletest`, `insta`, `pretty_assertions`,
`proptest`, `rstest`, `rstest-bdd`, `rstest-bdd-macros`, `serde`, `serde_json`,
and `vstd`. `[lints.rust]` sets `unsafe_code = "forbid"`, so
`CommandExt::pre_exec`, raw `libc` calls, and any other `unsafe` are
unavailable. `clippy.toml` bans `std::env::var` and its relatives, and the test
code bans `std::fs` in favour of the capability-based wrapper
`tests/toolchain/ambient.rs`.

`tests/documentation_examples/scope.rs` fails when a binary target appears in
`Cargo.toml`, because roadmap 2.1.4 owns the application edge. A
repository-owned launcher binary is therefore not available to this task, and
any helper process must be either a host program or the test binary itself.

The integration binary `tests/toolchain/main.rs` holds every toolchain
observation. Provisioned OPA and Lean tests live in `tests/toolchain/smoke/`
behind `#[ignore = "needs make tools; run make toolchain-smoke"]`, and
`make toolchain-smoke` runs them after `make tools` provisions the pinned
binaries into `.polean/tools`. `tests/toolchain/support/mod.rs` provides
`opa_path`, `lean_root`, `repository_path`, and `baseline_source_path`.
`tests/toolchain/scratch.rs` creates scratch directories under
`CARGO_TARGET_TMPDIR`.

Every existing subprocess call clears its environment and passes an argument
array. None sets a process group, a memory limit, or an output cap. The only
deadline helper, `bounded` in `tests/toolchain/smoke/audit/render.rs`, kills
only the direct child and reads its pipes without a bound; `smoke/mod.rs` says
enforced isolation is 1.2.4's work.

The Verus proofs follow one pattern that this plan reuses. The executable
kernel sits inside a `verus!` block in a test module, for example
`tests/toolchain/audit/settlement/decision/mod.rs`. Its specification, lemmas,
and witnesses live under `verus/` and are reached by `#[path]`. An entry point
such as `verus/audit_settlement.rs` includes the same directory, so there is
one copy of the logic. `make verus` runs every entry in `VERUS_PROOF_FILES`
and then `scripts/check_verus_controls.py`, which applies named mutations and
requires each to fail. Verus is outside `make all` and CI.

Bubblewrap has history here. ADR 0003 records that `lake check` needs `bwrap`
and that `bwrap` was absent on the development host, and that this constrains
1.2.4 and 4.3.2. ADR 0006 repeats it for `lake comparator`.

### Planning observations on the development host

These observations were made by hand while planning, on 2026-11-14. They are
inputs to the probe's hypotheses, not acceptance evidence. The probe re-observes
each one under test.

- **O1.** The host is Rocky Linux 10.2, kernel 6.12.0, systemd 257, util-linux
  2.40.2, glibc 2.39, x86_64. Unprivileged user namespaces are permitted
  (`user.max_user_namespaces` is 512556). The security modules are
  `lockdown,capability,yama,selinux,bpf,landlock,ima,evm`, and SELinux is
  permissive. Landlock reports ABI version 6. Swap is enabled.
- **O2.** cgroup v2 is mounted, and the user manager delegates the `cpu`,
  `memory`, and `pids` controllers.
  `systemd-run --user --scope -p MemoryMax=64M -p MemorySwapMax=0` placed a
  command in a new scope whose `memory.max` read 67108864.
- **O3.** A Python allocation of 300 MiB in such a scope was killed with status
  137, and `systemctl --user show -P Result` then reported `oom-kill` for the
  failed scope, with `OOMPolicy=stop`. The failed unit stays loaded until
  `systemctl --user reset-failed`.
- **O4.** `unshare --user --map-root-user --net --mount --pid --ipc --uts
  --fork --kill-child --mount-proc`, wrapped in `setpriv --no-new-privs` with
  a Landlock rule set allowing writes only beneath a private job directory, ran
  as root inside its namespaces. It saw three processes, only the loopback
  interface, could not connect to `1.1.1.1:53` (`Network is unreachable`), and
  could not write to `$HOME` or `/tmp`.
- **O5.** The same composition **could connect** to the D-Bus session bus at
  `/run/user/1000/bus`. A network namespace isolates abstract Unix sockets but
  not pathname ones, and Landlock ABI 6 does not mediate connecting to an
  existing socket file. Through that bus a job could ask the user's systemd
  manager to start a process outside every limit. A network namespace alone
  therefore does not deliver offline execution; the filesystem view must hide
  host sockets.
- **O6.** `bwrap` is not installed. Rocky BaseOS offers `bubblewrap`
  0.10.0-5.el10_2. Installing it needs `sudo`, which the agent cannot use.
  `crun` 1.30.1 and Podman 5.8.2 are installed.

### Prior art consulted

The Linux manual page `network_namespaces(7)` confirms that a network namespace
isolates the abstract Unix socket namespace, which leaves pathname sockets to
the filesystem view. Ubuntu 24.04 and later restrict unprivileged user
namespaces through AppArmor (`kernel.apparmor_restrict_unprivileged_userns`),
and bubblewrap-based sandboxes in several projects fail there until the
restriction is lifted or a profile allows `bwrap`. The Go runtime reserves
address space at start-up, and its maintainers describe `RLIMIT_AS` as an
outdated way to limit memory (Go issue 38010), so OPA is not limited through
address-space limits. These sources are cited in the ADR.

### Size history from comparable plans

The three previous spikes underestimated their size badly. The 1.2.1 plan set
30 files and about 1,200 net lines; review counts reached 36 files with no
recorded escalation. The 1.2.2 plan ended at 96 non-fixture files and 14,072
net added lines against a revised 40 files and 4,000 lines, and the user
approved the overrun after the fact. The 1.2.3 plan set 30 files and 3,000 net
lines, measured 66 files and 9,406 net lines, and the sponsor raised the limit
to 15,000 after the fact; its decision log records that "the estimate was wrong,
not the scope". The common causes were review-driven repair rounds,
documentation and fixture growth, and test infrastructure that the estimate did
not trace.

This task has no fixture corpus, which drove most of the 1.2.2 and 1.2.3
growth, and its Verus kernel is smaller than either earlier one. The budgets
below are therefore built from a traced file list rather than scaled from the
earlier limits, with explicit contingency for review rounds.

## Conformance basis

Upstream artefacts are taken at `main` commit `ad217c8`:

- [Terms of reference](../terms-of-reference.md): §7 SC6 and SC7; §8.1, the
  open supported-platform matrix; §8.2 A4 (local offline verification) and A6
  (the resource envelope).
- [Technical design](../technical-design.md) v0.1: §2, the "Evidence to
  reported result" boundary, whose residual trust includes
  "filesystem/process isolation"; §8, outcomes `unknown` (exit 3) and
  `tool_error` (exit 6); §10, V8; §11, resource control and execution
  environment; §13, the "Execution environment" prerequisite row.
- [Roadmap](../roadmap.md): 1.2.4, its success criterion, and the downstream
  4.3.1 and 4.3.3.
- [ADR 0002](../adrs/adr-0002-first-profile-assurance-boundary.md), the
  assurance statement; [ADR 0003](../adrs/adr-0003-pin-the-e0-toolchain-tuple.md),
  the tuple and the `bwrap` open item;
  [ADR 0006](../adrs/adr-0006-separate-proof-integrity-from-operational-integrity.md),
  the separation of proof integrity from operational integrity.
- `contracts/profile.json` budgets: `wall_seconds` 60, `memory_bytes`
  2147483648, `result_bytes` 1048576, `concurrent_proof_jobs` 1, and
  `network` false.

The trace chain this plan preserves:

```plaintext
TOR-A4, TOR-SC6, TDD §11 offline       -> RM-1.2.4 -> EP-M2, EP-M3 -> AC1
TDD §11 2 GiB memory, TOR-A6           -> RM-1.2.4 -> EP-M2, EP-M3 -> AC2
TDD §11 1 MiB output, TDD §8 bounded   -> RM-1.2.4 -> EP-M1, EP-M2 -> AC3
TDD §11 kill and reap, TDD §10 V8      -> RM-1.2.4 -> EP-M2         -> AC4
TDD §11 private job directory          -> RM-1.2.4 -> EP-M2, EP-M3 -> AC5
TDD §11 missing isolation fails        -> RM-1.2.4 -> EP-M1, EP-M2 -> AC6
TDD §6.3, §11 real tools usable        -> RM-1.2.4 -> EP-M3         -> AC7
RM-1.2.4 "record a decision"           -> EP-M4 -> ADR 0007, TDD §§2, 11, 13 -> AC8
```

The acceptance items are:

- **AC1.** Under the selected boundary, the offline probes cannot reach a host
  loopback TCP listener, a host abstract Unix socket, or a host pathname Unix
  socket; under the negative control, each connects.
- **AC2.** Under the selected boundary, the memory probe is killed and the
  outcome is `MemoryLimit`, read from the scope's `Result=oom-kill`; with the
  memory limit lifted, the same allocation completes.
- **AC3.** A stream that exceeds its cap yields `OutputLimit`, keeps exactly the
  first cap bytes, and stops the job; a stream within the cap is kept whole.
  Verus proves the capture kernel's invariants for every chunk sequence.
- **AC4.** On a deadline and on explicit cancellation, the selected boundary
  leaves the job's scope empty and the direct child reaped, including the
  escapee; a process-group kill alone leaves the escapee running.
- **AC5.** Under the selected boundary, the job writes only to its private
  scratch area, which is bounded, invisible to the host, and gone afterwards;
  inputs are read-only. Under the negative control, the job writes a marker on
  the host.
- **AC6.** When `bwrap`, `systemd-run`, a working user manager, or a usable
  user namespace is missing, the supervisor returns `Unavailable` naming every
  missing control and runs nothing.
- **AC7.** Pinned OPA returns the same baseline decisions inside the boundary as
  outside, and its `http.send` probe cannot reach a host listener; pinned Lean
  checks the smoke theorem inside the 2 GiB and 60 s defaults.
- **AC8.** ADR 0007 records the decision, the intended platform, the evidence,
  the alternatives, and the residual trust; the technical design, guides, and
  roadmap agree with it.

## Constraints

- Keep the four claims in `AGENTS.md` separate. The probe concerns the
  execution environment only. Never describe a passing probe as proving,
  verifying, or certifying a policy, a translation, or a runtime.
- Do not use `unsafe`, and do not relax `unsafe_code = "forbid"`. Do not use
  `CommandExt::pre_exec`.
- Do not add a binary target, an item to `src/`, or a runtime dependency. Do
  not add a dev-dependency without escalating; the plan needs none. Everything
  is test-only, under `tests/toolchain/boundary/` and `verus/`.
- Launch every program by an explicit absolute path with a cleared environment
  and an argument array. Never build a shell command from a path or from data.
  Never resolve a program through `PATH`.
- Do not weaken a control to make a probe pass, and do not report a missing
  control as anything other than `Unavailable`. Missing enforced isolation is
  an execution failure, never a degraded mode (technical design §11).
- Hostile probe behaviours run only inside a supervised job. The hostile entry
  point must do nothing unless it finds the supervisor's behaviour file at the
  fixed in-sandbox input path, so an accidental `--ignored` run on the host is
  inert. Never write a fork bomb or an unbounded allocation; every hostile
  behaviour has a finite size that the negative control can survive.
- Do not change a toolchain pin, `toolchain-pins.json`, `lean/`, the Verus
  pins, or `examples/`.
- Do not use `assume`, `admit`, `external_body`, or any other trusted proof
  body in `verus/`.
- Do not change the contracts in `contracts/`. Naming the exhausted bound in a
  result is roadmap 4.3.1's work; this task records the mapping in the ADR.
- Do not modify the host's configuration. Installing bubblewrap from the
  distribution is an operator action that the user performs or authorizes;
  sysctl changes and AppArmor profiles are out of scope.
- Keep Rust modules under 400 lines and functions under 70 lines, with a
  module-level comment on every module and documentation on every item.
- Use British English with Oxford spelling in prose and identifiers' doc
  comments.

If satisfying the objective requires violating a constraint, stop. Record the
conflict in `Decision log`, set the status to `BLOCKED`, and escalate.

## Tolerances (exception triggers)

Exclude this ExecPlan from every count below. Count against the comparison base
`git merge-base main HEAD`, which is `ad217c8` when work starts, with:

```bash
git diff --numstat "$(git merge-base main HEAD)" -- . \
  ':(exclude)docs/execplans/1-2-4-enforceable-linux-execution-boundary-decision.md'
```

Each limit is **net lines** (additions minus deletions) and **files touched**,
and each category is enforced on its own:

- **Code:** Rust under `tests/`, Verus under `verus/`, Python under `scripts/`,
  `.feature` files, and `Makefile` recipe lines. The traced estimate is about
  2,900 net lines in about 22 files: the supervisor, launcher, platform, and
  outcome modules (about 900), the capture kernel with its annotations (about
  250), Verus specification and lemmas (about 300), hostile behaviours (about
  200), probes and tool runs (about 600), behavioural bindings (about 250),
  unit tests (about 350), and control-script entries and tests (about 80).
  Limit: **3,700 net lines and 28 files**. The contingency of about 25 per cent
  covers two review rounds, which is what the earlier plans needed.
- **Configuration:** `Cargo.toml`, `Cargo.lock`, `.config/`, `.github/`, and
  other declarative settings. Expected: none. Limit: **20 net lines and
  2 files**. Any new dependency is an exception whatever its size.
- **Documentation:** Markdown outside `docs/execplans/`, including the new ADR,
  and `AGENTS.md`. The traced estimate is about 650 net lines in 9 files: the
  ADR (about 280), technical design (about 90), developers' guide (about 120),
  and the users' guide, repository layout, contents, roadmap, terms of
  reference, and `AGENTS.md` (about 160 together). Limit: **900 net lines and
  11 files**.
- **Generated:** reviewed `insta` snapshots under `tests/toolchain/` are
  excluded from line counts and limited to **4 files**.

Other triggers:

- **Platform:** if bubblewrap cannot be installed on the probe host, if a
  required mechanism is unavailable there, or if the selected boundary fails
  any of AC1 to AC5, stop and escalate. That is the "unavailable enforcement
  blocks public acceptance" branch, and it needs an explicit design decision.
- **Flakiness:** if any probe fails intermittently in five consecutive
  `make boundary-probe` runs, stop and escalate. Do not add retries.
- **Duration:** if `make boundary-probe` takes longer than five minutes after
  the tools are provisioned, escalate before tuning.
- **Proof:** if the capture proof needs more than about 400 lines of
  specification and lemmas, or forces a change to the kernel's planned
  interface, escalate. Do not weaken the specification to fit.
- **Scope:** if the work needs a design element not described here, such as a
  seccomp filter, a custom init, a pinned bubblewrap build, or a change to the
  technical design beyond §§2, 8, 11, and 13, escalate.

## Risks

- **R1. Bubblewrap is absent on the probe host.** Mitigation: milestones EP-M1
  and EP-M2 need no `bwrap`, and EP-M3 starts only once the user has installed
  or authorized `bubblewrap` (`sudo dnf install bubblewrap`). Until then the
  bubblewrap probes report `Unavailable`, which exercises AC6 but not AC1 to
  AC5.
- **R2. systemd-run cannot find the user manager from a cleared environment.**
  `systemd-run --user` reaches the manager through `XDG_RUNTIME_DIR`.
  Mitigation: pass exactly `XDG_RUNTIME_DIR=/run/user/<uid>` to `systemd-run`,
  with the user ID read from the metadata of `/proc/self` rather than the
  environment, and let `bwrap --clearenv` empty the job's environment. If that
  fails, record it and escalate.
- **R3. A successful scope disappears before the supervisor reads it.**
  Transient scopes are removed once empty; only failed scopes stay loaded (O3).
  Mitigation: the memory outcome depends only on `Result=oom-kill`, which a
  failed scope keeps. Peak memory is not required here; roadmap 4.3.3 measures
  it. Each probe resets its failed unit afterwards.
- **R4. Bubblewrap's options differ from the planned set.** Mitigation: EP-M3
  starts with `bwrap --version` and a snapshot of the argument vector, and the
  platform check requires the tested version or later. Options absent from
  0.10.0 are a scope exception.
- **R5. The pinned Lean or OPA binary needs a host path the allowlist hides,**
  such as `/etc` files or `/proc` entries. Mitigation: AC7 exposes it. Add a
  read-only bind only for a named, justified path and record each one in the
  decision log; never bind `$HOME`, `/run`, or `/tmp`.
- **R6. Self re-execution of the test binary behaves differently inside the
  sandbox.** The libtest harness prints its own lines and starts threads.
  Mitigation: the hostile entry point is a single ignored test run with
  `--exact --test-threads=1 --nocapture`, and the probe's assertions tolerate
  harness output. The output-flood behaviour writes far more than the harness
  does.
- **R7. Probes interfere with other agents on the shared host.** Mitigation:
  every hostile behaviour is finite and bounded, unit names carry a per-run
  nonce, memory probes use a 64 MiB limit against a 256 MiB allocation, and
  every probe resets its failed units and removes its scratch directories.
- **R8. The intended platform is narrower than reviewers' machines.** Ubuntu
  24.04 and later restrict unprivileged user namespaces by default, and hosts
  without a systemd user session have no delegated cgroup. Mitigation: the ADR
  names the platform prerequisites and the failure each produces, and the
  platform check fails closed with that diagnostic. A second platform belongs
  to 4.3.3's clean-machine replay.
- **R9. The Verus proof costs more than the kernel deserves,** as the 1.2.3
  plan feared. Mitigation: the kernel is length-and-prefix arithmetic over byte
  sequences, and the tolerance above bounds the proof.

## Design

### The boundary under test

The supervisor is test-only Rust in `tests/toolchain/boundary/`. For each job
it creates a scratch directory holding a read-only input snapshot, then spawns
one host program with a cleared environment, a new process group
(`CommandExt::process_group(0)`, which is safe), standard input from
`/dev/null`, and both output streams piped. It reads each pipe on its own
thread through the capture kernel, polls the child against a single deadline,
and on a limit, a deadline, or a cancellation, kills the whole job and waits
until it is gone.

The selected configuration, `Bubblewrap`, launches this chain, in which each
program replaces the previous one or runs as its only child:

```plaintext
supervisor (test binary)
  -> /usr/bin/systemd-run --user --scope --quiet --unit=polean-probe-<nonce>
       -p MemoryMax=<limit> -p MemorySwapMax=0 -p TasksMax=<n>
       -p OOMPolicy=stop
     -> /usr/bin/bwrap --unshare-user --unshare-net --unshare-pid
          --unshare-ipc --unshare-uts --unshare-cgroup --disable-userns
          --die-with-parent --new-session --cap-drop ALL --clearenv
          --ro-bind /usr /usr  (and the distribution's /bin, /lib, /lib64
          symlinks into /usr)
          --ro-bind <tool or test-binary directory> <same path>
          --ro-bind <input snapshot> /job/in
          --size <storage bytes> --tmpfs /job/work
          --proc /proc --dev /dev --chdir /job/work
          --setenv <each named variable>
          -- <program> <arguments>
```

`systemd-run --scope` runs the command itself rather than starting a service,
so the supervisor's direct child stays the process it spawned. The scope
supplies the memory, swap, and task limits and an authoritative whole-tree
kill. Bubblewrap supplies the namespaces and an allowlisted filesystem: only
`/usr`, the program's own directory, the inputs, a private `tmpfs` scratch
area, and fresh `/proc` and `/dev` exist inside, so `$HOME`, `/run`, `/tmp`,
and every host socket are absent. `--die-with-parent` and the PID namespace
mean that killing the direct child also ends the job, which gives cancellation
a second, independent layer. Pages written to the `tmpfs` are charged to the
scope's memory, so storage is bounded by both `--size` and `MemoryMax=`.

Cancellation proceeds in a fixed order. The supervisor sends `SIGKILL` to the
job's process group, asks systemd to kill every process in the scope
(`systemctl --user kill --signal=SIGKILL --kill-whom=all`), waits for the
direct child, and then confirms that the scope has no processes left. Only
then does it join the pipe readers, which cannot block on a pipe held open by a
surviving grandchild, because none survives. The ADR states the roadmap's
"process-group cancellation" precisely: the process group is killed, but the
cgroup is the authority, because the escapee probe shows a group kill alone is
insufficient.

The negative-control configuration, `Unconfined`, keeps the scope and process
group but omits bubblewrap. Each probe runs under both configurations, and the
negative control must show the hostile behaviour succeeding. The memory and
output negative controls instead lift the specific limit, and the cancellation
negative control kills only the process group.

The util-linux composition from O4 and O5 is not built into the probe. The ADR
records it as a rejected alternative with the planning transcript and the
exact command, because its pathname-socket leak is reproducible by hand and a
second configuration in code would be compatibility theatre.

### Hypotheses

- **H1.** Under `Bubblewrap`, AC1 to AC5 hold on the intended platform.
- **H2.** A network namespace does not, by itself, block pathname Unix
  sockets (O5); the allowlisted filesystem view does.
- **H3.** Killing the process group leaves an escapee running, and killing the
  scope does not.
- **H4.** The pinned OPA and Lean binaries work inside `Bubblewrap` within the
  default limits, with no host path beyond `/usr`, their own directories, and
  their inputs.
- **H5.** When any required mechanism is missing, the platform check reports
  `Unavailable` before anything runs.

Any falsified hypothesis is recorded in `Surprises & discoveries` and the
decision log. A falsified H1, H4, or H5 triggers the platform tolerance.

### Outcomes and their mapping

The probe's pure kernel `classify` turns the supervisor's observations into one
of these outcomes, with the bound named:

- `Completed`: the child exited with status 0, no limit fired, and the scope is
  empty.
- `ChildFailed`: the child exited non-zero or by a signal without a limit or
  cancellation firing.
- `MemoryLimit`, `OutputLimit`, `Deadline`, or `Cancelled`: the named bound or
  request ended the job and the scope is empty. Several can fire together, and
  the outcome lists all of them.
- `ContainmentFailure`: after cancellation the scope still holds a process, or
  the direct child could not be reaped. This outranks every other outcome.
- `Unavailable`: the platform check found a required control missing, and
  nothing ran.

The ADR records how roadmap 4.3.1 maps these onto the contract in
`contracts/cli.txt`: a named bound is `unknown`, exit 3, which the contract
already describes as "not settled, including resource exhaustion";
`Unavailable`, `ContainmentFailure`, and an unexpected `ChildFailed` are
`tool_error`, exit 6. No contract changes in this task.

### Intended platform

The decision names one intended platform for public acceptance: Linux on
x86_64 with glibc, cgroup v2 with a systemd user manager that delegates the
`memory` and `pids` controllers, unprivileged user namespaces usable by
bubblewrap, and bubblewrap from the distribution at the tested version or
later. The tested instance is the O1 host after `bubblewrap` is installed. The
ADR lists each prerequisite with the `Unavailable` reason that names it.
Hosted CI is not an intended platform, and `make boundary-probe` does not run
there.

This narrows the terms of reference's open supported-platform matrix (§8.1)
only for execution enforcement; it does not authorize a supported-platform
commitment, which remains open.

## Verification plan

### Obligations and methods

The supervisor enforces nothing itself except the output cap and the deadline;
the kernel and systemd enforce the rest. The obligations therefore split into
three kinds: a small pure kernel that Polean owns and must get right for every
input, finite decision logic, and external behaviour that only real processes
can demonstrate.

**V-CAP (capture kernel, Verus).** The capture kernel consumes chunks of one
output stream and decides what to keep. Its invariants, for every finite
sequence of chunks:

1. **I1, bound:** the retained length never exceeds the cap.
2. **I2, prefix:** the retained bytes equal the first `min(seen, cap)` bytes of
   the concatenated stream, where `seen` is the number of bytes offered.
3. **I3, trip:** the kernel reports overflow exactly when `seen > cap`.
4. **I4, monotone:** once overflow is reported it stays reported, and the
   retained bytes never change again.

Method: Verus, because I2 quantifies over sequences of unbounded length and the
kernel is a small, stable, pure function that the supervisor uses as a security
bound. Artefacts: the kernel in `tests/toolchain/boundary/capture/mod.rs`
inside `verus!`, `verus/boundary_capture_spec.rs`,
`verus/boundary_capture_lemmas.rs`, and the entry `verus/boundary_capture.rs`.
Command: `make verus`. Discharge: the proof verifies with zero errors and no
trusted bodies, and every new negative control fails as named.

**V-CLS (outcome classification, exhaustive enumeration).** `classify` never
returns `Completed` when any limit, deadline, or cancellation fired or when the
scope is non-empty; it returns `ContainmentFailure` whenever the scope is
non-empty after cancellation; and it names every bound that fired. Its input is
a finite product of a few Booleans and an exit form drawn from a fixed set of
representatives. Method: an `rstest` test that enumerates every combination and
checks each property against an independently written oracle. Enumeration of a
finite domain is complete, so Verus would add ceremony without rigour.

**V-AVL (fail-closed availability, exhaustive enumeration).** The platform
verdict is `Ready` exactly when all five controls are available, and otherwise
`Unavailable` listing every missing control. Method: enumeration of all 32
combinations, plus named tests of the real check with a missing `bwrap` path
and a missing `systemd-run` path, which run under `make test` on any host.

**V-ARGV (launcher arguments, snapshot).** The `Bubblewrap` argument vector for
a canonical job contains every isolating option and never binds `$HOME`,
`/run`, `/tmp`, or `/`. Method: an `insta` snapshot reviewed by hand, plus
`googletest` matchers for the forbidden binds, so a change to the vector is a
reviewed diff.

**E1 to E5 (enforcement, end-to-end probes).** Each property is demonstrated
by a hostile behaviour under `Bubblewrap` with a negative control, as AC1 to
AC5 describe. Method: ignored integration tests under `make boundary-probe`,
mirrored by behavioural scenarios. They are the only evidence for the external
behaviour, and they are run on the intended platform.

**E6 (real tools).** AC7. Method: ignored integration tests that reuse the
smoke tests' OPA and Lean invocations inside `Bubblewrap`.

### External assumptions (axioms)

The reasoning depends on the following documented behaviour, which the
probes exercise rather than prove:

1. The Linux kernel isolates network, mount, PID, IPC, and UTS resources per
   namespace as `namespaces(7)` documents, including abstract Unix sockets per
   network namespace.
2. Killing a PID namespace's init kills every process in that namespace.
3. cgroup v2 enforces `memory.max`, `memory.swap.max`, and `pids.max`, charges
   `tmpfs` pages to the writer's cgroup, and kills every member on
   `cgroup.kill`, whatever their process group or session.
4. systemd applies `MemoryMax=`, `MemorySwapMax=`, `TasksMax=`, and
   `OOMPolicy=` to transient scopes, records `Result=oom-kill`, and keeps a
   failed scope loaded until reset.
5. Bubblewrap 0.10.0 implements its documented options, in particular
   `--die-with-parent`, `--disable-userns`, `--clearenv`, `--size`, and the
   bind options.
6. `SIGKILL` cannot be caught or ignored, and a pipe reaches end-of-file once
   every write end is closed.

A kernel or bubblewrap defect that breaks one of these is outside the probe's
reach and is recorded as residual trust in the ADR and in technical design §2.

### Non-vacuity

- V-CAP: the Verus witnesses construct at least one chunk sequence that stays
  under the cap, one that lands exactly on it, and one that crosses it inside a
  chunk, so no precondition excludes the interesting cases. Three new controls
  in `scripts/check_verus_controls.py` must each fail: keeping the whole chunk
  instead of clamping it (breaks I1), tripping on `seen >= cap` (breaks I3),
  and a weakened specification that drops the prefix clause (the control's
  witness lemma must fail to verify). `make verus-controls` reports them.
- V-CAP, executable side: a `proptest` property splits random streams into
  random chunks and compares the retained bytes with the oracle
  `stream[..min(len, cap)]`, recording how many cases crossed the cap so that a
  generator that never overflows is noticed.
- V-CLS and V-AVL: the enumeration asserts that every outcome and both verdicts
  occur at least once. A deliberate mutation of `classify`, letting a fired
  limit return `Completed`, must fail the test; this is checked by hand once
  during EP-M1 and recorded in `Surprises & discoveries`.
- E1 to E5: each property's negative control must show the hostile behaviour
  succeeding on the same host in the same run. A probe whose negative control
  does not succeed fails, so the pair cannot pass vacuously.
- E6: the inside-boundary OPA decisions are compared with outside-boundary
  decisions for both a `true` and a `false` request, so a constant result
  cannot pass.

### Proof sketch for V-CAP

The abstract model is a state `(cap, kept, seen, tripped)`, where `kept` is a
sequence of bytes, `seen` a natural number, and `tripped` a Boolean, with the
initial state `(cap, [], 0, false)`. Feeding a chunk `c` produces
`kept' = kept + c[0..k]`, where `k = min(|c|, cap - |kept|)`,
`seen' = seen + |c|`, and `tripped' = tripped || seen' > cap`. The executable
kernel stores `seen` as a `u64` with saturating addition and `kept` as a
`Vec<u8>` whose view is `kept`.

The specification relates the state to the whole stream `s`, the concatenation
of every chunk fed so far: `kept == s[0..min(|s|, cap)]` and
`tripped == (|s| > cap)`.

The lemmas expected to carry the proof are:

1. **Prefix extension:** for sequences `s` and `c` and a cap `n`,
   `(s + c)[0..min(|s| + |c|, n)] == s[0..min(|s|, n)] + c[0..k]` with
   `k = min(|c|, n - min(|s|, n))`. This is the inductive step for I2.
2. **Saturation preserves the comparison:** if `seen` saturates at `u64::MAX`
   and the cap is below it, then `saturating_add(seen, x) > cap` exactly when
   the true sum exceeds the cap, so I3 survives saturation.
3. **Trip monotonicity:** once `|s| > cap`, every extension keeps it so, and
   `k` is zero, which gives I4.

These lemma statements are hypotheses about the proof's shape. The kernel's
interface, a constructor taking the cap and a `feed(&mut self, chunk: &[u8])`
returning the number of bytes kept and whether it tripped on this call, was
chosen so that each call is one step of the model. If verification shows a
lemma unnecessary or needing decomposition, record the change in the decision
log rather than weakening the specification.

## Test strategy

Unit tests use `rstest` fixtures and tables, `googletest` matchers, and
`pretty_assertions` for structural comparisons:

- `boundary::capture`: named cases for an empty stream, a stream exactly at the
  cap, one byte over, a crossing inside a chunk, and many empty chunks; the
  `proptest` property above.
- `boundary::outcome`: the V-CLS and V-AVL enumerations, and named regression
  cases for precedence, such as a deadline and a memory kill together.
- `boundary::launch`: the V-ARGV snapshot and forbidden-bind matchers.
- `boundary::platform`: fail-closed tests with a missing `bwrap` and a missing
  `systemd-run`, which run on every host because they spawn nothing.

End-to-end probes are ignored tests under `make boundary-probe`, which depends
on `make tools` for E6. Each runs a real supervised job. Behavioural scenarios
bind the main outcomes with `rstest-bdd`. Proposed feature file,
`tests/features/execution_boundary.feature`:

```gherkin
Feature: Enforced Linux execution boundary
  Polean runs each external tool under an enforced boundary, or not at all.

  Scenario: A job cannot reach the host network or host sockets
    Given the platform provides every required control
    And the host listens on a loopback TCP port, an abstract socket, and a socket file
    When a hostile job tries to connect to each of them
    Then every connection attempt fails inside the job
    And no host listener records a connection

  Scenario: A job that exceeds its memory limit is stopped and named
    Given the platform provides every required control
    When a job allocates 256 MiB under a 64 MiB limit
    Then the outcome is a memory limit
    And the job's scope holds no process

  Scenario: A job that floods its output is stopped at the cap
    Given the platform provides every required control
    When a job writes 2 MiB to standard output under a 1 MiB cap
    Then the outcome is an output limit
    And exactly the first 1 MiB of the output is kept

  Scenario: Cancellation reaches a process that left the process group
    Given the platform provides every required control
    And a job has started a grandchild in a new process group
    When the job's deadline expires
    Then the outcome is a deadline
    And the job's scope holds no process

  Scenario: A job writes only to its private scratch area
    Given the platform provides every required control
    When a job writes to its scratch area, its inputs, and the host
    Then only the scratch write succeeds
    And no file from the job remains on the host afterwards

  Scenario: Missing enforcement refuses to run the job
    Given bubblewrap is not at its configured path
    When a job is submitted
    Then the outcome is unavailable, naming bubblewrap
    And no process is started
```

The last scenario runs under `make test` on every host; the others are bound to
ignored tests. Where a scenario's step needs the negative control, the step
definition runs it in the same scenario and asserts it succeeded.

`insta` is used only where a reviewed, multivariant rendering matters: the
`Bubblewrap` argument vector and the probe's property matrix of configurations
against properties, rendered as a small text table. `kani` is not used: the
finite logic is enumerated exhaustively, and the one unbounded property is
proved in Verus.

## Milestones

### EP-M1: pure kernels, proof, and fail-closed verdict

Requirements: AC3 (kernel), AC6 (verdict), and the V-CAP, V-CLS, V-AVL, and
V-ARGV obligations. End state: `tests/toolchain/boundary/` exists with
`mod.rs`, `capture/mod.rs`, `outcome.rs`, `platform.rs`, `launch.rs`, and
`tests.rs`; the toolchain binary declares `mod boundary;`; the Verus entry,
specification, lemmas, and controls exist, and `VERUS_PROOF_FILES` includes
`verus/boundary_capture.rs`. Nothing spawns a sandboxed process yet.

Work test first. Add the named capture cases and the enumeration tests, run
`cargo test --test toolchain boundary::` and watch them fail to compile or fail
for the missing behaviour, then implement until they pass. Write the Verus
specification and lemma statements before the kernel body, run `make verus`,
and add the controls last.

Acceptance: `make check-fmt`, `make typecheck`, `make lint`, `make test`, and
`make verus` pass; `make test-scripts` passes under Python 3.14 after the
control-script change; the mutation check for `classify` is recorded.
Conformance check: no binary target, dependency, or `unsafe`; the kernel
interface matches the proof sketch. Recovery: the milestone is additive and can
be reverted as one commit range. Remaining gaps: no real process yet.

### EP-M2: supervisor, hostile behaviours, and negative controls (prototyping)

Requirements: the supervisor half of AC1 to AC5, AC4 in full, AC6 end to end.
End state: `supervisor.rs`, `hostile.rs`, and `probes.rs` exist; the hostile
entry point is an ignored test named `boundary::hostile_entry`, inert outside
a job; `make boundary-probe` exists and runs `boundary::probe` tests. Under
`Unconfined`, every negative control shows its hostile behaviour succeeding,
and the cancellation probe shows the escapee surviving a process-group kill and
dying when the scope is killed. Under `Bubblewrap`, if `bwrap` is absent, the
probes report `Unavailable`, which the probe asserts when an explicit
`expect unavailable` variable is set in the Makefile invocation, and fails
otherwise.

This milestone is labelled prototyping because it settles the supervisor's
shape. Keep it test-only and additive. Promote nothing into `src/`.

Acceptance: the M1 gates, plus `make boundary-probe` with every negative
control green on the O1 host. Conformance check: the supervisor uses only safe
`std`, explicit paths, and cleared environments; no host configuration
changed. Recovery: revert the milestone's commits. Remaining gaps: the
selected configuration is unproven until bubblewrap is installed.

### EP-M3: the selected boundary and the real tools

Precondition: the user has installed bubblewrap on the probe host, or
authorized its installation. If not, stop here, set the status to `BLOCKED`,
and report.

Requirements: AC1, AC2, AC4, AC5, and AC7 under `Bubblewrap`, and H1 to H5.
End state: every `Bubblewrap` probe passes beside its negative control;
`tools.rs` runs pinned OPA on the baseline policy for a `true` and a `false`
request inside and outside the boundary, runs an OPA query that calls
`http.send` against a host listener and observes no connection, and runs the
smoke theorem through pinned Lean under the 2 GiB, 60 s defaults; the property
matrix snapshot records the result; behavioural scenarios are bound.

Acceptance: the M1 gates, `make toolchain-smoke`, and five consecutive green
`make boundary-probe` runs, with the total duration recorded. Conformance
check: every extra read-only bind is named in the decision log; nothing binds
`$HOME`, `/run`, or `/tmp`. Recovery: revert to the M2 plateau, which is
coherent on its own. Remaining gaps: one platform instance only.

### EP-M4: the decision record and documentation

Requirements: AC8. End state:

- `docs/adrs/adr-0007-enforce-the-execution-boundary-with-a-user-scope-and-bubblewrap.md`
  follows ADR 0006's shape: Status (`Proposed` until the user accepts it in
  conversation), Date, Context and problem statement, Decision drivers,
  Decision, Consequences, Measured results with the probe matrix and the tool
  timings, Alternatives considered (the util-linux composition with O5's
  transcript and command, Landlock alone, `RLIMIT_AS`, Podman and `crun`, and a
  repository-owned launcher), Known risks and limitations, Outstanding
  decisions, Acceptance, and References.
- `docs/technical-design.md`: §2's "Evidence to reported result" row names the
  kernel, systemd, and bubblewrap as residual trust; §11 states the mechanism,
  the intended platform, the cancellation order, and the outcome mapping; §13's
  "Execution environment" row records that 1.2.4 is met for public release by
  ADR 0007 on the named platform, with 4.3.1 owning promotion; new references
  from R19 for bubblewrap, `systemd.resource-control(5)`, the kernel's cgroup v2
  documentation, `namespaces(7)`, `network_namespaces(7)`, and Go issue 38010;
  ADR 0007 joins the companions.
- `docs/terms-of-reference.md`: a `[KNOWN]` note under §8.2 after A6, in the
  style of the A5 note, stating what the probe established for execution
  enforcement and that A4 still needs a clean-machine replay.
- `docs/developers-guide.md`: a section "The execution-boundary probe (1.2.4)"
  with the prerequisites, `make boundary-probe`, how to read the property
  matrix, how to add a hostile behaviour safely, and a reuse strategy for
  4.3.1, which promotes the supervisor, kernels, and launcher and keeps the
  probes as its acceptance suite.
- `docs/users-guide.md`: under the planned workflow, the platform
  prerequisites that public `prove` and `verify` will need, labelled as
  planned.
- `docs/repository-layout.md`, `docs/contents.md`, and `AGENTS.md`'s gate list
  name the new module, ADR, plan, and target.
- `docs/roadmap.md`: 1.2.4 is marked done with a short "Recorded" note, once the
  user has accepted the ADR.

Acceptance: `make check-fmt`, `make lint`, `make test`, `make markdownlint`, and
`make design-check` pass. Conformance check: every upstream link in
`Conformance basis` is current, and no document claims more than the probe
showed. Recovery: documentation commits revert independently.

## Concrete steps

Run every command from the repository root. Write gate output to a log, for
example:

```bash
make lint 2>&1 | tee /tmp/lint-polean-1-2-4-enforceable-linux-execution-boundary-decision.out
```

1. Confirm the branch with `git branch --show-current`.
2. For EP-M1, add the module skeleton and red tests, then:

   ```bash
   cargo test --locked --all-features --test toolchain boundary::
   make check-fmt && make typecheck && make lint && make test
   make verus
   make test-scripts
   ```

   Expected: the new unit tests pass, `make verus` reports the new entry point
   verified with zero errors, and `make verus-controls` lists the three new
   controls as rejected.
3. For EP-M2, add the supervisor and probes, then:

   ```bash
   make boundary-probe
   ```

   Expected before bubblewrap is installed: every `Unconfined` negative control
   passes, and every `Bubblewrap` probe reports `Unavailable` naming
   bubblewrap.
4. Before EP-M3, the user runs `sudo dnf install bubblewrap`. Then:

   ```bash
   /usr/bin/bwrap --version
   make toolchain-smoke
   for run in 1 2 3 4 5; do make boundary-probe || break; done
   ```

   Expected: `bubblewrap 0.10.0`; every probe passes in all five runs.
5. For EP-M4, write the ADR and documentation, then run the documentation and
   Rust gates listed above.
6. Commit after each green step with an imperative subject and a body that
   explains the reason and the assurance impact.

## Skills and references

Load `execplans` to maintain this plan, `rust-router` first for any Rust
question, then `rust-unit-testing` for fixtures and matchers, `proptest` for
the capture property, `verus` for the proof, `rust-verification` if the method
choice is questioned, and `domain-cli-and-daemons` for process lifecycle and
shutdown. Use `arch-decision-records` for the ADR and `en-gb-oxendict-style`
for prose. Use `firecrawl-mcp` for any further external documentation. The
CodeGraph server was unavailable during planning; use it when it is.

Read these repository documents first: `docs/technical-design.md` §§2, 8, 10,
11, and 13; `docs/terms-of-reference.md` §§7 and 8;
`docs/adrs/adr-0006-separate-proof-integrity-from-operational-integrity.md` as
the ADR template; `docs/developers-guide.md` on tests, Verus, and toolchain
observations; `docs/reliable-testing-in-rust-via-dependency-injection.md` for
the injected platform check and paths;
`docs/rust-testing-with-rstest-fixtures.md`; `docs/rust-doctest-dry-guide.md`;
`docs/complexity-antipatterns-and-refactoring-strategies.md` for keeping the
supervisor's functions small; `docs/scripting-standards.md` for the control
script; and `docs/documentation-style-guide.md`. There is no
`docs/references.md`; references live in the technical design's `References`
section and in each ADR.

## Progress

- [x] 2026-11-14: renamed the branch and set its upstream to
  `origin/1-2-4-enforceable-linux-execution-boundary-decision`.
- [x] 2026-11-14: surveyed the repository, earlier plans, and documents with a
  reconnaissance team; probed the host (O1 to O6); consulted prior art.
- [x] 2026-11-14: drafted revision 1 of this plan.
- [ ] Expert review of the design, and revision.
- [ ] User approval.
- [ ] EP-M1.
- [ ] EP-M2.
- [ ] Bubblewrap installed on the probe host.
- [ ] EP-M3.
- [ ] EP-M4, and the user's acceptance of ADR 0007.

## Surprises & discoveries

- 2026-11-14: a network namespace with Landlock still reached the D-Bus session
  bus through its socket file (O5). Offline execution needs the filesystem
  view, not only the network namespace.
- 2026-11-14: a repository-owned launcher is ruled out twice over, by
  `unsafe_code = "forbid"` for `pre_exec` and by the binary-target guard in
  `tests/documentation_examples/scope.rs`, which steered the design to host
  programs.

## Decision log

- **D1, 2026-11-14:** compose a systemd user scope with bubblewrap, rather than
  util-linux `unshare` with Landlock, Podman or `crun`, or a repository-owned
  launcher. Rationale: bubblewrap builds an allowlisted filesystem view from
  arguments, which closes O5 without a repository-owned mount script; it is
  mature, already expected by `lake check`, and needs no `unsafe` or binary
  target; the scope supplies limits and a whole-tree kill that bubblewrap does
  not. Subject to expert review and the probe.
- **D2, 2026-11-14:** limit memory through cgroups, not `RLIMIT_AS`, because
  the Go runtime behind OPA reserves address space at start-up and Lean's
  runtime may too; a cgroup limit measures memory actually used.
- **D3, 2026-11-14:** prove the capture kernel in Verus and enumerate the
  finite decision logic; do not prove the supervisor's plumbing.
- **D4, 2026-11-14:** keep the util-linux composition out of the code and
  record it in the ADR with its reproducible command.

## Outcomes & retrospective

Not started.

## Revision notes

- Revision 1, 2026-11-14: initial draft for expert review.
