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

## 2. Capability Non-Forgeability

**Non-forgeability:** user code cannot manufacture authority by writing integer
values, memory bytes, registers, or message payloads.

Only the Capability/FDR engine, object owner engines, boot engine, and explicitly
authorized mint capabilities can create authority-bearing FDR entries.

Useful sub-theorems:

- GPR values are never interpreted as capabilities without FDR-table lookup.
- memory stores cannot create valid FDR entries.
- received capabilities must originate from `CAP_SEND` or an authorized object
  mint path.
- object ids without matching generation and rights confer no authority.

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

## 5. Generation Safety

**No stale aliasing:** a stale handle cannot accidentally authorize access to a
new object that reused the same table slot or object id.

Useful sub-theorems:

- object reuse increments generation before publishing new authority.
- FDR, VMA, waitable, call-gate, DMA-buffer, and domain operations check object
  generation before use.
- uncorrectable metadata faults do not advance generation into a fresh-looking
  valid object without supervisor acknowledgement.

## 6. Resource Domain Containment

**Domain containment:** a child domain cannot exceed the authority, security
policy, or budgets delegated by its ancestors.

Useful sub-theorems:

- limits are monotonic down the domain tree.
- usage rolls up to all ancestors.
- freeze/kill/revoke on a domain subtree reaches every attached descendant.
- ASLR/JIT/DMA/device/entropy/security policy in a child can only be stricter
  than the delegated parent policy.
- supervisor upcall policy can be masked or translated by parents but not used
  to escape containment.

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
- `WAKE`, signal delivery, fd readiness, timer expiry, call completion, and
  domain resume transition eligible threads back to runnable at most once.

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

## 9. VMA and Memory Safety

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

## 10. W^X and Executable Provenance

**W^X invariant:** no page is simultaneously writable and executable unless the
current Resource Domain has explicit JIT/loader authority and the mapping is in
an allowed transition state.

Useful sub-theorems:

- ordinary anonymous, heap, stack, queue, DMA, and device mappings are NX.
- executable mappings originate from executable image objects or authorized
  loader/JIT transitions.
- `ISYNC` cannot make non-executable memory executable by itself.

## 11. DMA Isolation

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

## 12. Signal and Fault Delivery Safety

**Faults are precise enough for recovery or termination:** synchronous faults
are delivered to the correct thread with a well-formed signal frame or structured
fault record.

Useful sub-theorems:

- divide-by-zero, illegal opcode, permission fault, guard fault, and bad device
  mapping produce the specified signal/upcall.
- signal frames are non-executable and cannot forge privilege or capability
  state.
- `SIGRET` restores only the hardware-saved context for that interrupted
  thread.
- supervisor opcode upcalls cannot bypass normal capability/domain checks.

## 13. Commit/Abort Atomicity

**No partial publication:** multi-step hardware operations expose either the old
state or the committed new state, never an inconsistent middle state.

Useful sub-theorems:

- `OPEN_AT`, `NS_CTL`, `SET_META`, `MMAP`, `MUNMAP`, `CLONE`, `EXEC`,
  `DOMAIN_CTL`, `CAP_REVOKE`, and `CALL_CAP` each have a single architectural
  commit point.
- cancellation before commit rolls back all reservations.
- cancellation after commit completes, rolls forward, or reports a defined
  failure without corrupting authority-bearing metadata.
- result registers and `ERRNO` are updated only at completion.

## 14. Filesystem Durability Contract

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

## 15. Boot Measurement Integrity

**Measured boot consistency:** the boot measurement log accurately reflects the
boot manifest, selected images, reset cause, FPGA build id, and boot policy
observed by PID 1.

Useful sub-theorems:

- measurement records are append-only during boot.
- PID 1 cannot observe a booted image without the corresponding measurement
  record being present.
- boot policy failure either records a structured fault and enters permitted
  development mode or enters hardware panic.

## 16. RAS Fault Containment

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

## 17. Trace and Counter Non-Interference

**Observability does not change authority:** reading counters, trace rings, or
fault records cannot create, broaden, or revive capabilities.

Useful sub-theorems:

- trace records are data, not FDR entries.
- destructive trace reads only advance trace-consumer cursors.
- counter overflow or wrap cannot affect scheduler, domain, VMA, or capability
  state.

## 18. Paravirtual Personality Containment

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

## 19. Cross-Domain Call Safety

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

## 20. Refinement Targets

After the abstract theorems exist, each hardware block should prove or test a
small refinement claim:

- its RTL-visible state maps to an abstract object state.
- every accepted command corresponds to an allowed abstract transition.
- every response corresponds to the abstract transition result.
- every timeout, reset, or fault maps to an allowed abort/degraded/panic
  transition.
- no local cache hit can bypass generation, rights, domain, or revocation
  checks required by the abstract model.

These refinement claims are narrower than the whole architecture proof, but they
are what make the abstract Lean model relevant to an FPGA implementation.
