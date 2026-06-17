# LNP64 FPGA Hardware Design Sketch

This document sketches a first real FPGA implementation of LNP64. It is not RTL
and it is not a module skeleton. It is an architectural design target for a large
FPGA with no built-in CPU cores. The central goal is to realize the POSIX-like
compatibility surface as native capability, event, VMA, scheduler, and Resource
Domain datapaths, not as software traps and not as a hidden microcoded
processor.

## 1. Implementation Target

Target class:

- Large standalone FPGA.
- External DDR memory.
- UART.
- SD card.
- SPI flash.
- Simplified Ethernet MAC.
- No ARM/RISC-V/MicroBlaze/Nios management CPU.

The design may use FPGA block RAM, ultra RAM, DSP blocks, PLLs, SERDES, and hard
DDR PHY/controller IP where the FPGA vendor requires it. Those are treated as
fabric resources and peripheral controllers, not as instruction execution
engines.

## 2. Design Goals

LNP64 v1 must support:

- General integer execution: registers, ALU, branches, calls, loads, stores.
- File descriptor registers as real hardware capability handles. POSIX file
  descriptors are the libc/personality profile over these handles.
- Native capability/object operations for streams, directories, waitables,
  memory mappings, domains, call gates, DMA buffers, and device views from day
  one. POSIX file, process, signal, futex, mmap, exec, fork, poll, and socket
  APIs lower cleanly to these primitives.
- True multi-context hardware scheduling with a real hardware runqueue.
- Coherent multicore execution across multiple fabric CPU tiles.
- External DDR virtual memory with hardware-managed translation and VMAs.
- Hardware-backed UART, SD, SPI flash, and simplified Ethernet file objects.
- PCIe host support through a hardware Root Complex, IOMMU, MSI routing, and a
  privileged software Bus Master domain that requests derived FDR capabilities
  from PCIe root/function authority.
- Deeply nestable Resource Domains for virtualization, containers, cgroups,
  jails, sandboxes, and supervisor upcalls, without adding traditional hosted-OS
  rings or syscall traps.
- Native security invariants: W^X, NX data defaults, ASLR, guard pages,
  hardware entropy, generation-checked objects, revocation, sealed/narrowed
  capabilities, tenant-strict domains, confidential-computing hooks, and DMA
  isolation as Resource Domain and capability policy.
- V1 operability hooks: critical metadata ECC/parity, structured fault events,
  per-engine watchdog/reset paths, observability counters, a small trace ring,
  remote-attestation records, checkpoint/live-migration compatibility hooks,
  line-rate record classification and queue steering, and explicit storage
  flush/barrier semantics.
- Deterministic instruction decode with a fixed binary encoding.
- Hardware-owned waitable/capability objects with local state, bounded
  transitions, and event delivery, usable by ordinary runtimes as well as
  POSIX-compatible OS operations.
- Hardware modules designed as small, explicit, enumerated-state machines where
  invalid states are unrepresentable or detected by construction.

The v1 design is allowed to be slow for complex compatibility operations.
For example, `EXEC` can take thousands or millions of cycles while the DMA
fabric copies ranges named by a prepared exec-plan descriptor. The important
requirement is that hardware commits a bounded architectural transition while
the issuing thread is parked and other threads continue. Hardware does not parse
ELF, dynamic-linker state, shebangs, relocation formats, library graphs, or Unix
credential transition policy.

A major design goal of the hardware resource modules is robustness, not just
speed. Compared with software subsystems, a good hard block should have a
smaller reachable bad-state space: explicit states, bounded transitions,
protected metadata, generation/check bits, and commit/abort paths that prevent
partial architectural publication.

## 3. Non-Goals

LNP64 v1 does not attempt:

- Out-of-order execution.
- Speculative branch prediction.
- Full Linux ABI compatibility directly in hardware; Linux compatibility is
  provided by targeted runtimes or paravirtual personalities.
- Full POSIX edge-case compatibility directly in hardware.
- A fully general PCIe device ecosystem with arbitrary hotplug and every
  vendor-specific quirk solved in hardware.
- Loadable microcode in the first FPGA implementation.

`LOAD_UCODE` is decoded in v1 as a reserved device-driver hook, but the FPGA v1
behavior is a stub. It must not install arbitrary executable control logic until
a separate driver safety model is specified.

## 4. Top-Level Hardware Blocks

The chip is organized as these blocks:

- Multiple LNP64 core tiles.
- Per-core Instruction Fetch Unit.
- Per-core Decode and Issue Unit.
- Per-core Integer Execute Unit.
- Per-core Load/Store Unit.
- Per-core FPU and Vector Unit.
- Per-core L1 instruction and data caches.
- Shared coherent interconnect.
- Inclusive shared L2 cache and MSI coherence controller.
- Thread Context Store.
- Hardware Scheduler and Runqueue.
- MMU, TLB, VMA Engine, and Page Allocator.
- Hardware Heap Engine.
- Capability File Descriptor Table.
- Namespace Dispatch and Capability Return Engine.
- File Operation Engine.
- Directory stream datapath inside the Object/File Operation Engine.
- Process Engine.
- Signal Engine.
- Futex and Atomic Engine.
- Resource Domain Engine.
- Supervisor Domain and Upcall Engine.
- Entropy and Randomization Engine.
- Measurement and Attestation Engine.
- Fault, Telemetry, and Trace Engine.
- PCIe Root Complex.
- PCIe IOMMU / DMA Remapper.
- PCIe MSI/MSI-X Event Router.
- DMA Fabric.
- Record Classification and Queue Steering Engine.
- Device adapters for UART, SD card, SPI flash, and Ethernet.
- DDR Memory Controller Interface.
- Interrupt and Event Router.

All long-latency resource instructions issue a command into a hardware engine and
park the issuing thread. Completion events write architectural results, update
`ERRNO`, and return the thread to the ready queue.

### 4.1 Module Interconnect

The v1 fabric is a set of fixed-function engines connected by simple
synchronous hardware channels. Modules may run independently and complete out of
order, but internally they are bounded FSMs or pipelines, not hidden CPUs,
interpreters, or firmware loops.

The design uses three planes:

- Control plane: narrow ready/valid command and response channels between
  decode/issue, engine command queues, shared metadata engines, and completion
  writeback.
- Data plane: wider cache, DDR, DMA, device FIFO, packet-buffer, and block-buffer
  paths. Bulk payloads do not travel over the control plane.
- Wakeup plane: parallel event wires plus compact event records into the Event
  Router, Signal Engine, and Scheduler.

The design is intentionally not "every module can read DDR." Raw memory
requesters are limited to the core LSU/cache hierarchy, DMA Fabric, VMA/Page
Walker, Page Allocator, Metadata Table Walker/Broker, and DDR controller.
Other engines issue semantic requests to the owner of the needed state. A module
that mostly sequences DDR reads and writes is not a successful hardware
implementation; it should be collapsed into the owning metadata engine or moved
back to software.

Where practical, hot state lives in small registers, FPGA RAMs, or tiny
set-associative caches:

- active thread context windows.
- low FDR bank and recently used dynamic FDR entries.
- current process credential snapshot.
- VMA root and recent translation metadata.
- event queue heads/tails and active wait slots.
- heap size-class windows and per-thread free/quarantine caches.
- futex hot buckets and waiter heads.
- cwd/root object ids and open-object metadata.

DDR is the architectural backing store and spill area, not the first stop for
common operations. The fast path of a hardware module must either complete from
local state, emit one semantic request to an owner engine, or issue a DMA
descriptor for bulk movement.

The internal FPGA design should be synchronous, not clockless asynchronous.
Ready/valid handshakes give the desired decoupling while keeping timing closure
tractable. External device clock domains, such as Ethernet PHY, SD card, UART,
SPI flash, and PCIe, cross into the core fabric through small async FIFOs or
standard CDC synchronizers.

Top-level connection sketch:

```text
Core Tile Decode/Issue
  |  engine_cmd
  v
Engine Command Router
  |--> VMA Engine <-----------> Page Allocator
  |      |                         |
  |      | tlb_inv/cache_inv       | page_alloc/free
  |      v                         v
  |    TLBs / I-cache        DDR metadata tables
  |
  |--> FDR Table Cache <-----> Capability/FDR DDR tables
  |      |
  |      +--> Namespace Dispatch Engine <-> namespace/service capability tables
  |      |       |
  |      |       +--> File Operation Engine
  |      |               |
  |      |               +--> DMA Fabric <--> L2/DDR Controller
  |      |               +--> UART / SD / SPI / Ethernet adapters
  |      |               +--> PCIe IOMMU / Root Complex
  |      |
  |      +--> Event Queue Builder
  |
  |--> Process Engine <------> Thread/Process Context Store
  |      |
  |      +--> Scheduler / Runqueue
  |
  |--> Heap Engine <---------> Heap metadata / Page Allocator
  |
  |--> Futex/Atomic Engine <-> L2 atomic port / futex waiter tables
  |
  |--> Signal Engine <-------> saved signal contexts
  |
  |--> Supervisor Upcall Engine <-> supervisor control FDRs
  |
  v
Completion Router --> Register Writeback / ERRNO / Event Router / Scheduler

Device IRQ/MSI/timer/fault lines --> Event Router --> Scheduler / Signal Engine
```

Canonical command channel:

```text
cmd_valid
cmd_ready
cmd_opcode
cmd_variant
cmd_pid
cmd_tid
cmd_op_id
cmd_result_reg
cmd_errno_policy
cmd_cancel_policy
cmd_arg0
cmd_arg1
cmd_arg2
cmd_arg3
cmd_arg_block_ptr
cmd_arg_block_len
cmd_credential_snapshot
```

Canonical completion channel:

```text
rsp_valid
rsp_ready
rsp_op_id
rsp_pid
rsp_tid
rsp_result_reg
rsp_result_value
rsp_errno
rsp_status
rsp_event_mask
```

Small engines may use a strict subset of these fields. For example, `YIELD`
needs only PID/TID and scheduler control; `ALLOC` needs PID/TID, size, result
register, and heap flags; `PULL` needs an FDR capability reference, buffer
address, length, operation id, and DMA completion target.

Wakeup/event wires:

- `thread_ready[tid_window]`: scheduler-visible ready hints for active on-chip
  contexts.
- `engine_done`: completion router has a result for a parked TID.
- `dma_done` / `dma_fault`: DMA descriptor completed or faulted.
- `timer_expired`: timer wheel produced a wakeable event.
- `irq_pending`: device adapter, PCIe MSI/MSI-X, or internal IRQ event.
- `signal_pending`: Signal Engine has an unmasked signal for a TID.
- `tlb_inv_req` / `tlb_inv_ack`: VMA Engine invalidation broadcast and tile
  acknowledgement.
- `icache_inv_req` / `icache_inv_ack`: executable mapping invalidation.
- `fatal_fault`: decode, execute, LSU, or bus fault must enter Signal Engine.
- `ras_fault`: ECC/parity, metadata-corruption, watchdog, local-reset, storage,
  or DMA isolation fault must enter the Fault, Telemetry, and Trace Engine.
- `trace_event`: compact event record is available for the optional trace ring.

Large or sparse events are carried as DDR-backed event records. The wire only
announces that an event record exists and identifies the active-window slot or
queue id.

Shared table access:

- FDR, process, thread, VMA, heap, futex, event-queue, namespace-dispatch, and
  object metadata live in DDR-backed tables with small on-chip caches.
- Critical authority-bearing metadata is parity- or ECC-protected according to
  storage width and target FPGA support. At minimum this includes FDR entries,
  VMA descriptors, domain descriptors, scheduler/runqueue entries, event queue
  heads/tails, heap metadata, DMA descriptors, and namespace/object metadata.
- Each table has one owning engine that arbitrates mutation. Other engines
  access it through request channels or read-only cached snapshots.
- Non-owner engines must not independently walk or mutate another engine's DDR
  tables. They request `validate_fd`, `pin_user_buffer`,
  `dispatch_namespace_request`, `allocate_pages`, `enqueue_event`, or similar
  semantic operations.
- Object locks are fixed hardware locks or scoreboard bits on table entries, not
  software mutexes. Locks must have bounded acquisition, timeout/cancel behavior,
  and deadlock ordering documented per engine.
- A command that needs multiple objects acquires them in this global order:
  process/thread, FDR, VMA, heap, object/namespace, device/DMA. If an engine cannot
  acquire the next object without violating the order, it releases and retries or
  parks behind the owning engine.

Serial submodules are allowed only at physical edges or naturally serial
protocols: UART shifters, SPI flash, SD card command/response, MDIO-like PHY
control, PCIe configuration accesses initiated by the Bus Master, and similar
adapters. Their boundary to the LNP64 fabric is still a command FIFO, data FIFO,
and event line.

Design rule: no module may hide a general instruction sequencer. A module can
walk bounded tables, arbitrate ports, issue DMA descriptors, enqueue event
records, and park/wake threads. Device-specific complexity that wants software
belongs in a delegated process such as the PCIe Bus Master or a driver domain,
behind FDR capabilities.

Additional design rule: no module should be accepted merely because it replaces
software with RTL. It must reduce memory traffic, reduce serialization, preserve
atomic capability/scheduling semantics, or improve streaming throughput. If the
module's steady-state behavior is just "submit memory request, inspect a word,
submit another memory request," it is an antipattern for this architecture.

### 4.2 Local-State Hard Block Review

The ambitious POSIX modules are retained in v1, but only as local-state hard
blocks with explicit fast paths. Their purpose is not to walk DDR on behalf of
software; their purpose is to terminate common semantic operations in
registers, FPGA RAM, small caches, scoreboards, or one bounded request to an
owner engine.

They also earn their silicon by being harder to corrupt than equivalent
software. Each hard block should have a small enumerated state model, explicit
transaction phases, invalid-state detection, and a bounded recovery path.

Module expectations:

- FDR/Capability Engine: cached descriptor validation and capability rights
  checks for hot descriptors. Common `PULL`, `PUSH`, `AWAIT`, `MMAP`, and
  `CLOSE` should not fetch the descriptor from DDR.
- Namespace Dispatch/Object Engine: hot namespace-root/service metadata,
  open-object metadata, object lock scoreboard, and capability return/install
  FSMs. It does not parse directory formats or implement general filesystem
  policy. Cold namespace lookup/control requests are dispatched to service
  domains.
- File Operation Engine: stream transaction compiler. Given a validated FDR and
  pinned buffer, it updates stream state, emits DMA/FIFO/packet descriptors, and
  posts completion. It does not independently walk process, FDR, VMA,
  namespace, or object tables.
- Directory datapath: subtype lane for directory streams, not a separate DDR
  walker. It handles dirent packing, directory cookies, end-of-directory, and
  stable iteration rules over cached directory pages.
- VMA/Page Engine: TLB miss handling, cached recent VMA ranges, COW/page-fault
  classification, buffer pinning, and invalidation broadcast. Tree/array walks
  are cold/refill paths.
- Heap Engine: per-thread allocation windows, free/quarantine caches, and
  small-allocation size-class state in local memory. Common `ALLOC`/`FREE` must
  not touch DDR metadata.
- Process/Scheduler Engine: active PID/TID windows, run queues, exec barriers,
  child-exit state, and parked-thread state in local memory. DDR is spill for
  oversubscription.
- Futex/Event/Signal Engine: hot futex buckets, event queue heads/tails,
  pending-signal bits, timer wheel entries, and active wait slots on chip. DDR
  carries overflow records and cold queues.
- Supervisor Upcall Engine: event shaping and delegated control-FDR enqueueing.
  Policy stays in the supervisor process; the hardware block only classifies,
  records, masks, parks, and wakes.

Any module that cannot meet its local-state fast path should be demoted to a
thin client of the relevant owner engine or moved into software behind an FDR
capability.

Hard-block robustness checklist:

- state registers use enumerated encodings with an explicit `INVALID` or
  recovery state.
- table/cache entries have valid bits, generation counters, and owner ids where
  stale references are possible.
- multi-step operations have documented phases: acquire, validate, prepare,
  commit, complete, abort/recover.
- architectural state is published only at commit points.
- cancellation, reset, fatal signal, and engine timeout behavior is defined for
  each phase.
- impossible combinations, such as a TID being both runnable and blocked, are
  rejected by scoreboard assertions in simulation and by runtime invariant checks
  where cheap.
- metadata written by devices or user memory is never trusted without capability
  and generation validation.
- watchdogs can force a stuck engine command into a bounded abort path or
  hardware panic state.
- local reset is defined per engine: either drain/abort outstanding commands and
  reinitialize local state from protected metadata, or enter a degraded state
  that rejects new commands and emits `ras_fault` until PID 1/supervisor action.
- parity/ECC faults are classified before recovery: correctable faults update
  counters and continue after repair; uncorrectable faults poison the affected
  object/page/descriptor and emit a structured fault event before any authority
  is reused.
- formal or exhaustive tests cover each module's state-transition graph before
  RTL freeze.
- the proof target is deterministic failure containment: a local engine fault,
  reset, timeout, or poisoned metadata record cannot silently create authority,
  corrupt unrelated domains, or require full-chip reset when the engine has a
  defined local recovery/degraded path.

## 5. Execution Model

The v1 processor contains a small number of identical in-order, multi-context,
barrel-style core tiles. A practical FPGA target is 2 to 4 tiles. Each tile can
execute one selected ready thread per cycle from its local issue lane, subject to
cache and engine availability.

Each hardware thread context contains:

- `pc`: 64-bit virtual instruction address.
- `lr`: 64-bit link register used by `CALL`, `CALL_REG`, and `RET`.
- 32 GPRs, 64-bit.
- 32 FPRs, 64-bit IEEE-754 storage.
- 16 vector registers, 128-bit.
- condition flags.
- current PID and TID.
- thread-local `ERRNO`.
- thread-local signal mask and pending per-thread signal queue.
- signal-delivery state.
- blocked/runnable/waiting state.

Each core tile executes one selected ready thread at a time. On each cycle, the
local scheduler front end supplies a runnable TID to fetch/issue. Simple ALU
instructions retire quickly. Complex instructions enqueue work and remove the
TID from the issuing core's active set.

This is not microcode: `OPEN_AT`, `CLONE`, `EXEC`, `MMAP`, and similar operations
are implemented by fixed hardware state machines and shared engines.

The global scheduler assigns runnable TIDs to core tiles. A thread may migrate
between tiles at scheduling boundaries. Migration transfers only ownership of
the thread context; cache coherence handles memory visibility.

## 6. Fixed Instruction Encoding

Every instruction is exactly 64 bits.

Common fields:

```text
63:56  opcode
55:52  format
51:48  flags/subop
47:40  a
39:32  b
31:24  c
23:16  d
15:0   imm16 or low control bits
```

The `format` field defines how the remaining bits are interpreted. Register
fields use the low bits of the 8-bit slots:

- GPR: 5 bits, values `0..31`.
- FPR: 5 bits, values `0..31`.
- VR: 4 bits, values `0..15`.
- FDR: 8 bits, values `0..255`.
- PCR: 4 bits.
- condition: 3 bits.
- width: 2 bits, `0=byte`, `1=half16`, `2=word32`, `3=double64`.

### 6.1 Formats

`F0`: no operands.

```text
opcode, format=0, rest ignored
```

Used by `NOP`, `RET`, `FENCE`, `YIELD`, `SIGRET`.

`F1`: register-register-register.

```text
a=dst, b=src1, c=src2
```

Used by integer ALU, FPU, and vector operations.

`LOCK_CMPXCHG` is the four-register exception using the existing `d` slot:

```text
a=dst_old, b=addr, c=expected, d=desired
```

`F2`: register-register.

```text
a=dst/src0, b=src
```

Used by `MOV`, `NOT`, `CMP`, `FREE`, `ERRNO_SET`, `SIGMASK_SET`,
`EXIT`, and similar two-register forms.

`F3`: register-immediate.

```text
a=dst, imm32=bits 31:0
```

Used by `LI32`, short constants, small offsets, and control registers.
`LI64` is encoded as two consecutive fixed 64-bit instructions:

```text
LI32.LO rD, imm32
LI32.HI rD, imm32
```

The assembler lowers source-level `LI rD, imm64` into one or two fixed
instructions.

`F4`: PC-relative control flow.

```text
a=condition or link mode, imm40=signed 8-byte instruction offset
```

Used by `JMP`, `BRANCH`, and `CALL`. Branch condition is encoded in
`flags/subop`: `0=EQ`, `1=NE`, `2=LT`, `3=GT`, `4=LE`, `5=GE`. The target is
`pc + sign_extend(imm40) * 8`, so branch targets are naturally aligned by
encoding.

`F5`: memory.

```text
a=gpr value, b=base gpr, width=flags[1:0], imm24=signed byte offset
```

Used by `LD` and `ST`. For `ST`, `a` is source. For `LD`, `a` is destination.
The assembler exposes width suffixes `LD.B`, `LD.H`, `LD.W`, `LD.D`, `ST.B`,
`ST.H`, `ST.W`, and `ST.D`.

`F6`: FDR/resource operation.

```text
a=fd_operand, b=result_dst, c=arg0, d=arg1, imm16=arg2/flags
```

Used by `OPEN_AT`, `PULL`, `PUSH`, `SEEK`, `GET_META`, `SET_META`, `CLOSE`, and
similar resource operations. `flags/subop` contains the fd operand mode:
`0=static low-256 fd immediate in a`, `1=GPR fd index in a`. This removes the
old `_DYN` opcode family while preserving both fast static descriptors and full
DDR-backed descriptor tables.

`F7`: two-resource operation.

```text
a=fd_operand0, b=fd_operand1/result_dst, c=arg0, d=arg1, imm16=flags
```

Used by `DUP` and compact two-descriptor operations. Source-level `pipe()` is an
`OBJECT_CTL queue` profile that returns narrowed read/write endpoint
capabilities; it is not a unique hardware primitive. Operand modes are encoded
in `flags/subop`.

`F8`: four-register system operation.

```text
a=result_dst, b=arg0, c=arg1, d=arg2, imm16=variant/flags
```

Used by `CLONE`, `MUNMAP`, `SIGACTION`, `KILL`, `AWAIT`, `WAKE`, `ENV_GET`,
`RANDOM`, `CALL_CAP`, `RET_CAP`, and message operations. The opcode selects the
operation; `imm16` selects variants or flags, not the primary operation.

`F9`: argument-block operation.

```text
a=result_dst, b=arg_block_ptr, c=arg_block_len, d=reserved, imm16=variant/flags
```

Used when the natural operand set does not fit in the fixed register slots.
`MMAP`, `EXEC`, `OPEN_AT`, namespace control, event-queue configuration,
object-control commands, DMA-control commands, supervisor-domain control
commands, resource-domain control commands, and capability transfer commands use
this format. Argument blocks are little-endian, naturally aligned, versioned,
and copied by the issuing hardware engine before the thread is parked.

`FA`: register-indirect control flow.

```text
a=target_reg, rest ignored
```

Used by `CALL_REG`.

`CALL` writes `pc + 8` to the thread-local `lr` and jumps to the F4 target.
`CALL_REG` writes `pc + 8` to `lr` and jumps to `a=target_reg`. `RET` is F0 and
sets `pc = lr`. Software call stacks are psABI conventions layered above this
architectural link register.

Architectural result convention:

- Instructions with a `result_dst` write their success value or all-ones `-1`
  sentinel to that register.
- Fallible instructions also update the issuing thread's `ERRNO`.
- Static legacy forms that omit an explicit result in source assembly are
  assembled with `result_dst=r1`; the binary encoding still contains the result
  destination.
- Documentation must not rely on an implicit `r1` result except as that assembly
  shorthand.

### 6.2 Opcode Map

The opcode map is fixed, but sparse:

```text
00 NOP
01 LI32.LO
02 LI32.HI
03 MOV
04 ADD
05 SUB
06 MUL
07 DIV
08 AND
09 OR
0A XOR
0B NOT
0C LSL
0D LSR
0E ASR
0F CMP

10 JMP
11 BRANCH
12 CALL
13 CALL_REG
14 RET

18 LD
19 ST
1A FENCE
1B ALLOC
1C FREE
1D ISYNC
1E ALLOC_EX
1F ALLOC_SIZE

20 OPEN_AT
21 CLOSE
22 DUP
23 PIPE_RESERVED
24 PULL
25 PUSH
26 SEEK
27 GET_META
28 SET_META
29 NS_CTL
2A EVENT_CTL_ALIAS_RESERVED
2B TIMER_CTL_ALIAS_RESERVED
2C CAP_SEND
2D CAP_RECV
2E CAP_DUP
2F CAP_REVOKE

30 AWAIT
31 WAKE
32 MSG_SEND
33 MSG_RECV_RESERVED
34 ERRNO_GET
35 ERRNO_SET
36 OBJECT_CTL
37 DMA_CTL
38 DOMAIN_CTL
39 CALL_CAP
3A RET_CAP
3B THREAD_JOIN

50 GET_PCR
51 SET_PCR
52 CLONE
53 EXEC
54 YIELD
55 EXIT
56 ENV_GET
57 RANDOM

60 MMAP
61 MUNMAP
62 SIGACTION
63 SIGMASK_SET
64 KILL
65 SIGRET
66 MPROTECT
67 SUPERVISOR_CTL
68 ALARM

70 LOCK_CMPXCHG
71 RESERVED
72 RESERVED

80 INB_RESERVED
81 OUTB_RESERVED
82 LOAD_UCODE
83 RESERVED
84 RESERVED

90 FADD
91 FSUB
92 FMUL
93 FDIV
A0 VADD32
```

