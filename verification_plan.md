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
- `bash scripts/run_demos.sh`
- `bash scripts/run_userland.sh`
- `bash scripts/run_real_packages.sh`
- focused gates listed in `README.md`
