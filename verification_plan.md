# LNP64 Verification Plan

Verification starts at the architectural model, then refines through emulator
tests, RTL assertions, simulation, and FPGA bring-up.

## Model-Level Targets

- Instruction encoding/decoding golden model.
- Abstract machine model for FDRs, Resource Domains, VMAs, waitables, DMA,
  scheduler state, and commit/abort transitions.
- Proof or exhaustive-test targets from `formal_theorems.md`.
- Conformance targets from `conformance_matrix.md`.

## Directed Emulator/RTL Tests

Core ISA:

- ALU, branch, call/return, load/store, atomics, `FENCE`, `ISYNC`.
- `ENV_GET` scalar keys, buffer keys, bad keys, and buffer faults.
- `ENV_GET` WCET discovery: latency class bounds, Class D submit bound,
  metadata/event/memory fabric wait bounds, servicelet lane limits, and absent
  feature behavior.
- Instruction-class miss behavior: hot success, bounded canonical error,
  explicit park, and `EINPROGRESS`/completion-token paths for cold FDR, VMA,
  heap window, gate continuation, waitable, scheduler slot, domain record, and
  servicelet attachment misses.
- Native fault gate delivery and POSIX signal-profile mapping: `SIGFPE`,
  `SIGILL`, `SIGSEGV`, `SIGBUS`, `SIGTRAP`.

Resource primitives:

- `PULL`, `PUSH`, `AWAIT`, `CAP_*`, `OBJECT_CTL`, `DOMAIN_CTL`, `GATE_CALL`,
  `GATE_RETURN`, `GATE_DELIVER`, `DMA_CTL`, `ALLOC`, `FREE`, `MMAP`, `MUNMAP`,
  `MPROTECT`.
- `OBJECT_CTL` profiles for `counter`, `queue`, and `memory_object`.
- Runtime profiles for pipe, semaphore, completion, channel, task event, shared
  arena, and DMA completion.

Capability and domain safety:

- FDR generation checks, lineage epochs, narrowing, sealing, transfer, revoke.
- Returned-capability proposals as data until Capability Engine commit.
- Nested Resource Domain create/destroy/freeze/resume/query.
- Monotonic limits, hierarchical accounting, stale generation rejection, and
  delegated authority bounds.

Memory:

- VMA state transitions, anonymous zero/COW, page-fill transactions,
  object-backed retry/error replies, dirty-range enumeration, executable
  provenance, W^X/NX, guard pages, ASLR, TLB/I-cache invalidation, and
  pending-fill cancellation.
- Heap allocation: invalid free, stale pointer, cross-thread free, COW/exec
  teardown, `ALLOC_SIZE`, and software-owned arena boundaries.

Scheduler and waits:

- Weighted-fair charging, virtual runtime/deadline ordering, quotas,
  hierarchy rollup, wakeup insertion, frozen-domain removal, bounded preemption,
  no lost wakeups, and no scheduler plugin/callback path.
- Hardware thread interleaving: one selected TID issues per tile per cycle in
  v1, blocked/pending TIDs leave the issue-eligible set, Class B/C pending work
  does not freeze unrelated eligible TIDs, and local engines pipeline requests
  across TIDs within published arbitration bounds.
- Scheduler contract tests: exactly-one scheduler state/location per live TID,
  domain-ancestor eligibility, affinity mask intersection, fixed weight-table
  monotonicity, latency-class placement caps, wakeup generation matching,
  active-window dispatch, DDR spill/refill preserving identity and accounting,
  quota-period replenishment, forced park, and stale scheduler-record rejection.
- Scheduler discovery tests: `ENV_GET` reports scheduler profile version,
  weight table version/count, latency class count, fairness window, maximum
  wakeup insertion latency, maximum preemption latency, active-window size,
  local queue count, spill threshold, refill batch size, migration interval, and
  reservation features.
- Futex wait/wake, timer waits, event queues, fd readiness, child waits,
  gate-delivery interruption, and gate waits.

Device and I/O:

- DMA copy/fill/scatter-gather, completion events, permission faults,
  cancellation, revocation, and cache-coherence behavior.
- PCIe BAR FDR minting, page-aligned `MMAP`, device memory types, DMA buffer
  export, IOMMU scope, MSI/MSI-X as `irq_event`, and revoke-after-quiesce.
