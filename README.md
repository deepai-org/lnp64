# LNP64

LNP64 is a draft capability-machine architecture for system software. It keeps
ordinary computation as a conventional load/store CPU, while making files,
memory, synchronization, service calls, devices, scheduling, and isolation
hardware-visible capability objects.

A formally verifiable, realtime-capable, capability-secure,
hardware-scheduled, hardware-allocated, service-oriented processor for
cloud-grade Unix-like systems.

Project keywords: capability-oriented, hardware-enforced, formally verifiable,
realtime-capable, scheduler-native, allocator-native, capability-secure,
domain-oriented, tenant-isolating, virtualization-native, container-native,
POSIX-projectable, attestable, observable, fault-contained, event-driven,
waitable-native, multicore-coherent, compiler-friendly, LLVM-targetable,
paravirtualization-friendly, servicelet-extensible, interrupt-abstracted,
bounded-latency.

Severe proof goals:

- **No forged authority:** capabilities, FDRs, returned capabilities, services,
  loaders, traces, and personalities cannot create, broaden, revive, or transfer
  authority outside the Capability Engine rules.
- **Domain confinement:** tenants, containers, VMs, services, drivers, and
  supervisors cannot observe or affect memory, devices, events, telemetry, or
  scheduler state outside delegated Resource Domain authority.
- **Memory and DMA authority:** loads, stores, VMAs, TLBs, caches, heap objects,
  device BARs, and DMA descriptors cannot access memory outside capability,
  VMA, IOMMU, and domain scope.
- **Scheduler and wait correctness:** every live thread has one scheduler state,
  waits cannot be lost, wakeups are generation-checked, and admitted work gets
  bounded service under the published realtime contract.
- **Bounded hardware behavior:** each instruction, engine command, servicelet,
  classifier action, queue operation, and fabric arbitration step either
  completes, parks on a valid waitable, submits bounded work, or fails closed.
- **Hardware totality and local progress:** every hardware FSM, pipeline, owner
  engine, queue, arbiter, decoder, reset path, and recovery path has only defined
  reachable states. Under its stated environment assumptions, each reachable
  nonterminal state must make bounded forward progress toward completion, apply
  bounded backpressure with a valid release condition, park on a valid
  wake/cancel/fault source, fail closed, reset locally, or escalate to a measured
  machine-fatal state. No hardware block may spin forever, wait on an impossible
  condition, hold unbounded backpressure, or require software intervention to
  escape an internal invalid state.
- **Fault containment and recovery:** malformed input, revocation races, ECC
  faults, watchdog events, service crashes, and local resets cannot publish
  partial authority or wedge the machine outside the bounded fault model.
- **Evidence honesty:** attestation, audit, debug, trace, observability counters,
  proof manifests, and compatibility personalities are data paths only; they
  cannot become hidden authority or bypass enforcement.

This repository contains the architecture documents, Rust emulator, assembler,
deprecated Rust bootstrap C compiler, libc/personality experiments, early LLVM
target work, and SystemVerilog/formal co-design work. It is an architecture
research repo, not a finished processor or production OS.

For the shortest project thesis, start with
`capability_machine_one_pager.md`.

## Architecture Summary

- **Capabilities everywhere:** file-descriptor registers, memory objects,
  queues, gates, DMA windows, domains, and services are unforgeable handles with
  generation checks.
- **Native resource operations:** `PULL`, `PUSH`, `WAITABLE_PROBE`, `AWAIT_EX`,
  `MMAP`, `OBJECT_CTL`, `DOMAIN_CTL`, `GATE_CALL`, `CAP_*`, and `DMA_CTL`
  operate on hardware-visible objects rather than raw global namespaces.
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
- `src/c_compiler.rs`: deprecated Rust bootstrap C compiler, retained only for
  legacy smoke generation behind `lnp64 cc --toy-bootstrap`.
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

The current formal/RTL work order is execution-first: make assembler and
LLVM-produced programs run through `rtl/top/lnp64_top.sv`, compare their retire
traces and architectural state against the emulator, and use that path as the
integration target for later proof slices. M1 capability/FDR refinement remains
the authority template, but it should become reachable through real retired
instructions rather than only isolated harness traces.

