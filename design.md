Here is the draft Instruction Set Architecture (ISA) for the **LNP64 (Linux-Native Processor 64-bit)**. The design explores putting POSIX-like resource, process, memory, and event primitives directly into fixed hardware.

---

# LNP64 Instruction Set Architecture (Draft v1.0)

## 1. Register Architecture
To support hardware-native OS primitives, the standard register file is expanded beyond General Purpose Registers (GPRs) to include File Descriptor Registers (FDRs) and Process Control Registers (PCRs).

*   **GPRs (General Purpose):** `r0` - `r31` (64-bit, standard ALU operations).
*   **LR (Link Register):** Thread-local 64-bit return-address register. `CALL` / `CALL_REG` write `LR = PC + 8`; `RET` jumps to `LR`.
*   **FDRs (File Descriptor Registers):** `fd0` - `fd255` are the static low-descriptor fast bank. Full process FDR tables are DDR-backed and addressed by dynamic FDR instructions. FDRs do not hold integers; they hold hardware capability references to Silicon VFS objects, device objects, event queues, timers, generic counters, generic queues, memory objects, PCIe BARs, DMA buffers, or supervisor controls. `fd0`, `fd1`, and `fd2` conventionally bind to STDIN, STDOUT, and STDERR streams of the controlling TTY.
*   **PCRs (Process Control Registers):**
    *   `PID`: Current Process ID, from process context.
    *   `PPID`: Parent Process ID, from process context, or `0` for root.
    *   `TID`: Current Thread ID, from thread context.
    *   `UID` / `GID`: User/Group ID from process credential context.
    *   `CAPMASK`: Process credential capability bitmap.
    *   `SIGMASK`: Thread-local 64-bit bitmask of currently blocked signals.
    *   `REALTIME_SEC` / `REALTIME_NSEC`: Read-only realtime clock snapshot
        fields used by libc/runtime clock surfaces. Timer waitability remains
        FDR-backed through timer profiles.
*   **ERRNO:** Thread-local POSIX error register. Fallible instructions write their result to the encoded destination register and update thread-local `ERRNO` on failure.

## 2. Process & Scheduling Instructions
The CPU features a hardware-managed runqueue. There is no mandatory OS scheduler tick; hardware scheduler and context-store blocks dispatch ready threads directly.

*   **`CLONE r_dest, r_flags_or_argblock`**
    *   *Action:* Creates a new process or thread according to share flags. Fork-like source forms lower to `CLONE` with a new PID and copy-on-write VMAs; thread-like source forms lower to `CLONE` with shared process resources and a new entry PC/stack.
*   **`EXEC r_path_ptr, r_argv_ptr`**
    *   *Action:* Enters a process-wide exec barrier, stops sibling threads, cancels/detaches in-flight operations, invalidates old thread contexts, loads the new image, and resumes with exactly one surviving thread.
*   **`YIELD`**
    *   *Action:* Suspends the current thread, saves state to the hardware thread context, and selects another ready TID from the hardware runqueue at a bounded scheduling point.
*   **`EXIT r_exit_code`**
    *   *Action:* Destroys the current hardware thread context. If it's the last thread in the PID group, triggers hardware VMA teardown and signals the parent PID with `SIGCHLD`.

## 3. I/O and File Operations
System calls are replaced by direct hardware VFS/File Engine commands. The binary ISA uses a compact stream/resource model; POSIX-shaped source names are assembler or libc lowering aliases.

*   **`OPEN_AT r_dest, r_dirfd, r_path_ptr, r_flags`**
    *   *Action:* Resolves a path/name relative to a directory/root FDR or cwd/root and returns a resource FDR. Source-level `open`, `openat`, and `opendir` lower to this instruction.
*   **`PULL r_result, r_fd, r_buf_ptr, r_len_or_argblock`**
    *   *Action:* Pulls records from a stream object into memory. Files produce bytes, directories produce dirent records, sockets produce packets/messages, event queues produce event records, and block-image FDRs may use explicit-offset argument blocks.
*   **`PUSH r_result, r_fd, r_buf_ptr, r_len_or_argblock`**
    *   *Action:* Pushes records from memory to a stream object. Files consume bytes, sockets consume packets/messages, control FDRs consume commands, and block-image FDRs may use explicit-offset argument blocks.
*   **`SEEK r_result, r_fd, r_offset_or_cookie, r_whence`**
    *   *Action:* Repositions a seekable stream. Directory rewind is `SEEK(fd, 0, SET)`, and directory cookies use the same instruction.
*   **`AWAIT r_result, r_waitable, r_mask_or_argblock`**
    *   *Action:* Parks the current thread until a waitable object changes state. FDs, event queues, timers, child exit, futex predicates, PCIe IRQ events, message channels, and supervisor upcalls all lower to `AWAIT`.
*   **`CLOSE r_result, r_fd`**
    *   *Action:* Releases an FDR capability reference.
