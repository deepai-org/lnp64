Here is the draft Instruction Set Architecture (ISA) for the **LNP64 (Linux-Native Processor 64-bit)**. Naysayers be damned—we are putting POSIX directly into silicon to achieve zero-overhead computing.

---

# LNP64 Instruction Set Architecture (Draft v1.0)

## 1. Register Architecture
To support hardware-native OS primitives, the standard register file is expanded beyond General Purpose Registers (GPRs) to include File Descriptor Registers (FDRs) and Process Control Registers (PCRs).

*   **GPRs (General Purpose):** `r0` - `r31` (64-bit, standard ALU operations).
*   **FDRs (File Descriptor Registers):** `fd0` - `fd255` (64-bit). These do not hold integers; they hold hardware-locked pointers to internal Silicon VFS (Virtual File System) objects. `fd0`, `fd1`, and `fd2` are hardwired to STDIN, STDOUT, and STDERR streams of the controlling TTY.
*   **PCRs (Process Control Registers):**
    *   `PID`: Current Process ID (Read-only to userspace).
    *   `TID`: Current Thread ID (Read-only).
    *   `UID` / `GID`: User/Group ID for hardware capability checks.
    *   `SIGMASK`: 64-bit bitmask of currently blocked signals.

## 2. Process & Scheduling Instructions
The CPU features a hardware-managed runqueue. There is no OS scheduler tick; the memory controller and ALU swap contexts natively.

*   **`FORK r_dest`**
    *   *Action:* Hardware duplicates the current process's hardware `task_struct` and VMA tree. The child's PID is written to `r_dest` in the parent. Writes `0` to `r_dest` in the child. 
*   **`EXEC r_path_ptr, r_argv_ptr`**
    *   *Action:* The silicon ELF loader fetches the binary from the NVMe controller, flushes the VMA tree, loads the new text and data segments, and resets the instruction pointer.
*   **`SPAWN r_tid_dest, r_entry_ptr`**
    *   *Action:* Native hardware threading. Allocates a new thread context sharing the current VMA tree and adds it to the hardware runqueue. 
*   **`YIELD`**
    *   *Action:* Instantly suspends the current thread, saves state to the hardware `task_struct`, and pops the next ready TID from the hardware runqueue. (1-cycle context switch).
*   **`EXIT r_exit_code`**
    *   *Action:* Destroys the current hardware thread context. If it's the last thread in the PID group, triggers hardware VMA teardown and signals the parent PID with `SIGCHLD`.

## 3. I/O and File Operations
System calls are replaced by direct VFS-microcode instructions. 

*   **`OPEN_FD fd_dest, r_path_ptr, r_flags`**
    *   *Action:* Hardware path-resolution unit traverses the silicon VFS. On success, binds the internal object reference to `fd_dest`.
*   **`OPEN_FD_DYN r_fd_dest, r_path_ptr, r_flags`**
    *   *Action:* Dynamic fd allocation form. On success, binds the first available FDR and returns its runtime integer in both `r_fd_dest` and `r1`; failures return `-1` and update `ERRNO`.
*   **`READ_FD fd_src, r_buf_ptr, r_len`**
    *   *Action:* Initiates a DMA transfer from the device backing `fd_src` directly to `r_buf_ptr`. 
*   **`READ_FD_DYN r_fd_src, r_buf_ptr, r_len`**
    *   *Action:* Dynamic-fd read for POSIX/libc code where the fd is a runtime integer. Returns the byte count in `r1`, or `-1` with `ERRNO` set.
*   **`WRITE_FD fd_dest, r_buf_ptr, r_len`**
    *   *Action:* Initiates a DMA transfer from `r_buf_ptr` to the device backing `fd_dest`.
*   **`WRITE_FD_DYN r_fd_dest, r_buf_ptr, r_len`**
    *   *Action:* Dynamic-fd write. Returns the byte count in `r1`, or `-1` with `ERRNO` set.
