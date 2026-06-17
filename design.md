Here is the draft Instruction Set Architecture (ISA) for the **LNP64 (Linux-Native Processor 64-bit)**. The design is a capability/event/domain machine with POSIX as its primary compatibility profile. It does not freeze historical Unix as the hardware model; it exposes durable primitives that make libc, Unix personalities, drivers, and runtimes straightforward to build.

---

# LNP64 Instruction Set Architecture (Draft v1.0)

## 1. Register Architecture
To support hardware-native resource primitives, the standard register file is expanded beyond General Purpose Registers (GPRs) to include File Descriptor Registers (FDRs) and Process Control Registers (PCRs).

*   **GPRs (General Purpose):** `r0` - `r31` (64-bit, standard ALU operations).
*   **LR (Link Register):** Thread-local 64-bit return-address register. `CALL` / `CALL_REG` write `LR = PC + 8`; `RET` jumps to `LR`.
*   **FDRs (File Descriptor Registers):** `fd0` - `fd255` are the static low-descriptor fast bank. Full process FDR tables are DDR-backed and addressed by dynamic FDR instructions. An FDR is a hardware capability handle, not a Unix integer descriptor. POSIX file descriptors are the libc/personality interpretation of these handles. FDRs reference namespace services, object services, streams, files, device objects, event queues, timers, generic counters, generic queues, memory objects, PCIe BARs, DMA buffers, call gates, or supervisor controls. `fd0`, `fd1`, and `fd2` conventionally bind to STDIN, STDOUT, and STDERR streams of the controlling TTY.
*   **PCRs (Process Control Registers):**
    *   `PID`: Current Process ID, from process context.
    *   `PPID`: Parent Process ID, from process context, or `0` for root.
    *   `TID`: Current Thread ID, from thread context.
    *   `UID` / `GID`: User/Group ID from process credential context.
    *   `CAPMASK`: Process credential capability bitmap.
    *   `SIGMASK`: Thread-local 64-bit bitmask of currently blocked signals.
    *   `REALTIME_SEC` / `REALTIME_NSEC`: Read-only realtime clock snapshot
        fields used by libc/runtime clock surfaces. Timer waitability remains
        FDR-backed through timer profiles.
*   **ERRNO:** Thread-local compatibility error register. Fallible instructions write their result to the encoded destination register and update thread-local `ERRNO` on failure so libc can expose normal POSIX semantics; the architectural result/error convention remains explicit.

## 1.1 Architectural Layering

The native LNP64 primitives are not Unix syscalls in silicon. They are:

*   **Capability handles:** FDR entries carrying unforgeable authority to objects.
*   **Objects:** streams, queues, counters, memory objects, namespace entries, service-owned files, devices, DMA buffers, call gates, event queues, and control endpoints.
*   **Waitables and events:** readiness, completion, timer, futex, child-exit, IRQ, signal, and supervisor events.
*   **Resource Domains:** nested containment, accounting, security policy, virtualization, cgroup, sandbox, and supervisor boundaries.
*   **VMAs and address spaces:** hardware-managed memory mappings derived from memory/image/device capabilities.
*   **Scheduler contexts:** hardware-owned runnable, running, blocked, parked, and destroyed thread state.
*   **Metadata/control surfaces:** typed `GET_META`, `SET_META`, `OBJECT_CTL`, `DOMAIN_CTL`, and `NS_CTL` operations.

POSIX, Linux syscall compatibility, and NetBSD rump-style services are profiles over these primitives. This keeps libc clean: familiar APIs lower to stable native operations, while native software can use the cleaner capability/event/domain model directly.

## 2. Process & Scheduling Instructions
The CPU features a hardware-managed runqueue. There is no mandatory OS scheduler tick; hardware scheduler and context-store blocks dispatch ready threads directly.

The v1 scheduler is a hardware weighted-fair virtual-time scheduler inspired by
Linux CFS/EEVDF, but it is not Linux CFS in RTL. Hardware owns runnable,
running, blocked, parked, frozen, and exited state transitions; wakeup
insertion; runqueue selection; Resource Domain budget accounting; and bounded
preemption points. Software owns policy intent: domain weights, quotas, latency
class hints, affinity masks, guest/personality scheduler mappings, and workload
admission. The scheduler uses fixed weight tables, virtual runtime/deadline
accounting, bucketed or small-window runnable queues, and hierarchical domain
credits. It must not expose scheduler bytecode, callbacks, arbitrary policy
plugins, or unbounded tree walks.

*   **`CLONE r_dest, r_flags_or_argblock`**
    *   *Action:* Native process/thread creation primitive. Creates a new thread or process according to an explicit profile and bounded share/copy flags. `pthread_create`-like source forms lower to `profile=thread`; native actor/process creation lowers to explicit `profile=process`; POSIX `fork()` lowers to constrained `profile=posix_fork` with a new PID, exactly one child thread, COW VMAs/heap metadata, defined FDR inheritance, copied credentials/dispositions, copied caller signal mask, cleared child pending signals, and no in-flight operation ownership copied.
*   **`EXEC r_result, r_exec_argblock`**
    *   *Action:* Commits a loader-produced exec-plan descriptor. POSIX `execve(path, ...)` first performs namespace-dispatch `OPEN_AT`, then a loader service or runtime parses the executable format, applies relocations and interpreter policy in software, prepares memory/source capabilities and startup metadata, and submits a hardware-visible exec plan. Hardware validates that plan, enters a process-wide exec barrier, stops sibling threads, cancels/detaches in-flight operations, invalidates old thread contexts, atomically replaces the VMA/register/startup state, and resumes with exactly one surviving thread. If validation or pre-commit cancellation fails, the old image remains runnable.
*   **`YIELD`**
    *   *Action:* Suspends the current thread, saves state to the hardware thread context, and selects another ready TID from the hardware runqueue at a bounded scheduling point.
*   **`EXIT r_exit_code`**
    *   *Action:* Destroys the current hardware thread context. If it's the last thread in the PID group, triggers hardware VMA teardown and signals the parent PID with `SIGCHLD`.

## 3. I/O and File Operations
System calls are replaced by direct hardware resource commands. The binary ISA uses a compact stream/object/capability model; POSIX-shaped source names are assembler or libc lowering aliases.

*   **`OPEN_AT r_dest, r_dirfd, r_path_ptr, r_flags`**
    *   *Action:* Hardware-mediated namespace dispatch. Validates the caller's directory/root/namespace capability, bounds and pins the path buffer, sends a lookup/open request to the owning namespace service domain, parks the caller, then verifies/narrows/installs the returned object capability. Source-level `open`, `openat`, and `opendir` lower to this instruction; hardware does not interpret filesystem formats or perform general directory walking.
*   **`PULL r_result, r_fd, r_buf_ptr, r_len_or_argblock`**
    *   *Action:* Pulls records from a stream object into memory. Files produce bytes, directories produce dirent records, sockets produce packets/messages, event queues produce event records, and block-image FDRs may use explicit-offset argument blocks.
*   **`PUSH r_result, r_fd, r_buf_ptr, r_len_or_argblock`**
    *   *Action:* Pushes records from memory to a stream object. Files consume bytes, sockets consume packets/messages, control FDRs consume commands, and block-image FDRs may use explicit-offset argument blocks.
*   **`SEEK r_result, r_fd, r_offset_or_cookie, r_whence`**
    *   *Action:* Repositions a seekable stream. Directory rewind is `SEEK(fd, 0, SET)`, and directory cookies use the same instruction.
*   **`AWAIT r_result, r_waitable, r_mask_or_argblock`**
    *   *Action:* Parks the current thread until a waitable object changes state. FDs, event queues, timers, child exit, futex predicates, PCIe IRQ events, message channels, and supervisor upcalls all lower to `AWAIT`.
*   **`CLOSE r_result, r_fd`**
    *   *Action:* Releases an FDR capability reference.
