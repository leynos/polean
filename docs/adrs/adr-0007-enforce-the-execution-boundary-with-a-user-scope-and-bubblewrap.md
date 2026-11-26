# Architectural decision record (ADR) 0007: enforce the execution boundary with a systemd user scope and bubblewrap

## Status

Proposed. The probe this record rests on is implemented and passes on the probe
host; the record awaits the project sponsor's acceptance in conversation. It
settles the mechanism roadmap 1.2.4 asked for and leaves the supported-platform
matrix (ToR §8.1) and the agreed measurement machine (OQ4, roadmap 4.3.3) open.

## Date

2026-11-22.

## Context and problem statement

Technical design §11 requires every external program Polean runs — the pinned
Lean toolchain and the pinned Open Policy Agent (OPA) binary — to work offline,
inside memory and output limits, in private storage, with cancellation killing
and reaping every process the job started. It also requires that missing
enforced isolation be an execution failure for the first public profile, never
a silently degraded mode. Roadmap 1.2.4 asked whether that boundary can be
enforced on a Linux host, and required a bounded probe to answer before public
acceptance.

The probe host is Rocky Linux 10.2 on kernel 6.12, systemd 257, util-linux
2.40.2, glibc 2.39, x86_64, six cores, with cgroup v2 and unprivileged user
namespaces available. The caller runs inside a user service, so a new scope is
a sibling in `app.slice`. The security modules are
`lockdown,capability,yama,selinux,bpf,landlock,ima,evm`; SELinux is permissive
and the caller is unconfined; Landlock reports ABI 6; `systemd-oomd` is
inactive. Bubblewrap 0.10.0 is installed from the distribution.

Two planning observations shaped the alternatives. A network namespace with a
Landlock rule set still connected to the user's D-Bus session socket at
`/run/user/1000/bus`: a network namespace isolates abstract Unix sockets, not
socket files, and Landlock ABI 6 does not mediate connecting to an existing
socket file. Through that bus a job could ask the user's systemd manager to
start a process outside every limit. The filesystem view, not only the network
namespace, has to hide host sockets. Separately, the Go runtime reserves
address space at start-up and its maintainers describe `RLIMIT_AS` as an
outdated way to limit memory (Go issue 38010), so OPA cannot be limited through
an address-space limit.

## Decision drivers

- The boundary must hold for an untrusted Rego source and a trusted but large
  Lean toolchain, without repository-owned mount, namespace, or supervisor code
  that would need its own proof.
- The mechanism must need no `unsafe` and no repository binary target: the
  crate forbids `unsafe`, and roadmap 2.1.4 owns the application edge.
- Limits must measure memory used, not address space, and must take the whole
  process group when cancellation or an out-of-memory kill fires.
- A job must not reach any host socket, including pathname sockets on the
  filesystem.
- A missing or ineffective mechanism must fail closed with a named reason.
- The pinned tools must run inside the boundary unchanged.

## Decision

Run each child in a **transient systemd user scope**, which supplies cgroup v2
memory, swap, and task limits and a whole-tree kill, and inside **bubblewrap**,
which supplies user, network, mount, PID, IPC, and UTS namespaces and an
allowlisted, read-only filesystem view.

The launch chain is:

```plaintext
systemd-run --user --scope --quiet --unit=<unit>
  -p MemoryMax=<bytes> -p MemorySwapMax=0 -p TasksMax=<n>
  -p OOMPolicy=kill -p KillSignal=SIGKILL -p RuntimeMaxSec=<deadline plus grace>
  -- bwrap --unshare-user --unshare-net --unshare-pid --unshare-ipc
       --unshare-uts --unshare-cgroup --disable-userns --die-with-parent
       --new-session --cap-drop ALL --clearenv
       --ro-bind /usr /usr (and /bin, /lib, /lib64 as symlinks into /usr)
       --ro-bind <program file or tool directory> <fixed path>
       --ro-bind <input snapshot> /job/in
       --proc /proc --dev /dev --remount-ro /dev
       --size <scratch bytes> --tmpfs /job/work --chdir /job/work
       --setenv <each named variable>
       --remount-ro /
       -- <program> <arguments>
```

