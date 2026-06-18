# LNP64 Formal/RTL Co-Design Roadmap

This roadmap defines a parallel path toward real formal proofs and
synthesizable SystemVerilog/RTL that can run LNP64 code in simulation and,
eventually, on FPGA.

The goal is not to write RTL first and prove it later. The goal is:

```text
executable spec -> proof model -> reference emulator -> RTL block
-> RTL simulation -> FPGA bring-up
```

Each hardware block should have:

- a small abstract model.
- invariants and theorem targets.
- test vectors generated from the model or emulator.
- synthesizable RTL.
- RTL assertions.
- co-simulation against the emulator/model.
- synthesis constraints.
- FPGA smoke tests.

The sequencing rule is:

```text
interfaces first -> stub behavior second -> vertical slices third
-> performance and completeness later
```

Stubs are acceptable only when they preserve the real command/response shape,
carry authority/generation/domain metadata, fail closed, and keep reset,
fault, event, and completion paths live.

## S0 Starter Contract

This section is the concrete starting point for an engineer implementing the
first whole-machine skeleton. It is intentionally still high level, but it fixes
the first module boundaries, records, reset behavior, and acceptance tests well
enough to start RTL/proof scaffolding without another architecture pass.

### Repository Layout

Use a layout that keeps the executable spec, proof model, RTL, and tests close
but separate:

- `rtl/`: synthesizable SystemVerilog.
- `rtl/include/`: shared packed structs, constants, opcodes, error codes, and
  feature bits.
- `rtl/top/`: top-level machine and clock/reset glue.
- `rtl/core/`: core tile, fetch/decode/issue/retire, register files, thread
  context window.
- `rtl/engines/`: capability, scheduler, object, gate, process, VMA, DMA, heap,
  futex, domain, service, classifier, RAS, and device shells.
- `rtl/sim/`: Verilator testbench, ROM/SRAM images, synthetic event/fault
  injectors, and trace comparison utilities.
- `formal/`: Lean model plus lightweight generated theorem/test artifacts.
- `formal/rtl_assertions/`: SystemVerilog assertions mirrored from the model
  invariants where practical.
- `tests/rtl/`: S0 and M1 simulation tests.
- `tests/traces/`: emulator/model traces used for co-simulation.

### First Module Names

S0 should instantiate the following top-level module shells, even when most
return only stub completions:

- `lnp64_top`
- `lnp64_reset_boot`
- `lnp64_clock_reset`
- `lnp64_core_tile`
- `lnp64_decode`
- `lnp64_issue_retire`
- `lnp64_thread_context`
- `lnp64_engine_router`
- `lnp64_completion_router`
- `lnp64_errno_writeback`
- `lnp64_scheduler`
- `lnp64_event_router`
- `lnp64_cap_engine`
- `lnp64_domain_engine`
- `lnp64_policy_engine`
- `lnp64_object_engine`
- `lnp64_gate_engine`
- `lnp64_process_engine`
- `lnp64_vma_engine`
- `lnp64_page_allocator`
- `lnp64_memory_fabric`
- `lnp64_metadata_broker`
- `lnp64_dma_fabric`
- `lnp64_service_boundary`
- `lnp64_futex_atomic`
- `lnp64_heap_engine`
- `lnp64_classifier_servicelet`
- `lnp64_fault_telemetry`
- `lnp64_watchdog`
- `lnp64_measurement_attestation`
- `lnp64_entropy_env`
- `lnp64_uart`
- `lnp64_storage_stub`
- `lnp64_eth_stub`
- `lnp64_pcie_stub`

The names are not sacred, but the boundaries are. Renaming is fine only if the
same shells and channels remain obvious.

### First Interfaces

Use ready/valid channels for all command and response paths:

- `cmd_valid`, `cmd_ready`, `cmd`.
- `rsp_valid`, `rsp_ready`, `rsp`.
- `event_valid`, `event_ready`, `event`.
- `fault_valid`, `fault_ready`, `fault`.

