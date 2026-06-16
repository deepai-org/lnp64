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
- FDR I/O via `WRITE_FD`, `READ_FD`, `OPEN_FD`, and `FD_DUP`.
- Real emulator-level process cloning for `FORK`, assembly-program loading for
  `EXEC`, hardware-thread contexts for `SPAWN`, ready-queue scheduling,
  futex parking/wake, signal delivery with `SIGRET`, IPC messages, and
  loadable microcode port hooks for `INB`/`OUTB`.
- Resource Domains, minimal `OBJECT_CTL`, and `CALL_CAP`/`RET_CAP` service
  calls.
- A small C compiler written in Rust that emits LNP64 assembly.

## Emulator ISA Deltas

The emulator tracks the current ISA direction in `design.md` and
`hardware_design.md`:

- `MSG_RECV` and `PIPE` are not accepted standalone assembly instructions.
- Scalar send remains `MSG_SEND`; receive lowers to `AWAIT` plus `PULL` on the
  reserved per-process message endpoint `fd255`.
- Source-level `pipe(fds)` lowers to `OBJECT_CTL create queue(profile=pipe)`,
  returning narrowed read/write endpoint FDRs.
- `EVENT_CTL` and `TIMER_CTL` are accepted by the assembler as aliases over
  `OBJECT_CTL`.
- `CALL_CAP`/`RET_CAP` implement synchronous calls, asynchronous completion to
  a counter or queue endpoint, and handoff mode.

Current emulator `OBJECT_CTL` v1 fields are:

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
bash scripts/run_sbase.sh
```
