# Stress Demo Catalog

This document lists candidate demo programs that are small enough to implement
as assembly or C smoke tests, but intentionally combine ISA features in awkward
ways. The expected outputs below are behavioral expectations for future tests,
not formal proof claims.

Use these as a menu. Each demo should be kept tiny, deterministic, and easy to
run under the emulator, RTL simulator, and eventually FPGA bring-up.

The most valuable early demos are short assembly programs. They should avoid
large libc dependencies, dynamic allocation unless testing `ALLOC`, complex text
parsing, and compatibility-personality policy. Prefer one or two threads, one
or two objects, fixed constants, and a single printed `ok ...` line.

## Conventions

- A demo prints `ok <name>` on success.
- A demo prints a compact failure code such as `fail <name> <step>` on first
  unexpected result.
- Expected fault cases should print the architectural error or delivery class
  observed by the program or supervisor shim.
- Any demo involving timing should print logical ordering, not wall-clock time.

## Focus: LNP64-Native ISA Features

This catalog should primarily stress what makes LNP64 unusual. Generic ALU,
branch, relocation, atomic, and ABI tests are still useful, but they should
serve the native architecture rather than dominate the demo set. The highest
value demos combine these LNP64-specific mechanisms:

| Native feature | What to stress | Representative demos |
| --- | --- | --- |
| FDR/capability machine | generation, sealing, narrowing, transfer, revocation | `stale_fdr_after_close`, `sealed_cap_cannot_delegate`, `cap_send_generation_race` |
| `PULL`/`PUSH` stream model | files, queues, pipes, sockets, directory streams as record/byte streams | `pull_after_writer_close`, `directory_stream_read`, `endpoint_rights_split` |
| `WAITABLE_PROBE` / `AWAIT_EX` | readiness probe, zero timeout, finite timeout, indefinite wait, no lost wakeups | `await_ready_before_park`, `await_zero_timeout_empty`, `revoke_while_polling_many` |
| `OBJECT_CTL` profiles | queue, counter, event, timer, memory object, call gate, DMA buffer | `object_ctl_pipe_alias`, `event_ctl_alias_multi_source`, `timer_ctl_alias_generation` |
| Resource Domains | VM/container/cgroup/sandbox as one primitive, freeze/resume/destroy, monotonic policy | `nested_quota_monotonic`, `destroy_domain_cancels_wait`, `nested_vm_container_pressure` |
| Hardware scheduler and barrel window | resident active window, skipped blocked TIDs, sticky affinity, no duplicate dispatch | `blocked_context_skipped`, `no_duplicate_tid_multicore`, `thread_window_spill_refill` |
| Native gates/deliveries | service calls, signal-like delivery, continuation tokens, sync/async/handoff calls | `sync_gate_return`, `gate_stale_continuation`, `eintr_blocked_await` |
| Service boundary | bounded requests, returned-capability proposals, service generation, no ambient privilege | `service_crash_pending_request`, `service_restart_stale_reply`, `cap_return_shape_bad` |
| Hardware heap | `ALLOC`, `ALLOC_EX`, `ALLOC_SIZE`, exact free, generation/hardening, domain policy | `heap_exact_pointer_free`, `alloc_disabled_domain`, `priority_inherited_heap_refill` |
| VMA/page engine | object-backed faults, COW, W^X/NX, JIT transition, revoke during fill | `jit_transition_ok`, `munmap_pending_fill`, `cow_fork_write` |
| DMA/IOMMU/device capabilities | DMA buffer generations, BAR `MMAP`, IRQ-as-event, coherent completion | `dma_completion_visibility`, `pcie_bar_mmap_page_aligned`, `irq_as_event_only` |
| Classifier/servicelets | bounded record classification, queue steering, verifier budget | `classifier_queue_steer`, `servicelet_budget_exhaust`, `servicelet_packet_to_gate` |
| Native metadata/control | `GET_META`/`SET_META`, bounded typed profiles, result prevalidation | `get_meta_set_meta_bounded`, `metadata_commit_cancel`, `result_prevalidation_no_side_effect` |
| PCR/env model | `ENV_GET`, read-only PCR rejection, TLS base, topology/scheduler constants | `env_get_feature_contract`, `set_pcr_readonly_denied`, `tls_errno_thread_local` |
| RAS/assurance | quotes, audit rings, trace scope, ECC/parity, watchdog/local reset | `quote_includes_domain`, `audit_tamper_evident`, `watchdog_local_reset` |
| Realtime ticket checks | rich setup policy reduced to compact hot-path tickets | `realtime_ticket_check_only`, `no_hot_policy_walk`, `network_event_to_frozen_domain` |

## Best Short Assembly Stress Demos

These are the first demos to implement as small `.s` files. They exercise unique
LNP64 instructions and should not require a real libc, filesystem, network
stack, loader personality, or complex service.

