# LNP64 FPGA Hardware Design Sketch

This document sketches a first real FPGA implementation of LNP64. It is not RTL
and it is not a module skeleton. It is an architectural design target for a large
FPGA with no built-in CPU cores. The central goal is to realize the POSIX-like
ISA instructions as hardware datapaths and hardwired state machines, not as
software traps and not as a hidden microcoded processor.

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
- File descriptor registers as real hardware capability entries.
- POSIX-like file, directory, process, signal, futex, mmap, exec, fork, and
  scheduling instructions from day one.
- True multi-context hardware scheduling with a real hardware runqueue.
- Coherent multicore execution across multiple fabric CPU tiles.
- External DDR virtual memory with hardware-managed translation and VMAs.
- Hardware-backed UART, SD, SPI flash, and simplified Ethernet file objects.
- PCIe host support through a hardware Root Complex, IOMMU, MSI routing, and a
  privileged software Bus Master domain that mints FDR capabilities.
- Deeply nestable Resource Domains for virtualization, containers, cgroups,
  jails, sandboxes, and supervisor upcalls, without adding traditional hosted-OS
  rings or syscall traps.
- Native security invariants: W^X, NX data defaults, ASLR, guard pages,
  hardware entropy, generation-checked objects, revocation, sealed/narrowed
  capabilities, and DMA isolation as Resource Domain and capability policy.
- Deterministic instruction decode with a fixed binary encoding.
- Hardware-owned waitable/capability objects with local state, bounded
  transitions, and event delivery, usable by ordinary runtimes as well as
  POSIX-like OS operations.
- Hardware modules designed as small, explicit, enumerated-state machines where
  invalid states are unrepresentable or detected by construction.

The v1 design is allowed to be slow for complex POSIX operations. For example,
`EXEC` can take thousands or millions of cycles while the SD controller streams
an ELF image. The important requirement is that the operation is performed by
dedicated hardware controllers and the issuing thread is parked while other
threads continue.

A major design goal of the POSIX hardware modules is robustness, not just speed.
Compared with software subsystems, a good hard block should have a smaller
reachable bad-state space: explicit states, bounded transitions, protected
metadata, generation/check bits, and commit/abort paths that prevent partial
architectural publication.

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
- Silicon VFS Namespace Engine.
- File Operation Engine.
- Directory stream datapath inside the VFS/File Operation Engine.
- Process Engine.
- Signal Engine.
- Futex and Atomic Engine.
- Resource Domain Engine.
- Supervisor Domain and Upcall Engine.
- Entropy and Randomization Engine.
- PCIe Root Complex.
- PCIe IOMMU / DMA Remapper.
- PCIe MSI/MSI-X Event Router.
- DMA Fabric.
- Device adapters for UART, SD card, SPI flash, and Ethernet.
- DDR Memory Controller Interface.
- Interrupt and Event Router.

All long-latency POSIX instructions issue a command into a hardware engine and
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
- heap size-class magazines and per-thread allocation caches.
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
  |      +--> VFS Namespace Engine <-----> VFS metadata/object tables
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

Large or sparse events are carried as DDR-backed event records. The wire only
announces that an event record exists and identifies the active-window slot or
queue id.

Shared table access:

- FDR, process, thread, VMA, heap, futex, event-queue, and VFS metadata live in
  DDR-backed tables with small on-chip caches.
- Each table has one owning engine that arbitrates mutation. Other engines
  access it through request channels or read-only cached snapshots.
- Non-owner engines must not independently walk or mutate another engine's DDR
  tables. They request `validate_fd`, `pin_user_buffer`, `lookup_path_component`,
  `allocate_pages`, `enqueue_event`, or similar semantic operations.
- Object locks are fixed hardware locks or scoreboard bits on table entries, not
  software mutexes. Locks must have bounded acquisition, timeout/cancel behavior,
  and deadlock ordering documented per engine.
- A command that needs multiple objects acquires them in this global order:
  process/thread, FDR, VMA, heap, VFS object, device/DMA. If an engine cannot
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
- VFS/Object Engine: hot cwd/root/open-object metadata, recent directory-entry
  cache, object lock scoreboard, and fixed metadata commit FSMs. Cold path
  namespace walks may use DDR, but hot `OPEN_AT` and metadata operations should
  avoid broad DDR traversal.
- File Operation Engine: stream transaction compiler. Given a validated FDR and
  pinned buffer, it updates stream state, emits DMA/FIFO/packet descriptors, and
  posts completion. It does not independently walk process, FDR, VMA, or VFS
  tables.
- Directory datapath: subtype lane for directory streams, not a separate DDR
  walker. It handles dirent packing, directory cookies, end-of-directory, and
  stable iteration rules over cached directory pages.