The bootstrap C compiler is temporary. The intended path is a real
LLVM/Clang/lld toolchain plus a software loader that emits hardware `EXEC` plan
descriptors.

Architectural opcodes must be compiler-visible. A native operation is not frozen
until the real LLVM MC layer can assemble, emit, disassemble, and expose it to C
runtime code through a target builtin, private `__lnp_*` intrinsic, or explicit
inline-assembly surface. POSIX-shaped helpers such as `poll`/`select`/`epoll`
remain libc/personality lowerings over native waitable operations.

## Quick Start

Run the normal host software gate:

```sh
bash scripts/run_software_gates.sh
```

Run the full repository gate:

```sh
bash scripts/run_all_gates.sh
```

Run a small demo through the legacy bootstrap compiler:

```sh
cargo run -- cc --toy-bootstrap demos/hello.c -o /tmp/hello.lnp64.s
cargo run -- run /tmp/hello.lnp64.s
```

The bootstrap compiler is deprecated. For real Clang/lld coverage, use the
Docker LLVM gate:

```sh
LNP64_LLVM_DOCKER_SKIP_BUILD=1 bash scripts/run_real_llvm_lnp64_docker.sh
```

For faster repeated software runs over the remaining legacy software gates and
the cached real LLVM package gate:

```sh
cargo build --release
export LNP64_BIN="$PWD/target/release/lnp64"
bash scripts/run_demos.sh
bash scripts/run_userland.sh
bash scripts/run_real_packages.sh
LNP64_LLVM_PACKAGE_FILTER=zlib bash scripts/run_real_llvm_package_gate.sh
LNP64_LLVM_PACKAGE_FILTER=sbase bash scripts/run_real_llvm_package_gate.sh
LNP64_LLVM_PACKAGE_FILTER=userland bash scripts/run_real_llvm_package_gate.sh
```

`scripts/run_demos.sh` now runs checked assembly demos by default. Migrated C
demos such as `hello`, `sqlite_lite`, `ping_pong`, `netcat`, and `httpd` are
covered by the real Clang/lld gate above. Use
`scripts/run_demos.sh --legacy-toy` only for the remaining toy-bootstrap C
personality smoke. `scripts/run_userland.sh` now defaults to the real Clang/lld
userland probes; use `scripts/run_userland.sh --legacy-toy` only for the old
host-directory fork/exec smoke. `scripts/run_real_packages.sh` and the
package-specific wrappers route package coverage through the same real LLVM
gate, reusing linked ELF artifacts under `target/llvm-lnp64-build` when they
already exist. `LNP64_LLVM_PACKAGE_FILTER` accepts `all`, `zlib`, `natsort`,
`jsmn`, `inih`, `cwalk`, `sbase`, `userland`, or a comma/space separated
subset.

## Common Gates

Use these from the repository root.

Host software:

```sh
cargo fmt --check
cargo test --quiet
bash scripts/run_demos.sh
git diff --check
```

`bash scripts/run_demos.sh` intentionally does not prove migrated C demo
coverage anymore; use `bash scripts/run_real_llvm_lnp64_docker.sh` for that.
Use `bash scripts/run_demos.sh --legacy-toy` only when checking the remaining
toy-bootstrap C personality smoke.

Toolchain contracts:

```sh
bash scripts/run_llvm_bootstrap_gates.sh --dry-run
bash scripts/run_real_llvm_tblgen_docker.sh
bash scripts/run_real_llvm_lnp64_mc_docker.sh
bash scripts/run_real_llvm_lnp64_docker.sh
bash scripts/run_toolchain_contracts.sh
```

`scripts/run_llvm_bootstrap_gates.sh --run` executes manifest rows already
marked `tested` and skips `planned` rows unless
`LNP64_RUN_PLANNED_LLVM_GATES=1` is set.
`scripts/run_real_llvm_lnp64_docker.sh` is the active LLVM porting gate: it
builds upstream LLVM/Clang/lld in Docker with the in-tree LNP64 backend, links
real Clang objects, and executes the linked ELFs with `lnp64 run-elf`.
`scripts/run_real_llvm_lnp64_mc_docker.sh` is the faster MC-only gate for
assembler, printer, encoding, and disassembler changes. It reuses the same
checkout/build directories but builds only `llvm-mc` and `llvm-objdump`.

