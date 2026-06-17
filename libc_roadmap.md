# LNP64 Libc Roadmap

This document sketches the libc/runtime work needed to make LNP64 software use the hardware naturally instead of treating the machine as a conventional CPU with unusual syscalls.

## Goal

LNP64 libc should be the normal way to access files, memory, synchronization, isolation, eventing, process control, and service calls. It should expose familiar POSIX and C interfaces where possible, while routing those interfaces to native capability/event/domain primitives such as `OPEN_AT`, `PULL`, `PUSH`, `AWAIT`, `OBJECT_CTL`, `DOMAIN_CTL`, `CALL_CAP`, `ALLOC`, `MMAP`, and capability transfer.

The implementation should avoid building a hidden software kernel inside libc. Libc is the ABI adapter, policy surface, and compatibility layer. Hardware-owned objects remain the authority for capabilities, waitability, memory mappings, domains, and scheduling-visible state.

The libc contract is profile-based: POSIX names are source and ABI compatibility, not the primitive architecture. `open` returns an FDR-backed capability handle, `pipe` creates a queue profile, `poll`/`select`/`epoll` use event-queue profiles, `fork` is the constrained `CLONE profile=posix_fork`, signals use the frozen hardware signal subset, and `errno` is a thread-local compatibility view of explicit result/error status.

## Layering

1. **Startup and ABI**

   Provide `_start`, crt objects, process entry, `argc`/`argv`/`envp`, auxiliary vector handling, thread pointer setup, TLS, hardware `errno` access, `atexit`, and clean `_exit`.

2. **Internal LNP64 Shim Layer**

   Keep public libc APIs separate from instruction details through private helpers such as `__lnp_openat`, `__lnp_pull`, `__lnp_push`, `__lnp_await`, `__lnp_object_ctl`, `__lnp_domain_ctl`, `__lnp_call_cap`, and `__lnp_alloc`.

   This gives the ISA room to evolve without rewriting every public interface.

3. **Files, Streams, and Namespaces**

   Implement `open`, `openat`, `close`, `read`, `write`, `pread`, `pwrite`, `lseek`, `dup`, `dup2`, `fcntl`, `stat`, `fstat`, directory iteration, stdio, and path helpers on top of capability FDRs and stream operations.

   `pipe()` should lower to `OBJECT_CTL create queue(profile=pipe)` with narrowed read/write endpoint capabilities.

4. **Memory and Allocation**

   Implement `malloc`, `free`, `realloc`, `calloc`, `aligned_alloc`, `posix_memalign`, `mmap`, `munmap`, `mprotect`, `msync`, and any `brk`/`sbrk` compatibility needed by imported software.

   The native allocator should wrap the LNP64 Default Heap Algorithm directly:
   `ALLOC`, `ALLOC_EX`, `ALLOC_SIZE`, and `FREE` expose allocation intent,
   alignment, zeroing, guard/debug/locality hints, arena tags, shared/DMA
   eligibility, and allocation-size queries. The hardware algorithm is a
   domain-aware segregated bump allocator with fixed size classes, per-thread
   allocation windows, slab/run backing, bounded cross-thread free transfer,
   quarantine/guard hooks, and VMA-backed large objects. Libc should not depend
   on raw freelists, slab layout, allocation-window depth, quarantine algorithms,
   or internal coalescing policy. Those remain Heap Engine representation
   details.

   Libc and runtimes must preserve the two-mode allocation model:

   - Hardware-owned allocations: `malloc`-like calls use `ALLOC`/`FREE`, so
     hardware tracks each allocation object and provides object-level safety,
     invalid-free detection, `ALLOC_SIZE`, hardening, and accounting.
   - Software-owned arenas: GC heaps, bump allocators, database slabs, packet
     pools, and language object pools use `MMAP`, `memory_object`, or
     arena-style `ALLOC_EX`. Hardware tracks the outer region; the runtime owns
     the inner object representation and inner correctness.

   Runtimes can use arenas, tags, page-run/large-object hints, `memory_object`,
   and `MMAP` for specialized heaps while keeping Resource Domain accounting and
   VMA/capability safety checks intact.

   Object-backed mappings use the hardware page transaction protocol, but
   `msync`, shared-file coherence, truncation behavior, and mapped-file
   writeback are service/personality policy surfaced through typed controls.

5. **Threads and Synchronization**

   Implement `pthread_create`, `pthread_join`, `pthread_detach`, `pthread_exit`, `pthread_self`, mutexes, condition variables, rwlocks, `pthread_once`, semaphores, C11 atomics, and futex-like internal waits.

   Libc synchronization should prefer hardware waitable objects and local-state fast paths, with `AWAIT` and `WAKE` only on contended paths.

6. **Events, Polling, and Time**

   Implement `poll`, `select`, `epoll`-style APIs, `clock_gettime`, `gettimeofday`, `nanosleep`, timers, alarms, and timeout handling using event-queue and timer object profiles over `OBJECT_CTL`.

   Multi-source waits should arm event queue sources atomically, check readiness before parking, and use generation counters to avoid lost wakeups.

