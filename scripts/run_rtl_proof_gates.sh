#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

lean_files=(
  formal/S0Model.lean
  formal/M1Model.lean
  formal/M1TransitionInvariantModel.lean
  formal/M2GateModel.lean
  formal/M2TransitionInvariantModel.lean
  formal/M3ProcessModel.lean
  formal/M4VmaModel.lean
  formal/M4TransitionInvariantModel.lean
  formal/M5DmaModel.lean
  formal/M5TransitionInvariantModel.lean
  formal/M6ServiceModel.lean
  formal/M7FutexAtomicModel.lean
  formal/M7TransitionInvariantModel.lean
  formal/M8HeapModel.lean
  formal/M9ClassifierServiceletModel.lean
  formal/M10RasModel.lean
  formal/M11DdrMetadataModel.lean
  formal/M12StorageBarrierModel.lean
  formal/M13PcieIommuModel.lean
  formal/M14ResourceDomainPolicyModel.lean
  formal/M14TransitionInvariantModel.lean
  formal/M15ObjectProfilesModel.lean
  formal/FormalTheoremsModel.lean
)

scripts/check_formal_proof_manifest.py
scripts/check_theorem_rtl_coupling.py
scripts/check_formal_rtl_roadmap_audit.py

if grep -RInE '(^|[^[:alnum:]_])(axiom|sorry|admit)([^[:alnum:]_]|$)' "${lean_files[@]}"; then
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
bash scripts/run_rtl_m1.sh
scripts/check_rtl_m1_typed_commit_trace.py
scripts/test_rtl_m1_typed_commit_checker.py
scripts/test_rtl_m1_schema_checker.py
bash scripts/run_rtl_m2.sh
bash scripts/run_rtl_m3.sh
bash scripts/run_rtl_m4.sh
bash scripts/run_rtl_m5.sh
bash scripts/run_rtl_m6.sh
bash scripts/run_rtl_m7.sh
scripts/check_rtl_m7_typed_commit_trace.py
scripts/test_rtl_m7_typed_commit_checker.py
bash scripts/run_rtl_m8.sh
bash scripts/run_rtl_m9.sh
bash scripts/run_rtl_m10.sh
bash scripts/run_rtl_m11.sh
bash scripts/run_rtl_m12.sh
bash scripts/run_rtl_m13.sh
bash scripts/run_rtl_m14.sh
bash scripts/run_rtl_m15.sh

if [[ "${LNP64_RTL_PROOF_RANDOM_COSIM:-1}" == "0" ||
      "${LNP64_RTL_PROOF_SKIP_RANDOM_COSIM:-0}" == "1" ]]; then
  scripts/check_rtl_cosim_manifest.py
  printf '%s\n' "rtl random cosim skipped (set LNP64_RTL_PROOF_RANDOM_COSIM=1 for full randomized/cosim sweep)"
else
  # LNP64_RTL_RANDOM_COSIM_JOBS is consumed by the randomized/cosim runner.
  bash scripts/run_rtl_random_cosim.sh
fi

printf '%s\n' "rtl/proof gates ok"
