# Redis-on-LNP64 under ISA v2 — status & remaining bug

_Last updated: 2026-06-23_

## Summary

Redis 7.0.15 now **boots** under the ISA-v2 emulator and reaches
`Ready to accept connections` in ~15 s. Two emulator bugs were found and fixed
(committed); one **networking bug remains** and is fully diagnosed below.

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

## REMAINING BUG — client socket reads target the wrong fd

**Symptom.** Server reaches Ready; a client TCP connect succeeds and is accepted
(host-side), but Redis never replies (PING times out). The kernel shows
`Recv-Q=6` — the 6 bytes of `PING\r\n` sit unread — and the emulator spins at
100 % CPU.

**Root cause (verified by instrumentation).** After `accept()` installs the
client socket at emulator-fd **9**, Redis's `readQueryFromClient` issues
`read(9)` → `__lnp_pull` → the **`PULL` (opcode 0x2b)** instruction. The
emulator's `Instr::Pull` handler uses **`fd.0`** — the *register number* of the
operand — directly as the fd index:

```rust
// src/emulator.rs  (Instr::Pull / Push / Await / AwaitEx)
if let Some(count) = self.read_fd_index(fd.0, addr, len)? { ... }
```

The fd value (9) is held in register **r2** (a0), so `fd.0 == 2` and the read
hits emulator-fd **2 = Stderr**. The real client fd is never read. (Writes have
the same bug but it's masked: misrouting stdout→stderr is invisible under the
`2>&1` log redirect.)

**Why this is the ISA-v2 contract, not the caller's fault.** Under the v2
"capabilities are GPR handles" migration, `PULL`/`PUSH`/`AWAIT` take their fd
operand as a **GPR whose value is the fd handle**, not a static fd-register
index:

- LLVM backend: `LNP64InstrInfo.td:750` — `def PULL ... (LNP64pull GPR:$cap, ...)`.
- Canonical v2 ABI asm: `toolchain/liblnp64_min.s` — `read: pull r2, r2, r3, r4`
  (fd value in r2).

The dynamic twins already do the right thing:

```rust
// Instr::PullDyn / PushDyn / AwaitDyn
let fd_value = self.read_reg(fd_reg)?;
let fd = self.decode_fd_value(fd_value)?;
```

## Proposed fix (emulator-only; no Redis rebuild needed)

Make the static `Instr::Pull` (0x2b), `Instr::Push` (0x2c), `Instr::Await`
(0x2e), and `Instr::AwaitEx` (0x71) handlers read the fd **value** from the GPR
named by the operand and `decode_fd_value` it — i.e. give them the same fd
resolution as their `*Dyn` twins. After the FDR→GPR migration the static and
dynamic forms are semantically identical.

**Blast radius / why this wasn't landed here:** ~12 emulator unit tests
construct these with `FdReg(n)` relying on the *old* "n is the fd index"
semantics (e.g. `Instr::Await(Reg(5), FdReg(3), Reg(2))`); they must preload the
operand register with the fd value. The legacy `src/asm.rs` `parse_fd` (`fdN`
syntax) path is also index-based and only used by these tests — real code is
emitted by LLVM/llvm-mc with GPR operands. Because this is core fd-execution
semantics that the **active ISA-v2 migration owns** (continuous commits to
master on a shared worktree), it should be landed there to avoid conflicting
edits, rather than as a side change from the Redis bring-up.

## Repro / verification harness

- `LNP64_BIN=./target/release/lnp64 bash scripts/run_redis.sh` (timeout bumped
  to 1800 s). Always run the emulator **under `ulimit -v`** and detached
  (`setsid`) — a foreground `lnp64 run-elf` shares the shell's process group and
  a host watchdog/earlyoom can take the shell down with it.
- Instrument `read_fd_index` to print `fd`/`len`/handle-kind, set a
  `dbg_post_accept` flag in `object_ctl_socket_accept`, and sample the guest PC
  in `run_committed_exec` to reproduce the diagnosis.
