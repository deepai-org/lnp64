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
- core-tile topology, tile-local running lanes, per-tile reset/fault state,
  cross-tile scheduler assignment, and coherence-domain membership.
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
  `memory_object`, `call_gate`, `dma_buffer`, and `dma_completion`; optional
  acceleration profiles such as `classifier_table` and `servicelet_program`
  refine these same object/capability invariants.
- wait/ready/runqueue state, scheduler eligibility, no-lost-wakeup transitions,
  timeout events, gate delivery, continuation return, and fault delivery.
- service-boundary request/reply records, continuation ids, copied/pinned
  buffers, returned-capability proposals, commit points, and canonical error
  outcomes.
- fault, poison, overflow, watchdog, local-reset, trace/audit/telemetry records
  as data, never authority.

The first safety model intentionally excludes:

- exact cycle timing, cache replacement policy, branch pipeline timing,
  physical DDR controller timing, Ethernet/PCIe electrical behavior, and
  FPGA-specific CDC.
- filesystem formats, executable formats, dynamic-linker behavior, TCP/IP
  protocol policy, PCIe device quirks, orchestration policy, and Linux/BSD ABI
  compatibility details.
- performance optimality. Proofs should establish safety, containment,
  determinism at architectural commit points, and specified liveness/fairness
  bounds where the architecture promises them.

RTL assertions and bounded model checking should later prove that each hard
block refines the abstract transition relation for its owned state machine.

A separate realtime refinement model should prove that implementation timing
refines the published `ENV_GET` WCET profile. It does not need to prove
performance optimality; it must prove that each latency class retires, parks, or
submits within its bound, that Class D work is explicit, and that shared-fabric
arbitration cannot create unbounded head-of-line blocking for admitted work.

## 0.1 Proof and Fault Model Assumptions

The theorem roadmap is only credible if its assumptions are explicit. The base
model assumes:

- clocks, reset distribution, power integrity, physical tamper resistance,
  analog behavior, and catastrophic silicon destruction are outside the normal
  architectural proof model.
- a stronger assurance profile may add measured boot, physical tamper evidence,
  redundant fabrics, hardened memories, or formal RTL/proof artifact
  requirements, but those are profile refinements rather than hidden base
  assumptions.
- bounded hardware faults include ECC/parity faults, malformed records, stale
  generations, watchdog timeouts, service crashes/restarts, queue overflow,
  revoked objects, poisoned metadata, local engine reset, and documented
  degraded states.
- Byzantine hardware behavior, malicious FPGA bitstream replacement after boot,
  arbitrary analog fault injection, and compromised proof tools are outside the
  base theorem unless bound by measured/quoted assurance profile evidence.
- liveness and global-progress claims require at least one schedulable core or
  tile, a functioning scheduler fabric path, a functioning event/fault route,
  and admitted Resource Domain budget for the work being considered.
- realtime claims are relative to the immutable `ENV_GET` implementation
  constants for the current boot epoch; a profile that reports weak or absent
  bounds cannot claim stronger realtime behavior.
- software services may be malicious, buggy, crashed, or slow. Theorems about
  hardware containment do not assume service correctness except where a service
  implementation is separately verified.

Proofs should separate normal correctness from fault containment. Normal
correctness proves that valid inputs and valid owned metadata preserve the
architectural invariants. Fault containment proves that malformed inputs,
poisoned metadata, watchdog timeouts, impossible state encodings, external-IP
errors, and bounded hardware faults transition only to canonical error,
poisoned, abort, degraded, or machine-fatal states that are
authority-decreasing and fail closed.

Not every module should have a large fault/degraded lifecycle. Pure datapath and
decode blocks should be total over their valid encodings or produce canonical
fault results. Small queues should expose empty/full/poison behavior. Owner
engines with commit points need explicit prepare/commit/complete/abort phases.
External-IP adapters need link/training/error/degraded states because their
environment is outside the proof boundary. The proof model should make invalid
states unrepresentable where practical, and otherwise prove that illegal
encodings cannot publish authority or partial commits.

## 0.2 Security Theorem Spine

