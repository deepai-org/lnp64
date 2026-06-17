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

POSIX is the primary compatibility profile, not the primitive architecture.
`fork`, signals, paths, UID/GID, `errno`, file descriptors, and `ioctl`-like
controls are mapped by libc or a Unix personality onto capability handles,
waitable objects, event queues, Resource Domains, VMAs, and typed metadata
operations. This keeps real software easy to port without making historical Unix
quirks the center of the hardware model.

Control surfaces are typed, versioned, and bounded. `GET_META`, `SET_META`,
`OBJECT_CTL`, `DOMAIN_CTL`, `NS_CTL`, socket options, storage barriers, and
service controls use a common control envelope with object class, profile id,
profile class, op id, flags, required rights, bounded input/output lengths,
scalar fields, capability arguments, and expected generation/lineage epoch.
Unknown operations fail as unsupported; control records cannot smuggle ambient
authority or raw driver command blobs.

The envelope is not a prettier `ioctl`. Architectural profiles are stable
hardware controls for domains, queues, counters, memory objects, telemetry,
attestation, storage barriers, and classifiers. Personality/service profiles
cover Unix compatibility, namespaces, sockets, loaders, and service metadata.
Vendor/device profiles are allowed only behind explicit device capabilities.
All profiles remain bounded, typed, versioned, fail-closed, and subject to
Capability Engine verification for any returned FDR authority.
Every control op has one commit point. Payload bytes are data only; capability
arguments are explicit FDR references; returned capabilities appear only in
declared returned-capability slots. Unknown versions, profile classes, ops,
flags, and returned-capability shapes fail before side effects.

`fork()` is not the conceptual center of process creation. Native `CLONE` has
explicit profiles for threads, processes, and POSIX fork compatibility. The
fork profile is intentionally narrow: one child thread, COW VMAs, defined FDR
inheritance, copied credentials/dispositions, no in-flight operation ownership,
and no hidden runtime lock semantics.

Signals are the exception where LNP64 intentionally freezes a clean Unix subset:
handler/default/ignore disposition, process-wide dispositions, per-thread masks,
process-directed and thread-directed pending state, precise fault-to-signal
mapping, checked `kill`/`raise`, `alarm`, fixed handler entry, and `SIGRET`.
Synchronous faults target the faulting thread. Process-directed signals use a
deterministic eligible-thread selection rule or remain pending. `SIGRET`
restores only from a Signal Engine-owned context token/generation; the visible
frame is diagnostic/runtime ABI data, not authority. Full realtime queues,
OS-specific restart quirks, and legacy delivery corner cases remain personality
policy.

Path and filesystem semantics are deliberately service-owned. Hardware mediates
`OPEN_AT`, namespace control, and returned capability installation, but it does
not implement a general writable filesystem, directory walker, symlink policy,
or inode model. A filesystem service, Unix personality, or rump kernel owns
those rules and returns narrowed capabilities through the same FDR mechanism as
every other service.

Service-owned policy does not imply service-minted authority. Namespace,
filesystem, PCIe, network, loader, and supervisor services may propose returned
capabilities, but the Capability Engine derives and installs FDR authority only
from an existing mint/root capability after checking class, rights, range,
generation, lineage, and domain policy. A compromised service can only misuse
authority it was explicitly delegated; it cannot manufacture a broader FDR by
writing a reply record.

This is the native service model. Hardware owns authority, scheduling,
waitability, memory safety, accounting, and commit semantics. Services own
evolving policy. A service receives bounded requests through call gates, queues,
event queues, namespace dispatch, typed controls, page-fill requests, or
`PULL`/`PUSH`; replies are data until hardware validates status, output shape,
service generation, and returned-capability proposals. Service crash,
revocation, caller cancellation, and queue pressure have typed outcomes rather
than ad hoc daemon behavior. Backpressure is explicit: full queues either park,
return `EAGAIN`, or fail with a bounded overflow status. No service gets raw
interrupts, raw DMA, raw physical memory, ambient device authority, or a private
capability table.

Executable formats follow the same boundary. Hardware `EXEC` commits a prepared
exec-plan descriptor atomically; software loaders own ELF, dynamic linking,
relocations, interpreters, auxv conventions, library policy, and credential
transition rules.

Unix personalities use a narrow native interface: lifecycle events, VMA events,
FDR/capability transfer, namespace dispatch, block-image objects, signal/fault
records, futex/timer/event queues, network endpoints, PCIe BAR/DMA/IRQ-event
capabilities, and domain-control upcalls. Linux or NetBSD may project rich kernel
semantics over those surfaces, but not take ownership of raw page tables,
interrupt vectors, DMA, scheduler dispatch, or capability minting.

