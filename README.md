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
- A small C compiler written in Rust that emits LNP64 assembly.

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