The high-level security claim is:

> For every reachable machine state, no subject can gain, use, observe, modify,
> schedule, signal, debug, DMA, or cause service action on any object outside
> the authority explicitly delegated to it, except through documented
> declassification, shared-memory, IPC, or service-boundary paths.

The desired proof list should therefore be organized around these top-level
security theorems. The detailed sections below are supporting lemmas and
refinement targets for this spine.

1. **Authority soundness:** all authority originates from boot roots, mint
   roots, or valid delegation. No register value, memory write, trace record,
   event, fault, packet, service reply, stale cache entry, or compatibility
   personality can create or broaden authority.
2. **Confinement and noninterference:** Resource Domains cannot read, write,
   execute, schedule, signal, debug, trace, receive events for, or otherwise
   observe another domain except through explicit capabilities, policy, or
   documented declassification paths.
3. **Memory confinement:** loads, stores, instruction fetches, atomics, page
   fills, DMA, service copies, snapshots, restore hooks, and debug reads all
   respect live capabilities, VMA permissions, Resource Domain policy,
   generations, lineage, revocation, guard pages, W^X/NX, and memory type.
4. **Revocation and freshness:** revoked, stale, poisoned, restored, reused, or
   generation-mismatched state cannot authorize new work or accidentally alias a
   new object.
5. **Mediation completeness:** every path to authority-bearing state goes
   through exactly one checked owner engine or its proven shard. There are no
   alternate write paths into FDR tables, VMA tables, scheduler state, domain
   policy, IOMMU tables, object state, debug authority, trace authority, or
   service-return capability install.
6. **Fail-closed fault containment:** malformed inputs, hostile service
   payloads, bad packets, queue overflow, parity/ECC faults, watchdog timeout,
   impossible state encodings, service crashes, and external-IP errors cannot
   mint authority, skip generation checks, publish partial commits, split
   scheduler state, or bypass memory/domain checks.
7. **Trusted boundary soundness:** boot, attestation, loader/runtime services,
   paravirtual personalities, PCIe bus master, debug/forensics mode, and
   external IP are either inside the proof or represented by explicit
   assume-guarantee contracts that cannot silently grant stronger authority
   than the theorem states.
8. **Compositional security:** local capability, domain, scheduler, VMA, DMA,
   object, gate, service, RAS, and fabric proofs compose at `lnp64_top`; a
   global transition preserves authority, confinement, memory safety, freshness,
   fail-closed behavior, and the trusted-boundary assumptions.

The hardest theorem is mediation completeness. If any authority-relevant state
can be mutated outside its checked owner path, local proofs can all be true
while the whole chip is insecure. The first deep proofs should therefore show
both local transition invariants and absence of bypass paths for the owner
state they protect.

Three enclave-strengthening theorem families sit on top of this spine:

- **No ambient authority:** supervisors, services, drivers, PCIe bus masters,
  loaders, personalities, debug tooling, and device frontends have no hidden
  ring-0 style authority. They can act only through explicit capabilities,
  Resource Domain policy, and checked owner-engine transitions.
- **Scoped observation:** official observation surfaces such as debug,
  forensics, trace, telemetry, audit, event metadata, scheduler pressure,
  queue occupancy, packet marks, and snapshot hooks expose only data authorized
  by Resource Domain policy or explicitly documented public/declassified state.
- **Attested fresh boundary:** a Resource Domain quote describes a fresh,
  measured, policy-scoped boundary; launch, restore, migration, gate calls, and
  delegated services cannot revive stale authority or amplify capabilities
  across that boundary.

These families are the basis for the high-assurance claim that a Resource
Domain can be treated as the enclave boundary when the corresponding assurance
profile is enabled. Physical probing, memory-encryption strength, and
microarchitectural side-channel elimination remain separate profile claims, not
hidden assumptions of the base proof.

## 0.3 Proof Priority Order

The architecture is intentionally rich, so proof work should start with the
smallest authority-bearing core and expand outward:

1. Capability/FDR non-forgeability, generation safety, and no authority
   amplification.
2. Resource Domain containment, monotonic delegation, accounting rollup, and
   lifecycle generations.