Illegal or unimplemented opcodes normally deliver hardware `SIGILL`. If the
running process is bound to a supervisor domain with opcode-event policy
enabled, decode routes the event to the Supervisor Domain and Upcall Engine
instead of raising `SIGILL`. This is a fixed decode-priority mux, not a software
trap for native implemented instructions.

## 7. Register Files and Context Storage

The active execution lane reads and writes architectural state through a thread
context store.

Recommended v1 capacities:

- 2 to 4 coherent core tiles.
- 64 to 256 active hardware thread contexts on chip, with DDR-backed spill for
  inactive contexts.
- DDR-backed process contexts, with at least 4096 architectural PIDs.
- DDR-backed FDR tables, defaulting to 4096 descriptors per process and
  expandable higher.
- DDR-backed pending event queues, with at least 4096 records per process.
- 4096 or more global futex hash buckets, with DDR-backed waiter records.

The GPR/FPR/VR files may be implemented as multi-ported block RAM or replicated
distributed RAM. Since each core tile issues only one hardware thread into its
local datapath per cycle in v1, the port pressure is manageable.

`r31` remains the architectural stack pointer. In this hardware design it is
ordinary register state saved per thread context. The implementation may enforce
stack-region bounds through the MMU rather than making `r31` unwriteable.

## 8. Pipeline

Each v1 core tile uses an intentionally conservative pipeline:

1. Select runnable TID.
2. Fetch instruction through instruction TLB.
3. Decode fixed 64-bit instruction.
4. Read context registers.
5. Execute or enqueue engine command.
6. Write back result or park thread.

Simple integer operations retire through the execute stage. Memory operations
may stall the issuing thread behind the LSU, but the scheduler may issue another
ready thread while the LSU waits on DDR.

Branches update only the issuing thread's PC. V1 allows same-TID back-to-back
issue and therefore includes a fixed same-TID squash path for wrong-path
instructions after a branch resolves. Simple conditional branches are resolved
early with a dedicated compare/branch unit in decode where possible; the maximum
same-TID squash window is 1-2 issue slots. There is no branch prediction and no
microcode.

`ISYNC` uses F8: `a=result_dst`, `b=addr_or_fd`, `c=len`, `d=reserved`,
`imm16=range/object flags`. It triggers instruction-cache invalidation for a
range or mapped executable object using the same invalidation fabric already
used by `EXEC`, page remap, and permission changes. It is required for in-place
code patching and JITs. The instruction is a hardware event trigger; it does not
load or alter control store.

## 9. Coherent Multicore

Coherent multicore is a v1 feature, but it is deliberately bounded. The initial
target is 2 to 4 identical LNP64 core tiles connected to a shared coherence
fabric and external DDR. The goal is not high-end server performance; the goal
is that ordinary shared-memory programs, futexes, copy-on-write VM, FDR metadata,
and DMA-backed file operations have correct cross-core visibility.

### 9.1 Cache Hierarchy

Each core tile has:

- private L1 instruction cache.
- private L1 data cache.
- private instruction and data TLBs.

FPGA v1 uses a shared L2 cache with inclusive tags. Directory-based coherence is
a later scaling option, not part of the frozen v1 baseline. L1 data caches are
write-back and coherent. L1 instruction caches are coherent through explicit
instruction-cache invalidation events during `EXEC`, page remap, and executable
file updates.

### 9.2 Coherence Protocol

The frozen v1 data coherence protocol is MSI. MESI is a later optimization.

Minimum line states:

- `I`: invalid.
- `S`: shared clean.
- `M`: modified and owned by one core.

Operations:

- load miss requests shared ownership.
- store miss or store upgrade requests modified ownership.
- modified owner supplies or writes back data before another core reads it.
- invalidations acknowledge before the requesting store can retire.

All coherence requests are tagged with core id, thread id, process id, physical
cache-line address, and operation type.

### 9.3 Memory Visibility Contract

LNP64 v1 uses a developer-friendly memory model:

- normal cached memory is coherent and TSO-like.
- weaker or device-specific behavior is opt-in through VMA memory type and
  explicit `FENCE`.
- locked atomics are single-copy atomic and sequentially consistent in v1 unless
  a future instruction encoding explicitly requests weaker acquire/release
  semantics.

Normal cached memory rules:

- each core observes its own loads and stores in program order.
- stores become visible to other cores in program order.
- loads may read the issuing core's older buffered stores to the same address.
- a load is not reordered after a later store as observed by other cores.
- aligned naturally sized loads/stores up to 64 bits are single architectural
  memory operations.
- instruction fetch observes code changes only after the required
  `MPROTECT`/`ISYNC` or exec/remap invalidation event.

Atomic and synchronization rules:

- `LOCK_CMPXCHG` is a read-modify-write transaction with sequentially
  consistent ordering in v1.
- successful and failed locked atomics have acquire+release ordering around the
  atomic access.
- futex `AWAIT` performs an acquire-style expected-value check before parking.
- futex `WAKE` performs release-style ordering before making waiters runnable.
- call-gate synchronous entry observes argument/register state after the caller
  has reached the call commit point; `RET_CAP` publishes return values before
  waking the continuation.
- signal delivery observes architectural state at a precise boundary; `SIGRET`
  resumes after the signal frame and handler-side memory effects are visible
  according to normal memory rules.

DMA and engine visibility rules:

- POSIX engine completions are ordered after their DMA writes, metadata updates,
  and result-register writes.
- coherent DMA participates in the L2-coherent fabric before completion is
  signaled.
- a non-coherent implementation must expose explicit cache maintenance or fail
  the coherent-DMA feature bit; it may not advertise coherent PCIe DMA.
- VMA permission changes, unmaps, revocation, and page installs complete their
  required TLB/cache/I-cache invalidation acknowledgements before affected
  threads resume or backing authority is reused.

This model is stronger than many commercial relaxed models, but it makes
personality ports, libc, language runtimes, and formal proofs less fragile.

### 9.4 Atomics and Futexes

`LOCK_CMPXCHG` enters the coherence fabric as a read-modify-write transaction.
The target cache line must be held in modified ownership by the atomic unit
before comparison and writeback.

`AWAIT` futex waits and `WAKE` futex wakeups use physical addresses after
translation. This is important: two processes mapping the same shared page must
wait on the same futex key even if their virtual addresses differ.

The Futex Engine snoops or receives explicit notifications for atomic writes to
futex-backed addresses only when needed. It does not need to observe every store
in the system.

Futex correctness depends on the Memory Visibility Contract: user code releases
shared state with normal TSO stores or locked atomics before `WAKE`, and a waiter
that returns from `AWAIT` observes the wake after an acquire-style transition.
The Futex Engine provides wait/wake atomicity, not a separate weak memory model.

### 9.5 TLB and VMA Coherence

Page table and VMA changes are coherent across cores.

For `MMAP`, `MUNMAP`, copy-on-write breaks, `EXEC`, and permission changes, the
VMA Engine emits TLB invalidation commands:

- target all TLBs for a process.
- target one virtual page in one process.
- target all executable mappings for instruction-cache invalidation.

The issuing hardware operation completes only after all relevant core tiles
acknowledge invalidation. Threads in the affected process cannot resume on stale
translations.

### 9.6 DMA Coherence

Device DMA is coherent by construction. In v1, the DMA Fabric participates at
the inclusive shared L2 boundary:

- DMA reads observe dirty CPU-owned lines by forcing writeback or intervention.
- DMA writes invalidate or update matching CPU cache lines before completion.
- completion events are not delivered until cache visibility is correct.

This rule is mandatory for `PULL`, `PUSH`, object-backed page-fill
transactions, Ethernet RX/TX, SD/SPI transfers, and PCIe DMA.

Hard invariant: no device may write DDR through a path that bypasses the
L2-coherent DMA fabric. PCIe requester traffic enters through the Root
Complex/IOMMU coherent bridge. If a target FPGA cannot provide coherent PCIe DMA
at the L2 boundary, the implementation must use explicit cache clean/invalidate
operations in the DMA Fabric before delivering completion events; it may not
advertise "coherent by construction" for PCIe.

### 9.7 Shared Capability and Object Metadata

FDR tables, process tables, VMA descriptors, namespace dispatch records,
service-owned object descriptors, pipe/queue buffers, socket queues, and wait
queues are shared architecture-visible data structures. Hardware owns the
capability, dispatch, wait, and object-state metadata needed for safety and
fast-path operation; filesystem-specific metadata such as inodes, extents,
journals, directory indexes, and rename policy belongs to filesystem service
domains. Shared hardware metadata is protected with hardware locks or
single-writer engine ownership:

- FDR table entry updates are serialized per process and fd index.
- namespace request/reply and capability install are serialized per namespace
  object or service endpoint.
- process table mutation is serialized per PID slot.
- runqueue updates are serialized by the scheduler fabric.
- pipe and socket queue updates are serialized per queue object.

The hardware engines may be internally pipelined, but they must expose atomic
architectural transitions to threads.

Critical metadata protection:

- FDR table entries, process/thread records, VMA descriptors, domain records,
  scheduler queue links, wait queue heads/tails, event queue indices, heap
  metadata, DMA descriptors, namespace dispatch records, and hardware-owned
  object metadata carry parity or ECC.
- Correctable errors increment per-engine and per-domain counters and repair the
  stored word where the target memory permits writeback.
- Uncorrectable errors poison the affected object, descriptor, page, or queue
  entry, prevent new authority-bearing operations through it, and emit a
  structured `ras_fault` event.
- Object generations are never advanced past an uncorrectable metadata fault
  without supervisor/PID 1 acknowledgement, preventing corrupt metadata from
  being silently recycled as fresh authority.

## 10. External DDR Memory Model

External DDR holds:

- Program text and data.
- Stacks.
- Heaps.
- File cache pages.
- Page tables.
- VMA descriptors.
- Process tables.
- Namespace dispatch/service records.
- Service-owned directory entries and file metadata cache.
- Pipe buffers and socket buffers.

The FPGA contains caches and metadata accelerators, but DDR is the architectural
backing store.

The MMU implements:

- 4 KiB pages.
- per-process address spaces.
- hardware page table walker.
- instruction and data TLBs.
- access bits and dirty bits.
- copy-on-write page table entries.
- page fault event generation.

On a page fault, the issuing thread is parked. The VMA/Page Engine owns a
bounded page-state machine. It decides whether to install a resident page,
allocate and zero an anonymous page, complete a COW break, start a bounded
object-fill transaction, report a guard/protection fault, or fail with a signal.
It does not own general file page-cache policy, dirty writeback policy,
truncation semantics, filesystem coherence, swap, overcommit, or service
restart behavior.

Frozen v1 page states:

- `UNMAPPED`: no valid VMA/PTE covers the address.
- `RESERVED`: VMA exists, but no physical page is yet committed.
- `NONRESIDENT_OBJECT`: mapping references an object capability, offset, rights,
  and generation; content must be supplied by the object owner.
- `FILL_PENDING`: a bounded page-fill request has been sent to the object owner,
  and the faulting thread is parked.
- `RESIDENT_CLEAN`: page is resident and not dirty through this mapping.
- `RESIDENT_DIRTY`: page has been written through a writable mapping.
- `COW_SHARED`: page is resident, shared by COW references, and requires copy on
  write.
- `PINNED_DMA`: page has active DMA pins and cannot be unmapped/reused.
- `REVOKING`: mapping generation is being invalidated; no new pins/fills are
  accepted.
- `POISONED`: metadata or page contents are known bad; access fails and emits a
  fault event.

Freezeable hardware mechanisms:

- VMA range/protection/generation checks.
- W^X, NX, guard, ASLR, and memory-type enforcement.
- anonymous zero-fill page allocation.
- anonymous COW copy and atomic PTE swap.
- accessed/dirty bit maintenance.
- page zeroing, poisoning, and optional quarantine.
- TLB and instruction-cache invalidation broadcast/ack.
- DMA pin/unpin checks with no-new-pins during revoke.
- Resource Domain page pressure and mapped-page counters.
- Object-Backed Page Transaction Protocol: validate capability/generation,
  emit fixed page request, park or redirect the faulting thread, verify the
  service reply, and install a returned page only if all original authority
  checks still match.

Service/object-owner responsibilities:

- object contents for `NONRESIDENT_OBJECT` fills.
- dirty writeback and `msync` policy.
- coherence between `PULL`/`PUSH` and mapped object pages.
- truncation, hole, sparse-file, and append semantics.
- filesystem/storage replay and service restart behavior.
- eviction/reclaim policy beyond hardware pressure counters and pinned-page
  safety.

### 10.1 Object-Backed Page Transaction Protocol

Object-backed faults are authority-bearing transactions, not a hidden hardware
page cache. Hardware owns the transaction skeleton and commit checks; the object
or service owner owns contents and backing-object policy.

Page request fields:

- request version and operation id `page_fill`.
- process id, thread id, and Resource Domain id/generation.
- mapping id and VMA generation.
- object class, object id, object generation, lineage root, and lineage epoch.
- page-aligned object offset and fixed page length.
- requested access direction: read, write-fault/COW, execute-fetch, or prefetch.
- requested VMA permissions and memory type.
- executable provenance requirement when the VMA may become executable.
- completion target and cancel token.

Allowed service replies:

- page-frame or memory-object page capability with matching range, generation,
  rights, and memory type.
- zero-page decision where the object profile permits holes or zero fill.
- shared memory-object page where the profile permits shared page identity.
- retry/later with a bounded wait/event token.
- typed error, including stale object, permission denial, truncated range,
  poisoned object, I/O failure, or unsupported mapping.

Commit rules:

- the returned page is a proposal until the VMA/Page Engine validates it.
- hardware installs only if VMA generation, object generation, lineage epoch,
  requested permissions, memory type, executable provenance, and domain policy
  still match the original fault.
- the returned page cannot grant broader read/write/execute, sharing, DMA,
  cacheability, or device-memory rights than the VMA and object capability
  allowed.
- if `MUNMAP`, `MPROTECT`, revocation, object generation change, truncation
  notice, domain teardown, or fatal signal wins the race before commit, the
  pending fill is canceled and returns `EREVOKED`, `EINTR`, `SIGBUS`, or the
  object-specific stale/fault status.
- service timeout or repeated retry emits a structured fault/pressure event and
  may return `EAGAIN`, park on a service event, or fail according to mapping
  policy.

Dirty/writeback rules:

- hardware owns accessed/dirty bits and can enumerate dirty pages/ranges for a
  mapping or object capability.
- dirty writeback policy, `msync`, truncation, holes, sparse files, and
  `PULL`/`PUSH` coherence are service-profile semantics.
- `MSYNC` is a typed control operation to the object service. Hardware supplies
  validated dirty-range metadata and enforces VMA/capability permissions, but it
  does not decide filesystem writeback ordering beyond the storage barrier
  contract.
- executable object-backed pages require executable provenance from the loader
  or object service and still pass W^X/NX/domain policy checks.

### 10.2 Frozen Memory Model Constants

LNP64 v1 fixes these constants:

- byte order: little-endian.
- address size: 64-bit virtual addresses.
- physical address size: implementation-defined, at least 40 bits for v1.
- page size: 4 KiB.
- cache line size: 64 bytes.
- instruction size: 64 bits, naturally aligned.
- integer load/store widths: 8, 16, 32, and 64 bits.
- atomic width for `LOCK_CMPXCHG`: 64 bits in v1.
- vector register width: 128 bits.
- floating point format: IEEE-754 binary64.
- VMA memory types: `normal_cached`, `uncached`, `device_ordered`,
  `write_combining`.

Alignment rules:

- instruction fetch from a non-8-byte-aligned PC raises `SIGBUS`.
- aligned loads and stores are single architectural memory operations.
- unaligned integer loads and stores are supported if contained within one page.
- unaligned accesses crossing a page boundary may complete only if both pages
  translate and permit the access; otherwise the instruction faults without a
  partial architectural write.
- `LOCK_CMPXCHG` requires 8-byte alignment; misalignment raises `SIGBUS`.

Device VMA rules:

- `device_ordered` mappings are uncached and strongly ordered for CPU
  loads/stores.
- `write_combining` mappings may combine writes but must not cache reads; this
  is required for GPU framebuffers and high-throughput device windows.
- software must execute `FENCE` before relying on write-combined stores being
  visible to a device doorbell, DMA engine, or completion observer.
- `uncached` mappings are not gathered into coherent CPU caches, but still obey
  explicit `FENCE` ordering with normal memory and DMA.
- device mappings are never executable.
- device mappings are created only by `MMAP` on an FDR whose object class grants
  device memory authority, such as `pcie_bar`.
- after a device mapping is installed, ordinary `LD` and `ST` instructions use
  the TLB/PTE memory type; there is no FDR lookup or capability check on every
  device access.

`FENCE` semantics:

- drains prior stores from the issuing core into the coherent fabric.
- waits for invalidation acknowledgements required by prior stores.
- orders prior normal-memory writes before later DMA or device operations.
- orders prior DMA/engine completions before later ordinary memory operations.
- orders `device_ordered` MMIO loads/stores against normal memory and DMA.
- flushes or commits write-combining buffers before later ordered MMIO
  doorbells, DMA submissions, or completion assumptions.
- does not flush unrelated cache lines unless an explicit cache-maintenance
  profile is added by a future implementation.

## 11. Hardware Scheduler and Runqueue

The scheduler is a fabric block, not software. In the coherent multicore design,
it has per-core ready queues plus a global scheduler arbiter.

V1 uses a hardware weighted-fair virtual-time model inspired by Linux
CFS/EEVDF, but not Linux CFS in RTL. The stable contract is weighted fair
dispatch over threads and Resource Domains using virtual runtime/deadline
accounting, hierarchical quotas, bounded wakeup placement, and bounded
preemption points. Linux nice values, cgroup CPU weights/quotas, affinity masks,
and latency hints map naturally onto this contract, but Linux scheduler
heuristics, red-black trees, PELT history, NUMA balancing, policy callbacks, and
plugin schedulers remain software/personality policy.

This is a fixed hardware algorithm, not a policy hook. Software configures
weights, quotas, latency classes, affinity masks, and domain hierarchy; hardware
owns ready/blocked state, no-lost-wakeup transitions, virtual-time accounting,
quota eligibility, preemption boundaries, and dispatch. A platform may implement
the queues with different bounded datapath structures, but it must expose the
same architectural fairness, accounting, and maximum-latency constants.

State:

- Per-core local ready queues of runnable TIDs.
- Global runnable overflow/spill queues.
- Per-thread virtual runtime/deadline, fixed weight index, latency class, and
  preemption accounting.
- Per-domain virtual runtime/deadline, hierarchical quota/period counters,
  dispatch budget, and allowed core-tile mask.
- Bucketed virtual-deadline queues and/or small sorted active windows in FPGA
  RAM, with DDR-backed spill for cold runnable entities.
- Fixed weight table and bounded normalization state.
- Sleeping timer wheel.
- fd wait queues.
- futex wait queues.
- child-exit wait queues.
- signal-pending queues.

Thread states:

- `READY`
- `RUNNING`
- `WAIT_DDR`
- `WAIT_FD`
- `WAIT_FUTEX`
- `WAIT_CHILD`
- `SLEEPING`
- `SIGNAL_DELIVERY`
- `ZOMBIE`
- `DEAD`

Instruction behavior:

- `YIELD`: charges consumed runtime, updates virtual time/deadline, and
  reinserts the current TID according to the weighted-fair queue policy.
- timer-flavored `AWAIT`: inserts current TID into timer wheel.
- `AWAIT`: attaches current TID to a waitable object's event mask or predicate.
- long resource operations: mark current TID blocked on an engine command.
- engine completion: writes result registers, updates errno, returns TID to
  ready queue unless a signal must be delivered first.

Each core-local scheduler chooses the next ready TID from its active window when
available. The global arbiter handles wakeups, new threads, thread migration,
load balancing, work stealing, virtual-time normalization, and DDR spill/refill.
Dispatch prefers the eligible runnable entity with the earliest virtual
deadline within the implementation's bounded approximation window. Blocked
threads do not consume CPU budget. Runnable threads whose Resource Domains have
exhausted quota remain ineligible until the next period or budget update.

Hardware-shaped representation rules:

- no red-black tree or arbitrary scheduler tree walk in the dispatch path.
- no scheduler bytecode, callbacks, or policy plugins.
- fixed weight table; software selects indices, not formulas.
- bounded number of latency classes.
- bucketed virtual-time wheels, bitmaps, and small sorted windows are allowed.
- DDR spill/refill is allowed only off the common dispatch path.
- approximation error, maximum preemption latency, and maximum wakeup insertion
  latency are implementation-profile constants exposed through `ENV_GET`.

Fairness and accounting rules:

- consumed CPU advances a runnable entity's virtual runtime inversely to weight.
- the v1 weight table is fixed by the implementation profile and monotonic:
  higher weight receives no less CPU share than a lower weight among equally
  eligible runnable entities.
- wakeup insertion never grants unbounded credit; a woken entity may receive a
  bounded latency placement adjustment, but the adjustment is capped by an
  implementation-profile constant.
- Resource Domains are schedulable entities as well as accounting containers;
  child CPU usage charges all ancestors.
- quotas and periods are hierarchical and monotonic downward.
- no runnable thread with eligible domain budget may starve beyond the
  implementation's bounded fairness window.
- a runnable thread is eligible only when every ancestor Resource Domain has
  budget, is not frozen, and permits the target core-tile mask.
- call-gate synchronous calls charge CPU to the executing target domain by
  default; explicit donation/charge-transfer profiles must be capability
  authorized and bounded.
- domain freeze removes all descendant runnable entities from eligibility after
  they reach a scheduling boundary or forced park point.

Preemption rules:

- hardware timer/accounting ticks may force a running thread to a bounded
  scheduling boundary.
- long engine operations park the thread and release the core to the scheduler.
- supervisor-domain timer upcalls may request forced park/redirection for threads
  in a delegated subtree, but the scheduler fabric still performs the transition
  and charges accounting.
- preemption cannot expose raw interrupts or software scheduler callbacks.

Timer and event-queue FDRs:

- A `timer` FDR represents a one-shot or periodic monotonic/realtime timer.
- An `event_queue` FDR aggregates readiness from other FDRs: file/device
  readiness, timers, child exit, signal delivery, futex events, supervisor
  upcalls, and PCIe IRQ event FDRs.
- `AWAIT` can block on an `event_queue` FDR and wake when any member source
  becomes ready.
- Event queue records are fixed-width, versioned, and carry source fd/object id,
  event mask, result code, and optional operation id.
- This is the hardware substrate for `poll`, `select`, `epoll`, `kqueue`,
  timeout waits, and supervisor-domain event dispatch.

Event fast path:

- timer wheel heads, active event queue heads/tails, and recently armed wait
  slots live in FPGA RAM.
- common wakeups update a small ready bitmap or local queue entry before touching
  DDR.
- DDR event records are used for overflow, large payload references, cold event
  queues, and durable ordering metadata.
- Event Router fan-in is distributed by source class where practical: timers,
  device IRQs, engine completions, futexes, signals, and supervisor upcalls each
  have narrow ingress queues before central arbitration.

Event queue acceleration:

- active event queues have on-chip source slots for the hot subset of watched
  objects.
- each source slot carries source object id or fd, event mask, readiness
  generation snapshot, trigger mode, user data, and ready bits.
- `OBJECT_CTL` add-source for an event queue is an atomic add/check/arm
  sequence: install the source, snapshot readiness generation, check current
  readiness, and enqueue immediately if the source is already ready.
- waitable objects publish narrow readiness-change events to the Event Router:
  object id, event mask, and generation.
- the Event Router matches active source slots, sets ready bits, appends compact
  event records, and wakes parked TIDs without a software scan.
- `AWAIT(event_queue)` checks nonempty ready bits before parking so add/check
  and park cannot lose a wakeup.
- `PULL(event_queue)` can batch multiple event records for `poll`, `select`,
  `epoll_wait`, `kqueue`, and runtime executor drains.
- timer profiles are ordinary source slots driven by the timer wheel.
- edge-triggered sources compare generation changes; level-triggered sources
  may requeue while the source remains ready; one-shot sources disable
  themselves after one emitted event until rearmed.
- if the active source window or event record queue overflows, hardware may
  coalesce by source, set an overflow/rescan event, or spill cold records to DDR
  according to queue policy.
- large or cold subscription sets live in DDR. Readiness changes for cold
  sources mark the queue overflow/rescan path rather than forcing the Event
  Router to walk thousands of subscriptions in hardware.

### 11.1 Hardware-Owned Runtime Objects

The same hard blocks that make POSIX compatibility cheap should also accelerate
ordinary application and language-runtime code. The general abstraction is:

```text
hardware-owned waitable/capability objects with local state,
bounded transitions, and event delivery
```

These objects are represented by FDR capabilities, but they are not limited to
Unix files. V1 exposes only three primitive generic object classes:

- `counter`: waitable integer state with threshold/predicate wakeups.
- `queue`: bounded byte or record queue with readable/writable wakeups.
- `memory_object`: capability-scoped memory range or arena with map/pin/protect
  operations.
