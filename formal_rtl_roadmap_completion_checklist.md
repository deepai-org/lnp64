# Formal/RTL Roadmap Completion Checklist

Objective: fully implement `formal_rtl_codesign_roadmap.md` as an
executable-spec -> proof-model -> reference-model -> RTL -> RTL-simulation ->
FPGA-bring-up flow, with Docker-backed verification. Physical board validation
is kept as an optional hardware-only extension because no real FPGA is available
in this environment.

Current completion status: incomplete for the full roadmap. The Docker-backed
RTL/proof/synthesis/FPGA-smoke scope is implemented and passing.
`formal_theorems.md` has checked Lean coverage artifacts through
`formal/FormalTheoremsModel.lean`. M1 capability/FDR, M2 gate/continuation,
M4 VMA/MMU, M5 DMA, M7 waitable/scheduler, and M14 Resource Domain now have
transition-invariant proof slices; remaining work is to replace the rest of the
coverage and
bounded-witness artifacts with transition-invariant proofs and RTL refinement
proofs for the full SystemVerilog chip and real architectural programs. The
current theorem-to-RTL coupling rows remain T1 because their RTL coupling still
uses string traces and assertion evidence; they are not T2 until typed
transition traces exist.

The RTL/proof/synthesis/FPGA-smoke deliverables are represented by
machine-checkable manifests and gates. The strict hardware audit is intentionally
separate and must fail until `build/lnp64-board-ice40-s0-evidence.json` exists
and validates with `scripts/check_board_evidence.py`.

## Roadmap-To-Artifact Checklist