*   **`GET_META r_result, r_fd, r_meta_ptr, r_flags`** / **`SET_META r_result, r_fd, r_meta_ptr, r_flags`**
    *   *Action:* Reads or mutates metadata on an opened object through the typed control envelope. The envelope is a bounded, typed, authority-checked transaction format, not an opaque `ioctl` blob. It names object class, profile class, profile id, op id, version, flags, required rights, expected generation/lineage, bounded input/output lengths, scalar fields, capability arguments, and returned-capability slots. For hardware-owned objects this is handled by the object owner engine; for service-owned filesystems/devices this is dispatched to the owning service domain and completed as a capability-checked transaction. POSIX `stat`, `chmod`, `chown`, `utime`, fd flags, rights, durability/flush state, observability counters, and backend-specific metadata are typed profiles over this mechanism. Unknown well-formed profile classes/objects/ops return `ENOTSUP`; malformed records return `EINVAL`; oversized valid records return `EOVERFLOW`; authority failures return `EPERM`/`EACCES`; stale lineage returns `EREVOKED` or the object-specific stale-reference error; pre-commit cancellation returns `ECANCELED`.
*   **`NS_CTL r_result, r_argblock`**
    *   *Action:* Hardware-mediated namespace control dispatch relative to directory/root/namespace FDRs using the typed control envelope. Operations such as mkdirat, unlinkat, renameat, linkat, symlinkat, readlinkat, chdir, mount/delegation, and storage barrier/flush profiles are interpreted by namespace or filesystem service domains. Hardware validates the authority envelope, packages the request, parks the caller, and verifies that returned capabilities/status do not exceed delegated rights.
*   **`DUP`**
    *   *Action:* Duplicates or moves FDR capabilities. Exact destinations and narrowing flags are encoded in the instruction or argument block. Source-level `pipe()` lowers to `OBJECT_CTL create queue(profile=pipe)` plus narrowed read/write endpoint capabilities.
*   **`CAP_DUP`, `CAP_SEND`, `CAP_RECV`, `CAP_REVOKE`**
    *   *Action:* Architectural FDR capability management. Capabilities can be duplicated with narrowed rights, sealed against further delegation, passed out-of-band over permitted pipe/socket/control FDRs, received into a target FDR table, or revoked along a revocable lineage. Revocation has four architectural classes: `lazy_epoch` for cheap cached authority invalidation, `forced_cancel` for waits and pre-commit operations, `synchronous_quiesce` for DMA/page/device safety before reuse, and `poison_fault` for corrupted or untrusted stale state. Service domains may request or propose derived capabilities, but the Capability Engine performs the final mint/install commit from an existing mint/root capability after checking class, rights, range, generation, lineage, and Resource Domain policy. This is how the Bus Master delegates PCIe BARs, drivers receive DMA buffers and IRQ events, and supervisor domains pass authority without ambient privilege.
*   **`EVENT_CTL` / `TIMER_CTL` / `SUPERVISOR_CTL`**
    *   *Action:* `EVENT_CTL` and `TIMER_CTL` are source-level/profile aliases over `OBJECT_CTL` for event-queue and timer profiles. `SUPERVISOR_CTL` is a source-level/profile alias over `DOMAIN_CTL` for delegated supervisor domains.
*   **`OBJECT_CTL r_result, r_argblock`**
    *   *Action:* Creates, configures, queries, resets, or destroys the three generic hardware-owned object primitives through the typed control envelope: `counter`, `queue`, and `memory_object`. Semaphores, completions, event counters, channels, task queues, shared arenas, and DMA completions are runtime profiles over these primitives, not separate hardware modules.
*   **`DMA_CTL r_result, r_argblock`**
    *   *Action:* Submits bulk memory/object operations to the DMA Fabric: large copy, fill/zero, scatter/gather copy, and optional checksum/hash profiles. Small operations may complete synchronously; long operations can complete through an `event_queue` FDR or a `counter` completion profile. DMA always runs through VMA permissions, capability checks, IOMMU/device scope, and Resource Domain accounting.
*   **`DOMAIN_CTL r_result, r_argblock`**
    *   *Action:* Creates, configures, queries, freezes, resumes, exposes checkpoint support hooks, or destroys nested Resource Domains. Virtual machines, containers, cgroups, jails, sandboxes, and supervisor domains are the same hardware primitive with different profile records: delegated capabilities, limits, namespace roots, device/network scope, scheduler policy, and upcall masks. Checkpointing is hook-based in v1: hardware defines quiescent boundaries, bounded state queries, dirty-state hooks, and generation/lineage checks; checkpoint image formats, migration transport, device/service state capture, and full restore policy stay in software.

Typed control profiles are split into three classes. **Architectural profiles**
are stable hardware profiles such as domain control, counters, queues,
memory objects, telemetry, attestation, storage barriers, and classifier tables.
**Personality/service profiles** cover POSIX/Linux/BSD compatibility,
namespaces, sockets, loaders, and service-owned metadata. **Vendor/device
profiles** are allowed only behind explicit device capabilities. All three use
the same envelope validation, bounded lengths, capability argument rules,
returned-capability verification, explicit commit/cancel rules, and fail-closed
error behavior. Payload bytes and scalar fields are data only; all authority
enters through FDR/capability arguments and all returned authority is installed
only through returned-capability slots verified by the Capability Engine.

### 3.1 Native Service Model
LNP64's service boundary is architectural. Hardware owns authority, scheduling,
waitability, memory safety, accounting, and commit semantics. Services own
evolving policy: filesystem formats, namespace rules, loaders, networking
protocols, PCIe quirks, device management, Unix personalities, and synthetic
metadata.

A service is a normal process or domain with explicit service capabilities,
bounded request queues, call gates, event queues, and Resource Domain budgets.
It receives work only through hardware-mediated surfaces: `OPEN_AT`, `NS_CTL`,
`GET_META`/`SET_META`, `OBJECT_CTL`, `PULL`/`PUSH`, object-backed page-fill
requests, `CALL_CAP`, and event queues. Hardware packages requests as bounded
records with caller identity, object generation, lineage epoch, rights,
Resource Domain id/generation, copied bytes or pinned-buffer descriptors, and
explicit capability arguments. Services never receive ambient raw pointers, raw
interrupt vectors, raw DMA authority, physical addresses, or hidden privilege.

Service replies are data until committed by hardware. A reply may include
status, metadata, copied output, event records, or returned-capability
proposals. Returned authority is installed only through declared
returned-capability slots after the Capability Engine verifies the proposal
against an existing mint/root capability, object class, range, rights,
generation, lineage, receiver domain policy, and destination FDR policy.

Every service transaction has exactly one commit point. Before commit,
cancellation, signal interruption, service crash, domain teardown, or revocation
aborts the transaction, releases reservations, and wakes the caller with a
typed error such as `ECANCELED`, `EINTR`, `EPIPE`, `EREVOKED`, or a
profile-specific stale-service status. After commit, the operation completes,
rolls forward, drains already committed data, or follows the object's documented
teardown policy. If service-side work commits but returned-capability install
fails, hardware reports the install failure and must not publish substitute
authority.

Backpressure is explicit. Service request queues, reply queues, page-fill
windows, event queues, and call-gate continuations have bounded capacity and
Resource Domain accounting. Full queues either park the caller with `AWAIT`,
return `EAGAIN` for nonblocking profiles, or return `EOVERFLOW` when the
profile forbids waiting. No service boundary may allocate unbounded hidden
state.

Blessed v1 service patterns are namespace/filesystem services, block-image
services, loader/exec-plan services, network endpoint services, PCIe Bus Master
and driver services, telemetry/attestation services, and supervisor/personality
services. Forbidden patterns are ambient privileged daemons, untyped `ioctl`
blobs carrying hidden authority, unbounded path/tree walkers in hardware, raw
physical memory or MMIO delegation, raw interrupt delivery to software, and
service replies that manufacture FDR authority without Capability Engine
commit.

*   **`CALL_CAP r_result, r_call_gate_fd, r_arg0, r_arg1`** / **`RET_CAP r_result, r_value0, r_value1`**
    *   *Action:* Performs a fast call and return through a callable FDR capability. Call gates may target another thread, service queue, driver service, supervisor service, runtime actor, or Resource Domain entry point. Hot calls use bounded register arguments and pre-provisioned target state; cold domain/container/VM creation remains a `DOMAIN_CTL` operation. Call gates support synchronous, asynchronous, and handoff profiles.