| Demo | Primary instructions | Scenario | Expected output |
| --- | --- | --- | --- |
| asm_env_get_contract | `ENV_GET`, `PUSH` | Read feature/topology/scheduler constants and print a compact mask. | `ok asm_env_get_contract` |
| asm_set_pcr_readonly | `SET_PCR`, `GET_PCR` | Attempt to write read-only PID/TID/topology selector. | `ok asm_set_pcr_readonly EPERM` |
| asm_tls_errno_two_threads | `SET_PCR`, `GET_PCR`, `CLONE`, `YIELD` | Two threads use distinct TLS/errno slots. | `ok asm_tls_errno_two_threads` |
| asm_object_queue_basic | `OBJECT_CTL`, `PUSH`, `PULL` | Create queue object, push fixed word, pull it back. | `ok asm_object_queue_basic 42` |
| asm_object_queue_full | `OBJECT_CTL`, `PUSH` | Create tiny queue, fill it, verify nonblocking overflow result. | `ok asm_object_queue_full EAGAIN` |
| asm_pipe_alias_queue | source alias over `OBJECT_CTL`, `PUSH`, `PULL` | Use pipe-profile queue and verify read/write endpoints. | `ok asm_pipe_alias_queue` |
| asm_wait_probe_empty | `WAITABLE_PROBE` | Probe an empty queue without parking. | `ok asm_wait_probe_empty not_ready` |
| asm_wait_ready_no_consume | `WAITABLE_PROBE`, `PULL` | Probe readable queue twice; one later pull still gets payload. | `ok asm_wait_ready_no_consume 42` |
| asm_await_ready_before_park | `AWAIT_EX`, `PUSH`, `PULL` | Make queue readable before await; verify no lost wake/park. | `ok asm_await_ready_before_park` |
| asm_await_zero_timeout | `AWAIT_EX` | Zero-timeout wait on empty queue. | `ok asm_await_zero_timeout ETIMEDOUT` |
| asm_await_timer | `OBJECT_CTL`, `AWAIT_EX` | Arm timer object and await expiry. | `ok asm_await_timer` |
| asm_revoke_wakes_waiter | `CAP_REVOKE`, `AWAIT_EX` | Thread waits on queue; another revokes it. | `ok asm_revoke_wakes_waiter EREVOKED` |
| asm_stale_fdr_token | `OBJECT_CTL`, `CAP_REVOKE`, `PULL` | Use saved FDR/cap token after object generation changes. | `ok asm_stale_fdr_token EREVOKED` |
| asm_cap_dup_narrow | `CAP_DUP`, `PUSH` | Duplicate cap with read-only rights; write through narrowed cap fails. | `ok asm_cap_dup_narrow EACCES` |
| asm_cap_seal_send_denied | `CAP_DUP`, `CAP_SEND` | Seal a cap and attempt further delegation. | `ok asm_cap_seal_send_denied EPERM` |
| asm_cap_send_recv | `CAP_SEND`, `CAP_RECV`, `PULL` | Parent sends queue read cap to child; child reads payload. | `ok asm_cap_send_recv 42` |
| asm_cap_send_revoke_race | `CAP_SEND`, `CAP_REVOKE`, `CAP_RECV` | Revoke sent cap before receiver installs it. | `ok asm_cap_send_revoke_race EREVOKED` |
| asm_gate_sync_return | `OBJECT_CTL`, `GATE_CALL`, `GATE_RETURN` | Enter call gate and return fixed value. | `ok asm_gate_sync_return 7` |
| asm_gate_stale_return | `GATE_RETURN` | Attempt to return twice with same continuation token. | `ok asm_gate_stale_return EREVOKED` |
| asm_gate_depth_limit | `GATE_CALL` | Re-enter gates past configured nesting depth. | `ok asm_gate_depth_limit EOVERFLOW` |
| asm_gate_async_event | `GATE_CALL`, `AWAIT_EX`, `PULL` | Async gate publishes completion event. | `ok asm_gate_async_event 7` |
| asm_delivery_mask_timer | `GATE_MASK_SET`, timer, `AWAIT_EX` | Mask timer delivery, unmask, verify delayed delivery. | `ok asm_delivery_mask_timer` |
| asm_div_fault_gate | arithmetic op, gate delivery | Divide by zero with native fault gate installed. | `ok asm_div_fault_gate fault=arith` |
| asm_illegal_opcode_upcall | disabled opcode, supervisor gate | Execute disabled opcode under supervisor-upcall policy. | `ok asm_illegal_opcode_upcall` |
| asm_domain_create_query | `DOMAIN_CTL` | Create child domain, query id/generation/budget. | `ok asm_domain_create_query` |
| asm_domain_freeze_resume | `DOMAIN_CTL`, `CLONE`, `YIELD` | Freeze child with runnable thread, then resume it. | `ok asm_domain_freeze_resume` |
| asm_domain_destroy_waiter | `DOMAIN_CTL`, `AWAIT_EX` | Destroy child domain while its thread is waiting. | `ok asm_domain_destroy_waiter ECANCELED` |
| asm_domain_quota_denied | `DOMAIN_CTL` | Request child budget above parent budget. | `ok asm_domain_quota_denied EQUOTA` |
| asm_domain_frozen_cap_send | `DOMAIN_CTL`, `CAP_SEND` | Try to send capability into frozen domain. | `ok asm_domain_frozen_cap_send EAGAIN` |
| asm_barrel_blocked_skip | `CLONE`, `AWAIT_EX`, `YIELD` | One resident TID blocks; sibling continues and increments counter. | `ok asm_barrel_blocked_skip` |
| asm_no_duplicate_wake | `AWAIT_EX`, event queue | Two event sources attempt to wake same TID. | `ok asm_no_duplicate_wake one` |
| asm_sticky_affinity | scheduler profile, `YIELD` | Repeated yields should keep thread on preferred eligible tile. | `ok asm_sticky_affinity` |
| asm_alloc_free_basic | `ALLOC`, `FREE` | Allocate small object, store/load word, free exact pointer. | `ok asm_alloc_free_basic` |
| asm_alloc_interior_free | `ALLOC`, `FREE` | Free pointer inside allocation. | `ok asm_alloc_interior_free EINVAL` |
| asm_alloc_double_free | `ALLOC`, `FREE` | Free same object twice. | `ok asm_alloc_double_free EREVOKED` |
| asm_alloc_disabled_domain | `DOMAIN_CTL`, `ALLOC` | Child domain with heap disabled attempts allocation. | `ok asm_alloc_disabled_domain EPERM` |
| asm_alloc_size_query | `ALLOC`, `ALLOC_SIZE`, `FREE` | Query allocated usable size and verify sane bound. | `ok asm_alloc_size_query` |
| asm_mmap_anon_guard | `MMAP`, load/store | Map guarded region and fault on guard access. | `ok asm_mmap_anon_guard fault=guard` |
| asm_mprotect_wx_denied | `MMAP`, `MPROTECT` | Request write+execute mapping without policy. | `ok asm_mprotect_wx_denied EPERM` |
| asm_jit_isync | `MMAP`, store, `MPROTECT`, `ISYNC`, branch/call | Write tiny code, switch W to X, sync, execute. | `ok asm_jit_isync 42` |
| asm_munmap_stale_access | `MMAP`, `MUNMAP`, load | Touch address after unmap. | `ok asm_munmap_stale_access fault` |
| asm_cow_clone_write | `CLONE`, `MMAP`, store/load | Parent/child write COW page and observe isolation. | `ok asm_cow_clone_write` |
| asm_dma_copy_completion | `DMA_CTL`, `AWAIT_EX` | Submit bounded DMA copy and await completion event. | `ok asm_dma_copy_completion` |
| asm_dma_stale_buffer | `DMA_CTL`, `CAP_REVOKE` | Reuse DMA descriptor after buffer generation changes. | `ok asm_dma_stale_buffer EREVOKED` |
| asm_irq_event_only | device stub event, `AWAIT_EX` | Inject device event and observe event record, not raw interrupt. | `ok asm_irq_event_only` |
| asm_classifier_two_queues | classifier/servicelet ctl, `PULL` | Install simple classifier mapping two tags to two queues. | `ok asm_classifier_two_queues` |
| asm_servicelet_budget | servicelet ctl | Load bounded servicelet that exceeds budget. | `ok asm_servicelet_budget EBOUND` |
| asm_quote_minimal | quote/attestation op | Request quote over root and child domain ids. | `ok asm_quote_minimal` |
| asm_trace_scope_denied | trace/control cap | Unauthorized trace read for another domain. | `ok asm_trace_scope_denied EPERM` |
| asm_watchdog_fault_event | watchdog inject, event queue | Inject local watchdog event and read structured fault. | `ok asm_watchdog_fault_event` |