- VMA/Page Engine: TLB miss handling, cached recent VMA ranges, COW/page-fault
  classification, buffer pinning, and invalidation broadcast. Tree/array walks
  are cold/refill paths.
- Heap Engine: per-thread/per-core magazines and small-allocation size classes
  in local memory. Common `ALLOC`/`FREE` must not touch DDR metadata.
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
- formal or exhaustive tests cover each module's state-transition graph before
  RTL freeze.

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

### 9.3 Memory Ordering

LNP64 v1 uses a conservative ordering model:

- each core observes its own loads and stores in program order.
- ordinary stores become globally visible when accepted by the coherent cache
  fabric.
- `FENCE` drains the issuing core's store buffer, completes prior DMA visibility
  requirements, and waits for coherence acknowledgements.
- atomic operations are sequentially ordered per physical address.
- POSIX engine completions are ordered after their DMA writes and metadata
  updates.

This is stronger than many commercial relaxed models, but it reduces the risk of
subtle hardware/software mismatches in the first implementation.

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

This rule is mandatory for `PULL`, `PUSH`, file-backed page faults,
Ethernet RX/TX, SD/SPI transfers, and PCIe DMA.

Hard invariant: no device may write DDR through a path that bypasses the
L2-coherent DMA fabric. PCIe requester traffic enters through the Root
Complex/IOMMU coherent bridge. If a target FPGA cannot provide coherent PCIe DMA
at the L2 boundary, the implementation must use explicit cache clean/invalidate
operations in the DMA Fabric before delivering completion events; it may not
advertise "coherent by construction" for PCIe.

### 9.7 Shared POSIX Metadata

FDR tables, process tables, VMA descriptors, VFS namespace nodes, inode metadata,
pipe buffers, socket queues, and wait queues are shared hardware data
structures. They are protected with hardware locks or single-writer engine
ownership:

- FDR table entry updates are serialized per process and fd index.
- VFS namespace mutation is serialized per directory object.
- process table mutation is serialized per PID slot.
- runqueue updates are serialized by the scheduler fabric.
- pipe and socket queue updates are serialized per queue object.

The hardware engines may be internally pipelined, but they must expose atomic
architectural transitions to threads.

## 10. External DDR Memory Model

External DDR holds:

- Program text and data.
- Stacks.
- Heaps.
- File cache pages.
- Page tables.
- VMA descriptors.
- Process tables.
- VFS namespace nodes.
- Directory entries and file metadata cache.
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

On a page fault, the issuing thread is parked. The VMA Engine decides whether to
allocate a zero page, fetch a file-backed page through DMA, signal `SIGSEGV`, or
complete a copy-on-write break.

### 10.1 Frozen Memory Model Constants

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
- device mappings are never executable.
- device mappings are created only by `MMAP` on an FDR whose object class grants
  device memory authority, such as `pcie_bar`.
- after a device mapping is installed, ordinary `LD` and `ST` instructions use
  the TLB/PTE memory type; there is no FDR lookup or capability check on every
  device access.

`FENCE` semantics:

- drains prior stores from the issuing core into the coherent fabric.
- waits for invalidation acknowledgements required by prior stores.
- orders prior DMA-visible writes before later DMA or device operations.
- orders device MMIO loads/stores against DMA and normal memory when required
  by the mapped memory type.
- orders prior POSIX engine completions before later ordinary memory operations.
- does not flush unrelated cache lines.

## 11. Hardware Scheduler and Runqueue

The scheduler is a fabric block, not software. In the coherent multicore design,
it has per-core ready queues plus a global scheduler arbiter.

State:

- Per-core ready queues of TIDs.
- Global runnable overflow queue.
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

- `YIELD`: moves current TID to the tail of ready queue.
- timer-flavored `AWAIT`: inserts current TID into timer wheel.
- `AWAIT`: attaches current TID to a waitable object's event mask or predicate.
- long POSIX operations: mark current TID blocked on an engine command.
- engine completion: writes result registers, updates errno, returns TID to
  ready queue unless a signal must be delivered first.

Each core-local scheduler chooses the next ready TID every cycle if available.
The global arbiter handles wakeups, new threads, thread migration, load
balancing, and work stealing. It should use round-robin in v1. Priority can be
added later.

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

The same hard blocks that make POSIX operations native should also accelerate
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
  `block_device`, `pipe_read`, `pipe_write`, `socket`, `listener`,
  `event_queue`, `timer`, `counter`, `queue`, `memory_object`, `call_gate`,
  `control`, `pci_function`, `pcie_bar`, `dma_buffer`, `irq_event`,
  `gpu_device`, `accelerator`.
- backend id: `none`, `uart0`, `sd0`, `spi_flash0`, `eth0`, `ramfs`,
  `pipe_engine`, `socket_engine`, `vfs_engine`, `supervisor_engine`,
  `pcie_root`, `pcie_iommu`, `pcie_msi`, `nvme_driver`, `ethernet_driver`,
  `gpu_driver`.
