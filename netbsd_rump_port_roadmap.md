# Real NetBSD Rump Port Roadmap

This is the plan to replace the clean-room NetBSD *personality* (an ABI-shaped
shim with no NetBSD code) with a **real NetBSD port** that stays a *correct*
LNP64 port: hardware owns mechanism, NetBSD owns policy.

It is the concrete realization of the "import NetBSD-derived components" open
work in `netbsd_personality_abi.md`, and the host/guest substrate for the
capability-delegated hypervisor described in `system_software_compatibility_roadmap.md`.

## Decision

- **Shape: three-way split, not "rump only."** The kernel work is divided across
  three owners, none of them a monolith:
  1. **hardware** owns *mechanism*: scheduler, allocator, VMA/memory, capability
     machinery;
  2. a **thin native process faction** owns the Unix process/VM/exec/signal
     *policy* that rump deliberately does not provide (see the note below) —
     `fork`/exec, address spaces, copy-on-write, page-fault routing, signal
     delivery, ttys/job control, ptrace;
  3. **rump components** own driver/filesystem/network policy (real NetBSD code).

  Real NetBSD libc and userland run on top. "No monolithic kernel" is still true:
  no single privileged blob reimplements mechanism. But it is **not** literally
  "rump only" — rump covers fs/net/drivers; the process faction is provided by
  hardware plus a small native policy service (the grown-up form of today's
  clean-room personality).
- **Guests too: enlightened.** Both the top-level host and every guest follow the
  same split and the same Correct Port Contract, differing only in held Resource
  Domain authority. A guest can itself be a host (recursion = the
  nested-containment thesis). They share an image/codebase and component format;
  host and guest differ in *which* components and capabilities they hold (e.g.
  the host loads real NIC drivers, the guest a device-service front end).
- **"Lazy" means reimplementing mechanism — not making a call.** The forbidden
  thing is an OS that brings its own run-queue, frame allocator, page tables, or
  DMA programming (a conventional MD port). It is **not** forbidden for libc or a
  component to reach a facility through a full service call rather than a
  vDSO-style fast path. A correctness-first generic path (the complete syscall
  surface routed through the owning service) is a *legitimate, expected* route;
  enlightened fast paths are an optimization layered on top, added rung by rung.

## Completeness: a real, full, correct port — with good breadth

The primary success metric is **reference correctness**: an honest, complete port
that is *what a correct port to this hardware looks like*, where every facility
real software actually touches either works correctly or is a tracked ladder rung
— never silently faked. Maximal software breadth is the job of the Linux track
(`linux_enlightened_port_roadmap.md`); this port should still reach **good
breadth** (a large fraction of pkgsrc), but breadth is a strong goal layered on
correctness, not the axis this port is judged on.

This is **not** a subset, a demo, or a "good enough" compatibility layer. The
target is that **NetBSD-compatible software runs correctly when compiled for
LNP64** — the same behavior you would expect on a stock NetBSD/amd64 or
NetBSD/aarch64 machine — for everything within the reached rungs.

Concretely, "full" means:

- **Complete libc and ABI.** The full real NetBSD libc, libpthread, libm, librt,
  dynamic loader, and the complete syscall surface — not a curated subset. POSIX,
  BSD extensions, signals, threads, and process semantics behave per NetBSD.
- **Real subsystems, not fixtures.** Real FFS/UFS, tmpfs, NFS, and the VFS layer;
  the real NetBSD TCP/IP stack and sockets; real device drivers — all as rump
  components running the *same code* NetBSD ships. (The driver/fs/net/userland
  code is verbatim NetBSD; the process/VM/exec/signal faction and the MD seam are
  LNP64-native — adapted, not "the same code." That scoping is deliberate.)
- **Good pkgsrc breadth.** The goal is that a large fraction of pkgsrc software
  builds and runs unmodified once retargeted to the LNP64 toolchain. If a normal
  NetBSD program needs a facility, that facility must exist and be correct, or be
  an explicit open rung.
