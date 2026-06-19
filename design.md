# LNP64 Instruction Set Architecture (Draft v1.0)

LNP64 is a capability/event/domain machine with POSIX as its primary
compatibility profile. It exposes hardware-visible primitives for libc, Unix
personalities, drivers, runtimes, and native services without freezing a
historical kernel as the hardware model.

## 1. Register Architecture
To support hardware-native resource primitives, the standard register file is expanded beyond General Purpose Registers (GPRs) to include FDR capability registers and Process Control Registers (PCRs).

*   **GPRs (General Purpose):** `r0` - `r31` (64-bit, standard ALU operations).
*   **LR (Link Register):** Thread-local 64-bit return-address register. `CALL` / `CALL_REG` write `LR = PC + 8`; `RET` jumps to `LR`.
*   **FDRs (FDR capability registers):** `fd0` - `fd255` are the static low-descriptor fast bank. Full process FDR tables are DDR-backed and addressed by dynamic FDR instructions. An FDR is a hardware capability handle, not a Unix integer descriptor. POSIX file descriptors are the libc/personality interpretation of these handles. FDRs reference namespace services, object services, streams, files, device objects, event queues, timers, generic counters, generic queues, memory objects, PCIe BARs, DMA buffers, call gates, or supervisor controls. `fd0`, `fd1`, and `fd2` conventionally bind to STDIN, STDOUT, and STDERR streams of the controlling TTY.
*   **PCRs (Process Control Registers):**
    *   `PID`: Current Process ID, from process context.
    *   `PPID`: Parent Process ID, from process context, or `0` for root.
    *   `TID`: Current Thread ID, from thread context.
    *   `TP` / `TLS_BASE`: Thread pointer used by the psABI and local-exec TLS.
    *   `CRED_PROFILE`: Read-only credential profile id for the active personality or service profile.
    *   `CRED_HANDLE`: Read-only opaque credential object/token plus generation for profile-specific checks.
    *   `UID` / `GID`: Compatibility PCR names backed by the active POSIX
        credential profile when present. `POSIX_UID` / `POSIX_GID` are accepted
        source aliases for the same selectors. They are not native authority
        roots.
    *   `SIGMASK`: Thread-local 64-bit bitmask of currently blocked signals.
    *   `SIGPENDING`: Read-only thread/process pending-delivery summary.
    *   `REALTIME_SEC` / `REALTIME_NSEC`: Read-only realtime clock snapshot
        fields used by libc/runtime clock surfaces. Timer waitability remains
        FDR-backed through timer profiles.
*   **ERRNO:** Thread-local compatibility error register. Native instructions use
    explicit result/status conventions: success writes the operation's value, or
    zero for pure-status operations, to the encoded destination register; failure
    writes the negative architectural error code to that destination and commits
    no partial authority/state change unless the object profile explicitly
    defines a post-commit failure state. Thread-local `ERRNO` is a
    compatibility view used by libc/personality boundaries; POSIX wrappers
    translate native negative errors to `-1` plus `ERRNO` where required.

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

## 1.2 Base Hardware Platform Contract

Serious software can rely on a mandatory hardware contract, independent of
which libc, Unix personality, filesystem service, or network stack is running.
V1 hardware must specify these mechanisms:

*   **Feature discovery:** `ENV_GET` exposes ISA revision, implementation
    profile, supported opcode groups, object profiles, domain/security features,
    timer features, topology, cache/page/DMA geometry, architectural limits,
    WCET class limits, and servicelet verifier limits.
*   **Realtime instruction contract:** The full enterprise profile is
    realtime-capable by construction. Native instructions complete a bounded
    local architectural step or submit a bounded transaction; no instruction may
    hide unbounded DDR traversal, path walking, page fill, filesystem policy,
    device completion, subtree traversal, or service execution in its retire
    latency. Implementations publish latency classes through `ENV_GET`:
    Class A register/local datapath, Class B local metadata hit, Class C
    bounded enqueue/state transition/crossbar arbitration, and Class D
    bounded-submit asynchronous transaction.
*   **Time:** hardware provides a monotonic timebase, realtime snapshot fields,
    timer object profiles, timeout semantics for `AWAIT`, and per-domain CPU
    accounting ticks. Timer precision, suspend/freeze behavior, and timestamp
    provenance are implementation-profile fields.
*   **Faults and overflow:** instruction, memory, capability, domain-policy,
    DMA/IOMMU, device, event overflow, watchdog, metadata, and machine-fatal
    faults have architectural classes and delivery rules. Bounded queues,
    rings, runqueues, audit streams, classifiers, DMA queues, and event queues
    must define full/overflow behavior: park, `EAGAIN`, `EOVERFLOW`, coalesce,
    drop-with-count, poison, or fatal fault.
*   **Bounded miss behavior:** User code does not need special slow-path
    knowledge. If a local FDR, VMA, heap window, gate continuation, waitable,
    scheduler slot, domain record, or servicelet attachment is cold, missing,
    full, or spilled, the instruction still completes in its latency class by
    returning a normal status, parking on an explicit waitable, or submitting a
    refill/owner-engine transaction. It must not silently become an unbounded
    instruction.
*   **Resource accounting:** Resource Domains account for CPU time, threads,
    processes, memory pages, VMAs, heap pages, FDRs, objects, event records,
    DMA bytes/ops, classifier entries, and queue occupancy. Parent domains see
    hierarchical usage snapshots subject to telemetry policy.
*   **Shared fabric arbitration:** Metadata engines, event routers, DMA paths,
    memory-controller ports, queue banks, and servicelet lanes expose bounded
    arbitration and Resource Domain admission. Best-effort traffic may be
    throttled or failed with pressure events, but it must not violate published
    bounds for admitted realtime work.
*   **Domain lifecycle:** `DOMAIN_CTL` defines create, configure, attach,
    detach, freeze, resume, destroy, revoke, query, and quiesce transitions.
    Domain ids, generations, usage records, scheduler state, and capability
    lineage are hardware state, not software convention.
*   **Snapshot hooks:** hardware defines quiescent boundaries, dirty-memory
    enumeration, bounded state cursors, object-generation changes, DMA/device
    quiescence, and restore reattachment checks. Image formats and migration
    transport remain software.
*   **Security state:** W^X/NX, ASLR enablement, entropy availability, measured
    boot, attestation, debug mode, DMA isolation, tenant-strict,
    confidential-domain, MLS, and audit-mode bits are queryable and enforced by
    Resource Domain policy.
*   **Topology:** `ENV_GET` reports core tiles, memory regions, cache/coherence
    domains, PCIe roots, DMA locality, and scheduler placement masks. FPGA v1
    may report a single coherent locality domain.
*   **Mandatory object profiles:** the base hardware object set is `counter`,
    `queue`, `event/completion`, `timer`, `memory_object`, `call_gate`,
    `dma_buffer`, `dma_completion`, and, when the classifier engine is present,
    `classifier_table` and `servicelet_program`. Pipes, semaphores, channels,
    epoll-like sets, task events, shared arenas, and socket readiness are
    source/runtime profiles over that set.

Software defines names, policies, file formats, protocol semantics, loader
rules, orchestration, and compatibility ABIs. Hardware defines the mechanisms,
state transitions, atomicity, isolation, accounting, and failure semantics above.

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

The scheduler contract is deliberately small and realtime-friendly:

*   Core tiles are hardware-multithreaded by interleaving in-order issue across
    many hardware contexts. V1 does not require speculative SMT/hyperthreading:
    one selected ready TID issues from a tile's active window, and blocked or
    pending TIDs stop occupying the issue lane.
*   Thread states are architectural: `READY`, `RUNNING`, `WAIT_*`,
    `GATE_DELIVERY`, `ZOMBIE`, and `DEAD`. A live thread is in exactly one
    state.
*   Runnable eligibility requires a non-frozen Resource Domain, positive budget
    across every ancestor domain, an allowed core/tile mask, and a resident or
    explicitly refillable scheduler context.
*   Affinity is split into a hard allowed core/tile mask and a soft preferred
    current tile. The scheduler is sticky by default: a runnable TID remains on
    its current tile when that tile is eligible and not under bounded balancing,
    affinity, quota, reservation, or fault pressure. Migration occurs only at
    scheduler boundaries and never while an instruction or owner-engine commit
    is mid-flight.
