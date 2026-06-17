# LNP64 Formal Theorem Roadmap

This document lists the high-level proofs that would be most valuable for
LNP64. The intended proof source is a Lean-style abstract machine model, with
RTL assertions and model checking used later for local refinement checks.

The guiding rule is: authority-bearing behavior should be proven correct,
locally checkable, or structurally impossible to violate.

## 1. Global State Validity

**State preservation:** if the machine state is valid and an architectural
transition succeeds or fails, the resulting state is valid.

Useful sub-theorems:

- every live PID/TID references a valid process/thread record.
- every runnable thread is in exactly one runqueue position.
- no thread is both running and blocked.
- every FDR entry either is invalid or references a live object/generation.
- every VMA belongs to exactly one process address space.
- every wait queue entry references a live blocked thread or is invalid.
- every Resource Domain has a valid parent/generation except the root domain.

## 2. Capability Non-Forgeability

**Non-forgeability:** user code cannot manufacture authority by writing integer
values, memory bytes, registers, trace records, event payloads, or message
payloads.

Only the Capability/FDR engine, object owner engines, boot engine, and explicitly
authorized mint capabilities can create authority-bearing FDR entries.

Useful sub-theorems:

- GPR values are never interpreted as capabilities without FDR-table lookup.
- memory stores cannot create valid FDR entries.
- received capabilities must originate from `CAP_SEND` or an authorized object
  mint path.
- object ids without matching generation and rights confer no authority.
- trace, counter, classifier, event, and fault records are data, not
  capabilities.

## 3. No Authority Amplification

**Monotonic delegation:** capability operations can preserve or narrow authority,
but cannot broaden it.

Useful sub-theorems:

- `CAP_DUP` cannot add rights, ranges, event masks, transfer rights, or mapping
  permissions.
- `CAP_SEND` cannot grant more authority than the sender held.
- sealed capabilities cannot be unsealed, narrowed, duplicated, reminted, or
  inspected unless their explicit rights permit it.
- child Resource Domains cannot receive broader authority than the parent
  delegated.
- classifier tables, namespace roots, packet filters, endpoint capabilities, and
  network namespaces can only narrow delegated authority.

## 4. Revocation Soundness

**Revoked authority cannot start new work:** once revocation reaches its commit
point, no descendant capability in that revocation lineage can start a new
operation.

Useful sub-theorems:

- cached FDR descriptors observe revocation before accepting new operations.
- mapped VMAs derived from revoked object authority are invalidated or
  generation-mismatched.
- event source bindings derived from revoked authority stop delivering new
  events.
- call gates derived from revoked authority cannot accept new calls.
- DMA exports derived from revoked authority reject new descriptors.
- classifier tables, network endpoints, packet queues, listeners, and namespace
  roots derived from revoked authority reject new use.

## 5. Generation Safety

**No stale aliasing:** a stale handle cannot accidentally authorize access to a
new object that reused the same table slot or object id.

Useful sub-theorems:

- object reuse increments generation before publishing new authority.
- FDR, VMA, waitable, call-gate, classifier, packet-queue, DMA-buffer, network
  endpoint, namespace, and domain operations check object generation before use.
- uncorrectable metadata faults do not advance generation into a fresh-looking
  valid object without supervisor acknowledgement.
- serialized checkpoint/restore records cannot revive stale generations.

## 6. Resource Domain Containment

**Domain containment:** a child domain cannot exceed the authority, security
policy, or budgets delegated by its ancestors.

Useful sub-theorems:

- limits are monotonic down the domain tree.
- usage rolls up to all ancestors.
- freeze/kill/revoke on a domain subtree reaches every attached descendant.
- ASLR/JIT/DMA/device/network/entropy/security policy in a child can only be
  stricter than the delegated parent policy.
- supervisor upcall policy can be masked or translated by parents but not used
  to escape containment.
- domain-scoped classifier tables, network namespaces, and device capabilities
  cannot target resources outside delegated domain authority.

## 7. Scheduler Safety

**Scheduler well-formedness:** hardware scheduling never loses or duplicates a
thread context.

Useful sub-theorems:

- every live thread is in exactly one scheduler state: running, runnable,
  blocked, parked, exiting, or destroyed.
- a thread can run only if its Resource Domain is not frozen and has dispatch
  budget.
- `AWAIT` atomically transitions a thread from running to blocked or returns
  ready without losing an event.
- `WAKE`, event delivery, signal delivery, fd readiness, timer expiry, call
  completion, classifier queue routing, and domain resume transition eligible
  threads back to runnable at most once.

## 8. No Lost Wakeups