`systemd-run --scope` runs the command itself, so the supervisor's direct child
is the job. The scope supplies the limits, the whole-group out-of-memory kill,
a backstop deadline that survives the supervisor's death, and `cgroup.kill` as
the authoritative whole-tree kill. Bubblewrap supplies the namespaces and an
allowlisted filesystem: only `/usr`, the bound program, the input snapshot, a
private size-limited scratch area, and fresh `/proc` and `/dev` exist inside, so
`$HOME`, `/run`, `/tmp`, and every host socket are absent, and the root is
read-only. `/dev` is remounted read-only as well, because `--dev` mounts a
writable tmpfs that includes `/dev/shm`, which would otherwise be a second
writable area outside the `--size` bound. Standard input is `/dev/null` and
`--new-session` closes the terminal-injection path that `legacy_tiocsti=1`
leaves open. No-new-privileges, the dropped capabilities, the unmapped real
root, and the PID namespace neutralize set-user-ID programs and hide host
processes from `ptrace` and signals.

The supervisor is test-only Rust under `tests/toolchain/boundary/`. For each
job it spawns the chain with a cleared environment, a new process group,
standard input from `/dev/null`, and both streams piped; it drains each stream
on its own thread through a bounded capture, polls one deadline, and then tears
the job down in a fixed order: kill the process group, record any survivor,
write `cgroup.kill`, reap, confirm the cgroup empty, read systemd's `Result=`,
and reset only the exact failed unit. A process that outlived the process-group
kill is reported separately from a non-empty cgroup, so the probe distinguishes
"the group kill was sufficient" from "the cgroup was the authority".

The enforcement prerequisites are, for any host on which Polean claims enforced
isolation:

1. Linux with cgroup v2 unified hierarchy.
2. A systemd user manager reachable through `XDG_RUNTIME_DIR` that delegates the
   `memory` and `pids` controllers.
3. `systemd-oomd` inactive, or confirmed not to have caused a reported kill.
4. Bubblewrap 0.10.0 or later from the distribution.
5. A host policy — AppArmor, SELinux, or sysctl — that permits bubblewrap to
   create its namespaces.

The tested instance is the probe host after bubblewrap was installed. Ubuntu
24.04 and later restrict unprivileged user namespaces through AppArmor by
default and fail closed until the restriction is lifted or a profile permits
`bwrap`; so do hosts without a systemd user session or with no delegated
controllers. Hosted CI is not an enforcement platform and does not run the
probe.

## Consequences

The probe shows all five properties enforced on the probe host, each beside a
negative control that shows the hostile behaviour succeeding without the
boundary:

| Property          | Selected boundary (observed)                                                                                                                                         | Negative control (observed)                                          |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| Offline execution | TCP refused; abstract and socket-file UNIX connections refused or absent; no listener recorded a connection; the canary's probe-owned listener silent; fds `[0,1,2]` | All three listeners accepted; the job held its standard streams only |
| Memory limit      | Allocation and scratch fill both `oom-kill`, scope empty                                                                                                             | 256 MiB allocation completed under a lifted limit                    |
| Output limit      | Exactly the first 1 MiB kept; job stopped                                                                                                                            | 2 MiB flood kept whole under a lifted cap                            |
| Cancellation      | Deadline or explicit request named; escapee removed by the PID namespace; cgroup empty                                                                               | Escapee outlived the group kill; only `cgroup.kill` removed it       |
| Private storage   | Only the scratch write succeeded; fill failed with `ENOSPC`; no host file                                                                                            | Marker written into the probe's host directory                       |

_Table 1: The probe's property matrix on the probe host._

Pinned OPA decided the same `true` and `false` queries inside and outside the
boundary, an `http.send` query reached no host listener, and pinned Lean checked
`PoleanSmoke.lean` inside the 2 GiB and 60 s defaults.

The cost is a dependency on bubblewrap and a systemd user manager, plus the
residual trust the next section records. A host that lacks either fails closed
with a named reason rather than running without isolation, which is the
behaviour technical design §11 requires.

## Observed results

