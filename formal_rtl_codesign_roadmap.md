# LNP64 Formal/RTL Co-Design Roadmap

This roadmap defines a parallel path toward complete Lean proofs and a complete
synthesizable SystemVerilog implementation of the LNP64 chip. Real FPGA hardware
is not available yet. The immediate hardware target is therefore a full-chip RTL
design that is buildable, lint-clean, and simulatable under Verilator or an
equivalent simulator. FPGA bring-up comes after the full-chip design has a
credible simulation/proof base.

The current RTL tree is not that implementation yet. It is a useful broad
skeleton plus bounded S0/M1-M15 smoke slices: top-level shells, record shapes,
small standalone engines, testbenches, and trace witnesses. That work is
valuable only as scaffolding. It must not be mistaken for the real processor,
for a complete hardware implementation of the ISA, or for a finished
enterprise/realtime design.

`rtl/top/lnp64_top.sv` is the intended complete top-level implementation file.
It should not be treated as a throwaway smoke harness. Early versions may expose
simulation status outputs and connect fail-closed shells, but the same top-level
module must grow into the integrated chip: real core tiles, scheduler, memory
fabric, VMA/MMU, capability/domain engines, object/gate engines, DMA, storage,
networking, RAS, boot, and external interfaces. Test-only behavior should move
into testbenches, bind modules, trace adapters, or explicitly named simulation
helpers rather than becoming the architecture.

The goal is not to write RTL first and prove it later. The goal is:

```text
executable spec -> proof model -> reference emulator -> RTL block
-> RTL simulation -> FPGA bring-up
```

The end state is not a partial demo core. The end state is:

- the full chip design expressed in synthesizable SystemVerilog.
- the whole machine simulatable with real LNP64 programs and architectural test
  images.
- existing assembly demos and compiler-generated test programs running through
  the top-level RTL simulator as features come online.
- a complete Lean architectural model covering the theorem set in
  `formal_theorems.md`.
- Lean proofs for the security, isolation, scheduler, capability, memory,
  servicelet, RAS, and global-progress theorems we have specified.
- RTL assertions and co-simulation traces linked back to the Lean model wherever
  practical.
- later FPGA board support once suitable hardware exists.

Each hardware block should have:

- a small abstract model.
- invariants and theorem targets.
- test vectors generated from the model or emulator.
- synthesizable RTL.
- RTL assertions.
- co-simulation against the emulator/model.
- simulation gates.
- synthesis constraints.
- FPGA smoke tests once real FPGA hardware is available.

The sequencing rule is:

```text
interfaces first -> fail-closed scaffolding second -> real integrated engines third
-> full architectural coverage fourth -> performance, timing, and FPGA later
```

Stubs are acceptable only when they preserve the real command/response shape,
carry authority/generation/domain metadata, fail closed, and keep reset,
fault, event, and completion paths live.

Stubs were enough to build the skeleton, and that was the right first step.
They showed that the project could name the machine, wire the major modules,
exercise reset, route commands/events/faults, and keep a proof/simulation gate
alive. That phase is now only the starting point. From this point forward,
stubbed behavior must be retired block by block and replaced with real
integrated RTL.

A block is not implemented merely because it has a shell, a smoke trace, or a
bounded single-scenario model. The roadmap target is a complete RTL
implementation of every required architectural block and complete Lean proofs
of every theorem we claim for the architecture.

The active hardware work should now move from scaffolding to real
implementation. "Real implementation" means synthesizable RTL state machines,
pipelines, arbiters, queues, tables, memories, and fabric logic that execute the
architectural behavior described in `design.md` and `hardware_design.md`, not
testbench-scripted traces or hardcoded success paths.

The active proof work should now move from coverage plumbing to deep proofs.
The existing manifests, schema checks, trace gates, and roadmap audits are
useful guardrails, but they are not the hard result. Do not add new manifest or
checker layers unless a real proof, typed transition trace, RTL assertion, or
top-level integration gate needs them. The next proof milestone is one honest
vertical proof slice deep enough to withstand review:

```text
Lean transition system -> reachable-state invariant proof
-> typed RTL commit records -> executable/refinement comparison
-> RTL assertions for assumptions -> top-level integration gate
```

The preferred first slice is `SG-AUTH` through the capability/FDR path. It
should prove non-forgeability, no authority amplification, stale-generation
rejection, revocation safety, and valid capability transfer for all reachable
states in the modeled slice. Other evidence work is secondary until this shape
exists.

## Execution-First Correction

The roadmap must not drift into proof artifacts that do not accelerate a real
machine. From this point forward, the main RTL question is:

```text
Can a program produced by the current assembler or LLVM path run through
rtl/top/lnp64_top.sv under simulation and produce the same architectural result
as the emulator?
```

Proof work remains central, but it should ride beside executable RTL rather
than replace it. The fastest useful progress is a narrow but honest vertical
machine:

1. fetch fixed LNP64 instructions from a ROM/SRAM image.
2. decode and execute the compiler-critical scalar baseline.
3. perform load/store against a simple memory model.
4. retire instructions with typed trace records.
5. support `ENV_GET`, canonical unsupported-opcode failure, and `EXIT`.
6. load the same tiny image into the emulator and RTL simulator.
7. compare final registers, memory checksums, errno/status, event/fault records,
   and retire traces.

Until this path exists, avoid opening new proof/checker surfaces unless they
directly help this execution loop. The first serious chip-design milestone is
not another isolated block trace; it is `lnp64_top` running real project code,
even if the first code is tiny and the memory system is SRAM-backed.

The near-term priority order is therefore:

1. **Top-level executable core path:** instruction memory, fetch/decode/execute,
   register file, PC/branch/call/return, scalar ALU, load/store, atomics
   stubbed or implemented as required by the test, retire trace, and `EXIT`.
2. **Program-image path:** one build script that takes an assembly file or an
   LLVM-generated object/flat image and feeds the same bytes to emulator and
   RTL simulation.
3. **Architectural comparison:** a checker that compares committed instruction
   traces and final architectural state, not just string markers.
4. **Then M1 authority refinement:** once real instruction retirement exists,
   drive the M1 capability/FDR path from actual instructions through
   `lnp64_top`, not only through a standalone M1 harness.

This does not reduce the proof ambition. It makes the proof target concrete:
prove the machine that is actually fetching, decoding, retiring, and running
programs.

## Severe Whole-Chip Proof Goals

The proof program should stay focused on a small set of severe top-level
claims. Local module proofs matter only because they support these claims at
the whole-chip boundary:

1. **`SG-AUTH` No forged authority:** no instruction, service, engine, trace,
   event, fault, DMA operation, reset path, stale cache entry, or compatibility
   personality can create, broaden, revive, or transfer capability authority
   outside the capability rules.
2. **`SG-ISO` Resource Domain isolation:** a domain cannot read, write,
   schedule, DMA into, signal, observe, debug, trace, or receive events for
   another domain except through explicitly delegated capabilities and policy.
3. **`SG-SCHED` Scheduler uniqueness:** every live TID is in exactly one
   scheduler state and at most one tile-local running lane; migration, wakeup,
   fault delivery, and cancellation cannot duplicate or lose thread context.
4. **`SG-WAKE` No lost waits or completions:** every accepted wait, gate call,
   futex wait, event subscription, DMA request, service request, or long engine
   operation either completes, wakes, cancels, times out, faults, parks on a
   valid waitable, or reaches a documented terminal/degraded/machine-fatal path.
5. **`SG-MEM` Memory and DMA authority:** VMA, TLB, cache, page-state, W^X/NX,
   guard, revocation, coherence, and DMA/IOMMU rules prevent stale,
   unauthorized, or cross-domain memory access.
6. **`SG-TOTAL` Hardware totality and local progress:** every hardware FSM,
   pipeline, owner engine, queue, arbiter, decoder, reset path, and recovery path
   has only defined reachable states. Under its stated environment assumptions,
   each reachable nonterminal state makes bounded forward progress, applies
   bounded backpressure with a valid release condition, parks on a valid
   wake/cancel/fault source, fails closed, resets locally, or escalates to a
   measured/audited machine-fatal state. No block may spin forever, wait on an
   impossible condition, hold unbounded backpressure, or require software
   intervention to escape an internal invalid state.
7. **`SG-PROGRESS` Fault containment and bounded recovery:** within the stated
   reset, clock, fabric, and external-IP assumptions, faults, malformed records,
   poison, watchdog timeout, revocation races, and resource exhaustion resolve to
   typed progress, typed refusal, bounded parking, degraded state where the
   lifecycle profile permits it, local reset, or measured/audited machine-fatal
   state without publishing partial authority or corrupting unrelated domains.
