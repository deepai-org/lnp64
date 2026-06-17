# LNP64 Formal Theorem Roadmap

This document lists the high-level proofs that would be most valuable for
LNP64. The intended proof source is a Lean-style abstract machine model, with
RTL assertions and model checking used later for local refinement checks.

The guiding rule is: authority-bearing behavior should be proven correct,
locally checkable, or structurally impossible to violate.

## 0. Formal Model Scope

The first formal model should be an architectural abstract machine, not a gate
or timing model. Its initial proof boundary is:

- instruction decode for the frozen base ISA formats and opcode/profile
  dispatch.
- GPR/FDR/PCR/thread/process state needed for architectural transitions.
- capability tables, generations, lineage epochs, narrowing, sealing, transfer,
  returned-capability install, and revocation.
- Resource Domain tree state, monotonic delegation, budgets, usage accounting,
  freeze/resume/destroy, and lifecycle generations.
- VMA/page states, TLB-visible permissions, W^X/NX/guard checks, COW,
  object-backed page-fill commit, DMA pin/unpin, and revocation races.
- the TSO-like memory consistency contract at the abstract event level:
  ordinary loads/stores, locked atomics, `FENCE`, `ISYNC`, engine completions,
  coherent DMA, and device-memory ordering classes.
- mandatory object profiles: `counter`, `queue`, `event/completion`, `timer`,
  `memory_object`, `call_gate`, `dma_buffer`, and `dma_completion`.
- wait/ready/runqueue state, scheduler eligibility, no-lost-wakeup transitions,
  timeout events, gate delivery, continuation return, and fault delivery.
- service-boundary request/reply records, continuation ids, copied/pinned
  buffers, returned-capability proposals, commit points, and canonical error
  outcomes.
- fault, poison, overflow, watchdog, local-reset, trace/audit/telemetry records
  as data, never authority.

The first model intentionally excludes:

- cycle timing, cache replacement policy, branch pipeline timing, physical DDR
  controller timing, Ethernet/PCIe electrical behavior, and FPGA-specific CDC.
- filesystem formats, executable formats, dynamic-linker behavior, TCP/IP
  protocol policy, PCIe device quirks, orchestration policy, and Linux/BSD ABI
  compatibility details.
- performance optimality. Proofs should establish safety, containment,
  determinism at architectural commit points, and specified liveness/fairness
  bounds where the architecture promises them.

RTL assertions and bounded model checking should later prove that each hard
block refines the abstract transition relation for its owned state machine.

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

Only the Capability/FDR engine, object owner engines, boot engine, and
class-scoped mint/root capabilities can create authority-bearing FDR entries.
Software service replies are data until the Capability Engine validates and
commits a derived capability.

Useful sub-theorems:

- GPR values are never interpreted as capabilities without FDR-table lookup.
- memory stores cannot create valid FDR entries.
- received capabilities must originate from `CAP_SEND` or an authorized object
  mint path.
- namespace, filesystem, network, PCIe, loader, and supervisor services cannot
  broaden authority by returning a capability proposal; installation succeeds
  only if the proposal derives from authority already delegated to that service.
- mint/install validates object class, rights, ranges, generations, lineage,
  receiver domain policy, and object-specific constraints before publishing the
  FDR entry.
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

- every authority-bearing cached record carries or can validate object
  generation, capability generation, lineage root, and lineage epoch.
- `CAP_REVOKE` advances the lineage or revocation-root epoch before descendant
  authority can be reused.
- cached FDR descriptors observe revocation before accepting new operations.
- mapped VMAs derived from revoked object authority are invalidated or
  generation-mismatched.
- event source bindings derived from revoked authority stop delivering new
  events.
- call gates derived from revoked authority cannot accept new calls.
- DMA exports derived from revoked authority reject new descriptors.
- classifier tables, network endpoints, packet queues, listeners, and namespace
  roots derived from revoked authority reject new use.
- waiters on revoked sources wake with a revoke/error event.
- operations before their commit point abort with the chosen revoked/stale
  error; operations after commit complete, roll forward, or follow documented
  teardown policy, but later use sees stale generation/epoch.
