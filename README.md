# LNP64 Emulator and C Compiler

LNP64 is a draft capability-machine architecture for system software. It keeps
ordinary computation as a conventional load/store CPU, but makes files,
memory, waitables, devices, service calls, and isolation hardware-visible
capability objects.

This repository contains a practical Rust emulator, assembler, and small C
compiler for exploring that design. It is not a transistor-accurate RTL model.

## Architecture In One Page

- FDR capability registers are unforgeable object handles; POSIX file
  descriptors are a compatibility view over them.
- Resource operations use native instructions such as `PULL`, `PUSH`, `AWAIT`,
  `MMAP`, `CAP_*`, `OBJECT_CTL`, `DOMAIN_CTL`, `GATE_CALL`, and `DMA_CTL`.
- Resource Domains unify containers, VMs, cgroups, sandboxes, supervisors, and
  mission/assurance profiles.
- Services own evolving policy: filesystems, loaders, networking, PCIe quirks,
  Unix compatibility, declassification, and orchestration.
- Hardware owns enforcement: capability validity, generation/lineage checks,
  VMA permissions, DMA/IOMMU scope, wait/wake transitions, scheduler dispatch,
  audit/debug gates, and commit points.
- Native APIs prefer selectors, capabilities, event queues, call gates,
  hardware object profiles, and Resource Domains; POSIX paths, POSIX UID/GID,
  signals, and `errno` are compatibility profiles.
- The full enterprise profile is intended to be realtime-capable: instructions
  retire, park, or submit explicit transactions within published latency
  classes, and long work is represented by waitables/completions rather than
  hidden stalls.
- Core tiles are hardware-multithreaded by interleaving in-order issue across
  many contexts; v1 does not require speculative SMT/hyperthreading.
- Bounded servicelets provide eBPF-like policy near queues, gates, domains,
  classifiers, audit streams, and device/event profiles using a verified subset
  of the existing ISA. They may run on tiny dedicated servicelet engines, but
  only inside verifier-approved cycle, memory, and authority bounds.
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
- `netbsd_personality_abi.md`: first NetBSD-like rump/personality ABI boundary
  and smoke gate mapping.
- `object_format.md`: target static ELF/software-loader profile and exec-plan
  boundary.
- `toolchain_roadmap.md`: LLVM/Clang/lld and NetBSD-derived toolchain bring-up
  plan, including the toy compiler freeze policy.
- `toolchain/lnp64_target.manifest`: checked seed contract for the future LLVM
  target skeleton.
- `libc_roadmap.md`: libc/runtime integration plan.
- `emulator_security_roadmap.md`: emulator security implementation roadmap.

Implementation:

- `src/isa.rs`: instruction and opcode definitions.
- `src/asm.rs`: assembler.
- `src/emulator.rs`: emulator runtime.
- `src/c_compiler.rs`: small C compiler.
- `rtl/`: synthesizable SystemVerilog S0 skeleton and Verilator testbench.
- `formal/`: S0 abstract proof model and mirrored RTL assertions.
- `demos/`: assembly and C demos.
- `userland/`: minimal bootable userland image with init, shell, and command binaries.
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
- Ready-queue scheduling, futex parking/wake, gate delivery with `GATE_RETURN`,
  queue/message endpoints, Resource Domains, `OBJECT_CTL`, and explicit
  call-gate profiles.
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

Working commands are meant to be run from the repository root. For a normal
host-side hygiene pass, use:

```sh
cargo fmt --check
cargo test
bash scripts/run_demos.sh
git diff --check
rg "MSG_RECV|\\bPIPE\\b"
rg "EVENT_CTL|TIMER_CTL"
```

Notes on the commands that are known to work:

- Run them from the repository root; most scripts `cd` there defensively, but
  the one-off `cargo run -- ...` examples assume repo-relative paths.
- `cargo test` is the authoritative emulator/compiler regression gate. Use
  `cargo test --quiet <test_name>` for a focused check before the full suite.
- `cargo fmt --check` and `git diff --check` are read-only checks. They are
  expected to fail only when formatting or whitespace needs fixing.
- `scripts/run_demos.sh` compiles C demos to `/tmp/*.s`, runs assembly demos,
  and exercises the loopback `netcat` and `httpd` demos. It needs bash
  `/dev/tcp` support and free localhost ports `41065` and `41066`.
- `scripts/run_real_packages.sh` is the aggregate package smoke command. It
  runs the individual `sbase`, `inih`, `zlib`, `natsort`, `cwalk`, `jsmn`, and
  `libc_test` package gates listed later in this section.
- After `cargo build --release`, set
  `LNP64_BIN="$PWD/target/release/lnp64"` before running demo/package scripts
  to avoid repeated `cargo run --release` rebuild checks. `run_software_gates.sh`
  does this automatically with a copied release binary under `/tmp`.