8. **`SG-RT` Realtime honesty:** any advertised Class A/B/C latency,
   scheduler, reservation, fabric, memory-controller, async-cancellation, or
   servicelet bound is conservative for the implementation; work that cannot meet
   the bound is Class D bounded-submit work, best-effort, or fails closed.
9. **`SG-EVIDENCE` Evidence honesty:** trace, telemetry, audit, quote,
   proof-manifest, and feature-discovery records are data, not authority, and
   accurately describe the implementation, assumptions, proof level, and known
   gaps for the boot epoch.

These claims should have stable ids in the theorem/RTL coupling manifest and
the human-readable evidence index. A proof that does not advance one of these
claims is allowed as a helper, regression witness, or local sanity check, but it
should not be presented as an architectural guarantee.

## Proof/RTL Coupling Contract

Lean proofs are valuable only if they describe the hardware that is actually
being built. The project should therefore treat the Lean model, RTL, simulator
traces, assertions, and manifests as one refinement chain rather than parallel
artifacts.

The intended trust chain is:

```text
architectural schema -> Lean transition model -> theorem obligations
-> generated RTL/assertion/trace schemas -> RTL module contract
-> RTL simulation/formal checks -> top-level integration evidence
```

The long-term goal is not merely "Lean file exists" or "RTL test passes." The
goal is a checked refinement claim for each block: every committed RTL-visible
architectural transition for that block corresponds to an allowed Lean
transition under an explicit set of assumptions.

The project should avoid evidence plumbing debt. A new JSON manifest, checker,
Markdown index, or trace vocabulary is justified only when it tightens this
chain: it must catch a real drift mode, connect a theorem to RTL behavior, record
an assumption that affects a proof, or make a top-level claim auditable. Pure
bookkeeping should be folded into an existing manifest or deferred.

### Proof Artifact Levels

Do not treat every Lean file as the same kind of evidence. The project should
use four distinct artifact levels:

- **Coverage artifact:** records that a theorem-roadmap topic has a named file,
  theorem name, manifest entry, and evidence link. This is useful bookkeeping,
  but it is not a proof of the architecture.
- **Bounded witness model:** proves properties of one scripted trace or small
  bounded scenario. This is useful for early RTL bring-up and regression, but it
  does not prove that all reachable states are safe.
- **Transition-invariant proof:** defines `State`, `Step`, `Reachable`, and one
  or more invariants, then proves each transition preserves the invariant. This
  is the first level that should be called a real architectural proof.
- **Refinement proof:** relates RTL-visible commit records to Lean transitions
  and proves that the RTL preserves the architectural invariant under explicit
  clock/reset/fairness/vendor-IP assumptions.

The current `formal/FormalTheoremsModel.lean` style should be treated as a
coverage artifact only. It may keep the manifest honest, but a theorem proved
by setting a coverage field to `true` and using `rfl` is not an architectural
guarantee. The current M1-M15 Lean files are bounded witness models unless and
until they define a transition relation and prove invariants over all reachable
states in the modeled slice.

This naming discipline matters. Public claims should say exactly what has been
proved: coverage exists, a bounded trace checks out, an invariant is preserved,
or RTL refines the Lean transition model.

### Shared Schemas

Architectural records must not be hand-copied between Lean, SystemVerilog, Rust,
and Python. For each stable record family, keep a single schema source or a
schema manifest that can generate or check all consumers:

- opcodes, profiles, status codes, and canonical errors.
- command, response, completion, event, fault, capability, domain, VMA, DMA,
  gate, object, scheduler, service, and telemetry records.
- feature bits, latency classes, rights masks, and lifecycle states.
- trace event names and normalized fields used by co-simulation.

The first acceptable implementation can be a checked manifest plus parsers. The
preferred end state is generated Lean structures, SystemVerilog `typedef struct
packed` records, Rust constants, Python model constants, and Markdown tables
from the same source. A CI check must fail if a field, width, enum value, or
trace event drifts without updating the schema.

### Refinement Artifacts

Each milestone block should carry these artifacts:

- `formal/Mx...Model.lean`: abstract state, transition relation, reachability
  definition, invariants, and theorem statements/proofs.
- `rtl/engines/lnp64_mx_*.sv`: RTL implementation.
- `formal/rtl_assertions/lnp64_mx_assertions.sv`: generated or hand-audited
  assertions mirroring the Lean preconditions, invariants, and postconditions.
- `formal/mx_*_model.py` or an extracted/generated executable model: produces
  canonical traces from the same abstract transition shape.
- `tests/rtl/mx_filelist.f`: exact RTL files under test.
- manifest entries tying theorem names, RTL modules, assertions, traces, and
  scripts together.

The manifest should answer: which theorem is this RTL transition meant to
support, which RTL signals witness it, which assumptions are required, which
assertions check those assumptions, and which simulation/formal gate exercised
it.

For a proof slice to advance beyond bounded-witness status, its Lean model must
include:

- a typed `State` record with authority, generation, scheduler, fault, event,
  and ownership fields relevant to that slice.
- a typed `Input` or command/event record.
- an inductive or functional `Step` relation.
- a `Reachable` definition from reset or a stated pre-state.
- at least one nontrivial invariant stated over arbitrary reachable states.
- preservation lemmas for each transition constructor or operation case.
- a top-level theorem of the form `forall s, Reachable s -> Invariant s`.

Fixed final-state proofs remain useful regression witnesses, but they must not
be the final proof shape for security, isolation, revocation, scheduler,
memory, DMA, servicelet, or RAS claims.

### Refinement Checks

For every block, the normal gate should check more than textual trace equality:

- decode RTL commit events into typed architectural transition records.
- run the corresponding Lean/executable transition from the same typed
  pre-state and inputs.
- compare post-state projections: authority, generations, scheduler location,
  wait sources, memory permissions, result/error, event/fault records, and
  telemetry counters.
- check that every RTL terminal path is one of the Lean terminal paths.
- check that every Lean theorem assumption is either proven in Lean, asserted in
  RTL, constrained by the top-level environment, or named as a trusted hardware
  assumption.

String traces are acceptable early scaffolding, but they are not the final
coupling. They should evolve into typed trace records generated from the shared
schema.

The preferred path is:

```text
bounded string trace -> typed transition trace -> executable Lean transition
comparison -> checked RTL-to-Lean refinement relation
```

At the typed-trace stage, every trace field used by a proof must come from the
shared schema. At the refinement stage, the trace is no longer the proof; it is
debug evidence for a checked relation between RTL commit events and Lean steps.

### Convincing Security and Crash-Freedom Bar

The project should not claim the hardware is convincingly free of security or
crash/stall bugs until the proof story includes the following artifacts. These
are stronger than ordinary guardrails; they are the minimum credible path from
local proofs to whole-chip assurance.

1. **RTL-to-model refinement:** every RTL architectural commit record decodes to
   an allowed Lean transition for the relevant state projection, or to a typed
   fail-closed terminal path. This is the main bridge from "the model is safe"
   to "the chip implements the safe model."
2. **Mediation completeness:** every write to authority-bearing state is owned
   by exactly one checked engine or proven shard. This covers FDR tables, VMA
   tables, domain policy, scheduler slots, continuations, IOMMU tables, object
   state, debug authority, trace authority, service-return capability install,
   and reset/recovery state. There must be no alternate RTL write path.
3. **No-stuck-state theorem:** every accepted command, request, wait, gate,
   DMA operation, service operation, or long owner-engine operation eventually
   reaches completion, park on a valid waitable, canonical error/fault, cancel,
   timeout, degraded state where the lifecycle permits it, or measured/audited
   fatal state. Watchdogs are fault containment, not normal progress proof.
4. **Reset and recovery correctness:** reset, local engine reset, poison,
   abort, degraded state, and recovery cannot publish partial authority,
   duplicate scheduler state, skip generation checks, or revive stale objects.
5. **Assume-guarantee composition:** each block publishes assumptions and
   guarantees; neighboring blocks either discharge those assumptions with
   assertions/proofs or list them as trusted-platform assumptions. Whole-chip
   claims are allowed only after the relevant assumptions compose at
   `lnp64_top`.
6. **Information-flow and noninterference:** isolation proofs cover observation
   as well as mutation. Loads, debug, telemetry, trace rings, audit, counters,
   scheduler pressure, queue occupancy, classifier marks, packet queues,
   snapshot hooks, and timing-visible features are scoped by Resource Domain
   policy or explicitly excluded from the claimed profile.