*   **`GET_META r_result, r_fd, r_meta_ptr, r_flags`** / **`SET_META r_result, r_fd, r_meta_ptr, r_flags`**
    *   *Action:* Reads or mutates metadata on an opened object: stat, chmod, chown, utime, fd flags, rights, and backend-specific metadata.
*   **`NS_CTL r_result, r_argblock`**
    *   *Action:* Performs namespace mutations relative to directory FDRs: mkdirat, unlinkat, renameat, linkat, symlinkat, readlinkat, chdir, and delegated namespace operations.
*   **`DUP`**
    *   *Action:* Duplicates or moves FDR capabilities. Exact destinations and narrowing flags are encoded in the instruction or argument block. Source-level `pipe()` lowers to `OBJECT_CTL create queue(profile=pipe)` plus narrowed read/write endpoint capabilities.
*   **`CAP_DUP`, `CAP_SEND`, `CAP_RECV`, `CAP_REVOKE`**
    *   *Action:* Architectural FDR capability management. Capabilities can be duplicated with narrowed rights, sealed against further delegation, passed out-of-band over permitted pipe/socket/control FDRs, received into a target FDR table, or revoked along a revocable lineage. This is how the Bus Master delegates PCIe BARs, drivers receive DMA buffers and IRQ events, and supervisor domains pass authority without ambient privilege.
*   **`EVENT_CTL` / `TIMER_CTL` / `SUPERVISOR_CTL`**
    *   *Action:* `EVENT_CTL` and `TIMER_CTL` are source-level/profile aliases over `OBJECT_CTL` for event-queue and timer profiles. `SUPERVISOR_CTL` is a source-level/profile alias over `DOMAIN_CTL` for delegated supervisor domains.
*   **`OBJECT_CTL r_result, r_argblock`**
    *   *Action:* Creates, configures, queries, resets, or destroys the three generic hardware-owned object primitives: `counter`, `queue`, and `memory_object`. Semaphores, completions, event counters, channels, task queues, shared arenas, and DMA completions are runtime profiles over these primitives, not separate hardware modules.
*   **`DMA_CTL r_result, r_argblock`**
    *   *Action:* Submits bulk memory/object operations to the DMA Fabric: large copy, fill/zero, scatter/gather copy, and optional checksum/hash profiles. Small operations may complete synchronously; long operations can complete through an `event_queue` FDR or a `counter` completion profile. DMA always runs through VMA permissions, capability checks, IOMMU/device scope, and Resource Domain accounting.
*   **`DOMAIN_CTL r_result, r_argblock`**
    *   *Action:* Creates, configures, queries, freezes, resumes, or destroys nested Resource Domains. Virtual machines, containers, cgroups, jails, sandboxes, and supervisor domains are profiles of the same domain primitive.
*   **`CALL_CAP r_result, r_call_gate_fd, r_arg0, r_arg1`** / **`RET_CAP r_result, r_value0, r_value1`**
    *   *Action:* Performs a fast call and return through a callable FDR capability. Call gates may target another thread, service queue, driver service, supervisor service, runtime actor, or Resource Domain entry point. Hot calls use bounded register arguments and pre-provisioned target state; cold domain/container/VM creation remains a `DOMAIN_CTL` operation. Call gates support synchronous, asynchronous, and handoff profiles.
*   **`ERRNO_GET r_dest`** / **`ERRNO_SET r_src`**
    *   *Action:* Reads or writes the thread-local POSIX error register. Fallible VFS instructions write success or `-1` to their encoded result register and set thread-local `ERRNO` on failure.
*   **Child Waits**
    *   *Action:* Child completion is a waitable event. Source-level `waitpid` lowers to `AWAIT` on a child/process waitable and then `GET_META` for status where needed.

## 4. Memory Management (Silicon VMAs)
Page tables and VMAs are managed by fixed hardware MMU/VMA engines using bounded hardware-walked metadata structures.

*   **`MMAP r_dest, r_hint_addr, r_len, r_prot, fd_src, r_offset`**
    *   *Action:* Hardware allocates physical pages and inserts a new VMA node into the current PID's silicon VMA tree. If `fd_src` is valid, configures hardware page-fault handlers to fetch from the storage controller. Returns the mapped virtual address in `r_dest`.
    *   *Protection Flags:* `r_prot` includes read/write/execute, shared/private, guard-page, and memory type: `normal_cached`, `uncached`, `device_ordered`, or `write_combining`. Writable-plus-executable mappings are rejected unless the current Resource Domain has an explicit JIT/loader policy bit.
*   **`MUNMAP r_addr, r_len`**
    *   *Action:* Invalidates the VMA range, flushes the relevant TLB entries, and releases affected physical pages through the hardware page allocator when no longer referenced.
