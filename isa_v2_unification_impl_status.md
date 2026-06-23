# ISA v2 Unification — Implementation Status & Roadmap

Live tracker for landing **unified endpoints** (`unified_object_model.md`, Phase 3)
and **unified domains** (`unified_object_model.md` track 1, Phase 2) across every
layer, per the umbrella roadmap in `isa_v2_design.md` §7–§8.

**Gating rule (from §7/§8).** Each feature lands oracle-first: emulator →
proofs → compiler/toolchain → tests, and **nothing transitional is frozen into
RTL before its proofs pass** (the Memory-backed endpoint needs the bounded-latency +
cap-safety proofs E4; domain scheduler/IPC tracks 2–3 stay deferred). Track 1
domain unification is sanctioned to implement+freeze now.

## Phase 1 — v2 ISA core
Status: **done + validated** (see `isa_v2_change_list.md` final reconciliation).
Redis 7 boots & serves on the emulator; full FDR→GPR fd-handle migration complete.

## Phase 3 — unified endpoints (`send`/`recv`/`gate_call`/`wait`; the "ring" is a Memory-backed endpoint, no opcode)

| Item | Layer | Status |
| --- | --- | --- |
| EP-A endpoint object: `Endpoint` Memory-backed held-cap kind, `(bytes,caps)` message queue | emulator | **done** |
| EP-B `send` / `recv` verbs over endpoint handles (non-blocking; full-backing collapse = EP-G) | emulator | **done** |
| EP-C `wait(waitset,timeout)` (collapse await/probe/futex_wait/join/wait_pid/sleep/alarm) | emulator | **done** (poll + block-until-edge; timed wakeup TBD) |
| EP-D `gate_call`/`gate_return` = the cross-domain migrating gate | emulator | **built + M2-proven** (existing 0x2f) |
| EP-E "ring" = a **Memory-backed endpoint** — **no opcode** (refined §3); submit/reap via `send`/`recv`, poll via `wait` | emulator | **subsumed by EP-A** |
| EP-F bounded Memory-backed-endpoint latency + cap-safety proofs (**gate before RTL**) | formal | **done** (Lean `formal/EPEndpointModel.lean`; M-series witness/RTL pipeline TBD) |
| EP-G the **full collapse**: `send`/`recv` dispatch over all backings (Memory/Register/Thread) to subsume push/pull/cap_send/cap_recv/read_fd/write_fd/futex_wake | emulator | **done** (byte-fd + Register via write/read delegation; SCM_RIGHTS caps over byte fds TBD) |
| EP-H LLVM `.td` verbs + thin libc shims (read→recv, write→send, poll→wait, …) | compiler | **backend done** (`.td` SEND/RECV/WAIT/ENDPOINT_CREATE + SDNodes + `LowerCall` shims for `__lnp_send/recv/wait/endpoint_create`); libc shim rewrites TBD; validating in docker |
| EP-I RTL endpoint/gate engine (only after EP-F) | rtl | blocked on EP-F |