*   **`ERRNO_GET r_dest`** / **`ERRNO_SET r_src`**
    *   *Action:* Reads or writes the thread-local POSIX error register. Fallible resource instructions write success or `-1` to their encoded result register and set thread-local `ERRNO` on failure.
*   **Child Waits**
    *   *Action:* Child completion is a waitable event. Source-level `waitpid` lowers to `AWAIT` on a child/process waitable and then `GET_META` for status where needed.

## 4. Memory Management (Silicon VMAs)
Page tables and VMAs are managed by fixed hardware MMU/VMA engines using bounded hardware-walked metadata structures. Hardware freezes the page-state machine and safety invariants, not a general file page cache or backing-object policy. The normative page states are `UNMAPPED`, `RESERVED`, `NONRESIDENT_OBJECT`, `FILL_PENDING`, `RESIDENT_CLEAN`, `RESIDENT_DIRTY`, `COW_SHARED`, `PINNED_DMA`, `REVOKING`, and `POISONED`; all multi-step transitions have explicit commit/abort points and deterministic race priority.

*   **`MMAP r_dest, r_hint_addr, r_len, r_prot, fd_src, r_offset`**
    *   *Action:* Hardware validates the mapping capability, allocates a VMA descriptor, and inserts it into the current PID's VMA tree. Anonymous mappings use hardware-owned zero/COW/page-state transitions. Object-backed mappings create nonresident object-backed page states; on fault, hardware runs the Object-Backed Page Transaction Protocol: it sends a fixed page request to the owning object/service, accepts only capability-authorized page/zero/shared-page/error/retry replies, and atomically installs the returned page only if the VMA generation, object generation, lineage epoch, permissions, memory type, and domain policy still match. Hardware does not implement general page-cache, dirty writeback, truncation, `msync`, or filesystem coherence policy.
    *   *Protection Flags:* `r_prot` includes read/write/execute, shared/private, guard-page, and memory type: `normal_cached`, `uncached`, `device_ordered`, or `write_combining`. Writable-plus-executable mappings are rejected unless the current Resource Domain has an explicit JIT/loader policy bit.
*   **`MUNMAP r_addr, r_len`**
    *   *Action:* Invalidates the VMA range, flushes the relevant TLB entries, and releases affected physical pages through the hardware page allocator when no longer referenced.
*   **`MPROTECT r_addr, r_len, r_prot`**
    *   *Action:* Updates the protection bits for an existing VMA range and invalidates affected translations. This supports software loaders, guard pages, W^X policy, and paravirtual Unix guests that map their process abstractions onto LNP64 VMAs.
*   **`ALLOC r_dest, r_size`** / **`ALLOC_EX r_dest, r_request_block`** / **`ALLOC_SIZE r_dest, r_ptr`** / **`FREE r_result, r_ptr`**
    *   *Action:* Allocates, queries, and frees byte-granular heap memory through the Hardware Heap Engine. The ISA exposes allocation intent and bounded policy hints, not allocator representation. `ALLOC`/`FREE` create hardware-owned allocation objects with object-level safety and accounting. `MMAP`, `memory_object`, and arena-style `ALLOC_EX` create backing regions for software-owned allocation representations with region-level safety and accounting. The heap is process-local, backed by anonymous NX VMAs, thread-safe in hardware, and integrated with `CLONE` copy-on-write and `EXEC` teardown. Hardware freezes the **LNP64 Default Heap Algorithm**: a domain-aware segregated bump allocator with fixed size classes, per-thread allocation windows, domain-owned slab/run pages, batched cross-thread frees, bounded quarantine/guard hooks, page-run large objects, checked metadata, generation checks, and Resource Domain accounting. Libc and language runtimes own higher-level object policy. `ALLOC_EX` supports alignment, zeroing, guard, debug, locality, allocation-class tags, arena profiles, shared/DMA eligibility hints, and optional memory-tag/debug-hardening flags. `ALLOC_SIZE` exposes the usable allocation extent to libc/runtime code so `realloc` can copy only the valid old allocation extent.

## 4.1 Exec-Plan Boundary
Hardware `EXEC` does not load programs or understand executable formats. It
commits a prepared architectural image. A loader, libc runtime, boot manifest
tool, or Unix personality owns ELF or other formats, dynamic-linker policy,
interpreter/shebang handling, relocation records, library search, auxv layout,
credential-transition policy, and package rules.

The exec-plan descriptor is the narrow hardware contract. It names the entry
PC, initial SP, optional TLS base, startup metadata pointer, VMA records, source
object capabilities, zero-fill ranges, FDR inheritance/close-on-exec behavior,
explicit startup FDR grants, executable provenance, ASLR-selected addresses, and
authorized domain/security deltas. Hardware checks only authority, generations,
lineage, W^X/NX, executable provenance, guard pages, memory type, Resource
Domain policy, FDR inheritance, and bounded descriptor shape.

The commit rule is simple: before the exec commit point, failure or cancellation
releases the exec barrier and the old image continues. After the commit point,
the old address space and sibling thread contexts are gone, exactly one thread
exists in the new image, and remaining failures are delivered through the normal
fault/termination path for that new image.

## 5. Signal Handling
LNP64 freezes a clean, widely used Unix signal subset in hardware because it is
useful for real software and precise hardware faults. Signals are still built on
the same event/fault delivery fabric as native waitables, but the v1 subset is
architectural: handler table, per-thread mask, pending signal state, saved
context, fault-to-signal mapping, `KILL`, `ALARM`, and `SIGRET`.

The hardware substrate is deliberately precise and bounded: dispositions are
process-wide, masks are per-thread, pending state is split into process-directed
and thread-directed records, synchronous faults target the faulting thread, and
process-directed signals select an eligible unmasked thread by a deterministic
implementation-profile rule. `SIGRET` restores only from a Signal Engine-owned
saved-context token/generation, never from authority stored in user memory.

The frozen subset intentionally excludes historical signal quirks that would
make the architecture less elegant: OS-specific restart behavior, arbitrary
signal-stack ABI variants, full POSIX realtime queueing semantics, signal-based
application IPC as a preferred primitive, and Linux/BSD-specific delivery
corner cases. Those remain libc/personality policy over hardware events.

*   **`SIGACTION r_signum, r_handler_ptr`**
    *   *Action:* Registers handler, default, or ignore disposition for a bounded v1 signal number. Handler entry is through a fixed psABI signal trampoline.
*   **`SIGMASK_SET r_mask`**
    *   *Action:* Updates the thread-local `SIGMASK` PCR and triggers delivery if an unmasked pending signal is now deliverable.
*   **`ALARM r_dest, r_seconds`**
    *   *Action:* Resets the process's POSIX alarm timer, returns the previous
        remaining whole seconds in `r_dest`, and enqueues `SIGALRM` when the
        timer expires. General multi-source timers remain FDR-backed timer
        profiles.
*   **`KILL r_pid, r_signum`**
    *   *Action:* Routes a signal request through the Signal Engine to the target PID/TID, subject to credential/capability checks, waking the target if it is in an interruptible wait.
*   **`SIGRET`**
    *   *Action:* Issued at the end of a signal handler. Restores the hardware-saved interrupted context for that thread and resumes normal execution.
*   **Fault Delivery**
    *   *Action:* Divide-by-zero and arithmetic traps raise `SIGFPE`; illegal or disabled opcodes raise `SIGILL` unless routed to a supervisor upcall; invalid or protected memory accesses raise `SIGSEGV`; alignment and unmappable physical/device accesses raise `SIGBUS`; breakpoints raise `SIGTRAP`. The signal frame records faulting PC, signal code, bad address where applicable, and the trapped opcode where useful.
*   **Interrupted Operations**
    *   *Action:* Interruptible `AWAIT`, futex, timer, `PULL`/`PUSH`, pending page-fill, and queued call-gate waits return `EINTR` or a typed interrupted status before handler entry. Operations past their commit point use their documented roll-forward/cancel policy. Linux/BSD `SA_RESTART` behavior is libc/personality policy.

---
To make the **LNP64** a fully functional processor, the capability/event/domain instructions must coexist with a conventional general-purpose compute architecture. Since namespace dispatch, capability, VMA, event, and runqueue logic consume meaningful FPGA resources, the general compute side should remain a lean in-order RISC architecture.

Here is how the general-purpose compute integrates with the LNP64 resource fabric.