**Wait/wake atomicity:** if a waitable becomes ready concurrently with a thread
arming a wait, either the wait returns immediately or a future wake/event is
recorded.

Useful sub-theorems:

- event queue add-source performs install, generation snapshot, readiness check,
  and arm as one atomic transition.
- futex wait validates the expected memory value and installs the waiter without
  a lost race against `WAKE`.
- call-gate completion cannot be delivered before the caller continuation is
  armed.
- timer expiry either enqueues an event or marks the waitable ready.
- classifier route-to-queue actions either enqueue a record and wake waiters or
  report/coalesce overflow according to queue policy.

## 9. Object Profile Refinement

**Profiles preserve primitive invariants:** higher-level object profiles behave
as refinements of the small native object primitives.

Useful sub-theorems:

- pipe profiles are queues with narrowed read/write endpoint capabilities.
- event queues are queues plus source bindings and generation snapshots.
- timer profiles are waitable/counter sources driven by hardware time.
- semaphore, completion, channel, and task-queue profiles preserve
  counter/queue rights and wakeup invariants.
- listener profiles are accept queues that return endpoint capabilities only
  through authorized object creation.
- datagram/stream/socket compatibility profiles preserve endpoint capability
  rights and event semantics.

## 10. Namespace Capability Containment

**Path lookup cannot escape delegated namespace authority:** path strings are
names, not authority.

Useful sub-theorems:

- `OPEN_AT` resolves only relative to a held directory/root capability or the
  process/domain delegated cwd/root.
- `..`, symlinks, bind/delegated mounts, synthetic namespace nodes, and guest
  namespace upcalls cannot escape the root capability.
- path lookup returns only object capabilities permitted by the namespace root,
  credentials, object metadata, and Resource Domain policy.
- delegated namespace upcalls cannot mint broader authority than the parent
  namespace capability allowed.
- POSIX global path syntax is a compatibility profile over explicit namespace
  root capabilities.

## 11. VMA and Memory Safety

**VMA protection:** memory accesses succeed only through a valid VMA with
compatible permissions and generation.

Useful sub-theorems:

- no load/store/fetch succeeds through unmapped memory.
- guard VMAs reject load, store, fetch, and DMA pin.
- NX mappings reject instruction fetch.
- writable mappings are not executable unless explicit domain policy permits
  the transition.
- `MUNMAP` and `MPROTECT` invalidate stale TLB/I-cache translations before
  affected threads resume.
- memory-object and DMA-buffer mappings cannot exceed the mapped capability's
  allowed range, direction, or memory type.

## 12. W^X and Executable Provenance

**W^X invariant:** no page is simultaneously writable and executable unless the
current Resource Domain has explicit JIT/loader authority and the mapping is in
an allowed transition state.

Useful sub-theorems:

- ordinary anonymous, heap, stack, queue, shared-memory, DMA, and device
  mappings are NX by default.
- executable mappings originate from executable image objects or authorized
  loader/JIT transitions.
- `ISYNC` cannot make non-executable memory executable by itself.
- BAR, DMA-buffer, packet-buffer, signal-frame, and queue mappings are never
  executable in v1.

## 13. DMA Isolation

**DMA confinement:** no DMA operation can read or write memory outside the
capability, VMA, IOMMU, direction, and Resource Domain scope that authorized it.

Useful sub-theorems:

- `DMA_CTL` translates user virtual addresses through the VMA engine.
- device-visible DMA requires a valid `dma_buffer` FDR.
- DMA pinning rejects guard, unmapped, stale, revoked, or direction-incompatible
  memory.
- PCIe IOMMU contexts include requester id, domain/generation, buffer
  generation, ranges, and direction.
- revocation of a DMA buffer prevents new descriptors before backing pages are
  released.
- packet DMA and network driver DMA follow the same confinement rules as
  ordinary `DMA_CTL`.

## 14. Raw Interrupt Non-Exposure

**Raw interrupt vectors are not software authority:** physical interrupt inputs
are consumed by hardware routing and exposed only as capability-scoped events,
signals, scheduler wakeups, or structured fault/control records.

Useful sub-theorems:

- device IRQ/MSI/MSI-X messages enter the Event Router, not a software-owned
  vector table.
- normal software and driver domains cannot install or receive raw interrupt
  vectors.
- `irq_event` records are delivered only to holders of the corresponding event
  capability and generation.
- revoking an `irq_event` capability prevents new delivery to that domain.
- machine-check/panic/debug exceptions remain outside the normal driver
  interrupt model.

## 15. Network Capability Containment