For LLVM iteration, keep `target/llvm-project-src` and
`target/llvm-lnp64-build` when disk allows. The gate reuses both directories for
incremental rebuilds; deleting them turns the next run into a cold LLVM build.
The script auto-selects Ninja parallelism capped at 16 jobs, and you can
override it explicitly:

```sh
LNP64_LLVM_JOBS=16 bash scripts/run_real_llvm_lnp64_docker.sh
```

After the Docker image exists and `Dockerfile.llvm` has not changed, skip the
image build prelude during tight LLVM loops:

```sh
LNP64_LLVM_DOCKER_SKIP_BUILD=1 LNP64_LLVM_JOBS=16 bash scripts/run_real_llvm_lnp64_docker.sh
LNP64_LLVM_DOCKER_SKIP_BUILD=1 bash scripts/run_real_llvm_lnp64_mc_docker.sh
```

When iterating only on LLVM compile/link behavior, skip the host `run-elf`
execution pass too. The full wrapper still builds the `lnp64` host binary once
and reuses it for all execution probes, but this shortens backend-only loops:

```sh
LNP64_LLVM_DOCKER_SKIP_BUILD=1 LNP64_LLVM_DOCKER_SKIP_RUN_ELF=1 LNP64_LLVM_JOBS=16 bash scripts/run_real_llvm_lnp64_docker.sh
```

Only remove the LLVM cache when you need the space or want to force a clean
checkout:

```sh
rm -rf target/llvm-project-src target/llvm-lnp64-build
```

## RTL And Proof Gates

RTL/proof in Docker:

```sh
bash scripts/run_rtl_proof_docker.sh
bash scripts/run_rtl_synth_docker.sh
```

Faster RTL/proof iteration in Docker:

```sh
bash scripts/run_rtl_quick_docker.sh top tests/rtl/programs/top_smoke.s
bash scripts/run_rtl_quick_docker.sh top tests/rtl/programs/top_immediate_alu.s
LNP64_RTL_TOP_PROGRAM_FILTER='*linked*' bash scripts/run_rtl_quick_docker.sh top
bash scripts/run_rtl_quick_docker.sh cosim
bash scripts/run_rtl_quick_docker.sh cosim m1 m7
bash scripts/run_rtl_quick_docker.sh proof
LNP64_RTL_TOP_PROGRAM_FILTER='top_*alu* demos/env_get.s' bash scripts/run_rtl_quick_docker.sh top
bash scripts/run_rtl_execution_fast_docker.sh tests/rtl/programs/top_smoke.s
LNP64_RTL_SKIP_BUILD=1 bash scripts/run_rtl_execution_fast_docker.sh tests/rtl/programs/top_immediate_alu.s
LNP64_RTL_TOP_PROGRAM_FILTER='*linked*' bash scripts/run_rtl_execution_fast_docker.sh
LNP64_RTL_FAST=1 bash scripts/run_rtl_proof_docker.sh
LNP64_RTL_FAST=1 bash scripts/run_rtl_m1_refinement_docker.sh
LNP64_RTL_PROOF_SKIP_BUILD=1 LNP64_RTL_FAST=1 LNP64_M1_TYPED_COMMIT_SEEDS="0 1 7" bash scripts/run_rtl_m1_refinement_docker.sh
LNP64_RTL_FAST=1 \
LNP64_COSIM_SEEDS=0 \
LNP64_M1_TYPED_COMMIT_SEEDS=0 \
LNP64_RTL_PROOF_SKIP_BUILD=1 \
LNP64_RTL_EXEC_SKIP_BUILD=1 \
LNP64_RTL_TOP_PROGRAM_QUIET=1 \
LNP64_RTL_TOP_PROGRAM_JOBS=auto \
  bash scripts/run_rtl_m1_refinement_docker.sh
LNP64_RTL_PROOF_RANDOM_COSIM=0 bash scripts/run_rtl_proof_docker.sh
LNP64_RTL_PROOF_SKIP_BUILD=1 LNP64_RTL_PROOF_RANDOM_COSIM=0 bash scripts/run_rtl_proof_docker.sh
LNP64_RTL_PROOF_SKIP_BUILD=1 LNP64_RTL_RANDOM_COSIM_JOBS=4 bash scripts/run_rtl_proof_docker.sh
LNP64_RTL_FAST=1 LNP64_RTL_PROOF_GATE_JOBS=auto bash scripts/run_rtl_proof_docker.sh
LNP64_RTL_EXEC_REBUILD_IMAGE=1 bash scripts/run_rtl_execution_fast_docker.sh tests/rtl/programs/top_smoke.s
LNP64_RTL_PROOF_REBUILD_IMAGE=1 bash scripts/run_rtl_proof_docker.sh
```

