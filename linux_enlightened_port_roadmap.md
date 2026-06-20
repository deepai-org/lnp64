# Enlightened Linux Port Roadmap

The Linux counterpart to `netbsd_rump_port_roadmap.md`. Goal: run **real Linux**
(fs/net/drivers/userland) as a *correct* LNP64 port and guest — hardware owns
scheduler/allocator/memory/caps, Linux owns policy — and chart how the deepest
version could eventually reach **mainline**.

This is a *later* track. NetBSD rump comes first and builds the native-op seam
Linux reuses (`rumpuser` and Linux's host-ops bottom out on the same primitives:
`CLONE`/`SCHED`, `ALLOC`, `MMAP`, `FUTEX_*`, `AWAIT_EX`, `DMA_CTL` + IOMMU
windows, device-BAR caps, interrupt-as-waitable).

## The Linux rump-equivalent is LKL

Linux has no anykernel/`rumpuser`, but **LKL (Linux Kernel Library)** is the
closest analog: the Linux kernel built as a library behind a small
host-operations seam (`lkl_host_operations`: threads, mutex/sem, memory, timers,
IRQs). That seam has the same shape as `rumpuser`, lowered to the same native ops.

| LKL host op | Native lowering (shared with rump seam) |
| --- | --- |
| thread create / sem / mutex | `CLONE`/`SCHED`, `FUTEX_*`, `AWAIT_EX` |
| mem alloc / virtual memory | `ALLOC`, `MMAP` |
| timer / clock | timer object profile, PCR/timebase |
| irq | interrupt-as-waitable (`AWAIT_EX`) |

Result: real Linux fs (ext4), real TCP/IP, and the Linux driver tree as
components, with the hardware keeping mechanism — the correct-port shape, for
Linux.

## Two tiers of "enlightened", because Linux is less factored than NetBSD

- **Tier 1 (pragmatic, achievable): LKL over native host-ops.** Real Linux
  software, correct *boundary*. But LKL still contains Linux's own internal
  scheduler and buddy allocator running on the threads/memory it is handed, so
  mechanism is partly *duplicated* internally (like UML). "Good enough
  enlightened," broad compatibility. This is the realistic landing spot.
- **Tier 2 (pure, research): delete Linux's scheduler/buddy/pmap and delegate.**
  What NetBSD's anykernel gives almost for free, Linux fights, because it is not
  componentized that way. A real research effort, not a port.

NetBSD rump lands near Tier 2 by design; Linux-LKL realistically lands at Tier 1
unless heavily invested. That asymmetry is itself a useful result: it shows why
the hardware rewards a rump-style OS and what a monolith leaves on the table.

## Could Tier 2 reach mainline Linux one day?

If the hardware were popular (cloud/hyperscaler silicon is the realistic wedge —
that is where `paravirt_ops` and `sched_ext` pressure came from), the
*delegation* can merge **through abstraction seams that already exist**, never as
a core rewrite. Mainline's bar: no common-case regression, no `#ifdef` pollution
of hot paths, contain novelty behind `arch/`/config.

| Delegation | Existing mainline seam | Merge realism |
| --- | --- | --- |
| DMA / IOMMU (`bus_dma`) | DMA-mapping API + IOMMU subsystem + dma-buf | already shaped for this; cleanest merge |
| MMU / page tables (pmap) | `CONFIG_PARAVIRT` `pv_mmu_ops` (Xen precedent) | plausible: a `pv_ops` backend → VMA/TLB engine |
| time / IRQ | `pv_ops` time/irq, irqchip, clocksource | plausible, same mechanism |
| scheduler | `sched_ext` / BPF scheduler (merged 6.12) | partial: offloads *decisions*, core still runs the runqueue |
| physical memory (buddy / `struct page`) | none exists | hardest; needs a new external-memory-provider abstraction |

**Realistic merge story** (not one patch): upstream an `arch/lnp64` port + a
`pv_ops` backend (MMU/time/IRQ) + a `sched_ext` scheduler, with the radical
delegation behind `CONFIG`/`pv_ops` so the common x86/ARM build is untouched —
exactly how `paravirt_ops` (2007) and `sched_ext` (2024) landed once the hardware
need was demonstrated. The allocator/`struct page` deletion is the last and
hardest piece and would likely live out-of-tree or behind heavy config until a
new mainline memory-provider abstraction exists. The seams can be *widened
upstream over time* (a pv_ops backend now; an external-memory-provider RFC
later) rather than forked.

## Milestone sketch (after the NetBSD rump seam exists)

- **L0** — implement `lkl_host_operations` against the native-op seam built for
  rump; boot LKL and run a trivial Linux syscall (open/read/write) — Tier 1
  beachhead.
- **L1** — real Linux fs (ext4) + TCP/IP as LKL components in Resource Domains.
- **L2** — LKL Linux as an **enlightened guest** under the NetBSD-rump host:
  one machine, two real OSes, both correct ports, sharing hardware
  scheduler/allocator/caps. The multi-personality demo.
- **L3 (research)** — Tier 2 spikes: `pv_ops` MMU backend, `sched_ext` scheduler
  delegation, DMA/IOMMU via the DMA-mapping API — the upstreamable pieces.
- **L4 (long horizon)** — external-memory-provider abstraction to delegate the
  buddy allocator; the mainline endgame.

## Relationship to the NetBSD track

Do NetBSD rump first. It proves the native-op seam, the Correct Port Contract,
and the negative gate; LKL reuses all three. See `netbsd_rump_port_roadmap.md`.