- `call_gate`: callable entry into another thread, service, or Resource Domain.

Higher-level runtime concepts are profiles over those primitives, not distinct
hardware classes:

- semaphore, event counter, completion, countdown latch: `counter`.
- channel, message queue, task queue, pipe-like runtime queue: `queue`.
- shared-memory object, memory arena, guarded region, DMA buffer profile:
  `memory_object`.
- task/runtime event: `counter` or `queue`, depending on runtime convention.
- protected procedure call, service call, cross-thread call, or cross-domain
  call: `call_gate`.

Common operations reuse the refined ISA:

- `OBJECT_CTL`: creates/configures `counter`, `queue`, and `memory_object`
  primitives, including queue depth, record size, wake policy, rights, and
  optional backing memory.
- `PULL` / `PUSH`: receive from or send to stream-like objects such as channels,
  queues, pipes, sockets, and event queues.
- `AWAIT`: parks a thread on an object state transition, memory predicate,
  timer, event counter, or runtime task event.
- `CAP_*`: duplicates, narrows, transfers, seals, or revokes object authority.
- `MMAP` / `MPROTECT`: maps shared-memory objects, arenas, guard pages, JIT
  regions, DMA buffers, and capability-scoped device memory.
- `ALLOC` / `FREE`: provide the default small-object allocation path for
  ordinary runtimes.
- `DMA_CTL`: submits memory-to-memory or memory-object bulk operations such as
  copy, fill, scatter/gather copy, and optional checksum/hash variants.
- `CALL_CAP` / `RET_CAP`: performs small-register calls through callable
  capabilities, including cross-thread and cross-domain call gates.

The point is not to make application code look like syscalls or to add a new
hardware module for every runtime abstraction. The point is that runtime
primitives such as channels, futures, async executors, condition variables, work
queues, shared arenas, large copies, and safe resource handles can be built from
three small object FSMs plus the existing Heap, Event, Futex, VMA, Capability,
Signal, and DMA blocks.

`OBJECT_CTL` uses F9. Its v1 argument block is versioned and names:

- object type: `counter`, `queue`, or `memory_object`.
- create, configure, query, reset, or destroy operation.
- initial rights and event mask.
- queue/record depth or size where applicable.
- optional backing FDR or memory range.
- wake policy: edge, level, one-shot, count-based, or predicate-based.
- destination FDR table slot policy.

The current emulator subset uses these v1 scalar constants:

- operation `1`: create.
- kind `1`: counter.
- kind `2`: queue.
- kind `3`: memory object.
- queue profile `1`: pipe endpoints.
- queue profile `4`: call gate.
- call-gate mode `0`: synchronous.
- call-gate mode `1`: asynchronous completion to a counter or queue endpoint.
- call-gate mode `2`: handoff without a return continuation.
- call-gate flag bit `0`: capability-marked scalar arguments are permitted.

`DMA_CTL` uses F9. Its v1 argument block is versioned and names:

- operation: copy, fill, scatter/gather copy, checksum, or hash profile.
- source address/object and destination address/object.
- byte length or descriptor-list pointer.
- memory ordering and cache-coherence policy.
- optional completion event object.
- optional cancellation policy.

Both instructions return success, a new FDR index, byte count, or operation id
in the encoded result register. Long operations park the issuing thread or
complete through an event object according to flags.

The current emulator subset intentionally implements only synchronous
memory-to-memory `DMA_CTL` commands:

- arg+0: operation, where `1` is copy and `2` is fill.
- arg+8: destination virtual address.
- arg+16: source virtual address for copy or byte fill value for fill.
- arg+24: byte length.
- arg+32: optional `dma_buffer` FDR token limiting the permitted byte range.

The emulator does not enqueue DMA descriptors, expose async DMA completion
objects, or keep a pending-DMA table. A successful `DMA_CTL` has completed and
made its writes visible before the next instruction can execute. `CAP_REVOKE`
therefore cannot race with an in-flight emulator DMA operation: revoke blocks
future submissions through that capability lineage, while operations that
already returned are complete. The hardware target above still needs a separate
quiesce-or-cancel policy before asynchronous DMA descriptors are enabled.

### 11.2 Capability Calls

`CALL_CAP` and `RET_CAP` provide a fast path for calling through hardware
capabilities. They are intended for pre-provisioned services, worker threads,
sandboxed components, driver services, supervisor services, and Resource Domain
entry points.

The goal is not to make cold container or VM creation as cheap as a function
call. Cold creation still allocates domains, VMAs, namespaces, FDR tables, and
budgets. The goal is to make hot calls into an already-provisioned thread or
domain close to a protected procedure call or hardware thread handoff.

`CALL_CAP` uses F8:

```text
a=result_dst, b=call_gate_fd, c=arg0, d=arg1, imm16=flags
```

`RET_CAP` uses F8:

```text
a=result_dst, b=value0, c=value1, d=reserved, imm16=flags
```

`call_gate` FDRs carry:

- mode: synchronous call, asynchronous call, or handoff.
- target kind: thread, service queue, Resource Domain entry, supervisor service,
  driver service, or runtime actor.
- target domain id and generation when cross-domain.
- target TID, parked worker pool, or service queue id.
- entry PC or service selector.
- allowed argument shape and register count.
- capability-passing permission.
- shared-memory or copied-buffer policy.
- scheduler and resource-accounting policy.
- return continuation policy.

Hot path requirements:

- caller holds a valid `call_gate` FDR with call rights.
- target thread/domain/worker is already provisioned and generation-valid.
- budget and domain checks hit local active-window state.
- arguments fit in fixed registers or pre-delegated FDR objects.
- no namespace, VMA, or FDR table reshaping is performed on the call path.

On `CALL_CAP`, hardware validates the call gate, records a bounded return
continuation, charges the caller/target domain according to policy, transfers
small register arguments, and schedules the target thread/service. For
same-domain cross-thread calls this is a hardware thread handoff. For
cross-domain calls it additionally switches domain id, credential snapshot, and
accounting context; with hot ASID/TLB state, no global flush is required.

`RET_CAP` resolves the saved continuation, writes small return values, updates
usage counters, wakes or resumes the caller, and retires the callee side of the
call according to the call-gate policy.

Call gate modes:

- synchronous: `CALL_CAP` parks the caller in a wait-for-return state, records a
  bounded return continuation, wakes or schedules the target, and requires
  `RET_CAP` to resume the caller with return values.
- asynchronous: `CALL_CAP` enqueues or starts work and returns status or an
  operation id to the caller immediately. Completion is delivered to an
  `event_queue`, `counter` completion profile, service queue, or other
  configured waitable object.
- handoff: `CALL_CAP` transfers request ownership to the target and does not
  create a return continuation for the original caller. Depending on flags, the
  caller may detach, park on a separate event, or end the current activation.

Mode constraints:

- synchronous calls require bounded return-continuation storage; if unavailable,
  `CALL_CAP` fails with `EAGAIN` or `ENOMEM`.
- asynchronous calls require a completion target unless the gate is explicitly
  marked fire-and-forget.
- handoff calls must define cancellation ownership and resource-accounting
  transfer.
- cross-domain calls charge resource usage according to the call gate policy:
  caller, callee, split, or parent-domain accounting.
- capability passing is denied unless explicitly enabled by the call gate.
- reentrant call depth is bounded per thread and per domain.

## 12. Capability File Descriptor Registers

FDRs are not integer registers. Each process owns a DDR-backed hardware FDR
capability table. The default architectural table has 4096 descriptor entries
per process and can be expanded by implementation.

Each FDR entry contains:

- valid bit.
- object class: `closed`, `regular_file`, `directory`, `char_stream`,
  `block_device`, `pipe_read`, `pipe_write`, `net_namespace`,
  `net_interface`, `packet_queue`, `datagram_endpoint`, `stream_endpoint`,
  `socket_compat`, `listener`, `event_queue`, `timer`, `counter`, `queue`,
  `memory_object`, `call_gate`, `control`, `pci_function`, `pcie_bar`,
  `dma_buffer`, `irq_event`, `gpu_device`, `accelerator`.
- backend id: `none`, `uart0`, `sd0`, `spi_flash0`, `eth0`, `ramfs`,
  `pipe_engine`, `namespace_service`, `object_engine`, `network_service`,
  `supervisor_engine`, `pcie_root`, `pcie_iommu`, `pcie_msi`, `nvme_driver`,
  `ethernet_driver`, `wifi_driver`, `gpu_driver`.
- protocol or subtype: `raw_frame`, `udp_datagram`, `stream`, `block_extent`,
  `block_image`, `tty`, `control`, `pci_config`, `bar_mmio`,
  `timer_oneshot`, `timer_periodic`, `msi_vector`, `msix_vector`,
  `pinned_dma`, `framebuffer`, `bounded_records`, `counting`,
  `single_assignment`, `runtime_task`, `shared_arena`, or
  backend-defined.
- rights: read, write, seek, stat, directory, execute, poll, wait, signal, map,
  dma, transfer, call, return.
- object id.
- object generation.
- capability generation.
- lineage root id and lineage epoch.
- parent capability generation or revocation-root pointer when derived.
- current offset.
- flags.
- reference count pointer.
- event mask.
- metadata cache pointer.
- backend-private pointer.

FDR operand mode is encoded in the instruction, not in separate opcodes. Static
mode addresses only the low 256 descriptors with the 8-bit FDR field. Register
mode uses a GPR containing the runtime descriptor index and can address the full
DDR-backed descriptor table. Legacy source-level names such as `READ_FD_DYN` may
remain assembler aliases for compatibility notes, but the binary ISA has one
opcode per operation.

The hardware validates range, valid bit, and rights before issuing the
operation.

Every authority-bearing FDR entry includes object generation, capability
generation, lineage root, and lineage epoch. Cached descriptor hits are valid
only when the cached generations and epoch still match the object owner and
lineage owner. This makes stale descriptor reuse, post-revocation use, and
destroy/recreate aliasing fail as `EBADF`, `EREVOKED`, or the object-specific
stale-reference error instead of silently targeting a new object.

Invalid descriptors write `-1` to the encoded result register where applicable
and set the issuing thread's `ERRNO=EBADF`.

The FDR/Capability Engine earns hard silicon only if descriptor validation is a
local fast path:

- active process low descriptors `0..255` are held in FPGA RAM or registers.
- recent dynamic FDR entries are cached per core or per active process window.
- cached entries include valid bit, rights, object class, backend id, event mask,
  current offset, object id, and metadata-cache pointer.
- object reference count updates are routed through the capability owner path,
  but common read-only validation does not fetch the full DDR table entry.
- capability transfer, narrowing, and revocation may use DDR lineage metadata,
  but already-cached descriptors are marked revoked or generation-mismatched by
  a compact invalidation event.

Fast path target: cached `PULL`, `PUSH`, `AWAIT`, `MMAP`, `GET_META`, `SEEK`,
and `CLOSE` descriptor checks complete without DDR table reads.

### 12.1 Capability Operations

FDRs are the security boundary, so capability movement is architectural.

`CAP_DUP`:

- uses F9.
- duplicates an FDR capability within the same process FDR table.
- may narrow rights, event masks, allowed ranges, or mapping permissions.
- may seal the duplicate when requested and permitted by the source rights.
- cannot broaden authority beyond the source capability.

`CAP_SEND` and `CAP_RECV`:

- use F9.
- transfer or copy an FDR capability over a pipe, socket, message channel, or
  supervisor control FDR that permits capability passing.
- capability payloads are delivered out-of-band beside ordinary message bytes,
  similar to Unix descriptor passing.
- `MSG_SEND` may carry small scalar messages only. Receiving is modeled as
  `AWAIT`/`PULL` over a message endpoint, queue, or call-gate completion object.
  Capability passing uses `CAP_SEND`/`CAP_RECV` so transfer, sealing, and
  revocation rules stay explicit.

`CAP_REVOKE`:

- uses F9.
- requests revocation of a revocable capability lineage.
- prevents new operations from starting through revoked descendants.
- emits compact invalidation events for active FDR caches, event-source slots,
  mapped VMAs, call-gate continuations, and DMA exports derived from the
  revoked lineage.
- waits for or cancels in-flight operations according to each object's
  cancellation policy.
- cannot revoke immutable capabilities unless the issuer marked them revocable.

Sealing and minting discipline:

- a sealed FDR may be transferred but not narrowed, duplicated, or used to mint
  related capabilities unless the sealed rights allow it.
- software services never write raw FDR authority. A service reply may carry a
  capability proposal, but that proposal is data until the Capability Engine
  validates and commits it.
- only hardware engines, the boot engine, object owner engines, or a process
  holding an explicit class-scoped mint/root capability can request creation of
  a new authority-bearing FDR.
- all new authority is derived from existing authority. Mint/install checks
  object class, object id/generation, rights, ranges, memory type, transfer
  rights, lineage root/epoch, owner service generation, receiver domain policy,
  and object-specific constraints before publishing the FDR table entry.
- the PCIe Bus Master can request `pci_function`, `pcie_bar`, `dma_buffer`, and
  `irq_event` FDRs only because reset grants it the PCIe Root Complex control
  FDR with mint rights; hardware performs the actual derivation and install.
- namespace, filesystem, network, loader, and supervisor services can request
  returned capabilities only inside the root capabilities delegated to them.
- supervisor domains can request delegated control/event FDRs only inside their
  assigned subtree.

### 12.2 Capability Lineage and Revocation Algebra

All authority-bearing objects use the same lineage model. This is the frozen
revocation algebra for FDRs, VMAs, event sources, call gates, DMA buffers,
mapped BARs, object-backed page fills, classifier tables, network endpoints,
namespace roots, and domain-delegated capabilities.

Every capability record contains, either directly or through owner-engine
metadata:

- object id.
- object generation.
- capability generation.
- rights mask.
- allowed byte/page/key range when the object is range-scoped.
- event mask and mapping permissions when relevant.
- transfer, seal, narrow, map, call, wait, and revoke permission bits.
- sealed bit and sealed-use rights.
- issuer domain id/generation.
- owner domain id/generation.
- lineage root id.
- lineage epoch.
- parent capability id/generation or `none` for root capabilities.
- optional revocation root id for batch revocation.

Rules:

- `CAP_DUP` creates a new capability in the same lineage. It may narrow rights,
  range, event mask, mapping permissions, transfer rights, and revoke rights; it
  cannot broaden any field.
- `CAP_SEND` preserves lineage across processes and domains. The sender must
  hold transfer authority, and the receiver's Resource Domain must allow the
  object class and rights.
- `CAP_RECV` installs the received capability with the same lineage root and
  epoch, a fresh local FDR slot generation, and any receiver-side narrowing
  required by domain policy.
- `CAP_SEAL` hides inspectability and ordinary delegation from software, but
  hardware keeps lineage metadata. Sealing cannot break revocation.
- `CAP_REVOKE` commits by advancing the lineage epoch or revocation-root epoch
  and emitting invalidation events to owner engines.
- operation issue checks object generation, capability generation, lineage
  epoch, rights, range, domain scope, and object-specific state before any
  side effect.
- cached FDR entries, TLB/PTE mappings, event-source bindings, call-gate
  continuations, classifier table entries, IOMMU contexts, DMA descriptors, and
  page-fill continuations carry enough generation/epoch bits to reject stale
  use without consulting stale software state.

Revocation classes:

| Class | Use | Commit action | In-flight behavior |
| --- | --- | --- | --- |
| `lazy_epoch` | cached descriptors, event bindings, classifier tables, namespace handles, endpoint readiness | advance lineage/revocation-root epoch and emit compact invalidations | existing committed records may be drained as data; new authority checks fail |
| `forced_cancel` | waits, pending page fills, queued async operations, not-yet-entered call gates, pending control operations | advance epoch and deliver cancel/revoke events | pre-commit work aborts with `EREVOKED`, `ECANCELED`, or object-specific stale status |
| `synchronous_quiesce` | DMA buffers, IOMMU contexts, BAR mappings, pages before reuse, domain freeze/teardown | block new work, advance epoch, wait/cancel in-flight users, acknowledge quiescence | backing memory/device authority is not reused until quiesce completes |
| `poison_fault` | corrupted metadata, untrusted stale state, failed local reset, integrity violation | mark object/page/descriptor poisoned and emit structured fault | all future use fails until supervisor/PID 1 policy clears or destroys it |

Object-specific revocation policy:

| Object or derived use | Revocation class | Required behavior |
| --- | --- | --- |
| FDR cache entry | `lazy_epoch` | mark revoked or generation-mismatched; cached issue checks fail without DDR table trust. |
| dynamic FDR table entry | `lazy_epoch` or `poison_fault` | advance slot/capability generation; poisoned entries cannot be recycled without acknowledgement. |
| VMA mapping | `synchronous_quiesce` for page reuse, otherwise `lazy_epoch` | enter `REVOKING`, reject new faults and pins, shoot down TLB/I-cache, release backing only after pins/fills settle. |
| object-backed page fill | `forced_cancel` | cancel if before page-install commit; reject stale replies by generation/lineage. |
| resident page with DMA pin | `synchronous_quiesce` | block new pins, wait/cancel descriptors, then permit page reuse or unmap. |
| event-source binding | `lazy_epoch` plus wake | detach source and enqueue revoke/overflow record where policy requests it. |
| parked `AWAIT` waiter | `forced_cancel` | wake with revoke/error event if its wait source was revoked. |
| call gate | `forced_cancel` for not-entered calls; target policy for entered calls | reject new calls; abort queued calls; calls past entry commit return, fault, or follow domain teardown policy. |
| `CALL_CAP` continuation | `forced_cancel` | missing or revoked continuation resumes caller with revoke/error status or emits a fault event. |
| DMA buffer / IOMMU context | `synchronous_quiesce` | reject new descriptors, quiesce/cancel accepted descriptors, tear down IOMMU mappings before backing memory reuse. |
| PCIe BAR mapping | `synchronous_quiesce` | reject new `MMAP`, invalidate PTEs, drain ordered device accesses before BAR authority is recycled. |
| packet queue / endpoint / listener | `lazy_epoch` plus optional drain | reject new sends/receives after epoch change; queued records may drain as data only if policy permits. |
| classifier table | `lazy_epoch` | stop matching new records; in-flight routed records remain data with source generation. |
| namespace root / service capability | `lazy_epoch` | reject new `OPEN_AT`/`NS_CTL` dispatches; pending dispatches validate epoch before returned-capability install. |
| Resource Domain subtree | `synchronous_quiesce` for teardown/freeze, `lazy_epoch` for delegated roots | block new dispatch/capability use, park/cancel descendants as policy requires, roll revocation through delegated roots. |

General commit rule:

- `CAP_REVOKE` commits when the relevant lineage or revocation-root epoch is
  advanced and owner engines have accepted the invalidation command.
- Before an operation's documented commit point, revocation aborts it with
  `EREVOKED`, `ECANCELED`, or the object-specific stale-reference error.
- After the commit point, the operation completes, rolls forward, drains as data,
  or follows the object's teardown policy, but any later authority check observes
  the stale generation/epoch.
- Authority-bearing returned capabilities are never installed from a reply whose
  source lineage changed before the Capability Engine commit.

This prevents each subsystem from inventing its own revocation behavior. The
local implementation may optimize cache invalidation, but it must refine this
single lineage/epoch model.

### 12.3 Typed Control and Metadata Envelope

`GET_META`, `SET_META`, `OBJECT_CTL`, `DOMAIN_CTL`, `NS_CTL`, source-level
`EVENT_CTL`/`TIMER_CTL`, network/socket options, storage barriers, and
service-owned controls all use a common typed control envelope. This prevents
the control plane from becoming an untyped `ioctl` tunnel.

The envelope is a bounded, typed, authority-checked transaction format. It is
not an opaque command blob. Every operation must name its object class, profile,
operation id, required rights, expected generation/lineage, bounded
input/output sizes, scalar fields, capability inputs, and returned-capability
slots before dispatch.

V1 control envelope:

```text
u16 version
u16 envelope_len
u16 object_class
u16 profile_class
u16 profile_id
u16 op
u16 flags
u32 rights_required
u32 input_len
u32 output_len
u16 scalar_count
u16 cap_arg_count
u16 ret_cap_count
u16 reserved
u64 expected_object_generation
u64 expected_lineage_epoch
u64 input_ptr
u64 output_ptr
u64 cap_arg_table_ptr
u64 ret_cap_table_ptr
u64 scalar0
u64 scalar1
u64 scalar2
u64 scalar3
```

Capability arguments are not embedded as raw integers. They are supplied through
explicit FDR operands, `CAP_SEND`/`CAP_RECV`, or a bounded side table named by
the envelope and validated by the Capability/FDR Engine. Capability argument
tables contain descriptor indices plus requested rights/masks; they do not
contain raw object ids that become authority by being copied. Returned authority
uses explicit returned-capability slots named by `ret_cap_table_ptr` and
`ret_cap_count`. Those slots are proposals until the Capability Engine verifies
object id, generation, rights, lineage, range, destination descriptor policy,
and Resource Domain policy, then commits the FDR table update.

Profile classes:

- `profile_class=architectural`: stable hardware profiles for domains,
  capabilities, queues, counters, memory objects, telemetry, attestation,
  storage barriers, classifier tables, and scheduler/domain metadata. These are
  frozen by the ISA profile and are suitable for formal proof.
- `profile_class=personality_service`: POSIX/Linux/BSD compatibility controls,
  namespace-service extensions, socket compatibility options, loader controls,
  and service-owned metadata. These may evolve by profile version but remain
  bounded by the envelope and capability-return discipline.
- `profile_class=vendor_device`: device-specific controls behind explicit device
  capabilities. These may be vendor-defined, but they cannot bypass object
  class, rights, range, generation, lineage, domain policy, bounded lengths, or
  returned-capability verification.

Profile-class rules:

- architectural profiles are part of the ISA profile and must have public,
  versioned records.
- personality/service profiles must be delegated by a namespace, service,
  supervisor, loader, socket, or compatibility capability.
- vendor/device profiles require an explicit device or vendor-control
  capability; holding a generic file, queue, or memory-object FDR is not enough.
- profile class is part of dispatch identity. The same `profile_id/op` in two
  classes is not the same operation.
- profile class cannot be inferred from payload bytes or service identity.

Versioning rules:

- `version=1` is the frozen v1 envelope layout.
- unknown envelope versions return `EINVAL` before reading payload buffers or
  capability argument tables.
- a supported envelope version with unknown `profile_class`,
  `object_class/profile_id`, or `op` returns `ENOTSUP` when the envelope is
  otherwise well-formed.
- flags are opt-in. Unknown flags return `EINVAL` before side effects.
- future profile versions may append fields only through bounded input records or
  profile-specific payloads; the common header layout remains stable for v1
  dispatch.
- a sender must set all reserved fields to zero; nonzero reserved fields return
  `EINVAL`.

Payload bounds:

- v1 envelope header size is fixed by `envelope_len`; shorter headers fail
  before side effects.
- `scalar_count`, `cap_arg_count`, `ret_cap_count`, `input_len`, and
  `output_len` are bounded by both global implementation limits and the selected
  object/profile/op.
- input/output buffers are copied for small records or pinned by bounded
  descriptors for larger records. Services never receive ambient user virtual
  pointers.
- pinned buffers carry VMA generation, permissions, memory type, direction, and
  cancel token. Revoke, unmap, protection change, or fatal signal before commit
  cancels the control operation.
- oversized payloads return `EOVERFLOW` when the shape is valid but exceeds a
  documented limit; malformed lengths return `EINVAL`.

Authority effects:

- scalar fields and payload bytes are never authority.
- capability arguments are FDR references plus requested masks; the
  Capability/FDR Engine resolves and validates them before dispatch.
- returned-capability slots are proposals until the Capability Engine validates
  object class, rights, range, generation, lineage, receiver domain policy, and
  destination FDR policy.
- returned capabilities cannot be hidden in payload bytes, scalar fields, status
  codes, trace records, or backend-defined data.
- service-owned controls may choose objects and propose returned capabilities,
  but they cannot install authority directly.

Commit and cancellation rules:

- every control op defines exactly one architectural commit point.
- before commit, cancellation, signal interruption, domain teardown, or
  revocation aborts the operation and releases reservations.
- after commit, the operation completes, rolls forward, drains as data, or uses
  the object's documented teardown policy.
- returned-capability install is a separate Capability Engine commit. If service
  work commits but returned-capability install fails, the operation reports the
  capability-install failure and must not publish a broader substitute.
- `GET_META` is normally side-effect-free except for explicit read/clear counter
  profiles.
- `SET_META`, `OBJECT_CTL`, `DOMAIN_CTL`, and `NS_CTL` must document whether
  their commit point is header validation, owner-engine state publication,
  service reply validation, or returned-capability install.

Common validation rules:

- unknown `version`, malformed `envelope_len`, or invalid reserved bits return
  `EINVAL`.
- unknown `profile_class`, `object_class/profile_id`, or `op` for a well-formed
  envelope returns `ENOTSUP`.
- unsupported flags return `EINVAL`.
- missing rights, failed credential checks, or denied Resource Domain policy
  return `EPERM` or `EACCES`.
- stale `expected_object_generation` or `expected_lineage_epoch` returns
  `EREVOKED` or the object-specific stale-reference error.
