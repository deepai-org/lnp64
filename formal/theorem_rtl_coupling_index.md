# Theorem/RTL Coupling Evidence Index

This index is the human-readable view of
`formal/theorem_rtl_coupling_manifest.json`. It is intentionally conservative:
the current claims are T1 bounded witnesses because they are tied through
coverage or bounded-witness Lean artifacts, RTL assertions, executable-model
string traces, and Docker gates. They are not T2 yet because typed transition
traces from the shared schema do not exist, and they are not T3/T4 transition
or refinement proofs.

| Claim | Trust | Lean Evidence And Artifact Level | RTL/Assertion Evidence | Trace/Gate Evidence | Known Gap |
| --- | --- | --- | --- | --- | --- |
| `no_forged_authority`: No forged authority | T1 | `s0_stubs_do_not_create_authority` [bounded_witness], `m1_no_forged_fdr` [bounded_witness], `m1_no_authority_amplification` [bounded_witness], `ft_capability_non_forgeability` [coverage] | `lnp64_m1_pingpong`, `lnp64_cap_engine`, `formal/rtl_assertions/lnp64_m1_assertions.sv` | `TRACE cap_dup`, `rights=`, `TRACE stale_pull`; `scripts/run_rtl_m1.sh`, `scripts/run_rtl_random_cosim.sh` | Not T2 yet: typed transition traces from the shared schema are missing. |
| `revocation_generation_safety`: Revocation/generation safety | T1 | `m1_cap_revoke_invalidates_generation` [bounded_witness], `m1_revoked_authority_cannot_start_new_work` [bounded_witness], `m5_revoked_generation_rejected` [bounded_witness], `ft_revocation_soundness` [coverage] | `lnp64_m1_pingpong`, `lnp64_m5_dma`, M1/M5 assertions | `TRACE stale_pull`, `TRACE revoked_submit`; M1/M5/random gates | No full lineage/revocation epoch transition or refinement proof across all object classes yet. |
| `domain_containment`: Domain containment | T1 | `m14_child_rights_subset_parent` [bounded_witness], `m14_child_budget_within_parent` [bounded_witness], `m14_policy_fail_closed` [bounded_witness], `ft_resource_domain_containment` [coverage] | `lnp64_m14_resource_domain_policy`, `lnp64_domain_engine`, M14 assertions | `TRACE delegate`, `child_rights=`, `TRACE policy`; M14/random gates | Cross-engine domain containment is not yet a T4 composition proof. |
| `dma_confined`: DMA confinement | T1 | `m5_dma_confined_to_capability_domain` [bounded_witness], `m5_pin_completes_with_authority` [bounded_witness], `m5_unpin_clears_pinned_state` [bounded_witness], `ft_dma_isolation` [coverage] | `lnp64_m5_dma`, `lnp64_dma_fabric`, M5 assertions | `TRACE dma_pin`, `TRACE dma_unpin`, `TRACE domain_isolation`, `TRACE coherence_flush`; M5/random gates | No full cache/TLB/DMA fabric refinement proof yet. |
| `scheduler_single_location`: Single scheduler location | T1 | `s0_every_live_thread_has_exactly_one_scheduler_location` [bounded_witness], `m7_exactly_one_scheduler_location` [bounded_witness], `ft_scheduler_safety` [coverage] | `lnp64_m7_futex_atomic`, `lnp64_scheduler`, S0/M7 assertions | `state=parked`, `woken=1`, `TRACE timer_wait`; S0/M7/random gates | No weighted-fair scheduler T3 transition proof yet. |
| `no_lost_wakeups`: No lost wakeups | T1 | `m7_no_lost_wakeup` [bounded_witness], `m7_futex_wake_delivered` [bounded_witness], `ft_no_lost_wakeups` [coverage] | `lnp64_m7_futex_atomic`, `lnp64_m1_pingpong`, M7/M1 assertions | `TRACE futex_wake`, `TRACE timer_expire`, `wake=2`; M7/M1/random gates | No full multi-source event-router refinement proof yet. |
| `servicelets_terminate_contained`: Servicelets terminate and stay contained | T1 | `m9_termination_by_construction` [bounded_witness], `m9_no_authority_creation` [bounded_witness], `m9_no_arbitrary_memory_access` [bounded_witness], `ft_classifier_servicelet_safety` [coverage] | `lnp64_m9_classifier_servicelet`, `lnp64_classifier_servicelet`, M9 assertions | `TRACE verifier`, `TRACE verifier_reject`, `TRACE budget_exhaust`; M9/random gates | Typed transition traces and extracted typed servicelet semantics are missing. |
| `faults_terminal_progress`: Faults reach terminal progress paths | T1 | `m10_adversarial_inputs_cannot_hang_owner_or_create_authority` [bounded_witness], `m10_bounded_local_fault_reaches_terminal_path` [bounded_witness], `m10_realtime_work_has_bounded_arbitration_progress` [bounded_witness], `ft_global_progress_bounded_faults` [coverage] | `lnp64_m10_ras`, `lnp64_watchdog`, M10 assertions | `TRACE watchdog_timeout`, `TRACE parity_poison`, `TRACE audit_mls`; M10/random gates | No full-chip T4 global progress refinement/composition proof yet. |

Validated by:

```sh
scripts/check_theorem_rtl_coupling.py
```
