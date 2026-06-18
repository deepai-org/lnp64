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

Working commands are meant to be run from the repository root. For the normal
host-side hygiene pass that has been used before emulator/compiler commits,
use:

```sh
cargo fmt --check
cargo test --quiet
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
- In a dirty tree, scope whitespace checks to the files you touched, for
  example `git diff --check -- src/emulator.rs README.md`, then run the full
  `git diff --check` before a broad commit.
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
- The `rg` alias checks are allowed to print documentation hits and negative
  assertions. Treat new emulator/compiler implementation hits for POSIX-first
  terms as layering regressions unless they are explicitly compatibility
  lowerings.

The `rg "MSG_RECV|\\bPIPE\\b"` results should be limited to documentation or
negative assertions. The `rg "EVENT_CTL|TIMER_CTL"` results should be
alias-only wording; new emulator/compiler work should use event queues,
waitables, typed control records, and native object/domain terminology instead.

Quick command notes for actually working commands:

- For RTL, proofs, synthesis, FPGA smoke, and board bring-up, use the
  Dockerfile-backed commands in the RTL/proof section below. Those are the
  reproducible paths exercised for Lean, Verilator, Yosys, nextpnr, IceStorm,
  and board-tool flows.
- The host commands in this first list are for the emulator/compiler and
  software-package side of the tree. They are useful sanity checks, but they
  are not the proof/RTL authority.

- Host-only, no Docker: `cargo fmt --check`, `cargo test --quiet`,
  `bash scripts/run_demos.sh`, `bash scripts/run_userland.sh`,
  `bash scripts/run_netbsd_personality_system.sh`,
  `bash scripts/run_real_packages.sh`, and `git diff --check`.
- Release-reuse path: `cargo build --release` followed by
  `export LNP64_BIN="$PWD/target/release/lnp64"` works for demo, userland, and
  package reruns without paying the `cargo run --release` startup path each
  time.
- Docker-required for a reproducible proof/RTL environment:
  `bash scripts/run_rtl_proof_docker.sh` and
  `bash scripts/run_rtl_synth_docker.sh`.
- Host RTL/proof scripts assume local Verilator, Python, and optionally Lean or
  Yosys. Use the Docker commands when those tools are not installed locally.
- Network demo scripts use localhost only, but they still need free loopback
  ports and a shell with `/dev/tcp` support.
- Cleanup after large gates: `cargo clean` removes Rust build artifacts, and
  Docker image cleanup is left to normal Docker tooling.

Actually working command recipes, written as they are meant to be pasted from
the repository root:

```sh
# Small Rust check while iterating on one emulator/compiler area.
# Replace the placeholder with a real Rust test filter.
cargo test --quiet <test_name_or_filter>

# Full host hygiene pass used before committing emulator/compiler work.
cargo fmt --check
cargo test --quiet
bash scripts/run_demos.sh
git diff --check
rg "MSG_RECV|\\bPIPE\\b"
rg "EVENT_CTL|TIMER_CTL"

# Scoped whitespace check when unrelated files are already dirty.
git diff --check -- src/emulator.rs README.md

# Avoid repeated cargo-run startup checks when rerunning demos/packages.
cargo build --release
export LNP64_BIN="$PWD/target/release/lnp64"
bash scripts/run_demos.sh
bash scripts/run_userland.sh
bash scripts/run_real_packages.sh

# Full host-side software gate.
bash scripts/run_software_gates.sh

# One standalone C demo compile/run path.
cargo run -- cc demos/hello.c -o /tmp/hello.lnp64.s
cargo run -- run /tmp/hello.lnp64.s

# Reclaim Rust build artifacts after heavy gates.
cargo clean
```

The placeholder in `cargo test --quiet <test_name_or_filter>` is not a shell
metavariable; replace the whole token with a real Rust test filter, or omit it
for the full suite. The release-reuse path is the fastest stable way to rerun
scripts that honor `LNP64_BIN`; unset it when intentionally testing the default
`cargo run --release` fallback path.

Recent host commands verified from this checkout on 2026-06-18:

```sh
cargo fmt --check
cargo test --quiet
cargo build --release --quiet
LNP64_BIN="$PWD/target/release/lnp64" bash scripts/run_demos.sh
git diff --check
rg -n "MSG_RECV|\\bPIPE\\b"
rg -n "EVENT_CTL|TIMER_CTL"
python3 -m py_compile \
  scripts/check_formal_proof_manifest.py \
  scripts/test_formal_proof_manifest_checker.py \
  scripts/check_rtl_cosim_manifest.py \
  scripts/test_rtl_cosim_manifest_checker.py \
  scripts/check_rtl_track_b_manifest.py \
  scripts/test_rtl_track_b_manifest_checker.py \
  scripts/check_fpga_bringup_manifest.py \
  scripts/test_fpga_bringup_manifest_checker.py \
  scripts/check_rtl_synth_constraints.py \
  scripts/test_rtl_synth_constraints_checker.py \
  scripts/check_rtl_dockerfiles.py \
  scripts/test_rtl_dockerfiles_checker.py \
  scripts/check_formal_rtl_roadmap_audit.py \
  scripts/test_formal_rtl_roadmap_strict_audit.py \
  scripts/test_uart_byte_checker.py \
  scripts/test_board_evidence_checker.py
