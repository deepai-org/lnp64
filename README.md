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

## Long-Term Repo Structure

This repository will stay usable only if every change has a clear layer and a
clear evidence path. New contributors should start by deciding which contract
they are changing:

- **Architecture contract:** edit `design.md`, `hardware_design.md`,
  `formal_theorems.md`, or `psABI.md`. Do not change emulator or RTL behavior
  first and document it later.
- **Software execution model:** edit `src/`, `demos/`, `userland/`, libc and
  personality docs, and the relevant software gates.
- **RTL implementation:** edit `rtl/include/`, `rtl/top/`, `rtl/core/`,
  `rtl/engines/`, `rtl/sim/`, and the RTL manifests/gates together.
- **Proof/evidence:** edit `formal/`, `formal/rtl_assertions/`,
  `formal/theorem_rtl_coupling_*`, and proof manifests together.
- **Board/synthesis:** edit `fpga/`, synthesis scripts, constraints, Docker
  images, and board-evidence manifests together.

Keep these boundaries intact:

- `rtl/top/lnp64_top.sv` is the real chip top. Testbench-only behavior belongs
  in `rtl/sim/`, bind modules, adapters, or scripts.
- `rtl/include/lnp64_pkg.sv` and `rtl/schema/lnp64_shared_schema.json` are a
  checked schema pair. Any record, enum, opcode, status, lifecycle, or trace
  field change must update both and pass the schema gate.
- The emulator is the executable oracle until the Lean model is strong enough
  to generate authoritative traces. Emulator behavior should not become a
  private second ISA.
- Lean proofs, RTL assertions, manifests, and trace checks are one refinement
  chain. Do not add a theorem claim without naming the RTL evidence or known
  gap.
- Compatibility layers must lower onto native capability, object, event,
  domain, VMA, gate, and service primitives. Avoid POSIX/Linux-specific logic in
  shared hardware or emulator internals unless the file is explicitly a
  compatibility surface.

Use this placement rule:

- new opcodes/profiles: `design.md`, `src/isa.rs`, assembler/compiler lowering,
  emulator, schema/RTL package if hardware-visible, demos/tests.
- new hardware record fields: schema JSON, `lnp64_pkg.sv`, RTL users,
  emulator/co-sim records, Lean/Python model, docs.
- new RTL block behavior: roadmap entry, block RTL, top-level wiring through
  `lnp64_top`, simulation gate, manifest entry, assertion/proof hook.
- new formal claim: `formal_theorems.md`, Lean model, proof manifest,
  theorem-to-RTL coupling index, and the gate that checks the evidence.
- new compatibility behavior: conformance matrix, libc/personality docs,
  lowering/runtime code, demo or package gate.

Before a broad change, check the affected layer locally:

```sh
cargo test --quiet
bash scripts/run_demos.sh
scripts/check_rtl_shared_schema.py
scripts/check_theorem_rtl_coupling.py
bash scripts/run_rtl_s0.sh
git diff --check
```

Before claiming an RTL/proof feature is integrated, it must be driven through
`lnp64_top`, have a manifest/checker entry, preserve the shared schema, and be
visible in the theorem/RTL coupling surface. Isolated demos are useful bring-up
tools, not completion evidence.

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

The command set above is the currently exercised full software hygiene pass:
`cargo test --quiet` runs the Rust unit/integration suite, `run_demos.sh` covers
the demo programs, `git diff --check` catches whitespace damage, and the two
`rg` scans are manual layering audits. Treat `MSG_RECV`, bare `PIPE`,
`EVENT_CTL`, and `TIMER_CTL` hits as acceptable only when they are docs,
compatibility-lowering names, or negative assertions.

For toolchain-contract changes, these focused commands have been working:

```sh
bash scripts/run_real_llvm_tblgen_docker.sh
bash scripts/run_real_llvm_lnp64_docker.sh
bash scripts/run_toolchain_contracts.sh
cargo test --quiet toolchain_contract_index_is_complete
cargo test --quiet llvm_gate_manifest_pins_non_toy_clang_commands
cargo test --quiet clang_driver_manifest_matches_llvm_gates
cargo test --quiet llvm_filemap_manifest_names_backend_source_surface
cargo test --quiet libc_shim_manifest_covers_runtime_surfaces
cargo test --quiet loader_security_manifest_covers_exec_plan_security
cargo test --quiet netbsd_layers_manifest_preserves_personality_order
cargo test --quiet conformance_gate_manifest_covers_required_layers
```

