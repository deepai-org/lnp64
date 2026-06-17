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
| Timers/time | `timerfd`, `nanosleep`, `usleep`, `alarm`, clocks | timer object profile, PCR/timebase reads, sleep queues, signal event delivery |
| Signals | `signal`, `sigaction`, masks, `raise`, `kill`, `SIGRET` | native event queue, signal frame construction at compatibility boundary |
| Sockets | `socket`, `bind`, `listen`, `connect`, `accept`, `send`, `recv` | endpoint object profiles via `OBJECT_CTL`, stream `PUSH`/`PULL`, readiness waits |
| Rump filesystem hook | mounted block image or object-backed storage service | checked block-image FDR fixture plus mmap/page-fill and service-owned filesystem logic |
| Gates/upcalls | rump service calls, cross-domain delivery | `OBJECT_CTL create call_gate`, `CALL_CAP`, `RET_CAP` |
| Resource domains | sandbox/container/rump service isolation | `DOMAIN_CTL` create/query/freeze/resume/attach/detach/destroy |

## Non-Goals For This Milestone

- No full monolithic NetBSD kernel port.
- No personality-owned page tables, scheduler dispatch, raw interrupts, raw
  DMA, or capability minting.
- No hardware TCP/IP, filesystem parser, or POSIX signal policy beyond the
  frozen compatibility subset.
- No untyped `ioctl` escape hatch for object behavior.

## Current Smoke Coverage

`scripts/run_netbsd_personality_smoke.sh` verifies:

- init-style startup and shell-like command dispatch output,
- pipe/fd inheritance through `fork`, `poll`, and `wait`,
- anonymous mmap memory and a checked block-image-backed rumpfs mount hook,
- signal delivery through `signal`, `raise`, and `SIGRET`,
- pthread startup/join, futex wake, `select`, and timerfd wait,
- TCP loopback through endpoint object controls,
- call-gate delivery and Resource Domain attach/detach/destroy,
- assembly evidence for FDR I/O, `FORK`, `SPAWN`, `FUTEX_*`, `OBJECT_CTL`,
  `MMAP`, `POLL_FD_DYN`, `AWAIT_DYN`, `SIGACTION`, `KILL`, `DOMAIN_CTL`,
  `CALL_CAP`, and `RET_CAP`.

## Open Work

- Expand the current checked block-image hook into a rump-style filesystem
  service that owns a block/object FDR and exposes namespace/file services back
  through capabilities.
- Add negative tests proving the personality cannot resolve paths without a
  delegated root/cwd capability and cannot use raw interrupts, raw DMA, raw page
  tables, or hidden scheduler authority.
- Move `poll`/`select` blocking paths toward a first-class event queue profile
  while keeping readiness probes as compatibility helpers.
- Add a software loader/exec-plan path for NetBSD-like userland images instead
  of relying on compiler-emitted assembly as the image format.
- Import small NetBSD-derived libc/userland components once the ABI smoke stays
  stable under this gate.