Every command-like record should carry the same authority and completion
identity fields unless the record is explicitly local-only:

- `op_id`
- `opcode`
- `profile`
- `pid`
- `tid`
- `domain_id`
- `domain_gen`
- `credential_snapshot_id`
- `result_reg`
- `rights_mask`
- `flags`
- `arg0` through `arg3`
- `arg_block_ptr`
- `arg_block_len`
- `cancel_class`
- `completion_target`

Every response-like record should carry:

- `op_id`
- `pid`
- `tid`
- `domain_id`
- `domain_gen`
- `result_reg`
- `result_value`
- `errno_value`
- `status`
- `event_mask`

The first implementation can use conservative placeholder widths, but the
records should be centrally declared so widening them does not rewrite every
module. Reasonable S0 placeholders:

- ids and generations: 32 bits.
- op ids: 32 bits.
- opcodes/profiles/status/errors: 16 bits.
- rights/flags/event masks: 64 bits.
- addresses, scalar args, result values, and pointers: 64 bits.

### Minimal S0 Opcode Surface

S0 does not need the full ISA. It needs enough to prove decode, retirement,
stubs, events, and errors:

- `NOP`
- `LI32` or equivalent small immediate load.
- `ADD` or one simple ALU op.
- `JMP` or one simple branch.
- `LD` and `ST` to internal SRAM only.
- `YIELD`
- `ENV_GET`
- `GET_ERRNO`
- `SET_ERRNO`
- one stubbed resource opcode, preferably `OBJECT_CTL` or `OPEN_AT`.
- one synthetic fault-injection opcode or simulation-only hook.

All other decoded native operations must fail through the same reserved-op or
unsupported-profile path that the real machine will use.

### Reset And Boot Sequence

S0 reset should be deterministic:

1. assert reset to every module shell.
2. initialize feature bits, build id, and skeleton limit records.
3. create root Resource Domain id/generation.
4. create PID 1/TID 1 context inside that domain.
5. initialize scheduler state with PID 1/TID 1 runnable.
6. initialize an empty or explicitly granted FDR table.
7. initialize fault, event, completion, telemetry, and watchdog counters.
8. load a tiny ROM/SRAM instruction stream for PID 1.
9. release the core tile.
10. if any mandatory step fails, emit a measured/audited boot fault and do not
    create an unaffiliated runnable thread.

Initial S0 feature bits should make unsupported blocks explicit. For example,
S0 may report `core_tile`, `decode`, `env_get`, `scheduler_stub`,
`event_router_stub`, `capability_stub`, `domain_stub`, `ras_stub`, and
`uart_stub`, while reporting VMA, DMA, heap, futex, classifier, PCIe, storage,
and Ethernet as absent or stubbed.

### Stub Terminal Behavior

Every accepted command must end in exactly one terminal path:

- normal response.
- canonical error response.
- event completion.
- cancellation/revocation response.
- structured fault event.
- watchdog/degraded-state fault.

No S0 stub may:

- mint a new capability without an existing root/mint authority.
- ignore a generation or domain field.
- park a thread without a wake, timeout, cancel, or fault source.
- return `EINPROGRESS` without a real operation id and completion path.
- expose raw physical interrupt, physical address, raw DMA, or ambient device
  authority to software.

### First Acceptance Tests

S0 is done only when these tests pass in simulation:

- reset reaches a stable state with exactly one runnable PID 1/TID 1.
- PID 1 executes `NOP`, immediate load, simple ALU, branch, SRAM `LD/ST`, and
  `YIELD`.
- `ENV_GET` reports expected S0 feature bits and limits.
- an unsupported opcode returns the canonical unsupported result.
- a stubbed resource instruction returns the expected fail-closed error.
- UART emits a boot/status byte or line.
- a synthetic event wakes or marks a parked thread through the Event Router.
- a synthetic stub-engine fault emits a structured fault event.
- a watchdog-injected stuck command reaches a defined degraded/fault terminal
  state.