| Roadmap requirement | Evidence | Gate |
| --- | --- | --- |
| Repository layout for `rtl/`, `rtl/include/`, `rtl/top/`, `rtl/core/`, `rtl/engines/`, `rtl/sim/`, `formal/`, `formal/rtl_assertions/`, `tests/rtl/`, and `tests/traces/` | RTL, formal, assertion, filelist, and trace files under those paths | `bash scripts/run_formal_rtl_roadmap_audit.sh` |
| S0 module shells and architectural records | `rtl/include/lnp64_pkg.sv`, `rtl/top/lnp64_top.sv`, `rtl/top/lnp64_reset_boot.sv`, `rtl/core/lnp64_core_tile.sv`, `rtl/engines/lnp64_engine_shells.sv` | `scripts/check_rtl_s0_contract.py` |
| S0 ready/valid command, response, event, and fault channels | S0 module port checks in `scripts/check_rtl_s0_contract.py` | `scripts/check_rtl_s0_contract.py` |
| S0 reset, boot, stub-terminal behavior, no raw authority, event, fault, watchdog, UART, and SRAM acceptance tests | `rtl/sim/lnp64_s0_tb.sv`, `formal/rtl_assertions/lnp64_s0_assertions.sv`, `scripts/run_rtl_s0.sh` | `bash scripts/run_rtl_s0.sh` |
| S0 typed transition trace scaffold | `rtl/sim/lnp64_s0_tb.sv` emits schema-named `TTRACE` JSON records for retire, scheduler, event, fault, TLB/cache invalidation, coherence, and watchdog/telemetry paths | `scripts/check_rtl_typed_trace_contract.py` |
| RTL S0 contract checker positive and negative checks | `scripts/check_rtl_s0_contract.py`, `scripts/test_rtl_s0_contract_checker.py` | `scripts/test_rtl_s0_contract_checker.py` |
| S0 proof obligations | `formal/S0Model.lean`, `formal/proof_obligations_manifest.json` | `scripts/check_formal_proof_manifest.py` |
| Track A A1-A10 formal model and theorem artifacts | `formal/proof_obligations_manifest.json`, `formal/S0Model.lean`, `formal/M1Model.lean`, `formal/M2GateModel.lean` through `formal/M15ObjectProfilesModel.lean` | `scripts/check_formal_proof_manifest.py`; Docker: `bash scripts/run_rtl_proof_docker.sh` |
| Formal theorem roadmap coverage | `formal_theorems.md`, `formal/FormalTheoremsModel.lean`, `formal/proof_obligations_manifest.json` obligation `FT`; coverage only, not a full architecture proof | `scripts/check_formal_proof_manifest.py`; Docker: `docker run --rm -e LNP64_REQUIRE_LEAN=1 -v "$PWD:/work" -w /work lnp64-rtl-proof lean formal/FormalTheoremsModel.lean` |
| Human-auditable theorem-to-RTL coupling evidence | `formal/theorem_rtl_coupling_manifest.json`, `formal/theorem_rtl_coupling_index.md`, theorem names, artifact levels, RTL modules, assertions, trace markers, trust levels, known gaps, and gates for each major guarantee | `scripts/check_theorem_rtl_coupling.py` |
| Formal proof manifest checker positive and negative checks | `scripts/check_formal_proof_manifest.py`, `scripts/test_formal_proof_manifest_checker.py` | `scripts/test_formal_proof_manifest_checker.py` |
| Track B B0-B14 RTL skeleton and vertical block coverage | `rtl/track_b_blocks_manifest.json`, S0 and M1-M15 RTL/testbench/filelist/assertion artifacts, `scripts/run_rtl_yosys_vertical_slices.sh` | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_yosys_vertical_slices.sh`; Docker synth gate |
| RTL Track B manifest checker positive and negative checks | `scripts/check_rtl_track_b_manifest.py`, `scripts/test_rtl_track_b_manifest_checker.py` | `scripts/test_rtl_track_b_manifest_checker.py` |
| Track C fixed and bounded-random model/RTL co-simulation | `tests/traces/rtl_cosim_manifest.json`, `scripts/run_rtl_random_cosim.sh`, `formal/m*_model.py` | `scripts/check_rtl_cosim_manifest.py`; `bash scripts/run_rtl_random_cosim.sh` |
| RTL co-simulation manifest checker positive and negative checks | `scripts/check_rtl_cosim_manifest.py`, `scripts/test_rtl_cosim_manifest_checker.py` | `scripts/test_rtl_cosim_manifest_checker.py` |
| Track D 17-step FPGA bring-up smoke metadata | `fpga/bringup/lnp64_track_d_bringup.json`, `fpga/constraints/lnp64_s0_smoke.json` | `scripts/check_fpga_bringup_manifest.py`; `scripts/check_rtl_synth_constraints.py` |
| FPGA bring-up manifest checker positive and negative checks | `scripts/check_fpga_bringup_manifest.py`, `scripts/test_fpga_bringup_manifest_checker.py` | `scripts/test_fpga_bringup_manifest_checker.py` |
| RTL synthesis constraints checker positive and negative checks | `scripts/check_rtl_synth_constraints.py`, `scripts/test_rtl_synth_constraints_checker.py` | `scripts/test_rtl_synth_constraints_checker.py` |
| FPGA report checker positive and negative checks | `scripts/check_ice40_report.py`, `scripts/check_icetime_report.py`, `scripts/test_fpga_report_checkers.py` | `scripts/test_fpga_report_checkers.py` |
| iCE40 bitstream, UART waveform, nextpnr timing, and icetime timing smoke | `fpga/rtl/lnp64_s0_fpga_top.sv`, `rtl/sim/lnp64_s0_fpga_tb.sv`, `fpga/constraints/lnp64_s0_ice40_hx8k_ct256.pcf` | `bash scripts/run_rtl_synth_docker.sh` or `docker run --rm -v "$PWD:/work" -w /work lnp64-rtl-synth bash scripts/run_rtl_synth_gates.sh` |
| First milestone: whole-machine skeleton | S0 RTL, assertions, Lean S0 model, S0 Verilator testbench, and S0 contract checker | `bash scripts/run_rtl_s0.sh`; `scripts/check_rtl_s0_contract.py` |
| Second milestone: bounded ping-pong witness | `rtl/engines/lnp64_m1_pingpong.sv`, `rtl/sim/lnp64_m1_tb.sv`, `formal/M1Model.lean`, `formal/m1_model.py`, `tests/rtl/m1_filelist.f` | `bash scripts/run_rtl_m1.sh`; `scripts/check_rtl_cosim_manifest.py` |
| Docker-backed proof, synthesis, FPGA, and board command paths | `Dockerfile.rtl-proof`, `Dockerfile.rtl-synth`, `Dockerfile.rtl-board`, `scripts/run_rtl_proof_docker.sh`, `scripts/run_rtl_synth_docker.sh`, `scripts/run_rtl_board_docker.sh` | `scripts/check_rtl_dockerfiles.py` |
| RTL Dockerfile checker positive and negative checks | `scripts/check_rtl_dockerfiles.py`, `scripts/test_rtl_dockerfiles_checker.py` | `scripts/test_rtl_dockerfiles_checker.py` |
| Board UART/evidence validator positive and negative checks | `scripts/check_uart_byte.py`, `scripts/test_uart_byte_checker.py`, `scripts/check_board_evidence.py`, `scripts/test_board_evidence_checker.py` | `scripts/test_uart_byte_checker.py`; `scripts/test_board_evidence_checker.py` |
| Strict roadmap audit board-evidence plumbing | `scripts/check_formal_rtl_roadmap_audit.py`, `scripts/check_board_evidence.py`, `scripts/test_formal_rtl_roadmap_strict_audit.py` | `scripts/test_formal_rtl_roadmap_strict_audit.py` |
| Board Docker wrapper no-hardware failure behavior | `scripts/run_rtl_board_docker.sh`, `scripts/test_rtl_board_no_hardware.sh` | `scripts/test_rtl_board_no_hardware.sh` |
| Optional physical iCE40 board programming and live UART capture | `build/lnp64-board-ice40-s0-evidence.json` generated by `bash scripts/run_rtl_board_docker.sh` with attached hardware | `scripts/check_board_evidence.py build/lnp64-board-ice40-s0-evidence.json` |

## Detailed Track A Proof Checklist

| Roadmap item | Evidence | Gate |
| --- | --- | --- |
| A1 state core | `formal/S0Model.lean` theorem entries in `formal/proof_obligations_manifest.json` | `scripts/check_formal_proof_manifest.py` |
| A2 capability/FDR engine | `formal/M1Model.lean` theorem entries in `formal/proof_obligations_manifest.json` | `scripts/check_formal_proof_manifest.py` |
| A3 waitable/scheduler core | `formal/M7FutexAtomicModel.lean` bounded-witness theorem entries plus `formal/M7TransitionInvariantModel.lean` reachable-state scheduler/wakeup invariants in `formal/proof_obligations_manifest.json` | `scripts/check_formal_proof_manifest.py` |
| A4 object profiles | `formal/M15ObjectProfilesModel.lean` theorem entries in `formal/proof_obligations_manifest.json` | `scripts/check_formal_proof_manifest.py` |
| A5 VMA/DMA slice | `formal/M4VmaModel.lean`, `formal/M4TransitionInvariantModel.lean` reachable-state VMA/MMU invariants, `formal/M5DmaModel.lean`, `formal/M5TransitionInvariantModel.lean` reachable-state DMA confinement invariants, and theorem entries in `formal/proof_obligations_manifest.json` | `scripts/check_formal_proof_manifest.py` |
| A6 servicelets/classifiers | `formal/M9ClassifierServiceletModel.lean` theorem entries in `formal/proof_obligations_manifest.json` | `scripts/check_formal_proof_manifest.py` |
| A7 resource domains and policy enforcement | `formal/M14ResourceDomainPolicyModel.lean` bounded-witness theorem entries, `formal/M14TransitionInvariantModel.lean` reachable-state containment/lifecycle/policy invariants, cross-policy theorem entries, and `formal/proof_obligations_manifest.json` | `scripts/check_formal_proof_manifest.py` |
| A8 gate delivery, faults, and compatibility signals | `formal/M2GateModel.lean`, `formal/M2TransitionInvariantModel.lean` reachable-state gate/fault-delivery invariants, `formal/M3ProcessModel.lean`, and theorem entries in `formal/proof_obligations_manifest.json` | `scripts/check_formal_proof_manifest.py`; Docker: `docker run --rm -v "$PWD:/work" -w /work lnp64-rtl-proof lean formal/M2TransitionInvariantModel.lean` |
| A9 memory consistency, coherence, and visibility | `formal/M4VmaModel.lean`, `formal/M4TransitionInvariantModel.lean`, `formal/M5DmaModel.lean`, `formal/M7FutexAtomicModel.lean`, `formal/M11DdrMetadataModel.lean`, `formal/M12StorageBarrierModel.lean`, and `formal/M13PcieIommuModel.lean` entries | `scripts/check_formal_proof_manifest.py` |
| A10 RAS, adversarial input, and global progress | `formal/M10RasModel.lean`, `formal/M6ServiceModel.lean`, and theorem entries in `formal/proof_obligations_manifest.json` | `scripts/check_formal_proof_manifest.py` |

## Detailed Track B RTL Checklist

| Roadmap item | Evidence | Gate |
| --- | --- | --- |
| B0 whole-machine skeleton | `rtl/track_b_blocks_manifest.json` entry `B0`, S0 RTL/testbench/assertions/filelist | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_s0.sh` |
| B1 ISA decode, canonical errors, and `ENV_GET` | `rtl/track_b_blocks_manifest.json` entry `B1`, S0 decode/errno/env modules and S0 testbench markers | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_s0.sh` |
| B2 minimal core tile | `rtl/track_b_blocks_manifest.json` entry `B2`, S0 core/retire/thread/memory modules and SRAM LD/ST markers | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_s0.sh` |
| B3 FDR/capability table block | `rtl/track_b_blocks_manifest.json` entry `B3`, M1 RTL/model/proof/assertions/filelist | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_m1.sh` |
| B4 scheduler/waitable block | `rtl/track_b_blocks_manifest.json` entry `B4`, S0/M1/M7/M14 RTL slices and await/timer/futex/domain markers | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_random_cosim.sh` |
| B5 object queue/counter block | `rtl/track_b_blocks_manifest.json` entry `B5`, M1/M15 queue/counter/event RTL slices | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_m15.sh` |
| B6 gate/continuation block | `rtl/track_b_blocks_manifest.json` entry `B6`, M2 RTL/model/proof/assertions with `TRACE signal_compat` | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_m2.sh` |
| B7 process/thread lifecycle block | `rtl/track_b_blocks_manifest.json` entry `B7`, M3 RTL/model/proof/assertions/filelist | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_m3.sh` |
| B8 tiny VMA/MMU block | `rtl/track_b_blocks_manifest.json` entry `B8`, M4 RTL/model/proof/assertions/filelist | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_m4.sh` |
| B9 DMA/memory object block | `rtl/track_b_blocks_manifest.json` entry `B9`, M5 RTL/model/proof/assertions with pin/unpin traces | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_m5.sh` |
| B10 typed control, namespace dispatch, and service boundary | `rtl/track_b_blocks_manifest.json` entry `B10`, M6 RTL/model/proof/assertions/filelist | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_m6.sh` |
| B11 futex/atomic block | `rtl/track_b_blocks_manifest.json` entry `B11`, M7 RTL/model/proof/assertions/filelist | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_m7.sh` |
| B12 heap block | `rtl/track_b_blocks_manifest.json` entry `B12`, M8 RTL/model/proof/assertions/filelist | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_m8.sh` |
| B13 classifier, servicelet, and networking prototype | `rtl/track_b_blocks_manifest.json` entry `B13`, M9 RTL/model/proof/assertions/filelist | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_m9.sh` |
| B14 RAS, observability, and assurance block | `rtl/track_b_blocks_manifest.json` entry `B14`, M10 RTL/model/proof/assertions/filelist | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_m10.sh` |