*   Dispatch chooses the eligible runnable entity with the earliest virtual
    deadline within a published approximation window; blocked threads do not
    consume CPU budget.
*   Wakeup insertion is bounded and may grant only a capped latency adjustment;
    it cannot reset virtual time or create unbounded credit.
*   Preemption occurs only at fixed accounting/timer boundaries, blocking
    resource commands, explicit `YIELD`, engine completion return, gate
    delivery boundaries, or supervisor-authorized forced park points.
*   `ENV_GET` exposes scheduler constants: weight table shape, latency classes,
    fairness approximation window, maximum wakeup insertion latency, maximum
    preemption latency, runqueue active-window size, spill/refill behavior, and
    supported reservation features.

*   **`CLONE r_dest, r_flags_or_argblock`**
    *   *Action:* Native process/thread creation primitive. Creates a new thread or process according to an explicit profile and bounded share/copy flags. `pthread_create`-like source forms lower to `profile=thread`; native actor/process creation lowers to explicit `profile=process`; POSIX `fork()` lowers to constrained `profile=posix_fork` with a new PID, exactly one child thread, COW VMAs/heap metadata, defined FDR inheritance, copied credentials/dispositions, copied caller signal mask, cleared child pending signals, and no in-flight operation ownership copied.
*   **`EXEC r_result, r_exec_argblock`**
    *   *Action:* Commits a loader-produced exec-plan descriptor. POSIX `execve(path, ...)` first performs namespace-dispatch `OPEN_AT`, then a loader service or runtime parses the executable format, applies relocations and interpreter policy in software, prepares memory/source capabilities and startup metadata, and submits a hardware-visible exec plan. Hardware validates that plan, enters a process-wide exec barrier, stops sibling threads, cancels/detaches in-flight operations, invalidates old thread contexts, atomically replaces the VMA/register/startup state, and resumes with exactly one surviving thread. If validation or pre-commit cancellation fails, the old image remains runnable.
*   **`YIELD`**
    *   *Action:* Suspends the current thread, saves state to the hardware thread context, and selects another ready TID from the hardware runqueue at a bounded scheduling point. `YIELD` is the canonical compiler-visible event-delivery intrinsic for code that wants to give queued gates/events/faults an explicit delivery point without performing I/O. C runtimes expose it as `__lnp_yield()` or an equivalent target builtin.
*   **`EXIT r_exit_code`**
    *   *Action:* Destroys the current hardware thread context. If it's the last thread in the PID group, triggers hardware VMA teardown and signals the parent PID with `SIGCHLD`.

## 3. I/O and File Operations
System calls are replaced by direct hardware resource commands. The binary ISA uses a compact stream/object/capability model; POSIX-shaped source names are assembler or libc lowering aliases.

*   **`OPEN_AT r_dest, r_dirfd, r_selector_ptr, r_flags_or_profile`**
    *   *Action:* Hardware-mediated namespace dispatch relative to a namespace, directory, root, or lookup-context capability. Hardware validates the caller's namespace capability, bounds and pins/copies a selector payload, dispatches a lookup/open request to the owning namespace service domain, parks the caller, then verifies/narrows/installs the returned object capability. POSIX `open`, `openat`, and `opendir` lower to the `posix_path` selector profile, where the selector bytes are a pathname string. Native services may define other selector profiles, such as object id, content hash, service key, package id, route tuple, or tenant-local name. Hardware does not parse `/`, `.`, `..`, symlinks, mounts, case folding, normalization, filesystem formats, or directory trees.
*   **`PULL r_result, r_fd, r_buf_ptr, r_len_or_argblock`**
    *   *Action:* Pulls records from a stream object into memory. Files produce bytes, directories produce dirent records, sockets produce packets/messages, event queues produce event records, and block-image FDRs may use explicit-offset argument blocks.
*   **`PUSH r_result, r_fd, r_buf_ptr, r_len_or_argblock`**
    *   *Action:* Pushes records from memory to a stream object. Files consume bytes, sockets consume packets/messages, control FDRs consume commands, and block-image FDRs may use explicit-offset argument blocks.
*   **`SEEK r_result, r_fd, r_offset_or_cookie, r_whence`**
    *   *Action:* Repositions a seekable stream. Directory rewind is `SEEK(fd, 0, SET)`, and directory cookies use the same instruction.
*   **`WAITABLE_PROBE r_result, r_waitable, r_mask_or_argblock`**
    *   *Action:* Performs a nonblocking readiness probe on a waitable object, fd,
        event queue, timer, child/process waitable, futex predicate, IRQ-event
        object, message endpoint, or gate completion. It never parks the caller.
        Success writes the ready mask/count/result bits to `r_result`; no-ready
        is a successful zero result; invalid, stale, or unauthorized waitables
        return a negative architectural error. POSIX `poll(..., timeout=0)`,
        `select(..., timeout=0)`, and epoll readiness scans lower here.
*   **`AWAIT_EX r_result, r_waitable, r_argblock`**
    *   *Action:* Atomically checks readiness and, if requested, parks the current
        thread on the waitable object. The argument block names wait mode
        (`probe`, `zero_timeout`, `relative_timeout`, `absolute_timeout`,
        `indefinite`), readiness mask/predicate, timeout value or timer object,
        interrupt/cancel policy, and optional completion event target.
        `probe` is equivalent to `WAITABLE_PROBE`; `zero_timeout` performs the
        same atomic check/arm path but returns immediately with zero if nothing
        is ready; bounded modes return zero on timeout unless the profile requests
        a timeout event record; indefinite mode parks until ready, interrupted,
        canceled, revoked, faulted, or domain-teardown. Ready completion writes
        the ready mask/count/result bits to `r_result`; failures write a negative
        architectural error.
*   **`AWAIT r_result, r_waitable, r_mask_or_argblock`**
    *   *Action:* Compact source/assembly form for the common indefinite wait.
        It lowers to `AWAIT_EX mode=indefinite` with the supplied mask or
        profile-specific wait predicate. FDs, event queues, timers, child exit,
        futex predicates, PCIe IRQ events, message channels, and supervisor
        upcalls all lower to `AWAIT_EX`; `POLL_FD`, `POLL_FD_DYN`, and
        `AWAIT_DYN` are compatibility/backend aliases, not preferred
        architectural names.
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
    *   *Action:* Creates, configures, queries, resets, or destroys hardware-owned object profiles through the typed control envelope: `counter`, `queue`, `event/completion`, `timer`, `memory_object`, `call_gate`, `dma_buffer`, `dma_completion`, and optional acceleration profiles such as `classifier_table` and `servicelet_program`. Semaphores, pipes, channels, task queues, shared arenas, socket readiness, DMA completions, filters, and servicelet attachments are runtime or acceleration profiles over these primitives, not separate ad hoc hardware modules.
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

### 3.1 Compiler-Visible ISA Contract

The ISA is frozen only where the toolchain contract is also frozen. Every
architectural opcode or required architectural profile must have:

* a fixed binary encoding and operand/result convention;
* assembler, disassembler, MC encoder, and object-roundtrip coverage in the real
  LLVM backend;
* a target builtin, private `__lnp_*` intrinsic, or inline-assembly constraint
  surface for C/C++ runtimes where source lowering needs it;
* emulator decode/execute coverage and an RTL decode path that either implements
  the operation or fails closed with the canonical architectural error; and
* conformance tests showing success, malformed input, stale generation,
  permission failure, and locked/result-prevalidation behavior where applicable.

Compatibility spellings do not become ISA merely because libc uses them.
`poll`, `select`, `epoll`, `kqueue`, `pipe`, `eventfd`, `timerfd`, POSIX
signals, and socket APIs are libc/personality names over native
`WAITABLE_PROBE`, `AWAIT_EX`, `OBJECT_CTL`, event queues, gates, and endpoint
objects. If a spelling such as `POLL_FD_DYN` survives in a bootstrap assembler
or emulator helper, it must be documented as a compatibility alias and either
lowered to the native operation or removed before ISA freeze.

The ISA does not add a general syscall instruction. Linux/POSIX syscall-number
compatibility, where needed, is a runtime/personality ABI over native calls,
control FDRs, gate calls, unsupported-opcode upcalls, and ordinary functions.

