# LNP64 Verification Plan

Verification starts at the architectural model, then refines through emulator
tests, RTL assertions, simulation, synthesis, and later FPGA evidence. This file
is the map. Detailed evidence lives in the manifests, scripts, and focused
roadmaps referenced below.

## Whole-Chip Validation Charter

Formal proofs and validation answer different questions. Proofs establish the
safety envelope: authority, confinement, scheduler correctness, DMA scope,
reset safety, realtime honesty, and bounded-progress contracts should be
impossible to violate under stated assumptions. Validation establishes feature
reality: the chip actually boots, runs programs, schedules work, serves I/O,
handles faults, exposes useful counters, meets advertised latency classes, and
keeps working under realistic stress.

The validation program should run in parallel with the proof program:

```text
formal proof        -> impossible states are unreachable
directed tests      -> intended features work
random testing      -> weird interactions are explored
differential tests  -> RTL matches the executable model
semantic coverage   -> exercised and unexercised behavior is visible
fault injection     -> recovery paths actually work
simulation/FPGA     -> long-running system behavior works
post-silicon tests  -> manufactured hardware matches the advertised contract
```

The project should treat the Rust emulator plus future model code as the seed
of a whole-machine executable model, not merely an ISA interpreter. The model
must cover instruction execution, capability objects, Resource Domains, VMAs,
scheduler/wait state, allocator behavior, service gates, DMA/IOMMU, queues,
fault/recovery, trace records, and realtime-class metadata. For each RTL test,
the long-term target is the same initial machine state, same instruction/device/
service inputs, and comparison of architectural state, memory effects, faults,
returned capabilities, scheduler transitions, and typed trace events.

Typed architectural traces are the primary validation interface. Internal RTL
signals may change; events such as `INSTR_RETIRE`, `CAP_DERIVE`,
`DOMAIN_TRANSITION`, `THREAD_PARK`, `THREAD_WAKE`, `WAIT_LINK`, `QUEUE_PUSH`,
`GATE_CALL`, `ALLOC_PUBLISH`, `DMA_COMPLETE`, `FAULT_RAISE`, and
`RESET_EPOCH_CHANGE` should remain stable enough for model/RTL comparison and
coverage. A trace checker should answer whether RTL performed the same legal
architectural transition as the model, emitted the promised evidence, used the
correct budget/realtime class, and failed closed where expected.

Validation should be tiered:

- unit tests for decoders, queues, tables, FSMs, and arbiters.
- engine-directed tests for capability, domain, scheduler/wait, VMA/MMU, DMA,
  gate/service, allocator, classifier/servicelet, RAS, and fabric blocks.
- differential instruction and object tests against the executable model.
- constrained-random whole-chip tests over domains, threads, caps, VMAs,
  queues, services, DMA, faults, revocations, resets, and device events.
- metamorphic tests for authority attenuation, stricter domains, migration,
  equivalent gates, debug-cap removal, and timing changes.
- crash/fault injection at model, RTL simulation, and FPGA/emulation levels.
- realtime validation for every advertised latency class under its named
  assumptions, including negative tests where hardware must refuse or downgrade
  unsupported claims.
- software workload tests for libc/personality, service, networking, storage,
  multi-tenant, and long-running scheduler behavior.

For every advertised feature, maintain a readiness scorecard: spec, executable
model, RTL, directed tests, random tests, differential tests, semantic coverage,
fault injection, realtime validation where applicable, multicore stress,
simulation/FPGA long-run evidence, software workload evidence, trace
observability, and proof connection. A feature is not done because one RTL path
exists.

## Model-Level Targets

- Instruction encoding/decoding golden model.
- Abstract machine model for capabilities, Resource Domains, VMAs, waitables,
  DMA, scheduler state, service boundaries, and commit/abort transitions.
- Lean theorem targets from `formal_theorems.md`.
- RTL/proof coupling targets from `formal_rtl_codesign_roadmap.md`.
- Compatibility targets from `conformance_matrix.md`.

## Directed Emulator And RTL Tests

Core ISA:

- ALU, branch, call/return, load/store, atomics, `FENCE`, and `ISYNC`.
- `ENV_GET` scalar keys, buffer keys, bad keys, buffer faults, topology,
  feature bits, latency classes, and WCET discovery.
- Instruction-class miss behavior: hot success, bounded canonical error,
  explicit park, and `EINPROGRESS` plus completion-token paths.
- Native fault-gate delivery and POSIX signal-profile mapping.

Resource primitives:

- `PULL`, `PUSH`, `AWAIT`, `CAP_*`, `OBJECT_CTL`, `DOMAIN_CTL`, `GATE_CALL`,
  `GATE_RETURN`, `GATE_DELIVER`, `DMA_CTL`, `ALLOC`, `FREE`, `MMAP`, `MUNMAP`,
  and `MPROTECT`.