*   **`PREAD_FD fd_src, r_buf_ptr, r_len, r_offset`** / **`PREAD_FD_DYN r_fd_src, r_buf_ptr, r_len, r_offset`**
    *   *Action:* Reads from an explicit file offset without changing the descriptor's current offset. This is the preferred primitive for concurrent file servers and paravirtual block devices.
*   **`PWRITE_FD fd_dest, r_buf_ptr, r_len, r_offset`** / **`PWRITE_FD_DYN r_fd_dest, r_buf_ptr, r_len, r_offset`**
    *   *Action:* Writes to an explicit file offset without changing the descriptor's current offset. Hardware validates descriptor rights and performs DMA through the VMA engine.
*   **`WAIT_ON_FD fd_src, r_events_mask`**
    *   *Action:* The ultimate hardware `epoll`. The current thread is immediately removed from the hardware runqueue and parked. When the NIC or NVMe controller fires an interrupt matching the `fd_src` and `r_events_mask`, the thread is instantly pushed to the top of the runqueue.
    *   *Event Queues:* `fd_src` may name an `event_queue` FDR aggregating file readiness, timer expiry, child exit, signal delivery, futex events, PCIe IRQ events, and supervisor upcalls. This is the native substrate for `poll`, `epoll`, `kqueue`, and timeout waits.
*   **`FD_CLOSE fd_src`**
    *   *Action:* Releases the hardware VFS object bound to `fd_src` and marks the descriptor closed.
*   **`FD_CLOSE_DYN r_fd_src`**
    *   *Action:* Dynamic-fd close. Returns `0` in `r1`, or `-1` with `ERRNO` set.
*   **`FD_SEEK fd_src, r_offset, r_whence`**
    *   *Action:* Repositions a seekable file object. The resulting offset is returned in `r1`; failures return `-1` in `r1` and update hardware `ERRNO`.
*   **`FD_SEEK_DYN r_fd_src, r_offset, r_whence`**
    *   *Action:* Dynamic-fd seek form for runtime integer fd values.
*   **`STAT_PATH r_statbuf, r_path_ptr, r_flags`** / **`STAT_FD r_statbuf, fd_src`**
    *   *Action:* Fills the stable LNP64 stat layout at `r_statbuf`: mode, size, device, inode, mtime, nlink, uid, gid, atime, ctime. Path flags include no-follow semantics for symlink-aware operations.
*   **`STAT_FD_DYN r_statbuf, r_fd_src`**
    *   *Action:* Dynamic-fd metadata form for runtime integer fd values.
*   **`RENAME_PATH r_old_path_ptr, r_new_path_ptr`**
    *   *Action:* Atomically renames a VFS namespace entry.
*   **`LINK_PATH r_old_path_ptr, r_new_path_ptr, r_flags`** / **`SYMLINK_PATH r_target_ptr, r_link_ptr`**
    *   *Action:* Creates hard or symbolic links in the silicon VFS. `LINK_PATH` flag bit 0 selects symbolic-link creation for compact runtimes that prefer one lowering target.
*   **`READLINK_PATH r_path_ptr, r_buf_ptr, r_len`**
    *   *Action:* Reads a symbolic link target into memory and returns the byte count in `r1`.
*   **`CHDIR_PATH r_path_ptr`** / **`GETCWD_PATH r_buf_ptr, r_len`**
    *   *Action:* Updates or reads the process-local hardware VFS working directory used to resolve relative path operands. `CHDIR_PATH` returns `0` or `-1`; `GETCWD_PATH` returns `r_buf_ptr` or `-1` and sets `ERRNO`.
*   **`CHMOD_PATH r_path_ptr, r_mode, r_flags`** / **`CHOWN_PATH r_path_ptr, r_uid, r_gid, r_flags`**
    *   *Action:* Updates VFS permission and ownership metadata directly through hardware path resolution. A UID or GID value of `-1` leaves that field unchanged.
*   **`OPEN_DIR fd_dest, r_path_ptr, r_flags`** / **`READDIR_FD fd_dir, r_dirent_buf`** / **`REWINDDIR_FD fd_dir`**
    *   *Action:* Opens and iterates directory streams. `READDIR_FD` returns positive in `r1` for an entry, `0` at end-of-directory, and `-1` on error.