## Detailed Track C Co-Simulation Checklist

| Roadmap item | Evidence | Gate |
| --- | --- | --- |
| Same input vector in emulator/model and RTL simulation | `tests/traces/rtl_cosim_manifest.json` fixed M1-M15 entries, `formal/m*_model.py`, and `scripts/run_rtl_m*.sh` trace diff commands | `scripts/check_rtl_cosim_manifest.py`; `bash scripts/run_rtl_random_cosim.sh` |
| S0 runtime typed trace records | `scripts/check_rtl_typed_trace_contract.py` validates S0 `TTRACE` records against `rtl/schema/lnp64_shared_schema.json`; M1-M15 typed transition comparison is still future refinement work | `scripts/check_rtl_typed_trace_contract.py` |
| Compare architectural state | `trace_comparison_contract.architectural_state` in `tests/traces/rtl_cosim_manifest.json` and checker trace-token validation | `scripts/check_rtl_cosim_manifest.py` |
| Compare result codes | `trace_comparison_contract.result_codes` in `tests/traces/rtl_cosim_manifest.json` and checker trace-token validation | `scripts/check_rtl_cosim_manifest.py` |
| Compare event records | `trace_comparison_contract.event_records` in `tests/traces/rtl_cosim_manifest.json` and checker trace-token validation | `scripts/check_rtl_cosim_manifest.py` |
| Compare FDR/generation/authority metadata | `trace_comparison_contract.authority_generation_metadata` in `tests/traces/rtl_cosim_manifest.json` and checker trace-token validation | `scripts/check_rtl_cosim_manifest.py` |
| Random but bounded traces from models | `bounded_random_gates` in `tests/traces/rtl_cosim_manifest.json` and `scripts/run_rtl_random_cosim.sh` shared seed set | `scripts/check_rtl_cosim_manifest.py`; `bash scripts/run_rtl_random_cosim.sh` |
| Top-level program corpus from existing assembly and LLVM C coverage | `tests/rtl/top_level_program_manifest.json` tracks every `demos/*.s` file plus active LLVM clang/linked C entries as feature-gated future `lnp64_top` Verilator tests | `scripts/check_rtl_top_level_program_manifest.py` |
| Verilator for fast CI | M1-M15 `scripts/run_rtl_m*.sh` build each RTL testbench with Verilator and require pass markers | `scripts/check_rtl_cosim_manifest.py` |
| FPGA simulation and synthesis checks | `scripts/run_rtl_fpga_uart_s0.sh`, `scripts/run_rtl_fpga_ice40_s0.sh`, and `scripts/run_rtl_synth_gates.sh` | `bash scripts/run_rtl_synth_docker.sh` |

