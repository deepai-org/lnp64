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
- Paravirtual Unix guest support through supervisor domains and hardware
  upcalls, without adding traditional hosted-OS rings or syscall traps.
- Deterministic instruction decode with a fixed binary encoding.

The v1 design is allowed to be slow for complex POSIX operations. For example,
`EXEC` can take thousands or millions of cycles while the SD controller streams
an ELF image. The important requirement is that the operation is performed by
dedicated hardware controllers and the issuing thread is parked while other
threads continue.

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
- Capability File Descriptor Table.
- Silicon VFS Namespace Engine.
- File Operation Engine.
- Directory Operation Engine.
- Process Engine.
- Signal Engine.
- Futex and Atomic Engine.
- Supervisor Domain and Upcall Engine.
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

## 5. Execution Model

The v1 processor contains a small number of identical in-order, multi-context,
barrel-style core tiles. A practical FPGA target is 2 to 4 tiles. Each tile can
execute one selected ready thread per cycle from its local issue lane, subject to
cache and engine availability.

Each hardware thread context contains:

- `pc`: 64-bit virtual instruction address.
- 32 GPRs, 64-bit.
- 32 FPRs, 64-bit IEEE-754 storage.
- 16 vector registers, 128-bit.
- condition flags.
- current PID and TID.
- signal-delivery state.
- blocked/runnable/waiting state.

Each core tile executes one selected ready thread at a time. On each cycle, the
local scheduler front end supplies a runnable TID to fetch/issue. Simple ALU
instructions retire quickly. Complex instructions enqueue work and remove the
TID from the issuing core's active set.

This is not microcode: `OPEN_FD`, `FORK`, `EXEC`, `MMAP`, and similar operations
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

Used by integer ALU, FPU, vector operations, `LOCK.CMPXCHG` variants that name
register operands through extension fields.

`F2`: register-register.

```text
a=dst/src0, b=src
```

Used by `MOV`, `NOT`, `CMP`, `FREE`, `ERRNO_SET`, `SIGMASK_SET`, `SLEEP`,
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
a=condition or link mode, imm40=signed byte offset
```

Used by `JMP`, `BEQ`, `BNE`, `BLT`, `BGT`, `BLE`, `BGE`, `CALL`.

`F5`: memory.

```text
a=gpr value, b=base gpr, width=flags[1:0], imm24=signed byte offset
```

Used by `LD` and `ST`. For `ST`, `a` is source. For `LD`, `a` is destination.
The assembler exposes width suffixes `LD.B`, `LD.H`, `LD.W`, `LD.D`, `ST.B`,
`ST.H`, `ST.W`, and `ST.D`.

`F6`: static FDR operation.

```text
a=fd0, b=gpr0, c=gpr1, d=gpr2, flags=subop/width
```

Used by static file descriptor instructions such as `READ_FD fdN, rBuf, rLen`.

`F7`: dynamic FDR operation.

```text
a=gpr fd/index/dest, b=gpr0, c=gpr1, d=gpr2
```

Used by runtime integer fd forms such as `READ_FD_DYN`.

`F8`: process, VM, and signal operation.

```text
a=gpr0, b=gpr1, c=gpr2, d=gpr3, imm16=subfunction
```

Used by `FORK`, `EXEC`, `SPAWN`, `WAIT_PID`, `MMAP`, `MUNMAP`, `SIGACTION`,
`KILL`, and message operations.

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

20 OPEN_FD
21 OPEN_FD_DYN
22 OPEN_DIR
23 OPEN_DIR_DYN
24 READ_FD
25 READ_FD_DYN
26 READDIR_FD
27 READDIR_FD_DYN
28 REWINDDIR_FD
29 REWINDDIR_FD_DYN
2A WRITE_FD
2B WRITE_FD_DYN
2C FD_CLOSE
2D FD_CLOSE_DYN
2E FD_SEEK
2F FD_SEEK_DYN

30 MKDIR_PATH
31 UNLINK_PATH
32 RENAME_PATH
33 LINK_PATH
34 SYMLINK_PATH
35 READLINK_PATH
36 CHDIR_PATH
37 GETCWD_PATH
38 CHMOD_PATH
39 CHOWN_PATH
3A STAT_PATH
3B STAT_FD
3C STAT_FD_DYN

40 WAIT_ON_FD
41 FD_DUP
42 FD_DUP2
43 PIPE
44 ERRNO_GET
45 ERRNO_SET
46 PREAD_FD
47 PREAD_FD_DYN
48 PWRITE_FD
49 PWRITE_FD_DYN

50 WAIT_PID
51 GET_PCR
52 SET_PCR
53 FORK
54 EXEC
55 SPAWN
56 YIELD
57 SLEEP
58 EXIT

60 MMAP
61 MUNMAP
62 SIGACTION
63 SIGMASK_SET
64 KILL
65 SIGRET
66 MPROTECT

70 LOCK_CMPXCHG
71 FUTEX_WAIT
72 FUTEX_WAKE

80 INB
81 OUTB
82 LOAD_UCODE
83 MSG_SEND
84 MSG_RECV

90 FADD
91 FSUB
92 FMUL
93 FDIV
A0 VADD32
```