7. **Bounded parser and adversarial-input proofs:** servicelets, packets,
   typed control envelopes, PCIe/config records, boot manifests, service
   replies, returned-capability proposals, and restore records have bounded
   parse depth, terminate, fail closed on malformed input, and do not allocate
   hidden authority-bearing state while parsing.
8. **CDC, reset-domain, and external-IP contracts:** clock crossings, reset
   release, PLLs, DDR, PCIe, Ethernet PHY/MAC, FPGA memories, SERDES, and
   vendor primitives are either proved locally or represented by named
   assume-guarantee contracts with reset behavior, ordering, integrity,
   maximum-wait/failure behavior, and fault signaling.
9. **Arithmetic and bounds safety:** address arithmetic, queue indices, bank
   selectors, generation counters, epochs, lengths, offsets, byte masks,
   packet sizes, DMA ranges, and table ids cannot overflow, wrap, truncate, or
   alias into authority outside the checked range. Any intentional wrap has a
   generation/epoch theorem and stale rejection proof.
10. **Proof coverage audit:** every top-level security/progress claim has a
   reviewer-readable row showing the claim, scope, assumptions, Lean theorem,
   RTL modules/signals, assertions, simulation/formal evidence, trust level,
   and remaining gaps. Evidence rows are data, not authority; they must not
   inflate T0/T1 coverage into T4/T5 refinement.

The intended final assurance statement is:

```text
Lean proves the architectural security and progress invariants.
RTL refinement proves the chip implements only those transitions.
Assume-guarantee composition proves the blocks preserve each other's
assumptions at lnp64_top.
Fault/recovery proofs show failures are contained and authority-decreasing.
Information-flow proofs show isolation includes unauthorized observation,
not only unauthorized mutation.
```

### Assumption Discipline

The proof gate must track assumptions as first-class objects:

- no `sorry`, `admit`, unchecked `axiom`, or hidden trusted lemma in production
  proof files.
- every environment assumption has an owner: reset, clocking, ready/valid
  fairness, bounded queue arbitration, memory model, vendor IP, or external
  device contract.
- every RTL `assume` has a matching Lean assumption or top-level environment
  contract.
- every Lean assumption has at least one of: RTL assertion evidence, bounded
  model-check evidence, simulation coverage, synthesis/CDC constraint evidence,
  or an explicit trusted-platform entry.
- black boxes such as DDR PHYs, FPGA PLLs, SERDES, and vendor PCIe/DDR IP are
  modeled as assume-guarantee contracts with named guarantees and failure modes.

This keeps the trusted computing base visible. A theorem should never appear to
prove more about the physical machine than the recorded assumptions support.

### Trust Levels

Use visible trust levels for each theorem/block pair:

- **T0 Sketch:** Lean theorem names and RTL tests exist, but coupling is mostly
  manual.
- **T1 Bounded Witness:** a Lean/Python/executable model proves or checks a
  scripted bounded scenario and RTL traces match that scenario.
- **T2 Assertion-Coupled:** RTL assertions check the local invariants and
  theorem assumptions at module boundaries, and typed traces compare RTL
  behavior to the executable model for representative and randomized inputs.
- **T3 Transition-Proven:** Lean proves an invariant for all reachable states of
  the block's abstract transition model.
- **T4 Refinement-Coupled:** a checked refinement artifact maps RTL transitions
  to Lean transitions for the block's architectural state projection.
- **T5 Integrated/Board-Qualified:** top-level simulation/formal evidence shows
  the block's guarantees survive composition with neighboring engines and
  shared fabrics. Later FPGA evidence confirms the same interfaces and
  assumptions hold on selected hardware.

Current early milestones may start at T0/T1. Security, isolation, authority,
DMA, Resource Domain, scheduler, and RAS guarantees should not be advertised as
strong hardware guarantees until they reach at least T4 for the relevant RTL
block, and T5 for cross-module or board-dependent properties.

### Human-Auditable Evidence

The proof system must also be legible. A casual technical observer should be
able to start at a small set of top-level claims and follow the evidence without
reverse-engineering the whole repository.

For each major guarantee, maintain a short evidence page or generated index
that shows:

- the severe whole-chip goal id it supports.
- the plain-English claim.
- the precise security/progress/realtime property being claimed and the scope:
  whole chip, block, profile, bounded witness, or external-IP contract.
- the exact Lean theorem names and their artifact level: coverage, bounded
  witness, transition proof, or refinement proof.
- the assumptions and trusted-platform contracts used by those theorems.
- the RTL modules, top-level `lnp64_top` signals, schema records, and trace
  fields covered by the claim.
- the assertion files, simulation gates, co-simulation traces, synthesis checks,
  and board evidence that connect the theorem to the implemented hardware.
- the current trust level, from T0 through T5.
- known gaps, exclusions, unproven assumptions, unconnected RTL, missing
  assertions, and reasons the claim is not stronger.

The top-level evidence index should be organized by the guarantees users care
about: no forged authority, revocation works, domains contain tenants, DMA is
confined, scheduler state cannot split, wakeups are not lost, servicelets
terminate, faults reach terminal paths, and admitted work makes bounded
progress. Each row should link to the theorem, RTL, assertion, trace, and gate
artifacts that support it.

The review question for every row is deliberately simple:

```text
What is claimed?
What assumptions make it true?
What RTL implements or witnesses it?
What proof/check/evidence connects the RTL to the claim?
What remains unproven?
```

This is not marketing material. It is a review surface for engineers, users,
security reviewers, open-source contributors, and future hardware partners. If a
guarantee cannot be explained through this chain, the guarantee is not yet
usable as an engineering claim.

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
- `rtl/core/`: replicated core tiles, fetch/decode/issue/retire, register
  files, thread context windows, and tile-local scheduler front ends.
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
- `lnp64_core_tile` replicated by `CORE_TILE_COUNT`
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

### Multicore From S0

The RTL skeleton should be multicore-shaped from the start. The default
simulation configuration is two coherent in-order core tiles. The top-level
should be parameterized so four tiles can be enabled for stress tests without
rewriting the fabric. A one-tile build may exist only as a debug convenience; it
must not be the proof or integration baseline.

S0 does not need a high-performance coherence implementation, but it must expose
the real multicore interfaces:

- per-tile fetch/issue/retire and local ready/park/submit channels.
- tile id in retire, fault, event, scheduler, trace, and coherence records.
- a global scheduler path that can assign runnable TIDs to at least two tiles.
- an inclusive-L2 or coherence-shell interface with invalidate/ack/writeback
  event records, even if early data traffic is SRAM-backed.
- per-tile reset, fault, watchdog, idle, and telemetry counters.
- `ENV_GET` topology records reporting tile count, active-window shape,
  coherence-domain id, and enabled/disabled tile state.
- scheduler records carrying hard affinity mask, current tile, preferred tile,
  and migration generation.

This prevents single-core assumptions from leaking into scheduler,
capability/domain, VMA/TLB, event, DMA, and proof work. Early tests may run PID
1 on tile 0, but the top-level machine is incomplete until a second tile can be
reset, observed, scheduled, faulted, idled, and woken through the same fabric.

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
9. release all enabled core tiles, with PID 1 initially runnable on tile 0.
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
- at least two enabled core tiles reset, report telemetry, and participate in
  the scheduler topology.
- PID 1 executes `NOP`, immediate load, simple ALU, branch, SRAM `LD/ST`, and
  `YIELD`.
- `ENV_GET` reports expected S0 feature bits and limits.
- `ENV_GET` reports tile count, tile ids, and a single coherent locality domain
  for the S0 profile.
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

Start with a Lean architectural model, not a full timing RTL model. Early files
may be small executable/proof sketches, but the target is a complete Lean model
and proof suite for the theorem set in `formal_theorems.md`.

The first cleanup pass in Track A should classify existing Lean files:

- `FormalTheoremsModel.lean` is a coverage ledger unless replaced by real
  abstract theorem statements and proofs.
- S0 and M1-M15 files are bounded witness models unless they include `State`,
  `Step`, `Reachable`, invariants, preservation lemmas, and reachable-state
  theorems.
- manifests should record this artifact level so reviewers can see what is
  actually proven.

After classification, choose one vertical slice and make it genuinely
mathematical before duplicating the pattern. The preferred first real slice is
the FDR/capability engine, because authority is the architecture's security
root and the state space is still small enough to model cleanly.