- protocol or subtype: `raw_frame`, `udp_datagram`, `stream`, `block_extent`,
  `block_image`, `tty`, `control`, `pci_config`, `bar_mmio`,
  `timer_oneshot`, `timer_periodic`, `msi_vector`, `msix_vector`,
  `pinned_dma`, `framebuffer`, `bounded_records`, `counting`,
  `single_assignment`, `runtime_task`, `shared_arena`, or
  backend-defined.
- rights: read, write, seek, stat, directory, execute, poll, wait, signal, map,
  dma, transfer, call, return.
- object id.
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

Every authority-bearing FDR entry includes an object generation. Cached
descriptor hits are valid only when the cached generation still matches the
object owner. This makes stale descriptor reuse, post-revocation use, and
destroy/recreate aliasing fail as `EBADF` or the object-specific stale-reference
error instead of silently targeting a new object.

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

Sealing and minting:

- a sealed FDR may be transferred but not narrowed, duplicated, or used to mint
  related capabilities unless the sealed rights allow it.
- only hardware engines and processes holding explicit mint capabilities can
  create new root capabilities.
- the PCIe Bus Master can mint `pci_function`, `pcie_bar`, `dma_buffer`, and
  `irq_event` FDRs only because reset grants it the PCIe Root Complex control
  FDR with mint rights.
- supervisor domains can mint delegated control/event FDRs only inside their
  assigned subtree.

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

## 13. Silicon VFS Namespace Engine

The VFS/Object Engine resolves paths and manages namespace metadata in hardware.
It is a local-state object engine first and a DDR namespace walker only on cold
paths.

Inputs:

- process cwd pointer.
- root namespace pointer.
- path string virtual address.
- operation type.
- flags.
- credential snapshot from PCRs.

Internal units:

- path string DMA reader.
- slash component tokenizer.
- directory entry lookup engine.
- permission checker.
- symlink resolver with depth limit.
- metadata allocator.
- object capability allocator.
- recent dentry/object cache.
- cwd/root/open-object register window.
- object lock scoreboard.
- fixed metadata commit FSM for namespace mutations.

The VFS namespace is stored in DDR as a compact tree of inode-like objects and
directory-entry arrays. Frequently used root, cwd, and open object metadata are
cached in FPGA RAM.

The path resolver is a hardwired FSM. It walks each component, performs directory
lookup, checks permissions, follows symlinks when permitted, and emits either an
object id or an errno.

Fast path target:

- `OPEN_AT` relative to cached cwd/root and a hot directory entry avoids a broad
  DDR namespace walk.
- `GET_META` and `SET_META` on open objects use cached object metadata and the
  object lock scoreboard.
- `NS_CTL` mutations use fixed commit FSMs for rename, link, unlink, mkdir,
  symlink, chmod/chown, and timestamp updates. The FSM may write journal/COW/log
  records, but the operation is not an arbitrary software-like metadata walk.

Directory handling is not a separate top-level DDR-walking engine. Directories
are stream objects with a directory datapath inside the VFS/File engines:

- cached directory pages feed `PULL` as ABI dirent records.
- directory cookies are tracked as stream positions.
- rewind is `SEEK(fd, 0, SET)`.
- stable iteration under concurrent mutation is enforced by object generation
  counters and cached directory-page pins.

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

The SD adapter provides block storage for the primary filesystem. The hardware
does not need a complete commercial filesystem in v1; it can use an LNP64-native
simple filesystem:

- fixed-size superblock.
- inode table.
- extent lists.
- directory entry arrays.
- symlink payload blocks.

The File Operation Engine translates VFS object ids and offsets into SD block
DMA commands. The SD adapter streams sectors to and from DDR buffers.

### 14.3 Static Filesystem Format Options

The v1 storage format is not frozen yet. The hardware needs a format simple
enough for deterministic path walking, `EXEC`, `MMAP`, metadata mutation, links,
symlinks, permissions, and crash recovery. Candidate options:

Option A: LNPFS, a purpose-built extent filesystem.

- Hardware-friendly fixed-endian superblock, inode table, extent arrays, and
  directory-entry blocks.
- Directly matches the VFS engine's object ids and metadata cache.
- Easiest option for real hardware path resolution and `EXEC`.
- Weakness: requires custom tooling and is not directly mountable on normal
  hosts without a userspace tool.

Option B: read-only boot filesystem plus mutable append log.

- Boot image contains a simple read-only tree for `/sbin/init`, libraries, and
  base files.
- Runtime mutations append records to a hardware-readable log.
- Simplifies early boot and recovery.
- Weakness: rename, unlink, chmod, chown, and directory compaction become
  log-replay operations, so long-running systems need garbage collection.