---

### 6. Memory Access (Load/Store Architecture)
The LNP64 is a strict Load/Store architecture. ALUs only operate on registers. Because the CPU manages VMAs and page faults natively, a `LOAD` that faults can park the issuing thread while the VMA/Page Engine handles resident, anonymous zero-fill, COW, guard, object-fill-pending, or failed mapping states. No conventional kernel trap is required for native LNP64 faults.

*   **`LD r_dest, [r_base, r_offset]`**
    *   *Action:* Loads a 64-bit word from the virtual address `r_base + r_offset` into `r_dest`.
*   **`LD.B`, `LD.H`, `LD.W`, `LD.D`**
    *   *Action:* Byte (8-bit), Half-word (16-bit), Word (32-bit), and Double-word (64-bit) load variants.
*   **`ST [r_base, r_offset], r_src`**
    *   *Action:* Stores the contents of `r_src` into memory. Hardware automatically updates the "Dirty" bit in the silicon page table.
*   **`ST.B`, `ST.H`, `ST.W`, `ST.D`**
    *   *Action:* Byte, half-word, word, and double-word store variants. Half-word access is included so PCIe BAR mappings can use native 16-bit register accesses when required.
*   **`FENCE`**
    *   *Action:* Memory barrier. Orders normal cached memory, atomics, DMA visibility, POSIX engine completions, and device-memory operations according to fence flags and VMA memory type.
*   **`ISYNC r_addr, r_len`**
    *   *Action:* Invalidates instruction-cache state for an executable range or mapped object. This is required for JITs and code patching and uses the same hardware invalidation fabric as `EXEC` and `MPROTECT`.

### 7. Arithmetic and Logic Unit (ALU)
Standard 64-bit integer operations. Because threads are managed in hardware, the ALU pipeline reads and writes architectural state through hardware thread contexts.

*   **`ADD r_dest, r_src1, r_src2`** / **`SUB r_dest, r_src1, r_src2`**
    *   *Action:* Standard integer addition/subtraction.
*   **`MUL r_dest, r_src1, r_src2`** / **`DIV r_dest, r_src1, r_src2`**
    *   *Action:* Integer multiplication and hardware division. Division by zero is delivered through the Signal Engine as `SIGFPE`.
*   **`AND`, `OR`, `XOR`, `NOT`**
    *   *Action:* Standard bitwise operations.
*   **`LSL`, `LSR`, `ASR`**
    *   *Action:* Logical Shift Left, Logical Shift Right, Arithmetic Shift Right.

### 8. Control Flow (Branching & Execution)
Since there is no Ring 0 / Ring 3 boundary, native control flow is about
executing user logic and jumping to functions. Compatibility personalities may
receive explicit supervisor upcalls, but native LNP64 resource operations are not
implemented as syscall traps.

*   **`JMP r_target`** / **`JMP immediate`**
    *   *Action:* Unconditional jump to a virtual address.
*   **`CALL r_target`**
    *   *Action:* Writes `PC + 8` to the thread-local Link Register (`LR`) and jumps to `r_target`.
*   **`RET`**
    *   *Action:* Sets `PC = LR`. Software stack frames and spilling the link register are psABI conventions.
*   **`CMP r_src1, r_src2`**
    *   *Action:* Compares two registers and sets the hardware condition flags (Zero, Carry, Negative, Overflow).
*   **`BEQ`, `BNE`, `BLT`, `BGT`**
    *   *Action:* Branch if Equal, Not Equal, Less Than, Greater Than (evaluates condition flags).

### 9. Hybrid Resource-Compute Instructions (The "Glue")
Because "everything is a capability object" is the native hardware reality, we need instructions to move data between the general compute realm (GPRs) and the resource realm (FDRs and PCRs).

*   **`MOV r_dest, r_src`**
    *   *Action:* Move data between general purpose registers.
*   **`DUP r_result, r_dst_or_flags, r_src`**
    *   *Action:* Duplicates or moves an FDR capability, including `dup`, `dup2`, and narrowed-rights forms where permitted by the source capability.
*   **`GET_PCR r_dest, pcr_name`**
    *   *Action:* Reads a Process Control Register (like `PID`, `UID`, or
        `REALTIME_SEC`) into a general-purpose register for user-space logic.
        (e.g., `GET_PCR r1, PID`).
*   **`SET_PCR pcr_name, r_src`**
    *   *Action:* Writes to a permitted Process Control Register. Credential changes are checked against UID/GID and process capability policy; denied changes fail with a permission error and update thread-local `ERRNO`.
*   **`ENV_GET r_dest, r_key, r_index_or_buf, r_len_or_flags`**
    *   *Action:* Reads read-only process and machine metadata for libc/runtime startup: ISA version, page size, cache-line size, hardware feature bits, architectural limits, startup metadata pointer, personality flags, and timebase frequency. POSIX `argc`, `argv`, `envp`, and auxv layout are libc/personality ABI data behind that pointer, not hardware-interpreted state. This is not a replacement for immediates; constants still use normal instruction encodings or literal loads.
*   **`RANDOM r_dest, r_len_or_flags`**
    *   *Action:* Returns hardware entropy for ASLR, stack canaries, randomized capability ids, allocator hardening, and libc/runtime seeding. Small scalar requests return in `r_dest`; larger requests use a versioned argument-block variant that copies entropy into a caller buffer.

---
**Summary of the Compute Pipeline:**
The ALU and Control Flow instructions avoid privilege-transition overhead for native resource operations. If an ALU instruction calculates a buffer address and the next instruction is `PUSH`, decode can enqueue a File/DMA Engine command directly rather than entering a software syscall path.
The core ISA also needs synchronization, device-driver boundaries, floating-point/vector compute, and a boot path to be a practical v1 target.

To make the LNP64 bootable and useful, v1 includes **Synchronization, Device Drivers, Floating Point, and Bootstrapping**.

The following sections sketch those remaining pieces of the LNP64 architecture:

### 10. Synchronization (The Silicon Futex)
Because the CPU manages threads in a hardware runqueue, traditional software spinlocks would waste issue slots under contention. Hardware-level concurrency controls let a thread park on a waitable condition and let the scheduler run another ready thread.

*   **`LOCK.CMPXCHG r_dest, [r_addr], r_expected, r_new`**
    *   *Action:* Atomic Compare-and-Swap. The standard building block for mutexes. V1 locked atomics are single-copy atomic and sequentially consistent unless a future encoding explicitly requests weaker acquire/release semantics.
*   **`AWAIT futex([r_addr], r_expected_val)`**
    *   *Action:* The hardware equivalent of a futex wait. If the value at `[r_addr]` equals `r_expected_val`, after an acquire-style check, the CPU removes the current thread from the runqueue and parks it in a hardware wait-state attached to that memory address.
*   **`WAKE futex([r_addr], r_num_threads)`**
    *   *Action:* The memory controller performs release-style wake ordering, checks if any threads are parked on `[r_addr]`, and pushes up to `r_num_threads` back onto the active runqueue.
*   **`THREAD_JOIN r_result, r_tid, r_retval_ptr`**
    *   *Action:* Parks the caller until the target same-process hardware thread exits. On completion, copies the target thread's exit value to `r_retval_ptr` when nonzero and returns `0`; returns a POSIX-style error code for invalid or self-join cases.

### 11. The Device Driver Problem (PCIe Bus Master + Capability Devices)
If resource authority is capability-native, how does the CPU know how to talk to a newly released GPU, NVMe drive, or network card? We do **not** hardwire the full PCIe enumeration and quirk universe into the CPU. The hardware provides the safety-critical substrate, and a trusted software **PCIe Bus Master** domain handles the messy device-specific reality.

The v1 hardware includes:

*   PCIe Root Complex link support.
*   IOMMU / DMA remapping.
*   MSI/MSI-X interrupt routing into FDR event objects.
*   Page-table memory types for device mappings: `device_ordered`, `uncached`, and `write_combining`.

The PCIe Bus Master is a privileged process created from the boot image. It alone receives the PCIe Root Complex control capability. It enumerates bus/device/function topology, assigns BARs, handles quirks, configures IOMMU entries, and requests derived device capabilities for driver processes. The Capability Engine, not the Bus Master process, performs the final FDR mint/install commit from the PCIe root-control capability.