The first real Lean slice should:

- define capability, object, domain, FDR-table, queue, event, and scheduler
  projections needed for that slice.
- define operation inputs for `CAP_DUP`, `CAP_SEND`, `CAP_RECV`, `CAP_REVOKE`,
  `PUSH`, `PULL`, and `AWAIT` only if those operations are part of the slice.
- define the allowed `Step` relation, including fail-closed error transitions.
- define `Reachable` from reset plus optional trusted initial grants.
- prove non-forgeability, no authority amplification, stale-generation
  rejection, and no-lost-wakeup over all reachable states in that slice.
- export or mirror typed transition records for the RTL testbench and Python
  co-sim model.

The Lean work is complete only when it covers:

- machine state well-formedness.
- capability non-forgeability, narrowing, sealing, lineage, and revocation.
- Resource Domain containment, monotonic delegation, and accounting.
- scheduler state, waitables, no-lost-wakeup, and bounded progress assumptions.
- multicore topology, tile-local running lanes, cross-tile scheduler handoff,
  coherence-shell acknowledgements, and tile-local fault containment.
- object profile state machines.
- gate delivery, continuations, faults, and compatibility signal profiles.
- memory permissions, W^X/NX/guards, VMA/TLB coherence, and memory consistency.
- DMA confinement and device-visible memory rules.
- service boundary transactions and returned-capability validation.
- classifier/servicelet termination, bounded action records, and containment.
- RAS, watchdog, adversarial input containment, and global progress under
  bounded faults.

Proofs may be phased, but every theorem advertised as an architectural guarantee
must eventually have a corresponding Lean statement and proof, plus a trace,
assertion, or test hook connecting it to the RTL where practical.

Every Track A theorem should declare which severe whole-chip proof goal it
supports. The declaration can be in the proof manifest at first, but the final
form should be mechanically checked by the theorem/RTL coupling gate. This keeps
the Lean work aimed at whole-chip authority, isolation, scheduler, memory,
totality, progress, realtime, and evidence-honesty claims instead of accumulating
unrelated local facts.

Near-term Track A work should be intentionally narrow. Until the first
capability/FDR transition-invariant slice reaches the first credible refinement
shape, do not expand the formal surface broadly except to remove false claims
or wire that slice to RTL. M1 is the template. If M1 stops at typed traces plus
assertions, every later proof slice is likely to copy that weakness. The
acceptance bar for the first deep slice is:

- a Lean `State`, `Step`, and `Reachable` model for the capability/FDR path.
- preservation lemmas for each modeled transition, including failure
  transitions.
- top-level reachable-state theorems for non-forgeability, no authority
  amplification, stale-generation rejection, revocation safety, and valid
  transfer.
- typed commit records from the corresponding RTL slice.
- a schema-owned commit/state-projection format shared by SystemVerilog, Lean,
  Python, and trace decoding, or mechanically checked against the same schema.
- an explicit refinement relation between RTL commit/state projection, Lean
  pre-state, Lean `Step`, and Lean post-state projection.
- assertions or explicit assumptions for every precondition the proof needs.
- a gate that demonstrates the RTL commit records match the executable model
  for representative and randomized traces.
- bypass/mediation assertions showing no alternate RTL path writes
  capability/FDR authority outside the owner transition path.

This is a hard work-order gate, not a preference. The next vertical slice must
not start while M1 is still only "typed traces plus assertions." During this
phase, broad new theorem files, new manifest/checker layers, and new RTL smoke
slices are justified only when they directly tighten the M1 refinement bridge:
schema ownership, RTL state projection, Lean transition preservation, executable
comparison, bypass mediation, or explicit trust-level accounting.

Only after that pattern is real should the project repeat it for scheduler
uniqueness, no-lost-wakeups, VMA/DMA authority, Resource Domain isolation, and
fault/progress containment. Do not start the next vertical slice merely because
the M1 trace checker is passing; the M1 row must state what is already
transition-proven, what is assertion-coupled, what is refinement-coupled, and
what remains for T4.

### Immediate Execution Focus

The efficient path from the current repository state is not to start more broad
proof slices. Finish one vertical slice deeply enough that it becomes the
template for the rest. The active slice is `SG-AUTH` through M1 capability/FDR.

Do this before opening another large proof area:

1. **Define the M1 refinement relation.** State exactly how an RTL
   capability/FDR commit record, an RTL state projection, a Lean `State`, and a
   Lean `Step` correspond. The relation should ignore irrelevant pipeline flops
   but include every authority-bearing field: object id/generation, FDR
   generation, rights, lineage, domain id/generation, operation id, status, and
   transfer/revocation state.
2. **Make M1 commit records schema-owned.** The typed commit record should be
   generated from, or mechanically checked against, the shared schema for
   SystemVerilog packed records, Lean records/projections, Python decoding, and
   any trace JSON. The schema is the source of truth; testbench JSON is only a
   decoded view and must not define an independent authority format.
3. **Prove the M1 model preserves `SG-AUTH`.** Preserve non-forgeability, no
   authority amplification, current-authority checks, valid transfer, revoke,
   stale-generation rejection, and fail-closed operations across all reachable
   states in the modeled slice.
4. **Check the RTL emits only valid M1 commits.** The RTL gate should decode
   typed commits and state projections from the schema-owned format, compare
   them against the executable model, and reject impossible authority
   transitions. This is still not a bit-level proof, but it is the bridge
   toward T4.
5. **Add bypass/mediation checks.** Prove or assert that no RTL path writes
   capability/FDR authority state except the M1 owner transition path or its
   explicitly named shard. This is the first concrete mediation-completeness
   check.
6. **Record the trust level honestly.** Every M1 evidence row should use the
   same vocabulary: model-only, typed-trace checked, assertion-coupled,
   transition-proven, refinement-coupled, or top-level composed. Until a
   checked RTL-to-Lean refinement relation exists, call the slice
   transition-proven plus typed-trace/assertion coupled, not finished T4/T5
   hardware assurance.

M1 reaches the minimum shape for the next vertical slice only when the roadmap
and manifests can answer all of these questions in one place:

- which RTL packed record fields form the commit and state projection.
- which Lean `Step` each commit may represent.
- which post-state projection is compared.
- which SG-AUTH invariant is preserved for reachable Lean states.
- which RTL assertions rule out bypasses and unchecked preconditions.
- which assumptions remain outside the proof, and whether the current claim is
  model-only, typed-trace checked, assertion-coupled, transition-proven,
  refinement-coupled, or top-level composed.

The current M1 artifacts should be read against this checklist. A passing trace
checker means the source-owned record and executable comparison are improving;
it does not by itself prove a T4 RTL-to-Lean refinement. The remaining T4 gap
must stay explicit until a checked artifact shows that every accepted M1 RTL
commit corresponds to one allowed Lean transition and that all authority-bearing
post-state projection fields match the Lean post-state.

After M1 reaches this shape, repeat the same pattern in this order:

1. scheduler uniqueness, no-lost-wakeup, and the Fixed Weighted-Fair
   Virtual-Deadline Active-Window Scheduler contract.
2. composition of M1 authority with scheduler state.
3. VMA/DMA memory authority.
4. Resource Domain isolation and monotonic delegation.
5. fault/progress containment.

This order matters. Scheduler duplication or lost wakeups can invalidate memory,
domain, and service proofs. VMA/DMA proofs are much more meaningful after the
thread that issues the memory operation is known to be unique and live.

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
- fixed monotonic weight table, virtual runtime/deadline accounting, sticky
  affinity, bounded migration, bounded wakeup insertion, and bounded preemption
  points.
- bounded active windows or virtual-deadline buckets, with no scheduler
  bytecode, callbacks, red-black tree policy, plugin dispatch, or unbounded tree
  walks.
- hierarchical Resource Domain quota/budget eligibility.

Prove:

- exactly-one scheduler state/location.
- no lost wakeups.
- wake generation matching.
- domain budget eligibility.
- bounded fairness for eligible admitted work under the published active-window
  approximation.
- no hidden unbounded path in Class A/B/C retire, park, wake, or dispatch
  behavior.

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
- end-to-end thread store confinement: a committed store by a running thread is
  authorized by that thread's current PID/domain, live writable VMA, matching
  generation/lineage, and Resource Domain policy at the commit point.
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

- tile ids, enabled-tile mask, tile-local running lanes, and coherence-domain
  membership.
- hard affinity masks, preferred/current tile placement, migration generation,
  and allowed migration reasons.