bash -n scripts/run_rtl_board_ice40_s0.sh
scripts/test_formal_proof_manifest_checker.py
scripts/test_rtl_track_b_manifest_checker.py
scripts/test_rtl_cosim_manifest_checker.py
scripts/test_fpga_bringup_manifest_checker.py
scripts/test_rtl_synth_constraints_checker.py
scripts/test_rtl_dockerfiles_checker.py
scripts/test_board_evidence_checker.py
scripts/test_uart_byte_checker.py
scripts/test_formal_rtl_roadmap_strict_audit.py
scripts/check_formal_rtl_roadmap_audit.py
```

Focused Rust regression filters that have recently been used successfully from
this checkout:

```sh
cargo test --quiet exec_
cargo test --quiet classifier_
cargo test --quiet env_get_
cargo test --quiet signal_
cargo test --quiet namespace_
cargo test --quiet cap_dup_
cargo test --quiet cap_send_
cargo test --quiet cap_recv_
cargo test --quiet cap_revoke_
cargo test --quiet domain_
cargo test --quiet socket_
cargo test --quiet object_ctl_
cargo test --quiet domain_ctl_
cargo test --quiet ns_ctl_resolve
cargo test --quiet memory_object_
cargo test --quiet pipe_
cargo test --quiet waiter
cargo test --quiet microcode
cargo test --quiet timer_expiration_wakes_reader_waiting_for_tick
cargo test --quiet event_counter_write_wakes_reader_waiting_for_nonzero_value
cargo test --quiet waitpid_live_child_parks_until_child_exit_event
cargo test --quiet thread_join_waiter_wakes_and_consumes_exit_status
cargo test --quiet repeated_parking_does_not_duplicate_wait_queue_entries
```

Use these focused filters for fast confirmation while editing one subsystem.
They are not replacements for the full host hygiene pass before committing a
batch that changes emulator or compiler behavior.

Recent focused userland/conformance smoke commands that have also worked from
this checkout:

```sh
cargo run --quiet -- cc userland/classifier_test.c -o /tmp/lnp64-classifier-test.s
cargo run --quiet -- run /tmp/lnp64-classifier-test.s

cargo run --quiet -- cc userland/signal_gate_test.c -o /tmp/lnp64-signal-gate-test.s
cargo run --quiet -- run /tmp/lnp64-signal-gate-test.s

cargo run --quiet -- cc userland/signal_fault_test.c -o /tmp/lnp64-signal-fault-test.s
cargo run --quiet -- run /tmp/lnp64-signal-fault-test.s

cargo run --quiet -- cc userland/domain_nested_test.c -o /tmp/lnp64-domain-nested-test.s
cargo run --quiet -- run /tmp/lnp64-domain-nested-test.s

cargo run --quiet -- cc userland/domain_budget_test.c -o /tmp/lnp64-domain-budget-test.s
cargo run --quiet -- run /tmp/lnp64-domain-budget-test.s

