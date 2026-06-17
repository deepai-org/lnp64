# NetBSD Personality ABI Boundary

This file defines the first executable boundary for a NetBSD-like rump
personality on LNP64. The intent is not to boot a full NetBSD kernel in v1.
The intent is to keep BSD/POSIX compatibility in a software personality while
LNP64 owns capabilities, VMAs, domains, scheduling, wait/wake, object profiles,
and gate delivery.

`demos/netbsd_personality_smoke.c` is the current smoke artifact for this
boundary. `scripts/run_netbsd_personality_smoke.sh` compiles it, runs it, and
checks that the generated assembly still uses the expected native primitives.

## ABI Surface

| Personality surface | Compatibility view | Required native path |
| --- | --- | --- |
| Process entry | init-style startup, `main`, argv/env/auxv metadata | process entry record, `ENV_GET`, FDR 0/1/2 grants |
| Shell command dispatch | shell-like command table and child execution policy | software dispatch over normal calls plus `EXEC` for image replacement when needed |
| File open | `open`, `openat`, `opendir` | namespace/root/cwd capability plus `OPEN_AT`/current emulator `OPEN_FD` lowering |
| Byte I/O | `read`, `write`, `pread`, `pwrite`, `readv`, `writev` | `PULL`/`PUSH` or current FDR read/write dispatch |
| Close/dup/pass | `close`, `dup`, `dup2`, `fcntl`, descriptor passing | FDR close, `CAP_DUP`, `CAP_SEND`, `CAP_RECV`, lineage/generation checks |
| Pipes | `pipe`, inherited descriptors across `fork` | `OBJECT_CTL create queue(profile=pipe)` plus narrowed read/write FDRs |
| Processes | `fork`, `_exit`, `wait`, `waitpid`, `exec*` | `CLONE profile=new_process_cow`, child event/wait state, `EXEC` commit boundary |
| Threads | `pthread_create`, `pthread_join`, TSD | `CLONE profile=new_thread_shared_vm` or current `SPAWN` lowering |
| Futex/sync | pthread mutexes, condvars, semaphores, raw futex waits | `LOCK_CMPXCHG`, `FUTEX_WAIT`, `FUTEX_WAKE`, scheduler wakeups |
| Memory mappings | `mmap`, `munmap`, `mprotect`, allocator arenas | VMAs over anonymous, file, or object capabilities; W^X/NX/domain policy |
| Polling | `poll`, `select`, `epoll` | readiness probes plus `AWAIT`/event-queue wait paths |
| Timers/time | `timerfd`, `nanosleep`, `usleep`, `alarm`, clocks | timer object profile, PCR/timebase reads, sleep queues, event-queue wake or `GATE_DELIVER` into the POSIX alarm profile |
| Signals | `signal`, `sigaction`, masks, `raise`, `kill`, `SIGRET` | `GATE_CTL` for POSIX disposition gates, `GATE_MASK_SET`, `GATE_DELIVER`, `GATE_RETURN`; `SIGRET` is the POSIX alias for gate return |
| Faults/exceptions | `SIGILL`, `SIGFPE`, `SIGSEGV`, `SIGBUS`, `SIGTRAP` | native fault delivery records routed through the Gate/Continuation Engine, then mapped by the POSIX profile |
| Sockets | `socket`, `bind`, `listen`, `connect`, `accept`, `send`, `recv` | endpoint object profiles via `OBJECT_CTL`, stream `PUSH`/`PULL`, readiness waits |
| Rump filesystem hook | mounted block image or object-backed storage service | tiny checked rumpfs mount/read service over a block-image FDR fixture plus mmap/page-fill |
| Gates/upcalls | rump service calls, cross-domain delivery | `OBJECT_CTL create call_gate`, `GATE_CALL`, `GATE_RETURN`; `CALL_CAP`/`RET_CAP` are source/profile spellings |
| Resource domains | sandbox/container/rump service isolation | `DOMAIN_CTL` create/query/freeze/resume/attach/detach/destroy |

## Non-Goals For This Milestone

- No full monolithic NetBSD kernel port.
- No personality-owned page tables, scheduler dispatch, raw interrupts, raw
  DMA, or capability minting.
- No hardware TCP/IP, filesystem parser, or historical POSIX signal quirks
  beyond the frozen gate-delivery compatibility subset.
- No untyped `ioctl` escape hatch for object behavior.

## Current Smoke Coverage

`scripts/run_netbsd_personality_smoke.sh` verifies:

- init-style startup, shell-like command dispatch output, and a forked `exec`
  shell command,
- `openat(AT_FDCWD, ...)` file-open compatibility plus descriptor reads,
- pipe/fd inheritance through `fork`, `poll`, and `wait`,
- descriptor passing through narrowed FDR capabilities over a queue,
- an mmap-backed allocator arena, `mprotect`/`munmap`, and a tiny checked
  rumpfs mount/read service over a block-image FDR,
- POSIX signal-profile delivery through gate disposition, mask/pending state,
  `raise`, and `SIGRET`/`GATE_RETURN`,
- pthread startup/join, futex wake, `select`, `epoll`, `usleep`, `alarm`, and
  timerfd wait,
- TCP loopback through endpoint object controls,
- call-gate delivery and Resource Domain freeze/resume/attach/detach/destroy
  with delegated budget isolation,
- assembly evidence for FDR I/O, `FORK`, `SPAWN`, `FUTEX_*`, `OBJECT_CTL`,
  `MMAP`, `MPROTECT`, `MUNMAP`, `POLL_FD_DYN`, `AWAIT_DYN`, `SIGACTION`,
  `KILL`, `CAP_DUP`, `CAP_SEND`, `CAP_RECV`, `DOMAIN_CTL`,
  `CALL_CAP`/`GATE_CALL`, and `RET_CAP`/`GATE_RETURN`.

## Open Work

- Expand the current tiny checked rumpfs service into a real NetBSD-derived
  filesystem component that owns a block/object FDR and exposes namespace/file
  services back through capabilities.
- Add negative tests proving the personality cannot resolve paths without a
  delegated root/cwd capability and cannot use raw interrupts, raw DMA, raw page
  tables, hidden scheduler authority, or widened transferred capabilities.
- Move `poll`/`select` blocking paths toward a first-class event queue profile
  while keeping readiness probes as compatibility helpers.
- Add a software loader/exec-plan path for NetBSD-like userland images instead
  of relying on compiler-emitted assembly as the image format.
- Import small NetBSD-derived libc/userland components once the ABI smoke stays
  stable under this gate.