Single runs on the probe host on 2026-11-22, through `make boundary-probe`.
Wall times include process start-up and, for the listener cells, a bounded
silence window; they are observations, not the budgets roadmap 4.3.3 measures.

| Probe                     | Observation                                              | Wall time |
| ------------------------- | -------------------------------------------------------- | --------- |
| Offline, selected         | Listeners silent; fds `[0,1,2]`; TCP `ECONNREFUSED`      | 6.1 s     |
| Memory, allocation        | `oom-kill` under 64 MiB; cgroup empty                    | 1.6 s     |
| Memory, scratch fill      | `oom-kill` with 128 MiB scratch under a 64 MiB limit     | 1.3 s     |
| Output                    | Exactly 1 MiB kept; `OutputLimit`                        | 0.9 s     |
| Escapee, selected         | Deadline named; empty survivor list and cgroup           | 1.9 s     |
| Storage                   | Scratch only; `ENOSPC`; no host file                     | 1.2 s     |
| Canary and platform       | `Ready`; limits read back; fds `[0,1,2]`                 | 1.0 s     |
| OPA, inside and outside   | Same boolean for a `true` and a `false` request          | 6.3 s     |
| OPA, `http.send`          | Listener silent; non-zero exit                           | 6.3 s     |
| Lean smoke                | Checked inside 2 GiB and 60 s                            | 5.8 s     |
| Offline, negative control | All listeners accepted                                   | 1.3 s     |
| Escapee, negative control | Survivor outlived the group kill; cgroup kill removed it | 2.6 s     |

_Table 2: Single-run observations from `make boundary-probe`._

Repeated `make boundary-probe` runs each passed all twenty-one probes in 30–41
s on the shared development host. A loopback TCP address inside the empty
network namespace reports `ECONNREFUSED` rather than `ENETUNREACH`, because the
loopback route exists even with `lo` down; the canary's probe-owned listener is
then refused or unreachable, and the offline probe accepts either.

## Alternatives considered

**A systemd user service with private namespaces.** No bubblewrap install and
no inherited descriptors, because the service manager starts the job. Its
filesystem view is a denylist built from service settings, so a host path not
named in the denylist stays visible; the allowlist bubblewrap builds is the
stronger property for hiding sockets. It also puts the launch chain behind the
service manager's own start-up, which the supervisor would have to observe.

**Rootless `crun` with a repository-owned OCI configuration.** Declarative and
already installed (crun 1.30.1, Podman 5.8.2). It brings container state and
storage to manage, starts more slowly, and the OCI configuration is another
repository-owned artefact whose view must be reviewed; bubblewrap's argument
vector is smaller and its filesystem view is the same construction Flatpak uses.

**The util-linux composition.**
`unshare --user --map-root-user --net --mount
--pid --ipc --uts --fork --kill-child --mount-proc`,
wrapped in `setpriv --no-new-privs` with a Landlock rule set allowing writes
only beneath a private directory, was exercised by hand. It saw three processes
and only the loopback interface, failed an outbound connection with
`Network is unreachable`, and could not write to `$HOME` or `/tmp`. It
**connected** to the D-Bus session socket at `/run/user/1000/bus`: a network
namespace does not isolate pathname sockets and Landlock ABI 6 does not mediate
connecting to one. It is kept out of the code for that reason. Reproducing it
needs `unshare`, `setpriv`, and a Landlock rule set expressed through a helper;
the record keeps the observation, not a launcher.

**Landlock alone.** On the recorded Linux 6.12 host, ABI 6 can restrict
filesystem access and also TCP connections, abstract Unix sockets, and signals,
but it still has no PID or cgroup control and no way to hide a pathname socket
file from a connect; it is useful as a second layer, not as the boundary.

**`RLIMIT_AS`.** Limits address space, not memory used. The Go runtime reserves
address space at start-up and the Go maintainers describe the limit as outdated
(Go issue 38010), so OPA would be limited in a way unrelated to its real memory
use.

**Podman.** A daemon-less but container-oriented runtime with its own storage
and state. It is heavier than the job needs and adds a second configuration
surface for the same kernel mechanisms.