- **No silent capability gaps.** Anything not yet done is tracked as an explicit
  ladder rung with a gate, never quietly stubbed, faked, or `ENOSYS`-ed in a way
  that lets broken software appear to pass.
- **Enlightened the right amount: mechanism is direct, policy is a service call.**
  The libraries (libc, libpthread, libm, librt, loader) are enlightened, but
  enlightenment does *not* mean dissolving the call boundary. The distinction is
  mechanism vs policy:
  - **Genuine hardware primitives** lower to a *direct* instruction with no
    service round-trip, because the Capability Engine already mediates every such
    op: `malloc`/arenas → `ALLOC`/`MMAP` of owned objects; threads/locks/condvars
    → `CLONE`/`SCHED`/`LOCK_CMPXCHG`/`FUTEX_*`/`AWAIT_EX`; `poll`/`select`/`epoll`
    readiness → `WAITABLE_PROBE`/`AWAIT_EX`; clocks → PCR/timebase reads; atomics.
    This is the **vDSO** idea (today's libc already does `clock_gettime` without a
    trap) generalized — safe *and* fast, and it is why a subset of Unix syscalls
    disappear into plain instructions.
  - **Policy operations still make system calls**, because the policy lives in a
    rump service, not in a single hardware op: `open` (path resolution, mounts,
    permissions, vnode allocation), socket/network operations (the TCP/IP stack),
    `fork`/exec/namespace operations. libc calls the owning service — and that
    call *is* the LNP64 system call: a typed, capability-mediated `GATE_CALL`
    into the service's domain rather than a privilege-ring trap into a monolithic
    kernel. The service then uses hardware primitives internally.

  So "enlightened" means the call boundary is a typed capability gate (not a ring
  trap) and that pure-primitive operations skip the call entirely — *not* that
  libc reaches around the services that own policy. A NetBSD syscall number also
  remains a valid, complete entry path for unmodified binaries; enlightened libc
  fast paths are an optimization over it, not a replacement for the service model.

The Correct Port Contract below restricts only *who provides mechanism*
(scheduler, allocator, page tables, DMA, permission checks → hardware), **never
what functionality exists**. Every mechanism the contract forbids NetBSD from
*reimplementing* is one LNP64 already provides and proves; behavior is preserved,
not dropped. "Delete the run-queue" means *delegate* it, not lose scheduling.

The only deliberate exclusions are mechanisms LNP64 owns by design (port-private
page tables, a software run-queue, raw interrupt/DMA programming) — and those are
*replaced* by native equivalents, so no software loses a capability because of
them. Where exact NetBSD semantics and the native model genuinely diverge, the
divergence is documented as a named, tested compatibility decision, not an
undocumented gap.

## Why rump, not a monolithic MD port

A monolithic NetBSD port brings its own run-queue, UVM, pmap, and locking — the
exact mechanisms LNP64 implements and proves in hardware. That is the *lazy*
port the personality non-goals already forbid. NetBSD is the one OS architected
for the alternative: the **anykernel** factoring lets drivers/fs/net run outside
a monolithic kernel over the **`rumpuser`** hypercall layer. `rumpuser` is the
seam where mechanism is delegated. Above it: real NetBSD code and broad pkgsrc
software. At it: our code, lowering to native ops.

**What rump does not give you (and who provides it here).** Rump kernels
deliberately omit the full Unix *process faction*: they provide `lwproc`
credential/descriptor contexts, but not separate-address-space processes with
`fork`/exec, copy-on-write, demand paging, full signal-delivery semantics,
ttys/job control, or ptrace. In ordinary rump deployments that faction is
borrowed from a host kernel. On LNP64 it is exactly what the hardware already
provides — Resource Domains + `CLONE`/`EXEC` + the VMA engine + gate-delivered
signals — wrapped by a thin **native process service** (owner #2 in the split
above; the grown-up form of the clean-room personality). This is a strength, not
a gap: the faction rump normally borrows is the one LNP64 is best at — but it
must be built and tested explicitly (see milestone R1.5), not assumed to come for
free with rump.