Compiler-visible system operations use one operand/result discipline:

| Operation family | Canonical source form | Success result | Failure result |
| --- | --- | --- | --- |
| `OBJECT_CTL`, `DOMAIN_CTL` | `op r_result, r_argblock` | zero, id, count, state, or snapshot size | negative architectural error |
| `CAP_DUP`, `CAP_SEND`, `CAP_RECV`, `CAP_REVOKE` | `op r_result, r_argblock` | token, count, operation id, or zero | negative architectural error |
| `GET_PCR` | `GET_PCR r_result, pcr_name` | PCR value | negative architectural error for malformed/unsupported selector |
| `SET_PCR` | `SET_PCR r_result, pcr_name, r_src` | zero | negative architectural error |
| `WAITABLE_PROBE`, `AWAIT_EX` | `op r_result, r_waitable, r_argblock` | ready mask/count/result bits or zero | negative architectural error |
| `GATE_CALL`, `GATE_RETURN` | fixed register form | value, operation id, token, or zero | negative architectural error |

All of these operations prevalidate the encoded result register before authority,
domain, VMA, or object mutation. A locked/unwritable result register fails before
side effects. Compatibility aliases may default the result to `r1` in source
assembly, but the binary form must still encode a result destination.

Mandatory object profiles have frozen state-machine shapes:

*   **`counter`:** `INVALID -> READY -> WAITING/READY -> REVOKING -> DESTROYED`.
    A counter has a value, generation, wait predicates, overflow mode, and
    event mask. Increment/decrement/set/test-and-wait transitions are atomic at
    the counter object. Waiters cannot be lost because predicate check,
    waiter-install, and wake publication are one owner-engine transaction.
*   **`queue`:** `INVALID -> OPEN -> READ_CLOSED/WRITE_CLOSED -> DRAINING ->
    DESTROYED`, with `REVOKING` and `POISONED` side states. A queue has bounded
    capacity, record mode, head/tail generations, readable/writable readiness,
    and explicit full behavior: park, `EAGAIN`, `EOVERFLOW`, coalesce, or
    drop-with-count where the profile permits drops.
*   **`event/completion`:** an event queue is a queue profile with source slots,
    source generations, trigger mode, ready bits, and overflow/rescan records.
    Add-source is atomic check-and-arm; `AWAIT` checks readiness before parking.
*   **`timer`:** `DISARMED -> ARMED -> EXPIRED -> DISARMED/ARMED`, with
    cancellation and revocation states. Timer expiry publishes a waitable event;
    periodic rearm is atomic with expiry publication.
*   **`memory_object`:** `UNMAPPED -> MAPPABLE -> MAPPED/PINNED -> REVOKING ->
    DESTROYED`, with `POISONED` for integrity failure. Mapping, pinning, dirty
    enumeration, and protection changes carry generation checks and VMA policy.
*   **`call_gate`:** `CLOSED -> READY -> ENTERED/QUEUED -> RETURNING -> READY`
    plus `REVOKING/DESTROYED`. Synchronous calls park the caller continuation;
    async and handoff calls publish completion through explicit event/counter
    objects. Entry and return have separate commit points.
*   **`dma_buffer` / `dma_completion`:** DMA buffers are memory-object profiles
    with pin direction, IOMMU/device scope, cacheability, generation, and
    quiesce state. Completion objects are event/counter profiles that publish
    success, partial, canceled, revoked, or fault status after DMA visibility is
    correct.

All object profiles share the same rules: state is generation-checked,
authority comes only from FDR capabilities, all waits are attached before the
condition can be missed, overflow is explicit, revocation wakes or cancels
waiters, and poisoned objects cannot be recycled as fresh authority without a
supervisor/PID 1 acknowledgement path.

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
requests, `GATE_CALL`/call-gate profiles, and event queues. Hardware packages requests as bounded
records with caller identity, object generation, lineage epoch, rights,
Resource Domain id/generation, copied bytes or pinned-buffer descriptors, and
explicit capability arguments. Services never receive ambient raw pointers, raw
interrupt vectors, raw DMA authority, physical addresses, or hidden privilege.

The v1 service request header is architectural:

*   version, size, profile class, profile id, op id, flags.
*   request id, continuation id, cancellation token, and deadline/timeout
    policy.
*   caller PID/TID, Resource Domain id/generation, credential snapshot, and
    nonblocking/wait policy.
*   target object class/id/generation, lineage root/epoch, requested rights,
    and expected commit class.
*   copied input bounds, pinned-buffer descriptors, scalar fields, explicit
    capability-argument slots, and expected returned-capability slots.

The v1 service reply header is also architectural:

*   matching request id, continuation id, service id/generation, status code,
    flags, and output length.
*   copied output bounds, event/pressure/fault metadata where applicable, and
    returned-capability proposals in declared slots.
*   optional committed-byte count or profile-specific progress marker for
    partial data operations.

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

*   **`GATE_CALL r_result, r_gate_fd, r_arg0, r_arg1`** / **`GATE_RETURN r_result, r_value0, r_value1`**
    *   *Action:* Native bounded activation through a gate capability and return through a trusted continuation token. `CALL_CAP`/`RET_CAP` remain source-level names for explicit call-gate profiles. The same Gate/Continuation Engine is also used by delivery profiles for faults, cancellation, supervisor upcalls, debug traps, timers, and POSIX signals. Hot activations use bounded register arguments and pre-provisioned target state; cold domain/container/VM creation remains a `DOMAIN_CTL` operation.
*   **`ERRNO_GET r_dest`** / **`ERRNO_SET r_src`**
    *   *Action:* Reads or writes the thread-local compatibility error register. Native instructions report failure as a negative architectural error in the encoded result register; libc/personality code uses `ERRNO_SET` when translating that result into a POSIX `-1`/`errno` API boundary.
*   **Child Waits**
    *   *Action:* Child completion is a waitable event. Source-level `waitpid` lowers to `AWAIT` on a child/process waitable and then `GET_META` for status where needed.

### 3.2 Canonical Error and Fault Codes

Fallible native instructions return success or a nonnegative value in the
encoded destination register. On ordinary failure they write the negative
architectural error code to that destination. Compatibility wrappers may then
write thread-local `ERRNO` and return `-1` where POSIX requires it. Hardware
engines use one canonical error namespace:

*   `EINVAL`: malformed record, bad length/alignment, invalid state transition,
    unsupported reserved bits, or invalid scalar shape.
*   `ENOTSUP`: well-formed but unsupported opcode profile, object profile, op,
    feature, or version.
*   `EBADF`: invalid FDR, wrong FDR class, closed descriptor, or descriptor
    without the required operation class.
*   `EPERM`: capability, delegation, security-profile, sealed-capability, or
    Resource Domain policy denial.
*   `EACCES`: credential or object permission denial after a valid capability
    was supplied.
*   `EFAULT`: invalid user buffer, failed copy/pin, unmapped memory, or
    protection failure during pre-commit argument access.
*   `EAGAIN`: nonblocking operation would block or bounded retry is required.
*   `EINTR`: interruptible operation canceled by handled signal before commit.
*   `ECANCELED`: operation canceled by teardown, explicit cancellation,
    service death, or pre-commit revoke.
*   `EREVOKED`: generation, lineage, or revocation epoch mismatch.
*   `EOVERFLOW`: well-formed request exceeds a hardware/profile limit or a
    bounded queue/ring reports overflow.
*   `EQUOTA`: Resource Domain limit, budget, or accounting admission failure.
*   `EBUSY`: object is frozen, quiescing, pinned, or in a conflicting committed
    operation.
*   `EPIPE`: peer/service endpoint is closed after the request was otherwise
    valid.
*   `ETIMEDOUT`: timeout expired before readiness or completion.
*   `EINPROGRESS`: operation was accepted as a bounded asynchronous transaction;
    completion is reported through the encoded waitable, event queue, counter,
    or result token.
*   `EIO`: service/device/storage operation failed after authority checks.
*   `EPOISONED`: object, page, descriptor, queue, or metadata is poisoned by an
    integrity/RAS failure.