*   **`MPROTECT r_addr, r_len, r_prot`**
    *   *Action:* Updates the protection bits for an existing VMA range and invalidates affected translations. This supports ELF loaders, guard pages, W^X policy, and paravirtual Unix guests that map their process abstractions onto LNP64 VMAs.
*   **`ALLOC r_dest, r_size`** / **`ALLOC_EX r_dest, r_request_block`** / **`ALLOC_SIZE r_dest, r_ptr`** / **`FREE r_result, r_ptr`**
    *   *Action:* Allocates, queries, and frees byte-granular heap memory through the Hardware Heap Engine. The heap is process-local, backed by anonymous NX VMAs, thread-safe in hardware, and integrated with `CLONE` copy-on-write and `EXEC` teardown. `ALLOC_EX` supports alignment, zeroing, guard, debug, locality, allocation-class, and optional memory-tag/debug-hardening flags. `ALLOC_SIZE` exposes allocation metadata to libc/runtime code so `realloc` can copy only the valid old allocation extent.

## 5. Signal Handling
Signals are no longer software constructs; they are asynchronous hardware events delivered directly to the thread. Hardware execution faults use the same delivery path as POSIX signals.

*   **`SIGACTION r_signum, r_handler_ptr`**
    *   *Action:* Registers a hardware trampoline address for a specific POSIX signal.
*   **`SIGMASK_SET r_mask`**
    *   *Action:* Updates the `SIGMASK` PCR.
*   **`ALARM r_dest, r_seconds`**
    *   *Action:* Resets the process's POSIX alarm timer, returns the previous
        remaining whole seconds in `r_dest`, and enqueues `SIGALRM` when the
        timer expires. General multi-source timers remain FDR-backed timer
        profiles.
*   **`KILL r_pid, r_signum`**
    *   *Action:* Routes a signal request through the Signal Engine to the target PID/TID, waking the target if it is in an interruptible wait.
*   **`SIGRET`**
    *   *Action:* Issued at the end of a signal handler. Pops the hardware-saved pre-interrupt register state off the thread's stack and resumes normal execution.
*   **Fault Delivery**
    *   *Action:* Divide-by-zero and arithmetic traps raise `SIGFPE`; illegal or disabled opcodes raise `SIGILL` unless routed to a supervisor upcall; invalid or protected memory accesses raise `SIGSEGV`; alignment and unmappable physical/device accesses raise `SIGBUS`; breakpoints raise `SIGTRAP`. The signal frame records faulting PC, signal code, bad address where applicable, and the trapped opcode where useful.

---
To make the **LNP64** a fully functional processor, the POSIX-like hardware instructions must coexist with a conventional general-purpose compute architecture. Since VFS, capability, VMA, event, and runqueue logic consume meaningful FPGA resources, the general compute side should remain a lean in-order RISC architecture.

Here is how the general-purpose compute integrates with the Linux-native silicon.

---

### 6. Memory Access (Load/Store Architecture)
The LNP64 is a strict Load/Store architecture. ALUs only operate on registers. Because the CPU manages VMAs and page faults natively, a `LOAD` that faults can park the issuing thread while the VMA/File/DMA engines resolve a resident, file-backed, COW, or failed mapping case. No conventional kernel trap is required for native LNP64 faults.

*   **`LD r_dest, [r_base, r_offset]`**
    *   *Action:* Loads a 64-bit word from the virtual address `r_base + r_offset` into `r_dest`.
*   **`LD.B`, `LD.H`, `LD.W`, `LD.D`**
    *   *Action:* Byte (8-bit), Half-word (16-bit), Word (32-bit), and Double-word (64-bit) load variants.
*   **`ST [r_base, r_offset], r_src`**
    *   *Action:* Stores the contents of `r_src` into memory. Hardware automatically updates the "Dirty" bit in the silicon page table.
*   **`ST.B`, `ST.H`, `ST.W`, `ST.D`**
    *   *Action:* Byte, half-word, word, and double-word store variants. Half-word access is included so PCIe BAR mappings can use native 16-bit register accesses when required.
*   **`FENCE`**
    *   *Action:* Memory barrier. Ensures all previous memory operations and hardware DMA transfers from stream/device operations are globally visible before proceeding.
*   **`ISYNC r_addr, r_len`**
    *   *Action:* Invalidates instruction-cache state for an executable range or mapped object. This is required for JITs and code patching and uses the same hardware invalidation fabric as `EXEC` and `MPROTECT`.

### 7. Arithmetic and Logic Unit (ALU)
Standard 64-bit integer operations. Because threads are managed in hardware, the ALU pipeline reads and writes architectural state through hardware thread contexts.

*   **`ADD r_dest, r_src1, r_src2`** / **`SUB r_dest, r_src1, r_src2`**
    *   *Action:* Standard integer addition/subtraction.
*   **`MUL r_dest, r_src1, r_src2`** / **`DIV r_dest, r_src1, r_src2`**
    *   *Action:* Integer multiplication and hardware division. Division by zero is delivered through the Signal Engine as `SIGFPE`.
