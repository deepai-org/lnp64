# LNP64: A Capability Machine for System Software

LNP64 is a processor architecture built around a simple premise: files, memory,
synchronization, isolation, and service calls should be hardware-native
capability operations, not conventions layered entirely above a generic CPU.

The design is not a traditional kernel machine with a faster syscall path, and
it is not quite Unix on a chip. It does not freeze one historical kernel, VFS,
filesystem policy, network stack, or scheduler policy into hardware. Instead,
LNP64 provides the durable substrate that makes building modern Unix-like
systems simpler: files/resources as capabilities, waitable objects, VMAs,
hardware thread scheduling, event queues, domains, call gates, DMA isolation,
revocation, and generation checks.

It is a capability machine. Programs hold unforgeable File Descriptor Register
(FDR) capabilities for resources: streams, files, device objects, queues,
counters, memory objects, DMA buffers, PCIe BARs, call gates, event queues, and
delegated domains. Authority flows by explicit capability delegation, not
ambient access to global device memory or privileged kernel-only tables.

The core ISA remains a normal load/store architecture for ordinary computation.
Branches, calls, loads, stores, atomics, floating point, and vector operations
stay direct and fast. The difference is at the system boundary: instead of
trapping into an operating system for every resource operation, LNP64 exposes
fixed hardware commands for the common primitives that operating systems and
runtimes repeatedly rebuild.

The resource model is intentionally small:

- `PULL` and `PUSH` move records through streams, files, queues, devices, and
  event objects.
- `AWAIT` parks a hardware thread on a waitable object or memory predicate.
- `CAP_*` transfers, narrows, seals, and revokes authority.
- `MMAP` maps files, memory objects, DMA buffers, and device BARs through the
  VMA engine.
- `OBJECT_CTL` creates only three generic runtime primitives: `counter`,
  `queue`, and `memory_object`.
- `DOMAIN_CTL` creates nested Resource Domains for virtualization, cgroups,
  containers, jails, sandboxes, and supervisor personalities.
- `CALL_CAP` and `RET_CAP` provide protected cross-thread and cross-domain
  service calls through call-gate capabilities.
- `DMA_CTL` exposes bulk copy, fill, scatter/gather, and checksum-style work
  through the same safe memory and capability rules as device DMA.

Higher-level abstractions are profiles over these primitives, not new hardware
subsystems. A pipe is a queue profile. A semaphore or completion is a counter
profile. A shared arena is a memory-object profile. A VM, container, cgroup, or
sandbox is a Resource Domain profile. This keeps the hardware surface small
while making the primitives useful to normal applications, language runtimes,
drivers, and Unix compatibility layers.

Isolation is built from Resource Domains. Domains form a tree. A child domain
can only use resources delegated by its parent, and usage accounting rolls up
the tree. CPU budget, memory budget, PID/thread limits, FDR limits, I/O limits,
device authority, namespace roots, and upcall policy are all domain-scoped. This
makes virtualization and cgroups the same mechanism: a VM is a domain with
strong virtualization policy, while a cgroup is a domain focused on accounting
and limits. Domains provide scheduler policy and accounting, but ready/blocked
thread state, wait transitions, and dispatch remain owned by the hardware
scheduler and runqueue.

Security policy is native to the same model. W^X, NX data defaults, ASLR, guard
pages, hardware entropy, generation-checked objects, revocation, sealed and
narrowed capabilities, and DMA isolation are enforced by Resource Domains, VMAs,
FDRs, and the coherent DMA/IOMMU path rather than by an ambient privileged
kernel path.

The first FPGA target also keeps cloud-operability hooks in the architecture:
critical metadata ECC/parity, structured fault events, per-engine watchdog and
local reset paths, observability counters, a small trace ring, measured boot
metadata, Resource Domain snapshot hooks, and explicit storage flush/barrier
semantics. These are not a full fleet-management stack, but they keep reliability
and diagnosis from becoming afterthoughts.

Physical interrupt inputs still exist for devices, timers, DMA, PCIe MSI/MSI-X,
watchdogs, and hardware faults, but raw interrupt vectors are not exposed to
normal software or drivers. The Event Router consumes physical interrupts and
normalizes them into FDR-backed waitables, signals, scheduler wakeups,
trace/fault records, or supervisor/control events. Driver domains wait on
delegated `irq_event` capabilities; they do not own interrupt vectors.

Service boundaries are built from call gates. A pre-provisioned domain or worker
thread can expose a callable FDR. `CALL_CAP` validates the gate, transfers small
register arguments, accounts resource usage, and hands the target to the
hardware scheduler. The call may be synchronous, asynchronous, or a handoff. Cold
domain creation is still a real operation, but hot calls into already-created
isolated services can be made close to protected procedure calls.

LNP64 deliberately avoids the failure mode of earlier high-level processors:
putting rich object or language semantics into the hot path and making ordinary
code pay for descriptor walks, policy decisions, or hidden microcoded loops. The
core stays a simple load/store machine. Hardware primitives are justified only
when they own useful local state, enforce an invariant software cannot reliably
preserve, avoid memory traffic, or make an atomic transition cheap. Complex,
evolving policy remains in libc, runtimes, service domains, or Unix
personalities.

The hardware philosophy is conservative: modules are not hidden CPUs. They are
small, enumerated-state machines with local registers, FPGA RAM, tiny caches,
bounded transitions, generation checks, and commit/abort points. A hardware
module earns silicon only if it reduces memory traffic, improves streaming,
enforces capability/scheduling semantics, or shrinks the reachable bad-state
space compared with software.

The verification philosophy follows the same line. The long-term target is
seL4-like confidence for the hardware-visible security model: capability
non-forgeability, monotonic delegation, revocation soundness, domain
containment, W^X, DMA isolation, scheduler state validity, no lost wakeups, and
crash-free engine transitions. LNP64 should use Lean or a similar theorem-prover
for the abstract machine and security invariants, with RTL assertions and model
checking reserved for local handshake, FSM, and refinement checks. The design
goal is that important guarantees are either proven, locally checkable, or
structurally impossible to violate.

The result is a system architecture where the operating-system boundary becomes
less special. Files, queues, memory mappings, timers, futexes, devices,
containers, virtual machines, and service calls are all forms of capability
objects with waitable state and explicit authority. LNP64 is therefore not just
"POSIX in hardware." It is a capability substrate for building operating
systems, runtimes, drivers, and isolated services on the same small set of
hardware-enforced primitives.