Synchronous architectural faults use the signal/fault path rather than only
`ERRNO`: illegal opcode or disabled feature `SIGILL`; arithmetic trap `SIGFPE`;
protection or unmapped memory `SIGSEGV`; alignment, device mapping, or physical
translation fault `SIGBUS`; breakpoint/debug trap `SIGTRAP`. Hardware also
emits structured fault records for RAS, DMA/IOMMU, watchdog, boot measurement,
metadata integrity, classifier, storage barrier, and machine-fatal faults.
Compatibility personalities may translate these codes into Linux/BSD names, but
native LNP64 engines use this canonical set.

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

## 5. Gate Delivery and POSIX Signals
The native asynchronous-control primitive is **bounded activation through a
gate plus return through a trusted continuation**. Explicit cross-thread or
cross-domain calls, synchronous hardware faults, cancellation, timer delivery,
debug traps, supervisor upcalls, and POSIX signal handlers all use the same
Gate/Continuation Engine.

Native gate delivery records are typed:

*   class: explicit_call, fault, cancel, timer, child, debug, resource, service,
    domain, supervisor, or software.
*   code: profile-specific reason such as arithmetic fault, memory protection,
    alarm timer, child exit, breakpoint, quota, revoke, or opcode upcall.
*   target scope: thread, process, Resource Domain, event queue, or gate.
*   source domain/PID/TID where permitted.
*   object id, operation id, fault PC/address, compact payload words, flags,
    generation, and optional continuation token.

Gate actions are also typed:

*   enqueue a record to an event queue.
*   wake an `AWAIT`.
*   enter a registered gate with bounded register arguments.
*   terminate a thread, process, or domain.
*   coalesce, ignore, or defer according to profile policy.

The native instructions are `GATE_CALL`, `GATE_DELIVER`, `GATE_CTL`,
`GATE_MASK_SET`, and `GATE_RETURN`. Source names such as `CALL_CAP`, `RET_CAP`,
`SIGACTION`, `SIGMASK_SET`, `KILL`, `ALARM`, and `SIGRET` are profiles or aliases
over those operations.

The POSIX signal profile freezes only the bounded pieces needed by libc and
Unix personalities: dispositions, per-thread masks, pending state,
fault-to-signal mapping, checked `kill`/`raise`, alarm delivery, fixed handler
entry, and trusted return. `SIGRET` is a POSIX spelling of `GATE_RETURN`; it
restores only from a Gate/Continuation Engine-owned saved-context
token/generation, never from authority stored in user memory.

The frozen POSIX profile intentionally excludes historical signal quirks:
OS-specific restart behavior, arbitrary signal-stack ABI variants, full POSIX
realtime queueing semantics, signal-based application IPC as a preferred
primitive, and Linux/BSD-specific delivery corner cases. Those remain
libc/personality policy over native gates, event queues, and compatibility
metadata.

Fault delivery maps native fault classes to POSIX signal numbers only inside the
POSIX profile: arithmetic faults to `SIGFPE`, decode faults to `SIGILL`, memory
protection faults to `SIGSEGV`, alignment/device/bus faults to `SIGBUS`, and
debug traps to `SIGTRAP`. Native code can consume the typed fault delivery
record directly.

Interruptible `AWAIT`, futex, timer, `PULL`/`PUSH`, pending page-fill, and queued
gate waits return `EINTR` or a typed interrupted status before profile handler
entry. Operations past their commit point use their documented roll-forward or
cancel policy. Linux/BSD `SA_RESTART` behavior is libc/personality policy.

---
To make the **LNP64** a fully functional processor, the capability/event/domain instructions must coexist with a conventional general-purpose compute architecture. Since namespace dispatch, capability, VMA, event, and runqueue logic consume meaningful FPGA resources, the general compute side should remain a lean in-order RISC architecture.

Here is how the general-purpose compute integrates with the LNP64 resource fabric.

---

## 6. Memory Access (Load/Store Architecture)
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

## 6.1 Memory Consistency Model

LNP64 v1 uses a coherent, TSO-like model for `normal_cached` memory:

*   Each hardware thread observes its own loads and stores in program order.
*   Stores become visible to other cores in program order.
*   Loads may read the issuing core's older buffered stores to the same
    address.
*   Aligned naturally sized 8-, 16-, 32-, and 64-bit loads/stores are single
    architectural memory operations.
*   Instruction fetch observes data-side code changes only after the required
    `MPROTECT`/`ISYNC` or exec/remap invalidation event.

Synchronization rules:

*   `LOCK_CMPXCHG` is a single-copy atomic read-modify-write and is
    sequentially consistent in v1.
*   `AMO.SWAP`, `AMO.ADD`, `AMO.AND`, `AMO.OR`, and `AMO.XOR` are 64-bit atomic
    read-modify-write operations. The destination receives the old memory value;
    memory receives the transformed value. Baseline AMOs are sequentially
    consistent in v1.
*   `FENCE.ACQ`, `FENCE.REL`, `FENCE.ACQ_REL`, and `FENCE.SC` are architectural
    profiles of `FENCE`. FPGA v1 may implement them identically, but the
    ordering meanings are fixed: acquire orders later operations after the
    fence, release orders earlier operations before the fence, acq_rel does
    both, and seq_cst also participates in one total order for locked atomics
    and seq_cst fences.
*   Futex `AWAIT` performs an acquire-style value check before parking; futex
    `WAKE` performs release-style ordering before making waiters runnable.
*   `GATE_CALL` commits argument/register state before target entry;
    `GATE_RETURN` commits return values before waking the caller continuation.
*   Forced gate delivery records a precise architectural boundary;
    `GATE_RETURN` resumes after handler memory effects are visible under the
    ordinary memory model.

DMA and device rules:

*   Hardware engine completions are ordered after their DMA writes, metadata
    updates, and result-register writes.
*   Coherent DMA participates in the L2-coherent fabric before completion is
    signaled. A non-coherent implementation must expose explicit cache
    maintenance and must not advertise coherent DMA.
*   VMA permission changes, unmaps, revocation, and page installs complete
    required TLB/cache/I-cache invalidations before affected threads resume or
    backing authority is reused.
*   `device_ordered` mappings are uncached and strongly ordered for CPU MMIO
    loads/stores. `write_combining` mappings may combine writes; software must
    execute `FENCE` before relying on those writes being visible to device
    doorbells, DMA engines, or completion observers.

The v1 model is intentionally stronger than many relaxed commercial CPUs
because libc, language runtimes, paravirtual Unix personalities, and formal
proofs should not need architecture-specific weak-memory folklore.

## 7. Arithmetic and Logic Unit (ALU)
Standard 64-bit integer operations. Because threads are managed in hardware, the ALU pipeline reads and writes architectural state through hardware thread contexts.

The scalar compute baseline is intentionally conventional. LNP64 should be a
boring, complete compiler target on the compute side; the distinctive ISA
surface is the capability/resource side. C, libc, runtimes, packet processing,
allocators, and personality ports must not need long software sequences for
ordinary integer operations.

*   **`ADD r_dest, r_src1, r_src2`** / **`SUB r_dest, r_src1, r_src2`**
    *   *Action:* Standard integer addition/subtraction.
*   **`ADDI r_dest, r_src, imm`** / **`ANDI`** / **`ORI`** / **`XORI`**
    *   *Action:* Immediate ALU forms with sign-extended immediates. These are
        baseline instructions, not assembler conveniences, so compiler output
        does not expand every small constant into `LI` plus a register ALU op.
*   **`MUL r_dest, r_src1, r_src2`** / **`DIV r_dest, r_src1, r_src2`**
    *   *Action:* Signed low multiply and signed quotient. Division by zero
        creates a native fault delivery; the POSIX profile maps it to `SIGFPE`.
*   **`UDIV`** / **`SREM`** / **`UREM`**
    *   *Action:* Unsigned quotient, signed remainder, and unsigned remainder.
        These are required by C and are baseline scalar instructions.
*   **`MULH`** / **`MULHU`** / **`MULHSU`**
    *   *Action:* High-half multiply for signed*signed, unsigned*unsigned, and
        signed*unsigned operands. These support wide arithmetic, division
        lowering, hashing, allocators, and runtimes.
*   **`AND`, `OR`, `XOR`, `NOT`**
    *   *Action:* Standard bitwise operations.
*   **`LSL`, `LSR`, `ASR`** / **`LSLI`, `LSRI`, `ASRI`**
    *   *Action:* Register and immediate logical/arithmetic shifts. Shift counts
        are masked to the low six bits for 64-bit operations.