**Network authority is capability-scoped:** network access requires delegated
`net_namespace`, `net_interface`, `packet_queue`, endpoint, listener, or related
capabilities.

Useful sub-theorems:

- a domain without a network capability has no ambient network access.
- packet queues receive only packets matching delegated interface/filter
  authority.
- bind, listen, connect, raw packet access, multicast/broadcast, and privileged
  port behavior cannot exceed namespace capability policy.
- accepted connections are endpoint capabilities and can only be passed by
  capability transfer.
- revoking namespace/interface authority revokes or generation-invalidates
  derived endpoints, listeners, packet queues, filters, and network events.
- POSIX sockets are a compatibility profile over endpoint and listener
  capabilities, not an alternate authority path.

## 16. Classifier Safety

**Bounded record classification cannot broaden authority or create unbounded
execution:** classifier tables classify and steer records only within delegated
source and destination capability scopes.

Useful sub-theorems:

- classifier tables are capabilities with owner domain, generation, source
  scope, destination queue set, and bounded rule limits.
- rule installation cannot name unauthorized source objects or destination
  queues.
- rule actions can only mark, count, drop, timestamp, hash-steer, or route
  records within authorized queues.
- malformed, unknown, too-deep, encrypted, fragmented, or unsupported records
  become `partial`, `unknown`, `needs_software`, dropped, or faulted according
  to policy; they do not produce undefined authority.
- classifier execution has no loops, recursion, arbitrary bytecode, unbounded
  parse, unbounded rule walk, mutable protocol state, or connection tracking.
- classifier marks, counters, flow hashes, and routed record envelopes are data,
  not capabilities.

## 17. Event, Signal, and Fault Delivery Safety

**Events and faults are delivered to the right authority scope:** synchronous
faults, asynchronous events, and POSIX signal compatibility frames are
well-formed and cannot forge privilege or capabilities.

Useful sub-theorems:

- divide-by-zero, illegal opcode, permission fault, guard fault, and bad device
  mapping produce the specified signal/upcall/fault record.
- POSIX signal delivery is a compatibility profile over hardware event delivery.
- signal frames are non-executable and cannot forge privilege or capability
  state.
- `SIGRET` restores only the hardware-saved context for that interrupted
  thread.
- supervisor opcode upcalls cannot bypass normal capability/domain checks.
- event/fault records are scoped to the owning domain or configured supervisor
  FDR and do not leak unauthorized payloads.

## 18. Cross-Domain Call Safety

**Call-gate safety:** `CALL_CAP` transfers control, arguments, accounting, and
optional capabilities only as authorized by the call-gate FDR.

Useful sub-theorems:

- synchronous calls park exactly one caller continuation.
- asynchronous calls produce at most one completion per accepted operation id.
- handoff calls transfer cancellation/accounting ownership according to gate
  policy.
- reentrant call depth is bounded.
- capability-marked arguments are rejected unless the gate permits capability
  passing.
- cross-domain calls cannot bypass child/parent domain budget and authority
  checks.

## 19. Snapshot/Restore Safety

**Checkpoint metadata cannot duplicate or revive authority:** freeze, query,
resume, and future restore hooks preserve containment and generation safety.

Useful sub-theorems:

- `freeze` reaches a quiescent boundary before exportable state is observed: no
  running threads, no new DMA descriptors, no in-progress metadata commits, and
  no unaccounted call-gate continuations.
- `query-state` is observation only and cannot mutate authority.
- `resume` restarts a quiesced domain without generation churn.
- future `restore` creates fresh domain ids/generation bases and requires
  explicit capability reattachment.
- serialized stale capabilities cannot revive revoked or destroyed authority.
- dirty-memory tracking hooks cannot grant read/write access outside the
  snapshotted domain.

## 20. Commit/Abort Atomicity

**No partial publication:** multi-step hardware operations expose either the old
state or the committed new state, never an inconsistent middle state.

Useful sub-theorems:

- `OPEN_AT`, `NS_CTL`, `SET_META`, `MMAP`, `MUNMAP`, `CLONE`, `EXEC`,
  `DOMAIN_CTL`, `OBJECT_CTL`, `CAP_REVOKE`, `CALL_CAP`, classifier table
  updates, and network endpoint creation each have a single architectural commit
  point.
- cancellation before commit rolls back all reservations.
- cancellation after commit completes, rolls forward, or reports a defined
  failure without corrupting authority-bearing metadata.
- result registers and compatibility `ERRNO` are updated only at completion.

## 21. Filesystem Durability Contract

**Durable metadata ordering:** after a successful synchronous metadata barrier,
the storage image can recover to a state that includes the committed operation
or a defined replay/fsck result.