- `input_len`, `output_len`, scalar count, and capability argument count must be
  bounded by the object profile before any user buffer is pinned.
- returned-capability count must be bounded by the object profile and receiver
  FDR table policy.
- hardware validates and pins user buffers before dispatch; service-owned
  objects receive bounded copied records or pinned-buffer descriptors, not raw
  ambient pointers.
- control operations have a documented commit point and use the common
  cancellation/revocation rules.
- no control operation may broaden authority. It can only return a new or
  changed FDR through the verified capability-return path.
- hardware-owned objects implement fixed op ids. Service-owned objects may
  define profile-specific op ids, but they still receive this envelope and
  cannot bypass capability, generation, domain, or length checks.
- backend-defined payload bytes are data only. They may configure a
  service/device operation after common validation, but they cannot encode
  ambient authority, unbounded pointers, hidden returned capabilities, or
  executable policy.
- profile versions must preserve fail-closed behavior: unknown profile versions,
  operations, flags, or returned-capability shapes fail before side effects.

Error convention:

- `EINVAL`: malformed envelope, bad length, unsupported flag combination, or
  invalid scalar shape.
- `ENOTSUP`: well-formed but unsupported profile class, object/profile, or op.
- `EPERM`/`EACCES`: authority, credential, or domain-policy denial.
- `EBADF`: invalid FDR operand.
- `EREVOKED` or object-specific stale-reference error: generation/lineage epoch
  mismatch.
- `EFAULT`: unreadable input buffer, unwritable output buffer, invalid pinned
  range, or user memory fault during pre-commit copying/pinning.
- `EOVERFLOW`: valid envelope shape exceeds an implementation or profile limit.
- `EBUSY`: operation cannot commit because a required object is quiescing,
  frozen, pinned, or in a conflicting committed operation.
- `ECANCELED`: operation was canceled before commit by signal, teardown,
  revocation, or explicit cancel policy.
- object-specific errors are allowed only after common envelope validation has
  succeeded.

### 12.4 Service Domain Transaction Model

Service domains are the only place v1 intentionally leaves evolving policy in
software. Filesystem formats, namespace rules, loader policy, TCP/IP, Wi-Fi,
PCIe quirks, device management, Unix personality semantics, and synthetic
metadata live in services. Hardware still owns the service boundary.

A service domain is a process or Resource Domain that holds explicit service
capabilities and receives requests through one or more bounded hardware-visible
endpoints:

- call gates for low-latency request/return.
- queue objects for asynchronous request/reply records.
- event queues for readiness, completion, pressure, and fault notification.
- namespace dispatch continuations from `OPEN_AT` and `NS_CTL`.
- typed control-envelope dispatch from `GET_META`, `SET_META`, `OBJECT_CTL`,
  `DOMAIN_CTL`, socket/storage profiles, and service-owned metadata controls.
- object-backed page-fill requests from the VMA/Page Engine.
- `PULL`/`PUSH` stream endpoints for data-plane services.

Every dispatched service request record includes the minimum hardware context
needed for safe completion:

- request id and continuation id.
- caller PID/TID and Resource Domain id/generation.
- target object id/generation and lineage epoch.
- requested rights, operation/profile id, flags, and nonblocking/wait policy.
- bounded copied input bytes or pinned-buffer descriptors.
- explicit capability argument table.
- expected returned-capability shape and destination FDR policy.
- timeout/cancellation token when the profile is interruptible.

Services never receive ambient pointers, ambient physical addresses, ambient
device access, raw interrupt vectors, or hidden authority. Pinned-buffer
descriptors are valid only for the named request, range, rights, memory type,
and generation; they are revoked automatically on cancellation, domain teardown,
or lineage mismatch.

Service replies are validated in two phases:

1. Reply-shape validation checks request id, continuation id, service domain
   generation, output length, status code, copied output shape, and profile
   version.
2. Returned-capability install checks each proposed FDR against an existing
   mint/root capability held by the service, object class, object id/range,
   rights, memory type, event mask, mapping permissions, transfer/seal/narrow
   flags, object generation, lineage epoch, receiver domain policy, and
   destination FDR table policy.

Until both phases commit, service output is data only. A service cannot mint an
FDR by encoding an integer, pointer, object id, trace record, or backend payload
field. If service work has committed but returned-capability install fails,
hardware reports the install failure and publishes no substitute authority.

Service crash and cancellation rules:

- before the service transaction commit point, service death, service-domain
  freeze, caller signal interruption, caller/domain teardown, queue cancellation,
  or revocation aborts the request and wakes the caller with `ECANCELED`,
  `EINTR`, `EPIPE`, `EREVOKED`, or a profile-specific stale-service error.
- after the commit point, already committed data is either visible, drained, or
  rolled forward according to the object profile. The service cannot be asked to
  undo an operation after its architectural commit point unless the profile
  explicitly defines compensation.
- namespace mutations, storage metadata changes, endpoint creation, loader
  exec-plan publication, and capability-return commits must each name their
  commit point.
- services that restart receive a new service generation. Pending continuations
  carrying the old service generation cannot complete successfully.

Backpressure rules:

- request queues, reply queues, page-fill windows, event queues, stream buffers,
  and call-gate continuation slots are bounded and charged to Resource Domains.
- when capacity is exhausted, blocking profiles park the caller on a waitable
  capacity event, nonblocking profiles return `EAGAIN`, and profiles that cannot
  wait return `EOVERFLOW`.
- pressure events are generated through normal event queues/telemetry FDRs;
  there is no hidden global service scheduler or emergency allocation path.

Blessed service shapes are namespace/filesystem services, block-image/storage
services, loader/exec-plan services, network endpoint services, PCIe Bus Master
and driver services, telemetry/attestation services, and supervisor/personality
services. Forbidden service shapes are ambient privileged daemons, untyped
authority-bearing `ioctl` blobs, raw pointers, raw interrupts, raw DMA, raw
physical memory, unbounded hardware walkers, or service-owned capability table
writes.

`EVENT_CTL`:

- is a reserved/source-level profile alias over `OBJECT_CTL`.
- creates or modifies an `event_queue` profile implemented as a `queue` of event
  records plus source bindings.
- adds/removes source FDRs, sets edge-triggered or level-triggered semantics,
  and arms one-shot events.
- source assembly may keep `EVENT_CTL` for clarity, but the architectural hard
  primitive is `OBJECT_CTL`.

`TIMER_CTL`:

- is a reserved/source-level profile alias over `OBJECT_CTL`.
- creates or modifies a `timer` profile implemented as a `counter`/waitable
  object driven by monotonic or realtime hardware time.
- supports one-shot and periodic timers.
- source assembly may keep `TIMER_CTL` for clarity, but the architectural hard
  primitive is `OBJECT_CTL`.

`OBJECT_CTL`:

- uses F9.
- consumes the typed control envelope from Section 12.3.
- creates, configures, queries, resets, or destroys generic hardware-owned
  waitable/capability objects.
- covers only three primitive hardware classes: `counter`, `queue`, and
  `memory_object`.
- higher-level names such as semaphore, completion, event counter, channel,
  task queue, shared arena, and DMA completion are runtime profiles over those
  three classes.
- returns a new FDR index, object state, operation id, or `-1` with
  thread-local `ERRNO`.
- cannot grant authority beyond the caller's existing capabilities and process
  capability bits.

`DMA_CTL`:

- uses F9.
- submits bulk memory/object operations to the DMA Fabric.
- covers memory-to-memory copy, fill, scatter/gather copy, and optional
  checksum/hash profiles.
- may complete synchronously for small operations or through an `event_queue` FDR
  or `counter` completion profile for long operations.
- obeys the VMA/Page Engine's pinning, protection, memory type, and coherence
  rules.

`DOMAIN_CTL`:

- uses F9.
- consumes the typed control envelope from Section 12.3 plus a domain-profile
  payload when needed.
- creates, configures, queries, freezes, resumes, or destroys nested Resource
  Domains.
- attaches or detaches process/thread subtrees where permitted.
- delegates, narrows, or revokes FDR capabilities and device authority for a
  child domain.
- configures scheduler, memory, PID/thread, FDR, I/O, device, and event limits.
- configures upcall policy for virtualization, resource pressure, limit
  violations, namespace delegation, memory-map events, and lifecycle events.
- returns a domain id, status, usage snapshot size, operation id, or `-1` with
  thread-local `ERRNO`.
- cannot grant authority or budget not already held by the caller's domain.

`SUPERVISOR_CTL`:

- uses F9.
- is retained as a narrower source-level alias/profile over `DOMAIN_CTL` for
  delegated supervisor/upcall domains.
- installs upcall policy for opcode events, namespace delegation, permission
  decisions, process lifecycle events, and memory map events.
- creates domain control FDRs and event queues.
- cannot grant authority outside the caller's own capabilities.

## 13. Namespace Dispatch and Capability Return Engine

The Namespace Dispatch and Capability Return Engine does not implement a full
writable filesystem, inode model, symlink policy, or hardware directory walker.
Its job is to mediate name/control requests as authority-bearing transactions:
validate who may ask, dispatch the request to the namespace or filesystem
service that owns the policy, and verify any returned capability before
installing it in the caller's FDR table.

Inputs:

- process cwd/root namespace capability pointers.
- directory or namespace FDR operand.
- path/control argument virtual address.
- operation type.
- requested rights and flags.
- credential snapshot from PCRs.
- caller Resource Domain id/generation.

Internal units:

- path/control buffer pinning and bounded copy/slice descriptor generation.
- namespace capability validator.
- service endpoint/generation validator.
- request record formatter.
- call-gate/event-queue dispatch to the namespace service domain.
- caller park/reply-continuation tracker.
- returned-capability verifier and narrower.
- FDR table install/update path.
- optional service-approved lookup cache.

`OPEN_AT` flow:

1. validate that the caller holds a directory, namespace-root, or lookup-context
   capability.
2. validate path length, component count limit, requested rights, domain policy,
   and service generation.
3. pin or copy the path slice into a bounded request record.
4. dispatch the request to the namespace/filesystem service through a service
   queue or call gate.
5. park the caller in the hardware scheduler.
6. on reply, treat any returned capability as a proposal until the Capability
   Engine verifies that it derives from the service's delegated namespace/object
   root.
7. narrow returned rights to the caller's request, namespace cap, and domain
   policy.
8. install the FDR entry or return the service error.

`NS_CTL`, `GET_META`, and `SET_META` use the same transaction model for
service-owned objects. Hardware validates the authority envelope and reply
capability/status; service domains own rename, link, unlink, symlink, chmod,
chown, overlay, procfs/sysfs-like synthetic nodes, network filesystems, and
crash-recovery policy.

Hardware may cache only service-approved lookup results:

- cache key: namespace service id/generation, root/dir token, name hash or
  bounded path digest, operation subset, and requested rights subset.
- cache value: sealed capability template plus generation and invalidation
  token.
- cache entries are created by the namespace service, not by hardware directory
  parsing.
- cache hits are still narrowed by caller rights and domain policy.
- revocation or generation mismatch invalidates the entry.

Directory handling is a service/object profile. A directory may be represented
as:

- a `directory_stream` returned by the namespace service.
- a service-owned object where `PULL` returns ABI dirent records.
- a memory/block-backed object where the service grants direct read access.

The architecture keeps `OPEN_AT` native as a hardware-mediated capability
transaction, but it deliberately avoids making path semantics or writable
filesystem policy part of the silicon contract.

## 14. Device Backends

### 14.1 UART

UART exposes character stream objects:

- `fd0`: stdin receive FIFO.
- `fd1`: stdout transmit FIFO.
- `fd2`: stderr transmit FIFO.

`PULL` from UART blocks if no data is available unless nonblocking flags are
set. `PUSH` writes bytes into the transmit FIFO and parks the thread if the FIFO
is full.

### 14.2 SD Card

The SD adapter provides block storage and boot image access. Hardware exposes
block-device and block-image capabilities; filesystem services decide how to
interpret bytes as extents, inodes, directories, overlays, logs, or guest
filesystem images.

The File/Block Operation Engine translates explicit-offset `PULL`/`PUSH` and
DMA requests on block objects into SD block commands. It does not understand
general writable filesystem metadata.

### 14.3 Boot Image and Filesystem Service Model

The v1 hardware does not require a native writable filesystem format. Boot and
runtime storage are split:

- **Boot image format:** a simple manifest-indexed object table used by reset
  logic to locate PID 1, initial service binaries, measured images, and initial
  FDR grants by offset/length/hash.
- **Filesystem service domains:** software services that implement path
  semantics, writable metadata, crash recovery, overlays, synthetic trees, and
  imported Unix filesystems.
- **Block-image FDRs:** explicit-offset storage capabilities used by guest
  Linux/NetBSD filesystems, filesystem services, or applications that want
  block-like persistence.

Boot image v1 requirements:

- fixed-endian manifest header.
- image records with offset, length, type, hash, and permissions.
- initial process records for PID 1 and optional service domains such as
  namespace service, filesystem service, PCIe Bus Master, and network service.
- initial FDR grant records.
- measurement records exposed through boot-control FDR/`ENV_GET`.
- no path walking required by reset logic.

Filesystem service responsibilities:

- path lookup, symlinks, links, rename, unlink, mkdir, chmod/chown, timestamps,
  directory iteration, overlays, mounts/delegations, procfs/sysfs-like synthetic
  trees, and policy.
- journaling, copy-on-write, append-log replay, fsck, or service-specific crash
  recovery.
- returning object capabilities to callers through the hardware namespace
  dispatch reply path.
- granting direct data-path objects where possible, such as memory objects,
  block-file extents, directory streams, or service streams.

Crash recovery requirement:

- Live object commit points are not sufficient for power-fail safety.
- Each filesystem service must define its own journaling, copy-on-write, or
  append-log protocol for writable metadata.
- Atomic rename, link/unlink, chmod/chown, directory creation, symlink creation,
  and allocation changes are software-service semantics, not hardware FSMs.
- Storage write barriers must order service metadata commits against
  SD/SPI/PCIe block-device flush completion.
- Hardware block objects expose flush/barrier completion and fault events; they
  do not prove filesystem-level recovery by themselves.

Minimal FPGA v1 storage durability contract for block/storage objects:

- `PUSH`/`SET_META` variants can request `sync_data`, `barrier_after_commit`,
  or backend flush semantics on block/storage objects.
- `GET_META` exposes dirty/committed/error state for block/storage objects.
- `PUSH` to a regular file may complete before media persistence unless the fd
  or operation requests synchronous data semantics through its owning service.
- a storage barrier completes only after prior data writes, metadata log writes,
  and backend flush commands have reached the documented persistence point.
- `FENCE` orders CPU/cache/DMA visibility; storage durability requires the
  explicit storage barrier or synchronous metadata/data flag.
- after reset, filesystem services must replay or reject their writable storage
  before exposing a namespace root.

### 14.4 SPI Flash

SPI flash is used for boot ROM assets and optional read-mostly files. It exposes
a block-like backend with slower writes. The boot path may fetch manifest
records, initial service images, and executable image records from SPI flash if
SD is absent.

### 14.5 Native Networking Substrate

Networking follows the general LNP64 rule: silicon owns safety, movement,
queues, events, and isolation; software domains own protocol policy, driver
quirks, and evolving network semantics. V1 freezes a TCP-friendly transport
substrate, not TCP/IP itself.

FPGA v1 Ethernet is a simplified packet device, not a full TCP/IP offload
engine. PCIe Ethernet and Wi-Fi use the same substrate through the PCIe Bus
Master, IOMMU, BAR, DMA-buffer, and IRQ-event capability path.

Native network object profiles:

- `net_namespace`: delegated network universe for a Resource Domain. It scopes
  visible interfaces, addresses, routes, port binding authority, raw-packet
  authority, packet filters, quotas, and network policy roots.
- `net_interface`: capability to a physical, PCIe, or virtual interface. It
  exposes link state, MTU, hardware address metadata, queue creation, offload
  capabilities, counters, and fault state.
- `packet_queue`: raw or filtered packet ingress/egress queue. It is used by
  driver domains, native network service domains, packet capture tools,
  virtual switches, DPDK-like runtimes, and paravirtual Linux/NetBSD stacks.
- `datagram_endpoint`: message-oriented endpoint profile for UDP-like traffic,
  raw datagram protocols, local datagrams, or QUIC-friendly flows. It is an
  endpoint object shape, not a hardware UDP state machine.
- `stream_endpoint`: ordered byte-stream endpoint profile for TCP-like
  connections, local streams, service-provided secure streams, QUIC streams,
  paravirtual transports, or a future optional transport accelerator. It is not
  a hardware TCP promise.
- `listener`: passive accept queue; `PULL(listener)` returns a new endpoint
  capability.

These are object profiles, not independent hardware modules. They are
implemented by the Namespace/Object Engine, generic queues, event queues, DMA
Fabric, driver domains, and network service domains. POSIX sockets are a
libc/personality profile over these objects.

Endpoint contract:

- `packet_queue` preserves packet/record boundaries. `PULL` returns one or more
  packet envelopes plus payload references or copied bytes according to queue
  policy. `PUSH` submits one or more packet envelopes. Ordering is per queue;
  multi-queue steering may reorder across queues by explicit policy.
- `datagram_endpoint` preserves message boundaries. Each successful `PULL`
  returns exactly one datagram record unless a batch flag is used. Datagram
  delivery, loss, source metadata, checksum status, and truncation behavior are
  endpoint-profile metadata, not hardwired UDP semantics.
- `stream_endpoint` exposes an ordered byte stream with backpressure. `PULL`
  returns available bytes, `PUSH` appends bytes in order, and readiness reports
  readable, writable, half-closed, reset, error, and quota-pressure states. It
  does not expose packet boundaries and does not imply hardware TCP.
- `listener` is a queue of accepted endpoint capabilities. `PULL(listener)`
  returns a new `stream_endpoint` or profile-compatible endpoint FDR whose
  rights, namespace, accounting domain, and telemetry scope are derived from the
  listener and accepting service policy.
- endpoint capabilities carry object id/generation, namespace lineage, rights,
  queue/event ids, accounting domain, readiness generation, and optional
  transport-service id. Revocation invalidates readiness bindings and queued
  completion records by generation.
- `GET_META`/`SET_META`/`OBJECT_CTL` expose bind, connect, listen, shutdown,
  close/reset, nonblocking mode, buffer sizing, event binding, queue selection,
  transport-service selection, and socket-option compatibility through typed
  profiles. Unknown or unsupported options fail closed with `ENOTSUP` or
  `EINVAL`.

Silicon responsibilities:

- move packets between MAC/device FIFOs and DDR buffers through coherent DMA.
- enforce VMA permissions, `dma_buffer` rights, requester id, direction, object
  generation, and Resource Domain budget before accepting packet DMA.
- route device interrupts/MSI/MSI-X into `irq_event` FDR records; raw interrupt
  vectors are not exposed to drivers.
- provide generic queue/counter/event objects for RX, TX, completion, link
  state, and worker handoff.
- expose counters for packets, bytes, drops, checksum status, DMA faults,
  queue pressure, link changes, and device errors.
- support simple MAC/interface filtering, packet length checks, optional
  checksum assist, optional timestamping, and bounded classifier-driven
  hash/steering when cheap in FPGA resources.
- provide timer/counter/event primitives that transport services can use for
  retransmission, pacing, keepalive, and timeout policy without making those
  policies hardware semantics.
- support zero-copy handoff of packet buffers through memory-object or
  `dma_buffer` capabilities where ownership and generation checks permit it.
- publish packet descriptors in a stable packet-envelope format.

Silicon does not implement in v1:

- TCP state machines or congestion control.
- TCP retransmission, SACK/loss-recovery, pacing, keepalive, ECN, MPTCP, TCP
  Fast Open, or socket-option semantics.
- TLS, DNS, DHCP, routing, NAT, firewall policy, or service discovery.
- Wi-Fi scan, association, authentication, roaming, regulatory behavior, or
  power-management policy.
- BPF/eBPF-scale programmable packet processing.
- NIC-specific quirks, firmware protocols, or PCIe enumeration policy.

Reserved future transport accelerator profile:

- A future accelerator may assist or implement selected TCP mechanics only as a
  replaceable service behind ordinary `stream_endpoint`, `listener`,
  `packet_queue`, timer/counter, and event capabilities.
- The accelerator must not become the architectural network model. Software TCP,
  local IPC streams, QUIC services, paravirtual transports, and accelerated TCP
  all expose the same endpoint capability shapes to libc and applications.
- The frozen v1 substrate must remain sufficient for software transport stacks:
  packet queues, flow hashing/steering, checksum assist, timestamps,
  timer/counter objects, zero-copy DMA/memory-object handoff, readiness events,
  per-flow counters, and bounded classifier rules.

#### 14.5.1 Record Classification and Queue Steering

The networking classifier is a profile of a more general fixed-function block:
the Record Classification and Queue Steering Engine. Its job is to classify
small structured records, stamp metadata, count, and steer records into
capability-scoped queues. It is useful for packets, but also for IPC,
service-call completions, storage completions, DMA faults, trace records, RAS
events, and runtime task queues.

The engine accepts:

- a record envelope pointer or on-chip record.
- record profile id.
- owner Resource Domain id/generation.
- source object id/generation.
- capability-scoped classifier table id.
- destination queue capability set.

Supported v1 matching primitives:

- exact match.
- masked value match.
- prefix match for fixed-width fields.
- small range match.
- small enum/set match.
- flow/hash bits computed over selected fixed fields.

Supported v1 actions:

- pass to one queue.
- steer to one queue by hash/table.
- drop with counter.
- mark class id, priority, timestamp, flow hash, or software-needed flag.
- increment per-rule, per-domain, per-source, and per-destination counters.
- emit pressure/fault events on overflow or malformed records.

Networking packet profile:

- shallow parse only.
- recognizes simple Ethernet, optional VLAN, IPv4/IPv6 base headers, and simple
  TCP/UDP/SCTP/ICMP header positions when not fragmented and not hidden behind
  deep extension chains.
- may validate or assist checksums where cheap.
- computes flow hash over available 5-tuple fields.
- marks `parse_status = full`, `partial`, `unknown`, or `needs_software`.

Non-network profiles:

- event profile: classify scheduler, signal, timer, RAS, and supervisor events
  into control queues.
- IPC profile: route typed message records or call-gate completions by service
  id, method id, tenant/domain id, priority, or hash.
- storage/DMA profile: route completion records by object id, operation id,
  error class, or owning domain.
- trace profile: classify trace records into per-domain or per-engine readers.
- runtime profile: steer task/executor records into per-core or per-domain work
  queues.

Hard limits:

- no loops.
- no unbounded header walks.
- no arbitrary instruction VM.
- no mutable protocol state.
- no connection tracking.
- no routing/firewall policy language.
- no packet decryption/encryption.
- no Wi-Fi management state.
- table sizes, parse depth, extracted fields, and action count are bounded and
  reported through `GET_META`.

Classifier tables are capabilities. A process may install or update a table
only when it holds the source object/control capability and destination queue
capabilities. Delegating a classifier can narrow sources, destination queues,
match masks, action types, and counters, but cannot broaden them.

Packet envelope metadata:

```text
u32 version
u32 flags
u64 buffer_fd_or_object
u64 offset
u64 length
u64 ingress_interface
u64 timestamp
u64 checksum_status
u64 vlan_or_tag
u64 flow_hash
u64 reserved
```

The envelope is a software-visible record format used by packet queues and
network services. It is not a promise that hardware parses every protocol field;
fields can be unknown/zero when unsupported.

FPGA v1 MAC path:

- `PULL(packet_queue)` receives frames into user buffers or memory-object/DMA
  buffers according to queue policy.
- `PUSH(packet_queue)` transmits frames from user buffers or memory-object/DMA
  buffers.
- `AWAIT(packet_queue)` waits for RX ready, TX space, TX completion, link
  change, error, or quota pressure.
- `GET_META(net_interface)` reports link state, MTU, MAC address, counters, and
  supported offload bits.
- `SET_META(net_interface)` configures delegated filters and queue parameters
  where the caller holds authority.

Network service domains:

- own ARP/NDP, IP, TCP, UDP, routing, firewall/NAT policy, TLS integration,
  DNS/resolver policy, and POSIX socket compatibility.
- expose `stream_endpoint`, `datagram_endpoint`, and `listener` capabilities to
  applications.
- can delegate accepted connection capabilities to worker domains with
  `CAP_SEND`.
- can expose virtio-net-like queue capabilities to Linux/NetBSD personalities
  that want to run their own stack.

Typed networking boundary:

- hardware packet envelopes, endpoint readiness states, event records, queue
  accounting, and classifier outputs are stable architectural records.
- TCP, UDP, QUIC, TLS, DNS, DHCP, routing, NAT, firewall languages, congestion
  control, retransmission, pacing, socket option policy, and Wi-Fi management are
  service/personality policy above those records.
- applications and libc must not observe whether a `stream_endpoint` is backed by
  software TCP, local IPC, paravirtual transport, QUIC service, or a future TCP
  assist block except through declared metadata/offload feature bits.
- a future accelerator may accelerate transport mechanics only by implementing
  the endpoint contract and cannot introduce a second socket authority path.

Security rules:

- no ambient network namespace exists. A process/domain needs a `net_namespace`
  or narrower endpoint/interface capability.