The focused manifest tests are useful while editing one contract file at a
time. Before committing a toolchain contract batch, also run
`bash scripts/run_toolchain_contracts.sh`; it checks the manifest index rather
than only the Rust unit that was under edit.

`scripts/run_real_llvm_tblgen_docker.sh` builds the checked Docker image from
`Dockerfile.llvm` and runs real `llvm-tblgen` against the LNP64 TableGen target
files. Its generated includes land under `target/real-llvm-tblgen`.

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

When measuring actual script behavior instead of compile time, build once and
pin the binary used by the shell gates:

```sh
cargo build --release
export LNP64_BIN="$PWD/target/release/lnp64"
bash scripts/run_demos.sh
bash scripts/run_userland.sh
bash scripts/run_real_packages.sh
```

Known working focused checks for recent emulator, loader, classifier, and
toolchain-manifest work:

```sh
cargo test --quiet classifier_
cargo test --quiet loader::tests
cargo test --quiet exec_descriptor
cargo test --quiet run_elf_probe_loads_and_commits_minimal_static_elf
cargo test --quiet run_elf_manifest_records_execution_boundary
cargo test --quiet inline_asm_manifest_records_backend_constraints
bash scripts/run_toolchain_contracts.sh
```

For compatibility-lowering and native-primitive naming work, these focused
manifest tests are the shortest useful loop before the full suite:

```sh
cargo test --quiet compatibility_table_names_native_primitives
cargo test --quiet compatibility_lowering_pins_native_architecture_boundaries
cargo test --quiet compatibility_surfaces_have_layer_policy
cargo test --quiet intrinsic_manifest_matches_target_manifest
cargo test --quiet intrinsic_header_matches_intrinsic_manifest
```

For classifier-table routing changes, use the broad classifier filter while
iterating. It covers IPC service routing, packet port/subnet/hash routing,
fallback, malformed packets, unauthorized and stale capabilities, queue wakeup,
and classifier counters:

```sh
cargo test --quiet classifier_
```

For the current conformance-gate and README/doc batches, this narrower
dirty-worktree check has been useful before staging:

```sh
cargo fmt --check
cargo test --quiet conformance_gate_manifest_covers_required_layers
cargo test --quiet toolchain_contract_index_is_complete
git diff --check -- README.md src/lowering.rs conformance_matrix.md toolchain_roadmap.md toolchain/lnp64_conformance_gates.manifest toolchain/lnp64_contracts.manifest toolchain/lnp64_target.manifest toolchain/lnp64_transition.manifest
```

`elf-plan` parses a static LNP64 ELF image, applies supported relocations,
materializes VMA bytes, builds encoded exec-plan records, and runs the
emulator-side descriptor validator. It is an inspection/validation command, not
yet a full process replacement path for ELF binaries.

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

The currently working proof-container command used by the RTL/proof gate is:

```sh
docker run --rm -e LNP64_REQUIRE_LEAN=1 -v "$PWD:/work" -w /work lnp64-rtl-proof bash scripts/run_rtl_proof_gates.sh
```

The focused Docker loop for the first SG-AUTH capability/FDR transition slice is:

```sh
docker run --rm -v "$PWD:/work" -w /work lnp64-rtl-proof lean formal/M1TransitionInvariantModel.lean
docker run --rm -v "$PWD:/work" -w /work lnp64-rtl-proof scripts/check_rtl_m1_typed_commit_trace.py
```

If the local host already has the required tools, the focused gates are:

```sh
bash scripts/run_rtl_s0.sh
scripts/check_rtl_s0_contract.py
scripts/check_rtl_typed_trace_contract.py
bash scripts/run_rtl_m1.sh
scripts/check_rtl_m1_typed_commit_trace.py
scripts/check_rtl_shared_schema.py
scripts/check_rtl_top_level_program_manifest.py
scripts/check_theorem_rtl_coupling.py
bash scripts/run_rtl_random_cosim.sh
bash scripts/run_rtl_proof_gates.sh
bash scripts/run_rtl_synth_gates.sh
```

`scripts/run_rtl_s0.sh` now elaborates `lnp64_top` with the default
`CORE_TILE_COUNT=2` and a supported `CORE_TILE_COUNT=4` stress instance. The S0
test requires tile 0 to run PID 1, tile 1 to be reset-stable and scheduler
observable, `ENV_GET` to report tile count/mask/coherence-domain/active-window
topology, cross-tile wake delivery to produce exactly one wake, and the exposed
coherence-shell invalidate/writeback paths to be live even while SRAM backs the
memory model.
`scripts/check_rtl_typed_trace_contract.py` runs the same S0 path and validates
the emitted `TTRACE` JSON records for retire, scheduler, event, fault,
TLB/cache invalidation, coherence, and watchdog/telemetry records against the
shared schema. This is a first S0 typed trace scaffold, not a full T4
RTL-to-Lean refinement proof.