- TSO-like normal memory rules.
- locked atomics and futex ordering.
- VMA/TLB invalidation.
- DMA visibility and device memory types.

Prove:

- no TID runs on two tiles at once.
- cross-tile migration preserves thread identity, registers, continuations,
  authority, delivery masks, `ERRNO`, and accounting.
- sticky placement is preserved unless a documented migration reason applies.
- migration generation rejects stale tile-local wakeups, completions, and
  balancing records.
- cross-tile wake/event/completion delivery cannot lose or duplicate a wakeup.
- tile-local fault, reset, watchdog, or degraded state cannot corrupt another
  tile's scheduler, capability, domain, or VMA state.
- `ENV_GET` topology records match initialized tile/coherence-domain state.
- coherence-shell invalidation, acknowledgement, writeback, and ownership
  transfer are paired before stores, remaps, DMA visibility, or authority reuse
  commit.
- single-copy atomicity for locked atomics.
- no access after unmap/revoke generation mismatch.
- no committed store can use stale TLB/cache authority, a stale migration
  context, or another domain's mapping unless the mapping was explicitly shared
  by capability.
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

The RTL track does not stop at S0/M1 or the M1-M15 smoke slices. Those
milestones only show that the interfaces and proof/simulation loop are viable.
The intended deliverable is a complete full-chip SystemVerilog implementation
of the architecture in `hardware_design.md`: core tiles, scheduler,
capabilities, Resource Domains, VMA/MMU, heap, futexes, gates, service
boundary, DMA, networking substrate, RAS/telemetry, storage/device frontends,
PCIe/IOMMU hooks, boot flow, and the shared fabrics that connect them. The full
design must remain simulatable even before it is mapped to a physical FPGA
board.

The canonical integration point for that deliverable is
`rtl/top/lnp64_top.sv`. Block-level RTL, smoke slices, and standalone harnesses
are useful only to the extent that they converge back into this top-level
machine. If a feature works only in an isolated testbench and cannot be driven
through `lnp64_top`, it is not integrated.

### Current RTL Reality Check

The current `rtl/` tree should be classified as **scaffolding plus bounded
smoke coverage**:

- S0 provides a broad top-level skeleton, reset/boot shape, shared records,
  stubs, and a small core/test path.
- `rtl/top/lnp64_top.sv` already names the intended whole-machine boundary, but
  it currently behaves as a smoke-oriented skeleton rather than a complete
  processor top.
- M1-M15 are narrow bounded slices that demonstrate specific ideas against
  small traces.
- `lnp64_engine_shells.sv` still contains shell/stub modules for major
  architectural engines and device frontends.
- `rtl/track_b_blocks_manifest.json` describes bounded smoke status, not
  implementation completeness.

This is acceptable as the bootstrap state. It is not acceptable as the final
RTL claim. New RTL work should preferentially replace these shells and bounded
slices with integrated engines rather than add more disconnected smoke demos.

### Real RTL Completion Criteria

A block may be marked implemented only when all of these are true:

- it participates in the top-level `rtl/top/lnp64_top.sv` reset, command, response,
  completion, event, fault, telemetry, and watchdog paths.
- it implements the architectural state machine, not a scripted trace or one
  hardcoded positive path.
- it handles normal success, canonical errors, malformed inputs, stale
  generations, revoked authority, cancellation, backpressure, timeout/fault
  paths, and reset/degraded recovery.
- it carries and checks PID/TID, Resource Domain id/generation, object
  id/generation, FDR generation, rights, lineage/revocation epoch, operation id,
  and completion target wherever the interface requires them.
- it uses real local state, memories, arbiters, queues, FIFOs, CAM/RAM tables,
  or fabric transactions appropriate to the block.
- if it owns shared architectural state, it defines whether the physical RTL is
  monolithic, banked, or sharded; the common case should prefer banking over a
  single global bottleneck when required for throughput, timing closure, or
  WCET.
- banked/sharded owners preserve single-writer semantics: every mutable record
  has exactly one owning shard at a time, and shard migration uses typed
  generation/epoch-protected transitions.
- it has bounded interaction with DDR-backed metadata where the architecture
  requires spill/refill, and it does not hide unbounded tree walks or software
  policy inside instruction retirement.
- it is exercised by top-level programs or architectural images, not only by an
  isolated block testbench.
- its assertions check local invariants and interface assumptions.
- its traces are typed architectural transition records or are on a planned path
  to typed records.
- its Lean/executable model has advanced beyond pure final-state witness form
  for security-critical properties.

Shells, stubs, fixed traces, and bounded smoke modules can remain in the tree as
bring-up aids, but they must be labeled as such and tracked as incomplete.

### Full Implementation Work Order

The next RTL phase should build the real machine in this order:

0. **Shared schemas and package hardening:** stabilize the packed records,
   enums, canonical errors, feature bits, latency classes, rights masks, and
   trace records used by `lnp64_top`, Lean, the emulator, and co-simulation.
1. **Executable top-level core path:** make `rtl/top/lnp64_top.sv` fetch,
   decode, execute, and retire real LNP64 instruction images. The minimum
   scalar subset is `LI`, `MOV`, integer ALU/immediate ops, extension/count/
   rotate/byte-swap ops, compare/branch/`CSEL`, load/store, `ENV_GET`,
   canonical unsupported-opcode failure, and `EXIT`. This path must run at
   least one assembler program and one LLVM-generated program in Verilator and
   compare against the emulator.
2. **Program-image and trace harness:** provide one boring script path that
   builds a program, creates the RTL ROM/SRAM image, loads the same image into
   the emulator, runs both, and compares committed retire records, final
   registers, final memory checksum, events/faults, errno/status, and exit
   code. Do not fork separate RTL-only demo programs unless they are explicitly
   temporary bring-up tests.
3. **Top-level transaction fabric:** replace ad hoc block wiring with shared
   command/response/completion/event/fault channels, routing, arbitration,
   operation ids, cancellation, and backpressure inside `rtl/top/lnp64_top.sv`
   and its immediate fabric modules.
4. **Core tiles and thread contexts:** expand the executable core path into
   replicated core tiles, result/error writeback, thread context windows,
   scheduler park/submit hooks, precise faults, atomics, and multicore retire
   records for real instruction streams.
5. **Capability/FDR, Resource Domain, and policy roots:** implement real FDR
   tables, generation and lineage checks, capability
   duplication/transfer/revocation, domain lifecycle, monotonic limits,
   accounting, freeze/resume/destroy, and policy enforcement.
6. **Scheduler and waitable core:** implement the Fixed Weighted-Fair
   Virtual-Deadline Active-Window Scheduler: fixed monotonic weight table,
   virtual runtime/deadline accounting, bounded active windows or
   virtual-deadline buckets, hierarchical Resource Domain quota/budget checks,
   sticky affinity, bounded migration, bounded wakeup insertion, bounded
   preemption points, wait queues, timers, futex wait/wake, event delivery,
   frozen/destroyed-domain rejection, spill/refill, no-lost-wakeup invariants,
   and no scheduler bytecode, callbacks, red-black tree policy, plugin dispatch,
   or unbounded tree walks.
7. **Object, gate, and process engines:** implement queue, counter,
   event/completion, memory object, call gate, continuation, fault delivery,
   cancellation, service-boundary object profiles, process/thread lifecycle,
   and exec-barrier behavior.
8. **VMA/MMU/page engine:** implement VMA tables, page-state transitions,
   permission checks, W^X/NX/guard, COW, page fill request/reply, TLB/I-cache
   invalidation, executable provenance, and deterministic race priority.
9. **Memory fabric and DDR metadata broker:** implement coherent memory access,
   metadata storage, ECC/parity hooks, spill/refill protocols, barriers, and
   cache/TLB/DMA visibility contracts.
10. **Heap and futex/atomic hard blocks:** implement the LNP64 Default Heap
   Algorithm: fixed size classes, per-thread allocation windows, local
   free/quarantine caches, bounded cross-thread free-transfer queues,
   domain-owned slab/run pages, VMA/Page-Engine large-object paths, protected
   generation-tagged metadata, exact-pointer `FREE`, invalid/double/foreign-free
   rejection, NX heap backing, bounded hot `ALLOC`/`FREE`, Class D
   refill/drain/large-allocation slow paths with inherited domain/deadline/
   cancellation metadata, locked atomics, futex buckets, and waiter spill/refill.
11. **Service boundary, typed control, and namespace dispatch:** implement
   typed control parsing, service request/reply continuation records,
   returned-capability validation, namespace dispatch stubs, and crash/cancel
   completion paths.