- raw packet access requires explicit packet authority.
- privileged-port behavior is compatibility policy on top of namespace
  capability rules, not an ambient UID 0 shortcut.
- filters, endpoint rights, port binding authority, and queue access can be
  narrowed when delegated and cannot be broadened by children.
- revoking a namespace or interface authority revokes derived endpoints, packet
  queues, filters, and events according to capability lineage.

### 14.6 PCIe Host Support

PCIe support preserves the capability model by exposing devices as FDR
capabilities. The FPGA v1 hardware includes the pieces that must be in hardware
for safety and link operation, while PCIe enumeration and quirks are handled by
a trusted software Bus Master process.

Hardware responsibilities:

- PCIe Root Complex link management, transaction layer, and physical interface,
  likely using vendor FPGA IP.
- IOMMU / DMA remapper so PCIe devices can DMA only into buffers explicitly
  exported by the VMA/DMA engine.
- MSI/MSI-X event routing into `irq_event` FDRs.
- device-memory PTE attributes for BAR mappings.
- reset-time creation of a single Bus Master authority domain.

Bus Master responsibilities:

- enumerate PCIe bus/device/function topology.
- read and write config space through its privileged root-complex mapping.
- assign BARs and handle device quirks in software.
- configure IOMMU mappings and MSI/MSI-X vectors.
- request `pci_function`, `pcie_bar`, `dma_buffer`, and `irq_event` capability
  derivation from the PCIe root/function authority it holds.
- delegate installed FDRs to driver processes through normal capability
  passing.
- publish higher-level device FDRs such as block, network, GPU, or accelerator
  objects through namespace or device service domains.

Raw PCIe config and BAR access is granted only to the Bus Master at boot. Normal
applications never receive ambient MMIO authority. Driver processes receive only
the FDRs explicitly delegated to them.

`pcie_bar` FDRs:

- are pure capabilities; possession of a valid `pcie_bar` FDR is the authority
  to map that BAR range.
- are page-granular: BAR offset and length must be multiples of the system page
  size.
- carry device/function id, BAR number, page base, page count, allowed read/write
  permissions, memory type, and ordering domain.
- may use `device_ordered`, `uncached`, or `write_combining` VMA memory types.
- are mapped with `MMAP`; after mapping, driver code uses ordinary `LD` and
  `ST` instructions for doorbells and status registers.
- are never executable.

The VMA engine enforces `pcie_bar` authority only at `MMAP` time. It does not
perform sub-page bounds checks or FDR checks on every load/store. After mapping,
access control and memory type are represented by standard PTE bits.

`dma_buffer` FDRs:

- represent pinned, device-visible memory ranges.
- are exported to a specific PCIe requester id through the IOMMU.
- may be shared between a driver process and a device without exposing unrelated
  physical memory.
- must be revoked or quiesced before their backing pages are unmapped.

`irq_event` FDRs:

- receive MSI/MSI-X events as fixed-size records.
- support `AWAIT` for interrupt-driven drivers.
- may be delegated per-vector so a driver receives only interrupts for its
  assigned device/function.

This model deliberately avoids a large hardware PCIe enumerator or BAR command
parser. PCIe complexity and quirks live in one isolated Bus Master process, but
the rest of the system still sees devices as capability handles with stream,
memory, DMA, event, and control profiles.

PCIe Ethernet bring-up path:

- Bus Master enumerates the NIC and requests `pci_function`, `pcie_bar`,
  `dma_buffer`, and `irq_event` authority for a NIC driver domain; hardware
  derives and installs those FDRs from PCIe root/function authority.
- NIC driver maps BARs with `MMAP` and uses ordinary `LD`/`ST` for doorbells,
  status registers, and descriptor-ring control.
- descriptor rings and packet buffers are allocated/exported as `dma_buffer`
  or memory-object-backed capabilities with requester id, direction, range, and
  generation.
- MSI/MSI-X completions arrive as `irq_event` records; the driver does not own
  raw interrupt vectors.
- the driver publishes `net_interface`, `packet_queue`, and control/event FDRs
  to a network service domain.
- the network service domain publishes application-facing `stream_endpoint`,
  `datagram_endpoint`, and `listener` FDRs.

PCIe Wi-Fi bring-up path:

- uses the same BAR, DMA, and IRQ-event primitives as Ethernet.
- Wi-Fi firmware loading, device mailbox protocols, scan, association,
  authentication, WPA/WPA2/WPA3, roaming, regulatory policy, and power
  management live in a Wi-Fi driver/service domain.
- after association, the Wi-Fi service publishes an ordinary `net_interface`
  FDR and packet queues; the rest of the system does not need Wi-Fi-specific
  silicon.

`INB_RESERVED` and `OUTB_RESERVED`:

- are not general application or driver I/O instructions.
- exist only as optional fallback/debug hooks for boot firmware or the PCIe Bus
  Master holding root-control authority.
- raise `SIGILL` or supervisor opcode upcall if executed without that authority.
- should not be used for normal device drivers, which use FDR capabilities,
  `MMAP`-mapped BARs, DMA buffer FDRs, and IRQ event FDRs.

## 15. File and Directory Instructions

All file instructions are decoded into File Operation Engine commands.

The File Operation Engine is a stream transaction compiler, not a metadata
owner. It must not independently walk process, FDR, VMA, or namespace DDR
tables. It consumes semantic handles from owner engines:

- validated FDR/object capability from the FDR/Capability Engine.
- pinned or translated user buffer from the VMA/Page Engine.
- stream/object state token from the Namespace/Object Engine when needed.
- backend queue availability from UART, SD, SPI, Ethernet, PCIe driver, pipe,
  socket, event, timer, or control-FDR adapters.

Its local state is intentionally small:

- active stream offset/cookie window.
- short per-backend issue queues.
- DMA descriptor staging registers.
- packet/block/FIFO byte counters.
- completion op id and result-register tags.

Fast path target: cached FDR plus pinned buffer becomes one DMA/FIFO/packet
descriptor and one completion event. Directory reads are object/service profiles:
they may be direct if the service returned a direct directory stream object, or
they may dispatch `PULL` to the owning service object.

FDR operand conventions:

- `OPEN_AT`: F9. Dispatches a bounded path/name request relative to a
  directory/root/namespace FDR and installs a verified returned capability.
  Source-level `open`, `openat`, `opendir`, and older draft `OPEN_FD`/`OPEN_DIR`
  names lower to this opcode.
- `PULL`: F6/F9. Pulls records from a stream object into memory. It covers byte
  reads, directory entry reads, message receives, event reads, packet receives,
  and explicit-offset pread when an offset field is present in the argument
  block.
- `PUSH`: F6/F9. Pushes records from memory to a stream object. It covers byte
  writes, message sends, packet transmit, and explicit-offset pwrite.
- `SEEK`: F6. Updates or queries stream position. Directory rewind is
  `SEEK(fd, 0, SET)` on a directory stream.
- `GET_META`/`SET_META`: F6/F9. Reads or mutates metadata on an already-open
  object FDR.
- `NS_CTL`: F9. Dispatches namespace control requests relative to directory or
  namespace FDRs: `mkdirat`, `unlinkat`, `renameat`, `linkat`, `symlinkat`,
  `readlinkat`, `chdir`, delegation, mount/profile controls, and storage
  barrier profiles.
- `DUP`: uses F7/F9 as needed and always names an encoded result register. It
  may overwrite explicit destination descriptors only when the opcode variant
  says so.
- source-level `pipe()` lowers to `OBJECT_CTL create queue(profile=pipe)` plus
  capability narrowing into read and write endpoint FDRs. There is no separate
  v1 hardware `PIPE` primitive.
- Source assembly may omit `result_dst` for legacy readability; the assembler
  inserts `r1`, but the binary result register is always explicit.

`OPEN_AT`:

- validate directory/root/namespace FDR, requested rights, flags, and domain
  policy.
- read, pin, or bounded-copy the path string through the MMU.
- format a lookup/open request for the owning namespace service.
- park the caller until service reply.
- verify service authority, returned object id/generation, rights, and
  narrowing constraints.
- allocate or overwrite an FDR capability entry according to flags.
- return descriptor index or error in the encoded result register.

`PULL`:

- validate capability and read rights.
- interpret the object class/subtype to determine record shape: bytes, dirents,
  packets, messages, events, or backend-defined records.
- issue DMA to user virtual buffer through MMU.
- update stream position unless the argument block supplies explicit-offset
  mode.
- write byte/record count to the encoded result register.

`PUSH`:

- validate capability and write rights.
- DMA from user buffer to backend.
- update stream position unless the argument block supplies explicit-offset
  mode.
- write byte/record count to the encoded result register.

`SEEK`:

- validates that the object class supports positioning.
- supports byte-stream offsets and directory stream cookies.
- returns the resulting position in the encoded result register.

`GET_META` and `SET_META`:

- operate on object FDRs, not raw global paths.
- use the typed control envelope from Section 12.3 for all nontrivial metadata
  records.
- cover stat, chmod, chown, utime, fd flags, object rights queries, and
  backend-specific metadata.
- dispatch to the owning service for service-owned objects; hardware-owned
  objects use their owner engine.
- reject unknown metadata/control op ids with `ENOTSUP`, malformed envelopes
  with `EINVAL`, authority failures with `EPERM`/`EACCES`, and stale
  generation/lineage with `EREVOKED` or the object-specific stale-reference
  error.
- path-oriented source forms lower to `OPEN_AT` plus metadata operations where
  possible.

`NS_CTL`:

- dispatches operations that necessarily name directory entries: rename, unlink,
  mkdir, link, symlink, readlink, chdir, and delegated namespace controls.
- uses the typed control envelope from Section 12.3 for namespace operation
  records, including service-owned filesystem and mount/delegation controls.
- uses directory/root FDRs and name buffers rather than direct global `_PATH`
  opcodes.
- verifies that any returned capability/status stays inside the caller's
  namespace capability, requested rights, and Resource Domain policy.

The v1 stat buffer is a 104-byte little-endian record:

```text
0x00 mode      0x08 size      0x10 device    0x18 inode
0x20 mtim.sec  0x28 mtim.nsec 0x30 nlink     0x38 uid
0x40 gid       0x48 atim.sec  0x50 atim.nsec 0x58 ctim.sec
0x60 ctim.nsec
```

## 16. Process Engine

The Process Engine owns PID allocation, process table entries, parent-child
relationships, and process-wide resources.

The Process/Scheduler Engine must keep active process and thread windows on
chip. DDR process/thread tables are architectural backing storage and overflow,
not the normal scheduler hot path.

Local state:

- active PID/TID slots for runnable and recently parked threads.
- per-core ready queue heads/tails plus global balancing state.
- parent/child wait summaries for active processes.
- exec-barrier state and sibling-thread stop acknowledgements.
- active credential and namespace pointers consumed by issue.
- compact zombie/exit-status records for children with waiting parents.

Fast path target: `CLONE` of a thread-like context, `YIELD`, `EXIT` of a thread,
child-exit wakeup, timer/event wakeup, and scheduler dispatch operate from local
state unless they overflow the active window.

Each process entry contains:

- PID.
- parent PID.
- address-space root pointer.
- VMA tree root pointer.
- FDR table pointer.
- cwd object id.
- root namespace pointer.
- credential pointer containing uid/gid and capability bitmap.
- process-wide signal handler table pointer.
- child state queue.
- thread list.
- process state.

Each thread entry contains:

- TID.
- owning PID.
- PC, LR, GPR/FPR/VR state, flags, and stack pointer.
- thread-local `ERRNO`.
- thread-local signal mask and pending per-thread signal queue.
- blocked state and wait object.
- cancellation/operation id for any in-flight hardware command.
- join completion record or join-waiter list for same-process thread joins.

`CLONE`:

- uses F8 or F9 depending on flag complexity.
- creates a new thread or process according to an explicit clone profile and
  bounded share/copy flags.
- is the native primitive. POSIX `fork()` is only one constrained compatibility
  profile over it.
- never copies in-flight operation ownership, runtime locks, pending DMA
  descriptors, partially committed metadata operations, or hidden runtime state.
- never broadens capability authority; copied or inherited FDRs preserve
  generation, rights, close-on-exec/inherit policy, and Resource Domain scope.
- thread-like source forms lower to `CLONE profile=thread` with shared address
  space, shared process resources, a new TID, explicit entry PC, and explicit
  stack pointer or stack VMA.
- native process-like source forms lower to `CLONE profile=process` with an
  explicit choice of COW/shared/new address space, FDR inheritance policy,
  namespace/credential policy, signal policy, heap policy, and child waitable.
- fork-like source forms lower to `CLONE profile=posix_fork`: new PID, exactly
  one child thread, COW VMAs and heap metadata, inherited/narrowed FDR table
  entries according to descriptor flags, copied cwd/root namespace references,
  copied credentials, copied signal dispositions, child thread mask copied from
  the calling thread, per-thread pending signals cleared, and no in-flight
  operation ownership copied.
- writes child TID/PID to the parent result register and zero to the child
  result register when using fork-compatible variants.
- enqueues the child thread when creation commits.

`CLONE` v1 profiles:

- `thread`: same PID, new TID, shared address space, shared FDR table, shared
  credentials, shared signal disposition table, explicit stack/entry.
- `process`: new PID with explicit share/copy/new flags for address space, FDR
  table, namespace references, credentials, signal dispositions, and heap.
- `posix_fork`: constrained process profile for libc `fork()`, as described
  above.

`CLONE` v1 flags include:

- share address space.
- COW address space.
- new empty address space.
- share FDR table.
- inherit FDR table by descriptor inheritance flags.
- start with explicit FDR grants only.
- share cwd/root namespace.
- share credentials.
- share signal handler table.
- copy signal handler table.
- copy calling thread signal mask.
- clear child pending signals.
- create new PID.
- set entry PC from argument block.
- allocate new stack VMA or use supplied stack pointer.
- create child-exit waitable.

Historical `pthread_atfork` handler ordering, runtime lock recovery, and
language-runtime consistency are not hardware semantics. Libc may run atfork
handlers before issuing the `posix_fork` clone profile; hardware only provides
the atomic clone transition and defined inherited state.

`THREAD_JOIN`:

- uses F8: `a=result_dst`, `b=target_tid_reg`, `c=retval_ptr_reg`.
- waits on a same-process thread completion record.
- parks the caller in the scheduler rather than spinning when the target thread
  is still live.
- writes the target thread's exit value to `retval_ptr` when nonzero.
- returns `0` on success or a POSIX-style error code such as `ESRCH` or
  `EDEADLK`.

`EXEC`:

- validates the F9 argument block and copies/pins the bounded exec-plan
  descriptor before any irrevocable state replacement.
- enters a process-wide exec barrier.
- prevents new threads from being spawned in the process.
- stops all sibling TIDs at scheduling boundaries or via forced scheduler park.
- cancels or detaches in-flight operations according to the cancellation rules.
- invalidates sibling thread contexts so exactly one thread survives the exec.
- consumes a prepared exec-plan descriptor from user memory or from a trusted
  boot-manifest record.
- validates executable image, memory object, startup metadata, VMA, FDR, and
  domain/security capabilities named by the descriptor.
- rejects malformed descriptors, stale generations, stale lineage epochs,
  unauthorized executable provenance, W^X/NX violations, unaligned mappings,
  unsupported memory types, unauthorized startup FDR grants, and denied Resource
  Domain policy before the commit point.
- tears down old VMAs except preserved process resources.
- builds the new VMA set from descriptor records: executable image ranges,
  read-only data, read/write data, anonymous BSS, heap seed, stack, guard pages,
  TLS, and optional shared objects already authorized by the loader.
- copies mapped ranges from their source object capabilities into DDR through
  the DMA fabric or installs lazy object-backed mappings according to descriptor
  flags. Hardware follows the descriptor; it does not parse segment tables.
- resets PC, LR, SP, registers, thread-local `ERRNO`, and signal state as
  specified by LNP64 ABI.
- preserves PID, parent, cwd, selected FDRs, and credentials except for
  explicitly authorized domain/security deltas named by the descriptor.
- exits the exec barrier and enqueues the single surviving thread.

The `EXEC` commit point is the atomic publication of the new process image:
new VMA root, new FDR table view, new startup metadata pointer, new PC/SP/TLS
state, reset signal/thread state, and invalidated sibling thread contexts. If
validation, cancellation, revocation, signal interruption, descriptor copy,
object access, or policy checks fail before that point, hardware releases the
exec barrier and the old image continues with an error in `r_result`. After that
point the old image no longer exists; later page-fill, startup, or fetch faults
are faults of the new image and use the normal signal/termination path.

`EXEC` uses F9 with this v1 argument block:

```text
u32 version
u32 flags
u64 exec_plan_ptr
u64 exec_plan_len
u64 startup_metadata_ptr
u64 reserved
```

The exec-plan descriptor is not an executable file format. It is a bounded
architecture record produced by a loader service, libc runtime, Unix
personality, or boot manifest tool. It contains only hardware-visible commit
data:

- descriptor version, total byte length, bounded record counts, flags, expected
  domain generation, expected process generation, and expected lineage epoch.
- entry PC, initial SP, optional TLS base, and startup metadata pointer.
- VMA records with target virtual address, length, protection, memory type,
  executable provenance class, source object capability, source offset,
  file/object generation, lineage epoch, and zero-fill length.
- FDR preservation/close-on-exec policy and explicit startup FDR grants.
- domain/security policy deltas already authorized by the parent domain.
- image measurement/hash references for measured boot or audit records.

Hardware must not interpret:

- ELF, Mach-O, PE, WebAssembly, or other executable formats.
- dynamic linker state, symbol binding, PLT/GOT layout, or relocation records.
- shebang/interpreter policy, library search paths, or package policy.
- Unix credential transition rules such as setuid/setgid semantics.
- auxv layout beyond treating `startup_metadata_ptr` as an opaque pointer for
  the runtime/personality ABI.

All of that belongs in software loader services or compatibility personalities.
They parse real binary formats, apply relocations under W^X rules, build the
exec-plan descriptor, then ask hardware to perform the atomic process
replacement.

`AWAIT`:

- suspends the current thread until a waitable object's state changes.
- supports event queues, timers, child process state, fd readiness, IRQ events,
  supervisor upcalls, message channels, and futex predicates.
- for futex waits, the AWAIT argument block includes address and expected value;
  the Futex Engine atomically compares before parking.
- source-level waitpid, sleep, fd-readiness wait, timer wait, and blocking
  message-receive forms lower to `AWAIT` on the appropriate waitable object.

`ALARM`:

- uses F2: `a=result_dst`, `b=seconds_reg`.
- resets the calling process's POSIX alarm timer and returns the previous
  remaining whole seconds.
- when it expires, the timer wheel enqueues `SIGALRM` for the process and wakes
  a runnable thread in that process.
- `ALARM 0` cancels the outstanding POSIX alarm. General multi-source timers
  remain timer/event-queue profiles over waitable FDRs.

`EXIT`:

- marks current TID dead.
- if last thread in process, closes process resources, marks process zombie,
  stores exit status, and signals parent with `SIGCHLD`.

## 17. MMAP and MUNMAP

`MMAP` is a real hardware VMA operation.

`MMAP` uses F9 with this v1 argument block:

```text
u32 version
u32 flags
u64 hint_addr
u64 length
u64 prot_and_memory_type
u64 fd_index_or_all_ones
u64 file_or_bar_offset
u64 reserved
```

The VMA Engine:

- validates length, protection, fd rights, and offset.
- chooses an address if hint is zero.
- allocates a VMA descriptor in DDR.
- inserts it into the process VMA tree.
- marks anonymous pages `RESERVED`/zero-fill-on-demand.
- marks object-backed pages `NONRESIDENT_OBJECT` with object id, offset, rights,
  memory type, owner endpoint, and mapping generation.
- for `pcie_bar` FDRs, validates page-granular BAR bounds and installs device
  PTEs with the FDR's allowed permissions and memory type.
- for `dma_buffer` FDRs, maps pinned DMA-safe pages with normal cached or
  device-appropriate attributes as specified by the FDR.
- returns the virtual address in `r_dest`.

`MMAP` protection flags include:

- read, write, execute.
- private or shared.
- guard/no-access.
- requested memory type: `normal_cached`, `uncached`, `device_ordered`, or
  `write_combining`.

For normal files and anonymous memory, unsupported memory type requests fail
with `EINVAL`. For `pcie_bar` FDRs, the requested memory type must be permitted
by the FDR and the mapping must be page-aligned. No sub-page BAR capability is
architectural in v1.

For service-owned object mappings, `MMAP` installs only the hardware-visible
mapping envelope. On a nonresident fault, the VMA/Page Engine sends a fixed
page-fill request according to the Object-Backed Page Transaction Protocol in
Section 10.1. The owner returns a page capability, zero-fill decision, shared
memory-object page, retry/later token, or typed failure. Hardware atomically
installs the returned page only if the VMA generation, object generation,
lineage epoch, requested permissions, memory type, executable provenance, and
domain policy still match.

Hardware does not implement a general file page cache. Immutable or executable
image objects may use a small read-only generation-stable fill cache keyed by
object id, offset, rights, and generation. Writable object mappings, dirty
writeback, truncation, `msync`, and coherence with `PULL`/`PUSH` are service
semantics. Hardware may expose dirty-range enumeration and service callback
events, but it does not choose filesystem page-cache or writeback policy.

Security policy is enforced at VMA creation and permission-change time:

- anonymous mappings, heaps, stacks, queues, DMA buffers, shared-memory objects,
  device BARs, and signal-frame regions default NX.
- executable mappings must be backed by an executable image/object capability or
  by a Resource Domain policy that permits loader/JIT executable transitions.
- writable-plus-executable permission is rejected unless the current Resource
  Domain has an explicit JIT/loader policy bit.
- sanctioned JIT flow is writable mapping, write/patch code, `MPROTECT` to
  executable and non-writable, then `ISYNC`.
- guard VMAs are represented as no-access descriptors that intentionally fault
  on load, store, fetch, or DMA pin.
- ASLR address selection uses the Entropy and Randomization Engine unless the
  current Resource Domain policy disables or constrains randomization.

`MUNMAP`:

- finds intersecting VMAs.
- splits or removes VMA descriptors.
- decrements page refcounts.
- invalidates matching TLB entries for that process.
- revokes or generation-invalidates mapped object views when teardown removes
  the last authority-bearing VMA.
- writes success or error sentinel to the encoded result register and updates
  thread-local `ERRNO` on failure.

`MPROTECT`:

- finds existing VMAs covering the requested range.
- updates read/write/execute and sharing permission bits.
- applies W^X, NX default, JIT/loader policy, and capability-derived permission
  limits before publishing the new protections.
- invalidates matching TLB and instruction-cache entries where permissions
  require it.
- is required for software loaders, language runtimes, JIT policy, guard pages,
  and paravirtual Unix guests mapping their own process abstractions onto LNP64
  VMAs.

### 17.1 Frozen VMA/Page State Machine

The VMA/Page Engine is a fixed page-state machine. It is not a software memory
manager, page replacement daemon, swap policy engine, or filesystem cache. Each
PTE/page slot is always in exactly one architectural state:

| State | Meaning |
| --- | --- |
| `UNMAPPED` | No VMA/PTE covers the virtual page. |
| `RESERVED` | VMA exists but no physical page is committed yet. |
| `NONRESIDENT_OBJECT` | Object-backed mapping with object id, offset, rights, memory type, owner endpoint, and generation. |
| `FILL_PENDING` | Object-fill request is outstanding; faulting thread is parked or redirected. |
| `RESIDENT_CLEAN` | Page is resident and not dirty through this mapping. |
| `RESIDENT_DIRTY` | Page is resident and dirty through this mapping. |
| `COW_SHARED` | Page is resident and shared by COW references; write requires a COW break. |
| `PINNED_DMA` | Page has active DMA pins and cannot be reused or unmapped without pin release/cancel. |
| `REVOKING` | Mapping/object generation is being invalidated; no new fills or pins are accepted. |
| `POISONED` | Metadata or contents are known bad; access faults and emits a structured event. |

Normative transitions:

| Operation/event | Source states | Required checks | Target/commit |
| --- | --- | --- | --- |
| `MMAP` anonymous | none | domain VMA/page budget, address placement, protection policy | VMA created; pages start `RESERVED`. Commit is VMA publication. |
| `MMAP` object-backed | none | FDR rights, page-aligned range, memory type, domain policy | VMA created; pages start `NONRESIDENT_OBJECT`. Commit is VMA publication. |
| load fault | `RESERVED` | read permission, budget | zero-fill page and install `RESIDENT_CLEAN`. Commit is PTE install. |
| store fault | `RESERVED` | write permission, budget | zero-fill page and install `RESIDENT_DIRTY`. Commit is PTE install. |
| fetch fault | `RESERVED` | execute permission and provenance | install executable page only through loader/JIT-approved path. Commit is PTE install plus I-cache invalidation. |
| object load/fetch/store fault | `NONRESIDENT_OBJECT` | VMA/object generation, rights, lineage, executable provenance if needed | emit page-fill request and enter `FILL_PENDING`. Commit is request publication plus parked-thread state. |
| fill reply | `FILL_PENDING` | original VMA/object generation, lineage epoch, permissions, memory type, domain policy, page capability bounds | install page as `RESIDENT_CLEAN`, `RESIDENT_DIRTY`, or shared object page. Commit is validated PTE install. |
| fill retry/later | `FILL_PENDING` | retry token/event is bounded and tied to same mapping generation | remain parked or attach to retry event. No page commit. |
| fill error | `FILL_PENDING` | typed error from owner or validation failure | fault parked thread with `EFAULT`, `EIO`, `EAGAIN`, `EREVOKED`, `SIGBUS`, or object-specific status. |
| COW write fault | `COW_SHARED` | write permission, page budget, no conflicting pin/revoke | copy page and atomically swap PTE to `RESIDENT_DIRTY`. Commit is PTE swap. |
| clean write | `RESIDENT_CLEAN` | write permission | set dirty/accessed bits and transition to `RESIDENT_DIRTY`. Commit is dirty-bit publication. |
| `MPROTECT` | resident or nonresident | requested permissions do not exceed VMA/object/domain rights; W^X/NX/JIT policy | update VMA/PTE permission generation and invalidate translations. Commit is generation publication. |
| `MUNMAP` | any mapped state | range ownership, no unhandled mandatory pin | enter `REVOKING`, invalidate translations, release pages when safe. Commit is VMA generation invalidation. |
| revoke | any mapped state | lineage/object/domain revocation request | enter `REVOKING`, block new fills/pins, invalidate cached uses. Commit is generation/epoch advance. |
| truncate notice | object-backed states | notice generation matches object lineage | resident pages past new range become faulting/stale; pending fills cancel. Commit is object range generation update. |
| DMA pin | resident states | mapped permission, memory type, FDR/domain DMA authority, direction, no guard/poison/executable-only conflict | increment pin state or enter `PINNED_DMA`. Commit is pin count publication. |
| DMA unpin | `PINNED_DMA` | matching descriptor/domain/generation | decrement pin count; return to resident state or complete pending revoke. |
| metadata fault | any | ECC/parity fault classification | enter `POISONED` and emit `ras_fault`. Commit is poison bit publication. |
| domain teardown | any | domain generation invalidated | cancel pending work, block new pins/fills, invalidate VMAs, release pages after pins quiesce. |

Race priority is deterministic. If multiple events target the same mapping/page,
the VMA/Page Engine resolves them in this priority order:

1. metadata poison or fatal RAS fault.
2. domain teardown.
3. capability/object/domain revocation.
4. `MUNMAP`.
5. `MPROTECT`, truncation notice, or object generation change.
6. DMA pin/unpin lifecycle.
7. object-fill reply or retry.
8. ordinary load/store/fetch access.

An operation before its commit point may abort and report a typed failure. After
commit, later conflicts are represented by a new transition. A service reply,
page capability, or retry token is never authority until the VMA/Page Engine
validates it and reaches the page-install commit point.

TLB and instruction-cache rules:

- `MUNMAP`, revoke, domain teardown, and VMA generation changes invalidate all
  matching TLB entries before affected threads resume or backing memory is reused.
- `MPROTECT` invalidates data TLB entries for permission narrowing and both TLB
  and I-cache entries for executable permission changes.
- COW PTE swaps invalidate stale writable aliases before the writing thread
  resumes.
- object page install publishes the PTE before waking parked threads.
- writable-to-executable transitions require `MPROTECT` to executable,
  non-writable memory plus `ISYNC` before execution of patched code is legal.

DMA pin rules:

- DMA pinning is allowed only for resident pages whose VMA, FDR, Resource Domain,
  memory type, and direction permit the requested DMA operation.
- guard pages, unmapped pages, poisoned pages, executable-only pages, stale
  generation pages, and pages in `REVOKING` reject new pins.
- revoke and `MUNMAP` block new pins first, then wait for, cancel, or fault
  in-flight pins according to the DMA descriptor policy before reusing backing
  memory.
- non-coherent platform profiles must expose explicit DMA synchronization
  controls and cannot advertise the coherent-DMA feature bit.

Dirty/writeback rules:

- hardware owns accessed/dirty bits, dirty transitions, and dirty-range
  enumeration.
- `MSYNC`, storage ordering, sparse-file holes, append behavior, truncate
  persistence, and `PULL`/`PUSH` coherence are service/personality policy.
- hardware enforces permissions, generation, lineage, memory type, and storage
  barrier commit ordering, but does not choose filesystem writeback policy.

The VMA tree can be a hardware-walked B-tree or interval tree in DDR. For FPGA
v1, a sorted VMA array per process is acceptable if bounded and checked in
hardware.

The VMA/Page Engine must also have a clear local fast path:

- recent VMA range cache keyed by PID/ASID and virtual page range.
- active process VMA root pointer cached with the thread context window.
- small page-state classification cache for resident, COW, object-fill,
  guard-page, pinned, and revoking cases.
- buffer pinning window for in-flight DMA descriptors.
- TLB and I-cache invalidation queues with acknowledgement bits per tile.

Fast path target: TLB miss on a hot resident mapping, COW classification for a
recent VMA, DMA buffer pinning for a cached range, and range invalidation issue
do not require a full DDR VMA tree walk. DDR walks are refill/cold paths.

## 18. Hardware Heap Engine

`ALLOC`, `ALLOC_EX`, `ALLOC_SIZE`, and `FREE` are v1 architectural instructions
backed by the Hardware Heap Engine. They are the preferred userspace allocation
primitive. The ISA exposes allocation intent and stable policy hints, not the
allocator representation.

LNP64 freezes one default general-purpose allocator in hardware: the **LNP64
Default Heap Algorithm**. It is a domain-aware segregated bump allocator:

- fixed size-class dispatch for small and medium objects.
- per-thread allocation windows for the common bump-pointer fast path.
- domain-owned slab/run pages with compact bitmaps or encoded free-state.
- bounded transfer queues for batched cross-thread frees.
- page-run large objects through the VMA/Page Engine.
- checked metadata with generation fields, exact-pointer free, optional
  quarantine, guard hooks, zero/poison policy, and Resource Domain accounting.

This algorithm is intentionally close to the shape modern general-purpose
allocators converged on, but with hardware-owned safety and accounting
invariants. `malloc` implementations should lower to these instructions by
default and layer language/runtime policy above them. `MMAP` remains the
primitive for page mappings, files, shared memory, executable memory, DMA
buffers, and device mappings.

Heap backing VMAs are NX by default. Guarded allocations use VMA guard regions
or heap-local guard slots depending on size and policy. Heap metadata includes
generation fields, arena ids, size-class ids, allocation state, and optional
quarantine state so stale or freed pointers can be rejected by hardened profiles
before an allocation slot is reused silently.

The architecture distinguishes two allocation modes:

- **Hardware-owned allocations:** `ALLOC`, ordinary `ALLOC_EX`, `FREE`, and
  `ALLOC_SIZE` create and manage individual allocation objects. The Heap Engine
  owns object-level metadata, ownership transitions, invalid-free detection,
  stale-generation checks, quarantine policy, and per-allocation accounting.
- **Software-owned arenas:** `MMAP`, `memory_object`, and arena-style
  `ALLOC_EX` provide backing regions for runtime-specific allocators, GC heaps,
  bump allocators, database slabs, packet pools, or object pools. Hardware owns
  the outer VMA/capability/domain boundary, but software owns the inner object
  representation and inner correctness.

The guarantee is always at the granularity hardware owns. Hardware-owned
allocations get object-level safety and accounting. Software-owned arenas get
region-level safety and accounting; object-level bugs inside the arena are the
runtime's responsibility unless the runtime uses hardware-owned allocations for
those objects.

`ALLOC`:

- uses F2: `a=result_dst`, `b=size_reg`.
- allocates from the current process's default heap.
- returns a virtual pointer in `result_dst`, or `-1` with thread-local
  `ERRNO`.
- reports `ENOMEM` for domain/system memory pressure, `EINVAL` for invalid
  size, and `EPERM` if the domain policy disables heap allocation.
- returns memory aligned to at least 16 bytes.
- does not guarantee zeroed memory unless process heap policy says otherwise.
- does not expose size-class ids, freelist pointers, allocation-window depth, slab
  layout, quarantine state, or coalescing policy.

`FREE`:

- uses F2: `a=result_dst`, `b=ptr_reg`.
- frees an exact pointer previously returned by `ALLOC` or `ALLOC_EX`.
- returns `0` on success or `-1` with thread-local `ERRNO`.
- detects invalid pointers and double free when heap metadata is intact; v1
  returns `EINVAL` and may additionally deliver `SIGSEGV`, poison the arena, or
  emit a heap-corruption fault event according to the domain hardening profile.
- never accepts interior pointers, foreign-arena pointers, or memory from
  `MMAP`, `memory_object`, DMA, device, or executable mappings.

`ALLOC_SIZE`:

- uses F2: `a=result_dst`, `b=ptr_reg`.
- reads Heap Engine metadata for an exact allocation pointer.
- returns the allocation's usable byte extent in `result_dst`.
- returns `0` for null.
- returns `-1` with `ERRNO=EINVAL` for unknown, freed, interior, or foreign
  pointers.
- lets libc implement `realloc` without copying beyond the old allocation's
  valid mapped extent.

`ALLOC_EX`:

- uses F9 with an argument block for runtime-quality allocation requests.
- supports size, alignment, flags, memory type, allocation class/tag, arena id,
  locality hints, shared-memory eligibility, DMA/pinning eligibility, and
  debug/hardening hints.
- can request either a hardware-owned allocation object or an arena-style
  backing region profile, according to flags and domain policy.
- supports only bounded, enumerated allocation profiles within the frozen heap
  substrate. Allocation classes are tags and accounting/locality hints, not
  executable allocator plugins.
- returns `ENOMEM`, `EINVAL`, `EPERM`, or `EFAULT` for pressure, malformed
  requests, disallowed policy, or unreadable argument blocks.

`ALLOC_EX` v1 argument block:

```text
u32 version
u32 flags
u64 size
u64 alignment
u64 memory_type
u64 allocation_class
u64 arena_id
u64 locality_hint
u64 eligibility_flags
u64 reserved0
```

`ALLOC_EX` flags:

- zeroed.
- nozero.
- guard_before.
- guard_after.
- debug_poison.
- prefer_locality.
- large_object.
- arena_select.
- shared_eligible.
- dma_pin_eligible.
- no_quarantine.

Heap model:

- each process has a default heap created at process start.
- heap metadata lives in protected DDR and is not directly mapped writable by
  the process.
- heap arena bases and large-object mappings are randomized by default.
- small and medium allocations use the fixed size-class table from the LNP64
  Default Heap Algorithm. The implementation profile publishes coarse limits and
  feature bits through `ENV_GET`, but not raw freelist, allocation-window, or
  slab state.
- each size class is backed by slab/run pages with a bitmap or compact encoded
  free-state representation owned by the Heap Engine.
- hot allocation uses per-thread allocation windows: base, cursor, limit,
  size-class id, arena id/generation, and domain/accounting id. A window hit
  returns by bumping the cursor and publishing checked metadata without touching
  DDR heap metadata.
- hot free uses per-thread free/quarantine caches and bounded transfer queues.
- cross-thread frees enter bounded local transfer queues and are batch-drained
  into the owning arena, slab/run, or free cache.
- large allocations use page runs from anonymous VMAs and are tracked as large
  heap objects, not sub-page slabs.
- the Heap Engine serializes metadata updates and is thread-safe across all
  threads in the process.
- fork-like `CLONE` marks heap backing pages copy-on-write and clones heap
  metadata with COW semantics.
- `EXEC` destroys old heap metadata and creates a fresh default heap for the new
  image.
- `MUNMAP` of heap-owned pages is illegal unless mediated by the Heap Engine.
- shared memory, executable memory, DMA memory, and device memory are not
  allocated directly by `ALLOC`; use `MMAP` and FDR-backed objects for those
  cases. `ALLOC_EX` eligibility flags only request memory that may later be
  exported, shared, or pinned if domain policy and capability checks permit it.
- runtimes that need a private region allocator should request a `memory_object`
  or `ALLOC_EX` arena profile, then suballocate inside it only if they accept
  responsibility for that specialized policy. The native heap remains the
  general-purpose path.
- `FREE` applies only to hardware-owned allocation objects. Software-owned
  arenas are released through their owning arena, object, or VMA control path;
  subobjects inside those arenas are invisible to the Heap Engine.
- language runtimes own object layout, GC coordination, arena selection,
  profiling policy, region lifetime, and fallback behavior. Hardware owns
  allocation ownership transitions, accounting, metadata integrity, and common
  small/medium block movement.

The Heap Engine is retained only if its common path is local:

- per-thread allocation windows for common size classes.
- local cross-thread free queues that drain in batches.
- slab/run metadata touched only on allocation-window refill/drain, large allocation,
  hardening, fork COW, or error detection.
- large objects request page runs through the VMA/Page Engine rather than
  walking page metadata directly.
- slow paths are owner-engine transactions with bounded table walks. They do
  not run an interpreter, policy script, bytecode allocator, custom allocator,
  or unbounded coalescing algorithm in hardware.

Fast path target: common `ALLOC` sizes up to the small/medium threshold complete
from a per-thread allocation window. Common `FREE` completes into a local
free/quarantine cache or transfer queue. Neither path reads DDR heap metadata on
a hit.
If the common path frequently devolves into DDR metadata walks, the
implementation has failed the architectural intent.

The design goal is cultural as well as technical: the native heap should be
fast, thread-safe, observable, hardened, and integrated with VMA/fork/exec
policy well enough that programmers and runtime authors are not tempted to
write general-purpose allocators in software. Specialized region allocators,
language object layouts, garbage collectors, and slab caches remain software
policies layered over `ALLOC_EX` arenas, `memory_object`, or `MMAP`.

The architectural contract deliberately omits:

- raw size-class table internals beyond the fixed architectural/profile limits.
- freelist pointers and slab/run physical layout.
- allocation-window depth and refill policy.
- quarantine algorithm and reuse delay.
- compaction, profiling, GC, and language object policies.

## 19. Signals

The Signal Engine handles asynchronous delivery and synchronous hardware fault
delivery. LNP64 does not expose a software interrupt-vector table for ordinary
processes; the architectural delivery surface is a clean hardware Unix-signal
subset plus structured event/fault records.

V1 freezes the useful, widely implemented subset:

- a bounded architectural signal number space.
- process-wide handler/default/ignore table.
- thread-local signal mask.
- process-pending and thread-pending signal bits plus bounded metadata records.
- deterministic fault-to-signal mapping for common CPU/MMU faults.
- `KILL` for checked software signal injection.
- `ALARM` as a compatibility timer profile over timer/event hardware.
- fixed psABI signal handler entry and `SIGRET`.
- default fatal termination for unhandled fatal signals.

V1 deliberately does not freeze:

- Linux/BSD-specific restart quirks for every blocking API.
- full POSIX realtime signal queueing and priority semantics.
- arbitrary signal stack ABI variants.
- signal-based application IPC as the preferred native mechanism.
- implementation-specific process-directed delivery corner cases.
- legacy `SA_*` flag matrices beyond the frozen subset.

Those behaviors may be emulated by libc or a Unix personality using event
queues, domain policy, call gates, and compatibility metadata.

Per process/thread state:

- process-wide disposition table: `default`, `ignore`, or `handler`.
- process-pending signal set plus bounded process-pending records.
- thread-local signal mask.
- thread-local pending signal set plus bounded thread-pending records.
- per-thread saved signal context stack with token/generation.
- per-thread signal-delivery depth counter.
- per-process child/exit signal state for `SIGCHLD`-style compatibility.

`SIGACTION` writes the handler table. Each entry is one of `default`, `ignore`,
or `handler_pc`. V1 handler flags are intentionally small: handler installed,
mask-while-running, and optional one-shot reset-to-default. Other POSIX or
Linux/BSD flags are compatibility metadata and do not change hardware delivery
unless a future profile freezes them.

`SIGMASK_SET` updates the issuing thread's signal mask and may trigger immediate
delivery of newly unmasked pending signals.

`KILL` finds target PID/TID, checks credential/capability/domain policy, appends
pending signal state, and wakes the target if it is in an interruptible wait.

Signal injection forms:

- synchronous hardware faults are always thread-directed to the faulting thread.
- `KILL` to a TID is thread-directed.
- `KILL` to a PID is process-directed and enqueues process-pending state.
- `raise` lowers to a thread-directed self-signal.
- `ALARM` enqueues process-directed `SIGALRM`.
- child exit enqueues process-directed `SIGCHLD` unless ignored or masked by
  compatibility policy.

Process-directed delivery selects a target by a fixed rule:

1. prefer a running or ready thread in the process with the signal unmasked.
2. otherwise prefer an interruptible waiting thread with the signal unmasked.
3. otherwise keep the signal in process-pending state until `SIGMASK_SET`,
   thread creation, wait interruption, or scheduler issue makes a thread
   eligible.

The selection rule is deterministic within an implementation profile. It is not
a Linux/BSD compatibility promise about which thread receives every
process-directed signal.

Signal delivery:

- scheduler sees pending unmasked signal before normal issue.
- Signal Engine writes a saved context record.
- PC is replaced with handler address.
- signal number is written to the first ABI argument register.
- a compact signal-code/source record is available through the signal frame or
  runtime metadata pointer.
- if `mask-while-running` is set, the delivered signal is temporarily masked for
  the handler.
- `SIGRET` restores saved PC, flags, registers, and signal mask from the
  Signal Engine-owned context token/generation.
- if another unmasked signal arrives while a handler is running, delivery is
  deferred unless the handler's entry explicitly permits bounded nesting.

V1 nesting rules:

- default maximum delivery depth is one active handler per thread.
- an implementation may support a small bounded nesting depth reported through
  `ENV_GET`.
- nested delivery never overwrites an active saved context token.
- overflow of the saved-context stack delivers a fatal `SIGSEGV`/`SIGBUS`-class
  signal or terminates according to domain hardening policy.

Fatal signals without handlers terminate the process through the same path as
`EXIT`.

Hardware fault mapping:

- integer divide-by-zero, floating-point invalid operation, floating-point
  divide-by-zero, overflow when trapping overflow is enabled: `SIGFPE`.
- illegal opcode, reserved opcode, disabled extension, malformed instruction:
  `SIGILL`, unless supervisor opcode-event policy captures it first.
- unmapped virtual address, write to read-only page, execute from non-executable
  page, permission failure on normal memory: `SIGSEGV`.
- alignment fault, physical/device mapping failure, access outside a mapped
  device aperture after translation, and non-recoverable bus response: `SIGBUS`.
- breakpoint, single-step/debug trap when enabled: `SIGTRAP`.

The saved signal context record includes:

- saved context token/generation.
- faulting PC.
- next PC where architecturally meaningful.
- signal number and POSIX-style signal code.
- bad virtual address or zero when not address-related.
- trapped instruction word for decode faults and debug tooling.
- source PID/TID/domain for software-injected signals where permitted.
- event/fault id for structured fault correlation.
- saved flags plus GPR/FPR/VR state needed by the psABI.

Signal-frame memory is not trusted authority. It is a runtime ABI record used
for debugging and handler convenience; `SIGRET` restores from the Signal
Engine's saved context identity/generation, not from arbitrary user-writable
frame data. Signal-frame stack regions are NX by default and may be guarded by
runtime policy.

Recoverable page faults are not delivered as signals immediately. The VMA Engine
first attempts resident-page install, anonymous zero-fill, copy-on-write, or an
object-owner page-fill transaction. Only failed, revoked, poisoned, guard, or
permission-denied faults enter the Signal Engine.

Interruptible operation behavior:

- if a handled signal arrives while a thread is blocked in an interruptible
  operation, the operation returns `-1` with thread-local `ERRNO=EINTR` or the
  operation's typed interrupted status before handler entry.
- non-interruptible operations that have passed their commit point run to their
  defined completion, roll-forward, or teardown policy before delivery.
- `AWAIT`, futex waits, timer waits, `PULL`/`PUSH` waits, object-backed page
  fills before page-install commit, and queued call-gate waits are interruptible
  unless their object profile explicitly marks the wait non-interruptible.
- DMA descriptors, VMA updates, control operations, and call gates use their
  documented commit/cancel rules from the Capability, VMA, DMA, and Typed
  Control sections.

Compatibility layering:

- libc/personality code may expand the compact frame into Linux/BSD `siginfo_t`
  and `ucontext_t` shapes in user memory.
- `SA_RESTART`, realtime signal queues, `sigqueue` payload priority, historical
  `sigaltstack` variants, and OS-specific delivery choices are compatibility
  policy over the hardware substrate.
- hardware signal delivery is for precise faults, process control, timers, and
  POSIX compatibility. Native high-rate application IPC should use queues,
  event queues, call gates, counters, or shared memory objects.

## 20. Futex and Atomic Engine

`LOCK_CMPXCHG` is implemented in the LSU/DDR atomic path:

- translate virtual address.
- lock the cache line or atomic DDR transaction slot.
- compare current value.
- conditionally write new value.
- return old value or success code in destination register.

Futex wait:

- is encoded as `AWAIT` with wait kind `futex`.
- translates address.
- atomically reads value.
- if value equals expected, parks TID on a hash bucket keyed by physical address.
- if not equal, returns immediately with `ERRNO=EAGAIN`.

`WAKE` with wake kind `futex`:

- translates address.
- finds matching wait bucket.
- moves up to requested count of TIDs to ready queue.
- returns wake count.

Futex local-state requirement:

- uncontended atomics bypass the Futex Engine and complete in the LSU/L2 atomic
  path.
- hot futex buckets keep physical address tag, waiter count, and head/tail
  pointers in FPGA RAM.
- DDR waiter records are spill storage for long wait queues, not the normal
  first lookup for a hot futex.
- wake paths should produce scheduler-ready TID lists directly rather than
  walking arbitrary waiter metadata.

## 21. PCRs and Credentials

PCR reads are backed by process credential state plus thread-local state:

- PID: read-only, from process context.
- PPID: read-only, from process context, or `0` for root.
- TID: read-only, from thread context.
- UID: from process credential context.
- GID: from process credential context.
- SIGMASK: from thread context.
- CAPMASK: from process credential context.
- REALTIME_SEC / REALTIME_NSEC: read-only scalar realtime clock snapshot fields
  for libc/runtime clock reads. Timer expiry and waitability remain represented
  by timer profiles and event/waitable FDRs.

`GET_PCR` reads from context into a GPR. `SET_PCR` is permission checked in
hardware. UID/GID/CAPMASK changes update credential context and require the
current effective UID/capability policy to permit the transition. `SIGMASK`
updates are thread-local. Realtime clock PCRs are read-only.

All namespace/object permission checks consume a snapshot of UID/GID from PCR
state at command issue time.

`ENV_GET` reads read-only process and machine metadata into a GPR or copies a
small metadata record to a user buffer. It is for libc, loaders, language
runtimes, and compatibility personalities; it is not a replacement for immediate
operands or literal loads.

`ENV_GET` uses F8:

```text
a=result_dst, b=key_reg, c=index_or_buf_reg, d=len_or_flags_reg, imm16=variant
```

Scalar keys return the value in `result_dst`. Buffer keys use `c` as a user
buffer pointer and `d` as byte length; success returns the number of bytes
written. Failure returns `-1` and updates thread-local `ERRNO`.

V1 metadata keys:

- `isa_version`.
- `page_size`.
- `cache_line_size`.
- `timebase_hz`.
- `hwcap0` and `hwcap1`.
- `architectural_thread_limit`.
- `process_limit`.
- `default_fdr_limit`.
- `event_queue_limit`.
- `futex_bucket_count`.
- `startup_metadata_ptr`.
- `startup_metadata_len`.
- `startup_metadata_format`.
- `startup_metadata_version`.
- process personality id.
- boot manifest flags exposed to PID 1.

`GET_PCR` remains the authority and credential path. `ENV_GET` is read-only and
must not expose mutable privilege state except through ordinary public metadata
such as PID/TID when a runtime asks for it.

POSIX `argc`, `argv`, `envp`, and auxv are runtime/personality structures inside
the startup metadata block. Hardware carries the pointer and basic format tag;
it does not index auxv, parse environment strings, or interpret dynamic-loader
metadata.

`RANDOM` is the architectural entropy instruction:

```text
a=result_dst, b=len_or_flags_reg, c=buf_reg, d=reserved, imm16=variant
```

Scalar variants return up to one machine word of entropy in `result_dst`.
Buffer variants copy entropy into `c` for `b` bytes and return the byte count.
Failures return `-1` and update thread-local `ERRNO`.

The Entropy and Randomization Engine feeds:

- ASLR decisions during `EXEC`, `MMAP`, stack creation, heap arena creation, and
  call-gate trampoline placement.
- libc stack canaries and runtime seeding.
- randomized object ids where an object class benefits from nonpredictability.
- allocator hardening and quarantine policies.

Entropy output is domain-accounted and rate-limited if needed, but it is not a
capability secret by itself. Capability authority still comes from unforgeable
FDR entries, rights, object ids, and generation checks.

### 21.1 Privilege and Security Model

V1 freezes a capability-native cloud profile with a POSIX credential
compatibility layer. The native authority model is capability possession,
object rights, generation validity, and Resource Domain policy. UID/GID and
permission bits remain because real Unix software expects them, but they are a
credential profile rather than the root of the architecture.