cargo run --quiet -- cc userland/namespace_test.c -o /tmp/lnp64-namespace-test.s
root=$(mktemp -d /tmp/lnp64-namespace-root.XXXXXX)
mkdir -p "$root/etc" "$root/tmp"
printf '%s\n' 'welcome to lnp64 namespace test' > "$root/etc/motd"
cargo run --quiet -- run --namespace-root "$root" /tmp/lnp64-namespace-test.s
rm -rf "$root"
```

The namespace smoke intentionally needs `--namespace-root`; running the compiled
program without a root capability is expected to fail because path resolution is
capability-relative.

For the alias scans, expected hits are documentation, compatibility-lowering
comments, script negative lists, or compiler negative assertions. Treat new
matches in emulator internals, RTL opcode definitions, or compiler lowering as
things to inspect before committing.

Known command pitfalls from this checkout:

- There is no `docs/` directory. Run repository-wide text checks without a
  stale `docs` path, for example `rg "MSG_RECV|\\bPIPE\\b"` from the repository
  root, or scope them to existing files/directories.
- `bash scripts/run_demos.sh` is the all-demos command. It already covers the
  assembly demos, C demos, and loopback network demos that are expected to run
  on the host.
- Dockerized RTL/proof and synth commands are the reproducible paths for Lean,
  Verilator, Yosys, nextpnr, and IceStorm. The board bring-up command is a real
  hardware path and requires an attached board plus UART device.

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

That command runs the RTL/proof Docker gate, the RTL synth/FPGA Docker gate,
the formal RTL roadmap audit, the host software gate, and `git diff --check`.

For RTL/proof work, use the Dockerized gates below first. These commands were
run successfully in this checkout on 2026-06-18; they are separated from the
emulator/Rust gates above because the Lean and RTL toolchains are intentionally
containerized.

The current first-class verification path is the RTL/proof Docker flow. It keeps
Lean, Verilator, and the proof gate dependencies out of the host environment.
The Docker commands are intentionally heavyweight because they install the tool
chains inside images; once an image exists, use the shorter `docker run ...`
rerun commands below for the live workspace.

Actually verified Docker commands from this checkout:

```sh
# Full proof/RTL gate: builds Dockerfile.rtl-proof, then reruns on the live tree.
bash scripts/run_rtl_proof_docker.sh

# Faster proof/RTL rerun after the image exists.
docker run --rm \
  -e LNP64_REQUIRE_LEAN=1 \
  -v "$PWD:/work" \
  -w /work \
  lnp64-rtl-proof \
  bash scripts/run_rtl_proof_gates.sh

# Full synth/FPGA smoke gate: builds Dockerfile.rtl-synth, then reruns live.
bash scripts/run_rtl_synth_docker.sh

# Faster synth/FPGA rerun after the image exists.
docker run --rm \
  -v "$PWD:/work" \
  -w /work \
  lnp64-rtl-synth \
  bash scripts/run_rtl_synth_gates.sh

# Focused iCE40 bitstream/timing smoke after the synth image exists.
docker run --rm \
  -v "$PWD:/work" \
  -w /work \
  lnp64-rtl-synth \
  bash scripts/run_rtl_fpga_ice40_s0.sh

# Board-tool image build; live validation still requires attached hardware.
docker build -f Dockerfile.rtl-board -t lnp64-rtl-board .

# Board-tool image smoke; confirms the container has programmer, FPGA, and UART deps.
docker run --rm \
  -v "$PWD:/work" \
  -w /work \
  lnp64-rtl-board \
  bash -lc 'set -euo pipefail; command -v iceprog; command -v yosys; command -v nextpnr-ice40; command -v icepack; command -v icetime; python3 -c "import serial; print(\"python serial ok\")"'
```

Expected success lines for those Docker reruns are:

```text
rtl/proof gates ok
rtl synthesis gates ok
rtl fpga uart s0 gate ok
ice40 timing ok fmax=37.723MHz
icetime timing ok fmax=37.83MHz
rtl fpga ice40 s0 bitstream ok
```

Run the reproducible RTL/proof co-design gate:

```sh
bash scripts/run_rtl_proof_docker.sh
```

This builds `Dockerfile.rtl-proof`, installs Lean and Verilator, runs the gate
during image construction, then reruns it against the mounted working tree with
Lean required. The gate checks S0 through M15, runs the Python mirrors, runs the
RTL simulations, runs the bounded M1-M15 randomized co-simulation seeds, checks
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
roadmap S0 shell/record contract, runs a Yosys S0 synthesis/netlist smoke,
runs Yosys netlist synthesis for the M1-M15 vertical RTL tops, statically
elaborates the S0 through M15 RTL tops with Verilator, simulates the
S0 FPGA wrapper UART waveform and status LEDs, then builds a generic iCE40 HX8K
S0 bitstream with Yosys `synth_ice40`, `nextpnr-ice40`, and `icepack` using the
package-level CT256 PCF at
`fpga/constraints/lnp64_s0_ice40_hx8k_ct256.pcf`. The iCE40 gate parses the
nextpnr JSON report and fails if the S0 wrapper misses the 12 MHz smoke timing
target or exceeds reported HX8K resource availability; it also runs Icestorm
`icetime` on the routed ASC and checks its independent timing estimate. It is
still not a board-schematic pinout or physical board bring-up claim.

After the image exists, rerun the same synthesis smoke gate against the live
workspace:

```sh
docker run --rm \
  -v "$PWD:/work" \
  -w /work \
  lnp64-rtl-synth \
  bash scripts/run_rtl_synth_gates.sh