**A repository-owned launcher binary.** Would put the mount, namespace, and
supervisor code under `unsafe` or a new dependency. `unsafe_code = "forbid"`
and the binary-target guard in `tests/documentation_examples/scope.rs` rule it
out, and roadmap 2.1.4 owns the application edge.

**Bubblewrap's `--json-status-fd` for set-up failure.** Passing an extra
descriptor needs `pre_exec`, which the unsafe ban forbids, and sharing standard
error would let the child forge status lines. The canary detects set-up failure
instead.

**A bubblewrap network namespace alone.** The D-Bus observation above shows it
does not hide socket files; the allowlisted filesystem view does.

## Known risks and limitations

- **Residual trust.** The kernel, systemd, and bubblewrap implement the
  properties; the probe observes them, it does not prove them. A defect in any
  of the three is outside the probe's reach and is recorded in technical design
  §2 as remaining trust.
- **One host.** The probe ran on one Rocky Linux host. Ubuntu 24.04 and later
  fail closed under their default AppArmor restriction; a second platform
  belongs to roadmap 4.3.3.
- **Tool jobs and the scope observation.** A pinned tool can finish in about ten
  milliseconds, before the supervisor's first successful scope poll, and
  systemd garbage-collects a successful scope immediately. Hostile probes hold
  a 250 ms observation window after their start line, which makes the
  observation deterministic; tool probes do not require it and instead require
  the unit's recorded `Result=success`, with the canary's proof of the same
  launch chain. A missing or different record is a containment diagnostic, not
  a pass. The tool probes therefore show the tools work inside the boundary and
  agree with their outside runs, not that the supervisor observed each tool's
  cgroup.
- **Proof composition.** The Verus proof checks the capture kernel's per-`feed`
  contract and the model's whole-stream theorem separately. Composing repeated
  executable calls into the model's fold is a manual argument over the two
  checked results; no verified trace harness checks that composition. The proof
  does not extend to the reader's I/O and threads, or to the operating system's
  enforcement.
- **Kernel invariant and public fields.** `Capture::feed` requires
  `well_formed()` — the kept length is at most the cap — and `Capture::new`
  establishes it. The struct's fields are public so the Verus datatype is not
  opaque, which means the invariant is a caller obligation, not a
  representation invariant the type enforces. The reader constructs its capture
  through `new` and never mutates a field directly, so it preserves the
  precondition; an unverified caller that broke it is outside what the proof
  guarantees.
- **Canary outbound target.** The canary connects to a loopback listener the
  probe owns and passes in. Inside the empty network namespace the connection
  is refused or unreachable. On a host whose isolation failed it normally
  reaches the probe's own listener and the control fails closed because no
  connection error was reported; a host firewall that refuses the connection
  instead would produce the same error as a working boundary, which is why the
  control does not rest on the connection alone. The supervisor also compares
  the job's network-namespace link target with its own, so a shared or
  unobserved network namespace is refused independently of the connection
  result. The probe therefore contacts nothing it does not own, which is the
  plan's offline-probe constraint; the expected error is refusal or
  unreachability rather than `ENETUNREACH` alone (D26).
- **Inherited descriptors.** The supervisor lists the job's own descriptor
  directory from outside on every poll and keeps only the descriptors present
  in every sample, so a handle the job opens for its own work cannot be
  mistaken for an inherited one; the hostile probe requires exactly its
  standard streams. Rust opens its own descriptors close-on-exec, and neither
  `systemd-run` nor `bubblewrap` closes foreign ones. The intersection reports
  descriptors present in every sample, not every descriptor inherited at
  launch: a leaked descriptor the job keeps open is reported, but one opened
  and closed between two polls is invisible, and so is an inherited descriptor
  the job closes before the first sample. The canary does not establish what a
  later job inherited, so it is not a substitute for that observation.
- **Kernel resources outside cgroup accounting.** Inotify instances, session
  keyrings, seccomp filters, and `AF_VSOCK` are not bounded here; roadmap 4.3.1
  decides them.
