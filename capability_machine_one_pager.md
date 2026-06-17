# LNP64: A Capability Machine for System Software

LNP64 is a load/store processor with hardware-native capabilities for files,
memory, synchronization, isolation, devices, and service calls. It is not a
kernel-in-hardware design. It provides a small substrate for building Unix-like
systems, runtimes, drivers, containers, VMs, and services without ambient
privilege.

## Core Model

Programs hold unforgeable FDR capability registers. FDRs name
streams, files, queues, counters, memory objects, DMA buffers, PCIe BARs, event
queues, call gates, namespace roots, devices, and Resource Domains. Authority
moves only through explicit capability transfer, narrowing, sealing, revocation,
and returned-capability commits.

The core ISA remains conventional for computation: registers, branches, calls,
loads, stores, atomics, floating point, and vectors stay direct. The difference
is the system boundary:

- `PULL` / `PUSH`: move bytes or records through streams, queues, files,
  devices, packet endpoints, and event objects.
- `AWAIT`: park a hardware thread on a waitable object or futex predicate.
- `MMAP` / `MPROTECT` / `MUNMAP`: manage VMAs through capability-checked memory
  objects, files, DMA buffers, and BARs.
- `CAP_*`: duplicate, transfer, narrow, seal, receive, and revoke capabilities.
- `OBJECT_CTL`: create/configure `counter`, `queue`, `event/completion`,
  `timer`, `memory_object`, `call_gate`, `dma_buffer`, and `dma_completion`
  profiles.
- `DOMAIN_CTL`: create/configure Resource Domains for VMs, containers, cgroups,
  jails, sandboxes, supervisors, and mission profiles.
- `GATE_CALL` / `GATE_RETURN`: perform protected cross-thread or cross-domain
  calls and return through trusted continuations. `CALL_CAP`, `RET_CAP`, and
  `SIGRET` are profile/source names over this gate mechanism.
- `DMA_CTL`: perform bulk copy/fill/scatter-gather/checksum work under VMA,
  IOMMU, capability, and domain checks.

Pipes, semaphores, completions, event counters, shared arenas, sockets, timers,
VMs, containers, and cgroups are profiles over these primitives, not separate
hardware subsystems.

Native APIs should prefer selectors, capabilities, event queues, gate
activations, runtime objects, and Resource Domains. POSIX paths, file
descriptors, POSIX UID/GID, signals, and `errno` remain compatibility profiles
over those primitives.

## Service Boundary

Hardware owns authority, scheduling, waitability, memory safety, accounting,
and commit semantics. Services own evolving policy.

Filesystems, path walking, executable formats, dynamic linking, TCP/IP, Wi-Fi,
PCIe quirks, socket policy, declassification policy, and Unix compatibility
rules live in service domains or personalities. Hardware dispatches bounded
namespace selectors, parks callers, validates replies, and installs returned FDR
authority only through the Capability Engine. POSIX paths are one selector
profile, not the hardware namespace model.

Service replies are data until committed. A namespace, filesystem, network,
PCIe, loader, or supervisor service may propose a returned capability, but it
cannot mint one by writing an integer or payload field. Hardware checks object
class, rights, range, generation, lineage, label, receiver domain policy, and
destination FDR policy before publishing authority.

Every service transaction has one commit point. Before commit, cancellation,
revocation, service crash, signal interruption, or domain teardown aborts with a
typed error. After commit, the object profile defines roll-forward, drain, or
teardown behavior. Backpressure is explicit: wait, `EAGAIN`, or bounded
overflow.

## Unix Compatibility

POSIX is the main compatibility profile, not the primitive model.

- POSIX file descriptors are FDR capability handles plus libc/personality
  metadata.
- `fork` is a constrained `CLONE profile=posix_fork`.
- `exec` commits a loader-produced exec-plan descriptor; hardware does not
  parse ELF, shebangs, auxv, dynamic-linker state, or credential transitions.
- Signals are a bounded POSIX profile over native gate delivery: dispositions,
  per-thread masks, directed pending state, fault-to-signal mapping, checked
  `kill`/`raise`, `alarm`, fixed handler entry, and trusted `SIGRET` as a
  `GATE_RETURN` alias. Native async code should use event queues, cancellation
  objects, counters, queues, and gate profiles.
- Linux and NetBSD can run as paravirtual personalities over lifecycle events,
  VMA events, namespace dispatch, block-image FDRs, gate-delivery/fault records,
  event queues, endpoint capabilities, and PCIe BAR/DMA/IRQ-event FDRs.

Personality software may emulate rich Unix behavior, but it cannot own raw page
tables, raw interrupts, raw DMA, scheduler dispatch, or capability minting.

## Isolation And Scheduling

Resource Domains are the containment primitive. They form a tree. Child domains
receive only delegated capabilities and monotonic resource limits. CPU, memory,
PID/thread count, FDRs, VMAs, devices, namespace roots, scheduler policy,
telemetry, and upcalls are domain-scoped.