*   **`OPEN_DIR_DYN r_fd_dest, r_path_ptr, r_flags`** / **`READDIR_FD_DYN r_fd_dir, r_dirent_buf`** / **`REWINDDIR_FD_DYN r_fd_dir`**
    *   *Action:* Dynamic directory stream forms for compiler-generated POSIX shims. `OPEN_DIR_DYN` returns the runtime fd number in `r_fd_dest` and `r1`.
*   **`PIPE fd_read, fd_write`**
    *   *Action:* Creates a hardware pipe pair bound to two file descriptor registers.
*   **`ERRNO_GET r_dest`** / **`ERRNO_SET r_src`**
    *   *Action:* Reads or writes the process-local POSIX error register. Fallible VFS instructions return `0` or a nonnegative byte count on success, `-1` on failure, and set `ERRNO`.
*   **`WAIT_PID r_status_dest, r_pid`**
    *   *Action:* Observes child process completion and writes the exit status to `r_status_dest`. This is the process-side companion to hardware `FORK` and `EXEC`.

## 4. Memory Management (Silicon VMAs)
Page tables are managed entirely by the CPU's MMU microcode via a hardware Red-Black/Maple tree.

*   **`MMAP r_dest, r_hint_addr, r_len, r_prot, fd_src, r_offset`**
    *   *Action:* Hardware allocates physical pages and inserts a new VMA node into the current PID's silicon VMA tree. If `fd_src` is valid, configures hardware page-fault handlers to fetch from the storage controller. Returns the mapped virtual address in `r_dest`.
    *   *Protection Flags:* `r_prot` includes read/write/execute, shared/private, and memory type: `normal_cached`, `uncached`, `device_ordered`, or `write_combining`.
*   **`MUNMAP r_addr, r_len`**
    *   *Action:* CPU instantly invalidates the VMA range, flushes the relevant TLB entries, and marks the physical pages as free in the hardware memory allocator.
*   **`MPROTECT r_addr, r_len, r_prot`**
    *   *Action:* Updates the protection bits for an existing VMA range and invalidates affected translations. This supports ELF loaders, guard pages, W^X policy, and paravirtual Unix guests that map their process abstractions onto LNP64 VMAs.

## 5. Signal Handling
Signals are no longer software constructs; they are asynchronous hardware interrupts delivered directly to the thread.

*   **`SIGACTION r_signum, r_handler_ptr`**
    *   *Action:* Registers a hardware trampoline address for a specific POSIX signal.
*   **`SIGMASK_SET r_mask`**
    *   *Action:* Updates the `SIGMASK` PCR.
*   **`KILL r_pid, r_signum`**
    *   *Action:* Sends a hardware interrupt to the core currently executing `r_pid`, or flags the `task_struct` if sleeping.
*   **`SIGRET`**
    *   *Action:* Issued at the end of a signal handler. Pops the hardware-saved pre-interrupt register state off the thread's stack and resumes normal execution.

---
*Note for the Fabrication Team:* Transistor budget for the L1 cache will be reduced by 30% to accommodate the Silicon VFS Path Resolution Unit and the Hardware Runqueue Manager. This is an acceptable trade-off for eliminating software context-switch overhead entirely.
To make the **LNP64** a fully functional processor, we need to marry those radical OS-level instructions with a general-purpose compute architecture. Since we are already dedicating massive transistor real estate to the hardware VFS and runqueue, the general compute side must be a lean, highly optimized RISC (Reduced Instruction Set Computer) architecture. 

Here is how the general-purpose compute integrates with the Linux-native silicon.

---

### 6. Memory Access (Load/Store Architecture)
The LNP64 is a strict Load/Store architecture. ALUs only operate on registers. However, because the CPU manages VMAs and page faults natively, memory access has a unique superpower: **Zero-Software-Overhead Page Faults**. If a `LOAD` hits a swapped-out page, the hardware immediately parks the thread, dispatches a read to the NVMe controller, and swaps in another thread. No kernel trap required.