## Short Assembly Cross-Feature Torture

These are still small, but combine multiple native surfaces. They are good
second-wave assembly tests after the single-feature smoke cases work.

| Demo | Primary instructions | Scenario | Expected output |
| --- | --- | --- | --- |
| asm_revoke_while_await_many | `AWAIT_EX`, `CAP_REVOKE`, event queue | Wait on several sources; revoke one while another becomes ready. | `ok asm_revoke_while_await_many ready+revoked` |
| asm_freeze_during_gate | `DOMAIN_CTL`, `GATE_CALL` | Freeze target domain while caller is parked in sync gate. | `ok asm_freeze_during_gate parked_or_canceled` |
| asm_signal_during_alloc_refill | `ALLOC`, gate delivery | Delivery arrives while thread is parked on heap refill. | `ok asm_signal_during_alloc_refill defined` |
| asm_mmap_fault_during_freeze | `MMAP`, domain freeze | Start page-fill/fault path, freeze domain before completion. | `ok asm_mmap_fault_during_freeze no_commit` |
| asm_exec_kills_siblings | `CLONE`, `EXEC` | Multithreaded process executes new image; siblings stop. | `ok asm_exec_kills_siblings one_survivor` |
| asm_cap_transfer_during_exec | `CAP_SEND`, `EXEC` | Capability arrives during exec with close-on-exec policy. | `ok asm_cap_transfer_during_exec once` |
| asm_dma_pin_then_munmap | `DMA_CTL`, `MUNMAP` | Attempt unmap while buffer is DMA-pinned. | `ok asm_dma_pin_then_munmap EBUSY` |
| asm_network_event_frozen_domain | classifier, domain freeze | Packet event targets frozen domain and must not dispatch. | `ok asm_network_event_frozen_domain queued` |
| asm_no_hot_policy_walk | event delivery instrumentation | Deliver event using precomputed ticket only. | `ok asm_no_hot_policy_walk resident_ticket` |
| asm_priority_inherited_async | `ALLOC`, scheduler metadata | RT thread's async refill beats background refill. | `ok asm_priority_inherited_async rt_first` |

## Native Control and Metadata Demos