Illegal or unimplemented opcodes deliver a hardware `SIGILL`.

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

Branches update only the issuing thread's PC. No global pipeline flush is needed
if fetch is tagged by TID and valid bits.

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

`FUTEX_WAIT` and `FUTEX_WAKE` use physical addresses after translation. This is
important: two processes mapping the same shared page must wait on the same
futex key even if their virtual addresses differ.

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

This rule is mandatory for `READ_FD`, `WRITE_FD`, file-backed page faults,
Ethernet RX/TX, and SD/SPI transfers.

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
- `SLEEP rTicks`: inserts current TID into timer wheel.
- `WAIT_ON_FD`: attaches current TID to a file object's event mask.
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
- `WAIT_ON_FD` can block on an `event_queue` FDR and wake when any member source
  becomes ready.
- Event queue records are fixed-width, versioned, and carry source fd/object id,
  event mask, result code, and optional operation id.
- This is the hardware substrate for `poll`, `select`, `epoll`, `kqueue`,
  timeout waits, and supervisor-domain event dispatch.

## 12. Capability File Descriptor Registers

FDRs are not integer registers. Each process owns a DDR-backed hardware FDR
capability table. The default architectural table has 4096 descriptor entries
per process and can be expanded by implementation.

Each FDR entry contains:

- valid bit.
- object class: `closed`, `regular_file`, `directory`, `char_stream`,
  `block_device`, `pipe_read`, `pipe_write`, `socket`, `listener`,
  `event_queue`, `timer`, `control`, `pci_function`, `pcie_bar`, `dma_buffer`,
  `irq_event`, `gpu_device`, `accelerator`.
- backend id: `none`, `uart0`, `sd0`, `spi_flash0`, `eth0`, `ramfs`,
  `pipe_engine`, `socket_engine`, `vfs_engine`, `supervisor_engine`,
  `pcie_root`, `pcie_iommu`, `pcie_msi`, `nvme_driver`, `ethernet_driver`,
  `gpu_driver`.
- protocol or subtype: `raw_frame`, `udp_datagram`, `stream`, `block_extent`,
  `block_image`, `tty`, `control`, `pci_config`, `bar_mmio`,
  `timer_oneshot`, `timer_periodic`, `msi_vector`, `msix_vector`,
  `pinned_dma`, `framebuffer`, or
  backend-defined.
- rights: read, write, seek, stat, directory, execute, poll.
- object id.
- current offset.
- flags.
- reference count pointer.
- event mask.
- metadata cache pointer.
- backend-private pointer.

Static FDR instructions address only the low 256 descriptors with the 8-bit FDR
field. They are a compact fast form for stdin/stdout/stderr, common runtime
handles, and compiler-selected hot descriptors.

The architectural FDR table is larger than the static encoding range. Dynamic
FDR instructions use a GPR containing the runtime descriptor index and can
address the full DDR-backed descriptor table. Any descriptor index above 255
must use a `*_DYN` instruction form.

The hardware validates range, valid bit, and rights before issuing the
operation.

Invalid descriptors return `-1` in `r1` where applicable and set `ERRNO=EBADF`.

## 13. Silicon VFS Namespace Engine

The VFS engine resolves paths and manages namespace metadata in hardware.

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

The VFS namespace is stored in DDR as a compact tree of inode-like objects and
directory-entry arrays. Frequently used root, cwd, and open object metadata are
cached in FPGA RAM.

The path resolver is a hardwired FSM. It walks each component, performs directory
lookup, checks permissions, follows symlinks when permitted, and emits either an
object id or an errno.

