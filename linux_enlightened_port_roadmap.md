# Enlightened Linux Port Roadmap

The Linux counterpart to `netbsd_rump_port_roadmap.md`. **Goal: a *fully
enlightened* Linux port — hardware owns scheduler/allocator/memory/caps, Linux
owns policy — built so the delegation pieces are acceptable upstream by kernel
maintainers, not a permanent out-of-tree fork.**

Like the NetBSD port, this is an **accessory software-compatibility track, not
part of the formal proof surface**: it *consumes* hardware whose properties are
proven elsewhere (the severe goals, M1–M15) and carries no theorems of its own.
Its evidence is software gates/tests and the negative trace-token gate. Linux is
a **guest payload** here, never the supervisor — the host role stays with the
NetBSD-rump supervisor (`netbsd_rump_port_roadmap.md`).

This is a *later* track. NetBSD rump comes first and builds the native-op seam
Linux reuses (`CLONE`/`SCHED`, `ALLOC`, `MMAP`, `FUTEX_*`, `AWAIT_EX`,
`DMA_CTL` + IOMMU windows, device-BAR caps, interrupt-as-waitable), **including
the native process faction** (fork/COW, exec, file-backed mmap faults, signals,
ptrace-as-delegated-capability) — see below, Linux needs the same one.

## The target: fully enlightened (what "Tier 2" means)

A fully enlightened Linux delegates *mechanism* to hardware and keeps only
*policy*, exactly like the NetBSD Correct Port Contract: no Linux run-queue, no
buddy allocator owning physical frames, no Linux-private page tables, no raw DMA.
Scheduling decisions, allocation, mapping, and DMA all bottom out in native ops.
That is the only configuration that satisfies the goal line above and would pass
the negative gate.

Linux is **less factored than NetBSD** for this, so full enlightenment is real
engineering (and, for the last piece, real upstream work). The honest spectrum:

| Piece | Fully-enlightened target | Upstream seam it rides |
| --- | --- | --- |
| scheduler | no Linux run-queue; tasks admitted to the hardware scheduler | `sched_ext` (BPF scheduler, merged 6.12) extended toward full delegation |
| MMU / page tables | mapping via the VMA/TLB engine, no Linux pmap minting | `CONFIG_PARAVIRT` `pv_mmu_ops` (Xen PV precedent, 2007) |
| time / IRQ | PCR/timebase reads; interrupt-as-waitable | `pv_ops` time/irq, irqchip, clocksource |
| DMA / IOMMU | `DMA_CTL` + IOMMU-scoped windows | DMA-mapping API + IOMMU subsystem (already shaped) |
| physical memory | hardware allocator owns frames; no buddy / no `struct page` | **no general seam yet** — narrow ones exist (`ZONE_DEVICE`/DAX, `guest_memfd`); needs a general external-memory-provider |

## LKL is the bring-up stepping stone, not the destination

Linux has **LKL (Linux Kernel Library)** — the kernel built as a library behind a
small host-ops seam (`lkl_host_operations`). It is the fastest way to get real
Linux fs/net/drivers executing on the native seam, so it is the **early bring-up
vehicle**. But be honest about what it is:

- LKL takes **one contiguous memory arena** and runs Linux's **own buddy
  allocator** inside it, and runs Linux's **own scheduler** over a few host
  threads. So an LKL bring-up is **not** a fully enlightened port — it still
  contains the run-queue and frame allocator the contract forbids. It is a
  *hosted-compatibility payload* (like running a VM): great for breadth and for
  proving the seam, but it does **not** pass the correct-port gate.
- LKL is also typically **single-address-space**: it does not by itself give a
  fork/exec multiprocess Unix. Real Linux userland is fork-heavy (shells, `make`,
  pipelines, daemons), so multiprocess must come from the **shared native process
  faction** (the same hardware-domains + process-service layer the NetBSD port
  builds), not from LKL.

So LKL gets real Linux code running fast; reaching the actual goal means moving
each mechanism off Linux's internal implementation onto the native ops above —
which is the same set of changes that makes the port upstreamable.

## Mainline acceptability (the spine)

Maintainers will not take a fork of `kernel/sched/` or `mm/`. The bar: no
common-case regression, no `#ifdef` pollution of hot paths, novelty contained
behind `arch/`/config/`pv_ops`. The delegation merges as **new opt-in backends on
existing abstraction seams**, the way `paravirt_ops` (2007) and `sched_ext`
(2024, 6.12) landed once real hardware demand existed. Realistic order:

1. **`arch/lnp64` + `pv_ops` backend** (MMU/time/IRQ) and the **DMA-mapping/IOMMU
   driver** — these ride seams that already exist; cleanest to upstream.
2. **`sched_ext` scheduler** delegating decisions to the hardware scheduler —
   merges as a BPF/ext scheduler; full "no run-queue" delegation is an extension
   of, not a fight with, this seam.
3. **External-memory-provider** to delegate the buddy/`struct page` model — the
   hardest and last piece. No general seam exists yet, but `ZONE_DEVICE`/DAX and
   `guest_memfd` are footholds to generalize via RFC. Likely lives behind heavy
   config / out-of-tree longest.

The wedge is cloud/hyperscaler silicon — the same constituency that drove
`paravirt_ops` and `sched_ext` upstream. The seams can be **widened upstream over
time** (pv_ops + sched_ext backends now; an external-memory-provider RFC later)
rather than forked.

## Milestone sketch (after the NetBSD rump seam + process faction exist)