- every revoked object follows one architectural revocation class:
  `lazy_epoch`, `forced_cancel`, `synchronous_quiesce`, or `poison_fault`.
- `lazy_epoch` revocation is sufficient only when stale cached records cannot
  start new authority-bearing work after the epoch advance.
- `forced_cancel` revocation wakes or aborts pre-commit waits, page fills,
  queued calls, and async operations with a revoke/cancel status.
- `synchronous_quiesce` revocation blocks new DMA, pinning, page reuse, BAR
  mapping, or domain dispatch before reuse can occur.
- `poison_fault` revocation prevents recycling corrupted authority as fresh
  authority without supervisor/PID 1 acknowledgement.

## 5. Generation Safety

**No stale aliasing:** a stale handle cannot accidentally authorize access to a
new object that reused the same table slot or object id.

Useful sub-theorems:

- object reuse increments generation before publishing new authority.
- FDR, VMA, waitable, call-gate, classifier, packet-queue, DMA-buffer, network
  endpoint, namespace, and domain operations check object generation before use.
- uncorrectable metadata faults do not advance generation into a fresh-looking
  valid object without supervisor acknowledgement.
- checkpoint records and future reattachment records cannot revive stale
  generations.

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
- VM, container, cgroup, jail, sandbox, and supervisor profiles are all refinements
  of the same child-domain creation transition; profile metadata cannot weaken
  tree topology, monotonic limits, capability lineage, accounting rollup, or
  generation checks.

## 7. Scheduler Safety

**Scheduler well-formedness and bounded fairness:** hardware scheduling never
loses or duplicates a thread context, and eligible runnable threads are
dispatched according to the bounded weighted-fair virtual-time contract.

Useful sub-theorems:

- every live thread is in exactly one scheduler state: running, runnable,
  blocked, parked, exiting, or destroyed.
- a thread can run only if its Resource Domain is not frozen and has dispatch
  budget.
- `AWAIT` atomically transitions a thread from running to blocked or returns
  ready without losing an event.
- `WAKE`, event delivery, gate delivery, fd readiness, timer expiry, call
  completion, classifier queue routing, and domain resume transition eligible
  threads back to runnable at most once.
- consumed CPU advances virtual runtime/deadline accounting according to the
  fixed weight table.
- child Resource Domain CPU usage charges all ancestors before later dispatch
  decisions can ignore it.
- quota exhaustion makes descendant threads ineligible until the next permitted
  budget update.
- no eligible runnable thread with domain budget can remain undispatched beyond
  the implementation's stated fairness/approximation bound.
- scheduler bucket/window spill and refill preserve runnable identity, virtual
  time order within the stated approximation, and domain accounting.
- no software callback, plugin, raw interrupt handler, or policy script can
  mutate ready/blocked state or dispatch order outside the fixed hardware
  weighted-fair transition relation.

## 7.1 Default Operating Envelope

**Reset starts from a valid policy-bearing machine state:** before PID 1 or any
service thread can dispatch, hardware has created the root domain, PID 1 domain,
scheduler profile, security defaults, telemetry/fault routes, boot measurements,
and explicit initial capabilities.

Useful sub-theorems:

- no runnable thread exists outside a valid Resource Domain.
- no thread can dispatch before domain budget, virtual-time state, and
  accounting records are initialized.
- initial authority is represented by FDR capabilities, not ambient reset-time
  privilege.
- raw physical interrupts are routed to Event Router inputs before driver code
  can run.
- absent optional services imply absent authority, not fallback ambient access.
- PID 1 can refine policy through capabilities, but cannot retroactively create
  unmeasured boot authority.

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

## 10. Namespace Dispatch Containment

**Namespace dispatch cannot escape delegated namespace authority:** path strings
are names, not authority.

Useful sub-theorems:

- `OPEN_AT` and `NS_CTL` dispatch only through a held directory/root/namespace
  capability or the process/domain delegated cwd/root.
- `..`, symlinks, bind/delegated mounts, synthetic namespace nodes, and guest
  namespace upcalls are service semantics, but returned capabilities cannot
  escape the root capability.
- namespace services can return only object capabilities permitted by their
  delegated namespace root, credentials, object metadata, and Resource Domain
  policy.