| Demo | Features | Scenario | Expected output |
| --- | --- | --- | --- |
| waitable_probe_then_await | `WAITABLE_PROBE`, `AWAIT_EX` | Probe empty queue, then arm wait, then producer pushes record. | `ok waitable_probe_then_await not_ready ready payload=42` |
| waitable_probe_no_consume | `WAITABLE_PROBE`, `PULL` | Probe readable queue twice, then pull one record. | `ok waitable_probe_no_consume probes=ready,ready payload=42` |
| object_ctl_pipe_alias | `OBJECT_CTL`, queue profile | Source-level `pipe()` lowers to queue-profile object creation. | `ok object_ctl_pipe_alias read=hello eof` |
| event_ctl_alias_multi_source | `OBJECT_CTL`, event profile | Source-level `EVENT_CTL` alias binds queue and timer sources. | `ok event_ctl_alias_multi_source queue timer` |
| timer_ctl_alias_generation | `OBJECT_CTL`, timer profile | Recreate timer and deliver old expiry record. | `ok timer_ctl_alias_generation stale_ignored` |
| get_meta_set_meta_bounded | `GET_META`, `SET_META` | Read metadata, update a bounded flag, reject oversized request. | `ok get_meta_set_meta_bounded updated EOVERFLOW` |
| result_prevalidation_no_side_effect | result register validation | Issue mutating op with locked/unwritable result destination. | `ok result_prevalidation_no_side_effect no_mutation` |
| native_error_negative_errno | native result convention | Trigger native authority failure and verify canonical negative error result. | `ok native_error_negative_errno -EPERM` |
| object_ctl_unknown_profile | `OBJECT_CTL` extensibility | Send well-formed unknown object profile/op. | `ok object_ctl_unknown_profile ENOTSUP` |
| malformed_control_record | typed control envelope | Send malformed size/version/shape record. | `ok malformed_control_record EINVAL` |
| env_get_feature_contract | `ENV_GET` | Read ISA, scheduler, topology, realtime, heap, and servicelet constants. | `ok env_get_feature_contract features=<mask>` |
| set_pcr_readonly_denied | PCR permissions | Attempt to write read-only PID/TID/topology PCR. | `ok set_pcr_readonly_denied EPERM` |
| pcr_tls_base_errno | PCR/TLS | Set allowed TLS-base selector and verify thread-local errno storage. | `ok pcr_tls_base_errno errno_is_thread_local` |

## Capability and FDR Edge Cases

| Demo | Features | Scenario | Expected output |
| --- | --- | --- | --- |
| stale_fdr_after_close | FDR, generation checks, `PULL` | Open queue, close/revoke it, try to pull using saved token. | `ok stale_fdr_after_close EREVOKED` |
| dup_then_revoke_parent | `CAP_DUP`, `CAP_REVOKE`, lineage | Duplicate a narrowed readable cap, revoke root lineage, use duplicate. | `ok dup_then_revoke_parent EREVOKED` |
| sealed_cap_cannot_delegate | sealed caps, `CAP_SEND` | Seal a cap, attempt to send or duplicate with broader rights. | `ok sealed_cap_cannot_delegate EPERM` |
| narrowed_write_denied | rights masks, `PUSH` | Narrow queue cap to read-only, then attempt `PUSH`. | `ok narrowed_write_denied EACCES` |
| receive_into_live_slot | `CAP_RECV`, destination FDR table | Receive a cap into an occupied descriptor without explicit overwrite flag. | `ok receive_into_live_slot EBUSY` |
| cap_send_to_frozen_domain | Resource Domain freeze, cap transfer | Freeze child domain, attempt to send it a capability. | `ok cap_send_to_frozen_domain EAGAIN` |
| cap_send_generation_race | `CAP_SEND`, revoke, generation | Queue cap transfer, revoke source before receiver commits. | `ok cap_send_generation_race EREVOKED` |
| cap_return_shape_bad | service reply, returned caps | Service returns a capability in an undeclared return slot. | `ok cap_return_shape_bad EINVAL` |
| cap_lineage_depth | nested narrow/dup | Create several narrowed descendants and revoke middle lineage. | `ok cap_lineage_depth` |
| fd_table_expand_boundary | FDR table, DDR-backed growth | Allocate around default FDR table size and verify descriptors remain valid. | `ok fd_table_expand_boundary count=<n>` |

## Object, Queue, and Waitable Demos

| Demo | Features | Scenario | Expected output |
| --- | --- | --- | --- |
| await_ready_before_park | `AWAIT_EX`, queue readiness | Make queue readable, then call `AWAIT_EX`; must not park forever. | `ok await_ready_before_park ready` |
| await_zero_timeout_empty | `AWAIT_EX`, timeout | Probe empty queue with zero timeout. | `ok await_zero_timeout_empty ETIMEDOUT` |
| await_bounded_timeout | timer, waitable | Wait on empty queue with finite timeout. | `ok await_bounded_timeout ETIMEDOUT` |
| await_indefinite_wake | `AWAIT_EX`, `PUSH` | Parent waits indefinitely; child pushes one record. | `ok await_indefinite_wake payload=42` |
| queue_full_eagain | queue profile, nonblocking | Fill fixed queue, nonblocking `PUSH` one more record. | `ok queue_full_eagain EAGAIN` |
| queue_full_park_then_wake | queue profile, scheduler | Fill queue, writer parks, reader drains one record. | `ok queue_full_park_then_wake writer_resumed` |
| queue_revoke_wakes_waiter | queue revoke, waitable | Thread waits on empty queue; another thread revokes queue. | `ok queue_revoke_wakes_waiter EREVOKED` |
| counter_wait_exact | counter object, wait predicate | Wait until counter reaches exact value after several increments. | `ok counter_wait_exact value=5` |
| counter_overflow_policy | counter object, overflow | Increment counter at max under selected overflow profile. | `ok counter_overflow_policy EOVERFLOW` |
| event_queue_overflow | event queue, overflow record | Produce more events than capacity and read overflow marker. | `ok event_queue_overflow dropped=<n>` |
| event_generation_stale | event source generation | Arm event source, destroy/recreate source, deliver stale event. | `ok event_generation_stale ignored` |
| timer_cancel_race | timer object, cancel | Arm timer, cancel near expiry, verify one clear result. | `ok timer_cancel_race canceled_or_expired_once` |
| timer_periodic_coalesce | timer, event queue | Periodic timer fires faster than reader drains. | `ok timer_periodic_coalesce count=<n> coalesced=<m>` |
| pull_after_writer_close | queue/pipe profile | Close writer side, reader drains data then sees EOF. | `ok pull_after_writer_close data eof` |