Rejected alternative A: Unix-like UID/GID plus capability bits only as an
informal policy.

- Familiar model for file permissions, signals, ownership, and setuid-like
  transitions.
- Root-equivalent UID 0 can mount devices, bind privileged endpoints, change
  ownership, and configure global hardware tables.
- Add per-process capability bits for narrower authority such as network
  binding, adapter configuration, raw device access, and process inspection.
- Good default if LNP64 wants to run conventional cloud software with minimal
  runtime changes.

Rejected alternative B: pure object capabilities.

- FDRs and process handles carry all authority.
- No global root user; authority is delegated by passing capabilities.
- Strong fit for hardware FDRs and least-privilege services.
- Weakness: conventional POSIX software expects UID/GID checks and ambient
  process authority.

Chosen model: capability-native cloud profile with POSIX credentials.

- Keep UID/GID and POSIX permission checks for compatibility.
- Represent privileged powers as hardware capability bits attached to process
  context.
- Require both UID/GID permission and specific capability bits for dangerous
  operations such as raw network access, mounting, adapter table loading,
  cross-user `KILL`, and process memory inspection.
- Chosen for v1 because it preserves POSIX shape for libc and Linux/BSD
  personalities while avoiding a single all-powerful root path in hardware.

UID/GID participates in compatibility decisions, but authority over files,
devices, memory objects, call gates, DMA buffers, namespaces, and supervisor
controls is carried by FDR capabilities and Resource Domain policy.

V1 security invariants:

- W^X is enforced by the VMA Engine. Writable-plus-executable mappings require
  an explicit Resource Domain JIT/loader policy bit and should be temporary.
- Data is NX by default: heap, stack, queues, shared memory, DMA buffers, device
  BARs, signal frames, and ordinary anonymous memory are not executable.
- ASLR is enabled by default for `EXEC`, stack placement, heap arenas, anonymous
  `MMAP`, shared objects, signal trampolines, and call-gate trampolines.
- Guard pages are first-class no-access VMAs used for stacks, signal frames,
  heap arenas, selected large allocations, and runtime hardening.
- Generation checks are mandatory on domains, FDRs, VMAs, heap arenas,
  waitable objects, event sources, call gates, DMA buffers, and mapped device
  objects.
- Revocation invalidates cached descriptors, mappings, event bindings, call
  gates, and DMA exports before object ids or authority-bearing slots are
  reused.
- Capability delegation can only narrow authority: rights, ranges, event masks,
  mapping permissions, transfer rights, and device scope cannot be broadened by
  a receiver.
- Sealed capabilities may be transferred and used according to their rights, but
  cannot be inspected, narrowed, duplicated, or used to mint related authority
  unless the sealed rights explicitly permit it.
- DMA isolation is mandatory. Internal devices, `DMA_CTL`, PCIe requesters, and
  file/page-fault DMA all pass through VMA permission checks, FDR capability
  checks, Resource Domain accounting, coherent-DMA visibility, and IOMMU/device
  scope where applicable.

The v1 process credential context contains UID, GID, supplementary group pointer
or group-set object, and a capability bitmap. Hardware permission-check FSMs must
consume this credential snapshot at operation issue time. Required capability
bits include:

- mount or remount device backends.
- configure Ethernet filters and privileged ports.
- access raw block devices.
- load or replace device-driver support tables.
- hold the PCIe Root Complex control FDR used by the Bus Master.
- change UID/GID upward.
- send signals across UID boundaries.
- inspect or mutate another process.
- alter namespace-service metadata outside delegated permissions.

PCIe delegation follows pure capability rules after bootstrapping. The Bus
Master is trusted because reset grants it the PCIe Root Complex and config-space
authority. Driver processes do not need a separate `driver_domain` bit to map a
BAR: possession of a valid `pcie_bar` FDR is the authority. The hardware VMA
engine checks only the FDR class, rights, page-granular bounds, and memory type
permissions at `MMAP` time.

### 21.2 Resource Domains, Virtualization, and Cgroups

Resource Domains unify virtualization, containers, cgroups, jails, sandboxes,
and supervisor domains. A Resource Domain is a nested hardware capability and
accounting container for a process subtree. The hardware primitive is the same
for all of them: `DOMAIN_CTL create child` with a profile record describing
delegated resources, budgets, capability roots, namespace/device/network scope,
security monotonicity, and upcall routing. Software presentation determines
whether the child is called a VM, container, cgroup, jail, sandbox, or
supervisor domain.

Each domain contains:

- parent domain id and generation.
- child domain table pointer.
- attached process/thread subtree root.
- resource limits and current usage.
- scheduler budget, weight index, quota/period, virtual runtime/deadline,
  latency class, dispatch eligibility, and allowed core-tile mask.
- memory budget, VMA budget, heap budget, and page pressure counters.
- PID/thread count limit.
- FDR table limit and capability delegation root.
- namespace root/cwd delegation pointers.
- event queue and upcall policy.
- device, DMA, and PCIe capability scope.
- security policy bits: ASLR enable/disable constraints, JIT/loader W^X
  exception authority, executable-memory source policy, entropy quota, and
  hardening profile.
- tenant/confidential profile bits: tenant-strict mode, parent-inspection
  denial, explicit shared-page policy, measured-launch requirement,
  memory-encryption/key-id tag, sealed-secret policy id, and telemetry scope.
- checkpoint hook state: frozen/quiescing/quiesced flag, dirty-memory tracking
  root, exportable state cursor, and explicit reattachment generation base.
- freeze/park state and teardown policy.

Hard invariants:

- domains form a tree, not an arbitrary graph.
- the domain tree is the ownership, accounting, and teardown structure;
  capability references, call gates, shared memory, and IPC endpoints may still
  form graphs across that tree.
- child limits are monotonic downward; no child can exceed budget delegated by
  its parent.
- child capabilities derive from parent capabilities and preserve delegation
  lineage.
- usage accounting is hierarchical; child usage rolls into all ancestors.
- freeze, kill, revoke, and teardown can apply to a whole subtree.
- stale domain references fail through generation checks.
- upcall policy can be delegated, masked, or translated by each parent domain.
- hardware enforces budgets and capability scope even when a guest personality
  implements its own policy.
- security policy is monotonic with delegation: a child may become stricter, but
  cannot enable broader executable-memory, DMA, device, entropy, or capability
  transfer authority than its parent delegated.

Linux-style cgroup controllers map directly onto domain fields:

- CPU controller: scheduler budget, weight, quota, and allowed core-tile mask.
- memory controller: physical page, VMA, heap, and mapped-object budgets.
- pids controller: PID and TID count limits.
- I/O controller: FDR/backend bandwidth tokens and outstanding operation limits.
- devices controller: delegated capability whitelist.
- cpuset: allowed core tiles and memory locality policy.
- freezer: domain-wide park/resume.
- pressure metrics: hardware usage counters and event records.

VMs and containers use the same creation primitive:

- `DOMAIN_CTL create child` allocates a child domain id/generation, installs
  monotonic limits, attaches an optional process subtree, delegates capability
  roots, installs security policy bits, and configures upcall masks.
- the profile type is descriptive metadata for software and validation. It does
  not select a different containment mechanism.
- a VM profile usually grants stronger supervisor/upcall policy, block-image or
  paravirtual device capabilities, virtual network endpoints, console/timer
  capabilities, and a delegated namespace/process view.
- a container profile usually shares the parent personality/runtime, receives
  narrower namespace roots, narrower FDR/device/network capability scope, and
  ordinary cgroup-like budgets.
- a cgroup profile is the same domain algebra with minimal namespace/device
  changes and stronger emphasis on accounting/pressure/freeze controls.
- a sandbox profile is the same domain algebra with sharply narrowed FDR,
  call-gate, memory, network, and device authority.
- nested virtualization and nested containers are just child domains under a
  parent domain; no separate VM tree or container tree exists in hardware.

The same operation therefore creates both a VM-like guest and a container-like
workload. The difference is delegated authority and upcall policy, not hardware
object kind.

`DOMAIN_PROFILE_TENANT_STRICT` is the production isolation profile:

- W^X, NX data, ASLR, guard pages, generation checks, and scoped entropy are
  mandatory and cannot be relaxed by the child.
- all device, DMA, packet queue, namespace, and storage access requires
  delegated capabilities.
- raw physical interrupts are unavailable; IRQs arrive only as event
  capabilities.
- parent domains may freeze, kill, measure, revoke delegated capabilities, and
  query permitted aggregate counters, but cannot read child memory or sealed
  secrets without an explicit shared-memory or inspection capability.
- trace, fault, and telemetry records for the child are redacted or aggregated
  according to the child's telemetry scope before delivery to monitoring
  domains.

Confidential-domain hooks are a stricter tenant profile extension:

- measured launch is required before the domain is marked runnable.
- domain memory carries a reserved encryption/key-id tag in VMA/page metadata.
- shared pages are explicit VMA states and are the only ordinary data path to
  parent or peer domains.
- sealed secrets can be released only to matching measurement and policy
  records.
- checkpoint encryption metadata is software-owned, but hardware enforces that
  confidential memory cannot be exported through ordinary query-state,
  telemetry, trace, DMA, or fault records.
- FPGA v1 may reject production confidential mode if no real memory-encryption
  block exists, but the architectural state, refusal behavior, and proof
  boundary are reserved.

Profile examples:

- a VM-like guest is a domain with strong upcall policy and delegated namespace,
  memory, process, and device views.
- a container is a domain sharing a parent personality/runtime but with narrower
  namespaces, budgets, and capabilities.
- a cgroup is a domain focused on resource accounting and limits.
- a sandbox is a domain focused on narrowed FDR/capability authority.
- nested virtualization is just child domains beneath a guest domain.

`DOMAIN_CTL` is the architectural control surface. It can create child domains,
set or query limits, snapshot usage counters, delegate or revoke capabilities,
attach process subtrees, configure upcall policy, and freeze/resume a subtree.
`SUPERVISOR_CTL` remains as a narrower compatibility/source-level profile for
domains whose main purpose is upcall supervision.

Checkpoint and live-migration compatibility hooks are v1 architectural metadata
hooks, not hardware checkpoint/restore or full live migration:

- `freeze` drives a subtree toward a quiescent boundary: no running threads,
  no new DMA descriptors, no in-progress metadata commits, and all call-gate
  continuations either parked or canceled according to policy.
- `query-state` exposes bounded records for thread contexts, FDR tables, VMA
  ranges, event queues, waitable objects, heap arenas, pending signals, and
  capability lineage/generation metadata.
- `resume` restarts a quiesced domain without changing object generations.
- dirty-memory tracking is optional in FPGA v1 but the VMA Engine must reserve
  the state bit or counter hook so checkpointing does not require redesign.
- service callback events let filesystem, network, device, and personality
  domains drain or serialize their own state before a checkpoint boundary.
- endpoint drain/redirect hooks let software quiesce packet queues, stream
  endpoints, call gates, and storage objects without teaching hardware TCP,
  TLS, filesystem, or application protocols.
- hardware does not define checkpoint image formats, compression, encryption,
  deduplication, migration transport, CRIU policy, device model save/restore,
  TCP migration, filesystem replay, or external resource resolution.
- future restore is software-owned: software creates a fresh child domain with
  fresh domain id/generation bases, replays state into native objects, and
  reattaches capabilities explicitly through capability and domain control
  operations. Hardware verifies lineage, generation, rights, and domain policy;
  it does not parse global checkpoint images.

### 21.3 Paravirtual Unix Guest Profile

LNP64 does not add a conventional hosted-OS profile with kernel rings, software
page tables, mandatory syscall traps, or an OS-owned scheduler. A future
Linux/NetBSD port is made plausible by treating the kernel as a paravirtual Unix
personality domain running on top of native LNP64 capability/event/domain
hardware.

The silicon remains authoritative for:

- hardware process and thread creation.
- runqueue scheduling and context storage.
- VMA creation, teardown, page faults, and copy-on-write.
- FDR capabilities, namespace-dispatch references, and hardware-owned object
  references.
- signals, futex queues, fd readiness, and DMA completion.

The guest kernel/personality owns:

- Linux/BSD-specific process metadata.
- domain profiles for namespaces, cgroups, jails, credentials, and policy state.
- emulation of APIs not directly represented by LNP64 opcodes.
- Linux syscall-number compatibility where a syscall-compatible runtime is used.
- filesystem images mounted inside block-image or storage-service FDRs.
- network stack policy above raw frame or datagram hardware objects.
- compatibility ABIs and userland conventions.

Targeted compatibility approaches:

- Linux as a paravirtual personality: a Linux kernel port runs as a supervisor
  domain over a delegated LNP64 process subtree. It maps Linux tasks, files,
  memory mappings, futexes, signals, and devices onto native hardware
  primitives.
- Linux syscall compatibility runtime: a loader/libc/runtime maps Linux syscall
  ABI calls onto native LNP64 instructions without booting a full Linux kernel.
  This is the shortest path to running many cloud-oriented programs.
- NetBSD rump-kernel style: selected NetBSD filesystem, networking, or device
  stacks run as LNP64 service processes. They receive block, network, PCIe, or
  delegated namespace FDRs and expose services back through native FDRs.

Minimal personality interface surface:

| Surface | Native mechanism | Personality use | Hardware remains owner of |
| --- | --- | --- | --- |
| process/thread lifecycle | `CLONE`, `EXEC`, `EXIT`, child waitables, lifecycle events | Linux tasks, BSD processes, pthreads, process groups, wait semantics | PID/TID allocation, thread contexts, exec barrier, runqueue state |
| memory maps | `MMAP`, `MUNMAP`, `MPROTECT`, page-fault/fill events, VMA change events | guest `mmap`, `brk`, COW policy presentation, loader mappings | VMA tree, page-state machine, TLB/I-cache shootdown, COW commit |
| FDR/fd tables | FDR tables, `CAP_SEND`/`CAP_RECV`, `CAP_DUP`, `CAP_REVOKE`, close-on-exec/inherit metadata | Linux fd table, descriptor passing, `/proc` views, rights emulation | capability validity, generation, lineage, returned-capability install |
| namespace/filesystem | `OPEN_AT`, `NS_CTL`, namespace dispatch FDRs, block-image/storage FDRs | mount namespaces, procfs/sysfs-like views, ext4/FFS inside images | namespace capability bounds, dispatch request shape, FDR authority |
| signals/faults | hardware signal subset, fault events, `SIGRET`, supervisor upcalls | Linux/BSD signal compatibility, restart policy, realtime emulation | precise fault classification, frame safety, mask/pending core state |
| wait/sync | `AWAIT`, futex-flavored waits, event queues, timer FDRs | `poll`, `epoll`, `kqueue`, futex ABI, sleeps, timeouts | no-lost-wakeup state, timer/event routing, scheduler park/wake |
| networking | `net_namespace`, `packet_queue`, `stream_endpoint`, `datagram_endpoint`, `listener` | Linux/BSD socket ABI, software TCP/IP, virtio-net-like queues | endpoint authority, packet DMA safety, readiness/events, classifier bounds |
| devices | `pcie_bar`, `dma_buffer`, `irq_event`, typed device controls | driver domains, vfio-like assignment, guest device models | IOMMU/DMA isolation, BAR mapping permissions, raw interrupt non-exposure |
| supervision | domain control FDR, fixed upcall records, `DOMAIN_CTL` | syscall ABI runtime, policy decisions, nested guest/container management | Resource Domain tree, budgets, monotonic delegation, scheduler/MMU authority |

Non-targeted approach:

- A full traditional Linux/NetBSD port that owns page tables, context switching,
  interrupts, and raw devices is not a v1 design target.

Compatibility layering rules:

- POSIX descriptors are represented by FDR capability handles; Linux fd tables
  map to FDR tables plus personality metadata.
- `fork` is a constrained compatibility profile over `CLONE`: new process,
  exactly one child thread, COW VMAs/heap metadata, inherited/narrowed
  capabilities according to descriptor flags, copied credentials/dispositions,
  caller signal mask copied, child pending signals cleared, no in-flight
  ownership copied, and POSIX parent/child return conventions.
- Linux/BSD-specific fork corners, `pthread_atfork` behavior, runtime lock
  recovery, and process-attribute quirks belong to libc or the personality
  domain before/after the hardware clone transition.
- `pthread_create`, native actors, and guest tasks use other `CLONE` profiles
  rather than pretending that fork is the fundamental process primitive.
- POSIX signals are represented by hardware event delivery plus an ABI signal
  frame. Native code may use event queues, cancellation objects, domain faults,
  and call-gate completions instead.
- `errno` is a libc/personality view of explicit result/error status.
- Path lookup is syntax and compatibility; authority comes from directory/root
  FDRs, namespace-root capabilities, and opened object capabilities.
- UID/GID and mode bits are credential metadata for Unix software; object
  capabilities and Resource Domain policy remain authoritative.

Supervisor domains:

- A process with domain-management authority may create a delegated Resource
  Domain and configure it for supervisor upcalls.
- Native processes inside that subtree are bound to the domain's policy.
- The supervisor may receive upcalls for selected events from its subtree.
- Hardware still executes native opcodes directly; the supervisor is a policy
  and compatibility layer, not the scheduler or MMU owner.

Upcall events:

- unsupported or disabled opcode attempted by a supervised process.
- Linux syscall-ABI event emitted by a syscall compatibility runtime.
- permission denial that the domain policy may virtualize.
- child exit, signal delivery, fd readiness, futex wait/wake, timer expiry.
- namespace lookup events for paths delegated to the guest personality.
- block-image completion events for guest filesystems.
- process creation, exec, exit, and memory map changes.

Upcalls are delivered through a domain control FDR with object class `control`
and backend `namespace_dispatch`, `object`, or `supervisor_engine` as
appropriate. The control FDR exposes event records through `PULL` and accepts
policy commands through `PUSH`. This keeps the mechanism inside the FDR model
instead of introducing a traditional syscall path.

The Supervisor Upcall Engine is an event shaper, not a policy processor. Its
hard logic is limited to:

- matching event type against registered masks.
- copying a fixed event record into an active queue slot.
- attaching object ids, fd indices, operation ids, errno/result fields, and
  short argument words.
- parking or waking supervised TIDs at documented boundaries.
- enqueueing overflow records to the delegated control/event FDR.

Namespace policy, syscall compatibility, Linux/BSD semantics, cgroup-like
accounting, and guest-specific decisions remain software in the supervisor
process. If an upcall path needs a complex decision tree, it belongs in that
process, not in RTL.

The upcall record format must be fixed-width, versioned, and endian-stable. At
minimum it carries event type, source PID/TID, domain id, object id or fd index,
operation id, errno/result fields, and four 64-bit argument slots. Larger event
payloads are referenced by FDR-backed buffers rather than embedded in the event
record.

Delegated namespaces:

- Namespace capabilities may be delegated to a supervisor, filesystem, or
  personality domain.
- Native `OPEN_AT` and `NS_CTL` operations become hardware-mediated dispatches
  to the owning namespace service at configured delegation points.
- The guest may implement Linux mount namespaces, bind mounts, procfs-like
  synthetic trees, or BSD jail views above those delegated roots.
- Non-delegated hardware-owned objects remain directly usable by capability, but
  general path semantics remain service-owned.

Block-image FDRs:

- A storage service, boot manifest, or block device may expose an object class
  `block_device` with subtype `block_image`.
- The guest block layer uses explicit-offset `PULL` and `PUSH` rather than
  descriptor seek state.
- Linux ext4, NetBSD FFS, or other guest filesystems can live inside one or
  more large block-image or storage-service objects.
- Hardware does not need to understand those guest filesystem formats.

Task mapping:

- Linux/BSD threads map one-to-one to LNP64 hardware threads where practical.
- The guest scheduler becomes a policy layer that creates, parks, wakes, and
  accounts for native hardware threads.
- The hardware scheduler still performs actual dispatch and context switching.
- Guest preemption is supported by supervisor-domain timer policy: when a domain
  timer fires, the scheduler fabric can force-park or redirect a running thread
  in that supervised subtree at a bounded scheduling boundary and deliver an
  upcall to the supervisor. Cooperative yield points remain an optimization, not
  the only preemption mechanism.

Memory mapping:

- The guest memory manager uses `MMAP`, `MUNMAP`, and `MPROTECT` to request
  hardware VMAs.
- It does not write page tables directly.
- Guest copy-on-write and process isolation are represented as LNP64 VMA and
  COW operations inside the delegated domain.

ABI requirements:

- LNP64 needs a stable psABI: calling convention, callee-saved registers, stack
  alignment, process entry layout, `argv`/`envp`/auxv layout, TLS register or
  TLS lookup mechanism, errno convention, and signal frame layout.
- The Linux syscall compatibility runtime needs a stable Linux-call dispatch
  ABI even if the hardware itself has no `SYSCALL` instruction. A conventional
  trap is not required; the runtime may use a reserved illegal opcode, a call
  gate function, or a control-FDR command path.
- Time support must include monotonic time, realtime clock, timer FDRs, and
  timer upcalls so `clock_gettime`, sleeps, timeouts, poll/epoll emulation, and
  scheduler accounting can be implemented.
- Event waiting needs a stable aggregation object that can wait on fd readiness,
  timer expiry, child exit, signal delivery, futex events, and supervisor
  upcalls. `AWAIT` is the primitive, but runtimes need a way to construct
  event-queue FDRs that represent sets of wait sources.

This profile preserves the LNP64 thesis: Linux/NetBSD can become personalities
that project their semantics onto native capability/event/domain hardware,
rather than forcing the chip to become a conventional trap-and-kernel machine.

## 22. DMA Fabric

The DMA Fabric moves bytes between:

- DDR user buffers.
- DDR user buffers for memory-to-memory `DMA_CTL` copy/fill operations.
- SD card sector buffers.
- SPI flash streams.
- UART FIFOs.
- Ethernet RX/TX buffers.
- PCIe DMA buffers.
- storage/object service buffers.

Every DMA command carries:

- process address-space id.
- virtual address.
- byte length.
- direction.
- Resource Domain id and generation.
- source and destination object ids when operating on FDR-backed objects.
- source and destination object generations.
- fault policy.
- completion target TID or engine.
- optional completion event object.
- optional PCIe requester id and IOMMU context.

The DMA fabric uses the MMU for user virtual addresses. If translation faults,
the fault is routed back to the VMA Engine. The original operation remains
blocked until the page fault resolves or fails.

`DMA_CTL` exposes the same fabric to ordinary code for large memory movement:

- large `memcpy` / `memmove`-style copies.
- large `memset` / zero-fill operations.
- scatter/gather copies for runtimes and I/O frameworks.
- optional checksum/hash profiles for networking and storage runtimes.
- completion through a result register for small synchronous commands or through
  `event_queue` FDRs or `counter` completion profiles for long commands.

The DMA Fabric must not bypass normal memory safety. `DMA_CTL` requests still
use VMA translation, permissions, cache-coherence rules, and capability checks
for FDR-backed memory objects.

DMA isolation rules:

- all DMA requests are checked against the issuing Resource Domain's memory,
  device, and DMA budgets before they are accepted.
- DMA pinning fails for guard pages, unmapped pages, executable-only pages,
  revoked mappings, stale generations, or pages outside the caller's delegated
  capability range.
- DMA buffers are explicit FDR-backed objects with rights, permitted direction,
  byte range, memory type, owner domain, and generation.
- completion events are not delivered until cache visibility and revocation
  checks have completed.
- revocation of a DMA buffer prevents new descriptors, waits for or cancels
  in-flight descriptors according to policy, tears down device mappings, and
  only then releases backing pages.

For PCIe, the DMA Fabric and IOMMU jointly enforce that a device can access only
pages exported through a valid `dma_buffer` FDR. Revocation requires the Bus
Master or driver to quiesce the device, tear down IOMMU entries, and wait for
in-flight DMA completion before the VMA Engine releases the backing pages.
The IOMMU context includes requester id, domain id/generation, allowed page
ranges, direction, and buffer generation; stale or revoked contexts fault and
emit an event to the owning driver/control FDR.

## 23. Boot Flow

There is no boot CPU.

Reset creates a default operating envelope before PID 1 or any service thread
can run. Hardware initializes the root Resource Domain, PID 1 domain, scheduler
profile, memory/security defaults, telemetry/fault routes, capability roots,
and initial device grants. PID 1 refines and supervises this world; it does not
rescue an unconfigured machine or invent the authority model in software.

Reset sequence:

1. Hardware reset controller initializes FPGA-local RAM structures, scheduler
   active windows, root runqueue state, and default weighted-fair scheduler
   parameters.
2. DDR controller calibration completes.
3. Page allocator marks DDR regions free or reserved.
4. Reset controller records FPGA build id, configuration hash where available,
   and reset cause into the boot measurement log.
5. Root Resource Domain and PID 1 Resource Domain records are created with
   valid generations, default weights/quotas, memory/FDR/event budgets,
   telemetry scope, security policy, and explicit empty capability roots.
6. W^X, NX data, ASLR, guard-page defaults, raw-interrupt non-exposure, fault
   routing, telemetry FDR templates, and boot-control/quote records are
   installed before first dispatch.
7. Boot image reader locates and validates the manifest table on SD, SPI flash,
   or another boot backend by fixed offset/header scan.