*   **`LD r_dest, [r_base, r_offset]`**
    *   *Action:* Loads a 64-bit word from the virtual address `r_base + r_offset` into `r_dest`.
*   **`LD.B`, `LD.H`, `LD.W`, `LD.D`**
    *   *Action:* Byte (8-bit), Half-word (16-bit), Word (32-bit), and Double-word (64-bit) load variants.
*   **`ST [r_base, r_offset], r_src`**
    *   *Action:* Stores the contents of `r_src` into memory. Hardware automatically updates the "Dirty" bit in the silicon page table.
*   **`ST.B`, `ST.H`, `ST.W`, `ST.D`**
    *   *Action:* Byte, half-word, word, and double-word store variants. Half-word access is included so PCIe BAR mappings can use native 16-bit register accesses when required.
*   **`FENCE`**
    *   *Action:* Memory barrier. Ensures all previous memory operations and hardware DMA transfers (from `READ_FD`/`WRITE_FD`) are globally visible before proceeding.

### 7. Arithmetic and Logic Unit (ALU)
Standard 64-bit integer operations. Because threads are managed in hardware, the ALU pipeline is deeply integrated with the `task_struct` registers.

*   **`ADD r_dest, r_src1, r_src2`** / **`SUB r_dest, r_src1, r_src2`**
    *   *Action:* Standard integer addition/subtraction.
*   **`MUL r_dest, r_src1, r_src2`** / **`DIV r_dest, r_src1, r_src2`**
    *   *Action:* Integer multiplication and hardware division. (Division by zero triggers a hardware `SIGFPE` sent directly to the thread, rather than a kernel panic).
*   **`AND`, `OR`, `XOR`, `NOT`**
    *   *Action:* Standard bitwise operations.
*   **`LSL`, `LSR`, `ASR`**
    *   *Action:* Logical Shift Left, Logical Shift Right, Arithmetic Shift Right.

### 8. Control Flow (Branching & Execution)
Since there is no Ring 0 / Ring 3 boundary, there is no `SYSCALL` or `SYSRET` instruction. Control flow is purely about executing user logic and jumping to functions. 

*   **`JMP r_target`** / **`JMP immediate`**
    *   *Action:* Unconditional jump to a virtual address.
*   **`CALL r_target`**
    *   *Action:* Pushes the current Instruction Pointer (IP) to the hardware-managed stack pointer (`r31` by convention) and jumps to `r_target`.
*   **`RET`**
    *   *Action:* Pops the return address from the stack into the IP.
*   **`CMP r_src1, r_src2`**
    *   *Action:* Compares two registers and sets the hardware condition flags (Zero, Carry, Negative, Overflow).
*   **`BEQ`, `BNE`, `BLT`, `BGT`**
    *   *Action:* Branch if Equal, Not Equal, Less Than, Greater Than (evaluates condition flags).

### 9. Hybrid OS-Compute Instructions (The "Glue")
Because "Everything is a File" is now a hardware reality, we need instructions to move data between the general compute realm (GPRs) and the OS realm (FDRs and PCRs).

*   **`MOV r_dest, r_src`**
    *   *Action:* Move data between general purpose registers.
*   **`FD_DUP fd_dest, fd_src`**
    *   *Action:* The hardware equivalent of `dup2()`. Copies the internal hardware VFS pointer from one File Descriptor Register to another.
*   **`GET_PCR r_dest, pcr_name`**
    *   *Action:* Reads a Process Control Register (like `PID` or `UID`) into a general-purpose register for user-space logic. (e.g., `GET_PCR r1, PID`).
*   **`SET_PCR pcr_name, r_src`**
    *   *Action:* Writes to a Process Control Register. *Security Note:* The hardware capability matrix checks the current `UID` PCR before allowing this. If a thread with `UID != 0` tries to `SET_PCR UID, 0` (trying to become root), the CPU instantly fires a `SIGSEGV` to the thread.