## Domains, Scheduling, and Realtime

| Demo | Features | Scenario | Expected output |
| --- | --- | --- | --- |
| frozen_domain_no_dispatch | `DOMAIN_CTL`, scheduler | Freeze a domain with runnable thread; verify it stops retiring. | `ok frozen_domain_no_dispatch` |
| resume_domain_dispatch | `DOMAIN_CTL`, scheduler | Resume frozen domain; thread continues from parked state. | `ok resume_domain_dispatch` |
| destroy_domain_cancels_wait | domain destroy, waitables | Child waits on queue; parent destroys domain. | `ok destroy_domain_cancels_wait ECANCELED` |
| domain_quota_exhaustion | domain budget, scheduler | Child consumes CPU quota and becomes ineligible until period refill. | `ok domain_quota_exhaustion throttled resumed` |
| nested_quota_monotonic | nested domains | Child asks for more CPU/memory budget than parent. | `ok nested_quota_monotonic EQUOTA` |
| domain_attach_busy_thread | domain attach, scheduler state | Move a running thread subtree while it has an in-flight engine op. | `ok domain_attach_busy_thread EBUSY` |
| sticky_affinity | scheduler affinity | Thread yields repeatedly and should stay on preferred tile when eligible. | `ok sticky_affinity tile=<same>` |
| forced_migration_boundary | affinity update, migration | Change affinity while thread is running; migration happens only at boundary. | `ok forced_migration_boundary no_mid_instruction_move` |
| no_duplicate_tid_multicore | scheduler uniqueness | Wake same TID from two event sources on different tiles. | `ok no_duplicate_tid_multicore one_dispatch` |
| priority_inherited_heap_refill | realtime, heap Class D | RT thread triggers heap refill behind background refills. | `ok priority_inherited_heap_refill rt_first` |
| async_dead_work_cancel | Class D cancellation | Submit async VMA/heap work, kill thread, verify work is canceled. | `ok async_dead_work_cancel canceled` |
| realtime_ticket_check_only | domains, event delivery | Deliver event to RT thread with precomputed rights. | `ok realtime_ticket_check_only bounded_path` |
| domain_freeze_during_gate | domain freeze, gate call | Caller enters sync gate; target domain freezes before return. | `ok domain_freeze_during_gate parked_or_canceled` |
| domain_generation_after_restore | snapshot hooks, generation | Simulate restore with fresh domain generation; stale operation completes. | `ok domain_generation_after_restore stale_rejected` |

## Barrel Processor and Thread Window

| Demo | Features | Scenario | Expected output |
| --- | --- | --- | --- |
| two_context_interleave | barrel issue, two TIDs | Two threads increment separate counters with yields. | `ok two_context_interleave a=<n> b=<n>` |
| blocked_context_skipped | barrel issue, waitable | One TID blocks on queue; second TID continues retiring. | `ok blocked_context_skipped progress=<n>` |
| branch_redirect_one_tid | branch, barrel metadata | One TID misbranches/redirects while another TID is in flight. | `ok branch_redirect_one_tid other_tid_unaffected` |
| fault_one_tid_continues | fault delivery, barrel | One TID causes guard fault; sibling TID keeps running. | `ok fault_one_tid_continues` |
| engine_completion_wrong_tid | op_id/TID matching | Deliver completion with wrong TID or generation. | `ok engine_completion_wrong_tid ignored` |
| thread_window_spill_refill | active window, DDR spill | More runnable TIDs than resident window; all complete eventually. | `ok thread_window_spill_refill completed=<n>` |
| yield_fairness_smoke | `YIELD`, scheduler | Several equal-weight TIDs yield in a loop. | `ok yield_fairness_smoke counts_near_equal` |
| high_weight_advances | scheduler weights | High-weight and low-weight threads compete for CPU. | `ok high_weight_advances high>low` |

## Memory, VMA, Heap, and Protection

| Demo | Features | Scenario | Expected output |
| --- | --- | --- | --- |
| guard_page_overflow | heap guard, fault delivery | Allocate guarded object, write one word past end. | `ok guard_page_overflow fault=guard` |
| nx_data_execute | NX data, fault gate | Try to jump into data page. | `ok nx_data_execute SIGSEGV_OR_EXEC_FAULT` |
| wx_denied | W^X, `MPROTECT` | Request writable+executable mapping without JIT authority. | `ok wx_denied EPERM` |
| jit_transition_ok | W^X, `ISYNC`, JIT policy | Write code, switch W->X, invalidate I-cache, execute. | `ok jit_transition_ok result=42` |
| jit_no_isync_fault_or_old | I-cache sync | Patch executable page without `ISYNC`; verify defined stale/fault behavior. | `ok jit_no_isync_fault_or_old defined` |
| mmap_object_revoke | object-backed VMA | Map object, revoke object, then touch mapping. | `ok mmap_object_revoke EREVOKED_OR_FAULT` |
| munmap_pending_fill | VMA, object page fill | Trigger object-backed page fill, `MUNMAP` before fill reply. | `ok munmap_pending_fill fill_rejected` |
| cow_fork_write | `CLONE`/fork, COW | Parent/child write same COW page after fork. | `ok cow_fork_write parent=1 child=2` |
| exec_barrier_threads | `EXEC`, multithread | One sibling loops while another execs; old siblings must die/park. | `ok exec_barrier_threads one_survivor` |
| heap_exact_pointer_free | `ALLOC`, `FREE` | Free interior pointer into an allocation. | `ok heap_exact_pointer_free EINVAL` |
| heap_double_free | heap generation | Free same allocation twice. | `ok heap_double_free EREVOKED_OR_EINVAL` |
| heap_realloc_pressure | heap windows, refill | Allocate/free mixed sizes across window boundary. | `ok heap_realloc_pressure live_bytes=<n>` |
| alloc_disabled_domain | heap policy, domain | Disable heap allocation in child domain and call `ALLOC`. | `ok alloc_disabled_domain EPERM` |
| alloc_large_to_vma | heap/VMA interaction | Allocate large object that routes through VMA path. | `ok alloc_large_to_vma mapped freed` |
| dma_pin_then_munmap | DMA pin, VMA revoke | Pin buffer for DMA, attempt `MUNMAP` while pinned. | `ok dma_pin_then_munmap EBUSY_OR_REVOKING` |
| dma_completion_visibility | DMA, coherence | DMA writes buffer, completion fires, CPU reads new value. | `ok dma_completion_visibility value=<expected>` |
| stale_dma_descriptor | DMA generation | Reuse old DMA descriptor after buffer generation changes. | `ok stale_dma_descriptor EREVOKED` |