## Detailed Track D FPGA Bring-Up Checklist

| Roadmap step | Evidence | Gate |
| --- | --- | --- |
| 1. top-level skeleton modules connected with stub responses | `fpga/bringup/lnp64_track_d_bringup.json` step 1, S0 FPGA top/filelists | `scripts/check_fpga_bringup_manifest.py`; `scripts/run_rtl_fpga_ice40_s0.sh` |
| 2. fixed decode table, canonical errors, and `ENV_GET` | `fpga/bringup/lnp64_track_d_bringup.json` step 2 and S0 decode markers | `scripts/check_fpga_bringup_manifest.py`; `bash scripts/run_rtl_s0.sh` |
| 3. soft SRAM only, no DDR | `fpga/bringup/lnp64_track_d_bringup.json` step 3 and S0 SRAM LD/ST markers | `scripts/check_fpga_bringup_manifest.py`; `bash scripts/run_rtl_s0.sh` |
| 4. UART output | `fpga/bringup/lnp64_track_d_bringup.json` step 4 and FPGA UART testbench | `scripts/check_fpga_bringup_manifest.py`; `scripts/run_rtl_fpga_uart_s0.sh` |
| 5. two core tiles, tile telemetry, and simple assembler program ROM | `fpga/bringup/lnp64_track_d_bringup.json` step 5 and S0 multicore retire/topology markers | `scripts/check_fpga_bringup_manifest.py`; `bash scripts/run_rtl_s0.sh` |
| 6. FDR/capability table and generation checks | `fpga/bringup/lnp64_track_d_bringup.json` step 6 and M1 cap traces | `scripts/check_fpga_bringup_manifest.py`; `bash scripts/run_rtl_m1.sh` |
| 7. scheduler, waitable, event router, and object queue smoke | `fpga/bringup/lnp64_track_d_bringup.json` step 7 and S0/M1 traces | `scripts/check_fpga_bringup_manifest.py`; `bash scripts/run_rtl_random_cosim.sh` |
| 8. gate/continuation and process lifecycle smoke | `fpga/bringup/lnp64_track_d_bringup.json` step 8 and M2/M3 traces | `scripts/check_fpga_bringup_manifest.py`; `bash scripts/run_rtl_random_cosim.sh` |
| 9. tiny VMA/MMU, TLB invalidation, and memory-protection smoke | `fpga/bringup/lnp64_track_d_bringup.json` step 9 and M4 traces | `scripts/check_fpga_bringup_manifest.py`; `bash scripts/run_rtl_m4.sh` |
| 10. external DDR and shared metadata broker | `fpga/bringup/lnp64_track_d_bringup.json` step 10 and M11 traces | `scripts/check_fpga_bringup_manifest.py`; `bash scripts/run_rtl_m11.sh` |
| 11. DMA/memory-object smoke | `fpga/bringup/lnp64_track_d_bringup.json` step 11 and M5 traces | `scripts/check_fpga_bringup_manifest.py`; `bash scripts/run_rtl_m5.sh` |
| 12. typed control, namespace/service dispatch, and capability-return smoke | `fpga/bringup/lnp64_track_d_bringup.json` step 12 and M6 traces | `scripts/check_fpga_bringup_manifest.py`; `bash scripts/run_rtl_m6.sh` |
| 13. futex/atomic and heap smoke | `fpga/bringup/lnp64_track_d_bringup.json` step 13 and M7/M8 traces | `scripts/check_fpga_bringup_manifest.py`; `bash scripts/run_rtl_random_cosim.sh` |
| 14. SD/SPI and storage-barrier smoke | `fpga/bringup/lnp64_track_d_bringup.json` step 14 and M12 traces | `scripts/check_fpga_bringup_manifest.py`; `bash scripts/run_rtl_m12.sh` |
| 15. Ethernet packet queue, classifier, and servicelet smoke | `fpga/bringup/lnp64_track_d_bringup.json` step 15 and M9 traces | `scripts/check_fpga_bringup_manifest.py`; `bash scripts/run_rtl_m9.sh` |
| 16. RAS/telemetry/watchdog/attestation smoke | `fpga/bringup/lnp64_track_d_bringup.json` step 16 and M10 traces | `scripts/check_fpga_bringup_manifest.py`; `bash scripts/run_rtl_m10.sh` |
| 17. PCIe later | `fpga/bringup/lnp64_track_d_bringup.json` step 17 and M13 PCIe/IOMMU stub traces | `scripts/check_fpga_bringup_manifest.py`; `bash scripts/run_rtl_m13.sh` |