- delegated namespace services cannot mint broader authority than the parent
  namespace capability allowed.
- POSIX global path syntax is a compatibility profile over explicit namespace
  root capabilities.

## 11. Typed Control Envelope Safety

**Control operations cannot become ambient `ioctl` authority:** metadata and
control surfaces are bounded, typed, versioned, and capability-checked before
dispatch.

Useful sub-theorems:

- `GET_META`, `SET_META`, `OBJECT_CTL`, `DOMAIN_CTL`, `NS_CTL`, socket options,
  storage barriers, and service-owned controls validate object class, profile
  class, profile id, op id, version, flags, lengths, required rights,
  generation, lineage epoch, capability argument shape, returned-capability
  shape, and domain policy before side effects.
- unknown well-formed operations return `ENOTSUP`; malformed envelopes return
  `EINVAL`; authority failures return `EPERM`/`EACCES`; stale lineage returns
  `EREVOKED` or the chosen stale-reference error.
- valid envelope shapes that exceed bounded implementation/profile limits return
  `EOVERFLOW`; user-buffer copy/pin faults return `EFAULT`; conflicting quiescent
  state returns `EBUSY`; pre-commit cancellation returns `ECANCELED`.
- capability arguments are passed only by FDR/capability mechanisms and cannot
  be forged by scalar fields or user memory.
- service-owned controls receive bounded copied records or pinned-buffer
  descriptors, not ambient raw pointers.
- returned authority is installed only through the verified capability-return
  path and cannot exceed delegated rights.
- every control operation has a single commit point and follows common
  cancellation/revocation semantics.
- returned-capability installation is a separate Capability Engine commit; a
  service reply cannot publish authority if capability-install validation fails.
- backend-defined payload bytes are data only and cannot encode ambient
  authority, unbounded pointers, hidden returned capabilities, or executable
  policy.
- architectural, personality/service, and vendor/device profiles all refine the
  same envelope validation relation; vendor profiles do not get a separate
  authority path.

## 11.1 Service Domain Boundary Soundness

**Services can implement policy but cannot escape the hardware authority
boundary:** a namespace, filesystem, loader, network, PCIe, telemetry, or
personality service can complete requests only through hardware-validated
continuations.

Useful sub-theorems:

- every service request is delivered through a bounded endpoint: call gate,
  queue, event queue, namespace dispatch, typed control envelope, page-fill
  request, or stream endpoint.
- request records contain caller domain/generation, target object generation,
  lineage epoch, rights, bounded input, explicit capability arguments, and an
  expected returned-capability shape.
- services never receive ambient physical addresses, raw interrupts, raw DMA,
  raw user pointers, or direct capability table write authority.
- service replies are data until hardware validates request/continuation id,
  service generation, output shape, and returned-capability proposals.
- service crash, restart, freeze, caller cancellation, signal interruption,
  domain teardown, and revocation before commit cannot publish partial
  authority.
- after commit, the operation either exposes exactly the committed effect or
  follows the object profile's documented roll-forward/drain/teardown rule.
- bounded queues, page-fill windows, stream buffers, and continuation slots
  prevent unbounded hidden service state; full capacity has a typed result:
  wait, `EAGAIN`, or `EOVERFLOW`.

## 12. VMA and Memory Safety

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
- page-state transitions follow the frozen state machine: `UNMAPPED`,
  `RESERVED`, `NONRESIDENT_OBJECT`, `FILL_PENDING`, `RESIDENT_CLEAN`,
  `RESIDENT_DIRTY`, `COW_SHARED`, `PINNED_DMA`, `REVOKING`, and `POISONED`.
- object-backed page replies are proposals until the VMA/Page Engine validates
  the returned page capability, range, memory type, executable provenance, and
  generation/lineage metadata.
- object-backed fills install a page only when VMA generation, object
  generation, lineage epoch, permissions, memory type, executable provenance,
  and domain policy still match the original fault.
- `MUNMAP`, `MPROTECT`, revocation, truncation notice, object generation change,
  domain teardown, or fatal signal before page-install commit cancels or faults
  the pending fill without publishing the page.