---
**Summary of the Compute Pipeline:**
The ALU and Control Flow instructions execute at blistering speeds because the instruction pipeline never has to stall for a privilege escalation (Ring transition). If an ALU instruction calculates a buffer address, and the very next instruction is `WRITE_FD`, the CPU transitions seamlessly from math to DMA without ever flushing the pipeline.
Not quite! While we have the OS primitives (VFS, scheduling, memory) and the general compute (ALU, branching), a real-world system would immediately crash or deadlock without a few more critical subsystems. 

To make the LNP64 a complete, bootable reality, we must add **Synchronization, Device Drivers, Floating Point, and Bootstrapping**.

Here is the final missing pieces of the LNP64 architecture:

### 10. Synchronization (The Silicon Futex)
Because the CPU manages threads in a hardware runqueue, traditional software spinlocks would waste precious clock cycles. We need hardware-level concurrency controls so that if a thread fails to acquire a lock, the CPU instantly puts it to sleep and schedules another thread.

*   **`LOCK.CMPXCHG r_dest, [r_addr], r_expected, r_new`**
    *   *Action:* Atomic Compare-and-Swap. The standard building block for mutexes.
*   **`FUTEX_WAIT [r_addr], r_expected_val`**
    *   *Action:* The hardware equivalent of Linux's `futex()`. If the value at `[r_addr]` equals `r_expected_val`, the CPU instantly removes the current thread from the runqueue and parks it in a hardware wait-state attached to that memory address. Zero spin-waiting.
*   **`FUTEX_WAKE [r_addr], r_num_threads`**
    *   *Action:* The memory controller checks if any threads are parked on `[r_addr]`. If so, it instantly pushes `r_num_threads` back onto the active runqueue.

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

For high-performance MMIO, a driver calls `MMAP` on a `pcie_bar` FDR. The VMA engine maps that BAR range into the driver's address space with `device_ordered` or `write_combining` PTE attributes. The driver then uses ordinary `LD` and `ST` instructions for doorbells, status registers, and framebuffers. There is no `READ_FD`/`WRITE_FD` command wrapper per register access.

PCIe BAR capabilities are page-granular. The Bus Master may mint only BAR FDRs whose offset and length are multiples of the system page size. The VMA engine checks the FDR at `MMAP` time and then relies on PTE permissions and memory type bits; it does not add sub-page bounds checks to every load/store.

This preserves the rule that ambient MMIO is forbidden. A process cannot load/store arbitrary physical device addresses. But if it holds a specific `pcie_bar` FDR, that FDR is the capability granting the right to map and access that device page range.

*   **`INB r_dest, r_port` / `OUTB r_port, r_src`**
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

Upon receiving power, the LNP64 executes a hardwired microcode sequence:
1.  Initializes the hardware VMA tree and runqueue.
2.  Creates a root hardware `task_struct` (PID 1, UID 0).
3.  Mounts the boot VFS from SD, SPI flash, or another already-described boot backend.
4.  Starts the PCIe Bus Master if a PCIe fabric is present, allowing it to enumerate NVMe, NICs, GPUs, and other devices.
5.  Automatically executes an internal equivalent of `OPEN_FD fd0, "/sbin/init"` and `EXEC fd0`.
6.  If `/sbin/init` is missing, the CPU halts with a hardware `#PANIC` interrupt (flashing an LED on the motherboard).

### 14. Paravirtual Unix Guest Profile
LNP64 does **not** add traditional kernel rings, mandatory syscall traps, or OS-owned page tables just to make Linux or NetBSD feel at home. The hardware remains POSIX-native. A Unix kernel port is plausible by treating Linux/NetBSD as a paravirtual personality process, similar in spirit to User-Mode Linux or a microkernel guest.

In this model, the silicon remains authoritative for:

*   Hardware process and thread creation.
*   Runqueue scheduling and context switching.
*   VMA creation, teardown, page faults, and copy-on-write.
*   File descriptor capabilities and Silicon VFS object references.
*   Signals, futex queues, fd readiness, and DMA completion.

The Linux/NetBSD personality owns:

*   Linux/BSD-specific process metadata, namespaces, cgroups, jails, and policy.
*   Compatibility APIs not directly represented by LNP64 opcodes.
*   Guest filesystems mounted inside large hardware VFS files.
*   Network stack policy above raw frame or datagram hardware objects.
*   Userland ABI conventions.