Option C: FAT-like filesystem profile.

- Easier host-side image creation and debugging.
- Simple block layout, but weak POSIX metadata.
- Requires hardware side tables for uid, gid, mode, symlink, hard link, and
  inode-like behavior.
- Better as an import/export or bootstrapping format than as the main cloud
  filesystem.

Option D: ext2-like restricted profile.

- Existing concepts for inodes, directories, permissions, links, and symlinks.
- Host tooling is easier than a custom format.
- Weakness: path walking and allocation logic are more complex in hardware than
  an LNP64-native extent format.

Recommended direction before freeze: use LNPFS for the native writable root
filesystem, plus a read-only SPI flash boot image format for recovery. Keep a
host-side image builder mandatory from day one.

Crash recovery requirement:

- Live VFS metadata commit points are not sufficient for power-fail safety.
- The selected writable filesystem must define either journaling, copy-on-write
  metadata, or an append-log protocol before RTL freeze.
- Atomic rename, link/unlink, chmod/chown, directory creation, symlink creation,
  and allocation bitmap changes must have explicit replay/fsck rules.
- `FENCE` and storage write barriers must be sufficient to order metadata
  commits against SD/SPI/PCIe block-device flush completion.
- Until this is specified, LNPFS is a format direction, not a frozen storage
  format.

### 14.4 SPI Flash

SPI flash is used for boot ROM assets and optional read-mostly files. It exposes
a block-like backend with slower writes. The boot path may fetch initial VFS
metadata and `/sbin/init` from SPI flash if SD is absent.

### 14.5 Simplified Ethernet

Ethernet v1 is a simplified packet device, not a full TCP/IP offload engine.

Supported model:

- raw frame RX/TX queues.
- optional UDP-like datagram objects.
- listener objects are event queues over configured ethertype/port filters.
- `PULL` receives frames/datagrams into user buffers.
- `PUSH` transmits frames/datagrams from user buffers.

The VFS can expose network endpoints under paths such as `/dev/eth0` or
`/net/udp/<port>`.

### 14.6 PCIe Host Support

PCIe support preserves the POSIX-native model by exposing devices as FDR
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
- mint `pci_function`, `pcie_bar`, `dma_buffer`, and `irq_event` FDRs.
- delegate those FDRs to driver processes through normal capability passing.
- publish higher-level device FDRs such as block, network, GPU, or accelerator
  objects under the hardware VFS.

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
the rest of the system still sees devices as POSIX-native capabilities.

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
- stream/object state token from the VFS/Object Engine when needed.
- backend queue availability from UART, SD, SPI, Ethernet, PCIe driver, pipe,
  socket, event, timer, or control-FDR adapters.

Its local state is intentionally small:

- active stream offset/cookie window.
- short per-backend issue queues.
- DMA descriptor staging registers.
- packet/block/FIFO byte counters.
- completion op id and result-register tags.

Fast path target: cached FDR plus pinned buffer becomes one DMA/FIFO/packet
descriptor and one completion event. Directory reads use the directory datapath
to pack dirents from cached directory pages; they do not route through a
separate directory walker.

FDR operand conventions:

- `OPEN_AT`: F9. Resolves a path/name relative to a directory/root FDR and
  returns a resource FDR index. Source-level `open`, `openat`, `opendir`, and
  older draft `OPEN_FD`/`OPEN_DIR` names lower to this opcode.
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
- `NS_CTL`: F9. Performs namespace mutations relative to directory FDRs:
  `mkdirat`, `unlinkat`, `renameat`, `linkat`, `symlinkat`, `readlinkat`,
  `chdir`, and delegated namespace operations.
- `DUP`: uses F7/F9 as needed and always names an encoded result register. It
  may overwrite explicit destination descriptors only when the opcode variant
  says so.
- source-level `pipe()` lowers to `OBJECT_CTL create queue(profile=pipe)` plus
  capability narrowing into read and write endpoint FDRs. There is no separate
  v1 hardware `PIPE` primitive.
- Source assembly may omit `result_dst` for legacy readability; the assembler
  inserts `r1`, but the binary result register is always explicit.

`OPEN_AT`:

- read path string through MMU.
- resolve path relative to a directory/root FDR or the process cwd/root.
- check rights and flags.
- allocate or overwrite an FDR capability entry according to flags.
- return descriptor index in the encoded result register.

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
- cover stat, chmod, chown, utime, fd flags, object rights queries, and
  backend-specific metadata.
- path-oriented source forms lower to `OPEN_AT` plus metadata operations where
  possible.

`NS_CTL`:

- handles operations that necessarily name directory entries: rename, unlink,
  mkdir, link, symlink, readlink, chdir, and delegated namespace controls.