Networking follows the same rule. Silicon provides a TCP-friendly transport
substrate: PCIe Root Complex support, IOMMU-scoped DMA buffers, BAR
capabilities, `irq_event` records, packet queues, endpoint capabilities, event
delivery, counters, checksum/hash assists, timer/counter objects, zero-copy
handoff, and simple MAC/packet movement. A bounded record classifier can stamp
metadata, compute hashes, count, and steer packets, IPC messages, storage
completions, trace records, or runtime events into capability-scoped queues.
Ethernet, Wi-Fi, TCP/IP, routing, firewall, TLS, DNS, and socket policy live in
driver or network service domains that publish endpoint capabilities. A future
TCP accelerator may exist only behind the same `stream_endpoint` shape; POSIX
sockets are a compatibility profile over those endpoints.

The endpoint contract is typed and implementation-independent. Packet queues
preserve packet records; datagram endpoints preserve message boundaries; stream
endpoints expose ordered bytes and backpressure; listeners return endpoint
capabilities. Applications see capabilities, readiness, metadata, and typed
errors, not whether software TCP, local IPC, QUIC, paravirtual networking, or a
future accelerator is doing the transport work.

Isolation is built from Resource Domains. Domains form a tree. A child domain
can only use resources delegated by its parent, and usage accounting rolls up
the tree. CPU budget, memory budget, PID/thread limits, FDR limits, I/O limits,
device authority, namespace roots, and upcall policy are all domain-scoped. This
makes virtualization, containers, cgroups, jails, and sandboxes the same
mechanism. `DOMAIN_CTL create child` is the operation for all of them. A VM is a
domain with stronger supervisor/upcall policy and paravirtual device views; a
container is a domain that shares more parent personality/runtime state with
narrower namespaces and capabilities; a cgroup is a domain focused on accounting
and limits. Hardware sees the same containment algebra in every case. Domains
provide scheduler policy and accounting, but ready/blocked thread state, wait
transitions, and dispatch remain owned by the hardware scheduler and runqueue.
The scheduler is a weighted-fair virtual-time machine inspired by CFS/EEVDF:
domains and threads carry weights, quotas, virtual runtime/deadlines, and
bounded latency classes. Software sets policy intent; hardware owns the
runqueue, wakeups, no-lost-wakeup transitions, accounting, and bounded dispatch.
It is not Linux CFS in RTL and exposes no scheduler plugins or callbacks:
weights, quotas, affinity, and latency class are inputs to one fixed hardware
algorithm.

Boot starts inside a default operating envelope. Reset creates the root domain,
PID 1 domain, default scheduler profile, security defaults, telemetry/fault
routes, boot measurements, and explicit initial capabilities before the first
user instruction. PID 1 configures services; it does not rescue an unconfigured
machine or create the authority model from scratch.

Security policy is native to the same model. W^X, NX data defaults, ASLR, guard
pages, hardware entropy, generation-checked objects, revocation, sealed and
narrowed capabilities, and DMA isolation are enforced by Resource Domains, VMAs,
FDRs, and the coherent DMA/IOMMU path rather than by an ambient privileged
kernel path.

Normal cached memory is intentionally developer-friendly: coherent and TSO-like
by default. Locked atomics are sequentially consistent in v1, futex wait/wake has
explicit acquire/release-style ordering, and `FENCE` is the boundary for DMA,
engine completions, and device memory. Drivers opt into `device_ordered`,
`uncached`, or `write_combining` mappings when they need device behavior; normal
programs and Unix personalities get the simpler default.

For cloud and government deployments, the same machinery gives a named
tenant-strict profile. `DOMAIN_PROFILE_TENANT_STRICT` requires mandatory memory
hardening, no ambient devices, no raw interrupts, scoped DMA, explicit shared
pages, scoped telemetry, and no parent memory inspection without a delegated
capability. Confidential-domain hooks add measured launch, memory-encryption
key-id metadata, sealed-secret release policy, and encrypted-checkpoint metadata
without changing the Resource Domain model.

Assured deployments are named profiles, not after-the-fact hardening scripts.
`ASSURANCE_DEV`, `ASSURANCE_FIELD`, `ASSURANCE_HIGH`, and `ASSURANCE_FORMAL`
bind measured boot, quote records, debug lockdown, metadata ECC/parity,
watchdogs, telemetry, audit roots, MLS labels, proof artifact hashes, RTL/IP
provenance, and toolchain/build ids into Resource Domain policy and remote
attestation. Hardware is the Policy Enforcement Point; PID 1, orchestration,
personalities, and services are Policy Decision Points whose requests become
real only after capability, label, lineage, generation, measurement, and domain
checks.

Audit, debug, and cross-domain controls follow the same capability rule.
Tamper-evident audit streams are append-only event logs with sequence numbers,
hash chaining, scoped disclosure, redaction, dropped-count metadata, and
quoteable roots. Debug and forensics require explicit debug-control FDRs,
measured unlocks, audit records, and domain/range/label-scoped rights; production
profiles can permanently disable invasive debug. MLS deployments attach labels
to domains, FDRs, memory objects, DMA buffers, endpoints, telemetry, and audit
streams. Declassification is an explicit audited service path, not a parent
privilege or debug shortcut.