3. Scheduler, waitable, gate-continuation, and no-lost-wakeup invariants.
4. Multicore topology, unique tile assignment, cross-tile wake, and coherence
   shell safety.
5. Realtime retire/park/submit and `ENV_GET` WCET/scheduler discovery
   soundness.
6. VMA/page-state, W^X/NX/guard, memory visibility, and DMA isolation.
7. Service-boundary request/reply, returned-capability install, and
   commit/abort atomicity.
8. Servicelet verifier/action safety and bounded classifier/queue steering.
9. Global progress under bounded faults and watchdog/local-reset containment.
10. Adversarial input containment for hostile code, packets, service payloads,
   and timing races.
11. Tenant-strict confidentiality, MLS, audit, debug/forensics, attestation,
    mission assurance, and personality containment.

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
- every mutable authority-bearing record has exactly one architectural owner at
  a time. If the owner is physically banked or sharded, the shard map assigns
  each record to exactly one owning shard, and shard migration preserves
  generation/epoch safety.
- no supervisor, personality, service, driver, debug path, or device frontend
  has ambient authority outside owner-engine transitions and explicit
  capabilities.

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
- flattened effective-domain records consumed by scheduler, heap, VMA, FDR, DMA,
  event, and gate engines are monotonic refinements of the Resource Domain tree
  state at their recorded generation.
- hot-path enforcement uses resident effective records and generation checks; it
  does not require an unbounded ancestor walk, policy recomputation, or software
  callback.
- Class D domain-engine refill/recompute of effective records preserves
  monotonic limits, rolls back failed pre-commit reservations, and cannot publish
  broader authority than the parent delegated.

## 7. Scheduler Safety

**Scheduler well-formedness and bounded fairness:** hardware scheduling never
loses or duplicates a thread context, and eligible runnable threads are
dispatched according to the Fixed Weighted-Fair Virtual-Deadline Active-Window
Scheduler contract.

Useful sub-theorems:

- every live thread is in exactly one scheduler state: running, runnable,
  blocked, parked, exiting, or destroyed.
- every live thread appears in exactly one scheduler location: running lane,
  ready queue/window/bucket, one wait queue, gate-delivery continuation, zombie
  table, or destroyed/free state.
- a thread can run only if its Resource Domain and every ancestor domain are not
  frozen, have dispatch budget, and permit the selected core/tile.
- hard affinity masks are eligibility constraints: no scheduler transition can
  dispatch a TID on a tile outside the intersection of thread and ancestor
  domain core/tile masks.
- sticky placement preserves the current/preferred tile unless a documented
  migration reason applies: explicit affinity/domain update, wake placement,
  quota/reservation/latency pressure, bounded load balancing, work stealing,
  tile fault/reset/degraded state, or administrative disable.
- `AWAIT` atomically transitions a thread from running to blocked or returns
  ready without losing an event.
- `WAKE`, event delivery, gate delivery, fd readiness, timer expiry, call
  completion, classifier queue routing, and domain resume transition eligible
  threads back to runnable at most once, and only when source id, operation id,
  generation, and wait predicate match.
- consumed CPU advances virtual runtime/deadline accounting according to the
  fixed weight table.
- scheduler dispatch consumes a resident effective scheduling record whose
  allowed tile mask, quota/reservation class, latency cap, frozen/quiescing bits,
  and accounting generation refine the Resource Domain tree.
- child Resource Domain CPU usage charges all ancestors before later dispatch
  decisions can ignore it.
- quota exhaustion makes descendant threads ineligible until the next permitted
  budget update.
- no eligible runnable thread with domain budget can remain undispatched beyond
  the implementation's stated fairness/approximation bound.
- scheduler bucket/window spill and refill preserve runnable identity, virtual
  time order within the stated approximation, and domain accounting.
- hardware thread interleaving preserves single-thread semantics: each tile
  issues at most one selected TID per issue slot in v1, blocked/pending TIDs are
  not issue-eligible, and switching issue between TIDs cannot merge registers,
  continuations, `ERRNO`, delivery masks, FDR authority, or accounting state.