- no test can observe raw physical interrupts, raw physical addresses, raw DMA
  authority, or ambient device authority.

### First Proof Obligations

The initial formal model can be smaller than the full architecture, but it must
cover the S0 state record:

- reset creates either a valid initial machine or an audited boot fault.
- every live thread has exactly one scheduler state/location.
- every accepted command has at most one terminal response/event/fault.
- every accepted command has at least one terminal path under the S0 fairness
  assumptions.
- stubs cannot create authority.
- unsupported operations fail closed.
- a parked thread names a valid wake, timeout, cancel, fault, or completion
  source.
- software-visible records contain no raw physical interrupt or raw physical
  address authority.

## Track A: Formal Model

Start with a Lean-style architectural model, not a full timing RTL model.

### A1. State Core

Model:

- GPR, FDR, PCR, thread, and process state.
- object ids and generations.
- Resource Domain tree roots.
- basic scheduler state.
- canonical errors.

Prove:

- state well-formedness.
- no forged FDRs.
- generation checks.
- domain parent validity.

### A2. Capability/FDR Engine

Model:

- `CAP_DUP`, `CAP_SEND`, `CAP_RECV`, `CAP_REVOKE`.
- narrowing, sealing, lineage epochs.

Prove:

- non-forgeability.
- no authority amplification.
- revoked authority cannot start new work.
- stale generation rejection.

### A3. Waitable/Scheduler Core

Model:

- ready, running, and wait states.
- `AWAIT`, wake, timers, and futex bucket head behavior.
- bounded scheduler eligibility.

Prove:

- exactly-one scheduler state/location.
- no lost wakeups.
- wake generation matching.
- domain budget eligibility.

### A4. Object Profiles

Model:

- counter.
- queue.
- event queue.
- call gate.

Prove:

- queue rights.
- explicit full/overflow behavior.
- gate continuation uniqueness.
- event source generation safety.

### A5. VMA/DMA Slice

Model:

- VMA permissions.
- page states.
- DMA buffer capabilities.
- pin, unpin, and revoke.

Prove:

- no invalid memory access.
- W^X, NX, and guard-page behavior.
- DMA confinement.

### A6. Servicelets/Classifiers

Model:

- servicelet verifier envelope.
- bounded action records.
- packet and generic record fields.

Prove:

- termination by construction.
- no authority creation.
- no arbitrary memory access.
- network action containment.

### A7. Resource Domains and Policy Enforcement

Model:

- nested domains, generations, lifecycle states, and monotonic budgets.
- attach/detach, freeze/resume, destroy, revoke, and query.
- delegated capability roots and policy masks.

Prove:

- children cannot exceed delegated authority or budgets.
- frozen/destroyed domains cannot dispatch new work.
- usage rolls up consistently through the domain tree.
- policy enforcement is fail-closed and cannot be bypassed by another engine.

### A8. Gate Delivery, Faults, and Compatibility Signals

Model:

- `GATE_CALL`, `GATE_RETURN`, asynchronous delivery, and handoff.
- fault delivery records.
- POSIX signal profile as a compatibility view over native delivery gates.

Prove:

- continuation uniqueness.
- stale or missing continuations do not resume the wrong caller.
- precise faults deliver at an architectural boundary.
- signal compatibility cannot create authority or bypass masks.

### A9. Memory Consistency, Coherence, and Visibility

Model:

- TSO-like normal memory rules.
- locked atomics and futex ordering.
- VMA/TLB invalidation.
- DMA visibility and device memory types.

Prove:

- single-copy atomicity for locked atomics.
- no access after unmap/revoke generation mismatch.
- DMA cannot observe or modify memory outside its capability and domain scope.
- cache/TLB invalidation reaches a defined quiescent point before authority is
  reused.