`formal/M1TransitionInvariantModel.lean` is the current focused Lean slice for
SG-AUTH. It is a small transition system, not a mature architecture-wide
capability proof: one root domain, one consumer domain, one main object, one
optional created object, one sent cap, and one minted cap. It defines state,
steps, reachability, capability tables, domains, generations, transfer, revoke,
object creation, and failed operations, with preservation lemmas and
reachable-state theorems for lineage-valid non-forgeability, no authority
amplification across every modeled authority slot, valid transfer, minted caps
that currently authorize their created object, and transition theorems that new
minted/sent/consumer authority appears only through authorized `objectCreate`,
`capSend`, `capDup`, or `capRecv` paths. `capSend` and `capRecv` now require
current authority, including live generation, known domain/object lineage,
sealed-state, and rights-subset checks. It also proves that revocation
invalidates any outstanding live main-object transfer for current-authorization
purposes; denied, stale, revoked, and full authority operations preserve every
modeled cap slot through explicit per-operation preservation lemmas;
stale-generation rejection, revocation safety, no lost wakeup; a narrow theorem
that every modeled typed commit transition refines an allowed Lean `Step`; and a
typed-commit theorem that failed authority commits preserve every modeled cap
slot. The model now
distinguishes lineage-valid caps from caps that currently authorize work: root
duplicate/mint/push, all-stored-cap lineage/no-amplification,
minted-created-object authority, and consumer pull are separate
predicates/theorems that include generation, domain, sealed-state, and rights
checks. The local M1 assertions/checker use the same predicate boundaries for
live root authority, created-object mint authority, consumer pull authority,
fail-closed duplicate denial, revoke, stale-generation refusal, and
dup/send/recv transfer continuity. The M1 RTL exports
`lnp64_m1_cap_commit_t` at the engine boundary, and
`scripts/check_rtl_m1_typed_commit_trace.py` executes a narrow M1
state-transition mirror over the RTL `TTRACE_M1` cap commit records. It checks
that the schema-backed RTL commit fields, op enum, and status enum map to Lean
`CommitRecord`, `CommitOp`, `CommitStatus`, `TypedCommitTransition`, `Op`, and
the typed-commit bridge, transfer-current-authority, and fail-closed
authority-slot theorem names, then
checks transfer, rights narrowing, fail-closed denied duplication, root-bounded
object creation/mint, revoke, and stale-generation behavior across the bounded M1 seed set unless
`LNP64_COSIM_SEEDS` overrides it; it is not a broad manifest expansion or a
bit-level formal RTL-to-Lean refinement proof.

Individual RTL/model slices are available as `scripts/run_rtl_m2.sh` through
`scripts/run_rtl_m15.sh`. See `verification_plan.md` for the current evidence
matrix and expected gate coverage.

For a casual theorem-to-RTL review surface, start with
`formal/theorem_rtl_coupling_index.md`; it maps the main claims to Lean theorem
names, artifact levels, RTL modules, assertion files, trace markers, gates,
trust levels, and known gaps. The table keeps RTL modules, assertion files,
trace markers, gates, trust levels, and proof gaps visible together. Current
rows remain T1 at the RTL-coupling level; M1, M2, M4, M5, M7, and M14 now
include T3 transition-invariant Lean slices, but the claims are not T2/T4 refinement
evidence until typed transition traces and checked RTL-to-Lean refinement exist.
`scripts/check_theorem_rtl_coupling.py` verifies that index against the
machine-readable coupling manifest.

The shared RTL schema is checked separately:

```sh
scripts/check_rtl_shared_schema.py
scripts/check_rtl_typed_trace_contract.py
scripts/check_rtl_top_level_program_manifest.py
```

The schema gate compares `rtl/schema/lnp64_shared_schema.json` with
`rtl/include/lnp64_pkg.sv` and the current trace manifest. The top-level program
manifest tracks `demos/*.s` and compiler-generated demo assembly as recurring
future `lnp64_top` Verilator tests once their required features exist. Both
gates intentionally label most current M1-M15 trace comparison as string-trace
scaffolding. M1 now has a focused schema-backed typed cap-commit trace and
executable transition comparison; S0 has an initial runtime schema check. Full generated
typed transition comparison and checked refinement remain future work.

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