Driver processes receive capabilities such as:

*   `pci_function` FDRs for device identity and config ownership.
*   `pcie_bar` FDRs for page-granular BAR windows.
*   `dma_buffer` FDRs for pinned, IOMMU-exported memory.
*   `irq_event` FDRs for MSI/MSI-X vectors.
*   Higher-level `block_device`, `net_device`, `gpu_device`, or `accelerator` FDRs published after a driver binds.

For high-performance MMIO, a driver calls `MMAP` on a `pcie_bar` FDR. The VMA engine maps that BAR range into the driver's address space with `device_ordered` or `write_combining` PTE attributes. The driver then uses ordinary `LD` and `ST` instructions for doorbells, status registers, and framebuffers. There is no `PULL`/`PUSH` command wrapper per register access.

PCIe BAR capabilities are page-granular. The Bus Master may request only BAR FDRs whose offset and length are multiples of the system page size; hardware derives them only from the held PCIe root/function authority after validating device identity, BAR bounds, page alignment, IOMMU scope, lineage, and domain policy. The VMA engine checks the FDR at `MMAP` time and then relies on PTE permissions and memory type bits; it does not add sub-page bounds checks to every load/store.

This preserves the rule that ambient MMIO is forbidden. A process cannot load/store arbitrary physical device addresses. But if it holds a specific `pcie_bar` FDR, that FDR is the capability granting the right to map and access that device page range.

*   **`INB_RESERVED r_dest, r_port` / `OUTB_RESERVED r_port, r_src`**
    *   *Action:* Reserved fallback/debug port I/O for trusted boot or Bus Master code. Normal applications and ordinary drivers use FDR capabilities and `MMAP`-mapped BARs instead.
*   **`LOAD_UCODE r_buf_ptr, r_len`**
    *   *Action:* Reserved device-driver acceleration hook. In FPGA v1 this is a stub; it does not replace the Bus Master, IOMMU, BAR FDR, or capability-delegation model.

### 11.1 Native Networking Model

Networking is not a hardware TCP/IP stack and not POSIX sockets in silicon. It
is a TCP-friendly transport substrate: capability-scoped packet/record movement,
endpoint objects, waitability, steering, counters, checksums, timers, and
zero-copy DMA handoff over the same object, queue, event, DMA, and Resource
Domain primitives used elsewhere.

Native network authority is rooted in capabilities:

*   **`net_namespace` FDR:** Delegated network universe for a process or Resource Domain. It controls visible interfaces, address/port binding authority, raw packet permission, route view, quotas, and optional firewall/filter policy. A domain without a `net_namespace` capability has no ambient network authority.
*   **`net_interface` FDR:** Capability to a physical, PCIe, or virtual interface. It exposes link state, MTU, counters, queue creation, packet filter attachment, and offload metadata through `GET_META`, `SET_META`, and `OBJECT_CTL`.
*   **`packet_queue` FDR:** Capability-scoped L2/L3 packet ingress or egress queue, optionally narrowed by MAC address, ethertype, VLAN, IP protocol, address, port, or service-defined filter. Used by native network services, packet capture, virtual switches, DPDK-like runtimes, and paravirtual Linux/NetBSD stacks.
*   **`datagram_endpoint` FDR:** Message-oriented endpoint profile for UDP-like traffic, raw datagram protocols, QUIC-friendly flows, or local datagram services. It is an endpoint shape, not a hardware UDP state machine.
*   **`stream_endpoint` FDR:** Ordered byte-stream endpoint profile for TCP-like connections, local streams, TLS-wrapped services, QUIC streams, paravirtual transports, or future transport accelerators. It is not a promise that silicon implements TCP.
*   **`listener` FDR:** Passive accept queue. `PULL(listener)` returns a new `stream_endpoint` capability.

The same ISA operations cover networking:

*   `OBJECT_CTL` creates namespaces, endpoints, listeners, packet queues, filters, and completion/event queues where the caller holds authority.
*   `SET_META` performs bind, connect, listen, option/filter configuration, route/address updates where delegated, and graceful close/reset controls.
*   `GET_META` reads local/peer addresses, MTU, link state, endpoint state, counters, errors, timestamp/offload metadata, and quota pressure.
*   `PULL` receives packets, datagrams, stream bytes, accepted connection capabilities, and network event records.
*   `PUSH` transmits packets, datagrams, and stream bytes.
*   `AWAIT` waits for readable, writable, accepted, connected, closed, error, link-change, quota, or completion events.
*   `CAP_SEND` passes listeners, accepted connections, packet queues, or namespace subsets between domains.
*   `CAP_REVOKE` tears down delegated network authority and derived endpoints.

The silicon/software split is deliberate:

*   **Silicon owns:** safe packet movement, packet DMA, coherent visibility, IOMMU enforcement, page-granular BAR mappings, `irq_event` delivery, generic queues/counters/events, basic MAC filtering/steering where cheap, simple checksums/classification where cheap, timestamps where cheap, per-domain quotas, counters, trace, fault events, timer/counter objects useful to transport services, and zero-copy buffer handoff.
*   **Software domains own:** PCIe enumeration and quirks, Ethernet NIC drivers, Wi-Fi firmware/device protocols, Wi-Fi scan/association/authentication/roaming/regulatory policy, ARP/NDP, IP, TCP, UDP, QUIC policy, routing, firewall/NAT policy, TLS, DNS, socket compatibility, service discovery, congestion control, retransmission, pacing, loss recovery, keepalive policy, and socket-option semantics.

The typed endpoint boundary is stable:

*   `packet_queue` preserves packet record boundaries and carries packet envelopes, payload references, checksum/timestamp/offload metadata, and queue readiness.
*   `datagram_endpoint` preserves datagram boundaries but does not imply hardware UDP; loss, truncation, peer metadata, and reliability are endpoint-profile/service policy.
*   `stream_endpoint` exposes ordered bytes, backpressure, close/reset/error readiness, and no packet boundaries; it may be backed by software TCP, local IPC, QUIC service, paravirtual transport, or future acceleration.
*   `listener` is an accept queue that returns endpoint capabilities whose rights and namespace scope derive from the listener and service policy.
*   `GET_META`, `SET_META`, and `OBJECT_CTL` expose bind/connect/listen/shutdown/nonblocking/buffer/event/socket-option profiles as bounded typed records. Unsupported options fail closed rather than becoming raw `ioctl` blobs.

A future TCP accelerator may be added only as an optional transport service
profile behind the same `stream_endpoint` capability shape. Applications, libc,
and POSIX socket compatibility must not depend on whether a stream endpoint is
implemented by software TCP, local IPC, QUIC service, paravirtual networking, or
a hardware assist block.

For PCIe Ethernet, the Bus Master requests `pci_function`, `pcie_bar`, `dma_buffer`, and `irq_event` capabilities for a NIC driver domain, and hardware derives/installs them from the PCIe root/function authority. The driver maps BARs with `MMAP`, allocates descriptor rings and packet buffers through `dma_buffer` capabilities, waits on `irq_event` records for MSI/MSI-X completion, and publishes `net_interface` plus packet queue capabilities to a network service domain. That service domain exposes `stream_endpoint`, `datagram_endpoint`, and `listener` FDRs to applications and libc.

For Wi-Fi, silicon remains the same PCIe/DMA/event substrate. Wi-Fi-specific firmware loading, scan, association, WPA/WPA2/WPA3, roaming, regulatory behavior, power management, and link policy belong in a Wi-Fi driver/service domain. Once associated, the service publishes a normal `net_interface` capability to the rest of the system.

POSIX sockets lower cleanly onto this model: `socket()` creates an endpoint under a `net_namespace`, `bind`/`connect`/`listen` become typed metadata/control operations, `accept` pulls a connection capability from a listener, `send`/`recv` become `PUSH`/`PULL`, `poll`/`epoll` bind endpoint readiness into event queues, `getsockopt`/`setsockopt` become typed metadata records, and descriptor passing maps to `CAP_SEND`.

### 11.2 Bounded Record Classification and Queue Steering

The networking classifier is useful beyond networking, so it should be specified as a generic bounded record-classification engine with packet parsing as one profile.

