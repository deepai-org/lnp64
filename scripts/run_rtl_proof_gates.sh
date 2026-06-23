#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [[ "${LNP64_RTL_FAST:-0}" == "1" ]]; then
  export LNP64_RTL_REUSE_BUILD="${LNP64_RTL_REUSE_BUILD:-1}"
  export LNP64_RTL_SKIP_LINT="${LNP64_RTL_SKIP_LINT:-1}"
  export LNP64_RTL_BUILD_ROOT="${LNP64_RTL_BUILD_ROOT:-$root/target/rtl-verilator}"
  export LNP64_RTL_PROOF_RANDOM_COSIM="${LNP64_RTL_PROOF_RANDOM_COSIM:-0}"
fi

proof_gate_jobs="${LNP64_RTL_PROOF_GATE_JOBS:-}"
if [[ -z "$proof_gate_jobs" ]]; then
  if [[ "${LNP64_RTL_FAST:-0}" == "1" ]]; then
    proof_gate_jobs=auto
  else
    proof_gate_jobs=1
  fi
fi
if [[ "$proof_gate_jobs" == "auto" ]]; then
  proof_gate_jobs="$(nproc 2>/dev/null || printf '1')"
fi
if ! [[ "$proof_gate_jobs" =~ ^[0-9]+$ ]] || (( proof_gate_jobs < 1 )); then
  printf 'LNP64_RTL_PROOF_GATE_JOBS must be a positive integer or auto, got %q\n' "$proof_gate_jobs" >&2
  exit 1
fi