- uses directory/root FDRs and name buffers rather than direct global `_PATH`
  opcodes.

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
- creates a new thread or process according to share flags.
- can share or copy address space, FDR table, cwd/root namespace, credentials,
  signal handler table, and heap metadata.
- thread-like source forms lower to `CLONE` with shared address space and
  process resources plus a new entry PC/stack.
- fork-like source forms lower to `CLONE` with a new PID, copied FDR table refs,
  copied process metadata, and copy-on-write VMA/heap metadata.
- writes child TID/PID to the parent result register and zero to the child
  result register when using fork-compatible variants.
- enqueues the child thread when creation commits.

`CLONE` v1 share flags include:

- share address space.
- share FDR table.
- share cwd/root namespace.
- share credentials.
- share signal handler table.
- create new PID.
- set entry PC from argument block.
- allocate new stack VMA or use supplied stack pointer.

`THREAD_JOIN`:

- uses F8: `a=result_dst`, `b=target_tid_reg`, `c=retval_ptr_reg`.
- waits on a same-process thread completion record.
- parks the caller in the scheduler rather than spinning when the target thread
  is still live.
- writes the target thread's exit value to `retval_ptr` when nonzero.
- returns `0` on success or a POSIX-style error code such as `ESRCH` or
  `EDEADLK`.

`EXEC`:

- enters a process-wide exec barrier.
- prevents new threads from being spawned in the process.
- stops all sibling TIDs at scheduling boundaries or via forced scheduler park.
- cancels or detaches in-flight operations according to the cancellation rules.
- invalidates sibling thread contexts so exactly one thread survives the exec.
- resolves executable path through VFS.
- validates execute permission.
- streams ELF headers through File Operation Engine.
- tears down old VMAs except preserved process resources.
- builds new text, data, heap, stack, and arg VMAs.
- DMA-loads program segments from storage into DDR.
- resets PC, LR, SP, registers, thread-local `ERRNO`, and signal state as
  specified by LNP64 ABI.
- preserves PID, parent, cwd, selected FDRs, uid/gid.
- exits the exec barrier and enqueues the single surviving thread.

`EXEC` uses F9 with this v1 argument block:

```text
u32 version
u32 flags
u64 path_ptr
u64 argv_ptr
u64 envp_ptr
u64 auxv_ptr
```

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
- marks pages nonresident for file-backed mappings or zero-fill for anonymous.
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
- is required for ELF loaders, language runtimes, JIT policy, guard pages, and
  paravirtual Unix guests mapping their own process abstractions onto LNP64 VMAs.

The VMA tree can be a hardware-walked B-tree or interval tree in DDR. For FPGA
v1, a sorted VMA array per process is acceptable if bounded and checked in
hardware.

The VMA/Page Engine must also have a clear local fast path:

- recent VMA range cache keyed by PID/ASID and virtual page range.
- active process VMA root pointer cached with the thread context window.
- small page-fault classification cache for resident, COW, file-backed, and
  guard-page cases.
- buffer pinning window for in-flight DMA descriptors.
- TLB and I-cache invalidation queues with acknowledgement bits per tile.

Fast path target: TLB miss on a hot resident mapping, COW classification for a
recent VMA, DMA buffer pinning for a cached range, and range invalidation issue
do not require a full DDR VMA tree walk. DDR walks are refill/cold paths.

## 18. Hardware Heap Engine

`ALLOC`, `ALLOC_EX`, `ALLOC_SIZE`, and `FREE` are v1 architectural instructions
backed by the Hardware Heap Engine. They are the preferred userspace allocation
primitive. `malloc` implementations should lower to these instructions by
default. `MMAP` remains the primitive for page mappings, files, shared memory,
executable memory, DMA buffers, and device mappings.

Heap backing VMAs are NX by default. Guarded allocations use VMA guard regions
or heap-local guard slots depending on size and policy. Heap metadata includes
generation fields so stale or freed pointers can be rejected by hardened
profiles before an allocation slot is reused silently.

`ALLOC`:

- uses F2: `a=result_dst`, `b=size_reg`.
- allocates from the current process's default heap.
- returns a virtual pointer in `result_dst`, or `-1` with thread-local
  `ERRNO=ENOMEM`/`EINVAL`.
- returns memory aligned to at least 16 bytes.
- does not guarantee zeroed memory unless process heap policy says otherwise.

`FREE`:

- uses F2: `a=result_dst`, `b=ptr_reg`.
- frees an exact pointer previously returned by `ALLOC` or `ALLOC_EX`.
- returns `0` on success or `-1` with thread-local `ERRNO`.
- detects invalid pointers and double free when heap metadata is intact; v1
  returns `EINVAL` and may additionally deliver `SIGSEGV` if the process has
  heap-hardening policy enabled.

