# Redis-on-LNP64 under ISA v2 — status

_Last updated: 2026-06-23_

## Summary

Redis 7.0.15 now **boots and serves clients** under the ISA-v2 emulator. The
full smoke test (`scripts/run_redis.sh`) passes end-to-end:
PING/SET/GET/DEL/INCR/RPUSH+LRANGE/HSET+HGET/SADD+SCARD/SISMEMBER/SMEMBERS/KEYS.
Three emulator bugs were found and fixed (all committed).

## Fixed (committed: `emulator(isa-v2): per-thread LR/SC reservation + gate retire trace to fix OOM`)

1. **Per-thread LR/SC reservation.** `reservation_addr` was per-machine and was
   cleared on every context switch. Once `bioInit()` spawned background threads,
   they and the main thread invalidated each other's reservations, livelocking
   every atomic (`pthread_mutex_lock`, `atomicIncr`) into a permanent LR/SC spin
   (~20 min "startup"). Moved into `Thread`; removed scheduler-level clears.

2. **Unbounded retire-trace OOM (host-crash cause).** `run_committed_exec`
   pushed a ~160-byte `CommittedExecRetireRecord` for *every* retired
   instruction. `run-elf` retires billions of instructions and never consumes
   the trace → the Vec grew to ~80 GB RSS and OOM-wedged the host (twice today).
   Gated behind `record_retire_trace` (default off); only the flat-exec / RTL
   co-sim paths that emit `EMULATOR_RETIRE` enable it. Verified: Redis init now
   holds flat RSS (~15 MB) and reaches Ready in ~15 s.

## FIXED — client socket reads targeted the wrong fd

_Committed: `emulator(isa-v2): PULL/PUSH/AWAIT/AWAITEX read fd handle from GPR`._

**Symptom.** Server reached Ready; a client TCP connect succeeded and was
accepted (host-side), but Redis never replied (PING timed out). The kernel
showed `Recv-Q=6` — the 6 bytes of `PING\r\n` sat unread — and the emulator
spun at 100 % CPU.

**Root cause.** After `accept()` installs the client socket at emulator-fd
**9**, Redis's `readQueryFromClient` issues `read(9)` → `__lnp_pull` → the
**`PULL` (opcode 0x2b)** instruction. The emulator's `Instr::Pull` handler used
**`fd.0`** — the *register number* of the operand — directly as the fd index:

```rust
// src/emulator.rs  (Instr::Pull / Push / Await / AwaitEx) — OLD, buggy
if let Some(count) = self.read_fd_index(fd.0, addr, len)? { ... }
```

The fd value (9) is held in register **r2** (a0), so `fd.0 == 2` and the read
hit emulator-fd **2 = Stderr**. The real client fd was never read. (Writes had
the same bug, masked by the `2>&1` log redirect.)

**Why this was the ISA-v2 contract, not the caller's fault.** Under the v2
"capabilities are GPR handles" migration, `PULL`/`PUSH`/`AWAIT` take their fd
operand as a **GPR whose value is the fd handle**, not a static fd-register
index:

- LLVM backend: `LNP64InstrInfo.td:750` — `def PULL ... (LNP64pull GPR:$cap, ...)`.
- Canonical v2 ABI asm: `toolchain/liblnp64_min.s` — `read: pull r2, r2, r3, r4`
  (fd value in r2).

**The fix.** The static `Instr::Pull` (0x2b), `Instr::Push` (0x2c),
`Instr::Await` (0x2e) and `Instr::AwaitEx` (0x71) handlers now read the fd
**value** from the GPR named by the operand and resolve it via
`decode_fd_value` / `checked_fd_index` — identical to their `*Dyn` twins. After
the FDR→GPR migration the static and dynamic forms are semantically the same.
The ~10 unit tests and the legacy `src/asm.rs` `fdN` asm-syntax tests that
encoded the old "n is the fd index" convention were updated to preload the
operand GPR with the fd handle value.

## FDR→GPR migration completed for all fd-handle instructions

The migration was finished so that **every** instruction whose operand is an
fd *handle* resolves it from a GPR (read_reg + `decode_fd_value` /
`checked_fd_index`), identical to the `*Dyn` twins:

- `ReadFd` (0x2d), `WriteFd` (0x57), `CallCap` (0x2f), `Mmap` (0x6a),
  `WaitableProbe` (0x6f) — the decodable, LLVM-emitted forms.
- `PreadFd`, `PwriteFd`, `ReaddirFd`, `RewinddirFd`, `StatFd`, `UtimeFd`,
  `FdSeek`, `FdClose`, `WaitOnFd` — static forms paired with existing `*Dyn`
  twins.

`Mmap`: an fd value that does not resolve to a file-backed fd produces an
anonymous mapping (also fixes real anonymous `mmap` passing a `-1` sentinel,
which previously indexed `fds[]` directly).

**Intentionally left index/slot-based** (documented in code): `OpenFd` /
`OpenDir` — the `dst` operand is an *output* slot to install into, not an
input handle; the GPR-model equivalents are `OpenFdDyn` / `OpenDirDyn`. And
`FdDup` / `FdDup2` — `dup2`'s target is a slot by definition; these have no
`*Dyn` twin and no binary opcode (not LLVM-emitted), so a "handle value"
interpretation of the destination is meaningless. Migrating them cleanly would
require first defining `dup`-under-capabilities semantics.

`cargo test`: 473 passed / 0 failed (the 4 previously-failing tests — stale
after the `ARG_BASE=0x1900000` / `STACK_TOP=0x1800000` layout migration — were
also fixed). Full Redis smoke test still passes end-to-end.

## Repro / verification harness

- `LNP64_BIN=./target/release/lnp64 bash scripts/run_redis.sh` (timeout bumped
  to 1800 s). Always run the emulator **under `ulimit -v`** and detached
  (`setsid`) — a foreground `lnp64 run-elf` shares the shell's process group and
  a host watchdog/earlyoom can take the shell down with it.
- Instrument `read_fd_index` to print `fd`/`len`/handle-kind, set a
  `dbg_post_accept` flag in `object_ctl_socket_accept`, and sample the guest PC
  in `run_committed_exec` to reproduce the diagnosis.