- `OBJECT_CTL` profiles for counter, queue, memory object, pipe, semaphore,
  completion, channel, task event, shared arena, and DMA completion.
- Returned-capability proposals as data until Capability Engine commit.

Capability and domain safety:

- FDR generation checks, lineage epochs, narrowing, sealing, transfer, revoke,
  stale-generation rejection, and no authority amplification.
- Nested Resource Domain create/destroy/freeze/resume/query.
- Monotonic limits, hierarchical accounting, delegated authority bounds, and
  frozen-domain dispatch rejection.

Memory and heap:

- VMA state transitions, anonymous zero/COW, page-fill transactions,
  object-backed retry/error replies, dirty-range enumeration, executable
  provenance, W^X/NX, guard pages, ASLR, TLB/I-cache invalidation, and
  pending-fill cancellation.
- Heap allocation: invalid free, stale pointer, cross-thread free, COW/exec
  teardown, `ALLOC_SIZE`, software-owned arena boundaries, guard/quarantine
  behavior, and default size-class hit paths.

Scheduler and waits:

- Weighted-fair charging, virtual runtime/deadline ordering, quotas,
  hierarchy rollup, wakeup insertion, frozen-domain removal, bounded preemption,
  no lost wakeups, and no scheduler plugin/callback path.
- Hardware thread interleaving: one selected TID issues per tile per cycle in
  v1; blocked/pending TIDs leave the issue-eligible set; long pending work must
  not freeze unrelated eligible TIDs.
- Futex wait/wake, timer waits, event queues, fd readiness, child waits,
  gate-delivery interruption, and gate waits.

Device, storage, and I/O:

- DMA copy/fill/scatter-gather, completion events, permission faults,
  cancellation, revocation, and cache-coherence behavior.
- PCIe BAR FDR minting, page-aligned `MMAP`, device memory types, DMA buffer
  export, IOMMU scope, MSI/MSI-X as event delivery, and revoke-after-quiesce.
- Namespace dispatch, bounded path slices, service reply continuation,
  returned-capability verification, and revoked service rejection.
- Storage barriers, flush ordering, backend failure, and replay-visible commit
  records.

Networking and classifiers:

- Packet, datagram, stream, listener, endpoint readiness, descriptor passing,
  and socket option fail-closed behavior.
- Record classification and queue steering: exact, masked, prefix, range, hash,
  counters, overflow events, destination authority, and malformed-rule
  rejection.
- Network servicelet integration for RX steering, TX admission, accept
  filtering, priority/mark propagation, telemetry redaction, `needs_software`
  fallback, and stale/revoked attachment rejection.
- Servicelet verifier rejection for blocking, allocation, arbitrary memory
  access, hidden helper calls, capability minting, unbounded loops, path
  walking, executable loading, PCIe enumeration, and allocator slow paths.

Realtime, assurance, and operability:

- Bounded arbitration for metadata engines, event routers, queue banks, DMA
  paths, memory-controller ports, and servicelet lanes.
- Domain reservation/admission where best-effort pressure cannot violate
  admitted WCET/fabric bounds.
- Critical metadata ECC/parity correction, poisoning, and fault delivery.
- Watchdog timeout and local engine reset before and after commit points.
- Global progress under bounded faults: every accepted long command completes,
  cancels, faults, times out, revokes, degrades, or escalates to measured
  machine-fatal state.
- Adversarial input containment for malformed packets, records, service replies,
  hostile peers, adversarial servicelets, corrupted bytes, floods, and service
  storms.
- Telemetry FDRs, redacted views, trace overflow, destructive/snapshot reads,
  measured boot, attestation, audit streams, controlled debug/forensics, MLS
  labels, mission profiles, owner-key boot policy, and no hidden
  management/telemetry/debug/DMA path.

## RTL Simulation Milestones

Detailed per-slice evidence is intentionally not duplicated here. Use:

- `formal_rtl_roadmap_completion_checklist.md`
- `rtl/track_b_blocks_manifest.json`
- `formal/proof_obligations_manifest.json`
- `formal/theorem_rtl_coupling_manifest.json`
- `tests/traces/rtl_cosim_manifest.json`