`ALLOC_SIZE`:

- uses F2: `a=result_dst`, `b=ptr_reg`.
- reads Heap Engine metadata for an exact allocation pointer.
- returns the allocation's usable byte extent in `result_dst`.
- returns `0` for null or unknown pointers in the emulator subset; hardware may
  return `-1` with `ERRNO=EINVAL` under stricter heap-hardening policy.
- lets libc implement `realloc` without copying beyond the old allocation's
  valid mapped extent.

`ALLOC_EX`:

- uses F9 with an argument block for runtime-quality allocation requests.
- supports size, alignment, flags, memory type, and runtime-defined allocation
  class/tag.

`ALLOC_EX` v1 argument block:

```text
u32 version
u32 flags
u64 size
u64 alignment
u64 memory_type
u64 allocation_class
u64 reserved0
u64 reserved1
```

`ALLOC_EX` flags:

- zeroed.
- nozero.
- guard_before.
- guard_after.
- debug_poison.
- prefer_locality.
- large_object.

Heap model:

- each process has a default heap created at process start.
- heap metadata lives in protected DDR and is not directly mapped writable by
  the process.
- heap arena bases and large-object mappings are randomized by default.
- small allocations use hardware-managed size classes and sub-page chunks.
- large allocations use page runs from anonymous VMAs.
- per-thread allocation caches and cross-thread free queues are allowed and
  expected for performance.
- the Heap Engine serializes metadata updates and is thread-safe across all
  threads in the process.
- fork-like `CLONE` marks heap backing pages copy-on-write and clones heap metadata with
  COW semantics.
- `EXEC` destroys old heap metadata and creates a fresh default heap for the new
  image.
- `MUNMAP` of heap-owned pages is illegal unless mediated by the Heap Engine.
- shared memory, executable memory, DMA memory, and device memory are not
  allocated by `ALLOC`; use `MMAP` and FDR-backed objects for those cases.

The Heap Engine is retained only if its common path is local:

- per-thread tiny free lists for very common size classes.
- per-core or per-active-process magazines for small allocations.
- local cross-thread free queues that drain in batches.
- central heap metadata used only on refill, drain, large allocation, hardening,
  fork COW, or error detection.
- large objects request page runs through the VMA/Page Engine rather than
  walking page metadata directly.

Fast path target: common `ALLOC`/`FREE` sizes up to the implementation's small
object threshold complete from local magazines without DDR metadata reads.

The design goal is cultural as well as technical: the native heap should be fast,
thread-safe, observable, and integrated with VMA/fork/exec policy well enough
that programmers and runtime authors are not tempted to write general-purpose
allocators in software.

## 19. Signals

The Signal Engine handles asynchronous delivery and synchronous hardware fault
delivery. LNP64 does not expose a software interrupt-vector table for ordinary
processes; the architectural delivery surface is POSIX signal state plus
`SIGRET`.

Per process/thread state:

- process-wide handler table.
- process-pending signal queue.
- thread-local signal mask.
- thread-local pending signal queue.
- per-thread saved signal context stack.

`SIGACTION` writes the handler table.

`SIGMASK_SET` updates the issuing thread's signal mask and may trigger immediate
delivery of newly unmasked pending signals.

`KILL` finds target PID/TID, appends pending signal state, and wakes the target
if it is in an interruptible wait.

Signal delivery:

- scheduler sees pending unmasked signal before normal issue.
- Signal Engine writes a saved context record.
- PC is replaced with handler address.
- signal number is written to the ABI argument register.
- `SIGRET` restores saved PC, flags, and registers.

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

- faulting PC.
- next PC where architecturally meaningful.
- signal number and POSIX-style signal code.
- bad virtual address or zero when not address-related.
- trapped instruction word for decode faults and debug tooling.
- saved flags plus GPR/FPR/VR state needed by the psABI.

Recoverable page faults are not delivered as signals immediately. The VMA Engine
first attempts hardware page-in, copy-on-write, or file-backed fault resolution.
Only failed or permission-denied faults enter the Signal Engine.

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

All VFS permission checks consume a snapshot of UID/GID from PCR state at command
issue time.

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
- `argc`.
- `argv_base`, `envp_base`, and `auxv_base`.
- indexed `auxv` entry.
- process personality id.
- boot manifest flags exposed to PID 1.

`GET_PCR` remains the authority and credential path. `ENV_GET` is read-only and
must not expose mutable privilege state except through ordinary public metadata
such as PID/TID when a runtime asks for it.

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

V1 freezes a hybrid cloud profile: POSIX-style UID/GID and permission bits for
compatibility, plus an explicit process credential capability bitmap for powers
that must not be represented by UID 0 alone.

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

Chosen model: hybrid cloud profile.