run_rtl_proof_gate_batch() {
  if (( proof_gate_jobs == 1 || $# == 1 )); then
    local gate
    for gate in "$@"; do
      bash "$gate"
    done
    return
  fi

  local log_dir
  log_dir="$(mktemp -d "${TMPDIR:-/tmp}/lnp64_rtl_proof_gates.XXXXXX")"
  local active_pids=()
  declare -A active_gates=()
  declare -A active_logs=()
  local failed=0

  proof_remove_active_pid() {
    local done_pid="$1"
    local remaining=()
    local pid
    for pid in "${active_pids[@]}"; do
      if [[ "$pid" != "$done_pid" ]]; then
        remaining+=("$pid")
      fi
    done
    active_pids=("${remaining[@]}")
  }

  proof_wait_one() {
    local done_pid gate log
    if wait -n -p done_pid; then
      gate="${active_gates[$done_pid]}"
      log="${active_logs[$done_pid]}"
      printf 'rtl/proof gate %s ok (%s)\n' "$gate" "$log"
    else
      gate="${active_gates[$done_pid]}"
      log="${active_logs[$done_pid]}"
      failed=1
      printf 'rtl/proof gate %s failed (%s)\n' "$gate" "$log" >&2
      cat "$log" >&2
    fi
    unset "active_gates[$done_pid]" "active_logs[$done_pid]"
    proof_remove_active_pid "$done_pid"
  }

  printf 'running rtl/proof gates with %s parallel job(s); logs in %s\n' "$proof_gate_jobs" "$log_dir"
  local gate label log pid
  for gate in "$@"; do
    label="${gate#scripts/run_rtl_}"
    label="${label%.sh}"
    log="$log_dir/${label}.log"
    bash "$gate" >"$log" 2>&1 &
    pid="$!"
    active_pids+=("$pid")
    active_gates[$pid]="$gate"
    active_logs[$pid]="$log"
    if (( ${#active_pids[@]} >= proof_gate_jobs )); then
      proof_wait_one
    fi
  done
  while (( ${#active_pids[@]} > 0 )); do
    proof_wait_one
  done

  if [[ "${LNP64_RTL_PROOF_KEEP_GATE_LOGS:-0}" != "1" ]]; then
    rm -rf "$log_dir"
  fi
  if (( failed != 0 )); then
    exit 1
  fi
}

lean_files=(
  formal/S0Model.lean
  formal/M1Model.lean
  formal/M1TransitionInvariantModel.lean
  formal/M2GateModel.lean
  formal/M2TransitionInvariantModel.lean
  formal/M3ProcessModel.lean
  formal/M3TransitionInvariantModel.lean
  formal/M4VmaModel.lean
  formal/M4TransitionInvariantModel.lean
  formal/M5DmaModel.lean
  formal/M5TransitionInvariantModel.lean
  formal/M6ServiceModel.lean
  formal/M6TransitionInvariantModel.lean
  formal/M7FutexAtomicModel.lean
  formal/M7TransitionInvariantModel.lean
  formal/M8HeapModel.lean
  formal/M8TransitionInvariantModel.lean
  formal/M9ClassifierServiceletModel.lean
  formal/M9TransitionInvariantModel.lean
  formal/M10RasModel.lean
  formal/M10TransitionInvariantModel.lean
  formal/M11DdrMetadataModel.lean
  formal/M11TransitionInvariantModel.lean
  formal/M12StorageBarrierModel.lean
  formal/M12TransitionInvariantModel.lean
  formal/M13PcieIommuModel.lean
  formal/M13TransitionInvariantModel.lean
  formal/M14ResourceDomainPolicyModel.lean
  formal/M14TransitionInvariantModel.lean
  formal/M15ObjectProfilesModel.lean
  formal/M15TransitionInvariantModel.lean
  formal/M16EndpointModel.lean
  formal/MvsHwSwBridge.lean
  formal/FormalTheoremsModel.lean
)

scripts/check_formal_proof_manifest.py
scripts/check_theorem_rtl_coupling.py
scripts/check_formal_rtl_roadmap_audit.py

if grep -RInE '(^|[^[:alnum:]_])(axiom|sorry|admit)([^[:alnum:]_]|$)' "${lean_files[@]}" formal/WholeChipComposition.lean; then
  printf '%s\n' "formal Lean files must not contain axiom, sorry, or admit" >&2
  exit 1
fi

if command -v lean >/dev/null 2>&1 && lean --version >/dev/null 2>&1; then
  for file in "${lean_files[@]}"; do
    lean "$file"
  done
elif [[ "${LNP64_REQUIRE_LEAN:-0}" == "1" ]]; then
  printf '%s\n' "lean is required for this gate but is not configured" >&2
  exit 1
else
  printf '%s\n' "lean not configured; skipping Lean syntax checks (set LNP64_REQUIRE_LEAN=1 to require them)"
fi

# Whole-chip composition: every reachable whole-chip state satisfies the
# conjunction of all fifteen engines' severe-goal transition invariants.
bash scripts/run_rtl_whole_chip_composition_gate.sh

# RTL-to-Lean refinement: the emitted M11 typed-commit op trace is a valid Lean
# Step path reaching a state that satisfies the proved transition invariant.
bash scripts/run_rtl_m11_refinement_gate.sh

formal/m1_model.py >/dev/null
formal/m2_gate_model.py >/dev/null
formal/m3_process_model.py >/dev/null
formal/m4_vma_model.py >/dev/null
formal/m5_dma_model.py >/dev/null
formal/m6_service_model.py >/dev/null
formal/m7_futex_atomic_model.py >/dev/null
formal/m8_heap_model.py >/dev/null
formal/m9_classifier_servicelet_model.py >/dev/null
formal/m10_ras_model.py >/dev/null
formal/m11_ddr_metadata_model.py >/dev/null
formal/m12_storage_barrier_model.py >/dev/null
formal/m13_pcie_iommu_model.py >/dev/null
formal/m14_resource_domain_policy_model.py >/dev/null
formal/m15_object_profiles_model.py >/dev/null

bash scripts/run_rtl_s0.sh
LNP64_TYPED_TRACE_USE_EXISTING=1 scripts/check_rtl_typed_trace_contract.py

m1_log="${TMPDIR:-/tmp}/lnp64_rtl_proof_m1.log"
if [[ "${LNP64_RTL_FAST:-0}" == "1" ]]; then
  default_m1_seeds="0"
else
  default_m1_seeds="0 1 7 42 255 1024 4095 4096 65536 1048576 16777216 134217728 268435456 536870912"
fi
LNP64_COSIM_SEEDS="${LNP64_M1_TYPED_COMMIT_SEEDS:-$default_m1_seeds}" \
  bash scripts/run_rtl_m1.sh | tee "$m1_log"
LNP64_M1_TYPED_COMMIT_USE_EXISTING=1 \
  LNP64_M1_TYPED_COMMIT_LOG="$m1_log" \
  scripts/check_rtl_m1_typed_commit_trace.py
scripts/test_rtl_m1_typed_commit_checker.py
scripts/test_rtl_m1_schema_checker.py
run_rtl_proof_gate_batch \
  scripts/run_rtl_m2.sh \
  scripts/run_rtl_m3.sh \
  scripts/run_rtl_m4.sh \
  scripts/run_rtl_m5.sh \
  scripts/run_rtl_m6.sh

m4_witness="${TMPDIR:-/tmp}/lnp64_rtl_proof_m4_witness.json"
LNP64_RTL_M4_WITNESS_OUT="$m4_witness" \
  scripts/check_rtl_m4_typed_commit_trace.py
scripts/check_rtl_m4_witness.py "$m4_witness"
scripts/test_rtl_m4_typed_commit_checker.py
scripts/test_rtl_m4_witness_checker.py
LNP64_RTL_M4_WITNESS_OUT="$m4_witness" \
  bash scripts/run_rtl_m4_lean_witness_gate.sh

m5_witness="${TMPDIR:-/tmp}/lnp64_rtl_proof_m5_witness.json"
LNP64_RTL_M5_WITNESS_OUT="$m5_witness" \
  scripts/check_rtl_m5_typed_commit_trace.py
scripts/check_rtl_m5_witness.py "$m5_witness"
scripts/test_rtl_m5_typed_commit_checker.py
scripts/test_rtl_m5_witness_checker.py
LNP64_RTL_M5_WITNESS_OUT="$m5_witness" \
  bash scripts/run_rtl_m5_lean_witness_gate.sh

m2_witness="${TMPDIR:-/tmp}/lnp64_rtl_proof_m2_witness.json"
LNP64_RTL_M2_WITNESS_OUT="$m2_witness" \
  scripts/check_rtl_m2_typed_commit_trace.py
scripts/check_rtl_m2_witness.py "$m2_witness"
scripts/test_rtl_m2_typed_commit_checker.py
scripts/test_rtl_m2_witness_checker.py
LNP64_RTL_M2_WITNESS_OUT="$m2_witness" \
  bash scripts/run_rtl_m2_lean_witness_gate.sh

m14_witness="${TMPDIR:-/tmp}/lnp64_rtl_proof_m14_witness.json"
LNP64_RTL_M14_WITNESS_OUT="$m14_witness" \
  scripts/check_rtl_m14_typed_commit_trace.py
scripts/check_rtl_m14_witness.py "$m14_witness"
scripts/test_rtl_m14_typed_commit_checker.py
scripts/test_rtl_m14_witness_checker.py
LNP64_RTL_M14_WITNESS_OUT="$m14_witness" \
  bash scripts/run_rtl_m14_lean_witness_gate.sh

m3_witness="${TMPDIR:-/tmp}/lnp64_rtl_proof_m3_witness.json"
LNP64_RTL_M3_WITNESS_OUT="$m3_witness" \
  scripts/check_rtl_m3_typed_commit_trace.py
scripts/check_rtl_m3_witness.py "$m3_witness"
scripts/test_rtl_m3_typed_commit_checker.py
scripts/test_rtl_m3_witness_checker.py
LNP64_RTL_M3_WITNESS_OUT="$m3_witness" \
  bash scripts/run_rtl_m3_lean_witness_gate.sh

m6_witness="${TMPDIR:-/tmp}/lnp64_rtl_proof_m6_witness.json"
LNP64_RTL_M6_WITNESS_OUT="$m6_witness" \
  scripts/check_rtl_m6_typed_commit_trace.py
scripts/check_rtl_m6_witness.py "$m6_witness"
scripts/test_rtl_m6_typed_commit_checker.py
scripts/test_rtl_m6_witness_checker.py
LNP64_RTL_M6_WITNESS_OUT="$m6_witness" \
  bash scripts/run_rtl_m6_lean_witness_gate.sh

m8_witness="${TMPDIR:-/tmp}/lnp64_rtl_proof_m8_witness.json"
LNP64_RTL_M8_WITNESS_OUT="$m8_witness" \
  scripts/check_rtl_m8_typed_commit_trace.py
scripts/check_rtl_m8_witness.py "$m8_witness"
scripts/test_rtl_m8_typed_commit_checker.py
scripts/test_rtl_m8_witness_checker.py
LNP64_RTL_M8_WITNESS_OUT="$m8_witness" \
  bash scripts/run_rtl_m8_lean_witness_gate.sh

m9_witness="${TMPDIR:-/tmp}/lnp64_rtl_proof_m9_witness.json"
LNP64_RTL_M9_WITNESS_OUT="$m9_witness" \
  scripts/check_rtl_m9_typed_commit_trace.py
scripts/check_rtl_m9_witness.py "$m9_witness"
scripts/test_rtl_m9_typed_commit_checker.py
scripts/test_rtl_m9_witness_checker.py
LNP64_RTL_M9_WITNESS_OUT="$m9_witness" \
  bash scripts/run_rtl_m9_lean_witness_gate.sh

m10_witness="${TMPDIR:-/tmp}/lnp64_rtl_proof_m10_witness.json"
LNP64_RTL_M10_WITNESS_OUT="$m10_witness" \
  scripts/check_rtl_m10_typed_commit_trace.py
scripts/check_rtl_m10_witness.py "$m10_witness"
scripts/test_rtl_m10_typed_commit_checker.py
scripts/test_rtl_m10_witness_checker.py
LNP64_RTL_M10_WITNESS_OUT="$m10_witness" \
  bash scripts/run_rtl_m10_lean_witness_gate.sh

m11_witness="${TMPDIR:-/tmp}/lnp64_rtl_proof_m11_witness.json"
LNP64_RTL_M11_WITNESS_OUT="$m11_witness" \
  scripts/check_rtl_m11_typed_commit_trace.py
scripts/check_rtl_m11_witness.py "$m11_witness"
scripts/test_rtl_m11_typed_commit_checker.py
scripts/test_rtl_m11_witness_checker.py
LNP64_RTL_M11_WITNESS_OUT="$m11_witness" \
  bash scripts/run_rtl_m11_lean_witness_gate.sh

m12_witness="${TMPDIR:-/tmp}/lnp64_rtl_proof_m12_witness.json"
LNP64_RTL_M12_WITNESS_OUT="$m12_witness" \
  scripts/check_rtl_m12_typed_commit_trace.py
scripts/check_rtl_m12_witness.py "$m12_witness"
scripts/test_rtl_m12_typed_commit_checker.py
scripts/test_rtl_m12_witness_checker.py
LNP64_RTL_M12_WITNESS_OUT="$m12_witness" \
  bash scripts/run_rtl_m12_lean_witness_gate.sh

m13_witness="${TMPDIR:-/tmp}/lnp64_rtl_proof_m13_witness.json"
LNP64_RTL_M13_WITNESS_OUT="$m13_witness" \
  scripts/check_rtl_m13_typed_commit_trace.py
scripts/check_rtl_m13_witness.py "$m13_witness"
scripts/test_rtl_m13_typed_commit_checker.py
scripts/test_rtl_m13_witness_checker.py
LNP64_RTL_M13_WITNESS_OUT="$m13_witness" \
  bash scripts/run_rtl_m13_lean_witness_gate.sh

m15_witness="${TMPDIR:-/tmp}/lnp64_rtl_proof_m15_witness.json"
LNP64_RTL_M15_WITNESS_OUT="$m15_witness" \
  scripts/check_rtl_m15_typed_commit_trace.py
scripts/check_rtl_m15_witness.py "$m15_witness"
scripts/test_rtl_m15_typed_commit_checker.py
scripts/test_rtl_m15_witness_checker.py
LNP64_RTL_M15_WITNESS_OUT="$m15_witness" \
  bash scripts/run_rtl_m15_lean_witness_gate.sh

m7_log="${TMPDIR:-/tmp}/lnp64_rtl_proof_m7.log"
m7_witness="${TMPDIR:-/tmp}/lnp64_rtl_proof_m7_witness.json"
LNP64_COSIM_SEEDS="${LNP64_M7_TYPED_COMMIT_SEEDS:-0}" \
  bash scripts/run_rtl_m7.sh | tee "$m7_log"
LNP64_M7_TYPED_COMMIT_USE_EXISTING=1 \
  LNP64_M7_TYPED_COMMIT_LOG="$m7_log" \
  LNP64_RTL_M7_WITNESS_OUT="$m7_witness" \
  scripts/check_rtl_m7_typed_commit_trace.py
scripts/check_rtl_m7_witness.py "$m7_witness"
LNP64_RTL_M7_WITNESS_OUT="$m7_witness" \
  bash scripts/run_rtl_m7_lean_witness_gate.sh
scripts/test_rtl_m7_typed_commit_checker.py
scripts/test_rtl_m7_witness_checker.py
scripts/test_rtl_top_m1_witness_checker.py
run_rtl_proof_gate_batch \
  scripts/run_rtl_m8.sh \
  scripts/run_rtl_m9.sh \
  scripts/run_rtl_m10.sh \
  scripts/run_rtl_m11.sh \
  scripts/run_rtl_m12.sh \
  scripts/run_rtl_m13.sh \
  scripts/run_rtl_m14.sh \
  scripts/run_rtl_m15.sh

if [[ "${LNP64_RTL_PROOF_RANDOM_COSIM:-1}" == "0" ||
      "${LNP64_RTL_PROOF_SKIP_RANDOM_COSIM:-0}" == "1" ]]; then
  scripts/check_rtl_cosim_manifest.py
  printf '%s\n' "rtl random cosim skipped (set LNP64_RTL_PROOF_RANDOM_COSIM=1 for full randomized/cosim sweep)"
else
  # LNP64_RTL_RANDOM_COSIM_JOBS is consumed by the randomized/cosim runner.
  bash scripts/run_rtl_random_cosim.sh
fi

printf '%s\n' "rtl/proof gates ok"
