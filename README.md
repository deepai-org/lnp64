# LNP64 Emulator and C Compiler

This repository contains a runnable Rust implementation of the draft LNP64
design in `design.md`.

It is intentionally a practical system emulator, not a transistor-accurate
model. The implemented subset covers:

- 64-bit GPRs (`r0`-`r31`), FDRs (`fd0`-`fd255`), PCRs, dedicated FPU
  registers (`f0`-`f31`), and vector registers (`v0`-`v15`).
- Integer ALU, compare, branches, jumps, calls, and returns.
- Load/store memory, `.data` labels, strings, quads, zero-fill, VMAs, and
  lazy file-backed `MMAP` page-in.
- Hardware-style `ALLOC`/`FREE`.
- FDR I/O through the current emulator instructions, with the architectural
  direction converging on `OPEN_AT`, `PULL`, `PUSH`, `SEEK`, and capability
  operations.
- Real emulator-level process cloning and assembly-program loading for `EXEC`,
  with the architectural direction converging on `CLONE` profiles for process,
  thread, and POSIX-fork compatibility.
- Ready-queue scheduling, futex parking/wake, signal delivery with `SIGRET`,
  IPC messages, Resource Domains, minimal `OBJECT_CTL`, and `CALL_CAP`/`RET_CAP`
  service calls.
- Reserved low-level device/debug hooks exist in the emulator, but ordinary
  device access is intended to flow through FDR capabilities, `MMAP`, DMA
  buffers, event objects, and service domains rather than raw `INB`/`OUTB`.
- A small C compiler written in Rust that emits LNP64 assembly.

## Formal Verification Direction

LNP64 is intended to be proof-friendly from the architecture level down. The
long-term goal is not only to test the emulator, but to make the important
security and crash-freedom properties either proven, locally checkable, or
structurally impossible to violate.

The preferred proof source is the Lean/pure-math ecosystem for the abstract
machine and security invariants, rather than relying only on hardware-design
verification tools. Lean should model domains, capabilities, FDR tables, VMAs,
waitables, scheduler state, DMA buffers, and architectural transitions such as
`CAP_DUP`, `CAP_REVOKE`, `MMAP`, `MPROTECT`, `AWAIT`, `WAKE`, `DOMAIN_CTL`,
`CALL_CAP`, and `DMA_CTL`.

Key proof targets include:

- capability non-forgeability.
- monotonic delegation and no authority amplification.
- revocation soundness and stale-generation rejection.
- Resource Domain containment.
- W^X and NX-data invariants.
- DMA isolation through exported buffer capabilities.
- scheduler state validity.
- no lost wakeups for waitable objects.
- crash-free hardware engine transitions with explicit commit/abort points.

RTL assertions, bounded model checking, and simulation still matter, but their
role is local: handshake correctness, valid FSM states, no double commit, no
response without request, and refinement checks against the Lean-level
transition model.

## Emulator ISA Deltas

The emulator tracks the current ISA direction in `design.md` and
`hardware_design.md`:

- `MSG_RECV` and `PIPE` are not accepted standalone assembly instructions.
- Scalar send remains `MSG_SEND`; receive lowers to `AWAIT` plus `PULL` on the
  reserved per-process message endpoint `fd255`.
- Source-level `pipe(fds)` lowers to `OBJECT_CTL create queue(profile=pipe)`,
  returning narrowed read/write endpoint FDRs.
- Source-level `poll(fds, nfds, timeout)` and 64-bit `fd_set` `select` lower to
  `POLL_FD_DYN` readiness probes plus `AWAIT_DYN` for blocking waits. This is
  the emulator's current single-process bridge toward the event-queue profile
  described in the design docs.
- Source-level `clock_gettime`, `gettimeofday`, and `time` read
  `REALTIME_SEC`/`REALTIME_NSEC` PCRs. `nanosleep`, `usleep`, and `alarm` use
  the emulator's current coarse timer path; architecturally these are timer
  object profiles waited on through `AWAIT`.