12. **DMA, storage, networking, and PCIe frontends:** implement DMA buffers and
    copy/fill/scatter-gather, storage/block/barrier objects, packet queues,
    endpoint queues, classifier/servicelet lanes, PCIe BAR/IOMMU/MSI event
    hooks, and driver-facing capability surfaces.
13. **RAS, assurance, and observability:** implement structured fault events,
    watchdog/local reset, counters, trace rings, measured boot records, quote
    FDRs, audit streams, debug/forensics control, MLS labels, and mission
    profile hooks.
14. **Full-system programs:** boot realistic LNP64 images through the RTL,
    exercise libc/personality paths, run userland tests in simulation, and
    retire the isolated smoke-only test posture.

Each step should leave the top-level simulator runnable. The broad skeleton
must stay connected while the internals become real.

### Integration Guardrails

The bundled goals are realistic only if the implementation keeps the machine
regular. The RTL must not grow into a set of mutually blocking owner engines
with ad hoc side channels. These guardrails are part of the work order:

- **No cyclic backpressure without an escape path.** Every ready/valid cycle
  must be broken by a bounded queue, credit return path, retry/error response,
  parkable waitable, watchdog/degraded path, or machine-fatal fault. A module
  must not hold a resource while waiting for another path that needs the held
  resource to make progress.
- **Separate terminal routes.** Command submission, completion, event, fault,
  cancellation, and telemetry routes may share physical fabric only if their
  arbitration proves that completion/fault/cancel traffic cannot be starved by
  new request traffic.
- **One owner per mutable record family.** Capability tables, domain records,
  scheduler slots, VMA entries, object state, gate continuations, heap metadata,
  DMA descriptors, and RAS records each have a named owner engine. Other blocks
  request changes through typed transactions; they do not mutate shared state
  behind the owner's back.
- **Bank shared owners, not ownership rules.** A shared owner engine may be
  physically banked or sharded by PID, domain, tile, object id, address range,
  queue id, FDR index, or hash bucket, but each record still has exactly one
  owning shard. Banking is a scaling and WCET tool, not permission to create
  competing writers.
- **Bounded local state first.** Hot paths use small local tables, windows,
  queues, CAMs, or FIFOs. DDR-backed metadata is accessed through explicit
  spill/refill transactions with bounded submit behavior, not hidden pointer
  chasing inside instruction retirement.
- **Realtime classes are implementation contracts.** A block may not advertise
  a Class A/B/C path until simulation, assertions, and the timing model show it
  retires, parks, or submits within the published bound. Long work is Class D:
  it must publish an operation id, waitable/completion target, timeout/watchdog
  class, and cancellation class before leaving the issuing instruction.
- **Realtime attribution is end-to-end.** Shared queues, fabrics, cache/memory
  partitions, async engines, DMA paths, and memory-controller queues preserve
  Resource Domain id/generation, submitter TID/generation, reservation/deadline
  metadata, operation id, cancellation epoch, and completion target. A realtime
  proof cannot stop at CPU dispatch if downstream shared resources may reorder
  or throttle the work as best-effort traffic.
- **Global time is a proof assumption and interface.** Deadlines, timeout expiry,
  reservation periods, watchdog windows, and Class D async deadlines use the
  synchronized hardware timebase. Any skew, pause, reset, or clock-domain
  assumption must be named in `ENV_GET` and the trusted-platform contract.
- **Best-effort traffic is never proof-critical.** Realtime and assurance
  proofs are stated for admitted domains and published reservations. Best-effort
  work may be throttled, failed with pressure events, or delayed within its own
  profile, but it must not consume the completion/fault/cancel capacity needed
  by admitted work.
- **Proof state is projected.** Lean models should prove architectural
  projections and transition invariants, not every pipeline flop. RTL traces
  expose commit records, ownership changes, terminal paths, and typed events so
  the proof model stays small enough to finish.
- **Do not prove beyond the contract.** Scheduler proofs target uniqueness,
  no-lost-wakeup, bounded fairness for eligible admitted work, deadline honesty,
  and fixed active-window behavior. Heap proofs target metadata integrity,
  exact-pointer free, invalid/double/foreign-free rejection, domain accounting,
  NX backing, and bounded hot paths. Ordinary C `LD`/`ST` remain VMA/page
  granularity checks unless a future ABI-compatible profile explicitly adds
  per-object memory-safety machinery.
- **Composition is staged.** A block is first proved locally, then with its
  immediate fabric, then through `lnp64_top`. No global claim is made until the
  block's assumptions are either discharged by neighboring assertions or listed
  in the trusted-platform contract.
- **Watchdogs are recovery evidence, not flow control.** A normal progress
  proof cannot rely on watchdog timeout. Watchdogs prove bounded failure
  containment when an engine violates its normal ready/progress contract.

Before any milestone is called complete, its test plan should include a small
deadlock audit: list every queue/fabric dependency, identify the owner of each
mutable state family, name the terminal path for every accepted command, and
show which traffic class can still complete under backpressure.

### Per-Engine Progress Contract

Every RTL block that accepts commands, owns mutable architectural state, parks
threads, arbitrates traffic, or adapts external IP must publish the same small
progress contract. This is the local contract used to prove `SG-TOTAL`,
`SG-WAKE`, `SG-PROGRESS`, and `SG-RT` compositionally:

- **Accepted command ownership:** an accepted command receives an operation id or
  is proven single-cycle/local. The command is owned by exactly one engine or
  shard until it reaches a terminal state.
- **Terminal completeness:** every accepted command has exactly one terminal
  outcome: success response, canonical error, completion event, cancellation,
  timeout, structured fault, permitted degraded state, local reset, or
  measured/audited machine-fatal escalation.
- **Bounded local progress:** from every reachable nonterminal state, the engine
  advances within a published bound when downstream assumptions hold, or moves
  to a named backpressure, park, retry, fail-closed, reset, or escalation state.
- **Backpressure release:** every asserted `ready == 0`, full queue, held credit,
  retry token, or parked state names the event, credit, drain, timeout, cancel,
  reset, or fault condition that releases it. No proof may rely on an unbounded
  spin loop.
- **No impossible waits:** an engine may not wait for a response on a path that
  cannot be driven while the engine holds the resource needed to produce that
  response.
- **Reset and poison behavior:** reset, local reset, ECC/parity poison, malformed
  input, and invalid encoded state have defined fail-closed behavior and cannot
  publish a partial commit.
- **Bypass exclusion:** any state update that affects authority, scheduler
  location, memory permission, domain accounting, wake state, or evidence records
  goes through the named owner transition path or a named shard transition.
- **Assumption boundary:** any reliance on clocks, CDC, DDR, PCIe, PHYs, SRAM
  macros, fairness, bounded response time, or arbitration service is named as an
  assumption and later discharged by a neighboring block, assertion, or
  trusted-platform contract.

A block without this contract can still be a bring-up stub, but it cannot support
a severe proof claim.

### Milestone Constraint Checklist

Each milestone should also run through a short constraint checklist. This keeps
the project from satisfying the visible feature requirement while missing a
hardware condition that later breaks proofs, realtime behavior, or synthesis.

- **Reset and initialization:** every state element has a defined reset,
  initialization, poison, or scrub state. No engine can accept commands before
  its owner records, feature bits, and terminal routes are initialized.
- **Clock and reset domains:** every crossing is explicit. CDC, reset release,
  PLL/clock-good, external PHY, DDR, PCIe, and debug clock assumptions are named
  in the trusted-platform contract.
- **Bounded identifiers:** operation ids, generations, epochs, sequence
  numbers, counters, and timestamps define wrap behavior, stale rejection, and
  comparison rules before they are used in proofs.
- **Resource exhaustion:** every table, FIFO, queue, ring, CAM, spill window,
  continuation slot, event slot, and trace/audit buffer defines full behavior
  and ownership of pressure events.
- **Replay and duplication:** completions, faults, cancels, wakeups, DMA
  completions, gate returns, and capability transfers are idempotent or
  generation-checked so a duplicate record cannot create authority or split
  scheduler state.
- **Livelock and retry storms:** retry paths are bounded by credits, retry
  tokens, backoff counters, parking, pressure events, or fault escalation.
  Proofs must not rely on a requester spinning forever.
- **Ordering and visibility:** memory, cache, TLB, DMA, engine completion,
  device, and service-return ordering points are explicit in both RTL and the
  abstract model.