- hardware dirty bits and dirty-range enumeration do not imply hardware
  ownership of filesystem writeback policy.
- dirty writeback, truncation, `msync`, and file/service coherence policy are
  service-level refinements and cannot bypass VMA permissions or capability
  generations.
- VMA race resolution is deterministic and follows the architectural priority:
  poison, domain teardown, revocation, `MUNMAP`, `MPROTECT`/truncate/object
  generation change, DMA pin lifecycle, object-fill reply, then ordinary access.
- every multi-step VMA operation has one commit point; before commit it can abort
  without publishing authority, and after commit later conflicts require a new
  transition.
- DMA pins are granted only for resident, authorized, non-poisoned, non-guard,
  non-stale pages, and revocation blocks new pins before backing memory can be
  reused.

## 12.1 Memory Visibility Contract

**Normal cached memory is coherent and TSO-like:** ordinary shared-memory
programs, futexes, atomics, call gates, signals, DMA completion, and Unix
personality ports observe the architecture's explicit visibility rules.

Useful sub-theorems:

- stores to normal cached memory become visible to other cores in program order.
- a core observes its own loads and stores in program order, including
  store-buffer forwarding to later loads from the same address.
- aligned naturally sized loads/stores up to 64 bits are single architectural
  memory operations.
- `LOCK_CMPXCHG` is single-copy atomic and sequentially consistent in v1.
- futex `AWAIT` performs an acquire-style check before parking, and futex
  `WAKE` performs release-style ordering before making waiters runnable.
- `FENCE` orders normal memory, DMA/engine completions, and device memory
  according to VMA memory type.
- `device_ordered` mappings are strongly ordered and uncached.
- `write_combining` mappings cannot be used for ordered device signaling
  without a following `FENCE`.
- VMA/TLB/I-cache invalidation completes before affected threads resume or
  backing authority is reused.
- coherent DMA writes are visible before completion events are delivered; a
  non-coherent implementation cannot claim the coherent-DMA feature bit.

## 13. W^X and Executable Provenance

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

## 14. Heap Allocation Safety

**The hardware heap substrate preserves allocation safety:** `ALLOC`,
`ALLOC_EX`, `ALLOC_SIZE`, and `FREE` expose allocation intent and policy hints
over the LNP64 Default Heap Algorithm without exposing allocator representation,
and cannot create writable authority outside heap-owned VMAs or corrupt
allocator metadata.

The theorem has two granularities. Hardware-owned allocations receive
object-level safety and accounting. Software-owned arenas receive region-level
VMA/capability/domain safety; object-level correctness inside the arena is a
runtime theorem, not a Heap Engine theorem.

Useful sub-theorems:

- every returned allocation pointer lies inside a heap-owned, non-executable
  VMA authorized by the process and Resource Domain.
- `FREE` succeeds only for an exact live allocation pointer from the current
  compatible heap arena.
- interior, stale, double-free, foreign-arena, `MMAP`, `memory_object`, DMA,
  device, and executable-memory pointers are rejected.
- allocator metadata is not directly writable by user mappings.
- generation/quarantine policy prevents silent stale-pointer authority reuse
  before a slot is republished.
- per-thread allocation-window hits refine the same abstract allocation
  transition as central heap metadata updates.
- slab/run refill and drain preserve size-class membership, live/free counts,
  and per-domain heap accounting.
- `ALLOC_EX` policy hints cannot bypass Resource Domain memory, DMA, sharing,
  hardening, or executable-memory restrictions.
- implementation-specific size classes, allocation-window depth, freelists,
  slab/run layout, and quarantine state are not architectural authority and
  cannot be observed or mutated directly by user code.
- arena-style `ALLOC_EX`, `memory_object`, and `MMAP` regions do not give the
  Heap Engine authority over software subobjects inside the region.
- releasing a software-owned arena revokes or invalidates the outer region
  according to VMA/capability rules, regardless of the runtime's inner object
  graph.

## 15. DMA Isolation

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

## 16. Raw Interrupt Non-Exposure

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

## 17. Network Capability Containment

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
- datagram and stream endpoints are capability-scoped object profiles; their
  implementation may be software TCP, UDP-like datagrams, local IPC, QUIC,
  paravirtual transport, or a future accelerator without changing authority
  checks.