- Namespace dispatch, bounded path slices, service reply continuation,
  returned-capability verification, and revoked service rejection.
- Storage barriers, flush ordering, backend failure, and replay-visible commit
  records.

Networking and classifiers:

- Packet, datagram, stream, listener, endpoint readiness, descriptor passing,
  and socket option fail-closed behavior.
- Record classification and queue steering: exact/masked/prefix/range matches,
  hash steering, counters, overflow events, destination authority, and malformed
  rule rejection.
- Network servicelet integration: attach/detach servicelets to `net_interface`,
  `packet_queue`, endpoint, and listener objects; verify RX steering, TX
  admission, accept filtering, priority/mark propagation, telemetry redaction,
  `needs_software` fallback, destination authority, and stale/revoked attachment
  rejection.
- Bounded servicelets: verifier accepts only the LNP64 subset, rejects blocking,
  allocation, arbitrary memory access, hidden helper calls, capability minting,
  and unbounded loops; accepted programs meet instruction/cycle limits, match
  the verifier envelope fields, execute on servicelet lanes or interpreter
  models equivalently, and can emit only authorized action records.
- Servicelet boundary tests: accepted programs can classify, steer, mark,
  count, redact, select authorized gates, or return `needs_software`; rejected
  programs attempt path walking, TCP-like mutable state, executable loading,
  PCIe enumeration, blocking waits, allocator slow paths, helper callbacks, or
  arbitrary service implementation.

Realtime fabric:

- Bounded arbitration for metadata engines, event routers, queue banks, DMA
  paths, memory-controller ports, and servicelet lanes.
- Domain reservation/admission tests where best-effort pressure cannot violate
  admitted WCET/fabric bounds; overflow produces visible pressure/status
  events instead of hidden stalls.

Assurance and operability:

- Critical metadata ECC/parity correction, poisoning, and fault delivery.
- Watchdog timeout and local engine reset before and after commit points.
- Global progress under bounded faults: every accepted long command completes,
  cancels, faults, times out, revokes, degrades, or escalates to measured
  machine-fatal state; parked TIDs always have a valid wake/cancel/fault source;
  full queues/fabrics/FIFOs produce documented backpressure outcomes.
- Adversarial input containment: malformed packets/records/service replies,
  hostile peers, adversarial servicelet programs, corrupted file bytes, packet
  floods, and service-call storms remain data, pressure, typed faults, drops,
  `needs_software`, quota exhaustion, or domain isolation, never authority or
  unspecified hardware state.
- Telemetry FDRs, aggregate/redacted views, trace overflow, destructive and
  snapshot reads, and denial without authority.
- Measured boot and attestation: bitstream/ROM identity, manifest/image/domain
  measurements, quote shape, development flag, capability-root binding.
- Assurance profiles, audit streams, controlled debug/forensics, MLS labels,
  mission profiles, owner-key boot policy, reproducible artifact fields, and no
  hidden management/telemetry/debug/DMA path.

## RTL Simulation Milestones

S0 whole-machine skeleton gate:

- `bash scripts/run_rtl_s0.sh`
- file list: `tests/rtl/s0_filelist.f`
- expected committed trace shape: `tests/traces/rtl_s0_expected.trace`
- RTL entry point: `rtl/top/lnp64_top.sv`
- shared records/constants: `rtl/include/lnp64_pkg.sv`
- abstract proof artifact: `formal/S0Model.lean`
- mirrored assertion module: `formal/rtl_assertions/lnp64_s0_assertions.sv`

The S0 gate builds and runs a Verilator simulation that resets the skeleton,
creates root domain/PID 1/root FDR state, executes `NOP`, `LI32`, `ADD`, `JMP`,
SRAM `LD/ST`, `YIELD`, `ENV_GET`, `GET_ERRNO`, `SET_ERRNO`, a fail-closed
resource stub, and an unsupported opcode path. It also checks UART boot output,
synthetic event wake, structured fault injection, watchdog degraded/fault
state, live coherence/DMA visibility stub paths, and absence of software-visible
raw interrupt, raw physical address, raw DMA, or ambient device authority.

M1 proven ping-pong gate:

- `bash scripts/run_rtl_m1.sh`
- executable model: `formal/m1_model.py`
- RTL slice: `rtl/engines/lnp64_m1_pingpong.sv`
- file list: `tests/rtl/m1_filelist.f`
- mirrored assertion module: `formal/rtl_assertions/lnp64_m1_assertions.sv`

The M1 gate runs the executable model and RTL simulation, extracts normalized
`TRACE` lines from the RTL log, and diffs them against the model trace. The
bounded slice covers two hardware thread contexts, a narrowed `CAP_DUP`, FDR
generation checks, one queue object, `AWAIT`, producer `PUSH`, consumer `PULL`,
event wake, explicit full-queue `EAGAIN`, stale-generation `EREVOKED`, and
assertions for no forged FDR, no lost wakeup, exactly-one scheduler location,
stale generation rejection, and explicit queue-full behavior.

M2 gate/continuation gate:

- `bash scripts/run_rtl_m2.sh`
- executable model: `formal/m2_gate_model.py`
- RTL slice: `rtl/engines/lnp64_m2_gate.sv`
- file list: `tests/rtl/m2_filelist.f`
- mirrored assertion module: `formal/rtl_assertions/lnp64_m2_assertions.sv`

The M2 gate runs the executable model and RTL simulation, extracts normalized
`TRACE` lines from the RTL log, and diffs them against the model trace. The
bounded slice covers same-domain `GATE_CALL`/`GATE_RETURN`, sync continuation
creation and return, async delivery without caller parking, handoff delivery,
stale continuation rejection with `EREVOKED`, fault delivery through a gate
with `EFAULT`, and assertions for continuation uniqueness, delivery modes,
stale-continuation rejection, and fault-gate entry.

M3 process/thread lifecycle gate:

- `bash scripts/run_rtl_m3.sh`
- executable model: `formal/m3_process_model.py`
- RTL slice: `rtl/engines/lnp64_m3_process.sv`
- file list: `tests/rtl/m3_filelist.f`
- mirrored assertion module: `formal/rtl_assertions/lnp64_m3_assertions.sv`

The M3 gate runs the executable model and RTL simulation, extracts normalized
`TRACE` lines from the RTL log, and diffs them against the model trace. The
bounded slice covers minimal `CLONE`, child `EXIT`, parent `JOIN`, an
exec-barrier epoch advance with sibling stop, stale join rejection with
`EREVOKED`, explicit exec cancellation with `ECANCELED`, and assertions for
thread-location consistency, child-exit waitable signaling, join completion,
exec-barrier behavior, stale-join rejection, and terminal cancellation.

M4 VMA/MMU gate:

- `bash scripts/run_rtl_m4.sh`
- executable model: `formal/m4_vma_model.py`
- RTL slice: `rtl/engines/lnp64_m4_vma.sv`
- file list: `tests/rtl/m4_filelist.f`
- mirrored assertion module: `formal/rtl_assertions/lnp64_m4_assertions.sv`

The M4 gate runs the executable model and RTL simulation, extracts normalized
`TRACE` lines from the RTL log, and diffs them against the model trace. The
bounded slice covers VMA creation, read-permitted load, W^X store denial,
NX execute fault, guard-page fault, stale VMA generation rejection with
`EREVOKED`, TLB invalidation event observation, and assertions for the same
memory-protection and invalidation properties.

M5 DMA/memory-object gate:

- `bash scripts/run_rtl_m5.sh`
- executable model: `formal/m5_dma_model.py`
- RTL slice: `rtl/engines/lnp64_m5_dma.sv`
- file list: `tests/rtl/m5_filelist.f`
- mirrored assertion module: `formal/rtl_assertions/lnp64_m5_assertions.sv`

The M5 gate runs the executable model and RTL simulation, extracts normalized
`TRACE` lines from the RTL log, and diffs them against the model trace. The
bounded slice covers DMA copy/fill completions, write-permission faulting,
revoked-buffer submit rejection with `EREVOKED`, cross-domain isolation with
`EPERM`, coherence visibility after flush, and exact completion counting.

M6 typed-control/namespace/service-boundary gate:

- `bash scripts/run_rtl_m6.sh`
- executable model: `formal/m6_service_model.py`
- RTL slice: `rtl/engines/lnp64_m6_service.sv`
- file list: `tests/rtl/m6_filelist.f`
- mirrored assertion module: `formal/rtl_assertions/lnp64_m6_assertions.sv`