- The Rust `target/` tree can get large after full gates. Run `cargo clean`
  when you want the workspace back to a small on-disk footprint.
- The `rg` alias checks are allowed to print documentation hits. Treat new
  emulator/compiler implementation hits for POSIX-first terms as layering
  regressions unless they are explicitly compatibility lowerings.

The `rg "MSG_RECV|\\bPIPE\\b"` results should be limited to documentation or
negative assertions. The `rg "EVENT_CTL|TIMER_CTL"` results should be
alias-only wording; new emulator/compiler work should use event queues,
waitables, typed control records, and native object/domain terminology instead.

Quick command notes:

- Host-only, no Docker: `cargo fmt --check`, `cargo test`,
  `bash scripts/run_demos.sh`, `bash scripts/run_userland.sh`,
  `bash scripts/run_netbsd_personality_system.sh`,
  `bash scripts/run_real_packages.sh`, and `git diff --check`.
- Release-reuse path: `cargo build --release` followed by
  `LNP64_BIN="$PWD/target/release/lnp64" bash scripts/run_demos.sh` works for
  demo reruns without paying the `cargo run --release` startup path each time.
- Docker-required for a reproducible proof/RTL environment:
  `bash scripts/run_rtl_proof_docker.sh` and
  `bash scripts/run_rtl_synth_docker.sh`.
- Host RTL/proof scripts assume local Verilator, Python, and optionally Lean or
  Yosys. Use the Docker commands when those tools are not installed locally.
- Network demo scripts use localhost only, but they still need free loopback
  ports and a shell with `/dev/tcp` support.
- Cleanup after large gates: `cargo clean` removes Rust build artifacts, and
  Docker image cleanup is left to normal Docker tooling.

For the broader host software gate, run:

```sh
bash scripts/run_software_gates.sh
```

That script runs Rust formatting/tests, builds the release emulator, then runs
the toolchain contracts, NetBSD personality smoke/system gates, demos,
userland, and real-package gates. It also exports `LNP64_BIN` so downstream
demo/package scripts reuse the release binary instead of rebuilding through
`cargo run`.

For the full repository gate with Dockerized RTL/proof coverage, run:

```sh
bash scripts/run_all_gates.sh
```

That command runs the RTL/proof Docker gate, the host software gate, and
`git diff --check`.

For RTL/proof work, use the Dockerized gates below first. These commands were
run successfully in this checkout on 2026-06-18; they are separated from the
emulator/Rust gates above because the Lean and RTL toolchains are intentionally
containerized.

The current first-class verification path is the RTL/proof Docker flow. It keeps
Lean, Verilator, and the proof gate dependencies out of the host environment.
The Docker commands are intentionally heavyweight because they install the tool
chains inside images; once an image exists, use the shorter `docker run ...`
rerun commands below for the live workspace.

Run the reproducible RTL/proof co-design gate:

```sh
bash scripts/run_rtl_proof_docker.sh
```

This builds `Dockerfile.rtl-proof`, installs Lean and Verilator, runs the gate
during image construction, then reruns it against the mounted working tree with
Lean required. The gate checks S0 through M13, runs the Python mirrors, runs the
RTL simulations, runs the bounded M1-M13 randomized co-simulation seeds, checks
the formal proof-obligation manifest under `formal/`, and rejects `axiom`,
`sorry`, and `admit` in the checked Lean files. The final line should be:

```text
rtl/proof gates ok
```

After the image exists, this is the shorter command for rerunning the same
gate against the live workspace:

```sh
docker run --rm \
  -e LNP64_REQUIRE_LEAN=1 \
  -v "$PWD:/work" \
  -w /work \
  lnp64-rtl-proof \
  bash scripts/run_rtl_proof_gates.sh
```

To run the same RTL/proof gate directly on a host that already has compatible
Lean, Python, and Verilator installed:

```sh
LNP64_REQUIRE_LEAN=1 bash scripts/run_rtl_proof_gates.sh
```

Without `LNP64_REQUIRE_LEAN=1`, the host gate still runs the Python mirrors and
RTL simulations, but skips Lean if `lean` is not configured. Use the Docker path
for proof work unless you are intentionally testing the host toolchain.

Run the Dockerized RTL synthesis/FPGA smoke gate:

```sh
bash scripts/run_rtl_synth_docker.sh
```

This builds `Dockerfile.rtl-synth`, checks the FPGA constraint manifest under
`fpga/constraints/`, checks the Track D bring-up coverage manifest under
`fpga/bringup/`, checks the Track B RTL block manifest under `rtl/`, checks the
roadmap S0 shell/record contract, runs a Yosys S0 synthesis/netlist smoke, and
statically elaborates the S0 through M13 RTL tops with Verilator. It is a
bring-up smoke gate for constraints, roadmap coverage, and synthesizable RTL
shape, not a claimed FPGA bitstream or board build.