## 14. Device Backends

### 14.1 UART

UART exposes character stream objects:

- `fd0`: stdin receive FIFO.
- `fd1`: stdout transmit FIFO.
- `fd2`: stderr transmit FIFO.

`READ_FD` from UART blocks if no data is available unless nonblocking flags are
set. `WRITE_FD` pushes bytes into the transmit FIFO and parks the thread if the
FIFO is full.

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
- `READ_FD` receives frames/datagrams into user buffers.
- `WRITE_FD` transmits frames/datagrams from user buffers.

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
- support `WAIT_ON_FD` for interrupt-driven drivers.
- may be delegated per-vector so a driver receives only interrupts for its
  assigned device/function.

This model deliberately avoids a large hardware PCIe enumerator or BAR command
parser. PCIe complexity and quirks live in one isolated Bus Master process, but
the rest of the system still sees devices as POSIX-native capabilities.

## 15. File and Directory Instructions

All file instructions are decoded into File Operation Engine commands.

`OPEN_FD` and `OPEN_FD_DYN`:

- read path string through MMU.
- resolve path through VFS engine.
- check rights and flags.
- allocate or overwrite FDR capability entry.
- return descriptor index for dynamic form.

`READ_FD`:

- validate capability and read rights.
- translate object offset and length into backend requests.
- issue DMA to user virtual buffer through MMU.
- update file offset.
- write byte count to `r1`.

`WRITE_FD`:

- validate capability and write rights.
- DMA from user buffer to backend.
- update offset or append metadata.
- write byte count to `r1`.

`PREAD_FD`, `PREAD_FD_DYN`, `PWRITE_FD`, and `PWRITE_FD_DYN`:

- validate capability and read/write rights.
- use an explicit byte offset register instead of the descriptor's current
  offset.
- do not mutate the descriptor offset.
- are the preferred interface for paravirtual block devices, concurrent file
  servers, and Linux/NetBSD guest block layers.

`READDIR_FD`:

- validate directory object.
- fetch next directory entry metadata.
- write stable LNP64 dirent layout into user buffer.
- return positive for entry, zero for end, `-1` on error.

`STAT_PATH`, `STAT_FD`, `CHMOD_PATH`, `CHOWN_PATH`, `LINK_PATH`,
`SYMLINK_PATH`, `READLINK_PATH`, `RENAME_PATH`, `UNLINK_PATH`, `MKDIR_PATH`,
`CHDIR_PATH`, and `GETCWD_PATH` are VFS engine operations with fixed state
machines for metadata mutation and buffer DMA.

## 16. Process Engine

The Process Engine owns PID allocation, process table entries, parent-child
relationships, and process-wide resources.

Each process entry contains:

- PID.
- parent PID.
- address-space root pointer.
- VMA tree root pointer.
- FDR table pointer.
- cwd object id.
- uid/gid.
- signal table pointer.
- child state queue.
- process state.

`SPAWN`:

- allocates a TID in the same process.
- creates a new thread context.
- sets the new PC to `r_entry_ptr`.
- allocates a stack VMA or stack subrange.
- returns TID to destination register.
- pushes TID to ready queue.

`FORK`:

- allocates a new process entry and PID.
- duplicates the parent's FDR table by incrementing object refcounts.
- duplicates the VMA tree using copy-on-write page table entries.
- copies the current thread context.
- writes child PID to the parent destination register.
- writes zero to the child destination register.
- enqueues the child thread.

`EXEC`:

- resolves executable path through VFS.
- validates execute permission.
- streams ELF headers through File Operation Engine.
- tears down old VMAs except preserved process resources.
- builds new text, data, heap, stack, and arg VMAs.
- DMA-loads program segments from storage into DDR.
- resets PC, SP, registers, signal state as specified by LNP64 ABI.
- preserves PID, parent, cwd, selected FDRs, uid/gid.

`WAIT_PID`:

- checks child state table.
- if child already exited, writes status immediately.
- otherwise parks current TID on child-exit wait queue.

`EXIT`:

- marks current TID dead.
- if last thread in process, closes process resources, marks process zombie,
  stores exit status, and signals parent with `SIGCHLD`.

## 17. MMAP and MUNMAP

`MMAP` is a real hardware VMA operation.

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
- requested memory type: `normal_cached`, `uncached`, `device_ordered`, or
  `write_combining`.