- **Timing closure:** any claimed Class A/B/C path has a plausible critical
  path, fanout, SRAM/CAM count, and arbitration depth for the target frequency.
  If not, it becomes Class D bounded-submit work.
- **Area and port pressure:** multiported memories, CAMs, crossbars, and global
  broadcast signals are treated as risks. The roadmap should prefer sharding,
  banking, local queues, and bounded migration over global all-to-all paths.
- **Side channels and observability:** counters, trace rings, timing behavior,
  queue pressure, classifier marks, and telemetry are scoped by Resource Domain
  policy. Debug and forensics paths are capability-gated and measured.
- **Malformed inputs:** every externally influenced record, packet, descriptor,
  servicelet, selector, and control envelope has bounded parse depth,
  canonical errors, and no hidden allocation on malformed input.
- **Vendor and external IP:** DDR, PCIe, Ethernet PHY/MAC, clocking, and FPGA
  primitives enter proofs only through explicit assume-guarantee contracts.
- **Synthesis/simulation parity:** simulation-only hooks are isolated in
  testbenches, binds, or named adapters. The synthesizable path through
  `lnp64_top` must not depend on testbench behavior.

### First-Pass Engineering Artifacts

Before expanding the RTL beyond the broad skeleton, create five small planning
artifacts. They are not bureaucracy; they keep the multicore, realtime, and
proof goals from diverging.

1. **Backpressure diagrams.** For each fabric slice, draw the command,
   response, completion, event, fault, cancel, telemetry, and refill paths. Each
   diagram must list queue depths, ready/valid dependencies, traffic classes,
   resources held while waiting, and the escape path for full/backpressured
   conditions: complete, retry, park, `EAGAIN`, `EOVERFLOW`, cancel, fault,
   degraded, or machine-fatal. Any cycle without a terminal escape path blocks
   milestone completion.
2. **Bank mapping table.** For each shared owner engine, record whether it is
   monolithic, banked, or sharded, and name the shard key. First-pass defaults:
   FDR by `(pid, fdr_index)`, VMA/page by `(pid, virtual_address_range)`, futex
   by hash bucket, event queue by queue id, object/queue by object id, heap by
   `(pid, arena, size_class)`, scheduler by `(tile, domain)`, DMA by descriptor
   queue id, and RAS/trace by source class plus domain. The table must also name
   the owner migration/rebalancing rule and generation/epoch check.
3. **Schema source of truth.** Create one checked schema source for packed RTL
   records, Lean structures, Rust/emulator constants, Python/co-sim records,
   and Markdown tables. It should cover opcodes, profiles, status/error codes,
   feature bits, rights masks, latency classes, lifecycle profiles, command and
   response records, events, faults, capabilities, domains, VMA/page, scheduler,
   object, DMA, service, telemetry, and trace records. The initial source can
   be a checked manifest; the target is generated or mechanically validated
   consumers.
4. **Reset and lifecycle matrix.** Do not give every module a large
   busy/fault/degraded/recovery FSM. Assign each module a lifecycle profile:
   pure combinational, local pipeline, queue/FIFO, owner engine,
   long-latency owner engine, or external-IP adapter. The profile defines which
   states are legal. Small blocks should make invalid states unrepresentable
   where possible; owner engines get typed abort/poison/degraded paths only when
   they own persistent or externally affected state.
5. **External IP contracts.** For every DDR, PCIe, Ethernet PHY/MAC, SERDES,
   clock/reset, FPGA RAM, and vendor primitive, write an assume-guarantee
   contract. The contract must state reset behavior, clock/CDC assumptions,
   ordering, integrity/ECC behavior, error signaling, maximum wait or
   unbounded/Class-D status, DMA/coherence obligations, and what hardware does
   when the IP violates or cannot meet the contract.

These artifacts should be versioned and referenced by the top-level evidence
index. A theorem may rely on them only by naming the specific assumption or
contract it uses.

### Reset, Fault, and Proof Shape

Faults do not replace proofs of correctness. The proof strategy has two layers:

- **Normal-operation proofs:** for modules whose inputs satisfy the contract and
  whose owned metadata is valid, transitions preserve authority, memory,
  scheduler, realtime, and object invariants. This is the primary correctness
  proof.
- **Fault-containment proofs:** for malformed input, parity/ECC poison,
  watchdog timeout, external-IP error, overflow, stale generation, or impossible
  state detection, the module follows a typed fail-closed transition that does
  not mint authority, skip generation checks, publish partial commits, lose a
  scheduler context, or reuse poisoned metadata as valid.

A module should not have `busy`, `fault`, `degraded`, and `recovering` states
unless its lifecycle profile requires them. Pure datapath and decode blocks
should be total or fail with a canonical result. Small queues should have
explicit empty/full/poison behavior. Owner engines with commit points may have
`idle`, `prepare`, `commit`, `complete`, and `abort` phases. External-IP
adapters may have `link_down`, `training`, `ready`, `error`, and `degraded`
states. The design goal is still to make invalid states unrepresentable; when
that is not practical in RTL, invalid encodings assert, emit a structured
fault, and enter a fail-closed terminal state.

### B0. Whole-Machine Skeleton

`LNP64-RTL-S0` is a synthesizable top-level machine skeleton. Most modules may
be stubs, but the top-level shape must be representative of the real
architecture.

The S0 top-level module is `rtl/top/lnp64_top.sv`, and that file remains the
architectural chip top after S0. S0 may include simulation-visible status
signals, but completion work must progressively replace those smoke hooks with
real architectural ports, internal typed traces, and testbench bindings. The
final `lnp64_top` should be suitable as the root module for full-chip
simulation, synthesis, and later FPGA integration.

Required top-level modules:

- reset/boot/manifest shell.
- clock/reset domain shell.
- replicated core tile shells.
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
- scheduler affinity/current-tile/preferred-tile/migration-generation record.
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
- every module declares its lifecycle profile and exposes only the legal
  reset/idle/active/fault/degraded/stub states for that profile.
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

### B2. Minimal Core Tiles

Implement:

- two replicated core tiles in the default simulation build.
- parameterized tile count with a four-tile stress configuration.
- fetch/decode/execute.
- GPR file.
- simple branch.
- load/store to simulated SRAM.
- hardware thread context switching.
- retire, park, and submit records.
- tile-id tagging on retire, fault, event, scheduler, and trace records.
- a coherence-shell/inclusive-L2 interface; no full cache initially.

Runs:

- tiny assembly programs in simulation.
- bounded retire/park/submit timing checks.
- tile-0 and tile-1 independent execution tests.
- cross-tile wake/scheduler handoff smoke.

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
- run the smallest existing assembly or compiler-generated program that
  exercises the block through `rtl/top/lnp64_top.sv`.
- compare architectural state, result codes, event records, and FDR generations.
- prefer typed transition-record comparison over free-form string trace
  comparison as soon as the shared schemas exist.
- generate random but bounded traces from the formal model where practical.
- use Verilator for fast CI.
- later add FPGA simulation and synthesis checks.

The emulator remains the executable architectural oracle until the formal model
is strong enough to generate authoritative traces directly.

### Program Corpus Simulation

The RTL simulator should run real project programs early and continuously. This
is not a substitute for proofs, but it is the fastest signal that the broad
architecture is becoming executable rather than only locally correct.

Maintain a small corpus of existing assembly demos and compiler-generated
programs, tagged by the architectural features they require. Examples include:

- tiny core/branch/load-store smoke programs.
- `ENV_GET`, canonical-error, and feature-discovery programs.
- capability/FDR generation and revocation demos.
- queue, `PUSH`/`PULL`, `AWAIT`, and ping-pong demos.
- Resource Domain lifecycle and budget demos.
- gate/call/continuation demos.
- VMA, heap, futex, poll/event, storage, networking, and servicelet demos as
  those RTL blocks become real.

For each corpus entry, keep one source path, one emulator expectation, and one
RTL simulation gate. The same program should run in the emulator and in
Verilator, with comparison at committed instruction, event, completion, fault,
and telemetry boundaries. Early gates may compare a short textual trace. The
target is a typed transition trace generated from the shared schema.

Do not wait for the full ISA before using this corpus. As soon as a milestone
implements the features needed by an existing demo, add that demo to the
top-level RTL gate. If the demo needs unavailable features, keep it marked
`blocked-by:<feature>` rather than copying it into a synthetic RTL-only test.
This keeps progress visible and prevents the simulator from drifting away from
the assembler, compiler, emulator, and libc/personality work.