*   **`SEXT.B`** / **`SEXT.H`** / **`SEXT.W`** and **`ZEXT.B`** / **`ZEXT.H`** / **`ZEXT.W`**
    *   *Action:* Sign-extend or zero-extend 8-, 16-, and 32-bit values to
        64-bit GPRs. These are baseline ABI cleanup and comparison operations.
*   **`CLZ`** / **`CTZ`** / **`POPCNT`**
    *   *Action:* Count leading zeroes, trailing zeroes, and one bits. Zero
        input returns 64 for `CLZ`/`CTZ`.
*   **`ROL`** / **`ROR`**
    *   *Action:* 64-bit rotate left/right with the count masked to six bits.
*   **`BSWAP16`** / **`BSWAP32`** / **`BSWAP64`**
    *   *Action:* Byte-swap the low 16, low 32, or full 64 bits.
*   **`CSEL.<cond> r_dest, r_true, r_false`**
    *   *Action:* Conditional select from the current condition flags. This
        avoids branchy lowering for ternaries, min/max, clamps, and simple
        bounds checks.

## 8. Control Flow (Branching & Execution)
Since there is no Ring 0 / Ring 3 boundary, native control flow is about
executing user logic and jumping to functions. Compatibility personalities may
receive explicit supervisor upcalls, but native LNP64 resource operations are not
implemented as syscall traps.

*   **`JMP r_target`** / **`JMP immediate`**
    *   *Action:* Unconditional jump to a virtual address.
*   **`CALL r_target`**
    *   *Action:* Writes `PC + 8` to the thread-local Link Register (`LR`) and jumps to `r_target`.
*   **`LR_GET r_dst`** / **`LR_SET r_src`**
    *   *Action:* Copies the thread-local Link Register to or from a GPR so non-leaf functions can spill and restore return state using normal stack-frame policy.
*   **`RET`**
    *   *Action:* Sets `PC = LR`. Software stack frames and spilling the link register are psABI conventions.
*   **`CMP r_src1, r_src2`**
    *   *Action:* Compares two registers and sets the hardware condition flags (Zero, Carry, Negative, Overflow).
*   **`BEQ`, `BNE`, `BLT`, `BGT`**
    *   *Action:* Branch if Equal, Not Equal, Less Than, Greater Than (evaluates condition flags).
*   **`AUIPC` address materialization contract**
    *   *Action:* The v1 software ABI has exactly one compiler-visible
        PC-relative symbol materialization scheme. Direct code/data addresses use
        `AUIPC rd, %pcrel_hi(symbol)` followed by
        `ADDI rd, rd, %pcrel_lo(symbol)`. Address slots, large constants, GOT-like
        entries, and local-exec TLS offsets use `AUIPC tmp, %pcrel_hi(slot)`
        followed by `LD rd, tmp, %pcrel_lo(slot)`. Assemblers may accept `LA` as a
        human convenience only if it expands to this exact sequence before
        object emission; backend, lld, loader, and conformance tests must not
        depend on any alternate pseudo-contract.

## 9. Hybrid Resource-Compute Instructions
Because "everything is a capability object" is the native hardware reality, we need instructions to move data between the general compute realm (GPRs) and the resource realm (FDRs and PCRs).

*   **`MOV r_dest, r_src`**
    *   *Action:* Move data between general purpose registers.
*   **`DUP r_result, r_dst_or_flags, r_src`**
    *   *Action:* Duplicates or moves an FDR capability, including `dup`, `dup2`, and narrowed-rights forms where permitted by the source capability.
*   **`GET_PCR r_result, pcr_name`**
    *   *Action:* Reads a Process Control Register (like `PID`, `CRED_PROFILE`,
        `POSIX_UID`, or `REALTIME_SEC`) into a general-purpose register for
        user-space logic.
        (e.g., `GET_PCR r1, PID`).
*   **`SET_PCR r_result, pcr_name, r_src`**
    *   *Action:* Writes to a permitted Process Control Register and reports
        status in `r_result`. Success returns `0`; failure returns a negative
        architectural error and performs no PCR update. Writable v1 selectors
        are `TP`/`TLS_BASE`, `SIGMASK`, and credential-profile `UID`/`GID` only
        when the active credential profile and Resource Domain policy authorize
        the mutation. `PID`, `PPID`, `TID`, `CRED_PROFILE`, `CRED_HANDLE`,
        `SIGPENDING`, `REALTIME_SEC`, and `REALTIME_NSEC` are read-only:
        `SET_PCR` on them must fail uniformly with `-EPERM`, not trap, not
        silently ignore, and not partially mutate state. Reserved or malformed
        PCR selector encodings fail with `-EINVAL`; unsupported optional
        selectors fail with `-ENOTSUP`.

Stable v1 PCR selector ids:

| Id | Canonical name | Source aliases | Write class |
| --- | --- | --- | --- |
| 0 | `PID` | none | read-only |
| 1 | `PPID` | none | read-only |
| 2 | `TID` | none | read-only |
| 3 | `TP` | `TLS_BASE` | writable thread-local |
| 4 | `UID` | `POSIX_UID` | credential-profile controlled |
| 5 | `GID` | `POSIX_GID` | credential-profile controlled |
| 6 | `SIGMASK` | none | writable thread-local |
| 7 | `SIGPENDING` | none | read-only |
| 8 | `REALTIME_SEC` | none | read-only |
| 9 | `REALTIME_NSEC` | none | read-only |
| 10 | `CRED_PROFILE` | none | read-only, optional until credential profiles are implemented |
| 11 | `CRED_HANDLE` | none | read-only, optional until credential profiles are implemented |
*   **`ENV_GET r_dest, r_key, r_index_or_buf, r_len_or_flags`**
    *   *Action:* Reads read-only process and machine metadata for libc/runtime startup: ISA version, implementation profile, page size, cache-line size, DMA alignment, hardware feature bits, supported opcode groups, object profiles, domain/security features, architectural limits, bounded topology records, startup metadata pointer, personality flags, and timebase frequency. POSIX `argc`, `argv`, `envp`, and auxv layout are libc/personality ABI data behind that pointer, not hardware-interpreted state. This is not a replacement for immediates; constants still use normal instruction encodings or literal loads.
*   **`RANDOM r_dest, r_len_or_flags`**
    *   *Action:* Returns hardware entropy for ASLR, stack canaries, randomized capability ids, allocator hardening, and libc/runtime seeding. Small scalar requests return in `r_dest`; larger requests use a versioned argument-block variant that copies entropy into a caller buffer.

---
**Summary of the Compute Pipeline:**
The ALU and Control Flow instructions avoid privilege-transition overhead for native resource operations. If an ALU instruction calculates a buffer address and the next instruction is `PUSH`, decode can enqueue a Stream/Object or DMA Engine command directly rather than entering a software syscall path.
The core ISA also needs synchronization, device-driver boundaries, floating-point/vector compute, and a boot path to be a practical v1 target.

To make the LNP64 bootable and useful, v1 includes **Synchronization, Device Drivers, Floating Point, and Bootstrapping**.

The following sections sketch those remaining pieces of the LNP64 architecture:

## 10. Synchronization
Because the CPU manages threads in a hardware runqueue, traditional software spinlocks would waste issue slots under contention. Hardware-level concurrency controls let a thread park on a waitable condition and let the scheduler run another ready thread.

*   **`LOCK.CMPXCHG r_dest, [r_addr], r_expected, r_new`**
    *   *Action:* Atomic Compare-and-Swap. The standard building block for mutexes. V1 locked atomics are single-copy atomic and sequentially consistent unless a future encoding explicitly requests weaker acquire/release semantics.
*   **`AWAIT futex([r_addr], r_expected_val)`**
    *   *Action:* The hardware equivalent of a futex wait. If the value at `[r_addr]` equals `r_expected_val`, after an acquire-style check, the CPU removes the current thread from the runqueue and parks it in a hardware wait-state attached to that memory address.
*   **`WAKE futex([r_addr], r_num_threads)`**
    *   *Action:* The memory controller performs release-style wake ordering, checks if any threads are parked on `[r_addr]`, and pushes up to `r_num_threads` back onto the active runqueue.