The Docker RTL/proof wrappers now reuse an existing tool image by default and
mount the checkout so Cargo and Verilator artifacts persist under `target/`.
Set `LNP64_RTL_EXEC_REBUILD_IMAGE=1` or `LNP64_RTL_PROOF_REBUILD_IMAGE=1` only
after changing a Dockerfile or base tool dependency. Set
`LNP64_RTL_PROOF_BUILD_GATES=1` only when you also want the full proof gate to
run during `docker build`. `LNP64_RTL_FAST=1` is the tight iteration profile: it
reuses Verilator build products under `target/rtl-verilator`, skips the
separate lint-only pass, uses parallel Verilator builds by default, reduces
default M1 typed-commit seeds to `0`, and skips the long randomized/cosim sweep
unless you turn it back on. For current execution-first RTL work, start with
`run_rtl_quick_docker.sh top` or `run_rtl_execution_fast_docker.sh`; for M1
authority-refinement work, use `run_rtl_m1_refinement_docker.sh` and widen
`LNP64_M1_TYPED_COMMIT_SEEDS` before treating the result as full evidence.
`run_rtl_execution_fast_docker.sh` is the Dockerized inner loop for the current
execution-first milestone: it reuses or builds a Rust+Verilator image, mounts
the checkout, keeps Docker-built Cargo and Verilator artifacts under
`target/docker-rust` and `target/rtl-verilator-docker`, and runs selected
programs through `rtl/top/lnp64_top.sv` against the emulator.
`run_rtl_m1_refinement_docker.sh` sets `LNP64_REQUIRE_LEAN=1`, so it is the
right fast path when the claim depends on the Lean M1 transition-invariant file;
the non-Docker host gate may skip Lean if no local Lean toolchain is installed.
After the Lean/standalone M1 gate passes, the Docker wrapper also runs the
top-level M1 capability program subset through `lnp64_top` with the same
retire-trace/final-state comparator. Set `LNP64_M1_REFINEMENT_SKIP_TOP=1` only
when intentionally debugging the standalone proof harness, or narrow
`LNP64_M1_REFINEMENT_TOP_FILTER` to one top-level cap program.
`run_rtl_quick_docker.sh` is the shortest day-to-day wrapper. It uses the repo
Dockerfiles, reuses existing Docker images unless
`LNP64_RTL_QUICK_REBUILD_IMAGE=1` is set, keeps Docker-side Cargo and Verilator
artifacts in `target/docker-rust` and `target/rtl-verilator-docker`, enables
`LNP64_RTL_FAST=1`, skips standalone lint, runs top-level program tests with
parallel jobs, and narrows `cosim` mode to M1 seed 0 unless you pass gate names
or set `LNP64_RTL_RANDOM_COSIM_GATES`/`LNP64_COSIM_SEEDS`. Use the older
wrappers or unset the narrowing variables for full evidence runs.
Reusable Verilator builds automatically retry once with a clean object
directory if generated make dependencies point at a stale host/container
Verilator install.
For proof-gate iteration, `LNP64_RTL_FAST=1` also defaults
`LNP64_RTL_PROOF_GATE_JOBS` to `auto`, so the independent M2-M6 and M8-M15
Verilator/cosim gates run in parallel after the special M1/M7 typed-commit
checks. Set `LNP64_RTL_PROOF_GATE_JOBS=1` for stable serial logs, or
`LNP64_RTL_PROOF_KEEP_GATE_LOGS=1` to keep temporary per-gate logs.