## Gates, Calls, Faults, and Signal Profile

| Demo | Features | Scenario | Expected output |
| --- | --- | --- | --- |
| sync_gate_return | `GATE_CALL`, `GATE_RETURN` | Caller enters service gate and receives return value. | `ok sync_gate_return value=7` |
| async_gate_completion | async gate, event queue | Async call returns completion through event object. | `ok async_gate_completion value=7` |
| handoff_gate_no_return | handoff call | Caller transfers continuation/ownership and should not resume normally. | `ok handoff_gate_no_return target_completed` |
| gate_stale_continuation | continuation generation | Return twice from same gate continuation token. | `ok gate_stale_continuation EREVOKED` |
| gate_depth_limit | bounded nesting | Recursively enter gates past configured depth. | `ok gate_depth_limit EOVERFLOW` |
| gate_target_revoked | gate revoke | Revoke gate while call is queued. | `ok gate_target_revoked ECANCELED` |
| divide_by_zero_delivery | fault as gate/signal | Divide by zero with registered fault gate. | `ok divide_by_zero_delivery class=fault code=arith` |
| illegal_opcode_supervisor | opcode upcall | Execute disabled opcode in domain with supervisor upcall policy. | `ok illegal_opcode_supervisor upcall_seen` |
| illegal_opcode_no_supervisor | fault delivery | Execute disabled opcode without supervisor policy. | `ok illegal_opcode_no_supervisor SIGILL` |
| masked_timer_delivery | gate mask, timer | Mask timer delivery, fire timer, unmask. | `ok masked_timer_delivery delivered_after_unmask` |
| fatal_default_action | fault default | Cause unhandled fatal memory fault. | `ok fatal_default_action child_dead` |
| eintr_blocked_await | signal profile, `AWAIT` | Thread blocked in interruptible wait receives handled delivery. | `ok eintr_blocked_await EINTR handler_ran` |

## Networking and Classifier/Servicelet Demos

| Demo | Features | Scenario | Expected output |
| --- | --- | --- | --- |
| classifier_queue_steer | classifier, queues | Install table steering two packet classes to two queues. | `ok classifier_queue_steer q0=<n> q1=<n>` |
| classifier_stale_table | classifier generation | Replace classifier table while packets from old generation arrive. | `ok classifier_stale_table old_dropped_or_rescanned` |
| servicelet_budget_exhaust | servicelet verifier/budget | Servicelet loops or exceeds instruction budget. | `ok servicelet_budget_exhaust EBOUND` |
| servicelet_no_cap_mint | servicelet sandbox | Servicelet attempts forbidden capability operation. | `ok servicelet_no_cap_mint rejected` |
| packet_queue_revoke_waiter | network queue, waitable | Wait for packets; revoke queue capability. | `ok packet_queue_revoke_waiter EREVOKED` |
| listener_accept_cap | listener object, cap return | Listener accepts connection and returns endpoint capability. | `ok listener_accept_cap endpoint_live` |
| endpoint_rights_split | endpoint caps | Split endpoint into read-only/write-only caps and test both. | `ok endpoint_rights_split` |
| tcp_friendly_ordering | packet queues, ordering | Send ordered records through simplified endpoint path. | `ok tcp_friendly_ordering in_order` |
| multicast_two_domains | queue steering, domains | Same classified record delivered to two authorized domains. | `ok multicast_two_domains a=1 b=1` |
| unauthorized_queue_steer | domain policy, classifier | Domain tries to steer packets to queue it does not own. | `ok unauthorized_queue_steer EPERM` |

## PCIe, Device, and Interrupt-Abstraction Demos