- Class B/C pending work for one TID cannot freeze unrelated eligible TIDs
  except through published bounded arbitration on the shared engine or port.
- wakeup placement grants at most the published bounded latency adjustment and
  cannot erase accumulated virtual runtime.
- preemption occurs at published accounting boundaries or forced park points
  within the maximum preemption latency, except inside non-interruptible
  Class A-C atomic transitions.
- frozen/quiescing domain transitions remove descendants from dispatch
  eligibility without duplicating or losing thread contexts.
- no software callback, plugin, raw interrupt handler, or policy script can
  mutate ready/blocked state or dispatch order outside the fixed hardware
  weighted-fair transition relation.

## 7.1 Realtime Boundedness

**Instruction and fabric boundedness:** every architected instruction either
retires, parks, or submits an explicit transaction within its published latency
class, and admitted Resource Domains cannot be blocked indefinitely by
best-effort traffic on shared fabrics.

Useful sub-theorems:

- Class A/B/C instructions do not perform unbounded DDR walks, path walks,
  service execution, page fills, queue scans, subtree traversals, or device
  waits before retire/park.
- Class D instructions publish an operation id, waitable, completion token, or
  blocking park state before long-latency work begins.
- local-cache misses for FDRs, VMAs, heap windows, gates, waitables, scheduler
  slots, domain records, and servicelet attachments have bounded outcomes:
  canonical error, park, or explicit refill/owner-engine transaction.
- metadata engines, event routers, DMA paths, memory-controller ports,
  servicelet lanes, and queue banks use bounded arbitration for admitted
  domains.
- Class D async work preserves the submitter's Resource Domain id/generation,
  TID/generation, reservation/deadline metadata, operation id, cancellation
  epoch, and completion target across queues, fabrics, memory-controller
  requests, DMA descriptors, and completion records.
- the DDR/HBM controller's admitted-realtime arbitration bound is conservative
  with respect to its published bank/pseudochannel reservation, refresh,
  calibration, ECC retry, and vendor-IP assumptions.
- deadline comparison, timeout expiry, watchdog windows, reservation periods,
  and Class D async deadlines are evaluated against the synchronized global
  timebase within the published skew bound.
- cancellation epochs reclaim reserved capacity for stale, revoked, expired, or
  dead-thread Class D operations within the published bound, without requiring
  unbounded queue scans.
- banked or sharded owner engines preserve the same transition relation as the
  abstract owner engine: banking changes placement and arbitration, not the
  authority, generation, or single-writer rules.
- pressure from best-effort domains can delay or fail best-effort work with
  visible pressure/status events, but cannot violate published bounds for
  admitted reservations.
- `ENV_GET` WCET and scheduler discovery records are sound and conservative:
  reported latency, arbitration, active-window, spill/refill, servicelet-lane,
  and reservation constants are immutable for the boot epoch and are no weaker
  than the implementation actually provides.
- absent realtime, scheduler, servicelet, classifier, DMA, or reservation
  features fail closed with defined status instead of falling back to hidden
  unbounded behavior.

## 7.2 Default Operating Envelope

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

- **Thread store confinement:** for every reachable machine state, if a thread
  commits a store to virtual address `A`, then that thread's current
  PID/Resource Domain owns or has been delegated a live writable VMA covering
  `A`; the VMA generation, object/capability generation, lineage epoch,
  permissions, guard status, memory type, and domain policy all match at the
  store commit point.
- thread stores cannot bypass the VMA/TLB permission check, cannot commit
  through stale translations, and cannot write through another PID/domain's VMA
  unless an explicit shared-memory capability authorized that mapping.
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

## 12.2 Multicore Topology and Cross-Tile Safety

**The multicore machine preserves single-thread identity and cross-tile
visibility:** adding 2-4 coherent core tiles does not duplicate thread
execution, lose wakeups, bypass domain eligibility, or publish stale memory,
TLB, event, or fault state across tiles.

Useful sub-theorems:

- `ENV_GET` topology records are truthful for the boot epoch: enabled tile ids,
  tile count, active-window shape, coherence-domain membership, and disabled
  tile state match the initialized hardware state.
