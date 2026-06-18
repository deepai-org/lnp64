# LNP64

LNP64 is a draft capability-machine architecture for system software. It keeps
ordinary computation as a conventional load/store CPU, while making files,
memory, synchronization, service calls, devices, and isolation hardware-visible
capability objects.

This repository contains the design documents, Rust emulator, assembler, toy C
compiler, libc/personality experiments, and early SystemVerilog/formal
co-design work. It is an architecture research repo, not a finished processor
or production OS.

## Architecture Summary

- **FDR capabilities:** file-descriptor registers are unforgeable object
  handles. POSIX file descriptors are a compatibility view over these handles.
- **Native resource operations:** `PULL`, `PUSH`, `AWAIT`, `MMAP`, `OBJECT_CTL`,
  `DOMAIN_CTL`, `GATE_CALL`, `CAP_*`, and `DMA_CTL` operate on capabilities,
  waitables, domains, gates, and memory objects.
- **Resource Domains:** containers, VMs, cgroups, supervisors, sandboxes, and
  assurance profiles are profiles of one nested hardware containment primitive.
- **Services own policy:** filesystems, loaders, networking stacks, PCIe quirks,
  Unix compatibility, orchestration, and declassification stay in software
  services behind explicit capabilities.
- **Hardware owns enforcement:** capability validity, generation/lineage checks,
  VMA permissions, DMA/IOMMU scope, scheduler state, wait/wake transitions,
  audit/debug gates, and commit points.
- **Proof and RTL direction:** the long-term target is a complete simulatable
  SystemVerilog chip plus Lean proofs connected to the RTL by schemas,
  assertions, typed traces, and refinement evidence. Real FPGA board evidence is
  still a later hardware bring-up step.

For the concise project thesis, start with
`capability_machine_one_pager.md`.

## Repository Map

Architecture and hardware:

- `design.md`: ISA and architectural contract.
- `hardware_design.md`: RTL-facing hardware design sketch.
- `formal_rtl_codesign_roadmap.md`: formal/RTL co-design plan, proof coupling,
  trust levels, and S0/M1+ work order.
- `formal_theorems.md`: top-level theorem and proof goals.
- `verification_plan.md`: software, RTL, proof, synthesis, and FPGA evidence
  gates.

Software compatibility:

- `conformance_matrix.md`: POSIX/libc/package status and compatibility gaps.
- `libc_roadmap.md`: libc/runtime integration plan.
- `netbsd_personality_abi.md`: NetBSD-like personality boundary and smoke gate.
- `psABI.md`: process ABI, calling convention, signal frame, FDR inheritance,
  and loader boundary.
- `object_format.md`: static ELF/software-loader profile and exec-plan boundary.
- `toolchain_roadmap.md`: LLVM/Clang/lld and toy-compiler retirement plan.

Implementation:

- `src/isa.rs`: instruction and opcode definitions.
- `src/asm.rs`: assembler.
- `src/emulator.rs`: emulator runtime.
- `src/c_compiler.rs`: bootstrap C compiler.
- `rtl/`: SystemVerilog skeletons and milestone slices.
- `formal/`: Lean models, executable mirrors, and RTL assertions.
- `fpga/`: early FPGA wrappers, constraints, and bring-up manifests.
- `demos/`, `userland/`, `third_party/`: smoke programs and package gates.
- `scripts/`: software, RTL, proof, synthesis, and board-gate runners.

## Current Status

The emulator and toy compiler exercise many native concepts: FDRs, VMAs,
allocation, events, futex-like waits, signal/gate delivery, Resource Domains,
object profiles, sockets/endpoints, namespace dispatch, and NetBSD-personality
smoke paths.

The RTL/formal side is still early. It includes an S0 whole-machine skeleton and
M1-M15 vertical slices with Lean-style models, Python executable mirrors,
SystemVerilog modules, assertions, trace comparison, randomized co-simulation,
and synthesis/FPGA smoke scaffolding. These are not yet a complete chip.

The toy C compiler is a bootstrap tool. The intended direction is a real
LLVM/Clang/lld path plus a software loader that produces hardware `EXEC` plan
descriptors.

## Quick Start

Run the normal host software gate:

```sh
bash scripts/run_software_gates.sh
```

Run the full repository gate:

```sh
bash scripts/run_all_gates.sh
```

Run a small demo manually:

```sh
cargo run -- cc demos/hello.c -o /tmp/hello.lnp64.s
cargo run -- run /tmp/hello.lnp64.s
```

For faster repeated software runs:

```sh
cargo build --release
export LNP64_BIN="$PWD/target/release/lnp64"
bash scripts/run_demos.sh
bash scripts/run_userland.sh
bash scripts/run_real_packages.sh
```

## Working Commands

These commands are meant to be pasted from the repository root. For the normal
host-side hygiene pass before emulator, compiler, or libc commits:

```sh
cargo fmt --check
cargo test --quiet
bash scripts/run_demos.sh
git diff --check
rg "MSG_RECV|\\bPIPE\\b"
rg "EVENT_CTL|TIMER_CTL"
```

Use focused Rust filters while iterating, then run the full suite before a broad
commit:

```sh
cargo test --quiet loader::tests
cargo test --quiet classifier_
cargo test --quiet namespace_
cargo test --quiet signal_
cargo test --quiet object_ctl_
```

Useful one-off software smoke commands:

```sh
cargo run -- cc demos/hello.c -o /tmp/hello.lnp64.s
cargo run -- run /tmp/hello.lnp64.s
cargo run -- elf-plan /path/to/program.elf
```

The release-reuse path above is the fastest stable way to rerun scripts that
honor `LNP64_BIN`; unset it when intentionally testing the default
`cargo run --release` fallback path. `scripts/run_demos.sh` uses localhost for
network demos, so it needs free loopback ports and a shell with `/dev/tcp`
support. The `rg` alias scans should only produce documentation,
compatibility-lowering, or negative-assertion hits; new emulator/compiler
implementation hits are layering regressions to inspect before committing.

When unrelated files are already dirty, scope whitespace checks to the files you
touched:

```sh
git diff --check -- src/emulator.rs README.md
```

After heavy Rust gates, reclaim build artifacts with:

```sh
cargo clean
```

## RTL And Proof Gates

The reproducible RTL/proof paths use Docker so Lean, Verilator, Yosys, nextpnr,
and IceStorm versions do not depend on the host.

```sh
bash scripts/run_rtl_proof_docker.sh
bash scripts/run_rtl_synth_docker.sh
```

If the local host already has the required tools, the focused gates are:

```sh
bash scripts/run_rtl_s0.sh
bash scripts/run_rtl_m1.sh
bash scripts/check_theorem_rtl_coupling.py
bash scripts/run_rtl_random_cosim.sh
bash scripts/run_rtl_proof_gates.sh
bash scripts/run_rtl_synth_gates.sh
```

Individual RTL/model slices are available as `scripts/run_rtl_m2.sh` through
`scripts/run_rtl_m15.sh`. See `verification_plan.md` for the current evidence
matrix and expected gate coverage.

For a casual theorem-to-RTL review surface, start with
`formal/theorem_rtl_coupling_index.md`; it maps the main claims to Lean theorem
names, artifact levels, RTL modules, assertion files, trace markers, gates,
trust levels, and known gaps. The table keeps RTL modules, assertion files,
trace markers, gates, trust levels, and proof gaps visible together. Current
rows are T1 bounded witnesses or coverage artifacts, not T2/T3/T4 architectural or refinement proofs.
`scripts/check_theorem_rtl_coupling.py` verifies that index against the
machine-readable coupling manifest.

## FPGA Board Note

The repo contains an iCE40 S0 smoke path and board-evidence checker, but real
FPGA hardware is not assumed to be available. Board validation commands require
an attached compatible board and UART device. Until then, Dockerized RTL/proof and synthesis/FPGA-smoke gates are the reproducible evidence path.

## Development Notes

- Keep compatibility work layered over native capability/event/domain
  primitives.
- Avoid adding package-specific hacks to the compiler or emulator.
- Prefer typed object/control/domain terminology over POSIX-first internal
  names unless implementing an explicit compatibility surface.
- Treat Lean proofs, RTL, assertions, schemas, and traces as one refinement
  chain. A theorem is only useful if its assumptions and RTL evidence are
  visible.
- Before broad commits, run `git diff --check` and the smallest relevant gate,
  then the aggregate gate for the area touched.