*   **`THREAD_JOIN r_result, r_tid, r_retval_ptr`**
    *   *Action:* Parks the caller until the target same-process hardware thread exits. On completion, copies the target thread's exit value to `r_retval_ptr` when nonzero and returns `0`; returns a POSIX-style error code for invalid or self-join cases.

## 11. Device and Network Capabilities
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
*   **`LOAD_UCODE_RESERVED r_buf_ptr, r_len`**
    *   *Action:* Reserved compatibility spelling for future driver/service
        acceleration; source-level `LOAD_UCODE` must lower to a reserved/stub
        result in v1. It must not install arbitrary microcode. The v1 direction
        is to load verified bounded servicelets through `OBJECT_CTL` into a
        `servicelet_program` object, then attach that object to a classifier,
        queue, gate, domain, telemetry stream, or device profile under
        capability control.

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

Bounded servicelets are first-class network attachment points. A
`net_interface`, `packet_queue`, `listener`, `datagram_endpoint`, or
`stream_endpoint` may reference a `classifier_table` and optional
`servicelet_program` for RX classification, TX admission, queue selection,
priority/mark assignment, checksum/offload policy selection, accept filtering,
endpoint handoff, telemetry redaction, and `needs_software` fallback. The
attachment is installed through `OBJECT_CTL` only when the caller holds the
target network object capability, the servicelet capability, and every
destination queue/gate capability named by possible actions.

The silicon/software split is deliberate:

*   **Silicon owns:** safe packet movement, packet DMA, coherent visibility, IOMMU enforcement, page-granular BAR mappings, `irq_event` delivery, generic queues/counters/events, basic MAC filtering/steering where cheap, servicelet/classifier execution where enabled, simple checksums/classification where cheap, timestamps where cheap, per-domain quotas, counters, trace, fault events, timer/counter objects useful to transport services, and zero-copy buffer handoff.
*   **Software domains own:** PCIe enumeration and quirks, Ethernet NIC drivers, Wi-Fi firmware/device protocols, Wi-Fi scan/association/authentication/roaming/regulatory policy, ARP/NDP, IP, TCP, UDP, QUIC policy, routing, firewall/NAT policy, TLS, DNS, socket compatibility, service discovery, congestion control, retransmission, pacing, loss recovery, keepalive policy, and socket-option semantics.

The typed endpoint boundary is stable:

*   `packet_queue` preserves packet record boundaries and carries packet envelopes, payload references, checksum/timestamp/offload metadata, classifier/servicelet marks, and queue readiness.
*   `datagram_endpoint` preserves datagram boundaries but does not imply hardware UDP; loss, truncation, peer metadata, and reliability are endpoint-profile/service policy.
*   `stream_endpoint` exposes ordered bytes, backpressure, close/reset/error readiness, and no packet boundaries; it may be backed by software TCP, local IPC, QUIC service, paravirtual transport, or future acceleration.
*   `listener` is an accept queue that returns endpoint capabilities whose rights and namespace scope derive from the listener and service policy.
*   `GET_META`, `SET_META`, and `OBJECT_CTL` expose bind/connect/listen/shutdown/nonblocking/buffer/event/socket-option profiles, classifier/servicelet attachment, and network queue steering as bounded typed records. Unsupported options fail closed rather than becoming raw `ioctl` blobs.

A future TCP accelerator may be added only as an optional transport service
profile behind the same `stream_endpoint` capability shape. Applications, libc,
and POSIX socket compatibility must not depend on whether a stream endpoint is
implemented by software TCP, local IPC, QUIC service, paravirtual networking, or
a hardware assist block.

For PCIe Ethernet, the Bus Master requests `pci_function`, `pcie_bar`, `dma_buffer`, and `irq_event` capabilities for a NIC driver domain, and hardware derives/installs them from the PCIe root/function authority. The driver maps BARs with `MMAP`, allocates descriptor rings and packet buffers through `dma_buffer` capabilities, waits on `irq_event` records for MSI/MSI-X completion, and publishes `net_interface` plus packet queue capabilities to a network service domain. The driver or network service may attach classifier tables and verified servicelets to RX/TX queues for steering, marking, admission, and telemetry before packets reach a software stack. That service domain exposes `stream_endpoint`, `datagram_endpoint`, and `listener` FDRs to applications and libc.

For Wi-Fi, silicon remains the same PCIe/DMA/event substrate. Wi-Fi-specific firmware loading, scan, association, WPA/WPA2/WPA3, roaming, regulatory behavior, power management, and link policy belong in a Wi-Fi driver/service domain. Once associated, the service publishes a normal `net_interface` capability to the rest of the system.

POSIX sockets lower cleanly onto this model: `socket()` creates an endpoint under a `net_namespace`, `bind`/`connect`/`listen` become typed metadata/control operations, `accept` pulls a connection capability from a listener, `send`/`recv` become `PUSH`/`PULL`, `poll`/`epoll` bind endpoint readiness into event queues, `getsockopt`/`setsockopt` become typed metadata records, and descriptor passing maps to `CAP_SEND`.

### 11.2 Bounded Record Classification, Servicelets, and Queue Steering

The networking classifier is useful beyond networking, so it is specified as a
generic `classifier_table` object profile created through `OBJECT_CTL`, with
packet parsing as only one record profile.

The engine accepts a record envelope plus a capability-scoped rule table and can:

*   extract a bounded set of fixed fields from known envelope profiles.
*   compare exact values, masks, prefixes, ranges, and small enumerations.
*   compute simple hashes for queue steering.
*   stamp metadata fields such as class id, flow hash, timestamp, priority, or mark bits.
*   increment counters.
*   route, drop, or mark records into capability-scoped queues from the table's
    authorized destination set.

The fixed field vocabulary is intentionally small. Generic records expose
`profile`, `domain_id`, and `inline0..inline2` fields; `service_id` is a
compatibility alias for `inline0` for IPC-style records. Packet records add
packet-only fields such as `dst_port`, `src_ipv4`, `dst_ipv4`, and `hash`.

Useful profiles include:

*   **Packet profile:** shallow L2/L3/L4 extraction for simple Ethernet, VLAN, IPv4/IPv6, TCP/UDP/SCTP/ICMP headers; checksum status; flow hash; queue steering.
*   **IPC/message profile:** route typed messages or call-gate completions to worker queues by service id, method id, tenant/domain id, priority, or hash.
*   **Storage/DMA completion profile:** route completions and faults by object id, operation id, domain id, priority, or error class.
*   **Event/trace profile:** classify structured fault, trace, scheduler, and RAS records for observability without waking a general supervisor for every record.
*   **Runtime profile:** steer task, actor, or executor records to per-core/per-domain queues.

The fixed table form is the default fast path. For policies that need more than
a table but still must remain realtime-safe, LNP64 also defines a
`servicelet_program` object profile. A servicelet is a verified, bounded subset
of the ordinary LNP64 ISA. It may execute on a small dedicated servicelet
micro-engine, because this is one place where a tiny programmable lane can be
physically sensible. The micro-engine is still not arbitrary microcode: it runs
only verifier-approved servicelet programs, has no ambient authority, and
publishes a fixed WCET/action contract. Servicelets use normal
integer/logic/compare/branch instructions, bounded literal loads, fixed-field
envelope loads, and a small set of object-profile action outputs.
It cannot block, allocate, perform normal memory loads/stores, issue `PULL` or
`PUSH`, call gates, touch FDR tables, walk VMAs, access DDR arbitrarily, loop
without a statically proven bound, or mint capabilities.

Servicelets are installed and attached through `OBJECT_CTL` after verifier
approval. The verifier checks instruction subset, maximum instruction count,
branch bounds, record-field bounds, authority scope, action set, stack/register
use, and a published worst-case cycle budget. A servicelet returns a bounded
action record such as pass, drop, mark, count, steer-to-authorized-queue,
select-gate, redact, sample, or `needs_software`. It may read only the provided
record envelope, constant table, verifier-approved immediate data, and selected
metadata fields from the object to which it is attached.

The servicelet verifier envelope is deliberately small: version, program length,
instruction count, maximum cycles, bounded-loop limit, ISA-subset bitmap,
attachment class, record profile, allowed record fields, immutable constant
table digest, action bitmap, authorized destination set, scratch-register
count, owner/source generations, and optional verifier-certificate hash. The
action record is fixed-width data and cannot carry or mint capabilities.