The M6 gate runs the executable model and RTL simulation, extracts normalized
`TRACE` lines from the RTL log, and diffs them against the model trace. The
bounded slice covers typed `OPEN_AT` envelope validation, namespace selector
dispatch to a service boundary, service request continuation creation,
returned-capability proposal narrowing and install, terminal cancellation with
`ECANCELED`, stale service-generation rejection with `EREVOKED`, and service
crash completion with `EIO`.

M7 futex/atomic gate:

- `bash scripts/run_rtl_m7.sh`
- executable model: `formal/m7_futex_atomic_model.py`
- RTL slice: `rtl/engines/lnp64_m7_futex_atomic.sv`
- file list: `tests/rtl/m7_filelist.f`
- mirrored assertion module: `formal/rtl_assertions/lnp64_m7_assertions.sv`

The M7 gate runs the executable model and RTL simulation, extracts normalized
`TRACE` lines from the RTL log, and diffs them against the model trace. The
bounded slice covers `LOCK_CMPXCHG` success and explicit mismatch failure,
futex wait expected-value parking, futex wake delivery without lost wakeup,
bucket spill/refill identity preservation, stale futex-address rejection with
`EREVOKED`, and exact atomic operation counting.

M8 heap gate:

- `bash scripts/run_rtl_m8.sh`
- executable model: `formal/m8_heap_model.py`
- RTL slice: `rtl/engines/lnp64_m8_heap.sv`
- file list: `tests/rtl/m8_filelist.f`
- mirrored assertion module: `formal/rtl_assertions/lnp64_m8_assertions.sv`

The M8 gate runs the executable model and RTL simulation, extracts normalized
`TRACE` lines from the RTL log, and diffs them against the model trace. The
bounded slice covers fixed-size allocation, `ALLOC_SIZE`, free/quarantine,
same-class reuse, double-free rejection with `EINVAL`, stale-pointer rejection
with `EREVOKED`, cross-thread free handoff, guard faulting with `EFAULT`, and
exact allocation/free counting.

M9 classifier/servicelet gate:

- `bash scripts/run_rtl_m9.sh`
- executable model: `formal/m9_classifier_servicelet_model.py`
- RTL slice: `rtl/engines/lnp64_m9_classifier_servicelet.sv`
- file list: `tests/rtl/m9_filelist.f`
- mirrored assertion module: `formal/rtl_assertions/lnp64_m9_assertions.sv`

The M9 gate runs the executable model and RTL simulation, extracts normalized
`TRACE` lines from the RTL log, and diffs them against the model trace. The
bounded slice covers verifier acceptance of a bounded servicelet, verifier
rejection of a blocking program with `EINVAL`, packet queue steering, IPC/generic
record steering, authorized `needs_software` action emission without authority
creation, per-domain cycle-budget enforcement with `EAGAIN`, stale attachment
rejection with `EREVOKED`, and exact packet/IPC/reject counting.

M10 RAS/observability/assurance gate:

- `bash scripts/run_rtl_m10.sh`
- executable model: `formal/m10_ras_model.py`
- RTL slice: `rtl/engines/lnp64_m10_ras.sv`
- file list: `tests/rtl/m10_filelist.f`
- mirrored assertion module: `formal/rtl_assertions/lnp64_m10_assertions.sv`

The M10 gate runs the executable model and RTL simulation, extracts normalized
`TRACE` lines from the RTL log, and diffs them against the model trace. The
bounded slice covers measured boot observability, metadata ECC correction,
parity poison faulting with `EIO`, watchdog timeout and local degraded reset,
scoped/redacted telemetry FDR reads, visible trace-ring overflow, a
measurement-bound development quote stub, audit emission, and debug/MLS denial
with `EPERM`.

M11 DDR/metadata broker gate:

- `bash scripts/run_rtl_m11.sh`
- executable model: `formal/m11_ddr_metadata_model.py`
- RTL slice: `rtl/engines/lnp64_m11_ddr_metadata.sv`
- file list: `tests/rtl/m11_filelist.f`
- mirrored assertion module: `formal/rtl_assertions/lnp64_m11_assertions.sv`