For normal files and anonymous memory, unsupported memory type requests fail
with `EINVAL`. For `pcie_bar` FDRs, the requested memory type must be permitted
by the FDR and the mapping must be page-aligned. No sub-page BAR capability is
architectural in v1.

`MUNMAP`:

- finds intersecting VMAs.
- splits or removes VMA descriptors.
- decrements page refcounts.
- invalidates matching TLB entries for that process.
- writes success or errno.

`MPROTECT`:

- finds existing VMAs covering the requested range.
- updates read/write/execute and sharing permission bits.
- invalidates matching TLB and instruction-cache entries where permissions
  require it.
- is required for ELF loaders, language runtimes, JIT policy, guard pages, and
  paravirtual Unix guests mapping their own process abstractions onto LNP64 VMAs.

The VMA tree can be a hardware-walked B-tree or interval tree in DDR. For FPGA
v1, a sorted VMA array per process is acceptable if bounded and checked in
hardware.

## 18. Signals

The Signal Engine handles asynchronous delivery.

Per process/thread state:

- signal mask.
- pending signal bitmap and queue.
- handler table.
- saved signal context stack.

`SIGACTION` writes the handler table.

`SIGMASK_SET` updates the mask and may trigger immediate delivery of newly
unmasked pending signals.

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

## 19. Futex and Atomic Engine

`LOCK_CMPXCHG` is implemented in the LSU/DDR atomic path:

- translate virtual address.
- lock the cache line or atomic DDR transaction slot.
- compare current value.
- conditionally write new value.
- return old value or success code in destination register.

`FUTEX_WAIT`:

- translates address.
- atomically reads value.
- if value equals expected, parks TID on a hash bucket keyed by physical address.
- if not equal, returns immediately with `ERRNO=EAGAIN`.

`FUTEX_WAKE`:

- translates address.
- finds matching wait bucket.
- moves up to requested count of TIDs to ready queue.
- returns wake count.

## 20. PCRs and Credentials

PCRs are stored in process context:

- PID: read-only.
- TID: read-only.
- UID.
- GID.
- SIGMASK.

`GET_PCR` reads from context into a GPR. `SET_PCR` is permission checked in
hardware. UID/GID changes require the current effective UID to be zero unless
the operation is a permitted drop in privilege.

All VFS permission checks consume a snapshot of UID/GID from PCR state at command
issue time.

### 20.1 Privilege and Security Model Options

The privilege model is not frozen yet. Because there is no kernel supervising
these operations, the hardware needs a crisp authority model before RTL.

Option A: Unix-like UID/GID plus capability bits.

- Familiar model for file permissions, signals, ownership, and setuid-like
  transitions.
- Root-equivalent UID 0 can mount devices, bind privileged endpoints, change
  ownership, and configure global hardware tables.
- Add per-process capability bits for narrower authority such as network
  binding, adapter configuration, raw device access, and process inspection.
- Good default if LNP64 wants to run conventional cloud software with minimal
  runtime changes.

Option B: pure object capabilities.

- FDRs and process handles carry all authority.
- No global root user; authority is delegated by passing capabilities.
- Strong fit for hardware FDRs and least-privilege services.
- Weakness: conventional POSIX software expects UID/GID checks and ambient
  process authority.

Option C: hybrid cloud profile.

- Keep UID/GID and POSIX permission checks for compatibility.
- Represent privileged powers as hardware capability bits attached to process
  context.
- Require both UID/GID permission and specific capability bits for dangerous
  operations such as raw network access, mounting, adapter table loading,
  cross-user `KILL`, and process memory inspection.
- Best candidate for v1 because it preserves POSIX shape while avoiding a single
  all-powerful root path in hardware.

Recommended direction before freeze: use the hybrid cloud profile. Define a
small capability bitmap in process context and require it for:

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

### 20.2 Paravirtual Unix Guest Profile

LNP64 does not add a conventional hosted-OS profile with kernel rings, software
page tables, mandatory syscall traps, or an OS-owned scheduler. A future
Linux/NetBSD port is made plausible by treating the kernel as a paravirtual Unix
personality process running on top of native LNP64 POSIX hardware.

The silicon remains authoritative for:

- hardware process and thread creation.
- runqueue scheduling and context storage.
- VMA creation, teardown, page faults, and copy-on-write.
- FDR capabilities and hardware VFS object references.
- signals, futex queues, fd readiness, and DMA completion.