The engine accepts a record envelope plus a capability-scoped rule table and can:

*   extract a bounded set of fixed fields from known envelope profiles.
*   compare exact values, masks, prefixes, ranges, and small enumerations.
*   compute simple hashes for queue steering.
*   stamp metadata fields such as class id, flow hash, timestamp, priority, or mark bits.
*   increment counters.
*   route, copy-reference, drop, or mark records into capability-scoped queues.

Useful profiles include:

*   **Packet profile:** shallow L2/L3/L4 extraction for simple Ethernet, VLAN, IPv4/IPv6, TCP/UDP/SCTP/ICMP headers; checksum status; flow hash; queue steering.
*   **IPC/message profile:** route typed messages or call-gate completions to worker queues by service id, method id, tenant/domain id, priority, or hash.
*   **Storage/DMA completion profile:** route completions and faults by object id, operation id, domain id, priority, or error class.
*   **Event/trace profile:** classify structured fault, trace, scheduler, and RAS records for observability without waking a general supervisor for every record.
*   **Runtime profile:** steer task, actor, or executor records to per-core/per-domain queues.

This is not an arbitrary packet VM or eBPF replacement. V1 classifier rules are bounded, table-driven, versioned, capability-owned, and loop-free. If a record is malformed, too deep, encrypted, fragmented, extension-header-heavy, or unknown, the classifier marks it `partial` or `needs_software` and still delivers it safely to a software-owned queue. Protocol state, connection tracking, routing policy, firewall languages, TLS, Wi-Fi management, and application semantics remain in software domains.

### 12. Floating Point & Vector Math (FPU/SIMD)
General compute isn't just integers. We need a standard IEEE 754 Floating Point Unit and SIMD (Single Instruction, Multiple Data) for multimedia and AI.

*   **`FADD`, `FSUB`, `FMUL`, `FDIV`**
    *   *Action:* Standard floating-point arithmetic operating on dedicated FPU registers (`f0` - `f31`).
*   **`VADD.32 v_dest, v_src1, v_src2`**
    *   *Action:* Vector addition. Adds multiple 32-bit integers simultaneously across wide vector registers (`v0` - `v15`), identical to AVX/NEON.

### 13. Bootstrapping (Hardware PID 1)
How does this machine actually turn on without a conventional bootloader or
kernel? The reset controller creates the initial operating envelope and commits
a bounded manifest-provided exec plan for PID 1. It is not a general executable
loader.

Upon receiving power, the LNP64 executes a hardwired reset sequence:
1.  Initializes the hardware VMA tree, scheduler fabric, root Resource Domain, default weighted-fair scheduler profile, telemetry/fault routes, capability roots, and runqueue.
2.  Creates the initial hardware process/thread context (PID 1, TID 1, UID 0) inside a PID 1 Resource Domain with valid CPU, memory, FDR, event, telemetry, and device budgets.
3.  Reads a boot manifest from SD, SPI flash, or another boot backend by fixed offset/table records, not by hardware path walking.
4.  Computes manifest/image measurements and exposes the measurement log and FPGA build id through `ENV_GET` and a boot-control FDR. Signed boot images are optional for FPGA v1, but the measurement path is architectural.
5.  Commits PID 1 from the manifest's exec-plan record and grants initial FDRs named by the manifest: stdio, boot-control, block/storage objects, root namespace service, and any initial service/control capabilities. The manifest exec-plan is an architecture record, not a general executable format.
6.  If a boot manifest names a namespace service, filesystem service, or PCIe Bus Master, creates those privileged service processes and grants only their explicit control capabilities. PCIe enumeration and path semantics are deferred to those services.
7.  If the manifest lacks a valid PID 1 image, the reset controller enters a hardware panic state and emits board diagnostics.

Reset creates a **default operating envelope** before any user instruction
executes. No runnable thread exists outside a Resource Domain, no thread can run
before its scheduler/accounting records are initialized, W^X/NX/ASLR/guard
defaults are installed, raw interrupts are already consumed by the Event Router,
and all initial authority is represented by explicit FDR capabilities. PID 1 may
refine policy and launch services, but it does not create the authority,
scheduler, memory, or telemetry model from scratch.

### 14. Paravirtual Unix Guest Profile
LNP64 does **not** add traditional kernel rings, mandatory syscall traps, or OS-owned page tables just to make Linux or NetBSD feel at home. The hardware remains capability/event/domain-native. A Unix kernel port is plausible by treating Linux/NetBSD as a paravirtual personality process, similar in spirit to User-Mode Linux or a microkernel guest.

In this model, the silicon remains authoritative for:

*   Hardware process and thread creation.
*   Runqueue scheduling and context switching.
*   VMA creation, teardown, page faults, and copy-on-write.
*   FDR capabilities, namespace dispatch, object references, and hardware-owned resource objects.
*   Signals, futex queues, fd readiness, and DMA completion.

The Linux/NetBSD personality owns:

*   Linux/BSD-specific process metadata and domain profiles for namespaces, cgroups, jails, credentials, and policy.
*   Compatibility APIs not directly represented by LNP64 opcodes.
*   Guest filesystems mounted inside block-image or storage-service FDRs.
*   Network stack policy above raw frame or datagram hardware objects.
*   Userland ABI conventions.

This makes the implementation path clean:

*   libc lowers POSIX APIs to native primitives: `open/read/write/close` to `OPEN_AT`/`PULL`/`PUSH`/`CLOSE`, `pipe` to a queue profile, `poll`/`epoll` to event queues, `mmap` to VMA mapping of a capability, and `errno` to the compatibility error register.
*   fork-like behavior is a `CLONE` profile, not the conceptual center of the machine. Native code can prefer spawn, call gates, domains, explicit shared memory, and event queues.
*   signals remain available for POSIX and hardware faults, but native code can use structured events and cancellation objects instead.
*   UID/GID is a credential profile for POSIX files and imported software; native authority is still capability possession plus Resource Domain policy.

The targeted compatibility approaches are:

*   **Linux as a paravirtual personality:** A Linux kernel port runs as a supervisor Resource Domain over a delegated LNP64 process subtree. Linux tasks, files, memory mappings, signals, futexes, cgroups, containers, nested guests, and devices are projected onto native hardware primitives.
*   **Linux syscall compatibility runtime:** A loader/libc/runtime maps Linux syscall ABI calls onto native LNP64 instructions without booting a full Linux kernel. This is the shortest path to running many cloud-oriented programs.
*   **NetBSD rump-kernel style:** Selected NetBSD filesystem, networking, or device stacks run as LNP64 service processes. They receive block, network, PCIe, or delegated namespace FDRs and expose services back through native FDRs.

A full traditional Linux/NetBSD port that owns page tables, context switching, interrupts, and raw devices is not the v1 target.

The compatibility interface is deliberately narrow. A personality observes and controls native objects through fixed FDR surfaces: lifecycle events for `CLONE`/`EXEC`/`EXIT`, VMA and page-fault events, FDR/capability transfer, namespace dispatch, block-image/storage objects, hardware signal/fault records, event queues, futex/timer waitables, network endpoint/packet queue capabilities, PCIe BAR/DMA/IRQ-event capabilities, and domain control upcalls. It may translate these into Linux or BSD concepts, but it cannot own page tables, raw interrupts, the scheduler, raw DMA, or capability minting.

The key hardware mechanism is a **Resource Domain**, not a privilege ring. A Resource Domain is a nested capability and accounting container for a process subtree, FDR authority, VMA/memory budget, scheduler budget, event policy, namespace root, and delegated devices. Virtual machines, containers, cgroups, jails, sandboxes, and supervisor domains are profiles of this same primitive. Hardware sees all of them as child domains in the same containment algebra; software decides whether a child domain is presented as a VM, a container, a cgroup, or a sandbox.

`DOMAIN_CTL` creates child domains by delegating a subset of the caller's own authority downward. Limits are monotonic: a child domain cannot exceed resources or capabilities delegated by its parent. Usage accounting rolls up the domain tree, so CPU, memory, PID/thread, I/O, device, and event pressure can be queried or limited at any nesting level.