The M11 gate runs the executable model and RTL simulation, extracts normalized
`TRACE` lines from the RTL log, and diffs them against the model trace. The
bounded slice covers a DDR line metadata allocation bound to a Resource Domain
and generation, write/read visibility, stale-generation rejection with
`EREVOKED`, cross-domain metadata rejection with `EPERM`, ECC scrub terminal
reporting with `EIO`, and metadata barrier quiescence.

M12 SD/SPI storage-barrier gate:

- `bash scripts/run_rtl_m12.sh`
- executable model: `formal/m12_storage_barrier_model.py`
- RTL slice: `rtl/engines/lnp64_m12_storage_barrier.sv`
- file list: `tests/rtl/m12_filelist.f`
- mirrored assertion module: `formal/rtl_assertions/lnp64_m12_assertions.sv`

The M12 gate runs the executable model and RTL simulation, extracts normalized
`TRACE` lines from the RTL log, and diffs them against the model trace. The
bounded slice covers boot-image read visibility, block-object write completion
under object authority, storage-barrier quiescence, stale-object rejection with
`EREVOKED`, cross-domain rejection with `EPERM`, media fault terminal reporting
with `EIO`, and absence of raw device authority in software-visible state.

M13 PCIe/IOMMU/MSI gate:

- `bash scripts/run_rtl_m13.sh`
- executable model: `formal/m13_pcie_iommu_model.py`
- RTL slice: `rtl/engines/lnp64_m13_pcie_iommu.sv`
- file list: `tests/rtl/m13_filelist.f`
- mirrored assertion module: `formal/rtl_assertions/lnp64_m13_assertions.sv`

The M13 gate runs the executable model and RTL simulation, extracts normalized
`TRACE` lines from the RTL log, and diffs them against the model trace. The
bounded slice covers PCIe root enumeration, BAR capability metadata, an
IOMMU-scoped DMA completion, MSI delivery as an event, fail-closed unbound
bus-master rejection with `EPERM`, stale BAR generation rejection with
`EREVOKED`, malformed config-space rejection with `EINVAL`, and absence of raw
DMA or raw interrupt authority in software-visible state.

M14 Resource Domain / policy gate:

- `bash scripts/run_rtl_m14.sh`
- executable model: `formal/m14_resource_domain_policy_model.py`
- RTL slice: `rtl/engines/lnp64_m14_resource_domain_policy.sv`
- file list: `tests/rtl/m14_filelist.f`
- mirrored assertion module: `formal/rtl_assertions/lnp64_m14_assertions.sv`

The M14 gate runs the executable model and RTL simulation, extracts normalized
`TRACE` lines from the RTL log, and diffs them against the model trace. The
bounded slice directly covers Resource Domain delegation and policy enforcement:
child rights are clipped to parent-delegated rights, child budgets stay within
the parent budget, excess budget requests fail closed with `EPERM`, frozen
domains reject dispatch until resumed, destroyed domains reject stale dispatch
with `EREVOKED`, child/sibling usage rolls up to the parent domain, and policy
denial is explicit and fail-closed.

M15 object-profile gate:

- `bash scripts/run_rtl_m15.sh`
- executable model: `formal/m15_object_profiles_model.py`
- RTL slice: `rtl/engines/lnp64_m15_object_profiles.sv`
- file list: `tests/rtl/m15_filelist.f`
- mirrored assertion module: `formal/rtl_assertions/lnp64_m15_assertions.sv`

The M15 gate runs the executable model and RTL simulation, extracts normalized
`TRACE` lines from the RTL log, and diffs them against the model trace. The
bounded slice directly covers object profiles: a counter reaches its threshold
and emits an event, queue push requires object rights, full queue behavior is an
explicit pressure/error event, event delivery requires matching source
generation and rejects a stale source with `EREVOKED`, and the call-gate profile
rejects duplicate continuation use.

Bounded randomized co-simulation:

- `bash scripts/run_rtl_random_cosim.sh`
- manifest: `tests/traces/rtl_cosim_manifest.json`
- manifest checker: `scripts/check_rtl_cosim_manifest.py`