- **L0 — LKL bring-up.** `lkl_host_operations` on the native seam; boot LKL, run a
  trivial Linux syscall. Hosted-compatibility payload, not yet enlightened.
- **L1 — real Linux fs (ext4) + TCP/IP** as LKL components in Resource Domains.
- **L1.5 — enlightened libc + distro recompile.** Port musl (`arch/lnp64`) to
  emit native ops + an LNP64 vDSO for the fast paths; recompile a small distro
  (Buildroot-style) so userspace inherits enlightenment unmodified. Add a Go
  runtime `syscall` port for the one ecosystem that bypasses libc.
- **L2 — multiprocess via the native process faction.** Real fork/exec Linux
  userland over hardware domains + the shared process service (reused from the
  NetBSD R1.5 work), not single-instance LKL.
- **L3 — enlighten the mechanisms (the actual goal).** Move scheduling onto the
  hardware scheduler (`sched_ext`), mapping onto the VMA engine (`pv_mmu_ops`),
  DMA onto `DMA_CTL`/IOMMU. Each step removes a piece of Linux-internal mechanism
  and is written to be upstreamable.
- **L4 — physical-memory delegation (long horizon).** External-memory-provider so
  the hardware allocator owns frames; the mainline endgame.
- **L2+ demo — Linux as an enlightened guest** under the NetBSD-rump host: one
  machine, two real OSes sharing the hardware scheduler/allocator/caps. Only the
  pieces that have reached L3 are "enlightened"; the rest run as hosted payload
  until they do.

## Userspace and the distro: enlighten libc, recompile the rest

Enlightenment is not only a kernel-side job, but userspace does **not** mean
hand-porting thousands of packages. The lever is **libc**.

- **Enlighten libc once; the distro inherits it.** Applications call
  `malloc`/`pthread_*`/`open`/`poll`/`mmap` — they do not issue syscalls
  themselves. If libc lowers those onto native ops (same rule as everywhere:
  *mechanism* → direct instruction, vDSO-style; *policy* → typed `GATE_CALL` into
  the owning service), then every package above libc becomes enlightened **just by
  recompiling, unmodified**. No per-library work.
- **It rides libc's existing per-arch port mechanism — no upstream required.**
  glibc has `sysdeps/<arch>`, musl has `arch/<arch>`; the syscall layer is already
  architecture-pluggable. An LNP64 port that emits native ops instead of the
  generic trap is a normal libc port that lives in **your distro**. **musl is the
  preferred target** (small, one clean syscall chokepoint); glibc is doable but
  heavier (NPTL/IFUNC/larger sysdeps surface). Optionally upstream to musl later —
  low-stakes, unlike the kernel pieces.
- **vDSO is the maintainer-friendly framing for the fast paths.** Linux already
  ships a vDSO (userspace `clock_gettime` with no trap today). An LNP64 vDSO that
  turns more former-syscalls into direct instructions *extends an accepted
  mechanism* rather than inventing one.
- **The only userspace needing separate work: runtimes that bypass libc.** The
  big one is **Go** (its runtime issues raw syscalls directly — needs a `syscall`/
  runtime port); plus the occasional statically-inlined-syscall program. Rust,
  C/C++, Python, etc. go through libc and are covered automatically.

**Upstreaming asymmetry (your assumption, confirmed):** upstream the *kernel*
delegation (`arch/lnp64` + `pv_ops` + `sched_ext` + DMA/IOMMU) because it must be
maintainable in-tree; keep *userspace* enlightenment in your distro via the libc
port + LLVM target + loader, then **recompile the whole distro**
(Buildroot/Yocto/Gentoo-style). The targeted enlightenment set is small —
**libc + toolchain/loader + Go's runtime** — and everything else just rebuilds.

## Distro upstreaming targets (who accepts weird arches)

We expect to upstream the userspace port into a few receptive distros rather than
carry a private one forever. Best fits, given our **musl + LLVM** choices:

- **Tier 1 — musl/LLVM, experiment-friendly (best fit):**
  - **Chimera Linux** — musl *and* an LLVM/Clang system toolchain, portability-
    first. Our toolchain choices are its defaults; the closest natural home.
  - **Alpine Linux** — musl, very large package set, adds arches readily; highest-
    leverage musl distro.
  - **Void Linux** — independent, musl option, experiment-friendly.
  - **Adélie Linux** — musl, portability/odd-hardware focused.
- **Tier 2 — formal ports / secondary-architecture channels (glibc; built to
  absorb new arches):**
  - **Debian Ports** (debian-ports.org) — canonical home for unofficial arches.
  - **Fedora secondary architectures** — where RISC-V matured (SIG + koji).
  - **openSUSE** via the Open Build Service — cheap to add arches.
  - **Gentoo** — source-based; new arch via keywords/profiles is trivial to carry.
- **Tier 3 — bring-up build systems, not upstream distros (use first):**
  Buildroot, Yocto, OpenWrt, NixOS.

Suggested path: bring up in **Buildroot**, make **Chimera or Alpine** the first
real musl/LLVM upstream, and use **Debian Ports / Fedora secondary** as the
glibc-world official channels. (Distro receptiveness shifts over time; confirm
current arch/port policy before committing to any one.)

## Relationship to the NetBSD track

Do NetBSD rump first. It proves the native-op seam, the Correct Port Contract,
the negative gate, **and the native process faction Linux also needs**. LKL
reuses all of it. See `netbsd_rump_port_roadmap.md`.