Useful sub-theorems:

- atomic rename is power-fail atomic under the selected log/journal/COW
  protocol.
- metadata commit records are written before commit publication.
- storage barriers order prior data writes, metadata writes, and backend flush
  completion.
- live-system atomicity and power-fail durability have separate proof
  obligations.

## 22. Boot Measurement Integrity

**Measured boot consistency:** the boot measurement log accurately reflects the
boot manifest, selected images, reset cause, FPGA build id, and boot policy
observed by PID 1.

Useful sub-theorems:

- measurement records are append-only during boot.
- PID 1 cannot observe a booted image without the corresponding measurement
  record being present.
- boot policy failure either records a structured fault and enters permitted
  development mode or enters hardware panic.
- boot-control FDR reads cannot alter measurement records or mint authority.

## 23. RAS Fault Containment

**Detected corruption does not silently become authority:** ECC/parity,
watchdog, and metadata faults are either corrected, poisoned, reported, or
panic the machine.

Useful sub-theorems:

- correctable metadata faults preserve object identity and generation.
- uncorrectable metadata faults poison affected objects before reuse.
- local engine reset does not publish partial state.
- degraded engines reject new commands until supervisor/PID 1 policy clears
  them.
- trace/fault records cannot grant authority.

## 24. Trace and Counter Non-Interference

**Observability does not change authority:** reading counters, trace rings, or
fault records cannot create, broaden, revive, or revoke capabilities.

Useful sub-theorems:

- trace records are data, not FDR entries.
- destructive trace reads only advance trace-consumer cursors.
- counter overflow or wrap cannot affect scheduler, domain, VMA, or capability
  state.
- trace/counter access is scoped by Resource Domain and control-FDR authority.
- classifier and network counters cannot leak unauthorized packet payloads.

## 25. POSIX/Profile Compatibility Refinement

**Compatibility APIs are refinements of native primitives:** POSIX, libc, Linux
syscall compatibility, and NetBSD rump-style interfaces cannot bypass native
capability/event/domain authority.

Useful sub-theorems:

- POSIX file descriptors are FDR capability handles plus compatibility metadata.
- `fork` is a `CLONE` profile and cannot duplicate authority beyond FDR/domain
  inheritance rules.
- POSIX signals are an event-delivery profile and cannot bypass capability or
  domain checks.
- UID/GID and mode bits participate in compatibility decisions but cannot
  replace required capabilities.
- `errno` is a compatibility view of explicit result/error status.
- socket APIs refine endpoint/listener/network namespace capabilities.
- ioctl-like controls refine typed metadata/control records or fail; they do
  not form a separate authority path.

## 26. Paravirtual Personality Containment

**Guest personality containment:** Linux/NetBSD-style personality domains can
emulate richer policy for their children, but cannot bypass hardware authority
and resource checks.

Useful sub-theorems:

- unsupported-opcode upcalls do not grant ambient memory/device access.
- guest-created tasks remain hardware threads or domains subject to parent
  budgets.
- guest filesystems mounted inside block-image FDRs cannot access storage
  outside that FDR.
- guest cgroups/containers map to child domains whose limits remain monotonic.
- guest network stacks can receive packet queues or virtio-like capabilities,
  but cannot access packets, interfaces, or DMA outside delegated authority.
- Linux syscall compatibility is a personality/runtime profile over native
  operations, not a privileged syscall escape hatch.

## 27. Confidentiality and No Unauthorized Observation

**A domain cannot observe data outside delegated authority:** memory, metadata,
events, packets, counters, traces, and fault records are readable only through
authorized capabilities and domain policy.

Useful sub-theorems:

- VMA permissions and FDR capabilities mediate memory and object reads.
- packet queues expose only packets allowed by delegated interface/filter
  authority.
- RAS/fault records do not include unauthorized payload bytes.
- trace and counter records are scoped by control capability and domain policy.
- scheduler/domain pressure counters expose only permitted aggregate data.
- classifier records, marks, hashes, and counters do not leak unauthorized
  packet/message payloads.

## 28. Refinement Targets

After the abstract theorems exist, each hardware block should prove or test a
small refinement claim:

- its RTL-visible state maps to an abstract object state.
- every accepted command corresponds to an allowed abstract transition.
- every response corresponds to the abstract transition result.
- every timeout, reset, or fault maps to an allowed abort/degraded/panic
  transition.
- no local cache hit can bypass generation, rights, domain, revocation,
  namespace, network, classifier, or DMA checks required by the abstract model.

These refinement claims are narrower than the whole architecture proof, but they
are what make the abstract Lean model relevant to an FPGA implementation.