Nested virtualization is modeled as nested domains. A Linux personality domain can create a KVM-like guest domain or a container-like child domain with the same `DOMAIN_CTL create child` operation. A VM profile grants stronger supervisor/upcall policy and paravirtual device views; a container profile shares more parent personality/runtime state and receives narrower namespace/device/resource scopes. Each layer may receive, translate, or mask upcalls for its children, but hardware still enforces resource budgets, capability lineage, and VMA/FDR isolation.

`DOMAIN_PROFILE_TENANT_STRICT` is the cloud isolation profile. It requires W^X, NX data, ASLR, guard-page support, generation checks, scoped entropy, DMA isolation, no raw interrupts, no ambient devices, no parent memory inspection unless explicitly delegated, and explicit shared-memory capabilities for every cross-tenant data path. Parent domains may freeze, kill, measure, query permitted aggregate usage, revoke delegated capabilities, and receive fault/pressure events, but they do not gain implicit read/write authority over tenant memory or sealed secrets.

Confidential-domain hooks extend the same profile without changing the object model. A confidential child can request a memory-encryption/key-id tag, measured launch policy, explicit shared-page declarations, sealed secret release only to matching measurements, and checkpoint encryption metadata owned by software. FPGA v1 may implement these as architectural hooks and refusal paths rather than production cryptography, but the domain record and capability rules must not require redesign to add real encryption later.

### 14.1 Assured Deployment Profiles
Assurance profiles are Resource Domain policy inputs and quoteable machine
facts:

The same mechanisms support three deployment postures: hyperscaler
multi-tenancy, federal/mission assurance, and owner-controlled open assurance.
They differ by profile policy, not by ISA fork.

*   **`ASSURANCE_DEV`:** development bitstreams may expose unsigned
    non-production quotes, wider debug, and permissive boot policy; quotes and
    audit records are marked development-mode.
*   **`ASSURANCE_FIELD`:** measured boot, locked debug by default,
    tenant-strict domain support, ECC/parity for critical metadata, watchdogs,
    telemetry FDRs, and tamper-evident audit streams.
*   **`ASSURANCE_HIGH`:** signed bitstream/manifest policy, production quotes,
    no invasive debug without explicit measured unlock, mandatory audit roots,
    MLS label enforcement, debug/forensics redaction, and no ambient device,
    interrupt, DMA, or telemetry access.
*   **`ASSURANCE_FORMAL`:** same runtime behavior as `ASSURANCE_HIGH`, plus
    proof artifact hashes, theorem coverage metadata, RTL/IP provenance hashes,
    and toolchain/build identifiers.

Hardware is the Policy Enforcement Point. PID 1, domain managers,
personalities, services, and orchestration software are Policy Decision Points.
Their requests take effect only after hardware validates FDR rights, Resource
Domain policy, generation, lineage, label, measurement state, and assurance
profile.

Audit streams are append-only telemetry profiles with sequence numbers,
wrap/dropped counts, event class, bounded payload, previous-record hash, and
quoteable roots. Audit FDRs are narrowable by domain, label, event class, read
mode, and redaction policy. Audit records are data; they cannot mint authority.

Debug and forensics require debug-control FDRs, measured/audited unlocks, and
domain/object/range-scoped rights. Tenant-strict, confidential, and MLS domains
deny parent inspection unless an inspection or shared-memory capability was
delegated. Production profiles may disable invasive debug or require destructive
domain freeze before capture.

MLS labels may attach to domains, FDRs, telemetry, audit streams, DMA buffers,
packet queues, and service endpoints. Cross-label sharing requires an allowed
label relation. Declassification uses explicit call gates or control FDRs,
emits audit records, and returns authority only through the Capability Engine.

`MISSION_PROFILE` is a Resource Domain profile, not a mission planner. Fields:
mission id, minimum assurance profile, audit/attestation level, dependency graph
hash, allowed degraded modes, recovery priority, stale event/time budget, and
failure policy: `fail_closed`, `fail_degraded`, `fail_over`, `freeze`, or
`quarantine`. Dependencies are ordinary FDRs.

Mission state is bounded: `normal`, `degraded`, `recovering`, `frozen`,
`failed_closed`, or `quarantined`. Recovery cannot broaden authority. Fallback
services must already be delegated. Stale service generations cannot complete.
Mission evidence in quotes may include boot measurements, assurance profile,
mission profile hash, dependency graph hash, state, degraded reason, audit root,
proof artifact hash, domain launch measurement, and delegated capability-root
summary.

### 14.2 Owner Sovereignty and Open Assurance
LNP64 supports open RTL, reproducible builds, owner-installed roots of trust,
owner-held debug-control FDRs, and replaceable service stacks. Quotes prove
measured artifacts and active policy; they are not a DRM mechanism.

The ISA does not require vendor-only software, remote kill switches,
signed-only execution, vendor-exclusive trust roots, hidden management engines,
ambient vendor access, secret DMA paths, raw interrupt backchannels, or
authority outside FDRs. Managed deployments may require signed manifests and
locked debug by profile policy.

Filesystems, loaders, network stacks, personalities, domain managers,
telemetry collectors, and declassification services are replaceable software
services behind explicit capabilities. Replacements must obey the same
capability, lineage, label, audit, and Resource Domain rules.

Pre-provisioned domains can expose `call_gate` FDRs for hot cross-domain calls. This makes sandboxed libraries, service calls, driver calls, and guest/supervisor calls use the same capability-call path as cross-thread calls, while preserving domain budget accounting and capability checks.

A capability-marked domain can also act as a supervisor domain and receive upcalls for selected events: unsupported opcodes, delegated namespace lookups, permission decisions, child exit, signal delivery, fd readiness, timer expiry, futex events, block-image completion, resource pressure, limit violation, and process lifecycle changes.

Upcalls are delivered through a normal FDR with object class `control`. The supervisor pulls event records with `PULL` and pushes policy commands with `PUSH`. This keeps the design inside the FDR/capability model instead of reintroducing a syscall path.

The precise claim is: native LNP64 resource operations are hardware commands, not
software traps. POSIX and Linux compatibility personalities may still receive explicit hardware
upcalls for virtualization policy, unsupported opcodes, delegated namespaces,
and Linux syscall ABI emulation.

For this to be practical, LNP64 needs a stable psABI: calling convention, process entry layout, TLS, signal frame layout, errno convention, time/timer FDRs, and event-queue FDRs that can aggregate fd readiness, timers, child exit, signals, futex events, and supervisor upcalls.

For storage, a guest kernel can treat a large block-image or storage-service FDR as a paravirtual block device. It uses explicit-offset `PULL` and `PUSH`, then mounts ext4, FFS, or another guest filesystem inside that image. LNP64 provides the outer capability, DMA, eventing, and durability hooks; the guest kernel or filesystem service provides the filesystem semantics.

For physical PCIe devices, the PCIe Bus Master delegates `pcie_bar`, `dma_buffer`, and `irq_event` FDRs to guest or native driver processes. Drivers map BARs with `MMAP`, use `LD`/`ST` for device registers, use DMA buffer FDRs for device-visible memory, and wait on IRQ event FDRs for MSI/MSI-X completion.

For memory, the guest uses `MMAP`, `MUNMAP`, and `MPROTECT` to request native hardware VMAs. It does not write page tables directly. Linux/BSD tasks map one-to-one to hardware threads where practical, while the guest scheduler becomes an accounting and policy layer over the hardware runqueue.

This preserves the vision: Linux and NetBSD can be personalities projected onto native capability/event/domain silicon, rather than forcing LNP64 to become another trap-and-kernel RISC machine.

### Native Security Invariants

LNP64 security is expressed through Resource Domains, VMAs, FDR capabilities, and hardware-owned object/capability generations plus lineage epochs rather than through a separate kernel ring model.

Hard v1 invariants:

*   **W^X by default:** The VMA Engine rejects simultaneous writable and executable permissions unless a domain explicitly holds a JIT/loader policy bit. JITs use write-then-execute transitions with `MPROTECT` and `ISYNC`, not permanent RWX mappings.
*   **NX data:** Heap, stacks, queues, shared memory, DMA buffers, device BARs, signal frames, and ordinary anonymous mappings default non-executable. Executable mappings must originate from executable image objects or an explicitly authorized loader/JIT transition.
*   **ASLR:** Process startup, `EXEC`, `MMAP`, heap arenas, stacks, signal trampolines, shared objects, call-gate trampolines, and guard regions are randomized with hardware entropy unless disabled by a delegated domain policy.
*   **Guard pages:** Stacks, heap arenas, signal frames, large allocations, and selected runtime objects can request unmapped or no-access guard VMAs. Guard faults route through the normal hardware signal path.
*   **Entropy:** `RANDOM` is the architectural entropy source for libc, loaders, domain managers, allocator hardening, and compatibility personalities. `ENV_GET` reports feature bits; it does not provide secret randomness.
*   **Generation and lineage checks:** Domains, FDR entries, VMAs, heap arenas, waitable objects, call gates, event sources, DMA buffers, mapped device objects, and capability lineages carry generation/epoch fields. Stale or revoked references fail deterministically instead of silently reusing authority.
*   **Classed revocation:** Revocation advances lineage or revocation-root epochs and then follows the object's class: lazy cached invalidation, forced cancel, synchronous quiesce, or poison/fault. DMA buffers, IOMMU contexts, BAR mappings, and pages before reuse require quiescence; event sources, endpoints, classifier tables, and namespace handles can usually use lazy epoch invalidation; corrupted metadata becomes poison/fault until supervisor/PID 1 action.
*   **Revocation:** `CAP_REVOKE`, `DOMAIN_CTL`, `MUNMAP`, `MPROTECT`, and object teardown advance lineage/revocation epochs and invalidate cached descriptors, mappings, event sources, call gates, page-fill continuations, and DMA exports before authority is reused.
*   **Sealed and narrowed capabilities:** Authority can only move by explicit capability operations. Delegation may narrow rights, ranges, event masks, memory permissions, device scope, and transfer rights. Sealed capabilities can be used or transferred according to their rights but cannot be broadened or reminted by receivers.
*   **Capability minting discipline:** Software services never manufacture raw FDR authority. Namespace, network, PCIe, loader, supervisor, and filesystem services may select objects and propose returned capabilities, but hardware mints or installs authority only by deriving from an existing mint/root capability and only after validating object class, rights, ranges, generations, lineage, and domain policy. Service replies are data until the Capability Engine commits them.
*   **Memory visibility contract:** Normal cached memory is coherent and TSO-like by default. Ordinary loads/stores are easy for C/C++/Rust/Go/JVM runtimes and Unix personalities to reason about; weaker or device-specific behavior is opt-in through VMA memory type and explicit `FENCE`. Locked atomics are single-copy atomic and sequentially consistent in v1. Futex wait/wake, call-gate handoff, signal delivery, VMA/TLB updates, DMA visibility, `device_ordered`, and `write_combining` mappings have explicit ordering rules.
*   **DMA isolation:** Internal DMA, `DMA_CTL`, file I/O DMA, Ethernet, SD/SPI, and PCIe requester DMA all pass through VMA/capability checks, the coherent DMA fabric, Resource Domain accounting, and IOMMU/device scope. No device may DMA to arbitrary DDR or bypass revocation.
*   **Tenant-strict isolation:** `DOMAIN_PROFILE_TENANT_STRICT` combines mandatory memory hardening, no ambient devices, no raw interrupts, scoped telemetry, scoped DMA, explicit shared pages, and no parent memory read authority without a delegated capability.
*   **Confidential computing hooks:** Domain records reserve measured-launch, memory-encryption/key-id, shared-page, sealed-secret, and encrypted-checkpoint fields. Software owns secret policy and checkpoint formats; hardware enforces that confidential-domain memory and sealed capabilities are not exposed through ordinary parent inspection, telemetry, trace, DMA, or fault paths.

### Native RAS and Operability Invariants

Cloud-grade LNP64 does not require a production fleet stack in FPGA v1, but the
first hardware version must preserve the architectural hooks that make reliable
operation possible.

Hard v1 requirements:

*   **Critical metadata ECC/parity:** FDR tables, VMA descriptors, domain tables, scheduler queues, event queues, heap metadata, DMA descriptors, namespace-dispatch records, and hardware-owned object metadata carry parity or ECC according to width and storage class. Corruption becomes a fault event, not silent authority reuse.
*   **Fault event model:** Engine faults, ECC/parity faults, invalid metadata, poisoned pages, DMA faults, watchdog timeouts, and boot measurement failures are delivered as structured events to PID 1, a supervisor Resource Domain, or a configured control FDR.
*   **Watchdogs and local reset:** Long-latency engines have bounded timeout states, abort paths, and local reset/degraded modes. A stuck File, VMA, DMA, Capability, Event, or Domain engine should not require full-chip reset when local recovery is possible.
*   **Observability counters:** Domains and engines expose counters for issued/completed/aborted operations, queue depth, stalls, faults, bytes moved, scheduler transitions, capability sends/revokes, and resource pressure.
*   **Fleet observability without privileged scraping:** Counters, trace rings, pressure events, and fault records are FDR-backed telemetry capabilities with scope, generation, and domain policy. Monitoring domains receive delegated aggregate views; they do not scrape raw memory, raw interrupts, or global privileged state.
*   **Trace ring:** FPGA v1 includes a small optional trace ring for scheduler transitions, domain events, faults, capability delegation/revocation, call-gate calls, DMA completions, queue-steering decisions, and storage barriers.
*   **Remote attestation primitive:** The boot path records build id, FPGA bitstream/ROM identity, boot manifest hash, image measurements, domain launch measurements, selected boot policy, and delegated capability roots into read-only measurement records. A quote/attestation FDR exposes signed or development-mode attestations to authorized domains.
*   **Checkpoint and live-migration compatibility hooks:** `DOMAIN_CTL freeze/query-state/resume` must define quiescent state boundaries, bounded state records, dirty-state hooks, service callback events, endpoint drain/redirect hooks, and storage barrier integration for software checkpointing and future live migration. Hardware does not own checkpoint image formats, migration transport, or full restore in v1; future restore must create fresh domain/generation bases and reattach capabilities explicitly.
*   **Line-rate record classification and queue steering:** The bounded classifier is a first-class v1 datapath for packets, IPC completions, storage/DMA completions, trace records, and RAS events. It provides hash/mark/count/steer/drop actions into capability-scoped queues without becoming an arbitrary packet VM.
*   **Storage durability contract:** Storage services and block objects define commit points, flush/barrier ordering, and replay/fsck expectations before RTL freeze. Live-system atomicity is not enough; power-fail durability must be specified, but general writable filesystem policy is not implemented in hardware.
*   **Deterministic failure containment proofs:** Each hardware engine must have a small enumerated state model, explicit commit/abort boundaries, local reset/degraded states, and proof or exhaustive-test obligations that faults cannot silently create authority, corrupt unrelated domains, or require full-chip reset when local recovery is possible.

### Native Adoption Strategy
The native path should be faster, safer, or simpler than recreating the same
behavior in software. v1 keeps this boundary:

*   **Thread contexts:** `r31` remains an ordinary register; hardware owns thread
    context state, guard-page enforcement, runqueue state, waits, and wakeups.
*   **Timers:** time is exposed through reads, timer FDRs, event queues, and
    supervisor upcalls; ambient timer interrupts are not exposed to processes.
*   **Allocation:** `ALLOC`, `ALLOC_EX`, `ALLOC_SIZE`, and `FREE` provide the
    default hardware heap. `MMAP`, `memory_object`, and arena-style `ALLOC_EX`
    remain the escape hatch for custom runtimes and GC heaps.
*   **Device memory:** device registers are reachable only through mapped
    capability objects such as `pcie_bar`; arbitrary physical MMIO is forbidden.
*   **IPC:** small scalar messages use `MSG_SEND`; receive paths use
    `AWAIT`/`PULL`. Larger byte or capability payloads use queues, streams,
    `CAP_SEND`, and `CAP_RECV`.
*   **Call gates:** `CALL_CAP` supports synchronous, asynchronous, and handoff
    calls into pre-provisioned threads, services, actors, supervisors, or domain
    entries. Cold domain creation remains `DOMAIN_CTL`.
*   **Runtime objects:** hardware exposes only `counter`, `queue`, and
    `memory_object`; semaphores, completions, channels, task queues, shared
    arenas, and DMA completions are profiles over those primitives.