Full-chip simulation is a required deliverable, not a convenience. The design
should keep a top-level Verilator path alive throughout development. Early
builds may use stubbed engines, but the active work is to replace them with
filled blocks. A block is not considered integrated until it participates in
top-level reset, command/response routing, event/fault delivery, telemetry, and
at least one model/emulator/RTL trace comparison.
For high-value guarantees, integration is not complete until the block has a
documented refinement relation to the Lean transition model and its assumptions
are either discharged by assertions/checks or listed in the trusted-platform
contract.

## Track D: FPGA Bring-Up

Real FPGA hardware is not yet available. Track D is therefore a deferred
hardware-port track, not the current primary target. Until a target board is
chosen, the project should optimize for portable synthesizable SystemVerilog,
clean simulation, clear clock/reset boundaries, and vendor-neutral constraints
where possible.

When hardware is available, bring-up should start with the broad skeleton, then
fill the smallest useful vertical slice:

1. top-level skeleton modules connected with stub responses.
2. fixed decode table, canonical errors, and `ENV_GET`.
3. soft SRAM only, no DDR.
4. UART output.
5. two core tiles, tile telemetry, and simple assembler program ROM.
6. cross-tile scheduler handoff and synthetic wake.
7. coherence-shell or inclusive-L2 invalidate/ack/writeback smoke.
8. FDR/capability table and generation checks.
9. scheduler, waitable, event router, and object queue smoke.
10. gate/continuation and process lifecycle smoke.
11. tiny VMA/MMU, TLB invalidation, and memory-protection smoke.
12. external DDR and shared metadata broker.
13. DMA/memory-object smoke.
14. typed control, namespace/service dispatch, and capability-return smoke.
15. futex/atomic and heap smoke.
16. SD/SPI and storage-barrier smoke.
17. Ethernet packet queue, classifier, and servicelet smoke.
18. RAS/telemetry/watchdog/attestation smoke.
19. PCIe later.

## First Milestone: Whole-Machine Skeleton

`LNP64-RTL-S0` should demonstrate that the overall architecture shape is correct
before any single block becomes sophisticated. It is not enough for S0 to print
status from a scripted testbench; S0 must establish the loader, reset, and
trace path that S1 will use for real programs.

Required slice:

- synthesizable top-level machine.
- default two-core-tile simulation topology, parameterized for a four-tile
  stress build.
- all major module shells present.
- command, response, event, fault, completion, capability, domain, scheduler,
  policy, VMA, DMA, service, RAS, and telemetry records defined.
- reset/boot path creates root domain, PID 1, scheduler state, and initial FDRs
  or emits a measured/audited boot fault.
- stubs fail closed with canonical errors.
- `ENV_GET` exposes feature bits, limits, topology stubs, and latency-class
  stubs.
- tile id is carried in retire, scheduler, event, fault, trace, and coherence
  records where relevant.
- event router and completion writeback paths exist.
- coherence/TLB/DMA visibility stubs exist before memory operations complete.
- no raw physical interrupts, raw DMA, raw physical addresses, or ambient device
  authority are software-visible.
- UART boot/status output works in simulation.
- Verilator simulation runs without hanging on stubbed operations.
- one ROM/SRAM instruction-image path exists and is owned by `lnp64_top`, not
  by a testbench-only behavioral model.

Proof targets:

- global state well-formedness over the broad skeleton state.
- no authority from stubs.
- no accepted stub command without a defined terminal path.
- exactly-one scheduler state/location.
- every runnable/running thread is assigned to at most one tile.
- a tile-local fault/degraded state cannot corrupt another tile's scheduler
  state.
- no software-visible raw interrupt/physical-address path.
- reset produces valid initial state or measured/audited boot fault.

Expected demo:

- reset the skeleton.
- run a tiny PID 1 ROM/SRAM program.
- observe two enabled tiles, with tile 0 running PID 1 and tile 1 idle or
  scheduler-ready.
- run the first corpus entry through the same top-level ROM/SRAM loader path.
- print status over UART.
- query `ENV_GET` feature bits.
- issue one unsupported native command and observe canonical failure.
- inject one stub-engine fault and observe a structured fault event.

## Second Milestone: Executable Compiler Path

`LNP64-RTL-S1` should make the design feel like a processor, not a collection
of proof slices. The machine should run small programs produced by the project
assembler and LLVM path through `rtl/top/lnp64_top.sv` under simulation.

Required slice:

- top-level instruction fetch from ROM/SRAM image.
- fixed decode for the scalar compiler baseline: constants, register moves,
  ALU/immediate ops, extension/truncation, compare/branch/`CSEL`, load/store,
  `ENV_GET`, unsupported-opcode failure, and `EXIT`.
- register file and PC state held in hardware thread context state.
- simple single-thread retirement, with a clean path to two-core/multithread
  replication rather than a throwaway single-core harness.
- SRAM-backed data memory with deterministic load/store semantics.
- typed retire trace records including pc, opcode, operands needed for replay,
  result register/value, errno/status, event/fault id, PID/TID, domain id/gen,
  and tile id.
- one script that builds a program, creates the RTL image, runs RTL simulation,
  runs the emulator on the same program, and compares final state plus trace
  prefixes.

Expected demos:

- tiny assembler arithmetic/branch/load-store program.
- tiny LLVM-generated integer program that avoids libc at first.
- `ENV_GET` feature/limit query.
- unsupported-opcode canonical failure.
- a final-state memory checksum test.

Acceptance bar:

- the same program source or image is used for emulator and RTL.
- no hand-authored RTL-only success trace is accepted as program execution.
- differences are reported as architectural mismatches: pc, register, memory,
  status/errno, event/fault, or trace divergence.
- S1 remains synthesizable and keeps `lnp64_top` as the root module.

Proof targets:

- decode is deterministic for the implemented scalar subset.
- every retired instruction has one architectural retire record.
- unsupported opcodes fail closed without changing unrelated architectural
  state.
- the single-thread register/PC/memory projection matches the emulator for the
  accepted test subset.

## Third Milestone: M1 Authority Refinement Template

`LNP64-RTL-M1` should demonstrate that the architecture is real enough to
execute code outside the Rust emulator and that one security-critical block can
be carried from Lean transition proof to typed RTL commit evidence. This
milestone is the template for later proof slices, not just a ping-pong demo.

Required slice:

- B1 through B5 implemented enough for the test.
- two enabled core tiles in the default simulation build.
- two hardware thread contexts.
- small FDR table with generation checks.
- queue object.
- `PUSH`, `PULL`, and `AWAIT`.
- scheduler ready/wait transitions.
- event wake or gate wake.
- cross-tile wake or scheduler handoff.
- schema-owned typed capability/FDR commit records.
- a documented RTL-state-projection to Lean-state relation for the M1 slice.
- a documented commit/pre-state/post-state relation showing which Lean `Step`
  each RTL M1 commit corresponds to.
- M1 operations are reachable from real instructions retired by `lnp64_top`, or
  the milestone explicitly states which S1 instruction/fabric hook is still
  missing.
- packed RTL commit/projection bit decoding checked against the schema before
  JSON trace fields are trusted.
- bypass/mediation assertions for capability/FDR authority writes.
- Verilator simulation.
- co-simulation against the emulator.

Proof targets:

- no forged FDR.
- no authority amplification.
- current-authority checks before transfer, receive, push, pull, revoke, and
  object creation.
- stale generation rejection.
- revoked authority cannot start new work.
- failed authority operations preserve authority slots and fail closed.
- M1 typed commit transitions preserve the Lean invariant.
- no RTL bypass path writes capability/FDR authority outside the owner
  transition path.
- no lost wakeup.
- exactly-one scheduler state/location.
- no runnable thread can execute simultaneously on two tiles.
- cross-tile wake/event delivery cannot lose or duplicate a wakeup.
- queue full behavior is explicit.

Expected demo:

- a tiny assembly ping-pong program runs in RTL simulation.
- one ping-pong variant runs across two tiles.
- the same program runs in the emulator.
- architectural traces match at committed instruction/event boundaries.
- typed M1 commit records match the executable model for representative and
  randomized traces.
- the ping-pong program is promoted into the recurring program corpus gate.

This milestone is intentionally small but deep. It is not finished until the
remaining gap to checked RTL-to-Lean refinement is explicit and the evidence row
says whether M1 is only typed-trace checked, assertion-coupled,
transition-proven, refinement-coupled, or top-level composed. Passing typed
trace checks and assertions alone is not T4. Once the refinement shape works,
the project has a reusable proof/RTL/emulator loop and can grow block by block
without betting everything on a whole-chip rewrite.
