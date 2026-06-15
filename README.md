# LNP64 Emulator and C Compiler

This repository contains a runnable Rust implementation of the draft LNP64
design in `design.md`.

It is intentionally a practical system emulator, not a transistor-accurate
model. The implemented subset covers:

- 64-bit GPRs (`r0`-`r31`), FDRs (`fd0`-`fd255`), and PCR reads.
- Integer ALU, compare, branches, jumps, calls, and returns.
- Load/store memory, `.data` labels, strings, quads, and zero-fill.
- Hardware-style `ALLOC`/`FREE`.
- FDR I/O via `WRITE_FD`, `READ_FD`, `OPEN_FD`, and `FD_DUP`.
- Process/scheduler/system instructions as deterministic emulator stubs where
  full host process virtualization would be outside scope.
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
for src in demos/*.c; do
  asm="/tmp/$(basename "$src" .c).s"
  cargo run --quiet -- cc "$src" -o "$asm"
  echo "== $src =="
  cargo run --quiet -- run "$asm"
done
for src in demos/*.s; do
  echo "== $src =="
  cargo run --quiet -- run "$src"
done
```

## Docker

```sh
docker build -t lnp64 .
docker run --rm lnp64
```

The container runs the Rust tests and compiles/runs all demo programs.