Mission assurance is the same idea applied to continuity. A mission workload is
a Resource Domain profile with an assurance floor, audit/attestation
requirements, dependency graph hash, delegated fallback capabilities, allowed
degraded modes, recovery priority, stale-data budget, and fail policy. Hardware
does not plan the mission; it enforces the small state machine: normal,
degraded, recovering, frozen, failed closed, or quarantined. Service restarts,
watchdog faults, revoked dependencies, audit or attestation failures, and label
violations cannot broaden authority during recovery. Quotes can bind the current
mission state, dependency graph, audit root, proof artifacts, and delegated
capability roots, so continuity under failure becomes evidence, not a promise.

Revocation is one algebra across the machine. Capabilities carry object
generation, capability generation, lineage root, lineage epoch, rights, ranges,
and domain scope. Narrowing and sending preserve lineage; sealing hides software
inspection but not hardware lineage; revocation advances an epoch and invalidates
derived FDR cache entries, VMAs, event bindings, call gates, DMA/IOMMU contexts,
packet queues, namespace roots, and page-fill continuations before new work can
start through them.

Revocation is classed so it stays fast without becoming unsafe. `lazy_epoch`
invalidates cached authority and readiness bindings cheaply. `forced_cancel`
wakes waits and aborts pre-commit work. `synchronous_quiesce` is required before
reusing DMA buffers, IOMMU contexts, BAR mappings, or pages. `poison_fault`
handles corrupted or untrusted stale metadata until supervisor/PID 1 policy
clears or destroys the object.

The VMA/Page Engine freezes a small page-state machine: unmapped, reserved,
nonresident object, fill pending, resident clean/dirty, COW shared, DMA pinned,
revoking, and poisoned. Hardware owns permission checks, COW, zero fill,
shootdown, pinning, revocation, atomic page install, deterministic race
priority, and explicit commit/abort points. Object owners provide contents and
writeback semantics, so LNP64 gets provable memory safety without turning the VMA
engine into a filesystem page-cache kernel.

Object-backed mappings use a fixed page transaction protocol. Hardware sends a
bounded page request with VMA/object generation, lineage, offset, permissions,
memory type, executable provenance, and domain identity. The service can return
a page capability, zero page, shared page, retry token, or error. Hardware
installs only if the original authority still matches; dirty writeback,
`msync`, truncation, and `PULL`/`PUSH` coherence remain service policy.

The first FPGA target also keeps cloud-operability hooks in the architecture:
critical metadata ECC/parity, structured fault events, per-engine watchdog and
local reset paths, telemetry capabilities, a small trace ring, remote
attestation records, Resource Domain checkpoint/live-migration compatibility
hooks, line-rate record classification and queue steering, and explicit storage
flush/barrier semantics. These are not a full fleet-management stack, but they
keep reliability, diagnosis, and tenant trust from becoming afterthoughts.

Observability is useful without privileged scraping. Counters, trace rings,
pressure events, and fault streams are exposed through narrowed FDR telemetry
capabilities, so a monitoring agent can receive aggregate or per-domain views
without raw memory access, raw interrupt vectors, or global debug authority.

Attestation is a first-class capability surface. A quote FDR exposes measured
FPGA/ROM identity, boot manifest hash, image measurements, domain launch
measurements, selected policy, and delegated capability roots. Production
implementations can sign those records through a board-rooted key; FPGA
development builds can expose explicit non-production quotes.

Checkpointing stays faithful to the capability model. Hardware can freeze a
domain subtree, expose bounded query-state records, reserve dirty-memory hooks,
and resume the same domain without generation churn. Checkpoint image formats,
migration transport, device/service state capture, and restore policy remain
software-owned; endpoint drain/redirect hooks and service callback events make
live migration practical without teaching hardware TCP, filesystems, TLS, or
application protocols. A future restore reattaches explicit capabilities into a
fresh domain under normal generation and lineage checks.

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

When a mechanism is stable and widely accepted, LNP64 can freeze a good
substrate into silicon without exposing its representation. The native heap is
the model: applications issue `ALLOC`, `ALLOC_EX`, `ALLOC_SIZE`, and `FREE`
with allocation intent and bounded policy hints. The Heap Engine implements the
LNP64 Default Heap Algorithm: a domain-aware segregated bump allocator with
fixed size classes, per-thread allocation windows, slab/run pages, batched
cross-thread frees, page-run large objects, checked metadata, generation checks,
and bounded hardening internally. The rule is not "never put policy in hardware";
it is "only freeze policy when the frozen design is simple, proven, and unlikely
to be worse than what programmers would write." Raw freelists, allocation-window
depths, slab layout, quarantine algorithm, profiling, GC, and language object
policy stay out of the ISA.

The allocation boundary is explicit. `ALLOC`/`FREE` create hardware-owned
allocation objects, so the machine can enforce object-level safety, accounting,
invalid-free detection, and hardening. `MMAP`, `memory_object`, and arena-style
`ALLOC_EX` create software-owned arenas, so the machine enforces the outer
region while runtimes own the inner representation. The architecture guarantees
safety at the granularity it owns.

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
