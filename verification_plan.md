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

Assurance and operability:

- Critical metadata ECC/parity correction, poisoning, and fault delivery.
- Watchdog timeout and local engine reset before and after commit points.
- Telemetry FDRs, aggregate/redacted views, trace overflow, destructive and
  snapshot reads, and denial without authority.
- Measured boot and attestation: bitstream/ROM identity, manifest/image/domain
  measurements, quote shape, development flag, capability-root binding.
- Assurance profiles, audit streams, controlled debug/forensics, MLS labels,
  mission profiles, owner-key boot policy, reproducible artifact fields, and no
  hidden management/telemetry/debug/DMA path.

## RTL Simulation Milestones

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
20. Checkpoint hooks, record classification/queue steering, tenant-strict,
    confidential, MLS, mission, and open-assurance smoke tests.

## Gates

- `cargo test`
- `bash scripts/run_all_gates.sh`
- `bash scripts/run_demos.sh`
- `bash scripts/run_userland.sh`
- `bash scripts/run_real_packages.sh`
- focused gates listed in `README.md`