The guest kernel/personality owns:

- Linux/BSD-specific process metadata.
- namespaces, cgroups, jails, credentials, and policy state.
- emulation of APIs not directly represented by LNP64 opcodes.
- Linux syscall-number compatibility where a syscall-compatible runtime is used.
- filesystem images mounted inside hardware VFS files.
- network stack policy above raw frame or datagram hardware objects.
- compatibility ABIs and userland conventions.

Targeted compatibility approaches:

- Linux as a paravirtual personality: a Linux kernel port runs as a supervisor
  process over a delegated LNP64 process subtree. It maps Linux tasks, files,
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

- A process with the `supervisor_domain` capability may create a delegated
  process subtree.
- Native processes inside that subtree are bound to the supervisor's policy
  domain.
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
records through `READ_FD` and accepts policy commands through `WRITE_FD`. This
keeps the mechanism inside the FDR model instead of introducing a traditional
syscall path.

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
- The guest block layer uses `PREAD_FD` and `PWRITE_FD` against explicit byte
  offsets rather than descriptor seek state.
- Linux ext4, NetBSD FFS, or other guest filesystems can live inside one or more
  large hardware VFS files.
- Hardware does not need to understand those guest filesystem formats.

Task mapping:

- Linux/BSD threads map one-to-one to LNP64 hardware threads where practical.
- The guest scheduler becomes a policy layer that creates, parks, wakes, and
  accounts for native hardware threads.
- The hardware scheduler still performs actual dispatch and context switching.
- Guest preemption can be modeled through cooperative yield points, timer
  upcalls to the supervisor domain, and hardware thread park/wake commands.

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
  upcalls. `WAIT_ON_FD` is the primitive, but runtimes need a way to construct
  event-queue FDRs that represent sets of wait sources.

This profile preserves the LNP64 thesis: Linux/NetBSD can become personalities
that project their semantics onto native POSIX hardware, rather than forcing the
chip to become a conventional trap-and-kernel machine.

## 21. DMA Fabric

The DMA Fabric moves bytes between:

- DDR user buffers.
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
- fault policy.
- completion target TID or engine.
- optional PCIe requester id and IOMMU context.

The DMA fabric uses the MMU for user virtual addresses. If translation faults,
the fault is routed back to the VMA Engine. The original operation remains
blocked until the page fault resolves or fails.

For PCIe, the DMA Fabric and IOMMU jointly enforce that a device can access only
pages exported through a valid `dma_buffer` FDR. Revocation requires the Bus
Master or driver to quiesce the device, tear down IOMMU entries, and wait for
in-flight DMA completion before the VMA Engine releases the backing pages.

## 22. Boot Flow

There is no boot CPU.

Reset sequence:

1. Hardware reset controller initializes FPGA-local RAM structures.
2. DDR controller calibration completes.
3. Page allocator marks DDR regions free or reserved.
4. VFS engine mounts boot backend from SD or SPI flash.
5. Process Engine creates PID 1, TID 1, UID 0.
6. FDR table binds `fd0`, `fd1`, `fd2` to UART.
7. If PCIe is present, Root Complex link training completes and a privileged
   PCIe Bus Master process is created or scheduled from the boot image.
8. VFS resolves `/sbin/init`.
9. `EXEC` engine loads `/sbin/init` into PID 1.
10. Scheduler marks TID 1 ready.
11. Fetch begins at PID 1 entry point.

If no boot image is found, the reset controller enters a hardware panic state
that emits a UART diagnostic and blinks a board LED pattern.

## 23. Error Reporting

Fallible POSIX-like instructions follow the emulator convention:

- success returns zero or a nonnegative byte/count/value.
- failure returns all-ones `-1` where applicable.
- process-local `ERRNO` is updated.

Hardware engines write result registers only at command completion. If a thread
is killed while an engine command is in flight, the Event Router cancels or
detaches the command according to object type.

### 23.1 Failure and Cancellation Semantics

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
- `FORK` does not clone in-flight operation ownership into the child.
- `EXEC` cancels or waits for all operations tied to the old address space
  before replacing mappings.

Operation classes:

- `READ_FD`, `WRITE_FD`, Ethernet receive/transmit, UART waits, and file-backed
  page reads are interruptible before DMA commit and return `EINTR` if canceled.