| Demo | Features | Scenario | Expected output |
| --- | --- | --- | --- |
| pcie_bar_mmap_page_aligned | PCIe BAR cap, `MMAP` | Map page-aligned BAR capability. | `ok pcie_bar_mmap_page_aligned mapped` |
| pcie_bar_subpage_denied | BAR page rule | Attempt to mint/map sub-page BAR range. | `ok pcie_bar_subpage_denied EINVAL` |
| pcie_bar_no_secondary_role | pure capability | Holder of valid BAR cap maps it without driver-domain flag. | `ok pcie_bar_no_secondary_role mapped` |
| irq_as_event_only | interrupt abstraction | Device interrupt appears as event queue record, not raw vector. | `ok irq_as_event_only event_seen raw_irq_hidden` |
| iommu_wrong_domain | IOMMU, DMA | Device attempts DMA to buffer belonging to wrong domain. | `ok iommu_wrong_domain fault` |
| bus_master_mints_bar | Bus Master domain | Trusted Bus Master configures device and delegates BAR cap. | `ok bus_master_mints_bar cap_received` |
| revoked_bar_mapping | BAR cap revoke, VMA | Revoke BAR cap while mapped; next access faults or mapping invalidates. | `ok revoked_bar_mapping revoked` |
| write_combining_bar | memory type | Map framebuffer-like BAR as write-combining and issue ordered fence. | `ok write_combining_bar fence_complete` |

## Storage, Services, and Namespace Boundary

| Demo | Features | Scenario | Expected output |
| --- | --- | --- | --- |
| openat_service_roundtrip | `OPEN_AT`, service boundary | Namespace service resolves simple path and returns object cap. | `ok openat_service_roundtrip fd=<n>` |
| openat_no_path_in_hardware | namespace policy | Verify hardware sees bounded service request, not path-walk authority. | `ok openat_no_path_in_hardware service_request` |
| service_crash_pending_request | service generation | Service dies while request pending; caller receives fault/cancel. | `ok service_crash_pending_request ECANCELED` |
| service_restart_stale_reply | service generation | Old service instance replies after restart. | `ok service_restart_stale_reply ignored` |
| object_page_fill_service | object-backed page fill | Map service-backed object, fault page, service supplies page. | `ok object_page_fill_service value=<expected>` |
| object_fill_bad_generation | page fill generation | Service replies with wrong object/VMA generation. | `ok object_fill_bad_generation rejected` |
| storage_barrier_order | storage barrier | Write record, issue barrier, observe ordered completion. | `ok storage_barrier_order committed` |
| metadata_commit_cancel | object metadata | Start metadata update and cancel before commit point. | `ok metadata_commit_cancel old_state` |
| flush_after_revoke | storage/object revoke | Try to flush object after its generation is revoked. | `ok flush_after_revoke EREVOKED` |
| directory_stream_read | stream model | Read directory as typed stream records through `PULL`. | `ok directory_stream_read entries=<n>` |

## ABI, Compiler, and Compute Baseline

These are supporting demos. They matter because LLVM, libc, and the RTL core
must be boring and reliable, but they are not the distinctive LNP64 tests. They
should usually run after the native capability/domain/waitable/gate smoke tests.

| Demo | Features | Scenario | Expected output |
| --- | --- | --- | --- |
| sext_zext_matrix | `SEXT`, `ZEXT` | Exhaust edge values for byte/half/word extension. | `ok sext_zext_matrix` |
| div_rem_signed_unsigned | `DIV`, `UDIV`, `SREM`, `UREM` | Exercise signed and unsigned edge cases excluding undefined C traps. | `ok div_rem_signed_unsigned` |
| mulh_matrix | `MULH`, `MULHU`, `MULHSU` | Compare high multiply results against software reference. | `ok mulh_matrix` |
| bitops_matrix | `CLZ`, `CTZ`, `POPCNT`, rotate, bswap | Run edge cases: zero, one, high bit, alternating bits. | `ok bitops_matrix` |
| csel_no_branch | `CSEL`, flags | Compute min/max/clamp without branch and verify result. | `ok csel_no_branch` |
| auipc_reloc_smoke | address materialization | Access local/global symbols through canonical PC-relative sequence. | `ok auipc_reloc_smoke` |
| tls_errno_thread_local | TLS, PCR/env | Two threads set `errno` independently. | `ok tls_errno_thread_local a=<ea> b=<eb>` |
| call_abi_clobbers | psABI, call lowering | Function/gate call preserves callee-saved registers. | `ok call_abi_clobbers` |
| native_call_error_shape | native call ABI | Failed native capability call returns canonical negative errno. | `ok native_call_error_shape -EPERM` |
| atomic_fetch_add | AMO, memory model | Several threads increment one counter with AMO add. | `ok atomic_fetch_add count=<expected>` |
| cmpxchg_contention | `LOCK_CMPXCHG`, futex | CAS loop under contention with fallback wait. | `ok cmpxchg_contention count=<expected>` |
| fence_dma_device_order | `FENCE`, DMA/device ordering | Write DMA descriptor, fence, ring doorbell, read completion. | `ok fence_dma_device_order` |
| memory_order_message | acquire/release | Producer writes data then flag; consumer sees data after flag. | `ok memory_order_message` |

## Attestation, Audit, RAS, and Recovery