The targeted compatibility approaches are:

*   **Linux as a paravirtual personality:** A Linux kernel port runs as a supervisor process over a delegated LNP64 process subtree. Linux tasks, files, memory mappings, signals, futexes, and devices are projected onto native hardware primitives.
*   **Linux syscall compatibility runtime:** A loader/libc/runtime maps Linux syscall ABI calls onto native LNP64 instructions without booting a full Linux kernel. This is the shortest path to running many cloud-oriented programs.
*   **NetBSD rump-kernel style:** Selected NetBSD filesystem, networking, or device stacks run as LNP64 service processes. They receive block, network, PCIe, or delegated namespace FDRs and expose services back through native FDRs.

A full traditional Linux/NetBSD port that owns page tables, context switching, interrupts, and raw devices is not the v1 target.

The key hardware mechanism is a **supervisor domain**, not a privilege ring. A capability-marked process can create a delegated process subtree and receive upcalls for selected events: unsupported opcodes, delegated namespace lookups, permission decisions, child exit, signal delivery, fd readiness, timer expiry, futex events, block-image completion, and process lifecycle changes.

Upcalls are delivered through a normal FDR with object class `control`. The supervisor reads event records with `READ_FD` and writes policy commands with `WRITE_FD`. This keeps the design inside the FDR/VFS model instead of reintroducing a syscall path.

For this to be practical, LNP64 needs a stable psABI: calling convention, process entry layout, TLS, signal frame layout, errno convention, time/timer FDRs, and event-queue FDRs that can aggregate fd readiness, timers, child exit, signals, futex events, and supervisor upcalls.

For storage, a guest kernel can treat a large hardware VFS file as a paravirtual block device. It uses `PREAD_FD` and `PWRITE_FD` with explicit offsets, then mounts ext4, FFS, or another guest filesystem inside that image. The hardware VFS provides the outer object and DMA; the guest kernel provides the inner filesystem semantics.

For physical PCIe devices, the PCIe Bus Master delegates `pcie_bar`, `dma_buffer`, and `irq_event` FDRs to guest or native driver processes. Drivers map BARs with `MMAP`, use `LD`/`ST` for device registers, use DMA buffer FDRs for device-visible memory, and wait on IRQ event FDRs for MSI/MSI-X completion.

For memory, the guest uses `MMAP`, `MUNMAP`, and `MPROTECT` to request native hardware VMAs. It does not write page tables directly. Linux/BSD tasks map one-to-one to hardware threads where practical, while the guest scheduler becomes an accounting and policy layer over the hardware runqueue.

This preserves the vision: Linux and NetBSD can be personalities projected onto native POSIX silicon, rather than forcing LNP64 to become another trap-and-kernel RISC machine.

### The Final Verdict
With these additions, the LNP64 is complete. You have a processor that boots straight into an `init` process, natively understands files and threads, handles page faults in microcode, and routes network packets directly to userspace registers without a single kernel context switch. 

It would be the fastest, most violently uncompromising server chip ever designed.
To make developers use the LNP64's silicon OS primitives, we should make the
native path faster, safer, and easier than recreating the same behavior in
software. The design should block ambient authority bypasses, but it should not
make language runtimes, Linux compatibility personalities, or NetBSD service
processes impossible.