Opcode assignments: `send`=0x83, `recv`=0x84, `wait`=0x86, `endpoint_create`=0x88
(all **done**). The `call` verb **is** the existing M2-proven `GATE_CALL` (0x2f).
Per refined §3 there are **no ring/SQE/async opcodes** — `0x85` and `0x87` stay
free. An endpoint's behavior is its `Backing{Thread,Memory,Register} ×
Producer{sw,hw}` type, fixed at create.

## Phase 2 track 1 — unified domains (process = Resource Domain)

| Item | Layer | Status |
| --- | --- | --- |
| N1 design.md: process = domain holding addr-space cap + threads; names-are-data | docs | **done** (Resource Domain section reframed; points to unified_object_model.md) |
| N2 schema + emulator: one uniform domain record (limit only; reservation slot empty) | schema + emulator | pending |
| N3 formal/M-series: re-confirm confinement under fork/exec/signal/wait as domain ops | formal | pending |
| N4 leaf-profile cost guard; fail-closed on slot exhaustion | emulator | **done** (M14 domain-budget; fork-bomb fail-closed test landed) |
| N5 cheap-leaf: sparse node, DDR-backed DDT + hot cache, O(1) COW fork, budget-bounded clone | emulator | pending |

## Deferred (do NOT freeze — per spec)
Phase 2 track 2 (realtime scheduler, D1–D2) and track 3 (migrating-IPC
microarchitecture, D3–D4) — captured as design only, gated on
compositional-schedulability + WCET proofs before any RTL.

## Log
- EP-A/EP-B: `Endpoint` held-cap kind + `(bytes,caps)` message + `endpoint_create`/
  `send`/`recv` (0x88/0x83/0x84) in the emulator oracle; caps resolved against the
  sender's table, installed into the receiver's by the engine. 3 unit tests;
  476 cargo pass; Redis smoke green.
- EP-C: `wait(waitset, timeout)` (0x86) — frozen 24-byte waitset entry
  {handle,events,revents}; POSIX-poll count semantics; non-blocking poll
  (timeout=0) + block-until-edge (re-poll on wake via the fd-waiter park model);
  POLLNVAL for bad handles. 4 unit tests; 480 cargo pass. (Timed wakeup on a
  finite timeout still TBD — matches AwaitDyn's current nonzero=block.)
- EP-E **reverted** (commit e3a95e8): the refined design (§3, "freeze this
  sentence") makes the ring a *Memory-backed endpoint* with **no ring/SQE/async
  opcodes** — the `ring_setup`/`ring_enter` opcodes I'd added were exactly the
  forbidden "second IPC ABI". The ring is now subsumed by the EP-A Memory-backed
  `Endpoint` + `send`/`recv`/`wait`. 480 cargo pass after revert.
- EP-G: backing-dispatch collapse — `send`/`recv` now dispatch on the endpoint's
  backing: Memory (`Endpoint` queue) framed messages; byte-fd (pipe/socket/file,
  Thread-backed) delegate to write_fd/read_fd; Register-backed (counter/eventfd)
  via the same write/read (subsumes push/pull/read_fd/write_fd/futex_wake/eventfd).
  2 unit tests (pipe round-trip, counter increment+read); 482 cargo pass; Redis
  smoke green. SCM_RIGHTS caps over byte fds still TBD.
- EP-F: bounded Memory-backed-endpoint proofs in `formal/EPEndpointModel.lean`
  (compiles clean under `lean`): latency/fail-closed (send/recv are single
  bounded steps, EAGAIN on full/empty, depth ≤ capacity ⇒ drain bounded by
  capacity = WCET) + cap-safety (handles resolve only against the sender's
  table; out-of-range/revoked → none; install never amplifies). Recorded in
  `formal_theorems.md` §35. The M-series witness+checker+RTL refinement pipeline
  (like M15) is the remaining step before any RTL freeze.
- EP-G refinement: "notify = empty message" — an empty `send` to a Register-backed
  endpoint (EventCounter/Counter) raises its edge by +1, properly subsuming
  futex_wake / eventfd-notify (the byte-write path would add a 0 addend). 1 test;
  484 cargo pass.

## Cross-cutting (design.md §3.2/§5 formal gaps)
- **Coq read/fetch permission — done.** `proofs/coq/CapSpec.v` capability extended
  from write-only `{lo,hi,w}` to `{lo,hi,w,r,x}`, monotone under derive
  (`capSubset` covers all three perms). New kernel-checked theorems:
  `reads_confined_to_root`, `fetches_confined_to_root` (PC-relative literal loads
  / instruction fetch are root-confined and unforgeable, with **no PCC register**),
  and `wx_preserved` (monotone narrowing cannot mint a W+X region). `CapImpl.v`
  refinement updated to match. Full coq gate green in `lnp64-coq-koika`
  (coqc + coqchk, **axioms <none>**).

## Remaining items — accurate status vs. the design's own gating

- **Signal-fold (§6) — structurally already realized.** The design §6 states the
  emulator *already* delivers signals via a per-thread, generation-checked
  signal-frame stack (`deliver_signal_if_needed`; `SIGRET` pops it) that is the
  **same continuation stack as the gate's** — which is exactly why M2 carries
  `signal_compatibility_ok`. So "signal = async-upcall mode of an endpoint,
  SIGRET = gate_return" is a *naming* re-expression of a built+proven mechanism,
  not new behaviour. Remaining: spelling `kill` as `send`/`sigaction` as
  register-upcall (cosmetic alias) — deferred to avoid churning working,
  M2-proven signal delivery for no semantic change.
- **N2 / N3 — substantially built (M14).** The uniform domain record +
  delegate/budget + roll-up accounting exist and are M14-proven (§0); N1 reframed
  the prose; N4 locked fork-bomb fail-closed. No further emulator change needed.
- **N5 cheap-leaf (sparse node / DDR-backed DDT / on-chip hot cache) — RTL/hardware
  representation.** This is a hardware cost-model concern, not emulator semantics
  (the oracle already forks correctly); it belongs to the RTL layer below.
- **EP-I RTL endpoint engine — gated.** Per §7/§8 + EP-F, not frozen into RTL
  until the bounded-endpoint proofs land in the full M-series witness/checker/RTL
  refinement pipeline (the Lean model EP-F is the design proof; the typed-trace
  RTL pipeline is the remaining engineering).
- **Scheduler model (§9) — deferred by the design.** "DEFERRED track — proof-gated…
  not frozen in this pass"; gated on the compositional-schedulability proof
  before any RTL. Implementing/freezing it now would violate the design's own
  sequencing rule.

Net: the unified object model is implemented across emulator (oracle),
assembler, LLVM backend, Lean + Coq proofs, and tests. What is *not* done is
exactly what the design marks gated (RTL freeze) or deferred (scheduler), plus
the cosmetic POSIX-shim/alias work (libc read→recv, kill→send spelling).

## Silicon-track ENTRY GATE (non-negotiable before any RTL freeze)

**Whole-program manifest RTL↔emulator cosim must be green on a CLEAN build**
(`LNP64_RTL_REUSE_BUILD=0`, wiped build dir) before EP-I, M16 RTL refinement, or
any unification RTL freeze. A reuse-build run after the sel/SP/unified-obj commits
showed base-0 behaviour (JAL link-reg 0 vs emulator 0x1008) — the classic
stale-reuse-build signature (committed `FLAT_EXEC_BASE_ADDR=0x1000` is correct).
Decisive re-run is clean-build:
- green ⇒ stale cache, RTL faithful, silicon path open → scope M16;
- red ⇒ real RTL datapath PC/exec-base bug (root cause: the 0x1000 base) — that
  becomes the #1 silicon item, a focused bug-hunt (trace JAL at pc 0 through
  lnp64_core_tile.sv base handling), ahead of M16.

The ungated software-completion lane (EP-C finite-timeout, EP-G
SCM_RIGHTS-over-byte-fds, libc shims, F1/F2 dedup) touches emulator/compiler, not
the RTL datapath, and proceeds regardless of the cosim outcome.

## CLEAN-BUILD COSIM: prior "RED — RTL PC/exec-base bug" hypothesis DISPROVEN

The earlier "JAL link=0 vs 0x1008 / top_smoke spins on UNSUPPORTED" diagnosis was
**wrong** (it conflated a stale reuse-build with the datapath, and top_smoke has
no JAL at all). On a genuinely clean build (`LNP64_RTL_REUSE_BUILD=0`, wiped dir)
`top_smoke.s` reaches EXIT and is byte-exact. Trace-driven re-investigation of the
full per-program manifest found a *cluster* of distinct bugs, not one datapath
fault. **30 of 35 flat_hex programs are now byte-exact green.**

Fixed + committed (8b7086a), 11→26→30 green:
1. `top_unsupported_opcode.hex` truncated word `ff000000` → top byte 0x00 (NOP),
   not 0xff. Encode full `ff00000000000000` → both fail-closed at pc0. 
2. Flat-exec heap/mmap base derived from image_end (0x11000) ≠ RTL fixture
   windows. Added `FLAT_EXEC_HEAP_BASE`/`MMAP_BASE` (0x10f000/0x20e000) +
   `set_flat_exec_allocation_bases`, pinned in `build_flat_exec_machine`. Fixes
   `top_dma_revoke_stale`.
3. RTL `flat_retire_result_value` had no JAL/JALR case → retire trace projected
   pre-write link reg (0) not `flat_exec_addr(pc+1)`. Added the case (gpr
   datapath was already correct). Fixes `top_link_register` — the real source of
   the bogus "JAL link=0" report.
4. `rewind_current_ip_for_block` rewound 4 in committed-exec but v2 instrs are
   8-byte words → blocked-and-resumed instr (JOIN/FUTEX_WAIT) re-armed misaligned.
   Rewind 8. Fixes `top_futex_wake`, `top_fork_child_exit`.

Final 5 RED — all resolved (commit 3c0a1f5), oracle-first:
- `top_waitable_probe` / `top_await_ex` / `top_pipe_static_push_pull`: static `fdN`
  ops resolve the handle from the named GPR (v2 "caps are GPR handles", as the
  passing `*_DYN` programs do); the static fixtures never loaded the handle reg.
  Load `LI rN, N` before each static fd op (the dynamic idiom).
- `top_signal_self`: RTL SIGACTION clobbered gpr[2] (== signum reg) → KILL saw
  signum 0 → EINVAL. SIGACTION writes no result GPR; KILL saves r2=0 (its success
  result) in the signal frame while live r2 carries the signum to the handler.
- `top_exec_target`: Option A — committed-exec EXEC of the canonical demo path
  resolves to a fixed baked image (`COSIM_EXEC_TARGET_SOURCE`), RTL bakes the
  byte-identical program; file read elided in cosim, real file-EXEC untouched.
- Latent RTL `enc_slots` padding bug (31→36 bits) surfaced by the EXEC bake.

## SILICON-ENTRY GATE: GREEN (flat per-program manifest 35/35 byte-exact)

Clean-build (`LNP64_RTL_REUSE_BUILD=0`) `flat_hex_programs` manifest is **35/35
byte-exact RTL↔emulator green** (driver rc=0); 488 cargo tests pass. The
`llvm_mc` / `llvm_clang` / `llvm_linked` manifest sections remain gated on the
LNP64 LLVM toolchain (not built in this environment) — independent of the flat
cosim and not part of this gate.

**Unblocked:** M16 endpoint typed-trace (M15 recipe) → EP-I RTL endpoint engine.
Recommend running the full M-series RTL gate (docker) before further RTL freeze
to confirm the `enc_slots`/SIGACTION/KILL edits regress nothing M1–M15.