- Keep UID/GID and POSIX permission checks for compatibility.
- Represent privileged powers as hardware capability bits attached to process
  context.
- Require both UID/GID permission and specific capability bits for dangerous
  operations such as raw network access, mounting, adapter table loading,
  cross-user `KILL`, and process memory inspection.
- Chosen for v1 because it preserves POSIX shape while avoiding a single
  all-powerful root path in hardware.

The hybrid model is still capability-native. UID/GID participates in
compatibility decisions, but authority over files, devices, memory objects,
call gates, DMA buffers, and supervisor controls is carried by FDR capabilities
and Resource Domain policy.

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
- alter global VFS metadata outside normal file permissions.

PCIe delegation follows pure capability rules after bootstrapping. The Bus
Master is trusted because reset grants it the PCIe Root Complex and config-space
authority. Driver processes do not need a separate `driver_domain` bit to map a
BAR: possession of a valid `pcie_bar` FDR is the authority. The hardware VMA
engine checks only the FDR class, rights, page-granular bounds, and memory type
permissions at `MMAP` time.

### 21.2 Resource Domains, Virtualization, and Cgroups

Resource Domains unify virtualization, containers, cgroups, jails, sandboxes,
and supervisor domains. A Resource Domain is a nested hardware capability and
accounting container for a process subtree.

Each domain contains:

- parent domain id and generation.
- child domain table pointer.
- attached process/thread subtree root.
- resource limits and current usage.
- scheduler budget, weight, quota, and allowed core-tile mask.
- memory budget, VMA budget, heap budget, and page pressure counters.
- PID/thread count limit.
- FDR table limit and capability delegation root.
- namespace root/cwd delegation pointers.
- event queue and upcall policy.
- device, DMA, and PCIe capability scope.
- security policy bits: ASLR enable/disable constraints, JIT/loader W^X
  exception authority, executable-memory source policy, entropy quota, and
  hardening profile.
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

Virtualization is a Resource Domain profile:

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

### 21.3 Paravirtual Unix Guest Profile

LNP64 does not add a conventional hosted-OS profile with kernel rings, software
page tables, mandatory syscall traps, or an OS-owned scheduler. A future
Linux/NetBSD port is made plausible by treating the kernel as a paravirtual Unix
personality domain running on top of native LNP64 POSIX hardware.

The silicon remains authoritative for:

- hardware process and thread creation.
- runqueue scheduling and context storage.
- VMA creation, teardown, page faults, and copy-on-write.
- FDR capabilities and hardware VFS object references.
- signals, futex queues, fd readiness, and DMA completion.

The guest kernel/personality owns:

- Linux/BSD-specific process metadata.
- domain profiles for namespaces, cgroups, jails, credentials, and policy state.
- emulation of APIs not directly represented by LNP64 opcodes.
- Linux syscall-number compatibility where a syscall-compatible runtime is used.
- filesystem images mounted inside hardware VFS files.
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

Non-targeted approach:

- A full traditional Linux/NetBSD port that owns page tables, context switching,
  interrupts, and raw devices is not a v1 design target.

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
and backend `vfs_engine` or `supervisor_engine`. The control FDR exposes event
records through `PULL` and accepts policy commands through `PUSH`. This keeps
the mechanism inside the FDR model instead of introducing a traditional syscall
path.

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

- The hardware VFS may delegate a subtree to a supervisor domain.
- Native path resolution enters the domain policy only at configured delegation
  points.
- The guest may implement Linux mount namespaces, bind mounts, procfs-like
  synthetic trees, or BSD jail views above those delegated roots.
- Non-delegated hardware paths remain resolved directly by the Silicon VFS.

Block-image FDRs:

- A regular hardware file may be opened as an object class `block_device` with
  subtype `block_image`.
- The guest block layer uses explicit-offset `PULL` and `PUSH` rather than
  descriptor seek state.
- Linux ext4, NetBSD FFS, or other guest filesystems can live inside one or more
  large hardware VFS files.
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
that project their semantics onto native POSIX hardware, rather than forcing the
chip to become a conventional trap-and-kernel machine.

## 22. DMA Fabric

The DMA Fabric moves bytes between:

- DDR user buffers.
- DDR user buffers for memory-to-memory `DMA_CTL` copy/fill operations.
- SD card sector buffers.
- SPI flash streams.
- UART FIFOs.
- Ethernet RX/TX buffers.
- PCIe DMA buffers.
- VFS metadata buffers.

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

Reset sequence:

1. Hardware reset controller initializes FPGA-local RAM structures.
2. DDR controller calibration completes.
3. Page allocator marks DDR regions free or reserved.
4. VFS engine mounts boot backend from SD or SPI flash.
5. FDR table template binds `fd0`, `fd1`, `fd2` to UART.
6. If PCIe is present, Root Complex link training completes, but enumeration is
   deferred until a Bus Master executable is loaded.