## Detailed Milestone Checklist

| Milestone requirement | Evidence | Gate |
| --- | --- | --- |
| First milestone required slice | S0 RTL/top/core/engine shells, records in `rtl/include/lnp64_pkg.sv`, S0 testbench/assertions/filelist, and S0 contract checker | `scripts/check_rtl_s0_contract.py`; `bash scripts/run_rtl_s0.sh` |
| First milestone proof targets | `formal/S0Model.lean` and S0 entries in `formal/proof_obligations_manifest.json` | `scripts/check_formal_proof_manifest.py` |
| First milestone expected demo | `rtl/sim/lnp64_s0_tb.sv` exercises reset, tiny PID 1 program, UART status, `ENV_GET`, unsupported command, and structured fault event | `bash scripts/run_rtl_s0.sh` |
| Second milestone required slice | M1 ping-pong RTL/testbench/assertions/filelist plus S0/B1-B5 shell coverage | `scripts/check_rtl_track_b_manifest.py`; `bash scripts/run_rtl_m1.sh` |
| Second milestone proof targets | `formal/M1Model.lean` bounded-witness theorem entries in `formal/proof_obligations_manifest.json`; later work must turn this into a reachable-state transition proof | `scripts/check_formal_proof_manifest.py` |
| Second milestone expected demo | `formal/m1_model.py`, `scripts/run_rtl_m1.sh`, and `tests/traces/rtl_cosim_manifest.json` compare M1 model and RTL traces | `scripts/check_rtl_cosim_manifest.py`; `bash scripts/run_rtl_m1.sh` |

## Completion Commands

Lightweight non-hardware audit:

```sh
bash scripts/run_formal_rtl_roadmap_audit.sh
```

Docker-backed proof and synth rerun:

```sh
bash scripts/run_formal_rtl_roadmap_audit.sh --docker-rerun
```

Strict hardware audit after live board capture:

```sh
bash scripts/run_formal_rtl_roadmap_audit.sh \
  --require-board-evidence \
  --board-evidence build/lnp64-board-ice40-s0-evidence.json
```

Expected strict blocker before hardware evidence exists:

```text
missing required live board evidence .../build/lnp64-board-ice40-s0-evidence.json
```

## Optional Hardware Validation

No real FPGA is available in this environment, so the live-board path is not a
completion requirement for the Docker-backed roadmap implementation. To run the
hardware-only validation later, attach a compatible iCE40 board and UART adapter,
set
`LNP64_BOARD_UART_DEVICE` to the board UART TTY, run
`bash scripts/run_rtl_board_docker.sh`, and then rerun the strict hardware
audit.