- every running TID is assigned to exactly one tile-local running lane, and no
  runnable/running TID can execute on two tiles in the same architectural step.
- tile migration is an atomic scheduler transition: ownership moves from one
  tile-local lane or ready structure to another without duplicating registers,
  continuations, `ERRNO`, delivery masks, FDR authority, accounting, or pending
  engine ownership.
- migration generation prevents stale tile-local queues, wakeups,
  completions, or balancing records from reviving a thread on a no-longer-owned
  tile.
- a tile-local fault, watchdog timeout, degraded state, or reset cannot corrupt
  another tile's running-lane state, ready queues, capability state, domain
  accounting, or VMA/TLB authority.
- cross-tile wake, event delivery, gate return, timer expiry, futex wake,
  classifier queue routing, and completion delivery make a blocked thread
  runnable at most once and on an eligible tile only.
- cross-tile park/wake races refine the same no-lost-wakeup theorem as
  single-tile waits: either the waiter observes readiness immediately, or a
  matching future wake/event remains attached to the wait predicate.
- coherence-shell invalidate, acknowledge, writeback, and ownership-transfer
  records are paired: a store, `MPROTECT`, `MUNMAP`, `EXEC`, `ISYNC`, DMA
  visibility point, or authority reuse cannot commit before required tile
  acknowledgements have arrived or a documented fault/degraded path is taken.
- L1/TLB/I-cache invalidation is scoped by generation and address range; an
  unrelated tile cannot be forced to drop authority it does not hold, and a
  stale tile cannot continue using authority after invalidation completion.
- locked atomics and futex expected-value checks are single-copy atomic across
  all enabled tiles in the coherence domain.
- tile-id tags on retire, event, fault, telemetry, and trace records are
  faithful to the tile that produced the architectural transition.
- a disabled or absent tile cannot be selected by the scheduler, receive new
  work, acknowledge coherence messages, or appear as live in topology records.
- four-tile stress configuration refines the same abstract multicore transition
  relation as the default two-tile configuration; increasing tile count does
  not add new authority or weaken scheduler/coherence invariants.

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

The base heap theorem is about allocation ownership, metadata integrity,
exact-pointer free, reuse discipline, and authority. It does not prove that every
ordinary `LD`/`ST` is checked against allocation-object bounds for untagged
C-style pointers. Rust-style intra-program memory safety, per-access
use-after-free prevention, and sub-object bounds enforcement are not v1 hardware
heap proof goals. Those properties belong to languages, runtimes, sanitizers, or
software-owned arena policies layered above the hardware heap. Pointer tagging,
fat pointers, capability-pointer C ABIs, or compiler-inserted bounds checks are
future profile topics, not default heap proof obligations, unless they can be
used by ordinary C source and libc with explicit ABI and WCET contracts.

Heap proofs are profile-scoped:

- `base_heap` proves metadata integrity, exact-pointer free, bounded hot
  allocation/free, domain accounting, NX heap backing, and fail-closed invalid
  free behavior with VMA/page-granularity load/store enforcement.
- `hardened_heap` additionally proves bounded quarantine, poison/zero behavior,
  guarded allocation behavior where selected, generation checks before slot
  reuse, and heap-corruption fault delivery.

All heap profiles refine the same abstract allocation transition relation.
Profile selection may strengthen allocator hardening and alter timing constants,
but it cannot weaken authority, accounting, exact-pointer free, or
metadata-integrity invariants.

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
- heap hot paths consume a resident effective heap-domain record whose budget,
  profile, hardening policy, large-object eligibility, locality policy, and
  accounting generation refine the Resource Domain tree.
- `ALLOC`/`FREE` hot paths do not walk an unbounded Resource Domain ancestor
  chain; cold or stale accounting state parks, fails, or submits a Class D
  heap/domain refill.
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
- network servicelet actions on `net_interface`, `packet_queue`, endpoint, or
  listener objects can only mark, count, drop, steer to authorized queues,
  select authorized gates, redact/sample telemetry, or request software; they
  cannot create namespace authority, mint endpoints, bypass listener policy, or
  implement a second socket authority path.