*   **`AND`, `OR`, `XOR`, `NOT`**
    *   *Action:* Standard bitwise operations.
*   **`LSL`, `LSR`, `ASR`**
    *   *Action:* Logical Shift Left, Logical Shift Right, Arithmetic Shift Right.

### 8. Control Flow (Branching & Execution)
Since there is no Ring 0 / Ring 3 boundary, native control flow is about
executing user logic and jumping to functions. Compatibility personalities may
receive explicit supervisor upcalls, but native LNP64 POSIX operations are not
implemented as syscall traps.

*   **`JMP r_target`** / **`JMP immediate`**
    *   *Action:* Unconditional jump to a virtual address.
*   **`CALL r_target`**
    *   *Action:* Writes `PC + 8` to the thread-local Link Register (`LR`) and jumps to `r_target`.
*   **`RET`**
    *   *Action:* Sets `PC = LR`. Software stack frames and spilling the link register are psABI conventions.
*   **`CMP r_src1, r_src2`**
    *   *Action:* Compares two registers and sets the hardware condition flags (Zero, Carry, Negative, Overflow).
*   **`BEQ`, `BNE`, `BLT`, `BGT`**
    *   *Action:* Branch if Equal, Not Equal, Less Than, Greater Than (evaluates condition flags).

### 9. Hybrid OS-Compute Instructions (The "Glue")
Because "Everything is a File" is now a hardware reality, we need instructions to move data between the general compute realm (GPRs) and the OS realm (FDRs and PCRs).

*   **`MOV r_dest, r_src`**
    *   *Action:* Move data between general purpose registers.
*   **`DUP r_result, r_dst_or_flags, r_src`**
    *   *Action:* Duplicates or moves an FDR capability, including `dup`, `dup2`, and narrowed-rights forms where permitted by the source capability.
*   **`GET_PCR r_dest, pcr_name`**
    *   *Action:* Reads a Process Control Register (like `PID`, `UID`, or
        `REALTIME_SEC`) into a general-purpose register for user-space logic.
        (e.g., `GET_PCR r1, PID`).
*   **`SET_PCR pcr_name, r_src`**
    *   *Action:* Writes to a permitted Process Control Register. Credential changes are checked against UID/GID and process capability policy; denied changes fail with a permission error and update thread-local `ERRNO`.
*   **`ENV_GET r_dest, r_key, r_index_or_buf, r_len_or_flags`**
    *   *Action:* Reads read-only process and machine metadata for libc/runtime startup: ISA version, page size, cache-line size, hardware feature bits, architectural limits, `argc`, `argv`/`envp`/`auxv` locations, personality flags, and timebase frequency. This is not a replacement for immediates; constants still use normal instruction encodings or literal loads.
*   **`RANDOM r_dest, r_len_or_flags`**
    *   *Action:* Returns hardware entropy for ASLR, stack canaries, randomized capability ids, allocator hardening, and libc/runtime seeding. Small scalar requests return in `r_dest`; larger requests use a versioned argument-block variant that copies entropy into a caller buffer.

---
**Summary of the Compute Pipeline:**
The ALU and Control Flow instructions avoid privilege-transition overhead for native POSIX-like operations. If an ALU instruction calculates a buffer address and the next instruction is `PUSH`, decode can enqueue a File/DMA Engine command directly rather than entering a software syscall path.
The core ISA also needs synchronization, device-driver boundaries, floating-point/vector compute, and a boot path to be a practical v1 target.

To make the LNP64 bootable and useful, v1 includes **Synchronization, Device Drivers, Floating Point, and Bootstrapping**.

The following sections sketch those remaining pieces of the LNP64 architecture:

### 10. Synchronization (The Silicon Futex)
Because the CPU manages threads in a hardware runqueue, traditional software spinlocks would waste issue slots under contention. Hardware-level concurrency controls let a thread park on a waitable condition and let the scheduler run another ready thread.

*   **`LOCK.CMPXCHG r_dest, [r_addr], r_expected, r_new`**
    *   *Action:* Atomic Compare-and-Swap. The standard building block for mutexes.
*   **`AWAIT futex([r_addr], r_expected_val)`**
    *   *Action:* The hardware equivalent of a futex wait. If the value at `[r_addr]` equals `r_expected_val`, the CPU removes the current thread from the runqueue and parks it in a hardware wait-state attached to that memory address.
*   **`WAKE futex([r_addr], r_num_threads)`**
    *   *Action:* The memory controller checks if any threads are parked on `[r_addr]`. If so, it pushes up to `r_num_threads` back onto the active runqueue.
*   **`THREAD_JOIN r_result, r_tid, r_retval_ptr`**
    *   *Action:* Parks the caller until the target same-process hardware thread exits. On completion, copies the target thread's exit value to `r_retval_ptr` when nonzero and returns `0`; returns a POSIX-style error code for invalid or self-join cases.