8. FDR table template binds `fd0`, `fd1`, `fd2` to UART.
9. If PCIe is present, Root Complex link training completes, but enumeration is
   deferred until a Bus Master executable is loaded.
10. The boot manifest names image records by type, offset, length, hash, and
   initial grants. Required records include PID 1 executable image and initial
   FDR grants. Optional records include namespace service, filesystem service,
   PCIe Bus Master, network service, and recovery service images.
11. The manifest bytes and named executable images are measured into the boot
   measurement log. Signature verification is optional for FPGA v1, but the
   measurement log is architectural.
12. Process Engine creates PID 1, TID 1, UID 0, loads the PID 1 image by
   manifest record, and grants stdio, boot-control, storage/block, and
   initial service capabilities named by the manifest.
13. If the manifest names namespace/filesystem services, the boot engine creates
   those service processes, grants their namespace/control/storage capabilities,
   and parks them or marks them ready according to manifest policy.
14. If the manifest names a Bus Master, the boot engine creates a privileged
   process for it, grants the PCIe Root Complex control FDR, loads it by
   manifest record, and parks it until PID 1 is ready to coordinate boot. If no
   Bus Master is named, PCIe enumeration is deferred to native userland.
15. Scheduler marks PID 1 and boot-manifest services ready only after their
   domain budget/accounting records and launch measurements are valid.
16. Fetch begins at PID 1 entry point.

Default operating envelope invariants:

- no runnable thread exists outside a valid Resource Domain.
- no thread can dispatch before its domain budget, virtual-time state, and
  accounting records are initialized.
- root/PID 1 capabilities are explicit FDRs, not ambient authority.
- raw physical interrupts are already routed into Event Router inputs before
  drivers or PID 1 run.
- initial fault, telemetry, boot-control, and quote FDRs exist according to
  manifest policy.
- optional namespace, filesystem, PCIe, and network services are launched only
  from measured manifest records and receive only explicit capabilities.
- if an optional service is absent, the corresponding authority remains absent
  rather than ambiently available.

Measured boot and attestation:

- `ENV_GET` exposes immutable scalar keys for FPGA build id, ISA revision,
  reset cause, boot measurement count, and boot policy flags.
- a read-only boot-control FDR exposes the measurement log to PID 1 and any
  delegated control domain.
- measurement records include FPGA bitstream/ROM identity, boot manifest hash,
  executable image measurements, domain launch measurements, selected security
  policy, and initial delegated capability roots.
- a quote/attestation FDR is the architectural surface for remote attestation.
  Production implementations sign measurement summaries through a hardware or
  board-rooted attestation key; FPGA development implementations may expose
  unsigned development quotes with an explicit non-production flag.
- tenant-strict and confidential domains can request per-domain launch
  measurement records before first dispatch.
- boot measurement failure emits a structured fault event and either continues
  in permissive FPGA-development mode or enters hardware panic according to boot
  policy.
- production key management may be board/vendor specific, but the quote record
  shape, measurement ordering, domain binding, and capability-root references
  are architectural.

If no boot image is found, the reset controller enters a hardware panic state
that emits a UART diagnostic and blinks a board LED pattern.

## 24. Error Reporting

Fallible POSIX-like instructions follow the emulator convention:

- success writes zero or a nonnegative byte/count/value to the instruction's
  encoded result register.
- failure writes all-ones `-1` to the encoded result register where applicable.
- issuing thread's `ERRNO` is updated on failure.
- source assembly may default some legacy static forms to `r1`, but the binary
  encoding always names the result register.

Hardware engines write result registers only at command completion. If a thread
is killed while an engine command is in flight, the Event Router cancels or
detaches the command according to object type.

### 24.1 Failure and Cancellation Semantics

Every long-latency hardware command has an operation id, owner PID, owner TID,
target object id, cancellation policy, and completion record.

Default rules:

- If the issuing thread receives a fatal signal, cancellable operations are
  canceled before signal termination completes.
- If the issuing thread receives a handled signal while blocked in an
  interruptible operation, the operation returns `-1` with `ERRNO=EINTR`.
- Non-interruptible metadata commits run to completion once they pass their
  commit point.
- Closing an fd from another thread detaches future access immediately but does
  not corrupt an operation that already holds an object reference.
- Process exit cancels all cancellable operations owned by that process and
  drops object references after completions or cancellations acknowledge.
- `CLONE` does not clone in-flight operation ownership into the child.
- `EXEC` cancels or waits for all operations tied to the old address space
  before replacing mappings.

Operation classes:

- `PULL`, `PUSH`, Ethernet receive/transmit, UART waits, and object-backed
  page-fill requests are interruptible before DMA/page-install commit and return
  `EINTR` if canceled.
- `NS_CTL` namespace mutations and `SET_META` metadata mutations become
  non-interruptible after the owning namespace/object service reaches its
  serialized commit point.
- `MMAP` and `MUNMAP` are cancelable before page table publication; after
  publication they complete and then report success or fault.
- `EXEC` is cancelable before the new image commit point; after commit, the old
  image no longer resumes.
- `CLONE` is cancelable before PID/TID publication; after publication the child
  must either become runnable or be reaped as a failed child with status.
- futex-flavored `AWAIT` is interruptible and returns `EINTR`; futex-flavored
  `WAKE` is nonblocking and noncancelable once issued.

Hardware engines must never deliver partial architectural writes to user memory
unless the instruction's POSIX result reports the number of bytes actually
transferred. Metadata operations are atomic at their documented commit point.

### 24.2 Structured Fault Event Model

Software-visible failures are not limited to POSIX `ERRNO` or synchronous
signals. Hardware engines also emit structured fault events through the Fault,
Telemetry, and Trace Engine.

Fault event sources:

- correctable and uncorrectable ECC/parity faults.
- poisoned page/object/descriptor access.
- malformed or generation-stale metadata detected by an owner engine.
- watchdog timeout or local engine reset.
- DMA translation, permission, direction, IOMMU, or coherence fault.
- storage barrier or media flush failure.
- boot manifest/image measurement failure.
- internal invariant violation that is recoverable enough to report.

Fault event records include:

- event class and severity.
- engine id.
- Resource Domain id/generation where applicable.
- PID/TID where applicable.
- object id/generation or physical/virtual page token where applicable.
- operation id.
- corrected/poisoned/degraded/panic disposition.
- compact implementation-specific syndrome bits.

Delivery rules:

- domain-scoped faults route to the owning domain's supervisor/control FDR when
  one is configured.
- otherwise recoverable system faults route to PID 1's control/event FDR.
- fatal unrecoverable faults enter hardware panic after attempting UART/trace
  emission.
- repeated fault storms may be coalesced, but the coalesced record must expose
  a lost-count field.

### 24.3 Observability Counters, Telemetry Capabilities, and Trace Ring

FPGA v1 includes low-cost counters, not a full dynamic tracing system.
Observability is delegated through telemetry FDRs; monitoring domains do not get
raw memory access, raw interrupt vectors, or an ambient global debug port.

Per-domain counters:

- CPU dispatch ticks, runnable time, blocked time, and forced parks.
- current and peak threads, FDRs, VMAs, heap pages, memory-object pages, and
  event records.
- `PULL`/`PUSH` ops and bytes, DMA ops and bytes, storage barriers, faults,
  signals, capability sends/receives/revokes, call-gate calls, and domain
  freeze/resume events.

Per-engine counters:

- commands issued/completed/aborted/canceled.
- local queue depth high-water marks.
- DDR requests, cache hits/misses where tracked, DMA descriptors, stalls,
  watchdog near-misses/timeouts, local resets, ECC corrections, and poisoned
  objects.

Access paths:

- `DOMAIN_CTL query` returns domain usage and pressure snapshots.
- `GET_META` on control FDRs returns engine counters, boot measurements, and
  object-local counters where permitted.
- supervisor/control FDRs can subscribe to threshold events for pressure, fault,
  and watchdog counters.
- telemetry FDRs can be narrowed to aggregate-only, per-domain, per-engine,
  redacted, destructive-read, snapshot-read, or threshold-event profiles.
- tenant-strict and confidential domains restrict trace/fault payloads to
  metadata that cannot reveal unauthorized memory, packets, secrets, or sealed
  capability contents.

The trace ring is optional but recommended for FPGA v1:

- fixed-size BRAM or DDR ring with a hardware write pointer and generation.
- records scheduler transitions, wait/ready transitions, `CALL_CAP` entry/exit,
  domain freeze/resume, capability send/revoke, DMA completion/fault, storage
  barrier completion/failure, and structured fault events.
- readable through a control FDR with destructive or snapshot read mode.
- overflow is explicit: records include wrap generation and dropped-count
  metadata.

### 24.3.1 Assured Deployment, Audit, Debug, and MLS Hooks

Assurance profile is a boot/domain policy field:

- `ASSURANCE_DEV`: non-production quotes/audit; board policy may allow debug.
- `ASSURANCE_FIELD`: measured boot, metadata ECC/parity, watchdogs, telemetry,
  locked debug, tenant-strict domains, audit stream.
- `ASSURANCE_HIGH`: signed bitstream/manifest policy, production quotes, audit
  roots, MLS labels, measured debug unlock, no ambient device/interrupt/DMA.
- `ASSURANCE_FORMAL`: `ASSURANCE_HIGH` plus proof, theorem coverage, RTL/IP,
  synthesis, build, and toolchain identifiers in quote records.

Hardware is the Policy Enforcement Point for capabilities, domains, labels,
generations, lineage, measurements, VMAs, DMA/IOMMU scope, audit append,
debug gates, and commit points. PID 1, domain managers, personalities, and
services are Policy Decision Points.

Audit stream record fields:

- event class, domain/service id+generation, object id+generation, lineage
  epoch, label, bounded payload hash or redacted payload.
- monotonic sequence, dropped count, previous-record hash, current audit root.
- scope fields for domain subtree, label, event class, read mode, redaction.

Audit records are data. Overflow records a gap/dropped count. Audit roots are
quoteable.

Debug/forensics rules:

- debug authority is a debug-control FDR.
- rights are distinct: halt/freeze, step, breakpoint, register read, memory
  read/write, trace read, crash snapshot, dump export, engine diagnostics.
- access is scoped by domain, label, object/range, generation, and profile.
- parent domains do not gain inspection rights by ancestry.
- production profiles may disable invasive debug or require destructive freeze.
- dumps leave only through FDRs and are redacted by tenant/confidential/MLS
  policy.

MLS hooks:

- domains, FDRs, memory objects, DMA buffers, packet queues, event queues,
  telemetry, audit streams, and endpoints may carry `label_id` and
  `label_generation`.
- stale/unknown label generation fails closed.
- cross-domain send/map/DMA/telemetry/debug/packet/page-fill/reply/cap-return
  operations must satisfy the active label relation.
- declassification uses explicit call gates or control FDRs, emits audit, and
  returns authority only through the Capability Engine.

Mission profile fields:

- mission id, minimum assurance, audit/attestation level, dependency graph hash,
  degraded-mode bitmap, recovery priority, stale event/time budget, fail policy.
- dependencies are explicit FDRs: storage, network, sensor/device, telemetry,
  audit, supervisor, declassification, fallback, recovery.
- states: `normal`, `degraded`, `recovering`, `frozen`, `failed_closed`,
  `quarantined`.
- triggers: service restart, stale generation, watchdog fault, revocation, audit
  failure, attestation failure, device fault, label violation, budget exhaustion,
  policy denial, supervisor command.

Recovery/failover cannot broaden authority. Fallback services must already be
delegated. Restarted services receive new generations. Mission-state changes
emit fault/audit events and update quoteable mission evidence.

Open-assurance hooks:

- boot/quote policy may name owner, organization, vendor, development, or
  unsigned-development trust roots.
- no architectural profile requires vendor-exclusive keys, vendor debug unlock,
  remote kill switch, mandatory signed-only execution, hidden management,
  ambient vendor telemetry, secret DMA, or raw interrupt backchannel.
- quote records may include public RTL/source hashes, reproducible bitstream and
  toolchain manifests, proof artifacts, and service image hashes.
- owner-held debug-control FDRs are allowed in open-owner profiles.
- loader, filesystem, network, personality, telemetry, domain-manager, and
  declassification services are replaceable under normal capability, lineage,
  label, audit, and domain checks.

### 24.4 Watchdogs and Local Engine Reset

Each long-latency engine has a watchdog budget in cycles or fabric ticks.

Watchdog behavior:

- before commit, a timed-out operation aborts if the engine can prove no
  architectural state was published.
- after commit, the engine must complete, roll forward, or enter degraded mode;
  it may not silently roll back published state.
- local reset drains or cancels ingress queues, invalidates local caches, reloads
  protected metadata if safe, increments reset counters, and emits a fault
  event.
- engines whose owner metadata is corrupt enter degraded mode and reject new
  commands until supervisor/PID 1 policy clears or reinitializes them.
- watchdogs are not normal flow control. Persistent timeouts are treated as RAS
  faults and are visible in counters and the trace ring.

## 25. FPGA Resource Strategy

Likely expensive blocks:

- Namespace dispatch/reply-continuation engine.
- VMA and page table walker.
- DDR-backed FDR table cache and descriptor walkers.
- Resource Domain active-window and accounting caches.
- multi-context register storage.
- Hardware Heap Engine metadata caches.
- DMA buffers.
- SD and Ethernet adapters.

To keep v1 feasible:

- Use one shared POSIX engine pipeline rather than duplicating per thread.
- Prefer fewer owner engines with strong local state over many small engines that
  submit independent DDR requests.
- Keep hot state in registers or FPGA RAM wherever practical; use DDR for
  backing/spill and cold metadata.
- Route metadata access through semantic owner-engine requests instead of
  letting every module become a DDR table walker.
- Limit path/control request size and component count before dispatch.
- Bound FPGA-local active windows for files, processes, threads, VMAs, and wait
  queues while keeping architectural state in DDR.
- Use DDR for large tables and FPGA RAM for hot caches.
- Keep Ethernet simple.
- Use a small number of in-order coherent core tiles.

Suggested v1 limits:

```text
core tiles:                      2-4 coherent in-order tiles for FPGA v1
active hardware thread contexts: 64-256 on chip, DDR-backed spill
architectural threads:           DDR-backed, at least 16384 system-wide
process contexts:                DDR-backed, at least 4096 architectural PIDs
resource domains:                DDR-backed, at least 4096 domains
domain nesting depth:            at least 16 architectural levels
fdrs/process:                    DDR-backed, default 4096, expandable higher
pending events/process:          DDR-backed event queues, at least 4096
futex buckets:                   4096+ global hash buckets, DDR-backed waiters
vmas/process:                    DDR-backed, at least 4096
path bytes:                      4096
path components:                 256
open objects:                    DDR-backed, at least 262144 system-wide
pipe/queue profile buffers:      DDR-backed, 64 KiB default, resizable
heap algorithm:                   LNP64 Default Heap Algorithm
heap size classes:                fixed by implementation profile; query coarse limits with ENV_GET
per-thread heap windows:          on-chip active windows, DDR-backed slab/run metadata
```

## 26. Verification Plan

Verification should start at the architectural level before RTL:

- Build an instruction encoding/decoding golden model.
- Extend the current Rust emulator to consume encoded 64-bit instructions.
- Add traces for thread scheduling, FDR table transitions, VMA changes, and
  signal delivery.
- Write directed tests for every native resource instruction and for the POSIX
  compatibility profiles layered over them.
- Write state-machine invariant tests for every hard block: legal state
  transitions, invalid-state detection, commit/abort behavior, timeout recovery,
  and reset recovery.
- Write directed tests for `ENV_GET` scalar keys, buffer keys, bad keys, and
  buffer faults.
- Write directed tests for hardware fault-to-signal mapping: `SIGFPE`, `SIGILL`,
  `SIGSEGV`, `SIGBUS`, and `SIGTRAP`.
- Write directed tests for the weighted-fair scheduler: fixed weight table
  charging, virtual runtime/deadline ordering, bucket/window approximation,
  quota exhaustion and replenishment, hierarchy rollup, wakeup insertion,
  bounded preemption latency, call-gate charge policy, affinity masks, frozen
  domain removal, and no scheduler plugin/callback path.
- Write directed tests for `OBJECT_CTL` `counter`, `queue`, and `memory_object`
  primitives plus profile mappings for semaphores, channels, task events,
  completions, shared arenas, and capability delegation.
- Write directed tests for `DMA_CTL` copy, fill, scatter/gather, completion
  events, cancellation, permission faults, and cache-coherence behavior.
- Write directed tests for the Object-Backed Page Transaction Protocol: request
  shape, page/zero/shared/retry/error replies, VMA/object generation mismatch,
  lineage mismatch, permission and memory-type narrowing, executable provenance,
  pending-fill cancellation, dirty-range enumeration, service-owned `msync`,
  and timeout/fault events.
- Write directed tests for `DOMAIN_CTL`: nested create/destroy, monotonic
  resource limits, hierarchical accounting, freeze/resume, capability
  delegation/revocation, stale generation rejection, checkpoint/query-state
  records, dirty-memory tracking hooks, upcall masking, tenant-strict profile
  enforcement, parent-inspection denial, scoped telemetry, and confidential-mode
  refusal when production encryption is unavailable.
- Write directed tests for `CALL_CAP`/`RET_CAP`: same-domain cross-thread calls,
  cross-domain call gates, stale gate generation rejection, budget accounting,
  synchronous return continuation handling, asynchronous completion delivery,
  handoff cancellation ownership, reentrant-depth limits, and denied capability
  passing.
- Write directed tests for critical metadata ECC/parity injection: correction,
  poisoning, generation preservation, and structured fault delivery.
- Write directed tests for watchdog timeout and local engine reset before and
  after commit points.
- Write directed tests for telemetry capabilities: observability counters,
  aggregate-only views, redacted tenant views, trace-ring overflow, snapshot
  reads, destructive reads, and rejection without delegated telemetry FDRs.
- Write directed tests for measured boot and attestation metadata: build id,
  reset cause, bitstream/ROM identity, manifest/image/domain measurement log,
  boot policy failure behavior, quote FDR shape, development quote flag, and
  capability-root binding.
- Write directed tests for assurance profiles: `ASSURANCE_DEV`,
  `ASSURANCE_FIELD`, `ASSURANCE_HIGH`, and `ASSURANCE_FORMAL` profile bits,
  minimum-profile domain admission, quote binding for proof/IP/toolchain/build
  metadata, and rejection when mandatory profile controls are absent.
- Write directed tests for tamper-evident audit streams: monotonic sequence,
  hash-chain root, dropped-count/gap records, narrowed audit FDR disclosure,
  quoteable audit roots, destructive/snapshot reads, and no authority encoded in
  audit payloads.
- Write directed tests for controlled debug and forensics: debug-control FDR
  rights per operation, measured/audited unlock, production debug lockout,
  domain/range/label scoping, parent-inspection denial, destructive freeze
  policy, and crash-dump redaction.
- Write directed tests for MLS/cross-domain labels: stale label generation
  rejection, denied cross-label `CAP_SEND`, `MMAP`, DMA, debug, telemetry,
  service reply, and returned-capability installs, explicit audited
  declassification gates, and unlabeled-object policy.
- Write directed tests for mission assurance profiles: minimum-assurance
  admission, dependency graph hash validation, dependency-as-FDR enforcement,
  mission-state transitions, fail-closed dispatch denial, failover without
  authority broadening, stale service-generation rejection, audit/fault emission,
  stale-data budget enforcement, and quoteable mission evidence.
- Write directed tests for owner sovereignty and open assurance: owner-key boot
  policy, unsigned development policy with explicit non-production quote,
  reproducible artifact hash fields, owner-held debug-control FDR unlock, no
  vendor-exclusive key requirement, no hidden management/telemetry/debug/DMA
  path, and replacement service image authority checks.
- Write directed tests for storage barriers: data sync, metadata sync,
  barrier-after-commit ordering, backend flush failure, and replay/fsck-visible
  commit records.
- Write directed tests for namespace dispatch: authority validation, bounded
  path slices, service reply continuation, returned-capability verification,
  narrowing, generation rejection, revoked service rejection, and optional
  service-approved lookup-cache invalidation.
- Write directed tests for the Record Classification and Queue Steering Engine:
  exact/masked/prefix/range matches, hash steering, counter updates,
  capability-scoped destination queues, overflow events, malformed record
  handling, packet profile parsing, IPC/event/storage completion profiles, and
  rejection of unbounded or unauthorized rules.
- Write randomized tests for invalid FDs, bad paths, page faults, and killed
  blocked threads.
- Run the same binaries against emulator and RTL simulation.

RTL simulation milestones:

1. Fetch/decode/ALU/load/store from DDR model.
2. weighted-fair multi-context scheduler with `CLONE`, `YIELD`, `AWAIT`,
   `EXIT`, virtual-time accounting, quotas, and bounded wakeup insertion.
3. FDR table plus UART `PULL`/`PUSH`.
4. default operating envelope plus boot manifest image loading by offset/hash:
   root/PID 1 domains, scheduler profile, security defaults, telemetry/fault
   FDRs, measurements, and explicit initial grants.
5. Namespace Dispatch Engine with service-domain `OPEN_AT`, `NS_CTL`,
   `GET_META`, and `SET_META` request/reply capability installation.
6. `MMAP`, page-state transitions, anonymous COW, and object-backed page-fill
   transactions.
7. Hardware Heap Engine: `ALLOC`, `ALLOC_EX`, `FREE`, invalid free detection,
   fork COW, exec teardown, and cross-thread frees.
8. `CLONE`, copy-on-write, and child-exit `AWAIT`.
9. `EXEC` commit from a boot-manifest or loader-produced exec-plan descriptor.
10. signals, hardware fault delivery, `ENV_GET`, and futexes.
11. Ethernet packet objects.
12. explicit-offset `PULL`/`PUSH` block-image object.
13. generic runtime objects with `OBJECT_CTL`, `PULL`/`PUSH`, `AWAIT`, and
    `CAP_*`.
14. `DMA_CTL` memory copy/fill/scatter-gather with event completion.
15. Resource Domains with `DOMAIN_CTL`: nested limits, freeze/resume, usage
    accounting, and capability delegation.
16. `CALL_CAP`/`RET_CAP` same-domain and cross-domain call gates.
17. supervisor-domain control FDR and upcall delivery as a Resource Domain
    profile.
18. minimal paravirtual Unix personality that boots a guest init process over
    native LNP64 tasks and a block-image filesystem.
19. Linux syscall compatibility runtime for a static userland smoke test:
    open/read/write/mmap/futex/clock/wait/exec paths mapped to native opcodes.
20. NetBSD rump-kernel style filesystem service over a block-image FDR, exposed
    back through delegated native FDRs.
21. PCIe Root Complex smoke test with Bus Master config-space enumeration.
22. page-granular `pcie_bar` FDR minting and `MMAP` to device-ordered and
    write-combining VMAs.
23. `dma_buffer` FDR export through IOMMU and revocation after DMA quiesce.
24. MSI/MSI-X delivery through `irq_event` FDRs and `AWAIT`.
25. simple NVMe or NIC driver domain using BAR `LD`/`ST`, DMA buffers, and IRQ
    events to publish high-level block or network FDRs.
26. RAS smoke tests: metadata ECC/parity fault injection, structured fault
    events, watchdog/local-reset recovery, observability counters, and trace
    ring reads.
27. measured boot and attestation skeleton: build id/reset cause keys,
    bitstream/ROM identity, manifest/image/domain measurement log,
    boot-control FDR, quote FDR, and development quote flag.
28. storage service durability smoke test: service metadata commit log,
    flush/barrier ordering, reset replay, and service-level atomic rename
    persistence over block-image FDRs.
29. domain checkpoint hook smoke test: freeze to quiescence, query bounded
    state records, resume without generation churn, and reject future
    reattachment records with invalid capability lineage.
30. Record Classification and Queue Steering smoke test: packet envelope
    metadata, flow hash steering, packet queue routing, IPC completion routing,
    storage/DMA fault routing, counters, and bounded-rule rejection.
31. tenant-strict/confidential-domain hook smoke test: mandatory hardening bits,
    no parent memory inspection without a capability, scoped telemetry,
    explicit shared pages, measured launch records, sealed-secret refusal on
    measurement mismatch, and production confidential-mode refusal when the
    implementation lacks memory encryption.

## 27. Main Architectural Risk

The hard part is not the integer CPU. The hard part is keeping historical Unix
compatibility useful without letting historical Unix become the whole hardware
model. LNP64 v1 should deliberately define a small native substrate:
capability handles, waitable objects, VMAs, Resource Domains, hardware
scheduler contexts, event queues, call gates, and typed metadata/control
surfaces.

POSIX is the primary compatibility profile over that substrate. The compiler,
libc shim, Linux syscall runtime, and paravirtual personalities should target
that profile rather than assuming every Linux behavior is replicated in
hardware. When a POSIX feature is awkward historically, such as fork, signals,
global paths, UID/GID, or ioctl-like controls, the native primitive remains the
cleaner capability/event/domain operation and the compatibility layer performs
the translation.

The core architectural bet is that resource operations are capability-checked
hardware commands that park threads and let the scheduler run other work. That
keeps the ISA promise: files, streams, memory mappings, waitables, domains,
devices, and service calls are real hardware-visible resources, not software
calls with different names.