- endpoint substitution preserves authority: software TCP, local IPC, QUIC,
  paravirtual transport, and future accelerators may all implement a
  `stream_endpoint`, but none can broaden rights, bypass namespace scope, or
  change the endpoint readiness/capability contract observed by applications.
- packet queues preserve packet-record authority, datagram endpoints preserve
  message-boundary authority, stream endpoints preserve ordered-byte authority,
  and listeners mint accepted endpoint capabilities only through the Capability
  Engine's returned-capability path.

## 18. Classifier and Servicelet Safety

**Bounded record classification and servicelets cannot broaden authority or
create unbounded execution:** classifier tables and verified servicelet programs
classify and steer records only within delegated source and destination
capability scopes.

Useful sub-theorems:

- classifier tables are capabilities with owner domain, generation, source
  scope, destination queue set, and bounded rule limits.
- servicelet programs are capabilities with owner domain, generation, verifier
  certificate, allowed attachment class, maximum instruction/cycle budget,
  allowed record fields, and allowed action set.
- a dedicated servicelet execution lane refines the same verified subset
  semantics; its implementation may be a tiny programmable engine, but it
  cannot execute outside the accepted envelope or access state not named by the
  attached record/profile.
- rule installation cannot name unauthorized source objects or destination
  queues.
- rule and servicelet actions can only mark, count, drop, timestamp,
  hash-steer, select authorized gates, redact/sample telemetry, report
  `needs_software`, or route records within authorized queues.
- malformed, unknown, too-deep, encrypted, fragmented, or unsupported records
  become `partial`, `unknown`, `needs_software`, dropped, or faulted according
  to policy; they do not produce undefined authority.
- classifier execution has no loops, recursion, arbitrary bytecode, unbounded
  parse, unbounded rule walk, mutable protocol state, or connection tracking.
- servicelet execution uses only the verified LNP64 ISA subset; it cannot block,
  allocate, call gates, issue object operations, access arbitrary memory, mint
  or delegate capabilities, or perform hidden helper calls.
- verifier acceptance implies termination within the published bound and memory
  access only to the provided record envelope, constant table, fixed metadata
  window, and action record.
- verifier-envelope fields are complete for authority and timing: program
  length, instruction count, cycle bound, loop bound, ISA subset, attachment
  class, record profile, allowed fields, action set, authorized destinations,
  scratch state, and owner/source generations.
- servicelet semantics are limited to bounded prelude/postlude/filter/steering
  decisions. They cannot implement general service bodies that require
  blocking, allocation, unbounded traversal, mutable long-lived protocol state,
  helper callbacks, storage recovery, path walking, executable loading, or
  device enumeration.
- classifier/servicelet marks, counters, flow hashes, redacted records, and
  routed record envelopes are data, not capabilities.

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
- Resource Domain quote records include launch generation, restore generation,
  debug/forensics mode, delegated root capabilities, policy profile, and
  assurance-profile state for the quoted boundary.
- launch, restore, migration, and reattachment records cannot mark a boundary
  fresh unless stale capabilities, stale domain ids, stale generations, and
  previous restore epochs have been rejected or rebased.
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
- engines whose lifecycle profile includes degraded mode reject new commands or
  accept only explicit recovery/query commands until supervisor/PID 1 policy
  clears them.
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
- a compromised personality remains a confined subject; it cannot inspect,
  schedule, debug, DMA into, or broaden authority for domains outside its
  delegated subtree.

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
- debug, forensics, snapshot, telemetry, and audit paths are observation
  surfaces; they cannot bypass Resource Domain policy or expose another domain
  without an explicit scoped observation capability.
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

## 32. Global Progress Under Bounded Faults

**The machine has no unspecified stuck state inside the fault model:** assuming
clock, reset, power, and at least one schedulable core/fabric path remain within
the implementation's bounded fault assumptions, every engine, thread, domain,
and accepted operation either progresses, parks on a valid waitable, fails with
a typed error, is canceled/revoked, enters a documented degraded state, or
escalates to a measured/audited machine-fatal state.