### 11. The Device Driver Problem (PCIe Bus Master + Capability Devices)
If the VFS is baked into silicon, how does the CPU know how to talk to a newly released GPU, NVMe drive, or network card? We do **not** hardwire the full PCIe enumeration and quirk universe into the CPU. The hardware provides the safety-critical substrate, and a trusted software **PCIe Bus Master** domain handles the messy device-specific reality.

The v1 hardware includes:

*   PCIe Root Complex link support.
*   IOMMU / DMA remapping.
*   MSI/MSI-X interrupt routing into FDR event objects.
*   Page-table memory types for device mappings: `device_ordered`, `uncached`, and `write_combining`.

The PCIe Bus Master is a privileged process created from the boot image. It alone receives the PCIe Root Complex control capability. It enumerates bus/device/function topology, assigns BARs, handles quirks, configures IOMMU entries, and mints FDR capabilities for driver processes.

Driver processes receive capabilities such as:

*   `pci_function` FDRs for device identity and config ownership.
*   `pcie_bar` FDRs for page-granular BAR windows.
*   `dma_buffer` FDRs for pinned, IOMMU-exported memory.
*   `irq_event` FDRs for MSI/MSI-X vectors.
*   Higher-level `block_device`, `net_device`, `gpu_device`, or `accelerator` FDRs published after a driver binds.

For high-performance MMIO, a driver calls `MMAP` on a `pcie_bar` FDR. The VMA engine maps that BAR range into the driver's address space with `device_ordered` or `write_combining` PTE attributes. The driver then uses ordinary `LD` and `ST` instructions for doorbells, status registers, and framebuffers. There is no `PULL`/`PUSH` command wrapper per register access.

PCIe BAR capabilities are page-granular. The Bus Master may mint only BAR FDRs whose offset and length are multiples of the system page size. The VMA engine checks the FDR at `MMAP` time and then relies on PTE permissions and memory type bits; it does not add sub-page bounds checks to every load/store.

This preserves the rule that ambient MMIO is forbidden. A process cannot load/store arbitrary physical device addresses. But if it holds a specific `pcie_bar` FDR, that FDR is the capability granting the right to map and access that device page range.

*   **`INB_RESERVED r_dest, r_port` / `OUTB_RESERVED r_port, r_src`**
    *   *Action:* Reserved fallback/debug port I/O for trusted boot or Bus Master code. Normal applications and ordinary drivers use FDR capabilities and `MMAP`-mapped BARs instead.
*   **`LOAD_UCODE r_buf_ptr, r_len`**
    *   *Action:* Reserved device-driver acceleration hook. In FPGA v1 this is a stub; it does not replace the Bus Master, IOMMU, BAR FDR, or capability-delegation model.

### 12. Floating Point & Vector Math (FPU/SIMD)
General compute isn't just integers. We need a standard IEEE 754 Floating Point Unit and SIMD (Single Instruction, Multiple Data) for multimedia and AI.

*   **`FADD`, `FSUB`, `FMUL`, `FDIV`**
    *   *Action:* Standard floating-point arithmetic operating on dedicated FPU registers (`f0` - `f31`).
*   **`VADD.32 v_dest, v_src1, v_src2`**
    *   *Action:* Vector addition. Adds multiple 32-bit integers simultaneously across wide vector registers (`v0` - `v15`), identical to AVX/NEON.

### 13. Bootstrapping (Hardware PID 1)
How does this machine actually turn on if there is no bootloader or kernel to load? The CPU itself is the bootloader.

Upon receiving power, the LNP64 executes a hardwired reset sequence:
1.  Initializes the hardware VMA tree and runqueue.
2.  Creates the initial hardware process/thread context (PID 1, TID 1, UID 0).
3.  Mounts the boot VFS from SD, SPI flash, or another already-described boot backend.
4.  If a boot manifest names a PCIe Bus Master and PCIe is present, creates that privileged process and grants it the Root Complex control FDR; otherwise PCIe enumeration is deferred.
5.  Automatically executes an internal equivalent of `OPEN_AT "/sbin/init"` and `EXEC` on that executable FDR.
6.  If `/sbin/init` is missing, the reset controller enters a hardware panic state and emits board diagnostics.

### 14. Paravirtual Unix Guest Profile
LNP64 does **not** add traditional kernel rings, mandatory syscall traps, or OS-owned page tables just to make Linux or NetBSD feel at home. The hardware remains POSIX-native. A Unix kernel port is plausible by treating Linux/NetBSD as a paravirtual personality process, similar in spirit to User-Mode Linux or a microkernel guest.

In this model, the silicon remains authoritative for:

*   Hardware process and thread creation.
*   Runqueue scheduling and context switching.
*   VMA creation, teardown, page faults, and copy-on-write.
*   File descriptor capabilities and Silicon VFS object references.
*   Signals, futex queues, fd readiness, and DMA completion.