7. **Processes and Signals**

   Implement `fork` as `CLONE profile=posix_fork`: new PID, one child thread,
   COW VMAs/heap metadata, defined FDR inheritance, copied credentials and
   signal dispositions, caller mask copied, child pending signals cleared, and
   no in-flight operations copied. Libc-owned `pthread_atfork` handlers run
   before issuing the hardware clone; lock recovery is not a hardware semantic.

   Implement `exec`, `_exit`, `wait`, `waitpid`, `kill`, `raise`,
   `alarm`, `sigaction`, `sigprocmask`, signal delivery frames, signal return,
   default/ignore/handler dispositions, and thread-local signal masks over the
   frozen hardware signal subset.

   Hardware faults should enter the same signal path as software-raised POSIX
   signals. Full realtime queueing, legacy restart behavior, and OS-specific
   delivery corners should remain libc/personality compatibility policy.
   Libc should expand the compact hardware frame into the target `siginfo_t` and
   `ucontext_t` shapes, implement `SA_RESTART` and `sigaltstack` compatibility
   policy above the hardware substrate, and preserve the hardware rule that
   `SIGRET` restores only from a Signal Engine-owned context token/generation.

8. **Sockets and Networking**

   Provide POSIX socket APIs over native endpoint capabilities and network-driver
   services. Treat `stream_endpoint`, `datagram_endpoint`, and `listener` as
   endpoint shapes, not evidence that hardware implements TCP or UDP. Early
   libc can support a small socket subset first, but the ABI should leave room
   for `select`/`poll`, nonblocking I/O, accepted listener endpoints,
   descriptor passing, software TCP services, QUIC/local transports, and future
   optional transport accelerators behind the same endpoint ABI.

   Libc should preserve the typed endpoint boundary: packet queues preserve
   packet records, datagram endpoints preserve message boundaries, stream
   endpoints expose ordered bytes and backpressure, and listeners return endpoint
   capabilities. Socket options should lower to typed `GET_META`/`SET_META` or
   fail cleanly; libc should not depend on whether a stream is software TCP,
   local IPC, paravirtual transport, QUIC, or an accelerator.

9. **LNP64 Extension APIs**

   Expose first-class interfaces for hardware capabilities that POSIX does not model cleanly:

   - Native service helpers for issuing bounded service transactions over call
     gates, queues, namespace dispatch, typed controls, page-fill callbacks, and
     stream endpoints. Libc should surface service generation checks, bounded
     input/output records, capability argument tables, returned-capability
     slots, backpressure outcomes, cancellation, and service-crash errors
     consistently instead of inventing one-off daemon protocols for filesystems,
     loaders, networking, PCIe, telemetry, or personalities.
   - Typed control envelope helpers for `GET_META`, `SET_META`, `OBJECT_CTL`,
     `DOMAIN_CTL`, `NS_CTL`, socket options, storage barriers, and service
     controls. Libc should not expose ad hoc raw `ioctl` blobs when a typed
     envelope profile exists. Public headers should distinguish architectural
     profiles, personality/service profiles, and vendor/device profiles, and
     should preserve bounded input/output, explicit capability arguments,
     returned-capability slots, copied-vs-pinned buffer semantics, single
     commit-point behavior, and fail-closed error behavior. Payload bytes should
     be data only; returned authority must be surfaced as installed FDR
     capabilities, not as integers hidden inside backend payloads.
   - Resource Domain creation, configuration, freeze/resume, destroy, and usage
     queries, with VM/container/cgroup/sandbox profiles over the same
     `DOMAIN_CTL create child` primitive.
   - Mission-assurance helpers over `DOMAIN_CTL`/`GET_META`: configure/query
     mission profiles, dependency graph hashes, fail policies, degraded-state
     records, recovery priority, stale-data budgets, and quoteable mission
     evidence. These APIs should expose mission continuity as Resource Domain
     metadata, not as a separate supervisor daemon protocol.
   - Checkpoint support helpers for `freeze`, `query-state`, `resume`, dirty
     tracking, and explicit capability reattachment. Checkpoint image formats,
     migration transport, device/service state capture, and restore policy stay
     in software.
   - Capability call gates, sync calls, async calls, handoff calls, and completion waits.
   - Counter, queue, and memory object creation.
   - Capability send, receive, duplicate, narrow, seal, revoke, and generation checks.
   - Revocation helper APIs should expose the four architectural outcomes:
     `lazy_epoch` invalidation, `forced_cancel`, `synchronous_quiesce`, and
     `poison_fault`, so runtimes can choose cheap cached invalidation for ordinary
     handles and explicit quiesce for DMA/page/device-backed resources.
   - Native allocator arenas and shared-memory objects.

## Practical Order

1. Stabilize process startup, TLS, `errno`, and the private `__lnp_*` shim layer.
2. Complete files, descriptors, stdio, and namespace basics.
3. Build the native allocator and `mmap` family.
4. Implement pthreads and synchronization on hardware waitables.
5. Finish `poll`, `select`, event queues, timers, and time APIs.
6. Add process and signal compatibility.
7. Add LNP64 extension APIs for domains, call gates, and capability objects.
8. Expand sockets and service-oriented runtime support.

## Testing Strategy

Libc should be tested at three levels:

- **Instruction lowering tests** for compiler/runtime paths that must map to native primitives.
- **POSIX compatibility tests** for imported C software expecting normal libc behavior.
- **LNP64-native tests** for domains, call gates, capability transfer, object queues, hardware allocator behavior, and event queue semantics.

The target is not just passing small programs. The target is making correct native hardware usage the path of least resistance for real C software.