```

To run only the FPGA bitstream smoke inside an existing synth image:

```sh
docker run --rm \
  -v "$PWD:/work" \
  -w /work \
  lnp64-rtl-synth \
  bash scripts/run_rtl_fpga_ice40_s0.sh
```

To run only the S0 FPGA UART/status waveform simulation inside an existing synth
image:

```sh
docker run --rm \
  -v "$PWD:/work" \
  -w /work \
  lnp64-rtl-synth \
  bash scripts/run_rtl_fpga_uart_s0.sh
```

The S0 iCE40 wrapper drives `uart_tx` with a real 115200-baud 8N1 boot/status
byte (`0x53`) and status LEDs for the bring-up predicates. A live board
validation path is available, but it intentionally fails unless a compatible
iCE40 board and UART adapter are attached and named:

```sh
export LNP64_BOARD_UART_DEVICE=/dev/ttyUSB1
# Container path under the mounted repo; appears on the host as build/...
export LNP64_BOARD_EVIDENCE_OUT=/work/build/lnp64-board-ice40-s0-evidence.json
bash scripts/run_rtl_board_docker.sh
```

That builds `Dockerfile.rtl-board`, generates/programs the S0 iCE40 bitstream
with `iceprog`, runs a preflight probe (`iceprog -t`) before programming unless
`LNP64_BOARD_SKIP_PROGRAMMER_PROBE=1`, starts UART capture before programming,
and requires the boot/status byte. If your programmer needs explicit IceStorm
selection, set `LNP64_ICEPROG_DEVICE`, `LNP64_ICEPROG_INTERFACE`, or
`LNP64_ICEPROG_SLOW=1` before running the Docker wrapper. It keeps the generated
bitstream under `build/` by default and writes a structured evidence JSON
containing the bitstream path and SHA-256, preflight log path, programmer log
path, UART capture path, UART device, observed boot byte, and FPGA/programmer
tool-version probes. Validate a saved evidence file with:

```sh
scripts/check_board_evidence.py build/lnp64-board-ice40-s0-evidence.json
```

The expected live-hardware success lines are:

```text
rtl board ice40 s0 preflight ok
rtl board ice40 s0 program ok
board evidence ok
rtl board ice40 s0 live uart ok
```

Focused M5 DMA pin/unpin work was checked from this checkout on 2026-06-18
with the following commands. The Docker commands are the reproducible proof and
synthesis paths; the host commands are focused manifest/model/simulation checks
that do not replace the Docker gates:

```sh
docker run --rm \
  -v "$PWD:/work" \
  -w /work \
  lnp64-rtl-proof \
  lean formal/M5DmaModel.lean

python3 formal/m5_dma_model.py
scripts/check_formal_proof_manifest.py
scripts/check_rtl_track_b_manifest.py
bash scripts/run_rtl_m5.sh
bash scripts/run_rtl_random_cosim.sh

docker run --rm \
  -v "$PWD:/work" \
  -w /work \
  lnp64-rtl-proof \
  bash scripts/run_rtl_proof_gates.sh

docker run --rm \
  -v "$PWD:/work" \
  -w /work \
  lnp64-rtl-synth \
  bash scripts/run_rtl_synth_smoke.sh

bash scripts/run_formal_rtl_roadmap_audit.sh

# Expected to fail until a live board run writes this evidence file.
bash scripts/run_formal_rtl_roadmap_audit.sh \
  --require-board-evidence \
  --board-evidence build/lnp64-board-ice40-s0-evidence.json
```

The M5 traces in those runs include `TRACE dma_pin ... pinned=1` before copy/fill
and `TRACE dma_unpin ... pinned=0` before the permission/revoke/domain fault
checks. As of 2026-06-18, the strict roadmap audit reports only the missing live
board evidence file when no compatible iCE40 board/UART capture has been run.

Focused M2 gate/fault/signal compatibility work was checked from this checkout
on 2026-06-18 with these commands:

```sh
python3 -m py_compile \
  scripts/check_rtl_track_b_manifest.py \
  scripts/check_rtl_cosim_manifest.py \
  formal/m2_gate_model.py
scripts/check_rtl_track_b_manifest.py
scripts/check_rtl_cosim_manifest.py
bash scripts/run_rtl_m2.sh

docker run --rm \
  -v "$PWD:/work" \
  -w /work \
  lnp64-rtl-proof \
  lean formal/M2GateModel.lean