The Linux/NetBSD personality owns:

*   Linux/BSD-specific process metadata and domain profiles for namespaces, cgroups, jails, credentials, and policy.
*   Compatibility APIs not directly represented by LNP64 opcodes.
*   Guest filesystems mounted inside large hardware VFS files.
*   Network stack policy above raw frame or datagram hardware objects.
*   Userland ABI conventions.

The targeted compatibility approaches are:

*   **Linux as a paravirtual personality:** A Linux kernel port runs as a supervisor Resource Domain over a delegated LNP64 process subtree. Linux tasks, files, memory mappings, signals, futexes, cgroups, containers, nested guests, and devices are projected onto native hardware primitives.
*   **Linux syscall compatibility runtime:** A loader/libc/runtime maps Linux syscall ABI calls onto native LNP64 instructions without booting a full Linux kernel. This is the shortest path to running many cloud-oriented programs.
*   **NetBSD rump-kernel style:** Selected NetBSD filesystem, networking, or device stacks run as LNP64 service processes. They receive block, network, PCIe, or delegated namespace FDRs and expose services back through native FDRs.

A full traditional Linux/NetBSD port that owns page tables, context switching, interrupts, and raw devices is not the v1 target.

The key hardware mechanism is a **Resource Domain**, not a privilege ring. A Resource Domain is a nested capability and accounting container for a process subtree, FDR authority, VMA/memory budget, scheduler budget, event policy, namespace root, and delegated devices. Virtual machines, containers, cgroups, jails, sandboxes, and supervisor domains are profiles of this same primitive.

`DOMAIN_CTL` creates child domains by delegating a subset of the caller's own authority downward. Limits are monotonic: a child domain cannot exceed resources or capabilities delegated by its parent. Usage accounting rolls up the domain tree, so CPU, memory, PID/thread, I/O, device, and event pressure can be queried or limited at any nesting level.

Nested virtualization is modeled as nested domains. A Linux personality domain can create a KVM-like guest domain; that guest can create another child guest or cgroup-like subtree. Each layer may receive, translate, or mask upcalls for its children, but hardware still enforces resource budgets, capability lineage, and VMA/FDR isolation.

Pre-provisioned domains can expose `call_gate` FDRs for hot cross-domain calls. This makes sandboxed libraries, service calls, driver calls, and guest/supervisor calls use the same capability-call path as cross-thread calls, while preserving domain budget accounting and capability checks.

A capability-marked domain can also act as a supervisor domain and receive upcalls for selected events: unsupported opcodes, delegated namespace lookups, permission decisions, child exit, signal delivery, fd readiness, timer expiry, futex events, block-image completion, resource pressure, limit violation, and process lifecycle changes.

Upcalls are delivered through a normal FDR with object class `control`. The supervisor pulls event records with `PULL` and pushes policy commands with `PUSH`. This keeps the design inside the FDR/VFS model instead of reintroducing a syscall path.

The precise claim is: native LNP64 POSIX operations are hardware commands, not
software traps. Compatibility personalities may still receive explicit hardware
upcalls for virtualization policy, unsupported opcodes, delegated namespaces,
and Linux syscall ABI emulation.

For this to be practical, LNP64 needs a stable psABI: calling convention, process entry layout, TLS, signal frame layout, errno convention, time/timer FDRs, and event-queue FDRs that can aggregate fd readiness, timers, child exit, signals, futex events, and supervisor upcalls.

For storage, a guest kernel can treat a large hardware VFS file as a paravirtual block device. It uses explicit-offset `PULL` and `PUSH`, then mounts ext4, FFS, or another guest filesystem inside that image. The hardware VFS provides the outer object and DMA; the guest kernel provides the inner filesystem semantics.

For physical PCIe devices, the PCIe Bus Master delegates `pcie_bar`, `dma_buffer`, and `irq_event` FDRs to guest or native driver processes. Drivers map BARs with `MMAP`, use `LD`/`ST` for device registers, use DMA buffer FDRs for device-visible memory, and wait on IRQ event FDRs for MSI/MSI-X completion.

For memory, the guest uses `MMAP`, `MUNMAP`, and `MPROTECT` to request native hardware VMAs. It does not write page tables directly. Linux/BSD tasks map one-to-one to hardware threads where practical, while the guest scheduler becomes an accounting and policy layer over the hardware runqueue.

This preserves the vision: Linux and NetBSD can be personalities projected onto native POSIX silicon, rather than forcing LNP64 to become another trap-and-kernel RISC machine.

### Native Security Invariants

LNP64 security is expressed through Resource Domains, VMAs, FDR capabilities, and hardware-owned object generations rather than through a separate kernel ring model.

Hard v1 invariants:

*   **W^X by default:** The VMA Engine rejects simultaneous writable and executable permissions unless a domain explicitly holds a JIT/loader policy bit. JITs use write-then-execute transitions with `MPROTECT` and `ISYNC`, not permanent RWX mappings.
*   **NX data:** Heap, stacks, queues, shared memory, DMA buffers, device BARs, signal frames, and ordinary anonymous mappings default non-executable. Executable mappings must originate from executable image objects or an explicitly authorized loader/JIT transition.
*   **ASLR:** Process startup, `EXEC`, `MMAP`, heap arenas, stacks, signal trampolines, shared objects, call-gate trampolines, and guard regions are randomized with hardware entropy unless disabled by a delegated domain policy.
*   **Guard pages:** Stacks, heap arenas, signal frames, large allocations, and selected runtime objects can request unmapped or no-access guard VMAs. Guard faults route through the normal hardware signal path.
*   **Entropy:** `RANDOM` is the architectural entropy source for libc, loaders, domain managers, allocator hardening, and compatibility personalities. `ENV_GET` reports feature bits; it does not provide secret randomness.
*   **Generation checks:** Domains, FDR entries, VMAs, heap arenas, waitable objects, call gates, event sources, DMA buffers, and mapped device objects carry generation fields. Stale references fail deterministically instead of silently reusing authority.
*   **Revocation:** `CAP_REVOKE`, `DOMAIN_CTL`, `MUNMAP`, `MPROTECT`, and object teardown invalidate cached descriptors, mappings, event sources, call gates, and DMA exports before authority is reused.
*   **Sealed and narrowed capabilities:** Authority can only move by explicit capability operations. Delegation may narrow rights, ranges, event masks, memory permissions, device scope, and transfer rights. Sealed capabilities can be used or transferred according to their rights but cannot be broadened or reminted by receivers.
*   **DMA isolation:** Internal DMA, `DMA_CTL`, file I/O DMA, Ethernet, SD/SPI, and PCIe requester DMA all pass through VMA/capability checks, the coherent DMA fabric, Resource Domain accounting, and IOMMU/device scope. No device may DMA to arbitrary DDR or bypass revocation.

### The Final Verdict
With these additions, the LNP64 has a coherent v1 shape: it boots into an `init` process, represents files and threads as hardware-managed resources, handles native page faults in VMA/MMU engines, and routes I/O through capability-checked FDR objects without a conventional kernel syscall path.

To make developers use the LNP64's silicon OS primitives, we should make the
native path faster, safer, and easier than recreating the same behavior in
software. The design should block ambient authority bypasses, but it should not
make language runtimes, Linux compatibility personalities, or NetBSD service
processes impossible.

Language runtimes and compatibility layers will still build their own abstractions when the hardware primitives are too narrow, too slow, or too awkward. The ISA should make the native path practical enough that runtimes can adopt it instead of bypassing it.

Here is how we tune the LNP64 ISA to prefer the "Silicon OS" paradigm without
breaking practical runtimes:

### 1. Hardware-Owned Thread Contexts (Without Locking the Stack Pointer)
To build a software scheduler (green threads/coroutines), a developer must be able to save the CPU registers to memory, change the Stack Pointer (`r31`), and jump to a new function. 
*   **The Fix:** Keep `r31` as an ordinary architectural register, but make hardware thread contexts, stacks, guard pages, and runqueue state first-class kernel-less objects.
*   **ISA Change:** `CLONE`, `YIELD`, `AWAIT`, futex waits, signal delivery, and supervisor upcalls operate on hardware-owned thread contexts. The MMU enforces stack VMA bounds and guard pages.
*   **The Result:** Language runtimes and compatibility layers can set up stacks normally, but native hardware threads are the efficient scheduling unit. Linux and NetBSD personalities can map tasks onto hardware threads instead of fighting a locked stack pointer.