After the image exists, rerun the same synthesis smoke gate against the live
workspace:

```sh
docker run --rm \
  -v "$PWD:/work" \
  -w /work \
  lnp64-rtl-synth \
  bash scripts/run_rtl_synth_smoke.sh
```

Run only the bounded randomized RTL/model co-simulation smoke:

```sh
bash scripts/run_rtl_random_cosim.sh
```

This validates `tests/traces/rtl_cosim_manifest.json` and runs the seedable
M1-M13 model/RTL trace comparisons for the default bounded seed set. Override
the seed set with `LNP64_COSIM_SEEDS`.

The currently exercised random slices are:

- M1 ping-pong queues: queue generation, push payload, and refill payload.
- M2 gates: gate generation, continuation id, and call targets.
- M3 process/thread lifecycle: parent/child ids, exit code, exec epoch, and
  stopped-sibling count.
- M4 VMAs: VMA id, page count, base address, and VMA generation.
- M5 DMA/memory objects: root domain, source/destination buffers, copy/fill
  sizes, fill value, and isolation-domain checks.
- M6 typed control/service boundary: root/namespace ids, path length, service
  and operation ids, continuation id, returned rights, and returned object id.
- M7 futex/atomic: root domain, initial atomic value, compare-exchange values,
  futex address, and bucket id.
- M8 heap: root domain, heap generation, pointer, size class, owner/freeing
  thread ids, and pointer generation.
- M9 classifier/servicelet: root/table ids, verifier program and instruction
  count, packet and IPC steering fields, and budget cycle count.
- M10 RAS/observability: measurement and telemetry ids, ECC correction count,
  watchdog reset id, visible counters, trace-ring capacity/writes, quote id,
  and audit label.
- M11 DDR/metadata broker: root domain, DDR line id/generation, metadata epoch,
  byte length, data value, cross-domain id, and ECC correction count.
- M12 SD/SPI storage barrier: root domain, object id/generation, barrier id,
  block index, byte length, data value, cross-domain id, and media status.
- M13 PCIe/IOMMU/MSI: root domain, requester id, BAR id/generation, IOMMU
  context, DMA byte count, MSI vector, rogue domain, and malformed field id.

The following host checks have also been kept as small, actually runnable
sanity commands for the RTL/proof path:

```sh
bash -n scripts/run_rtl_*.sh
scripts/check_formal_proof_manifest.py
scripts/check_rtl_cosim_manifest.py
scripts/check_rtl_synth_constraints.py
scripts/check_fpga_bringup_manifest.py
scripts/check_rtl_track_b_manifest.py
scripts/check_rtl_s0_contract.py
bash scripts/run_rtl_yosys_s0.sh
rg -n "\\baxiom\\b|sorry|admit" formal || true
bash scripts/run_rtl_synth_smoke.sh
git diff --check
```

Run individual RTL/model vertical slices:

Run the first RTL/formal co-design skeleton gate:

```sh
bash scripts/run_rtl_s0.sh
```

Run the first RTL/model co-simulation vertical slice:

```sh
bash scripts/run_rtl_m1.sh
```

Run the gate/continuation RTL/model vertical slice:

```sh
bash scripts/run_rtl_m2.sh
```

Run the process/thread lifecycle RTL/model vertical slice:

```sh
bash scripts/run_rtl_m3.sh
```

Run the VMA/MMU RTL/model vertical slice:

```sh
bash scripts/run_rtl_m4.sh
```

Run the DMA/memory-object RTL/model vertical slice:

```sh
bash scripts/run_rtl_m5.sh
```

Run the typed-control/namespace/service-boundary RTL/model vertical slice:

```sh
bash scripts/run_rtl_m6.sh
```

Run the futex/atomic RTL/model vertical slice:

```sh
bash scripts/run_rtl_m7.sh
```

Run the heap RTL/model vertical slice:

```sh
bash scripts/run_rtl_m8.sh
```

Run the classifier/servicelet RTL/model vertical slice:

```sh
bash scripts/run_rtl_m9.sh
```

Run the RAS/observability/assurance RTL/model vertical slice:

```sh
bash scripts/run_rtl_m10.sh
```

Run the DDR/metadata-broker RTL/model vertical slice:

```sh
bash scripts/run_rtl_m11.sh
```

Run the SD/SPI storage-barrier RTL/model vertical slice:

```sh
bash scripts/run_rtl_m12.sh
```

Run the PCIe/IOMMU/MSI RTL/model vertical slice:

```sh
bash scripts/run_rtl_m13.sh
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
bash scripts/run_netbsd_personality_system.sh
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