- transport assists such as checksum, timestamp, flow hash, queue steering,
  timer/counter use, and zero-copy handoff cannot create protocol authority or
  bypass endpoint capability scope.
- endpoint substitution preserves authority: software TCP, local IPC, QUIC,
  paravirtual transport, and future accelerators may all implement a
  `stream_endpoint`, but none can broaden rights, bypass namespace scope, or
  change the endpoint readiness/capability contract observed by applications.
- packet queues preserve packet-record authority, datagram endpoints preserve
  message-boundary authority, stream endpoints preserve ordered-byte authority,
  and listeners mint accepted endpoint capabilities only through the Capability
  Engine's returned-capability path.

## 18. Classifier Safety

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

## 19. Event, Gate, and Fault Delivery Safety

**Events, gate activations, and faults are delivered to the right authority
scope:** synchronous faults, asynchronous events, explicit calls, forced
deliveries, and the POSIX signal profile are well-formed and cannot forge
privilege or capabilities.

Useful sub-theorems:

- divide-by-zero, illegal opcode, permission fault, guard fault, and bad device
  mapping produce the specified gate-delivery/upcall/fault record.
- frozen v1 POSIX signal delivery refines native gate delivery and respects
  process disposition, thread mask, pending state, and domain policy.
- synchronous faults are thread-directed to the faulting thread.
- process-directed signals are delivered only to an eligible unmasked thread
  chosen by the fixed implementation-profile rule, or remain process-pending.
- signal/delivery frames are non-executable and cannot forge privilege or capability
  state.
- `GATE_RETURN` restores only the hardware-saved continuation for that
  activation.
- `GATE_RETURN` validates saved context token/generation and cannot restore
  from an arbitrary user-writable frame.
- interruptible operations return `EINTR` or a typed interrupted status before
  handler entry; post-commit operations follow their documented roll-forward or
  teardown policy.
- unsupported POSIX/Linux/BSD signal quirks can be emulated by personality code
  but cannot bypass native event, domain, or capability checks.
- supervisor opcode upcalls cannot bypass normal capability/domain checks.
- event/fault records are scoped to the owning domain or configured supervisor
  FDR and do not leak unauthorized payloads.

## 20. Gate/Continuation Safety

**Gate safety:** `GATE_CALL`, `GATE_DELIVER`, and `GATE_RETURN` transfer
control, arguments, accounting, continuations, and optional capabilities only as
authorized by the gate FDR or delivery profile.

Useful sub-theorems:

- synchronous calls park exactly one caller continuation.
- forced deliveries save at most one bounded continuation per activation unless
  a profile explicitly permits bounded nesting.
- asynchronous calls produce at most one completion per accepted operation id.
- handoff calls transfer cancellation/accounting ownership according to gate
  policy.
- reentrant call depth is bounded.
- capability-marked arguments are rejected unless the gate permits capability
  passing.
- cross-domain calls cannot bypass child/parent domain budget and authority
  checks.

## 21. Checkpoint Hook Safety

**Checkpoint metadata cannot duplicate or revive authority:** freeze,
query-state, resume, dirty tracking, and future explicit reattachment preserve
containment and generation safety.

Useful sub-theorems:

- `freeze` reaches a quiescent boundary before exportable state is observed: no
  running threads, no new DMA descriptors, no in-progress metadata commits, and
  no unaccounted call-gate continuations.
- `query-state` is observation only and cannot mutate authority.
- `resume` restarts a quiesced domain without generation churn.
- future software restore creates fresh domain ids/generation bases and
  requires explicit capability reattachment; hardware does not parse checkpoint
  image formats.
- serialized stale capabilities cannot revive revoked or destroyed authority.
- dirty-memory tracking hooks cannot grant read/write access outside the
  checkpointed domain.

## 22. Commit/Abort Atomicity

**No partial publication:** multi-step hardware operations expose either the old
state or the committed new state, never an inconsistent middle state.

Useful sub-theorems:

- `OPEN_AT`, `NS_CTL`, `SET_META`, `MMAP`, `MUNMAP`, `CLONE`, `EXEC`,
  `DOMAIN_CTL`, `OBJECT_CTL`, `CAP_REVOKE`, `GATE_CALL`, classifier table
  updates, and network endpoint creation each have a single architectural commit
  point.
- cancellation before commit rolls back all reservations.
- cancellation after commit completes, rolls forward, or reports a defined
  failure without corrupting authority-bearing metadata.
- result registers and compatibility `ERRNO` are updated only at completion.

## 23. Clone/Fork Profile Safety

**`CLONE` creates only explicitly described process/thread state:** native clone
profiles and POSIX `fork()` compatibility cannot duplicate hidden authority or
in-flight ownership.

Useful sub-theorems:

- `profile=thread` creates a new TID in the same PID with only the explicitly
  shared address space, FDR table, credentials, signal dispositions, and
  scheduler state.
- `profile=process` creates a new PID with only the requested share/copy/new
  state permitted by Resource Domain policy.
- `profile=posix_fork` creates one child thread, COW VMAs and heap metadata,
  inherited/narrowed FDRs according to descriptor flags, copied credentials and
  signal dispositions, copied caller signal mask, and cleared child pending
  signals.
- no clone profile copies in-flight operation ownership, pending DMA
  descriptors, partially committed metadata operations, runtime locks, or
  hidden personality state.
- parent and child return-register conventions are published at one commit
  point, and failure before commit leaves the parent unchanged.

## 24. Storage and Filesystem-Service Durability Contract

**Durable storage ordering:** after a successful synchronous storage barrier,
the relevant block/storage object can recover to a state that includes all
committed writes before the barrier or a defined service replay/fsck result.

Useful sub-theorems:

- filesystem-service operations such as atomic rename are power-fail atomic
  only under the selected service log/journal/COW protocol.
- service commit records are written before commit publication.
- storage barriers order prior data writes, metadata writes, and backend flush
  completion.
- hardware proves ordering and completion for block/storage objects; filesystem
  semantics are proved for the service implementation that owns them.
- live-system atomicity and power-fail durability have separate proof
  obligations.

## 25. Exec-Plan Commit Soundness

**`EXEC` commits only validated architectural state:** hardware process
replacement consumes a bounded exec-plan descriptor, not an executable file
format.

Useful sub-theorems:

- every VMA installed by `EXEC` is authorized by an executable image, memory
  object, anonymous-memory, or loader/JIT capability named in the exec plan.
- W^X, NX, guard-page, ASLR, Resource Domain, and executable-source policy are
  checked before the commit point.
- sibling threads are stopped or invalidated before the old address space is
  destroyed.
- startup metadata is treated as opaque runtime/personality data; hardware does
  not derive authority from auxv, environment strings, interpreter paths,
  dynamic-linker records, or relocation tables.
- failure before commit preserves the old process image; success after commit
  exposes exactly one surviving thread in the new image.

## 26. Boot Measurement and Attestation Integrity

**Measured boot and quote consistency:** the boot measurement log and quote FDR
accurately reflect the boot manifest, selected images, reset cause, FPGA/ROM
identity, domain launch measurements, delegated capability roots, and boot
policy observed by authorized domains.

Useful sub-theorems:

- measurement records are append-only during boot.
- PID 1 cannot observe a booted image without the corresponding measurement
  record being present.
- a tenant or confidential domain cannot be marked runnable before its launch
  measurement is recorded.
- quote records are derived from measurement records and cannot include
  unmeasured capability roots or omit measured policy bits.
- boot policy failure either records a structured fault and enters permitted
  development mode or enters hardware panic.
- boot-control and quote FDR reads cannot alter measurement records or mint
  authority.

## 26.1 Assurance Profile and Policy Enforcement Soundness

**Assurance profiles are enforceable machine state:** development, field,
high-assurance, and formal-assurance profiles are reflected in boot records,
domain policy, quote records, and hardware checks.

Useful sub-theorems:

- `ASSURANCE_DEV` quotes and audit records cannot be mistaken for production
  quotes.
- a domain requiring `ASSURANCE_FIELD`, `ASSURANCE_HIGH`, or
  `ASSURANCE_FORMAL` cannot become runnable unless the active machine/domain
  policy satisfies that minimum profile.