Useful sub-theorems:

- every hardware engine state is reachable with a defined outgoing transition,
  intentionally terminal, watchdog-recoverable, degraded if the module profile
  permits degraded mode, or machine-fatal.
- every parked TID is attached to a live waitable, operation id, timer, gate
  continuation, capacity event, revoked source, or fault source that can wake,
  cancel, timeout, or fail it.
- every accepted owner-engine command eventually completes, cancels, faults,
  times out, is revoked, or transfers to a documented recovery/degraded path
  when that path is part of the module lifecycle profile.
- every long-latency command has an operation id, owner domain/thread
  generation, timeout/watchdog class, cancellation class, and completion/fault
  target before it can outlive the issuing instruction.
- queue, ring, fabric, event-router, servicelet-lane, and command-FIFO
  backpressure is closed: full conditions produce park, `EAGAIN`, `EOVERFLOW`,
  coalesce/drop-with-count, pressure event, degraded state, or machine-fatal
  escalation.
- a local VMA, DMA, Capability, Event, Domain, Scheduler, Servicelet, Stream, or
  Object engine failure cannot silently corrupt unrelated domains or require
  full-chip reset when a local recovery/degraded path is specified; if no such
  path is specified, the only legal outcomes are typed error, poison, abort, or
  measured/audited machine-fatal state.
- local reset/degraded recovery cannot mint authority, skip generation checks,
  reuse poisoned metadata as valid, or complete stale continuations.
- if recovery is impossible, hardware enters a structured measured/audited
  panic or machine-fatal state rather than undefined behavior.

## 33. Adversarial Input Containment

**Hostile code and hostile inputs remain data, pressure, or typed faults:**
arbitrary malicious user code, malformed packets, hostile peers, malicious
service payloads, corrupted files, adversarial servicelets, and adversarial
timing cannot create authority, unbounded execution, cross-tenant observation,
or unspecified hardware states.

Useful sub-theorems:

- malformed, fragmented, encrypted, extension-header-heavy, oversized, or
  unsupported packets are dropped, marked `partial`/`unknown`/`needs_software`,
  queued as data, or faulted according to policy; they cannot crash
  classifier/servicelet hardware or create authority.
- all hardware record and packet parsing has bounded depth, bounded field
  extraction, fail-closed behavior, and no unbounded header walk.
- adversarial servicelet programs are rejected unless verifier-safe; accepted
  servicelets terminate within bound and cannot access arbitrary memory, block,
  allocate, helper-call, or mint/delegate capabilities.
- remote peers can consume only delegated endpoint, packet-queue, servicelet,
  event, and Resource Domain budgets; packet floods, connection floods,
  malformed records, and service-call storms resolve to bounded queueing,
  throttling, drop/coalesce counters, pressure events, `EAGAIN`, `EOVERFLOW`,
  quota exhaustion, or domain isolation.
- packet DMA writes only to authorized DMA buffers and cannot escape VMA,
  IOMMU, requester id, direction, generation, or Resource Domain scope.
- packet bytes, protocol payloads, filesystem bytes, service replies, trace
  records, and event payloads are data only; they cannot become executable
  mappings, FDR entries, gate continuations, servicelet programs, or scheduler
  state without explicit loader/verifier/capability paths.
- processes without `net_namespace`, `packet_queue`, endpoint, listener, or
  related capabilities cannot observe, inject, or steer network traffic.
- a compromised namespace, filesystem, loader, network, telemetry, or PCIe
  service still cannot mint capabilities, receive raw interrupts, perform raw
  DMA, inspect unrelated memory, bypass Resource Domain limits, or publish
  broadened returned-capability proposals.
- telemetry, trace, audit, and fault records caused by hostile input cannot leak
  unauthorized payloads, tenant secrets, sealed confidential-domain data, or raw
  packet contents outside delegated observability scope.
- adversarial timing cannot turn races around revocation, wait/wake, page fill,
  DMA completion, service reply, or gate return into authority amplification or
  lost scheduler state.

## 34. Refinement Targets

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