### A10. RAS, Adversarial Input, and Global Progress

Model:

- canonical fault classes, structured fault events, and watchdog/reset states.
- bounded queues and overflow behavior.
- malformed typed envelopes, records, packets, and servicelet programs.

Prove:

- adversarial inputs cannot hang an owner engine or create authority.
- a bounded local fault reaches a terminal path: response, event, degraded
  state, or machine-fatal fault.
- watchdog/local reset cannot silently corrupt unrelated domains.
- admitted realtime work has a bounded arbitration/progress path under the
  published assumptions.

## Track B: RTL Skeleton And Blocks

Start broad. The first RTL objective is not performance or full behavior; it is
getting the whole architectural skeleton right so later blocks have the right
interfaces, records, reset paths, ownership fields, and failure paths.

### B0. Whole-Machine Skeleton

`LNP64-RTL-S0` is a synthesizable top-level machine skeleton. Most modules may
be stubs, but the top-level shape must be representative of the real
architecture.

Required top-level modules:

- reset/boot/manifest shell.
- clock/reset domain shell.
- core tile shell.
- ISA format/opcode decode and profile-dispatch shell.
- fetch/decode/issue/retire shell.
- register/thread-context file shell.
- scheduler/runqueue shell.
- canonical error, cancellation, and completion-writeback shell.
- FDR/capability engine shell.
- Resource Domain engine shell.
- PCR/credential and policy-enforcement shell.
- object engine shell.
- typed control-envelope parser/validator shell.
- namespace dispatch and capability-return shell.
- stream/object instruction frontend shell.
- event router shell.
- gate/continuation engine shell.
- process/thread lifecycle engine shell.
- futex/atomic engine shell.
- heap engine shell.
- fault/telemetry/trace shell.
- watchdog/local-reset shell.
- measurement, attestation, audit, debug, and MLS hook shell.
- VMA/MMU/TLB-invalidation shell.
- page allocator shell.
- coherent memory fabric/cache shell.
- external DDR controller shell.
- shared metadata table broker shell.
- DMA fabric shell.
- service transaction boundary/continuation shell.
- classifier/servicelet/network steering shell.
- entropy/`ENV_GET`/machine-metadata shell.
- UART shell.
- SD/SPI flash shells.
- boot-image, block-object, and storage-barrier shell.
- simplified Ethernet packet-queue shell.
- PCIe root/IOMMU/MSI shells as optional empty ports for the first skeleton,
  with interfaces reserved.

Required architectural records:

- instruction decode record.
- opcode/profile/version feature record.
- engine command record.
- engine response record.
- completion record.
- event record.
- fault record.
- canonical error/cancellation record.
- FDR/capability record.
- object id/generation record.
- typed control envelope record.
- namespace selector and returned-capability proposal record.
- Resource Domain id/generation/accounting record.
- PCR/credential snapshot and policy-decision record.
- thread context/scheduler record.
- retire/park/submit record.
- waitable binding record.
- gate continuation record.
- process/exec-plan lifecycle record.
- VMA/page request record.
- TLB/cache invalidation record.
- coherence transaction record.
- heap allocation record.
- futex/atomic wait record.
- DMA descriptor/request record.
- storage barrier record.
- service request/reply continuation record.
- classifier/servicelet action record.
- watchdog/reset/degraded-state record.
- telemetry/trace record.
- audit/attestation/quote record.
- boot/measurement metadata record.

Required cross-cutting fields:

- ISA/profile version and feature id where relevant.
- owner PID/TID where relevant.
- owner Resource Domain id/generation.
- object id/generation.
- capability/FDR generation.
- lineage/revocation epoch where relevant.
- operation id for long or async work.
- rights/mask/profile bits.
- byte length, range, and bounds fields for memory/object work.
- latency class or WCET-class tag where the interface admits realtime work.
- policy, assurance, tenant, or label id where relevant.
- integrity state such as parity/ECC poison/corrected metadata bits where
  relevant.