| Slice | Focus | Primary Gate | Main Evidence |
| --- | --- | --- | --- |
| S0 | Whole-machine skeleton | `bash scripts/run_rtl_s0.sh` | `rtl/top/lnp64_top.sv`, `formal/S0Model.lean`, `formal/rtl_assertions/lnp64_s0_assertions.sv` |
| M1 | Capability/FDR queue ping-pong | `bash scripts/run_rtl_m1.sh` | `formal/M1Model.lean`, `formal/m1_model.py`, M1 RTL/assertions |
| M2 | Gates, continuations, faults | `bash scripts/run_rtl_m2.sh` | `formal/M2GateModel.lean`, M2 RTL/assertions |
| M3 | Process/thread lifecycle | `bash scripts/run_rtl_m3.sh` | `formal/M3ProcessModel.lean`, M3 RTL/assertions |
| M4 | VMA/MMU permissions | `bash scripts/run_rtl_m4.sh` | `formal/M4VmaModel.lean`, M4 RTL/assertions |
| M5 | DMA and memory objects | `bash scripts/run_rtl_m5.sh` | `formal/M5DmaModel.lean`, M5 RTL/assertions |
| M6 | Typed control and service boundary | `bash scripts/run_rtl_m6.sh` | `formal/M6ServiceModel.lean`, M6 RTL/assertions |
| M7 | Futexes and atomics | `bash scripts/run_rtl_m7.sh` | `formal/M7FutexAtomicModel.lean`, M7 RTL/assertions |
| M8 | Heap engine | `bash scripts/run_rtl_m8.sh` | `formal/M8HeapModel.lean`, M8 RTL/assertions |
| M9 | Classifier and servicelet | `bash scripts/run_rtl_m9.sh` | `formal/M9ClassifierServiceletModel.lean`, M9 RTL/assertions |
| M10 | RAS, observability, assurance | `bash scripts/run_rtl_m10.sh` | `formal/M10RasModel.lean`, M10 RTL/assertions |
| M11 | DDR metadata broker | `bash scripts/run_rtl_m11.sh` | `formal/M11DdrMetadataModel.lean`, M11 RTL/assertions |
| M12 | SD/SPI storage barrier | `bash scripts/run_rtl_m12.sh` | `formal/M12StorageBarrierModel.lean`, M12 RTL/assertions |
| M13 | PCIe/IOMMU/MSI | `bash scripts/run_rtl_m13.sh` | `formal/M13PcieIommuModel.lean`, M13 RTL/assertions |
| M14 | Resource Domain policy | `bash scripts/run_rtl_m14.sh` | `formal/M14ResourceDomainPolicyModel.lean`, M14 RTL/assertions |
| M15 | Object profiles | `bash scripts/run_rtl_m15.sh` | `formal/M15ObjectProfilesModel.lean`, M15 RTL/assertions |
| Random co-sim | Seeded M1-M15 model/RTL traces | `bash scripts/run_rtl_random_cosim.sh` | `tests/traces/rtl_cosim_manifest.json` |

Each M-slice follows the same evidence pattern: Lean model, executable Python
model or extracted equivalent, RTL engine, RTL assertions, file list, focused
gate, and trace/coupling manifest entry. The current traces are bounded witness
evidence. The roadmap target is typed transition traces and checked
RTL-to-Lean refinement for security-critical blocks.

## Integration Order

1. Fetch/decode/ALU/load/store from a DDR model.
2. Weighted-fair multi-context scheduler with `CLONE`, `YIELD`, `AWAIT`,
   `EXIT`, quotas, and bounded wakeup insertion.
3. FDR table plus UART `PULL`/`PUSH`.
4. Default operating envelope and boot manifest grants.
5. Namespace Dispatch Engine and returned-capability installation.
6. `MMAP`, page-state transitions, anonymous COW, and object-backed page fill.
7. Hardware Heap Engine.
8. `CLONE`, child-exit `AWAIT`, and `EXEC` from an exec-plan descriptor.
9. Signals/fault gates, `ENV_GET`, futexes, and timers.
10. Generic runtime objects through `OBJECT_CTL`, `PULL`/`PUSH`, `AWAIT`, and
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
    servicelets, WCET discovery, fabric arbitration, tenant-strict
    confidentiality, MLS, mission, and open-assurance smoke tests.

## Gates

Software:

- `bash scripts/run_software_gates.sh`
- `cargo test`
- `bash scripts/run_demos.sh`
- `bash scripts/run_userland.sh`
- `bash scripts/run_real_packages.sh`

RTL, proof, and co-simulation:

- `bash scripts/run_rtl_proof_gates.sh`
- `bash scripts/run_rtl_proof_docker.sh`
- `bash scripts/run_rtl_random_cosim.sh`
- focused gates listed in `README.md`

Synthesis and FPGA smoke:

- `bash scripts/run_rtl_synth_smoke.sh`
- `bash scripts/run_rtl_synth_gates.sh`
- `bash scripts/run_rtl_synth_docker.sh`
- `bash scripts/run_rtl_fpga_ice40_s0.sh`

Full repo:

- `bash scripts/run_all_gates.sh`

The Docker RTL/proof gate is the canonical proof environment. It installs Lean
and Verilator, rejects `axiom`, `sorry`, and `admit` in checked Lean files,
validates proof/coupling manifests, and runs S0 through M15 plus bounded random
co-simulation.

The Docker RTL synthesis smoke gate is the reproducible static hardware path.
It validates synthesis constraints, FPGA bring-up metadata, S0 contracts,
Yosys/nextpnr/IceStorm smoke output, and timing-report parsers. It does not
claim live board bring-up.