The randomized co-simulation gate currently validates the co-sim manifest, then
runs the M1 ping-pong, M2
gate/continuation, M3 process/thread lifecycle, M4 VMA/MMU, M5 DMA/memory
object, M6 typed-control/service-boundary, M7 futex/atomic, M8 heap, M9
classifier/servicelet, M10 RAS/observability, M11 DDR/metadata, M12 SD/SPI
storage-barrier, M13 PCIe/IOMMU/MSI, M14 Resource Domain/policy, and M15
object-profile models and RTL with the same bounded
seed vector. The M1 seed varies queue generation,
push payload, and refill payload while preserving the same
generation, full-queue, wakeup, and stale-rejection invariants. The M2 seed
varies gate generation, continuation id, sync target, async target, and
handoff target while preserving continuation uniqueness, stale-return
rejection, and fault-gate delivery. The M3 seed varies parent/child ids, child
exit code, exec epoch, and stopped-sibling count while preserving
clone/exit/join, stale join rejection, and terminal exec cancellation. The M4
seed varies VMA id, page count, base address, and VMA generation while
preserving W^X, NX, guard-page, stale generation, and TLB invalidation
behavior. The M5 seed varies root domain, source/destination buffers,
copy/fill sizes, fill value, and isolation-domain checks while preserving DMA
completion, revoked-submit rejection, permission faults, and coherence
visibility. The M6 seed varies root/namespace ids, path length, service and
operation ids, continuation id, returned rights, and returned object id while
preserving typed envelope validation, namespace dispatch, narrowed returned
capability install, terminal cancellation, stale-service rejection, and crash
completion. The M7 seed varies root domain, initial atomic value,
compare-exchange desired and failure values, futex address, and bucket id while
preserving explicit compare-exchange success/failure, futex wait/wake,
bucket-spill identity preservation, stale-address rejection, no-lost-wakeup,
and exact atomic counting. The M8 seed varies root domain, heap generation,
pointer, size class, owner/freeing thread ids, and pointer generation while
preserving allocation, allocation-size reporting, free/quarantine, same-class
reuse, double-free rejection, stale-pointer rejection, cross-thread handoff,
guard faulting, and exact allocation/free counts. The M9 seed varies root/table
ids, verifier program and instruction count, packet/IPC steering fields, and
budget cycle count while preserving verifier acceptance/rejection, packet and
IPC steering, authorized action emission, budget exhaustion, stale attachment
rejection, no-authority creation, and exact packet/IPC/reject counts. The M10
seed varies measurement and telemetry ids, ECC correction count, watchdog reset
id, visible telemetry counters, trace-ring capacity/writes, quote id, and audit
label while preserving measured boot, telemetry FDR presence, ECC correction,
parity poison faulting, watchdog local degraded reset, scoped/redacted
telemetry, visible trace overflow, measurement-bound development quote,
audit/debug/MLS denial, and exact fault/telemetry/audit counts. The M11 seed
varies root domain, DDR line id/generation, metadata epoch, byte length, data
value, cross-domain id, and ECC correction count while preserving metadata
domain binding, read-after-write visibility, stale-generation rejection,
cross-domain rejection, ECC scrub terminal reporting, barrier quiescence, and
exact completion/fault counts. The M12 seed varies root domain, object
id/generation, barrier id, block index, byte length, data value, cross-domain
id, and media status while preserving boot-image visibility, block-object write
authorization, barrier quiescence, stale-object rejection, cross-domain
rejection, media fault terminal reporting, no raw device authority, and exact
completion/fault counts. The M13 seed varies root domain, requester id, BAR
id/generation, IOMMU context, DMA byte count, MSI vector, rogue domain, and
malformed field id while preserving enumeration, BAR capability creation,
IOMMU-scoped DMA, MSI event delivery, bus-master/stale/malformed fail-closed
paths, no raw PCIe authority, and exact completion/fault counts. The M14 seed
varies root/child domain ids, parent and child budgets, requested rights,
child/sibling usage, policy mask, and policy label while preserving delegated
authority clipping, budget containment, lifecycle dispatch rejection,
hierarchical usage roll-up, and fail-closed policy denial. The M15 seed varies
object id/generation, counter threshold, queue payload, event generation, and
continuation id while preserving queue rights, explicit overflow, event-source
generation safety, and continuation uniqueness. Additional blocks should grow
the same seedable model/RTL path as their input surfaces become parameterized.