## The Correct Port Contract

A correct LNP64 OS port **owns policy, not mechanism**. It MUST NOT contain:

- a thread run-queue or context-switch dispatcher (→ `SCHED` admit + `AWAIT_EX`),
- a physical-frame allocator (→ `ALLOC`),
- a page-table walker / pmap that mints mappings (→ `MMAP`/`OBJECT_CTL`/VMA),
- a DMA programmer that touches raw addresses (→ `DMA_CTL` + IOMMU windows),
- an MMIO poke path outside a narrowed device-BAR capability (→ `bus_space`),
- a permission/ACL enforcer (→ Capability Engine; uids/modes are labels only).

The existing system gate already enforces the negative side of this contract:
it **rejects raw interrupt/MMIO/DMA/page-table/scheduler trace tokens, and
ring-trap / emulator-escape syscalls** (`scripts/run_netbsd_personality_system.sh`).
Note the distinction: the gate rejects *mechanism-bypassing* paths — a privileged
trap into a monolithic kernel or a direct emulator syscall escape. It does **not**
reject the sanctioned LNP64 system call, which is a typed, capability-mediated
`GATE_CALL` into the owning service's domain. "Services own policy" requires those
calls. Every rump milestone extends the rejection to the new layer. The contract +
the failing gate *is* the argument to OS authors.

## The seam: rumpuser + NetBSD MD hooks → native ops

NetBSD already abstracts drivers from hardware via `bus_space`/`bus_dma`, and the
rump base via `rumpuser`. A correct port implements only this hook set; the
unmodified code above it inherits the discipline.

| NetBSD / rump hook | Correct LNP64 lowering | Severe goal earned |
| --- | --- | --- |
| `rumpuser` thread/mutex/cv | `CLONE`/`SCHED` admit, `LOCK_CMPXCHG`, `FUTEX_*`, `AWAIT_EX` | scheduler/wait correctness |
| `rumpuser` malloc/anon memory | `ALLOC`, anon `MMAP` | memory authority |
| `rumpuser` clock | PCR/timebase reads, timer object profile | realtime contract |
| `pmap` / UVM backing | `MMAP`/`OBJECT_CTL`/VMA engine (no port page tables) | memory authority |
| `cpu_switchto` / run-queue | `SCHED` reservation admit + `AWAIT_EX` (no run-queue) | scheduler + realtime |
| `kmem`/pool backing | `ALLOC` | memory authority |
| `bus_dma` | `DMA_CTL` + IOMMU-scoped DMA windows | **DMA authority** |
| `bus_space` (MMIO) | narrowed device-BAR capability | whole-chip mediation |
| `intr_establish` | interrupt-as-waitable (`AWAIT_EX` on an IRQ event) | interrupt-abstracted / bounded latency |
| vfs / fd table | FDRs, `PULL`/`PUSH`, generation/narrowing | no forged authority |

## Milestone ladder

Aligned to the governing method (fix every gap in the lowest correct layer;
drive everything through `lnp64_top`-reachable paths and a checker/manifest).

- **R0 — rumpuser seam + real NetBSD libc (first target).** Implement the
  `rumpuser` hypercall layer + minimal MD hooks against native ops. Build real
  NetBSD libc with Clang/lld and run a trivial rump component (e.g. rump_syscall
  open/read/write) over it. Beachhead: first real NetBSD code executing. Replaces
  the clean-room shim incrementally, ladder-rung by ladder-rung.
- **R1 — rump VFS + FFS.** Real NetBSD FFS as a rump filesystem service in a
  Resource Domain, over an object-backed block FDR. Retires the fixed-record
  service-owned image fixture.
