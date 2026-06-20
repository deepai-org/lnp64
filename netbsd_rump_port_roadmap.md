# Real NetBSD Rump Port Roadmap

This is the plan to replace the clean-room NetBSD *personality* (an ABI-shaped
shim with no NetBSD code) with a **real NetBSD port** that stays a *correct*
LNP64 port: hardware owns mechanism, NetBSD owns policy.

It is the concrete realization of the "import NetBSD-derived components" open
work in `netbsd_personality_abi.md`, and the host/guest substrate for the
capability-delegated hypervisor described in `system_software_compatibility_roadmap.md`.

## Decision

- **Shape: rump / anykernel only.** Real NetBSD drivers, filesystems, network
  stack, libc, and userland run as **rump components** over a thin native seam.
  No monolithic NetBSD kernel. The hardware keeps the scheduler, allocator,
  memory, and capability machinery; NetBSD code never reintroduces them.
- **Guests too: pure enlightened.** Both the top-level host and every guest are
  the same correct port, differing only in held Resource Domain authority. A
  guest can itself be a host (recursion = the nested-containment thesis).
- **No lazy/monolithic path.** A conventional NetBSD MD port (its own scheduler,
  UVM, page tables, locking) is explicitly *not* a goal: it violates the Correct
  Port Contract below and undercuts the "this is what a correct port looks like"
  message to OS authors.

## Completeness: this is a real, full port

This is **not** a subset, a demo, or a "good enough" compatibility layer. The
target is an honest, complete NetBSD port in which **essentially all
NetBSD-compatible software runs correctly when compiled for LNP64** — the same
binaries' worth of behavior you would expect on a stock NetBSD/amd64 or
NetBSD/aarch64 machine.

Concretely, "full" means:

- **Complete libc and ABI.** The full real NetBSD libc, libpthread, libm, librt,
  dynamic loader, and the complete syscall surface — not a curated subset. POSIX,
  BSD extensions, signals, threads, and process semantics behave per NetBSD.
- **Real subsystems, not fixtures.** Real FFS/UFS, tmpfs, NFS, and the VFS layer;
  the real NetBSD TCP/IP stack and sockets; real device drivers — all as rump
  components, with the *same code* NetBSD ships, not reimplementations.
- **Full pkgsrc breadth.** The goal is that arbitrary pkgsrc software builds and
  runs unmodified once retargeted to the LNP64 toolchain. If a normal NetBSD
  program needs a facility, that facility must exist and be correct.
- **No silent capability gaps.** Anything not yet done is tracked as an explicit
  ladder rung with a gate, never quietly stubbed, faked, or `ENOSYS`-ed in a way
  that lets broken software appear to pass.
- **Fully enlightened system libraries, top to bottom.** The enlightenment is not
  confined to the rump kernel seam. libc, libpthread, libm, librt, and the
  dynamic loader are themselves enlightened: their primitives lower *directly*
  onto native operations, not onto an emulated NetBSD syscall trap that is then
  re-lowered. `malloc`/arenas → `ALLOC`/`MMAP`; threads/locks/condvars →
  `CLONE`/`SCHED`/`LOCK_CMPXCHG`/`FUTEX_*`/`AWAIT_EX`; file/socket/pipe I/O →
  FDRs + `PULL`/`PUSH`; `poll`/`select`/`epoll` → `WAITABLE_PROBE`/`AWAIT_EX`;
  signals → gate delivery; cross-domain/service calls → `GATE_CALL`. A NetBSD
  syscall number remains a valid, complete entry path (for unmodified binaries),
  but the libc fast paths take the native route so the *whole stack*, not just
  the kernel components, runs as a correct hardware-native port. Where a library
  today bottoms out in a generic syscall, that is a tracked rung to enlighten,
  not the intended end state.

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

## The Correct Port Contract

A correct LNP64 OS port **owns policy, not mechanism**. It MUST NOT contain:

- a thread run-queue or context-switch dispatcher (→ `SCHED` admit + `AWAIT_EX`),
- a physical-frame allocator (→ `ALLOC`),
- a page-table walker / pmap that mints mappings (→ `MMAP`/`OBJECT_CTL`/VMA),
- a DMA programmer that touches raw addresses (→ `DMA_CTL` + IOMMU windows),
- an MMIO poke path outside a narrowed device-BAR capability (→ `bus_space`),
- a permission/ACL enforcer (→ Capability Engine; uids/modes are labels only).

The existing system gate already enforces the negative side of this contract:
it **rejects raw interrupt/MMIO/DMA/page-table/scheduler/syscall trace tokens**
(`scripts/run_netbsd_personality_system.sh`). Every rump milestone extends that
rejection to the new layer. The contract + the failing gate *is* the argument to
OS authors.

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