- POSIX-style `getpid`/`getppid`/`getuid`/`getgid` aliases read PCRs,
  `wait(status)` lowers to `WAIT_PID`, `raise(signum)` sends through `KILL`,
  and the current 64-bit `sigset_t`/`sigprocmask` subset updates the `SIGMASK`
  PCR.
- `EVENT_CTL` and `TIMER_CTL` are accepted by the assembler as aliases over
  `OBJECT_CTL`.
- `CALL_CAP`/`RET_CAP` implement synchronous calls, asynchronous completion to
  a counter or queue endpoint, and handoff mode.
- C allocation builtins lower to the native heap path: `malloc`/`calloc` use
  `ALLOC`, `aligned_alloc`/`posix_memalign` use `ALLOC_EX`, and `realloc` uses
  `ALLOC_SIZE` metadata before copying and freeing the old allocation. The
  `brk`/`sbrk` compatibility layer tracks a process break cursor while positive
  `sbrk` growth still obtains memory from `ALLOC`.
- The current pthread/sync subset maps `pthread_create` to a same-process
  thread creation path equivalent to `CLONE profile=thread`, `pthread_join` to a
  join wait, and implements mutexes, condition variables, rwlocks,
  `pthread_once`, and POSIX-style semaphores with atomics and futex-flavored
  wait/wake operations. The architectural form is `LOCK_CMPXCHG` plus `AWAIT`
  and wake operations over waitable objects.

Current emulator `OBJECT_CTL` v1 fields are a compact implementation ABI, not
the final typed control envelope described in the architecture documents:

```text
arg+0  op: 1=create
arg+8  kind: 1=counter, 2=queue, 3=memory_object
arg+16 profile: 1=pipe, 4=call_gate for queue objects
arg+24 requested/result fd0
arg+32 requested/result fd1 or target domain id for call_gate
arg+40 initial counter value, memory size, or call_gate entry PC
arg+48 call_gate mode: 0=sync, 1=async, 2=handoff
arg+56 async completion fd
arg+64 call_gate flags, bit0 permits capability-marked args
```

## Build and Test

```sh
cargo test
```

## Compile and Run a Demo

```sh
cargo run -- cc demos/hello.c -o /tmp/hello.lnp64.s
cargo run -- run /tmp/hello.lnp64.s
```

Run all demos:

```sh
bash scripts/run_demos.sh
```

Build and boot the minimal userland image:

```sh
bash scripts/run_userland.sh
```

Run the `inih` real-package smoke gate:

```sh
bash scripts/run_inih.sh
```

Run the `zlib` real-package checksum smoke gate:

```sh
bash scripts/run_zlib.sh
```

Run the `natsort` real-package string comparison smoke gate:

```sh
bash scripts/run_natsort.sh
```

Run the `cwalk` real-package path manipulation smoke gate:

```sh
bash scripts/run_cwalk.sh
```

Run the `jsmn` real-package JSON parser smoke gate:

```sh
bash scripts/run_jsmn.sh
```

Run the `libc-test` focused libc conformance subset:

```sh
bash scripts/run_libc_test.sh
```

Run all checked third-party real-package gates:

```sh
bash scripts/run_real_packages.sh
```

For the current POSIX/libc surface, real-program gates, and open compatibility
bugs, see `conformance_matrix.md`. For the current emulator process ABI, see
`psABI.md`; for the target ELF/static object profile, see `object_format.md`.

## Docker

```sh
docker build -t lnp64 .
docker run --rm lnp64
```

The container runs the Rust tests and compiles/runs all demo programs.

## Real C Program Targets

Unmodified upstream C sources live under `third_party/` as compiler and runtime
targets. The project should grow support for these programs through general C
frontend, libc, and emulator work, not by replacing specific programs with
handwritten implementations.

```sh
bash scripts/run_real_packages.sh
bash scripts/run_sbase.sh
bash scripts/run_jsmn.sh
bash scripts/run_natsort.sh
bash scripts/run_cwalk.sh
bash scripts/run_libc_test.sh
```