The randomized/cosim sweep is serial and full-seed by default for stable logs.
For an inner loop, run only the slices and seeds you need:

```sh
LNP64_RTL_FAST=1 LNP64_RTL_RANDOM_COSIM_GATES="m1 m7" bash scripts/run_rtl_random_cosim.sh
LNP64_RTL_FAST=1 LNP64_RTL_SKIP_BUILD=1 LNP64_RTL_RANDOM_COSIM_GATES="m1 m7" bash scripts/run_rtl_random_cosim.sh
LNP64_RTL_REUSE_BUILD=1 LNP64_RTL_SKIP_LINT=1 LNP64_RTL_BUILD_ROOT="$PWD/target/rtl-verilator" bash scripts/run_rtl_s0.sh
LNP64_RTL_REUSE_BUILD=1 LNP64_RTL_SKIP_LINT=1 LNP64_RTL_SKIP_BUILD=1 LNP64_RTL_BUILD_ROOT="$PWD/target/rtl-verilator" bash scripts/run_rtl_s0.sh
LNP64_RTL_FAST=1 LNP64_COSIM_SEEDS="0 1 7" LNP64_RTL_RANDOM_COSIM_JOBS=auto bash scripts/run_rtl_random_cosim.sh
LNP64_RTL_REUSE_BUILD=1 LNP64_RTL_SKIP_LINT=1 LNP64_RTL_COSIM_SEED_JOBS=auto LNP64_COSIM_SEEDS="0 1 7 42" bash scripts/run_rtl_m1.sh
LNP64_RTL_PROOF_SKIP_BUILD=1 LNP64_RTL_SKIP_BUILD=1 LNP64_RTL_COSIM_SEED_JOBS=4 LNP64_COSIM_SEEDS="0 1 7 42" bash scripts/run_rtl_proof_docker.sh
```

Set `LNP64_RTL_RANDOM_COSIM_JOBS=4` or
`LNP64_RTL_RANDOM_COSIM_JOBS=auto` to run independent M1-M15 randomized/cosim
gates in parallel; the runner starts a replacement gate as soon as any active
gate finishes, so long-running gates no longer stall a whole fixed batch. Each
gate writes its own temporary log and failures replay that log. Set
`LNP64_RTL_COSIM_SEED_JOBS=4` or `auto` when one gate is running many seeds;
this parallelizes the per-seed model/RTL trace comparisons inside
`run_rtl_m1.sh` through `run_rtl_m15.sh`. The seed runner starts a replacement
seed as soon as any active seed finishes, so uneven seed runtimes do not stall a
whole fixed batch. After a successful build, add `LNP64_RTL_SKIP_BUILD=1` to
reuse the existing S0/M1-M15 Verilator binaries and rerun only model
generation, RTL simulation, trace extraction, and diffs. Remove
`LNP64_RTL_FAST=1`, `LNP64_RTL_SKIP_BUILD=1`, seed/gate narrowing, and restore
the full seed list before using randomized/cosim output as broad evidence.

The full RTL/proof gate avoids rerunning the M1 and M7 Verilator builds solely
for typed-checker parsing: it tees each gate log once and then runs the matching
typed checker with `LNP64_M1_TYPED_COMMIT_USE_EXISTING=1` or
`LNP64_M7_TYPED_COMMIT_USE_EXISTING=1`. For manual debugging, the same pattern
works directly:

```sh
bash scripts/run_rtl_m7.sh | tee /tmp/lnp64_rtl_m7_debug.log
LNP64_M7_TYPED_COMMIT_USE_EXISTING=1 LNP64_M7_TYPED_COMMIT_LOG=/tmp/lnp64_rtl_m7_debug.log scripts/check_rtl_m7_typed_commit_trace.py
```

Focused RTL/proof loop:

```sh
bash scripts/run_rtl_execution_fast.sh tests/rtl/programs/top_smoke.s
LNP64_RTL_SKIP_BUILD=1 bash scripts/run_rtl_execution_fast.sh tests/rtl/programs/top_immediate_alu.s
bash scripts/run_rtl_s0.sh
bash scripts/run_rtl_top_program_manifest.sh
bash scripts/run_rtl_top_program_smoke.sh
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_smoke.s
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_unsupported_opcode.hex
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_immediate_alu.s
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_extend.s
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_count_rotate_bswap.s
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_cmpu_csel.s
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_cset.s
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_mulh.s
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_auipc_fence.s
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_half_word_load_store.s
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_amo.s
bash scripts/run_rtl_top_llvm_mc_smoke.sh
bash scripts/run_rtl_top_clang_smoke.sh
bash scripts/run_rtl_top_linked_llvm_smoke.sh
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_return_12.c
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_branch_if.c
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_loop_sum.c
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_factorial_mul.c
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_subtract.c
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_bitwise.c
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_shift.c
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_udiv_urem.c
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_signed_division.c
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_not.c
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_call_return.c
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_byte_array.c
bash scripts/run_rtl_top_program_smoke.sh demos/ping_pong.c
bash scripts/run_rtl_top_program_smoke.sh demos/allocator.c
bash scripts/run_rtl_top_program_smoke.sh demos/allocator_native.s
bash scripts/run_rtl_top_program_smoke.sh demos/env_get.s
bash scripts/run_rtl_top_program_smoke.sh demos/exec_target.s
bash scripts/run_rtl_top_program_smoke.sh demos/dma_copy.s
bash scripts/run_rtl_top_program_smoke.sh demos/revoked_dma_buffer.s
bash scripts/run_rtl_top_program_smoke.sh demos/guarded_heap_overflow.s
bash scripts/run_rtl_top_program_smoke.sh demos/factorial.c
bash scripts/run_rtl_top_program_smoke.sh demos/hello.c
bash scripts/run_rtl_top_program_smoke.sh demos/memory_order.s
cargo run -- asm-flat-exec tests/rtl/programs/top_smoke.s -o /tmp/top_smoke.hex
bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_smoke.hex
bash scripts/run_rtl_m1.sh
scripts/run_rtl_m1_refinement_gate.sh
scripts/check_rtl_shared_schema.py
scripts/check_rtl_typed_trace_contract.py
scripts/check_rtl_top_level_program_manifest.py
scripts/check_rtl_m1_typed_commit_trace.py
scripts/check_rtl_m7_typed_commit_trace.py
scripts/check_theorem_rtl_coupling.py
bash scripts/run_rtl_proof_gates.sh
bash scripts/run_rtl_synth_gates.sh
```

The current execution-first top-level smoke that exercises a compiler-generated
two-thread queue path is:

```sh
LNP64_RTL_TOP_PROGRAM_QUIET=1 LNP64_RTL_TOP_PROGRAM_MAX_CYCLES=50000 bash scripts/run_rtl_top_program_smoke.sh demos/ping_pong.c
```

After one normal top-program run has rebuilt the Verilator binary, reuse it for
fast checks:

```sh
LNP64_RTL_TOP_PROGRAM_QUIET=1 LNP64_RTL_TOP_PROGRAM_SKIP_BUILD=1 LNP64_RTL_TOP_PROGRAM_MAX_CYCLES=5000 bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_smoke.s
LNP64_RTL_TOP_PROGRAM_QUIET=1 LNP64_RTL_TOP_PROGRAM_SKIP_BUILD=1 LNP64_RTL_TOP_PROGRAM_MAX_CYCLES=5000 bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_call_return.c
```

`scripts/run_rtl_top_program_manifest.sh` builds the top-level Verilator
program test once, then reuses that binary for the remaining selected program
images. The full manifest includes LLVM MC and clang object-byte smokes, so run
the LLVM Docker gate first or set `LLVM_MC`/`LLVM_CLANG`/`LLVM_OBJDUMP` when
using a non-default tool path. For a manual multi-program loop, run one normal
smoke first and then reuse the binary explicitly:

```sh
LNP64_RTL_VERILATOR_BUILD_JOBS=auto LNP64_RTL_BUILD_ROOT="$PWD/target/rtl-verilator" bash scripts/run_rtl_top_program_manifest.sh
LNP64_RTL_FAST=1 LNP64_RTL_TOP_PROGRAM_JOBS=auto bash scripts/run_rtl_top_program_manifest.sh
LNP64_RTL_FAST=1 LNP64_RTL_TOP_PROGRAM_TILE_COUNT=2 LNP64_RTL_REUSE_BUILD=1 LNP64_RTL_TOP_PROGRAM_SKIP_BUILD=1 LNP64_RTL_BUILD_ROOT="$PWD/target/rtl-verilator" LNP64_RTL_TOP_PROGRAM_QUIET=1 LNP64_RTL_TOP_PROGRAM_JOBS=auto bash scripts/run_rtl_top_program_manifest.sh
LNP64_RTL_REUSE_BUILD=1 LNP64_RTL_SKIP_LINT=1 LNP64_RTL_TOP_PROGRAM_JOBS=4 LNP64_RTL_BUILD_ROOT="$PWD/target/rtl-verilator" bash scripts/run_rtl_top_program_manifest.sh
LNP64_RTL_FAST=1 LNP64_RTL_REUSE_BUILD=1 LNP64_RTL_TOP_PROGRAM_SKIP_BUILD=1 LNP64_RTL_TOP_PROGRAM_JOBS=4 LNP64_RTL_BUILD_ROOT="$PWD/target/rtl-verilator" bash scripts/run_rtl_top_program_manifest.sh
LNP64_RTL_FAST=1 LNP64_RTL_TOP_PROGRAM_FILTER='*linked*' bash scripts/run_rtl_top_program_manifest.sh
LNP64_RTL_FAST=1 LNP64_RTL_TOP_PROGRAM_FILTER='demos/*.s top_heap_byte_lanes.c' bash scripts/run_rtl_top_program_manifest.sh
LNP64_RTL_FAST=1 LNP64_RTL_TOP_PROGRAM_FILTER='top_dma_revoke_stale.s' bash scripts/run_rtl_top_program_manifest.sh
LNP64_RTL_TOP_PROGRAM_TILE_COUNT=4 LNP64_RTL_TOP_PROGRAM_QUIET=1 LNP64_RTL_REUSE_BUILD=0 LNP64_RTL_BUILD_ROOT="$PWD/target/rtl-verilator" bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_cap_transfer_valid.s
LNP64_RTL_TOP_PROGRAM_QUIET=1 LNP64_RTL_REUSE_BUILD=1 LNP64_RTL_TOP_PROGRAM_SKIP_BUILD=1 LNP64_RTL_BUILD_ROOT="$PWD/target/rtl-verilator" bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_dma_revoke_stale.s
LNP64_RTL_REUSE_BUILD=1 LNP64_RTL_TOP_PROGRAM_SKIP_BUILD=1 LNP64_RTL_BUILD_ROOT="$PWD/target/rtl-verilator" bash scripts/run_rtl_top_program_manifest.sh tests/rtl/programs/top_linked_high_mul.c
LNP64_RTL_FAST=1 LNP64_RTL_TOP_PROGRAM_FILTER='top_linked_clone_join.c' bash scripts/run_rtl_top_program_manifest.sh
LNP64_RTL_REUSE_BUILD=1 LNP64_RTL_BUILD_ROOT="$PWD/target/rtl-verilator" bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_smoke.s
LNP64_RTL_REUSE_BUILD=1 LNP64_RTL_TOP_PROGRAM_SKIP_BUILD=1 LNP64_RTL_BUILD_ROOT="$PWD/target/rtl-verilator" bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_immediate_alu.s
LNP64_RTL_REUSE_BUILD=1 LNP64_RTL_SKIP_BUILD=1 LNP64_RTL_BUILD_ROOT="$PWD/target/rtl-verilator" bash scripts/run_rtl_top_program_smoke.sh tests/rtl/programs/top_extend.s
LNP64_RTL_REUSE_BUILD=1 LNP64_RTL_TOP_PROGRAM_SKIP_BUILD=1 LNP64_RTL_BUILD_ROOT="$PWD/target/rtl-verilator" bash scripts/run_rtl_top_llvm_mc_smoke.sh
LNP64_RTL_REUSE_BUILD=1 LNP64_RTL_TOP_PROGRAM_SKIP_BUILD=1 LNP64_RTL_BUILD_ROOT="$PWD/target/rtl-verilator" bash scripts/run_rtl_top_clang_smoke.sh
LNP64_RTL_REUSE_BUILD=1 LNP64_RTL_TOP_PROGRAM_SKIP_BUILD=1 LNP64_RTL_BUILD_ROOT="$PWD/target/rtl-verilator" bash scripts/run_rtl_top_linked_llvm_smoke.sh
```

