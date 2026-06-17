# LNP64 Emulator and C Compiler

LNP64 is a draft capability-machine architecture for system software. It keeps
ordinary computation as a conventional load/store CPU, but makes files,
memory, waitables, devices, service calls, and isolation hardware-visible
capability objects.

This repository contains a practical Rust emulator, assembler, and small C
compiler for exploring that design. It is not a transistor-accurate RTL model.

## Architecture In One Page

- File Descriptor Registers (FDRs) are unforgeable capabilities, not just Unix
  integer descriptors.
- Resource operations use native instructions such as `PULL`, `PUSH`, `AWAIT`,
  `MMAP`, `CAP_*`, `OBJECT_CTL`, `DOMAIN_CTL`, `CALL_CAP`, and `DMA_CTL`.
- Resource Domains unify containers, VMs, cgroups, sandboxes, supervisors, and
  mission/assurance profiles.
- Services own evolving policy: filesystems, loaders, networking, PCIe quirks,
  Unix compatibility, declassification, and orchestration.
- Hardware owns enforcement: capability validity, generation/lineage checks,
  VMA permissions, DMA/IOMMU scope, wait/wake transitions, scheduler dispatch,
  audit/debug gates, and commit points.
- The design aims to be useful to hyperscalers, federal/mission deployments,
  and open-source owner-controlled systems without changing the ISA.

## Repository Map

Core architecture docs:

- `capability_machine_one_pager.md`: concise project thesis.
- `design.md`: ISA and architectural contract.
- `hardware_design.md`: RTL-facing FPGA hardware sketch.
- `formal_theorems.md`: proof obligations and security theorems.
- `verification_plan.md`: directed tests and RTL simulation milestones.
- `conformance_matrix.md`: POSIX/libc/package status and open compatibility
  gaps.
- `psABI.md`: current process ABI, calling convention, signal frame, FDR
  inheritance, and loader boundaries.
- `object_format.md`: target static ELF/software-loader profile and exec-plan
  boundary.
- `libc_roadmap.md`: libc/runtime integration plan.
- `emulator_security_roadmap.md`: emulator security implementation roadmap.

Implementation:

- `src/isa.rs`: instruction and opcode definitions.
- `src/asm.rs`: assembler.
- `src/emulator.rs`: emulator runtime.
- `src/c_compiler.rs`: small C compiler.
- `demos/`: assembly and C demos.
- `userland/`: minimal bootable userland image.
- `third_party/`: real package smoke targets.
- `scripts/`: test, demo, userland, and package gates.

## Current Emulator Scope

Implemented areas include:

- 64-bit GPRs, FDRs, PCRs, FPU/vector register models.
- Integer ALU, branches, calls/returns, load/store memory, VMAs, data labels,
  strings, quads, and zero-fill.
- Hardware-style allocation through `ALLOC`, `ALLOC_EX`, `ALLOC_SIZE`, and
  `FREE`.
- FDR I/O and POSIX-shaped lowering for files, pipes, poll/select/epoll,
  signals, timers, process APIs, and libc allocation APIs.
- Ready-queue scheduling, futex parking/wake, signal delivery with `SIGRET`,
  IPC messages, Resource Domains, `OBJECT_CTL`, and `CALL_CAP`/`RET_CAP`.
- Current emulator `EXEC` loads assembly programs; the architectural target is
  loader-produced exec-plan descriptors, not hardware ELF parsing.
- Reserved low-level device/debug hooks exist, but ordinary device access is
  intended to flow through FDR capabilities, `MMAP`, DMA buffers, event objects,
  and service domains.

Current emulator `OBJECT_CTL` fields are a compact implementation ABI, not the
final typed control envelope:

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

## Build And Test

Run Rust tests:

```sh
cargo test
```

Run the full checked repository gate:

```sh
bash scripts/run_all_gates.sh
```

Build and run a demo:

```sh
cargo run -- cc demos/hello.c -o /tmp/hello.lnp64.s
cargo run -- run /tmp/hello.lnp64.s
```

Run demos and userland:

```sh
bash scripts/run_demos.sh
bash scripts/run_userland.sh
```

Run real-package gates:

```sh
bash scripts/run_real_packages.sh
bash scripts/run_sbase.sh
bash scripts/run_inih.sh
bash scripts/run_zlib.sh
bash scripts/run_natsort.sh
bash scripts/run_cwalk.sh
bash scripts/run_jsmn.sh
bash scripts/run_libc_test.sh
```

Docker:

```sh
docker build -t lnp64 .
docker run --rm lnp64
```

The container runs Rust tests and all demo programs.

## Development Notes

- Compatibility should grow through general compiler, libc, emulator, and
  service-boundary work, not package-specific rewrites.
- POSIX and Linux/BSD behavior are compatibility profiles over native
  capability/event/domain primitives.
- Attestation should prove measured artifacts and active policy; it should not
  become a vendor-only DRM path.
- The preferred high-level proof direction is a Lean-style abstract machine for
  capabilities, domains, VMAs, waitables, scheduler state, DMA, and architectural
  transitions. RTL assertions and model checking remain important for local
  FSM/refinement checks.