- policy decisions from PID 1, domain managers, personalities, or services have
  no effect until the hardware Policy Enforcement Point validates capability,
  domain, generation, lineage, label, measurement, and profile constraints.
- `ASSURANCE_FORMAL` quote records bind proof artifact hashes, theorem coverage
  metadata, RTL/IP provenance hashes, and toolchain/build ids to the measured
  image.

## 26.2 Owner Sovereignty and Open Assurance

**Attestation is evidence, not ambient vendor control:** measured boot and quote
records describe artifacts and policy, but cannot create a vendor-only execution
gate unless the machine owner selected that policy.

Useful sub-theorems:

- boot policy can name owner, organization, vendor, development, or unsigned
  development trust roots, and quote records identify which root policy was
  active.
- no architectural state transition requires a vendor-exclusive key, remote
  authorization service, hidden management domain, or ambient vendor capability.
- owner-held debug-control FDRs in open-owner profiles can unlock debug only
  through measured/audited transitions and still obey domain/range/label
  authority.
- quote records can bind public RTL/source hashes, reproducible bitstream
  hashes, toolchain manifests, proof artifact hashes, and service image hashes
  without requiring those artifacts to be secret or vendor-controlled.
- telemetry, audit, trace, debug, and DMA paths cannot bypass FDR capability
  checks to create hidden owner/vendor access.
- replacing loader, filesystem, network, personality, telemetry, domain-manager,
  or declassification services cannot broaden authority unless the replacement
  receives explicit capabilities and passes generation/lineage/label/domain
  checks.

## 27. RAS Fault Containment

**Detected corruption does not silently become authority:** ECC/parity,
watchdog, local engine reset, and metadata faults are either corrected,
poisoned, reported, locally degraded, or panic the machine.

Useful sub-theorems:

- correctable metadata faults preserve object identity and generation.
- uncorrectable metadata faults poison affected objects before reuse.
- local engine reset does not publish partial state.
- a local engine fault cannot corrupt unrelated domains or require full-chip
  reset when the engine has a defined recovery/degraded path.
- degraded engines reject new commands until supervisor/PID 1 policy clears
  them.
- trace/fault records cannot grant authority.

## 28. Telemetry, Trace, and Counter Non-Interference

**Observability does not change authority:** reading counters, trace rings, or
fault records cannot create, broaden, revive, or revoke capabilities.

Useful sub-theorems:

- trace records are data, not FDR entries.
- destructive trace reads only advance trace-consumer cursors.
- counter overflow or wrap cannot affect scheduler, domain, VMA, or capability
  state.
- trace/counter/telemetry access is scoped by Resource Domain and control-FDR
  authority.
- aggregate telemetry cannot be refined into unauthorized per-tenant memory,
  packet, or secret contents.
- classifier and network counters cannot leak unauthorized packet payloads.

## 28.1 Tamper-Evident Audit Integrity

**Audit streams are append-only evidence, not authority:** audit records cannot
create or broaden capabilities, and missing or reordered records are detectable
within the stated overflow model.

Useful sub-theorems:

- audit records have monotonically increasing sequence numbers within a stream.
- each committed record includes the previous audit-root hash or a documented
  reset/gap marker.
- audit overflow advances dropped-count or gap metadata before later roots are
  accepted.
- narrowed audit FDRs expose only authorized domains, labels, event classes, and
  redacted payload fields.
- audit roots included in quote records correspond to the hardware-owned audit
  stream state.
- audit read, destructive read, snapshot read, and quote operations cannot alter
  Resource Domain, scheduler, VMA, DMA, or FDR authority.

## 29. POSIX/Profile Compatibility Refinement

**Compatibility APIs are refinements of native primitives:** POSIX, libc, Linux
syscall compatibility, and NetBSD rump-style interfaces cannot bypass native
capability/event/domain authority.

Useful sub-theorems:

- POSIX file descriptors are FDR capability handles plus compatibility metadata.
- `fork` is the constrained `CLONE profile=posix_fork` and cannot duplicate
  authority beyond FDR/domain inheritance rules or copy in-flight ownership.
