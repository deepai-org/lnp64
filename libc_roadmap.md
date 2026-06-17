# LNP64 Libc Roadmap

This document sketches the libc/runtime work needed to make LNP64 software use the hardware naturally instead of treating the machine as a conventional CPU with unusual syscalls.

## Goal

LNP64 libc should be the normal way to access files, memory, synchronization, isolation, eventing, process control, and service calls. It should expose familiar POSIX and C interfaces where possible, while routing those interfaces to native capability/event/domain primitives such as `OPEN_AT`, `PULL`, `PUSH`, `AWAIT`, `OBJECT_CTL`, `DOMAIN_CTL`, `CALL_CAP`, `ALLOC`, `MMAP`, and capability transfer.

The implementation should avoid building a hidden software kernel inside libc. Libc is the ABI adapter, policy surface, and compatibility layer. Hardware-owned objects remain the authority for capabilities, waitability, memory mappings, domains, and scheduling-visible state.

The libc contract is profile-based: POSIX names are source and ABI compatibility, not the primitive architecture. `open` returns an FDR-backed capability handle, `pipe` creates a queue profile, `poll`/`select`/`epoll` use event-queue profiles, `fork` is a `CLONE` profile, signals are an ABI view over hardware event delivery, and `errno` is a thread-local compatibility view of explicit result/error status.

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

   Implement `malloc`, `free`, `realloc`, `calloc`, `aligned_alloc`, `posix_memalign`, `mmap`, `munmap`, `mprotect`, and any `brk`/`sbrk` compatibility needed by imported software.

   The native allocator should use hardware heap arenas and object metadata so ordinary programs are not tempted to ship custom allocators for performance or correctness.

5. **Threads and Synchronization**

   Implement `pthread_create`, `pthread_join`, `pthread_detach`, `pthread_exit`, `pthread_self`, mutexes, condition variables, rwlocks, `pthread_once`, semaphores, C11 atomics, and futex-like internal waits.

   Libc synchronization should prefer hardware waitable objects and local-state fast paths, with `AWAIT` and `WAKE` only on contended paths.

6. **Events, Polling, and Time**

   Implement `poll`, `select`, `epoll`-style APIs, `clock_gettime`, `gettimeofday`, `nanosleep`, timers, alarms, and timeout handling using event-queue and timer object profiles over `OBJECT_CTL`.

   Multi-source waits should arm event queue sources atomically, check readiness before parking, and use generation counters to avoid lost wakeups.

7. **Processes and Signals**

   Implement `fork`, `exec`, `_exit`, `wait`, `waitpid`, `kill`, `sigaction`, `sigprocmask`, signal delivery frames, signal return, default dispositions, and thread-local signal masks.

   Hardware faults should enter the same signal path as software-raised POSIX signals.

8. **Sockets and Networking**

   Provide POSIX socket APIs over native endpoint capabilities and network-driver services. Early libc can support a small socket subset first, but the ABI should leave room for `select`/`poll`, nonblocking I/O, accepted listener endpoints, and descriptor passing.

9. **LNP64 Extension APIs**

   Expose first-class interfaces for hardware capabilities that POSIX does not model cleanly:

   - Resource Domain creation, configuration, freeze/resume, destroy, and usage queries.
   - Capability call gates, sync calls, async calls, handoff calls, and completion waits.
   - Counter, queue, and memory object creation.
   - Capability send, receive, duplicate, narrow, seal, revoke, and generation checks.
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