```

The M2 RTL/model trace now includes
`TRACE signal_compat mask=honored authority=0`, and the co-sim manifest requires
that marker together with the honored mask and cleared authority fields.

Run only the bounded randomized RTL/model co-simulation smoke:

```sh
bash scripts/run_rtl_random_cosim.sh
```

This validates `tests/traces/rtl_cosim_manifest.json` and runs the seedable
M1-M15 model/RTL trace comparisons for the default bounded seed set. Override
the seed set with `LNP64_COSIM_SEEDS`.

The currently exercised random slices are:

- M1 ping-pong queues: queue generation, push payload, and refill payload.
- M2 gates: gate generation, continuation id, call targets, fault delivery, and
  signal compatibility.
- M3 process/thread lifecycle: parent/child ids, exit code, exec epoch, and
  stopped-sibling count.
- M4 VMAs: VMA id, page count, base address, and VMA generation.
- M5 DMA/memory objects: root domain, source/destination buffers, copy/fill
  sizes, fill value, pin/unpin state, and isolation-domain checks.
- M6 typed control/service boundary: root/namespace ids, path length, service
  and operation ids, continuation id, returned rights, and returned object id.
- M7 futex/atomic: root domain, initial atomic value, compare-exchange values,
  futex address, timer deadline, and bucket id.
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
- M14 Resource Domain/policy: root and child domains, parent/child budgets,
  requested rights, child/sibling usage, policy mask, and policy label.
- M15 object profiles: object id/generation, counter threshold, queue payload,
  event generation, and gate continuation id.

The following host checks have also been kept as small, actually runnable
sanity commands for the RTL/proof path:

```sh
bash -n scripts/run_rtl_*.sh
python3 -m py_compile scripts/check_rtl_s0_contract.py
scripts/check_formal_proof_manifest.py
scripts/test_formal_proof_manifest_checker.py
scripts/check_rtl_cosim_manifest.py
scripts/test_rtl_cosim_manifest_checker.py
scripts/check_rtl_synth_constraints.py
scripts/test_rtl_synth_constraints_checker.py
scripts/check_fpga_bringup_manifest.py
scripts/test_fpga_bringup_manifest_checker.py
scripts/check_rtl_track_b_manifest.py
scripts/test_rtl_track_b_manifest_checker.py
scripts/check_rtl_s0_contract.py
scripts/check_rtl_dockerfiles.py
scripts/test_rtl_dockerfiles_checker.py
scripts/test_board_evidence_checker.py
scripts/test_uart_byte_checker.py
scripts/test_formal_rtl_roadmap_strict_audit.py
scripts/test_rtl_board_no_hardware.sh
scripts/check_formal_rtl_roadmap_audit.py
bash scripts/run_formal_rtl_roadmap_audit.sh
bash scripts/run_rtl_yosys_s0.sh
rg -n "\\baxiom\\b|sorry|admit" formal || true
bash scripts/run_rtl_synth_smoke.sh
git diff --check
```

The host list is intentionally a sanity list, not the authoritative proof/synth
flow. The local host may not have Lean, Verilator, Yosys, nextpnr, IceStorm, or
the iCE40 chip database installed. Use the Docker commands above for the
reproducible Lean proof and RTL synthesis/FPGA gates.

When changing the S0 shell contract, run the focused contract and simulation
gates before the Docker synth/proof reruns:

```sh
python3 -m py_compile scripts/check_rtl_s0_contract.py && scripts/check_rtl_s0_contract.py
bash scripts/run_rtl_s0.sh
bash scripts/run_rtl_synth_smoke.sh
```

The roadmap audit checker has a strict hardware mode for completion audits:

```sh
bash scripts/run_formal_rtl_roadmap_audit.sh

bash scripts/run_formal_rtl_roadmap_audit.sh --docker-rerun

bash scripts/run_formal_rtl_roadmap_audit.sh --docker-build

bash scripts/run_formal_rtl_roadmap_audit.sh \
  --require-board-evidence \
  --board-evidence build/lnp64-board-ice40-s0-evidence.json

LNP64_REQUIRE_BOARD_EVIDENCE=1 \
  scripts/check_formal_rtl_roadmap_audit.py \
  --board-evidence build/lnp64-board-ice40-s0-evidence.json
```

The first command is the lightweight checklist gate and reports pending hardware
when no board evidence exists. The strict commands are expected to fail until
`bash scripts/run_rtl_board_docker.sh` has programmed a real board and
captured/validated the live UART evidence file.

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

Run the Resource Domain/policy RTL/model vertical slice:

```sh
bash scripts/run_rtl_m14.sh
```

Run the object-profile RTL/model vertical slice:

```sh
bash scripts/run_rtl_m15.sh
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