Useful servicelet attachment points include packet and generic record
classification, queue steering, socket accept/filter policy, gate admission,
capability-narrowing policy, Resource Domain accounting/classification, audit
filtering/redaction, storage/DMA completion routing, IRQ/event routing,
observability sampling, and seccomp-like personality filtering. Attachment
authority is capability-scoped: holding a servicelet object is not enough; the
caller must also hold the target object/control capability and any destination
queue or gate capabilities named by possible actions.

Servicelets are a bounded prelude/postlude/filter/steering layer for services,
not a substrate for implementing arbitrary services inside the hardware. A
filesystem servicelet may reject malformed selectors or steer an `OPEN_AT`
request, but it does not walk directories, follow symlinks, resolve mounts, or
perform storage recovery. A network servicelet may classify packets or choose a
flow queue, but it does not implement TCP congestion control, retransmission,
reassembly, TLS, routing, NAT, or Wi-Fi association. A loader servicelet may
recognize or reject a bounded header shape, but it does not perform ELF parsing,
relocation, dynamic linking, interpreter policy, or credential transitions.
Those general cases return `needs_software` or are dispatched to software
service domains.

This is not arbitrary eBPF-scale programmable packet processing. V1 classifier
rules and servicelets are bounded, versioned, capability-owned, proof-friendly,
and either table-driven or verified against a fixed LNP64-ISA subset. If a
record is malformed, too deep, encrypted, fragmented, extension-header-heavy,
or unknown, the classifier/servicelet marks it `partial` or `needs_software`
and still delivers it safely to a software-owned queue. Protocol state,
connection tracking, routing policy, firewall languages, TLS, Wi-Fi management,
and application semantics remain in software domains.

## 12. Floating Point and Vector Math
Integer scalar compute is the v1 portable baseline. Floating point and vector
compute are explicit extension profiles until they are specified with the same
precision as the integer ISA. A half-defined vector ISA is worse than no vector
baseline.

*   **`FADD`, `FSUB`, `FMUL`, `FDIV`**
    *   *Action:* Optional IEEE 754 binary64 scalar floating-point arithmetic
        operating on dedicated FPU registers (`f0` - `f31`). Implementations
        that do not advertise the FP profile must raise the standard disabled
        opcode event.
*   **`VADD.32 v_dest, v_src1, v_src2`**
    *   *Action:* Optional vector-profile example over vector registers
        (`v0` - `v15`). The full vector profile, element widths, masks,
        predication, memory operations, and ABI are deferred to an extension
        document.

## 13. Bootstrapping (Hardware PID 1)
How does this machine actually turn on without a conventional bootloader or
kernel? The reset controller creates the initial operating envelope and commits
a bounded manifest-provided exec plan for PID 1. It is not a general executable
loader.

Upon receiving power, the LNP64 executes a hardwired reset sequence:
1.  Initializes the hardware VMA tree, scheduler fabric, root Resource Domain, default weighted-fair scheduler profile, telemetry/fault routes, capability roots, and runqueue.
2.  Creates the initial hardware process/thread context (PID 1, TID 1) inside a PID 1 Resource Domain with valid CPU, memory, FDR, event, telemetry, and device budgets. If the boot profile enables POSIX compatibility, PID 1 starts with a POSIX credential profile equivalent to UID 0; the authority still comes from explicit boot FDRs and domain policy.
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

## 14. Paravirtual Unix Guest Profile
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
*   `OPEN_AT` is namespace selector dispatch, not a path walker. POSIX paths are the `posix_path` selector profile; native code may use object-id, content-hash, service-key, package-id, route-tuple, or tenant-local selector profiles under the same capability checks.
*   fork-like behavior is a `CLONE` profile, not the conceptual center of the machine. Native code can prefer spawn, call gates, domains, explicit shared memory, and event queues.
*   POSIX signals remain available as a gate-delivery profile, but native code can use typed gate deliveries, structured events, and cancellation objects instead.
*   POSIX UID/GID is a credential profile for imported software; native authority is capability possession, credential-profile tokens where explicitly requested, and Resource Domain policy.

The targeted compatibility approaches are:

*   **Linux as a paravirtual personality:** A Linux kernel port runs as a supervisor Resource Domain over a delegated LNP64 process subtree. Linux tasks, files, memory mappings, signals, futexes, cgroups, containers, nested guests, and devices are projected onto native hardware primitives.
*   **Linux syscall compatibility runtime:** A loader/libc/runtime maps Linux syscall ABI calls onto native LNP64 instructions without booting a full Linux kernel. This is the shortest path to running many cloud-oriented programs.
*   **NetBSD rump-kernel style:** Selected NetBSD filesystem, networking, or device stacks run as LNP64 service processes. They receive block, network, PCIe, or delegated namespace FDRs and expose services back through native FDRs.

A full traditional Linux/NetBSD port that owns page tables, context switching, interrupts, and raw devices is not the v1 target.

The compatibility interface is deliberately narrow. A personality observes and controls native objects through fixed FDR surfaces: lifecycle events for `CLONE`/`EXEC`/`EXIT`, VMA and page-fault events, FDR/capability transfer, namespace dispatch, block-image/storage objects, gate-delivery/fault records, event queues, futex/timer waitables, network endpoint/packet queue capabilities, PCIe BAR/DMA/IRQ-event capabilities, and domain control upcalls. It may translate these into Linux or BSD concepts, but it cannot own page tables, raw interrupts, the scheduler, raw DMA, or capability minting.

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

A capability-marked domain can also act as a supervisor domain and receive upcalls for selected events: unsupported opcodes, delegated namespace lookups, permission decisions, child exit, gate delivery, fd readiness, timer expiry, futex events, block-image completion, resource pressure, limit violation, and process lifecycle changes.

Upcalls are delivered through a normal FDR with object class `control`. The supervisor pulls event records with `PULL` and pushes policy commands with `PUSH`. This keeps the design inside the FDR/capability model instead of reintroducing a syscall path.

The precise claim is: native LNP64 resource operations are hardware commands, not
software traps. POSIX and Linux compatibility personalities may still receive explicit hardware
upcalls for virtualization policy, unsupported opcodes, delegated namespaces,
and Linux syscall ABI emulation.

For this to be practical, LNP64 needs a stable psABI: calling convention, process entry layout, TLS, POSIX signal/gate-delivery frame layout, errno convention, time/timer FDRs, and event-queue FDRs that can aggregate fd readiness, timers, child exit, gate-delivery events, futex events, and supervisor upcalls.

For storage, a guest kernel can treat a large block-image or storage-service FDR as a paravirtual block device. It uses explicit-offset `PULL` and `PUSH`, then mounts ext4, FFS, or another guest filesystem inside that image. LNP64 provides the outer capability, DMA, eventing, and durability hooks; the guest kernel or filesystem service provides the filesystem semantics.

For physical PCIe devices, the PCIe Bus Master delegates `pcie_bar`, `dma_buffer`, and `irq_event` FDRs to guest or native driver processes. Drivers map BARs with `MMAP`, use `LD`/`ST` for device registers, use DMA buffer FDRs for device-visible memory, and wait on IRQ event FDRs for MSI/MSI-X completion.

For memory, the guest uses `MMAP`, `MUNMAP`, and `MPROTECT` to request native hardware VMAs. It does not write page tables directly. Linux/BSD tasks map one-to-one to hardware threads where practical, while the guest scheduler becomes an accounting and policy layer over the hardware runqueue.

This preserves the vision: Linux and NetBSD can be personalities projected onto native capability/event/domain silicon, rather than forcing LNP64 to become another trap-and-kernel RISC machine.

## 15. Native Security Invariants

LNP64 security is expressed through Resource Domains, VMAs, FDR capabilities, and hardware-owned object/capability generations plus lineage epochs rather than through a separate kernel ring model.

Hard v1 invariants:

*   **W^X by default:** The VMA Engine rejects simultaneous writable and executable permissions unless a domain explicitly holds a JIT/loader policy bit. JITs use write-then-execute transitions with `MPROTECT` and `ISYNC`, not permanent RWX mappings.
*   **NX data:** Heap, stacks, queues, shared memory, DMA buffers, device BARs, signal frames, and ordinary anonymous mappings default non-executable. Executable mappings must originate from executable image objects or an explicitly authorized loader/JIT transition.
*   **ASLR:** Process startup, `EXEC`, `MMAP`, heap arenas, stacks, signal trampolines, shared objects, call-gate trampolines, and guard regions are randomized with hardware entropy unless disabled by a delegated domain policy.
*   **Guard pages:** Stacks, heap arenas, delivery/signal frames, large allocations, and selected runtime objects can request unmapped or no-access guard VMAs. Guard faults route through native gate delivery.
*   **Entropy:** `RANDOM` is the architectural entropy source for libc, loaders, domain managers, allocator hardening, and compatibility personalities. `ENV_GET` reports feature bits; it does not provide secret randomness.
*   **Generation and lineage checks:** Domains, FDR entries, VMAs, heap arenas, waitable objects, call gates, event sources, DMA buffers, mapped device objects, and capability lineages carry generation/epoch fields. Stale or revoked references fail deterministically instead of silently reusing authority.
*   **Classed revocation:** Revocation advances lineage or revocation-root epochs and then follows the object's class: lazy cached invalidation, forced cancel, synchronous quiesce, or poison/fault. DMA buffers, IOMMU contexts, BAR mappings, and pages before reuse require quiescence; event sources, endpoints, classifier tables, and namespace handles can usually use lazy epoch invalidation; corrupted metadata becomes poison/fault until supervisor/PID 1 action.
*   **Revocation:** `CAP_REVOKE`, `DOMAIN_CTL`, `MUNMAP`, `MPROTECT`, and object teardown advance lineage/revocation epochs and invalidate cached descriptors, mappings, event sources, call gates, page-fill continuations, and DMA exports before authority is reused.
*   **Sealed and narrowed capabilities:** Authority can only move by explicit capability operations. Delegation may narrow rights, ranges, event masks, memory permissions, device scope, and transfer rights. Sealed capabilities can be used or transferred according to their rights but cannot be broadened or reminted by receivers.
*   **Capability minting discipline:** Software services never manufacture raw FDR authority. Namespace, network, PCIe, loader, supervisor, and filesystem services may select objects and propose returned capabilities, but hardware mints or installs authority only by deriving from an existing mint/root capability and only after validating object class, rights, ranges, generations, lineage, and domain policy. Service replies are data until the Capability Engine commits them.
*   **Memory visibility contract:** Normal cached memory is coherent and TSO-like by default. Ordinary loads/stores are easy for C/C++/Rust/Go/JVM runtimes and Unix personalities to reason about; weaker or device-specific behavior is opt-in through VMA memory type and explicit `FENCE`. Locked atomics are single-copy atomic and sequentially consistent in v1. Futex wait/wake, gate handoff/delivery, VMA/TLB updates, DMA visibility, `device_ordered`, and `write_combining` mappings have explicit ordering rules.
*   **DMA isolation:** Internal DMA, `DMA_CTL`, file I/O DMA, Ethernet, SD/SPI, and PCIe requester DMA all pass through VMA/capability checks, the coherent DMA fabric, Resource Domain accounting, and IOMMU/device scope. No device may DMA to arbitrary DDR or bypass revocation.
*   **Tenant-strict isolation:** `DOMAIN_PROFILE_TENANT_STRICT` combines mandatory memory hardening, no ambient devices, no raw interrupts, scoped telemetry, scoped DMA, explicit shared pages, and no parent memory read authority without a delegated capability.
*   **Confidential computing hooks:** Domain records reserve measured-launch, memory-encryption/key-id, shared-page, sealed-secret, and encrypted-checkpoint fields. Software owns secret policy and checkpoint formats; hardware enforces that confidential-domain memory and sealed capabilities are not exposed through ordinary parent inspection, telemetry, trace, DMA, or fault paths.

## 16. Native RAS and Operability Invariants

Cloud-grade LNP64 does not require a production fleet stack in FPGA v1, but the
first hardware version must preserve the architectural hooks that make reliable
operation possible.

Hard v1 requirements:

*   **Critical metadata ECC/parity:** FDR tables, VMA descriptors, domain tables, scheduler queues, event queues, heap metadata, DMA descriptors, namespace-dispatch records, and hardware-owned object metadata carry parity or ECC according to width and storage class. Corruption becomes a fault event, not silent authority reuse.
*   **Fault event model:** Engine faults, ECC/parity faults, invalid metadata, poisoned pages, DMA faults, watchdog timeouts, and boot measurement failures are delivered as structured events to PID 1, a supervisor Resource Domain, or a configured control FDR.
*   **Watchdogs and local reset:** Long-latency engines have bounded timeout states, abort paths, and local reset/degraded modes. A stuck Stream/Object, VMA, DMA, Capability, Event, or Domain engine should not require full-chip reset when local recovery is possible.
*   **Observability counters:** Domains and engines expose counters for issued/completed/aborted operations, queue depth, stalls, faults, bytes moved, scheduler transitions, capability sends/revokes, and resource pressure.
*   **Fleet observability without privileged scraping:** Counters, trace rings, pressure events, and fault records are FDR-backed telemetry capabilities with scope, generation, and domain policy. Monitoring domains receive delegated aggregate views; they do not scrape raw memory, raw interrupts, or global privileged state.
*   **Trace ring:** FPGA v1 includes a small optional trace ring for scheduler transitions, domain events, faults, capability delegation/revocation, call-gate calls, DMA completions, queue-steering decisions, and storage barriers.
*   **Remote attestation primitive:** The boot path records build id, FPGA bitstream/ROM identity, boot manifest hash, image measurements, domain launch measurements, selected boot policy, and delegated capability roots into read-only measurement records. A quote/attestation FDR exposes signed or development-mode attestations to authorized domains.
*   **Checkpoint and live-migration compatibility hooks:** `DOMAIN_CTL freeze/query-state/resume` must define quiescent state boundaries, bounded state records, dirty-state hooks, service callback events, endpoint drain/redirect hooks, and storage barrier integration for software checkpointing and future live migration. Hardware does not own checkpoint image formats, migration transport, or full restore in v1; future restore must create fresh domain/generation bases and reattach capabilities explicitly.
*   **Line-rate record classification and queue steering:** The bounded classifier is a first-class v1 datapath for packets, IPC completions, storage/DMA completions, trace records, and RAS events. It provides hash/mark/count/steer/drop actions into capability-scoped queues without becoming an arbitrary packet VM.
*   **Bounded servicelets:** Where fixed tables are too rigid, verified
    servicelets provide eBPF-like programmability using a strict subset of the
    existing LNP64 ISA. Servicelets are capability objects with bounded cycles,
    bounded actions, no blocking/allocation/arbitrary memory access, and
    verifier/proof obligations. They make realtime service personalities
    programmable without introducing general-purpose hidden CPUs, arbitrary
    bytecode VMs, or loadable control-store microcode.
*   **Storage durability contract:** Storage services and block objects define commit points, flush/barrier ordering, and replay/fsck expectations before RTL freeze. Live-system atomicity is not enough; power-fail durability must be specified, but general writable filesystem policy is not implemented in hardware.
*   **Deterministic failure containment proofs:** Each hardware engine must have a small enumerated state model, explicit commit/abort boundaries, local reset/degraded states, and proof or exhaustive-test obligations that faults cannot silently create authority, corrupt unrelated domains, or require full-chip reset when local recovery is possible.

## 17. Native Adoption Strategy
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
*   **IPC:** small scalar messages use queue, endpoint, or call-gate profiles;
    source-level `MSG_SEND` lowers to `PUSH` on a message endpoint. Receive
    paths use `AWAIT`/`PULL`. Capability payloads use `CAP_SEND` and
    `CAP_RECV`.
*   **Call gates:** `GATE_CALL` supports synchronous, asynchronous, and handoff
    activations into pre-provisioned threads, services, actors, supervisors, or
    domain entries. `CALL_CAP` remains a source/profile spelling. Cold domain
    creation remains `DOMAIN_CTL`.
*   **Runtime objects:** hardware exposes only `counter`, `queue`, and
    `memory_object`; semaphores, completions, channels, task queues, shared
    arenas, and DMA completions are profiles over those primitives.