| Demo | Features | Scenario | Expected output |
| --- | --- | --- | --- |
| quote_includes_domain | attestation | Create child domain and request quote. | `ok quote_includes_domain domain=<id> gen=<g>` |
| audit_tamper_evident | audit ring | Emit two audited events and verify monotonic sequence/hash link. | `ok audit_tamper_evident seq=2` |
| trace_scope_denied | trace capability | Unauthorized domain tries to read another domain's trace. | `ok trace_scope_denied EPERM` |
| counter_scope_allowed | observability | Authorized domain reads own counters only. | `ok counter_scope_allowed own_only` |
| ecc_correctable_event | metadata ECC/parity | Inject correctable metadata fault. | `ok ecc_correctable_event corrected fault_record` |
| ecc_uncorrectable_poison | metadata poison | Inject uncorrectable object metadata fault and attempt reuse. | `ok ecc_uncorrectable_poison poisoned` |
| watchdog_local_reset | watchdog, local reset | Hang one engine in test mode; watchdog resets local engine only. | `ok watchdog_local_reset other_engine_alive` |
| degraded_rejects_new_cmd | degraded state | Engine enters degraded/recovery state and rejects normal commands. | `ok degraded_rejects_new_cmd EAGAIN_OR_EFAULT` |
| machine_fatal_measured | fatal escalation | Force unrecoverable top-level invariant failure in test mode. | `ok machine_fatal_measured fatal_record` |
| snapshot_quiesce_hooks | snapshot hooks | Freeze/quiesce child domain and query bounded state cursors. | `ok snapshot_quiesce_hooks cursors=<n>` |
| restore_fresh_generations | restore hooks | Restore-like replay creates fresh generations; old caps fail. | `ok restore_fresh_generations stale_rejected` |

## Cross-Feature Torture Demos

| Demo | Features | Scenario | Expected output |
| --- | --- | --- | --- |
| fork_with_pending_gate | `CLONE`, gate, continuation | Fork while parent has pending but unentered gate delivery. | `ok fork_with_pending_gate child_no_parent_continuation` |
| exec_with_mapped_dma | `EXEC`, DMA pin, VMA | Exec process while sibling has pinned DMA buffer. | `ok exec_with_mapped_dma canceled_or_busy` |
| revoke_while_polling_many | `AWAIT_EX`, cap revoke | Poll many queues; revoke one while event arrives on another. | `ok revoke_while_polling_many event=<good> revoked=<bad>` |
| domain_destroy_with_service_request | domain, service | Destroy caller domain with pending filesystem/service request. | `ok domain_destroy_with_service_request request_canceled` |
| signal_during_heap_refill | gate delivery, heap | Delivery arrives while thread is parked on heap refill. | `ok signal_during_heap_refill EINTR_or_after_refill` |
| mmap_fault_during_freeze | VMA page fault, domain freeze | Page fault starts object fill; domain freezes before reply. | `ok mmap_fault_during_freeze parked_no_commit` |
| network_event_to_frozen_domain | packet queue, domain freeze | Packet arrives for frozen domain. | `ok network_event_to_frozen_domain queued_not_dispatched` |
| servicelet_packet_to_gate | classifier, servicelet, gate | Packet classified to queue that triggers gate call. | `ok servicelet_packet_to_gate gate_seen payload=<n>` |
| allocator_pressure_with_rt | heap, scheduler, realtime | Background heap churn while RT thread alloc/free small objects. | `ok allocator_pressure_with_rt rt_within_bound` |
| cache_dma_revoke_race | cache, DMA, revoke | CPU caches buffer, DMA writes, cap revoked near completion. | `ok cache_dma_revoke_race no_stale_authority` |
| cap_transfer_after_exec | exec, FDR inheritance | Send cap to process while it execs with close-on-exec rules. | `ok cap_transfer_after_exec delivered_or_rejected_once` |
| nested_vm_container_pressure | domains, scheduler, heap | VM-like domain contains container-like child under pressure. | `ok nested_vm_container_pressure isolated` |
| mls_declass_gate | labels, gates, audit | High domain sends through authorized declassification gate. | `ok mls_declass_gate audited` |
| forbidden_mls_direct_queue | labels, queue | High domain tries direct queue send to low without gate. | `ok forbidden_mls_direct_queue EPERM` |
| debug_mode_generation | controlled debug | Enable debug for domain, revoke/debug generation, try old debug cap. | `ok debug_mode_generation EREVOKED` |
| attested_migration_stub | quote, snapshot hooks | Quiesce domain, emit migration quote, resume. | `ok attested_migration_stub quote resume` |
| boot_policy_no_raw_interrupt | boot policy, device | Device event after boot must appear as event record only. | `ok boot_policy_no_raw_interrupt raw_hidden` |
| no_hot_policy_walk | domains, event | Instrument event delivery to confirm only ticket fields are read. | `ok no_hot_policy_walk resident_ticket` |

## Suggested First Batch

Start with short assembly demos that cover LNP64's unique mechanisms:

1. `asm_env_get_contract`
2. `asm_set_pcr_readonly`
3. `asm_object_queue_basic`
4. `asm_wait_probe_empty`
5. `asm_wait_ready_no_consume`
6. `asm_await_ready_before_park`
7. `asm_stale_fdr_token`
8. `asm_cap_dup_narrow`
9. `asm_cap_send_recv`
10. `asm_gate_sync_return`
11. `asm_gate_stale_return`
12. `asm_domain_create_query`
13. `asm_domain_freeze_resume`
14. `asm_domain_destroy_waiter`
15. `asm_barrel_blocked_skip`
16. `asm_no_duplicate_wake`
17. `asm_alloc_free_basic`
18. `asm_alloc_interior_free`
19. `asm_mmap_anon_guard`
20. `asm_mprotect_wx_denied`
21. `asm_dma_copy_completion`
22. `asm_irq_event_only`
23. `asm_revoke_while_await_many`
24. `asm_freeze_during_gate`
25. `asm_no_hot_policy_walk`

This first batch covers feature discovery, PCR permissions, object profiles,
waitable semantics, capability generations, cap transfer, gates/continuations,
Resource Domains, barrel scheduling, native heap behavior, VMA protection, DMA
completion, interrupt abstraction, and the realtime ticket-check rule. C demos
and larger service/personality demos should follow once the assembly-level
native mechanisms are stable.