- **R1.5 — native process faction (the hard, software-critical layer).** Real
  separate-address-space processes: `fork` with copy-on-write, `exec`,
  **file-backed mmap with demand-fault routing to the fs service**, `mprotect`
  faults, full signal delivery, ttys/job control, and ptrace — built on Resource
  Domains + `CLONE`/`EXEC` + the VMA engine + gate signals, wrapped by the native
  process service. This is what rump does not provide and what most real software
  depends on; it gates real shells, dynamic linking, and databases. It is *not*
  optional and is sequenced early on purpose.
- **R2 — rump network stack.** Real NetBSD TCP/IP as a rump service; sockets
  lower to endpoint object profiles + readiness waits. Loopback then real NIC.
- **R3 — host role.** A supervisor rump instance holds device + domain authority
  and carves child domains with memory-object + scheduler-reservation caps.
- **R4 — enlightened guest.** The same rump port runs inside a child domain on
  only delegated capabilities. Host and guest are the same binary.
- **R5 — scheduler-reservation delegation.** Guest admits its threads under a
  delegated RT budget/class; prove no double scheduling and inherited bounded
  service.
- **R6 — device-as-service + IOMMU.** Host re-exports a real driver to the guest
  via an IOMMU-scoped DMA window + `GATE_CALL`; revoked window fails closed
  (seed: `demos/revoked_dma_buffer.s`).
- **R7 — recursion proof.** A guest becomes a host for its own sub-guest:
  container/VM/supervisor demonstrated as one nested primitive.

## Named compatibility risks to decide explicitly

These are genuine places where exact NetBSD semantics and the native model may
diverge. Each must become a *named, tested compatibility decision*, not a silent
assumption behind "all software runs":

- **POSIX scheduling policy over a frozen hardware scheduler.** Software sets
  `SCHED_FIFO`/`SCHED_RR`/`SCHED_OTHER`, priority ranges, `nice`, and affinity.
  A frozen silicon scheduler may not express every policy exactly. The mapping
  from POSIX scheduling onto the hardware's fixed model (and what is reported as
  best-effort) is a decision to specify and test, not assume.
- **mmap/COW/demand-paging/file-backed faults.** Used by `ld.so`, databases, and
  much of userland. A file-backed page fault must route from the VMA engine to a
  userspace fs service to fill the page — a hard hardware↔service interaction. The
  fault path, COW-on-fork, `msync`, `madvise`, and `MAP_FIXED`/overcommit
  behavior must be specified against the VMA engine, since there are no
  port-private page tables to fall back on. This is where "no port page tables"
  gets stress-tested; R1.5 owns it.
- **Anything else where native and NetBSD semantics differ** is documented as a
  named decision with a test, never an undocumented gap.

## Device drivers for the FPGA host

A real top-level host needs real drivers. Minimum set: console **UART**, a
**storage** device (SD/MMC, SPI-flash, or virtio-blk soft IP), optionally a
**NIC**, plus interrupt + DMA plumbing. Each runs as a rump driver in a domain
with: a narrowed device-BAR capability (`bus_space`), an IOMMU-scoped DMA window
(`bus_dma`), and an interrupt waitable (`intr_establish` → `AWAIT_EX`). Guests
receive these as device-services (R6), never as raw hardware.

## Relationship to the existing personality

The clean-room personality (`src/personality_lowering.rs`, `netbsd_personality_abi.md`)
stays as the ABI oracle and negative gate while R0–R2 import real code under the
same checks. Each rump rung that goes green retires the matching shim surface.
The layer-order contract `toolchain/lnp64_netbsd_layers.manifest` already orders
this: libc → rump fs → rump net/socket → process/signal/thread → userland →
fuller machine port.

## Later: Linux

The same native-op seam built here is reused by a later **enlightened Linux**
port (via LKL), including how the deep delegation could reach mainline. See
`linux_enlightened_port_roadmap.md`.