- canonical result/error code.
- cancellation/revocation class.
- completion/event target.

Stub behavior:

- unsupported opcodes/profiles return `ENOTSUP` or a defined reserved-op fault.
- missing capabilities return `EBADF`, `EPERM`, `EACCES`, or `EREVOKED` as
  appropriate.
- full queues return `EAGAIN`, `EOVERFLOW`, park on a capacity event, or emit a
  pressure event according to the stub profile.
- long unsupported work may return `EINPROGRESS` only if it also creates a real
  completion token/event path; otherwise it must fail closed.
- every accepted stub command produces exactly one response, event, cancel,
  timeout, or fault.
- no stub may mint authority, bypass generation checks, or silently drop a
  parked thread.

Skeleton invariants:

- reset creates a root Resource Domain, PID 1 thread context, root scheduler
  state, and explicit initial FDR grants or an audited boot fault.
- every live thread has exactly one scheduler state/location.
- every parked thread has a waitable, operation id, timer, gate continuation,
  capacity event, fault source, or revoke source.
- every command/response path carries domain/generation metadata.
- every module has reset, idle, busy, fault, and degraded/stub states.
- every module exposes a minimal fault/telemetry counter.
- `ENV_GET` can report feature bits and skeleton limits.
- event routing and completion writeback exist even when most producers are
  stubs.
- unsupported features are visible through `ENV_GET` and fail with canonical
  errors instead of hidden partial behavior.
- command channels use bounded ready/valid handshakes or explicitly modeled
  queues.
- no software-visible path exposes physical interrupts, raw physical addresses,
  raw DMA authority, or ambient device authority.
- coherence, TLB invalidation, and cache/DMA visibility have stubbed event paths
  before any memory instruction can claim architectural completion.
- watchdog, degraded-state, and fault-event paths exist for every long-latency
  owner engine.

S0 simulation goals:

- reset reaches a stable machine state.
- PID 1 executes a tiny instruction stream from ROM/SRAM.
- unsupported native operations fail closed with canonical errors.
- UART can emit a boot/status byte or line.
- a synthetic event can route through the Event Router to the scheduler.
- fault injection into one stub engine produces a structured fault event rather
  than an unknown simulator hang.

S0 formal goals:

- global state well-formedness over the broad state record.
- no authority from stubs.
- exactly-one scheduler state/location.
- every accepted stub command has a defined terminal path.
- reset produces either a valid initial machine or a measured/audited boot
  fault.

After S0, fill vertical slices through this skeleton. Avoid implementing a
polished isolated block that cannot plug into the real top-level interfaces.

### B1. ISA Decode, Canonical Errors, and `ENV_GET`

Implement:

- fixed instruction format decode records.
- opcode/profile feature table.
- canonical error and fault-code constants.
- result-register and thread-local `ERRNO` writeback convention.
- `ENV_GET` skeleton for feature bits, limits, latency classes, topology, and
  unsupported-feature reporting.

Runs:

- decode table tests.
- unsupported-opcode and unsupported-profile tests.
- `ENV_GET` feature discovery tests.

Why first:

- every later block needs stable opcodes, result/error conventions, and feature
  discovery before its behavior can be tested or proven.

### B2. Minimal Core Tile

Implement:

- fetch/decode/execute.
- GPR file.
- simple branch.
- load/store to simulated SRAM.
- hardware thread context switching.
- retire, park, and submit records.
- no cache initially.

Runs:

- tiny assembly programs in simulation.
- bounded retire/park/submit timing checks.

### B3. FDR/Capability Table Block

Implement:

- small on-chip FDR table.
- generation checks.
- `CAP_DUP`.
- narrow/seal metadata bits.
- invalid/stale rejection.

Co-sim:

- compare RTL results to emulator/model.

### B4. Scheduler/Waitable Block

Implement:

- ready queue.
- active window.
- `YIELD`.
- `AWAIT`.
- wake event.
- timer event.
- Resource Domain budget admission hook.
- frozen-domain dispatch rejection.

Runs:

- ping-pong.
- timer wait.
- futex-like wake.
- frozen-domain and exhausted-budget dispatch tests.

### B5. Object Queue/Counter Block

Implement:

- bounded queue.
- counter/wait threshold.
- overflow behavior.
- event generation.
- queue/counter object profiles through `OBJECT_CTL`.

Runs:

- producer/consumer.
- pipe-like test.

### B6. Gate/Continuation Block

Implement:

- `GATE_CALL`.
- `GATE_RETURN`.
- continuation slots.
- stale continuation rejection.
- synchronous, asynchronous, and handoff mode records.
- delivery-gate entry for fault/signal/supervisor profiles.

Runs:

- service call roundtrip.
- delivered fault/gate roundtrip.

### B7. Process/Thread Lifecycle Block

Implement:

- minimal `CLONE`, `EXIT`, and `JOIN`.
- exec-barrier state machine stub.
- parent/child waitable state.
- sibling-thread stop/invalidate path for future `EXEC`.

Runs:

- clone/exit/join tests.
- exec-barrier stub cancellation tests.

### B8. Tiny VMA/MMU Block

Implement:

- simple page/VMA SRAM.
- permissions.
- NX and guard checks.
- page fault event.
- TLB invalidation event path.
- VMA generation checks.
- minimal cache/coherence visibility hooks.

Runs:

- memory protection tests.
- W^X/NX/guard tests.
- stale VMA generation tests.

### B9. DMA/Memory Object Block

Implement:

- copy/fill engine.
- permission checks.
- completion event.
- revoke-before-submit rejection.
- coherent visibility and cache/TLB invalidation handshake stubs.
- DMA buffer object profile.

Runs:

- DMA copy/fill tests.
- revoke and domain-isolation tests.

### B10. Typed Control, Namespace Dispatch, and Service Boundary

Implement:

- typed control-envelope parser/validator.
- namespace selector dispatch stub.
- service request/reply continuation records.
- capability-return proposal validation and install.
- service crash/cancel/error completion paths.

Runs:

- `OPEN_AT` dispatch-to-stub tests.
- returned-capability narrowing tests.
- service cancellation and stale-service tests.

### B11. Futex/Atomic Block

Implement:

- `LOCK_CMPXCHG` through the coherent atomic port.
- futex-flavored `AWAIT` expected-value check.
- futex `WAKE`.
- hot bucket head/tail state with DDR-spill stubs.

Runs:

- compare-exchange tests.
- no-lost-wakeup futex tests.
- stale VMA/address rejection tests.

### B12. Heap Block

Implement:

- default heap object metadata shell.
- per-thread allocation window.
- size-class hit path.
- `ALLOC`, `FREE`, and `ALLOC_SIZE` for a small fixed profile.
- double-free/stale-pointer rejection where metadata is resident.

Runs:

- allocation/free/reuse tests.
- cross-thread free handoff tests.
- guard/quarantine hardening tests.

### B13. Classifier, Servicelet, and Networking Prototype

Implement:

- fixed classifier table first.
- tiny servicelet engine second.
- bounded instruction subset.
- action record output.
- packet queue and generic record profiles.
- servicelet verifier envelope.
- per-domain servicelet-cycle budget hook.

Runs:

- packet record steering tests.
- IPC record steering tests.
- verifier rejection tests.

### B14. RAS, Observability, and Assurance Block

Implement:

- metadata parity/ECC injection hooks.
- watchdog/local-reset state.
- telemetry counters and scoped telemetry FDR reads.
- small trace ring.
- measurement/attestation records and quote-FDR stub.
- audit/debug/MLS control hooks as capability-scoped records.

Runs:

- ECC/parity poison tests.
- watchdog timeout and degraded-state tests.
- scoped telemetry/trace tests.
- measured-boot and quote-stub tests.

## Track C: Co-Simulation

Every RTL block should have a matching harness:

- run the same input vector in emulator/model and RTL simulation.
- compare architectural state, result codes, event records, and FDR generations.
- generate random but bounded traces from the formal model where practical.
- use Verilator for fast CI.
- later add FPGA simulation and synthesis checks.

The emulator remains the executable architectural oracle until the formal model
is strong enough to generate authoritative traces directly.

## Track D: FPGA Bring-Up

Bring-up should start with the broad skeleton, then fill the smallest useful
vertical slice:

1. top-level skeleton modules connected with stub responses.
2. fixed decode table, canonical errors, and `ENV_GET`.
3. soft SRAM only, no DDR.
4. UART output.
5. one core tile and simple assembler program ROM.
6. FDR/capability table and generation checks.
7. scheduler, waitable, event router, and object queue smoke.
8. gate/continuation and process lifecycle smoke.
9. tiny VMA/MMU, TLB invalidation, and memory-protection smoke.
10. external DDR and shared metadata broker.
11. DMA/memory-object smoke.
12. typed control, namespace/service dispatch, and capability-return smoke.
13. futex/atomic and heap smoke.
14. SD/SPI and storage-barrier smoke.
15. Ethernet packet queue, classifier, and servicelet smoke.
16. RAS/telemetry/watchdog/attestation smoke.
17. PCIe later.

## First Milestone: Whole-Machine Skeleton

`LNP64-RTL-S0` should prove that the overall architecture shape is correct
before any single block becomes sophisticated.

Required slice:

- synthesizable top-level machine.
- all major module shells present.
- command, response, event, fault, completion, capability, domain, scheduler,
  policy, VMA, DMA, service, RAS, and telemetry records defined.
- reset/boot path creates root domain, PID 1, scheduler state, and initial FDRs
  or emits a measured/audited boot fault.
- stubs fail closed with canonical errors.
- `ENV_GET` exposes feature bits, limits, topology stubs, and latency-class
  stubs.
- event router and completion writeback paths exist.
- coherence/TLB/DMA visibility stubs exist before memory operations complete.
- no raw physical interrupts, raw DMA, raw physical addresses, or ambient device
  authority are software-visible.
- UART boot/status output works in simulation.
- Verilator simulation runs without hanging on stubbed operations.

Proof targets:

- global state well-formedness over the broad skeleton state.
- no authority from stubs.
- no accepted stub command without a defined terminal path.
- exactly-one scheduler state/location.
- no software-visible raw interrupt/physical-address path.
- reset produces valid initial state or measured/audited boot fault.

Expected demo:

- reset the skeleton.
- run a tiny PID 1 ROM/SRAM program.
- print status over UART.
- query `ENV_GET` feature bits.
- issue one unsupported native command and observe canonical failure.
- inject one stub-engine fault and observe a structured fault event.

## Second Milestone: Proven Ping-Pong Machine

`LNP64-RTL-M1` should prove that the architecture is real enough to execute code
outside the Rust emulator.

Required slice:

- B1 through B5 implemented enough for the test.
- two hardware thread contexts.
- small FDR table with generation checks.
- queue object.
- `PUSH`, `PULL`, and `AWAIT`.
- scheduler ready/wait transitions.
- event wake or gate wake.
- Verilator simulation.
- co-simulation against the emulator.

Proof targets:

- no forged FDR.
- no lost wakeup.
- exactly-one scheduler state/location.
- stale generation rejection.
- queue full behavior is explicit.

Expected demo:

- a tiny assembly ping-pong program runs in RTL simulation.
- the same program runs in the emulator.
- architectural traces match at committed instruction/event boundaries.

This milestone is intentionally small. If it works, the project has a real
proof/RTL/emulator loop and can grow block by block without betting everything
on a whole-chip rewrite.