The hardware scheduler owns ready/blocked transitions, wakeups, no-lost-wakeup
state, budget accounting, and dispatch. It is a fixed weighted-fair virtual-time
algorithm with weights, quotas, affinity, and latency-class inputs. It is not a
scheduler plugin interface.

Boot creates a default operating envelope before user code: root domain, PID 1
domain, scheduler profile, security defaults, telemetry/fault routes,
measurements, and explicit initial FDR grants. PID 1 refines policy; it does
not create the authority model from scratch.

## Memory, Devices, And Networking

VMAs use a fixed page-state machine: unmapped, reserved, nonresident object,
fill pending, resident clean/dirty, COW shared, DMA pinned, revoking, and
poisoned. Hardware owns permission checks, COW, zero fill, pinning, shootdown,
revocation, atomic page install, and deterministic race priority. Object
contents, writeback, `msync`, truncation, and filesystem coherence remain
service policy.

Device access is capability-scoped. Drivers do not receive raw interrupts,
physical addresses, or ambient MMIO. PCIe BARs are page-granular FDRs mapped
with `MMAP`; DMA buffers are FDRs scoped by IOMMU, VMA, domain, direction, and
generation; MSI/MSI-X becomes `irq_event` records.

Networking exposes packet queues, datagram endpoints, stream endpoints,
listeners, classifiers, counters, and endpoint capabilities. Ethernet, Wi-Fi,
TCP/IP, routing, firewall, DNS, TLS, socket options, and transport policy remain
software-owned. Future accelerators must preserve the same endpoint shapes.

## Security And Assurance

Security is native to the object model:

- W^X, NX data, ASLR, guard pages, entropy, generation checks, revocation,
  sealed/narrowed capabilities, DMA isolation, and scoped telemetry are enforced
  by Resource Domains, VMAs, FDRs, and the coherent DMA/IOMMU path.
- Raw interrupt vectors are not exposed to software. The Event Router converts
  physical interrupts into waitables, gate deliveries, scheduler events, fault records,
  or supervisor/control events.
- Audit streams are append-only FDR-backed logs with sequence numbers, hash
  chaining, dropped-count/gap metadata, scoped disclosure, redaction, and
  quoteable roots.
- Debug and forensics require explicit debug-control FDRs, measured unlocks,
  audit records, and domain/range/label-scoped rights.

Deployment profiles are named policy inputs:

- `DOMAIN_PROFILE_TENANT_STRICT`: no ambient devices, no raw interrupts, scoped
  DMA/telemetry, explicit shared pages, and no parent memory inspection without
  delegated authority.
- `ASSURANCE_DEV`, `ASSURANCE_FIELD`, `ASSURANCE_HIGH`, `ASSURANCE_FORMAL`:
  bind measured boot, quotes, debug posture, ECC/parity, watchdogs, telemetry,
  audit roots, MLS labels, proof artifacts, RTL/IP provenance, and build ids.
- `MISSION_PROFILE`: adds continuity metadata: assurance floor, audit and
  attestation requirements, dependency graph hash, fallback capabilities,
  degraded modes, stale-data budget, recovery priority, and fail policy.

These profiles serve different operators without changing the architecture:
hyperscalers get tenant isolation, live migration hooks, scoped telemetry, and
fault containment; federal users get assurance, MLS, audit, attestation, and
mission continuity; open-source users get owner keys, reproducible builds,
replaceable services, and no hidden vendor path.

Mission state is bounded: normal, degraded, recovering, frozen, failed closed,
or quarantined. Recovery and failover cannot broaden authority; fallback
services must already be delegated; stale service generations cannot complete.

## Open Assurance

The same mechanisms support owner-verifiable computing. LNP64 should be able to
ship as open RTL with reproducible bitstream manifests, public proof artifacts,
owner-installed trust roots, owner-held debug-control capabilities, and
replaceable service stacks. Attestation proves measured artifacts and active
policy; it is not a DRM path. Managed fleets may require signed images or locked
debug by policy, but the ISA does not require vendor-exclusive keys, hidden
management engines, remote kill switches, ambient telemetry channels, or secret
debug/DMA paths.

## Hardware Philosophy

Hardware modules are small state machines, not hidden CPUs. A block earns
silicon only when it owns useful local state, enforces a critical invariant,
reduces memory traffic, improves streaming, or makes an atomic transition
cheap. Complex or fast-changing policy stays in libc, runtimes, service domains,
drivers, or Unix personalities.

The verification target is seL4-like confidence for the hardware-visible
security model: capability non-forgeability, monotonic delegation, revocation
soundness, domain containment, W^X, DMA isolation, scheduler validity, no lost
wakeups, precise gate/fault delivery, service-boundary soundness, and
crash-free engine transitions. The preferred high-level proof source is a
Lean-style abstract machine, with RTL assertions and model checking for local
FSM/refinement checks.

LNP64 is therefore not "POSIX in hardware." It is a capability substrate for
building operating systems, runtimes, drivers, and isolated services on one
small set of hardware-enforced primitives.
