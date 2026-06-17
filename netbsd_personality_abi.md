# NetBSD Personality ABI Boundary

This file defines the first executable boundary for a NetBSD-like rump
personality on LNP64. The intent is not to boot a full NetBSD kernel in v1.
The intent is to keep BSD/POSIX compatibility in a software personality while
LNP64 owns capabilities, VMAs, domains, scheduling, wait/wake, object profiles,
and gate delivery.

`demos/netbsd_personality_smoke.c` is the focused ABI smoke artifact for this
boundary. `scripts/run_netbsd_personality_smoke.sh` compiles it, runs it, and
checks that the generated assembly still uses the expected native primitives.
`scripts/run_netbsd_personality_system.sh` is the larger userland-style system
gate: it boots `userland/netbsd_init.c`, executes `userland/netbsd_sh.c`, runs
several compiled C test programs, and audits the generated native trace.
`src/lowering.rs` is the typed compatibility dispatch table for NetBSD/POSIX
surfaces used by this gate. It also carries the initial NetBSD-current
syscall-number subset for the gate's supported calls, routing them to the same
compatibility surfaces instead of creating an emulator syscall escape.

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
| Rump filesystem hook | mounted block image or object-backed storage service | tiny checked rumpfs mount/read service over a block-image FDR fixture plus the system gate's fixed-record service-owned image with mmap, mutation, metadata, and flush/barrier checks |
| Gates/upcalls | rump service calls, cross-domain delivery | `OBJECT_CTL create call_gate`, `GATE_CALL`, `GATE_RETURN`; `CALL_CAP`/`RET_CAP` are source/profile spellings |
| Resource domains | sandbox/container/rump service isolation | `DOMAIN_CTL` create/query/freeze/resume/attach/detach/destroy |

The checked lowering table in `src/lowering.rs` covers cwd/root/openat, byte
I/O, pipes, poll/select/epoll, fork/exec, pthreads, mmap, fd passing, sockets,
timers, call gates, signals, Resource Domains, errno, and metadata operations.
The initial NetBSD syscall-number dispatch subset covers the corresponding
open/read/write/close, pipe, poll/select/epoll, fork/exec/wait, LWP/thread,
mmap, descriptor passing, socket, timer, cwd/root, metadata, and signal calls
used by the system gate.

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
  `raise`, child-exit `SIGCHLD`, and `SIGRET`/`GATE_RETURN`,
- pthread startup/join, futex wake, `select`, `epoll`, `usleep`, `alarm`, and
  timerfd wait,
- TCP loopback through endpoint object controls,
- call-gate delivery and Resource Domain freeze/resume/attach/detach/destroy
  with delegated budget isolation,
- assembly evidence for FDR I/O, `FORK`, `SPAWN`, `FUTEX_*`, `OBJECT_CTL`,
  `MMAP`, `MPROTECT`, `MUNMAP`, `POLL_FD_DYN`, `AWAIT_DYN`, `SIGACTION`,
  `KILL`, `CAP_DUP`, `CAP_SEND`, `CAP_RECV`, `DOMAIN_CTL`,
  `CALL_CAP`/`GATE_CALL`, and `RET_CAP`/`GATE_RETURN`.

## Current System Gate

`scripts/run_netbsd_personality_system.sh` builds a temporary personality root
with `/sbin/init.s`, `/bin/netbsd_sh.s`, and compiled test programs, then boots
it with `run --namespace-root` so guest absolute paths resolve inside that root.
The test set covers cwd/root/openat, a fixed-record software exec-plan smoke,
threads, poll/select/epoll, a service-owned filesystem image, mmap, fd passing,
loopback sockets, signal gates, call gates, timers, and Resource Domain budget
checks. The scripted shell runs:

```sh
/init
/bin/sh -c 'netbsd personality system script'
echo hello > /tmp/a
cat /tmp/a | wc
mkdir /tmp/d
ls /tmp
./thread_test
./namespace_test
./loader_test
./poll_test
./fs_service_test
./mmap_test
./fd_passing_test
./gate_trace_test
./timer_test
./socket_loopback_test
./signal_gate_test
./domain_nested_test
./domain_budget_test
```

The runner verifies the transcript, checks native primitive evidence including
FDR I/O, `CHDIR_PATH`, `GETCWD_PATH`, `MMAP`, `PWRITE_FD_DYN`, `FD_SEEK`,
`AWAIT_DYN`, `OBJECT_CTL`, `DOMAIN_CTL`, `CAP_*`, `CALL_CAP`/`RET_CAP`,
`FORK`, `EXEC`, `SPAWN`, `SLEEP`, `ALARM`, `SIGACTION`, `SIGRET`, and rejects
raw interrupt/MMIO/DMA/page-table/scheduler/syscall trace tokens. It also
verifies stale FDR generation rejection via
`demos/stale_fd_token.s`; the shell launches each compiled child program in a
fresh Resource Domain, verifies that domain's PID counter returns to zero after
`wait`, destroys the child domain, and checks the supervisor domain PID counter
returns to its baseline. The filesystem-service test maps a generated
fixed-record image, performs service-owned path walking, create, rename, link,
unlink, metadata update, and an explicit flush/barrier through offset I/O, then
reopens the image to verify persisted state. The loader test validates a
service-owned `/etc/loader_target.execplan` record before forking and execing
the planned target, keeping the compatibility decision in userland while the
full ELF-to-exec-plan loader remains future work.
`signal_gate_test` covers masked compatibility delivery and child-exit
`SIGCHLD` as native event delivery before ABI handler return.
`domain_nested_test` creates jail/container-style nested Resource Domains from
compiled C, verifies parent/depth/child-count query fields, checks delegated
limit rejection, exercises attach/detach accounting, and confirms freeze/resume
propagates through the nested tree.

## Open Work

- Promote the current fixed-record service-owned filesystem image into a real
  NetBSD-derived component that exposes namespace/file services back through
  capabilities.
- Broaden negative tests for delegated namespace roots/cwd, lexical escape, raw
  interrupts, raw DMA, raw page tables, hidden scheduler authority, and widened
  transferred capabilities.
- Move `poll`/`select` blocking paths toward a first-class event queue profile
  while keeping readiness probes as compatibility helpers.
- Replace the current fixed-record exec-plan smoke with a software loader for
  NetBSD-like userland images instead of relying on compiler-emitted assembly as
  the image format.
- Import small NetBSD-derived libc/userland components once the ABI smoke stays
  stable under this gate.