7. The boot manifest is read from the boot backend. It names `/sbin/init` and
   may optionally name a Bus Master executable such as `/sbin/pcie-busmaster`.
8. If the manifest names a Bus Master, the boot engine creates a privileged
   process for it, grants the PCIe Root Complex control FDR, loads it with the
   same `EXEC` machinery, and parks it until PID 1 is ready to coordinate boot.
   If no Bus Master is named, PCIe enumeration is deferred to native userland.
9. Process Engine creates PID 1, TID 1, UID 0.
10. VFS resolves `/sbin/init`.
11. `EXEC` engine loads `/sbin/init` into PID 1.
12. Scheduler marks PID 1 and any boot-manifest Bus Master ready.
13. Fetch begins at PID 1 entry point.

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
- `CLONE` does not clone in-flight operation ownership into the child unless an
  explicit future variant defines that behavior.
- `EXEC` cancels or waits for all operations tied to the old address space
  before replacing mappings.

Operation classes:

- `PULL`, `PUSH`, Ethernet receive/transmit, UART waits, and file-backed
  page reads are interruptible before DMA commit and return `EINTR` if canceled.
- `NS_CTL` namespace mutations and `SET_META` metadata mutations become
  non-interruptible after the VFS engine reaches its serialized metadata commit
  point.
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

## 25. FPGA Resource Strategy

Likely expensive blocks:

- VFS path resolver.
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
- Limit path length and component count.
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
heap size classes:               hardware-managed, implementation-defined count
per-thread heap caches:          on-chip active windows, DDR-backed metadata
```

## 26. Verification Plan

Verification should start at the architectural level before RTL:

- Build an instruction encoding/decoding golden model.
- Extend the current Rust emulator to consume encoded 64-bit instructions.
- Add traces for thread scheduling, FDR table transitions, VMA changes, and
  signal delivery.
- Write directed tests for every POSIX instruction.
- Write state-machine invariant tests for every hard block: legal state
  transitions, invalid-state detection, commit/abort behavior, timeout recovery,
  and reset recovery.
- Write directed tests for `ENV_GET` scalar keys, buffer keys, bad keys, and
  buffer faults.
- Write directed tests for hardware fault-to-signal mapping: `SIGFPE`, `SIGILL`,
  `SIGSEGV`, `SIGBUS`, and `SIGTRAP`.
- Write directed tests for `OBJECT_CTL` `counter`, `queue`, and `memory_object`
  primitives plus profile mappings for semaphores, channels, task events,
  completions, shared arenas, and capability delegation.
- Write directed tests for `DMA_CTL` copy, fill, scatter/gather, completion
  events, cancellation, permission faults, and cache-coherence behavior.
- Write directed tests for `DOMAIN_CTL`: nested create/destroy, monotonic
  resource limits, hierarchical accounting, freeze/resume, capability
  delegation/revocation, stale generation rejection, and upcall masking.
- Write directed tests for `CALL_CAP`/`RET_CAP`: same-domain cross-thread calls,
  cross-domain call gates, stale gate generation rejection, budget accounting,
  synchronous return continuation handling, asynchronous completion delivery,
  handoff cancellation ownership, reentrant-depth limits, and denied capability
  passing.
- Write randomized tests for invalid FDs, bad paths, page faults, and killed
  blocked threads.
- Run the same binaries against emulator and RTL simulation.

RTL simulation milestones:

1. Fetch/decode/ALU/load/store from DDR model.
2. multi-context scheduler with `CLONE`, `YIELD`, `AWAIT`, and `EXIT`.
3. FDR table plus UART `PULL`/`PUSH`.
4. SD-backed simple filesystem with `OPEN_AT`, `PULL`, and `GET_META`.
5. VFS mutations with `NS_CTL` and `SET_META`: mkdir, link, unlink, rename,
   chmod, chown.
6. `MMAP`, page faults, and file-backed pages.
7. Hardware Heap Engine: `ALLOC`, `ALLOC_EX`, `FREE`, invalid free detection,
   fork COW, exec teardown, and cross-thread frees.
8. `CLONE`, copy-on-write, and child-exit `AWAIT`.
9. `EXEC` from SD.
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

## 27. Main Architectural Risk

The hard part is not the integer CPU. The hard part is bounding POSIX semantics
so they fit into fixed hardware controllers. LNP64 v1 should deliberately define
an FPGA-native POSIX subset with hard limits. The compiler, libc shim, and
runtime should target that subset rather than assuming every Linux behavior is
replicated.

The core architectural bet is that POSIX operations are represented as
capability-checked hardware commands that park threads and let the scheduler run
other work. That keeps the ISA promise: the file, process, VM, and synchronization
operations are real hardware features, not software calls with different names.