- POSIX signals are an event-delivery profile and cannot bypass capability or
  domain checks.
- POSIX UID/GID and mode bits participate in compatibility decisions through a
  credential profile, but cannot replace required capabilities.
- `errno` is a compatibility view of explicit result/error status.
- socket APIs refine endpoint/listener/network namespace capabilities.
- ioctl-like controls refine typed metadata/control records or fail; they do
  not form a separate authority path.

## 30. Paravirtual Personality Containment

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
- personality control surfaces are limited to fixed FDR mechanisms: lifecycle
  events, VMA events, capability transfer, namespace dispatch, block/storage
  objects, signal/fault records, event queues, network endpoints, PCIe
  BAR/DMA/IRQ-event capabilities, and domain-control upcalls.
- personality domains cannot mutate raw page tables, receive raw interrupts,
  bypass the hardware scheduler, perform raw DMA, or mint capabilities outside
  the Capability Engine.

## 31. Tenant-Strict Confidentiality and No Unauthorized Observation

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
- `DOMAIN_PROFILE_TENANT_STRICT` forbids parent memory inspection without an
  explicit shared-memory or inspection capability.
- confidential-domain sealed secrets are released only when measurement and
  policy records match.
- confidential-domain memory cannot be exported through ordinary query-state,
  telemetry, trace, DMA, packet, or fault records.

## 31.1 Controlled Debug and Forensics Non-Bypass

**Debug is not an ambient superuser path:** every debug or forensic observation
is mediated by explicit capability, domain, label, generation, and assurance
policy.

Useful sub-theorems:

- no debug operation succeeds without a debug-control FDR carrying the specific
  right for halt/freeze, single-step, breakpoint, register read, memory read,
  memory write, trace read, crash snapshot, dump export, or engine diagnostic
  access.
- production profiles can permanently disable invasive debug or require
  destructive domain freeze before forensic export.
- tenant-strict, confidential, and MLS domains cannot be inspected by parent
  domains without an explicit inspection or shared-memory capability.
- debug unlock and forensic export produce audit records and are reflected in
  quoteable state where the assurance profile requires it.
- crash dumps and snapshots are redacted according to tenant/confidential/MLS
  policy before leaving the domain.

## 31.2 Cross-Domain MLS Noninterference

**Labels compose with capabilities:** a held capability is necessary but not
sufficient when MLS policy is active; the label relation must also permit the
operation.

Useful sub-theorems:

- cross-domain send, map, DMA, telemetry, debug, packet, page-fill,
  returned-capability, and service-reply operations fail closed when label
  generation is stale or the label relation denies the flow.
- declassification/release/downgrade requires an explicit declassification
  service capability and produces an audit record.
- raw interrupts, raw DMA, parent inspection, trace/fault records, service
  replies, and debug paths cannot bypass label checks.
- unlabeled objects cannot enter an MLS domain except through a profile-defined
  default-label rule that is itself quoteable and auditable.

## 31.3 Mission Assurance Continuity

**Mission recovery cannot broaden authority:** mission-domain degraded,
recovery, failover, freeze, quarantine, and fail-closed transitions preserve the
Resource Domain and capability containment model.

Useful sub-theorems:

- a mission domain cannot enter `normal`, `degraded`, or `recovering` unless its
  minimum assurance, audit, attestation, dependency graph, and label constraints
  are satisfied.
- mission dependencies are exactly the delegated FDR capabilities named by the
  mission profile; undeclared services or devices cannot become implicit
  dependencies.
- service restart or failover creates new service generations, and stale
  continuations or returned-capability proposals from old generations cannot
  complete.
- `fail_closed`, `freeze`, and `quarantine` states reject new dispatch or
  authority use except for explicitly permitted recovery/control FDRs.
- `fail_degraded` and `fail_over` states cannot add rights beyond already
  delegated fallback capabilities.
- mission-state transitions emit audit/fault events and update quoteable
  mission evidence before the next successful mission dispatch.
- mission evidence quote fields match the active mission profile hash,
  dependency graph hash, current state, degraded reason, audit root, proof
  artifact hash, domain launch measurement, and delegated capability-root
  summary.

## 32. Refinement Targets

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
