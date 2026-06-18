# LNP64

LNP64 is a draft capability-machine architecture for system software. It keeps
ordinary computation as a conventional load/store CPU, while making files,
memory, synchronization, service calls, devices, scheduling, and isolation
hardware-visible capability objects.

This repository contains the architecture documents, Rust emulator, assembler,
bootstrap C compiler, libc/personality experiments, early LLVM target work, and
SystemVerilog/formal co-design work. It is an architecture research repo, not a
finished processor or production OS.

For the shortest project thesis, start with
`capability_machine_one_pager.md`.

## Architecture Summary

- **Capabilities everywhere:** file-descriptor registers, memory objects,
  queues, gates, DMA windows, domains, and services are unforgeable handles with
  generation checks.
- **Native resource operations:** `PULL`, `PUSH`, `AWAIT`, `MMAP`,
  `OBJECT_CTL`, `DOMAIN_CTL`, `GATE_CALL`, `CAP_*`, and `DMA_CTL` operate on
  hardware-visible objects rather than raw global namespaces.
- **Resource Domains:** containers, VMs, supervisors, sandboxes, assurance
  profiles, and cgroup-like limits are profiles of one nested containment
  primitive.
- **Services own policy:** filesystems, loaders, networking stacks, PCIe quirks,
  Unix compatibility, orchestration, and declassification stay in software
  services behind explicit capabilities.
- **Hardware owns enforcement:** capability validity, VMA permissions, DMA/IOMMU
  scope, scheduler state, wait/wake transitions, audit/debug gates, and commit
  points are enforced by hardware contracts.
- **Proof and RTL direction:** the long-term target is a complete simulatable
  SystemVerilog chip plus Lean proofs connected to RTL by schemas, assertions,
  typed traces, and refinement evidence. Real FPGA board evidence is a later
  bring-up step.

## Repository Map

Architecture and hardware:

- `design.md`: ISA and architectural contract.
- `hardware_design.md`: RTL-facing hardware design sketch.
- `formal_rtl_codesign_roadmap.md`: formal/RTL co-design plan, trust levels,
  proof coupling, and S0/M1+ work order.
- `formal_theorems.md`: top-level theorem and proof goals.
- `verification_plan.md`: software, RTL, proof, synthesis, and FPGA evidence
  gates.

Software compatibility:

- `conformance_matrix.md`: POSIX/libc/package status and compatibility gaps.
- `libc_roadmap.md`: libc/runtime integration plan.
- `netbsd_personality_abi.md`: NetBSD-like personality boundary and smoke gate.
- `psABI.md`: process ABI, calling convention, signal/gate frame, FDR
  inheritance, and loader boundary.
- `object_format.md`: static ELF/software-loader profile and exec-plan boundary.
- `toolchain_roadmap.md`: LLVM/Clang/lld and toy-compiler retirement plan.

Implementation:

- `src/isa.rs`: instruction and opcode definitions.
- `src/asm.rs`: assembler.
- `src/emulator.rs`: emulator runtime.
- `src/c_compiler.rs`: bootstrap C compiler.
- `llvm/lib/Target/LNP64/`: early LLVM target implementation.
- `rtl/`: SystemVerilog top, core tiles, engines, schema, and simulation
  slices.
- `formal/`: Lean models, proof manifests, coupling indexes, and RTL
  assertions.
- `fpga/`: early FPGA wrappers, constraints, and bring-up manifests.
- `demos/`, `userland/`, `third_party/`: smoke programs and package gates.
- `scripts/`: software, RTL, proof, synthesis, and board-gate runners.

## Current Status

The emulator and bootstrap compiler exercise many native concepts: FDRs, VMAs,
allocation, events, futex-like waits, gate/signal delivery, Resource Domains,
object profiles, sockets/endpoints, namespace dispatch, and NetBSD-personality
smoke paths.

The RTL/formal side is early but active. It includes an S0 whole-machine
skeleton, M1-M15 vertical slices, shared schema checks, Lean transition models,
RTL assertions, typed trace checks, randomized co-simulation, and synthesis/FPGA
smoke scaffolding. These are not yet a complete chip.

The current formal/RTL work order is deliberately narrow: finish the M1
capability/FDR authority slice to the first credible refinement shape before
starting another vertical proof slice. M1 must show the pattern from
schema-owned RTL commit/state records through Lean transition preservation,
executable pre/commit/post refinement comparison, bypass/mediation assertions, and honest trust
level accounting. Passing typed trace checks and assertions is useful evidence,
but it is not T4 RTL-to-Lean refinement by itself.

The bootstrap C compiler is temporary. The intended path is a real
LLVM/Clang/lld toolchain plus a software loader that emits hardware `EXEC` plan
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

## Common Gates

Use these from the repository root.

Host software:

```sh
cargo fmt --check
cargo test --quiet
bash scripts/run_demos.sh
git diff --check
```