The third command above is the current full active top-level manifest loop: it
reuses the 2-tile Verilator build and has been used to run 83 active assembly,
LLVM MC, clang, linked LLVM, demo, and compiler-generated programs through
`lnp64_top` against the emulator comparator. Use
`LNP64_RTL_TOP_PROGRAM_TILE_COUNT=4` for the supported four-tile stress shape;
tile-count-specific builds live in separate Verilator object directories.

Set `LNP64_RTL_TOP_PROGRAM_JOBS=4` or
`LNP64_RTL_TOP_PROGRAM_JOBS=auto` to run the remaining top-level program images
in parallel after the first program has built and checked the shared Verilator
binary. `LNP64_RTL_FAST=1` defaults this manifest runner to `auto`; leave the
variable unset for serial logs. `LNP64_RTL_TOP_PROGRAM_FILTER` accepts
space/comma-separated glob or substring patterns and keeps manifest gate
dispatch, so linked LLVM entries still run through
`scripts/run_rtl_top_linked_llvm_smoke.sh`. Explicit source arguments also use
the manifest gate when the source is active; unknown explicit paths fall back to
the generic top-program smoke gate. `LNP64_RTL_FAST=1` also makes individual
top-program smokes quiet by default: successful runs keep full RTL/emulator logs
for comparison but only print the final pass line. Set
`LNP64_RTL_TOP_PROGRAM_QUIET=0` when you need live retire-record output.
Parallel top-program workers build their own program images outside the shared
Verilator build lock; the lock only protects the reusable
`lnp64_top_program_tb` binary. This keeps assembler/C-to-hex prep concurrent
when `LNP64_RTL_TOP_PROGRAM_JOBS` is greater than one.

Individual top-level program smokes default to `10000` cycles, matching the
manifest runner. For longer exploratory programs, raise the simulation retire
limit without changing the RTL testbench:

```sh
LNP64_RTL_TOP_PROGRAM_MAX_CYCLES=2000 bash scripts/run_rtl_top_program_smoke.sh demos/rot13.c
LNP64_RTL_TOP_PROGRAM_MAX_CYCLES=10000 bash scripts/run_rtl_top_program_smoke.sh demos/json_parser.c
```

The top-level program manifest runner uses the same `10000`-cycle default so
longer active compiler-generated demos stay in the recurring gate.

`scripts/run_rtl_top_linked_llvm_smoke.sh` is the first narrow linked-ELF
top-level RTL gate. It builds a clang object, links it with LNP64 lld using a
flat-compatible linker script, validates the existing software-loader
`elf-plan`, exports the ELF through `lnp64 elf-flat-exec`, and feeds the result
to the same RTL/emulator retire-trace comparator. When a source provides
`main` instead of `_start`, the script assembles a tiny flat startup object with
LLVM MC that calls `main` and exits with `r1`. Linked data images are passed as
top-level SRAM data hex when present. It is not a full VMA/MMU loader in RTL yet;
non-flat ELF layouts intentionally fail at export time.

`LNP64_RTL_VERILATOR_BUILD_JOBS=auto` uses the host CPU count for the Verilator
build step; set it to a fixed number such as `4` on shared machines. The
top-program smoke script locks the shared build directory before preparing or
compiling it, so parallel ad hoc probes do not corrupt the reusable Verilator
object tree.

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
slices. M1 also has a narrow executable RTL-to-Lean-shaped check from emitted
packed commit/projection bits, but it is still not T2/T4 refinement evidence until the
schema is generated or otherwise mechanically owned and the bit-level
RTL-to-Lean refinement is formal/proven rather than Python-mirrored.
M7 now has a first narrow typed scheduler/wakeup commit checker for the seed-0
Lean transition shape, but the later slices still need the same discipline
generalized and composed.
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