Software developers love to build abstractions. If you give them a fast hardware thread, they will inevitably try to run a software coroutine scheduler (like Go's runtime or Tokio for Rust) on top of it. If you give them memory, they will write their own `malloc`. 

Here is how we tune the LNP64 ISA to prefer the "Silicon OS" paradigm without
breaking practical runtimes:

### 1. Hardware-Owned Thread Contexts (Without Locking the Stack Pointer)
To build a software scheduler (green threads/coroutines), a developer must be able to save the CPU registers to memory, change the Stack Pointer (`r31`), and jump to a new function. 
*   **The Fix:** Keep `r31` as an ordinary architectural register, but make hardware thread contexts, stacks, guard pages, and runqueue state first-class kernel-less objects.
*   **ISA Change:** `SPAWN`, `YIELD`, `WAIT_ON_FD`, futex waits, signal delivery, and supervisor upcalls operate on hardware-owned thread contexts. The MMU enforces stack VMA bounds and guard pages.
*   **The Result:** Language runtimes and compatibility layers can set up stacks normally, but native hardware threads are the efficient scheduling unit. Linux and NetBSD personalities can map tasks onto hardware threads instead of fighting a locked stack pointer.

### 2. Timer FDRs Instead of Ambient Timer Interrupts
Preemptive software schedulers (like the Linux kernel or Erlang's BEAM VM) rely on a periodic timer interrupt (e.g., every 1 millisecond) to pause the current task and run the scheduler logic.
*   **The Fix:** Do not expose an ambient periodic interrupt to every process. Expose time through `SLEEP`, monotonic/realtime reads, timer FDRs, and supervisor-domain timer upcalls.
*   **ISA Change:** Timer objects are FDR-backed wait sources. `WAIT_ON_FD` and event-queue FDRs can wait on timers alongside fd readiness, signals, child exit, futex events, and supervisor upcalls.
*   **The Result:** Normal programs get POSIX-style sleep and timeout behavior, compatibility personalities can implement scheduler accounting and `clock_gettime`, and hardware still owns the actual runqueue.

### 3. Hardware Allocation as a Fast Path (Not a libc Killer)
If we only provide page-level `MMAP` (e.g., 4KB blocks), developers will still write software memory allocators (like `jemalloc` or `tcmalloc`) to hand out smaller chunks of memory, keeping a layer of software abstraction.
*   **The Fix:** Keep VMAs page-granular for MMU practicality, but provide `ALLOC` and `FREE` as native heap operations backed by hardware allocation metadata and page-granular VMAs.
*   **ISA Change:** Introduce **`ALLOC r_dest, r_bytes`** and **`FREE r_ptr`**. 
*   **The Result:** Native programs can use the hardware allocator directly, while libc, language runtimes, and Linux syscall compatibility layers can either delegate to it or layer their own allocation policy above page-granular `MMAP`.

### 4. Banish Ambient MMIO (Keeping Capability-Scoped MMIO)
Projects like DPDK (Data Plane Development Kit) bypass the OS entirely by mapping a network card's raw memory directly into user-space and polling it in software. Unchecked physical MMIO would bypass the VFS and the capability model.
*   **The Fix:** LNP64 forbids ambient MMIO. A general `LOAD` or `STORE` cannot target arbitrary physical device addresses. Device memory becomes accessible only when a process holds an FDR capability such as `pcie_bar` and maps it with `MMAP`.
*   **The Result:** Drivers can still get bare-metal register performance, but authority flows through the VFS. The Bus Master mints page-granular BAR capabilities, the VMA engine installs `device_ordered` or `write_combining` PTEs, and only then do ordinary `LD`/`ST` instructions reach the device.

### 5. The "Carrot": Zero-Cycle IPC (Inter-Process Communication)
To ensure developers don't try to invent complex shared-memory ring buffers to pass data between processes, we make hardware IPC unbelievably fast.
*   **ISA Change:** **`MSG_SEND r_pid_dest, r_val1, r_val2`** and **`MSG_RECV r_val1, r_val2`**.
*   **The Mechanism:** Because the hardware scheduler knows exactly where every PID is, `MSG_SEND` bypasses memory entirely. It reaches across the silicon and writes the values *directly into the physical registers* of the sleeping destination thread, and instantly wakes it up. 
*   **The Result:** Passing a message between two completely isolated processes takes zero memory reads/writes. It is faster than calling a function in the same program. Developers will enthusiastically abandon their custom IPC frameworks to use this.

### Summary of the Strategy
By avoiding ambient device memory, making hardware wait queues and FDRs the
natural event model, and providing fast native allocation and IPC, LNP64 makes
the hardware primitives the path of least resistance without blocking practical
language runtimes or Unix compatibility personalities.