- `RENAME_PATH`, `LINK_PATH`, `UNLINK_PATH`, `CHMOD_PATH`, `CHOWN_PATH`, and
  directory entry mutations become non-interruptible after the VFS engine reaches
  its serialized metadata commit point.
- `MMAP` and `MUNMAP` are cancelable before page table publication; after
  publication they complete and then report success or fault.
- `EXEC` is cancelable before the new image commit point; after commit, the old
  image no longer resumes.
- `FORK` is cancelable before PID publication; after PID publication the child
  must either become runnable or be reaped as a failed child with status.
- `FUTEX_WAIT` is interruptible and returns `EINTR`; `FUTEX_WAKE` is
  nonblocking and noncancelable once issued.

Hardware engines must never deliver partial architectural writes to user memory
unless the instruction's POSIX result reports the number of bytes actually
transferred. Metadata operations are atomic at their documented commit point.

## 24. FPGA Resource Strategy

Likely expensive blocks:

- VFS path resolver.
- VMA and page table walker.
- DDR-backed FDR table cache and descriptor walkers.
- multi-context register storage.
- DMA buffers.
- SD and Ethernet adapters.

To keep v1 feasible:

- Use one shared POSIX engine pipeline rather than duplicating per thread.
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
fdrs/process:                    DDR-backed, default 4096, expandable higher
pending events/process:          DDR-backed event queues, at least 4096
futex buckets:                   4096+ global hash buckets, DDR-backed waiters
vmas/process:                    DDR-backed, at least 4096
path bytes:                      4096
path components:                 256
open objects:                    DDR-backed, at least 262144 system-wide
pipe buffers:                    DDR-backed, 64 KiB default, resizable
```

## 25. Verification Plan

Verification should start at the architectural level before RTL:

- Build an instruction encoding/decoding golden model.
- Extend the current Rust emulator to consume encoded 64-bit instructions.
- Add traces for thread scheduling, FDR table transitions, VMA changes, and
  signal delivery.
- Write directed tests for every POSIX instruction.
- Write randomized tests for invalid FDs, bad paths, page faults, and killed
  blocked threads.
- Run the same binaries against emulator and RTL simulation.

RTL simulation milestones:

1. Fetch/decode/ALU/load/store from DDR model.
2. multi-context scheduler with `SPAWN`, `YIELD`, and `EXIT`.
3. FDR table plus UART `READ_FD`/`WRITE_FD`.
4. SD-backed simple filesystem with `OPEN_FD`, `READ_FD`, `STAT`.
5. VFS mutations: mkdir, link, unlink, rename, chmod, chown.
6. `MMAP`, page faults, and file-backed pages.
7. `FORK`, copy-on-write, `WAIT_PID`.
8. `EXEC` from SD.
9. signals and futexes.
10. Ethernet packet objects.
11. `PREAD_FD`/`PWRITE_FD` block-image object.
12. supervisor-domain control FDR and upcall delivery.
13. minimal paravirtual Unix personality that boots a guest init process over
    native LNP64 tasks and a block-image filesystem.
14. Linux syscall compatibility runtime for a static userland smoke test:
    open/read/write/mmap/futex/clock/wait/exec paths mapped to native opcodes.
15. NetBSD rump-kernel style filesystem service over a block-image FDR, exposed
    back through delegated native FDRs.
16. PCIe Root Complex smoke test with Bus Master config-space enumeration.
17. page-granular `pcie_bar` FDR minting and `MMAP` to device-ordered and
    write-combining VMAs.
18. `dma_buffer` FDR export through IOMMU and revocation after DMA quiesce.
19. MSI/MSI-X delivery through `irq_event` FDRs and `WAIT_ON_FD`.
20. simple NVMe or NIC driver domain using BAR `LD`/`ST`, DMA buffers, and IRQ
    events to publish high-level block or network FDRs.

## 26. Main Architectural Risk

The hard part is not the integer CPU. The hard part is bounding POSIX semantics
so they fit into fixed hardware controllers. LNP64 v1 should deliberately define
an FPGA-native POSIX subset with hard limits. The compiler, libc shim, and
runtime should target that subset rather than assuming every Linux behavior is
replicated.

The core architectural bet is that POSIX operations are represented as
capability-checked hardware commands that park threads and let the scheduler run
other work. That keeps the ISA promise: the file, process, VM, and synchronization
operations are real hardware features, not software calls with different names.