Toolchain contracts:

```sh
bash scripts/run_real_llvm_tblgen_docker.sh
bash scripts/run_real_llvm_lnp64_docker.sh
bash scripts/run_toolchain_contracts.sh
```

`scripts/run_real_llvm_lnp64_docker.sh` is the active LLVM porting gate: it
builds upstream LLVM/Clang/lld in Docker with the in-tree LNP64 backend, links
real Clang objects, and executes the linked ELFs with `lnp64 run-elf`.

## RTL And Proof Gates

RTL/proof in Docker:

```sh
bash scripts/run_rtl_proof_docker.sh
bash scripts/run_rtl_synth_docker.sh
```

Focused RTL/proof loop:

```sh
bash scripts/run_rtl_s0.sh
bash scripts/run_rtl_m1.sh
scripts/check_rtl_shared_schema.py
scripts/check_rtl_typed_trace_contract.py
scripts/check_rtl_top_level_program_manifest.py
scripts/check_rtl_m1_typed_commit_trace.py
scripts/check_theorem_rtl_coupling.py
bash scripts/run_rtl_proof_gates.sh
bash scripts/run_rtl_synth_gates.sh
```

Board validation commands require compatible hardware. Until then, Dockerized RTL/proof and synthesis/FPGA-smoke gates are the reproducible evidence path.

## FPGA Board Note

No real FPGA is assumed to be available for routine work. Board validation
commands require compatible hardware and live UART evidence; until then,
Dockerized RTL/proof and synthesis/FPGA-smoke gates are the reproducible evidence path.

## Development Rules

Keep changes in the right layer:

- Architecture changes start in `design.md`, `hardware_design.md`,
  `formal_theorems.md`, or `psABI.md`.
- Software behavior changes update `src/`, demos/tests, conformance docs, and
  software gates together.
- RTL behavior changes update `rtl/include/`, `rtl/schema/`, `rtl/top/`,
  `rtl/core/`, `rtl/engines/`, `rtl/sim/`, assertions, manifests, and gates
  together.
- Formal claims update `formal_theorems.md`, Lean models, proof manifests,
  theorem/RTL coupling docs, and the checker that enforces the evidence.
- Compatibility work must lower onto native capability, object, event, domain,
  VMA, gate, and service primitives. Avoid POSIX/Linux-specific policy in
  shared hardware or emulator internals.

Important invariants:

- `rtl/top/lnp64_top.sv` is the real chip top. Testbench behavior belongs in
  `rtl/sim/`, bind modules, adapters, or scripts.
- `rtl/include/lnp64_pkg.sv` and `rtl/schema/lnp64_shared_schema.json` are a
  checked schema pair. Hardware-visible records, enums, opcodes, statuses,
  lifecycle states, and trace fields must update both.
- The emulator is the executable oracle until the Lean model is strong enough to
  generate authoritative traces. Emulator behavior must not become a private
  second ISA.
- Lean proofs, RTL assertions, schemas, manifests, and traces are one
  refinement chain. A theorem claim should name the RTL evidence or the known
  gap.

Before claiming an RTL/proof feature is integrated, it must be driven through
`lnp64_top`, have a manifest/checker entry, preserve the shared schema, and be
visible in the theorem/RTL coupling surface. Isolated demos are useful bring-up
tools, not completion evidence.

For a casual theorem-to-RTL review surface, start with
`formal/theorem_rtl_coupling_index.md`; it maps the main claims to Lean theorem
names, artifact levels, RTL modules, assertion files, trace markers, gates,
trust levels, and known gaps. Current rows remain T1 at the RTL-coupling level;
M1 now has a first explicit RTL-projection refinement-shape relation: the RTL
engine exports `typed_state_projection`, assertions consume that packed record,
and `scripts/check_rtl_m1_typed_commit_trace.py` checks every emitted state
projection field and packed commit/projection bit record against the transition
mirror. The Lean M1 model also names the packed commit/projection schema and
widths, and the checker compares those Lean schema mirrors to the shared RTL
schema. M1, M2, M4, M5, M7, and M14 now include T3 transition-invariant Lean
slices, but the claims are not T2/T4 refinement evidence until generated typed
transition traces and checked RTL-to-Lean refinement from emitted bits exist.
`scripts/check_theorem_rtl_coupling.py` verifies that index against the
machine-readable coupling manifest.

## Where To Go Next

- Read `formal_rtl_codesign_roadmap.md` before doing RTL or proof work. It
  defines the current work order and the M1 authority-refinement template.
- Read `formal/theorem_rtl_coupling_index.md` to see which theorem claims map to
  which RTL modules, assertions, traces, gates, trust levels, and known gaps.
- Read `verification_plan.md` for the current evidence matrix.
- Read `toolchain_roadmap.md` before extending LLVM, the loader, or the
  bootstrap compiler.