### 2. Timer FDRs Instead of Ambient Timer Interrupts
Preemptive software schedulers (like the Linux kernel or Erlang's BEAM VM) rely on a periodic timer interrupt (e.g., every 1 millisecond) to pause the current task and run the scheduler logic.
*   **The Fix:** Do not expose an ambient periodic interrupt to every process. Expose time through monotonic/realtime reads, timer FDRs, `AWAIT` on timer waitables, and supervisor-domain timer upcalls.
*   **ISA Change:** Timer objects are FDR-backed wait sources. `AWAIT` and event-queue FDRs can wait on timers alongside fd readiness, signals, child exit, futex events, and supervisor upcalls.
*   **The Result:** Normal programs get POSIX-style sleep and timeout behavior, compatibility personalities can implement scheduler accounting and `clock_gettime`, and hardware still owns the actual runqueue.

### 3. Hardware Allocation as a Fast Path (Not a libc Killer)
If we only provide page-level `MMAP` (e.g., 4KB blocks), developers will still write software memory allocators (like `jemalloc` or `tcmalloc`) to hand out smaller chunks of memory, keeping a layer of software abstraction.
*   **The Fix:** Keep VMAs page-granular for MMU practicality, but make allocation a native hardware service through the Hardware Heap Engine.
*   **ISA Change:** `ALLOC r_dest, r_size`, `ALLOC_EX r_dest, r_request_block`, `ALLOC_SIZE r_dest, r_ptr`, and `FREE r_result, r_ptr` are v1 architectural instructions. The default heap is per-process, backed by anonymous VMAs, thread-safe in hardware, and integrated with `CLONE` copy-on-write and `EXEC` teardown.
*   **The Result:** Native programs, libc, and language runtimes get a fast, observable, guarded, thread-safe allocator by default. Custom allocators remain possible, but the native heap should be good enough that programmers are not tempted to replace it for general-purpose allocation. `MMAP` remains the right primitive for files, shared memory, executable mappings, DMA buffers, and device mappings.

### 4. Banish Ambient MMIO (Keeping Capability-Scoped MMIO)
Projects like DPDK (Data Plane Development Kit) bypass the OS entirely by mapping a network card's raw memory directly into user-space and polling it in software. Unchecked physical MMIO would bypass the VFS and the capability model.
*   **The Fix:** LNP64 forbids ambient MMIO. A general `LOAD` or `STORE` cannot target arbitrary physical device addresses. Device memory becomes accessible only when a process holds an FDR capability such as `pcie_bar` and maps it with `MMAP`.
*   **The Result:** Drivers can still get bare-metal register performance, but authority flows through the VFS. The Bus Master mints page-granular BAR capabilities, the VMA engine installs `device_ordered` or `write_combining` PTEs, and only then do ordinary `LD`/`ST` instructions reach the device.

### 5. Fast Scalar IPC
To reduce pressure to build ad hoc shared-memory IPC for small control messages, LNP64 includes a hardware scalar-message path.
*   **ISA Change:** **`MSG_SEND r_pid_dest, r_val1, r_val2`** remains the tiny scalar send fast path. Receiving is `AWAIT`/`PULL` over a message endpoint, queue, or call-gate completion object; byte or capability payloads use `PULL`/`PUSH` plus `CAP_SEND`/`CAP_RECV`.
*   **The Mechanism:** Because the hardware scheduler tracks parked receiver contexts, `MSG_SEND` can deliver a small fixed payload through scheduler/register-transfer fabric and wake a matching receiver without building a shared-memory queue.
*   **The Result:** Small control messages between isolated processes avoid most software IPC overhead. Larger byte streams and capability payloads still use queue/stream objects and FDR capability passing.

### 5.1 Capability Calls
For structured service boundaries, `CALL_CAP` and `RET_CAP` provide call/return semantics over FDR capabilities. A `call_gate` can target a parked worker thread, service queue, driver service, supervisor service, runtime actor, or Resource Domain entry point. This is the fast path for cross-thread and hot cross-domain calls; it does not make cold VM/container creation free, but it makes calls into already-provisioned isolated components cheap enough to use as a normal software abstraction.

Call gates have three profiles:

*   **Synchronous:** `CALL_CAP` parks the caller until the target executes `RET_CAP`, then writes bounded return values and wakes the caller.
*   **Asynchronous:** `CALL_CAP` starts or enqueues work and returns immediately with status or an operation id. Completion is delivered to an `event_queue`, `counter` completion profile, or service queue.
*   **Handoff:** `CALL_CAP` transfers request ownership to the target and does not create a return continuation for the original caller. This is useful for request routing, pipelines, and supervisor handoff.

Cross-domain calls charge resource usage according to call-gate policy, and capability passing is denied unless explicitly enabled by the gate.

### 6. Hardware-Owned Runtime Objects
The broader reusable primitive is not just "everything is a file." It is hardware-owned waitable/capability objects with local state, bounded transitions, and event delivery.
*   **The Fix:** Expose only three primitive generic object classes in hardware: `counter`, `queue`, and `memory_object`. Channels, semaphores, completions, event counters, task queues, shared arenas, DMA buffers, and runtime events are profiles over those primitives, not separate hardware modules.
*   **ISA Change:** `OBJECT_CTL` creates/configures these primitives. `PULL`/`PUSH` move queue records, `AWAIT` parks on counter/queue/memory-object state changes, `CAP_*` delegates authority, `MMAP` maps memory objects, and `DMA_CTL` accelerates large copy/fill/scatter-gather work.
*   **The Result:** Heap, Event, Futex, VMA, Capability, Signal, and DMA hardware become useful to normal application code: async runtimes, channels, worker pools, language schedulers, safe handles, arenas, guard pages, large copies, and runtime synchronization all share the same hard blocks.

### Summary of the Strategy
By avoiding ambient device memory, making hardware wait queues and FDRs the
natural event model, exposing reusable hardware-owned waitable/capability
objects, and providing fast native allocation, IPC, and DMA bulk movement, LNP64 makes
the hardware primitives the path of least resistance without blocking practical
language runtimes or Unix compatibility personalities.