1. Fetch/decode/ALU/load/store from a DDR model.
2. Weighted-fair multi-context scheduler with `CLONE`, `YIELD`, `AWAIT`,
   `EXIT`, quotas, and bounded wakeup insertion.
3. FDR table plus UART `PULL`/`PUSH`.
4. Default operating envelope and boot manifest grants.
5. Namespace Dispatch Engine and returned-capability installation.
6. `MMAP`, page-state transitions, anonymous COW, and object-backed page fill.
7. Hardware Heap Engine.
8. `CLONE`, child-exit `AWAIT`, and `EXEC` from an exec-plan descriptor.
9. Signals, hardware faults, `ENV_GET`, futexes, and timers.
10. Generic runtime objects with `OBJECT_CTL`, `PULL`/`PUSH`, `AWAIT`, and
    `CAP_*`.
11. DMA copy/fill/scatter-gather with event completion.
12. Resource Domains, nested limits, freeze/resume, usage accounting, and
    capability delegation.
13. `GATE_CALL`/`GATE_RETURN` same-domain and cross-domain gates.
14. Supervisor-domain control FDR and upcall delivery.
15. Minimal paravirtual Unix personality over native tasks and block-image FDRs.
16. Linux syscall compatibility smoke test for static userland.
17. NetBSD rump-style filesystem service over a block-image FDR.
18. PCIe Root Complex, Bus Master enumeration, BAR FDRs, DMA buffers, IOMMU,
    MSI/MSI-X event delivery, and a simple NIC or NVMe driver domain.
19. RAS, telemetry, measured boot, quote FDR, audit stream, and watchdog reset.
20. Checkpoint hooks, record classification/queue steering, bounded
    servicelets, WCET discovery, fabric arbitration, tenant-strict,
    confidential, MLS, mission, and open-assurance smoke tests.

## Gates

- `cargo test`
- `bash scripts/run_all_gates.sh`
- `bash scripts/run_rtl_proof_gates.sh`
- `bash scripts/run_rtl_proof_docker.sh`
- `bash scripts/run_rtl_random_cosim.sh`
- `bash scripts/run_rtl_synth_smoke.sh`
- `bash scripts/run_rtl_synth_gates.sh`
- `bash scripts/run_rtl_fpga_ice40_s0.sh`
- `bash scripts/run_rtl_synth_docker.sh`
- `bash scripts/run_software_gates.sh`
- `bash scripts/run_demos.sh`
- `bash scripts/run_userland.sh`
- `bash scripts/run_real_packages.sh`
- focused gates listed in `README.md`

The Docker RTL/proof gate is the canonical proof environment. It installs Lean
and Verilator in `Dockerfile.rtl-proof`, rejects `axiom`, `sorry`, and `admit`
in the checked Lean files, validates `formal/proof_obligations_manifest.json`
against the A1 through A10 theorem artifacts, runs the Lean models, then runs
the S0 through M15 RTL/model trace gates plus the bounded randomized
co-simulation smoke.

The Docker RTL synthesis smoke gate installs Verilator, Yosys, nextpnr-ice40,
and Icestorm in
`Dockerfile.rtl-synth`, validates `fpga/constraints/lnp64_s0_smoke.json` and
`fpga/constraints/lnp64_s0_stub.xdc`, validates the Track D bring-up manifest
at `fpga/bringup/lnp64_track_d_bringup.json`, checks the S0 shell/record
contract, runs a Yosys S0 synthesis/netlist smoke through
`scripts/run_rtl_yosys_s0.sh`, then statically elaborates the S0 through M15
RTL tops and runs `scripts/run_rtl_fpga_ice40_s0.sh` to produce a generic iCE40
HX8K place/route/icepack bitstream for the S0 FPGA wrapper using the
package-level CT256 PCF at `fpga/constraints/lnp64_s0_ice40_hx8k_ct256.pcf`.
The FPGA gate checks the nextpnr JSON report with
`scripts/check_ice40_report.py`, requiring the smoke target frequency and
positive in-budget LC/IO utilization, and checks an independent Icestorm
`icetime` timing report with `scripts/check_icetime_report.py`. This gate is
the current FPGA bring-up constraint/coverage/static-RTL/netlist/bitstream and
timing-report smoke; it still does not claim a board-schematic pinout or
physical board bring-up.