- **Systemd's `Result=`.** The memory bound is read from systemd's
  `oom-kill` record, not from the exit status. A systemd defect that
  misreported the result would be invisible to this probe.
- **Shared host.** The probes are finite and remove their listeners and scratch
  directories, but they do start scopes and allocate memory on the development
  host. `make boundary-probe` runs with one test thread for that reason.

## Outstanding decisions

- Roadmap 4.3.1 promotes the supervisor, capture kernel, launcher, and probes
  into the product and enforces the boundary end to end. It owns the proposed
  outcome mapping: a named bound maps to `unknown` (exit 3, "not settled,
  including resource exhaustion" in `contracts/cli.txt`), and `Unavailable`,
  `ContainmentFailure`, and an unexpected child failure map to `tool_error`
  (exit 6). An explicit user cancellation has no contract outcome today, and
  this record says so rather than inventing one.
- Roadmap 4.3.1 also decides spawning and reaping on one thread, the two
  per-stream caps (1 MiB for standard output, a probe-chosen 64 KiB for
  standard error), and the inotify, keyring, and `AF_VSOCK` questions.
- Roadmap 4.3.3 measures budgets, including Lean's start-up and peak memory, on
  an agreed machine. The 2 GiB and 60 s defaults passed here as observations,
  not as measured limits.
- The supported-platform matrix (ToR §8.1) and the terms of reference's A4 and
  A6 assumptions remain open; this record names enforcement prerequisites, not
  a supported platform.

## Acceptance

Proposed. The project sponsor accepted the 1.2.4 ExecPlan on 2026-11-21 and
this record awaits acceptance in conversation. The measured results are
recorded in that plan's `Outcomes & retrospective` section.

## References

- [Bubblewrap](https://github.com/containers/bubblewrap): the sandboxing
  program and its option set.
- [Flatpak sandbox permissions](https://docs.flatpak.org/en/latest/sandbox-permissions.html):
  the same allowlisted-filesystem construction in a production sandbox.
- [`systemd.resource-control(5)`](https://www.freedesktop.org/software/systemd/man/latest/systemd.resource-control.html):
  cgroup v2 properties including `MemoryMax=`, `MemorySwapMax=`, `TasksMax=`,
  and `OOMPolicy=kill`.
- [`systemd.scope(5)`](https://www.freedesktop.org/software/systemd/man/latest/systemd.scope.html):
  transient scope units and `systemd-run --scope`.
- [`systemd.service(5)`](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html):
  `KillSignal=`, `OOMPolicy=`, and the stop timeout the default policy imposes.
- [Control Group v2](https://docs.kernel.org/admin-guide/cgroup-v2.html): the
  unified hierarchy, `memory.max`, `memory.swap.max`, `pids.max`,
  `memory.oom.group`, `cgroup.kill`, and `cgroup.events`.
- [`namespaces(7)`](https://man7.org/linux/man-pages/man7/namespaces.html) and
  [`network_namespaces(7)`](https://man7.org/linux/man-pages/man7/network_namespaces.html):
  namespace isolation and the abstract Unix socket namespace.
- [`user_namespaces(7)`](https://man7.org/linux/man-pages/man7/user_namespaces.html):
  unprivileged user namespaces.
- [`landlock(7)`](https://man7.org/linux/man-pages/man7/landlock.html): the
  filesystem restriction and its ABI.
- [Go issue 38010](https://github.com/golang/go/issues/38010): `RLIMIT_AS` and
  the Go runtime's reserved address space.
- [Ubuntu's unprivileged user namespace restriction](https://ubuntu.com/blog/ubuntu-23-10-restricted-unprivileged-user-namespaces):
  the AppArmor restriction that makes bubblewrap fail closed by default.
- [Technical design](../technical-design.md) §§2, 11, and 13.
- [ADR 0002](adr-0002-first-profile-assurance-boundary.md),
  [ADR 0003](adr-0003-pin-the-e0-toolchain-tuple.md), and
  [ADR 0006](adr-0006-separate-proof-integrity-from-operational-integrity.md).
- [1.2.4 ExecPlan](../execplans/1-2-4-enforceable-linux-execution-boundary-decision.md).
