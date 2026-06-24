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
| EP-F bounded Memory-backed-endpoint latency + cap-safety proofs (**gate before RTL**) | formal | **done + promoted to M16** (Lean `formal/EPEndpointModel.lean` → full M16 witness/refinement pipeline: schema→pkg→RTL→checker→witness→Lean, all green) |
| EP-G the **full collapse**: `send`/`recv` dispatch over all backings (Memory/Register/Thread) to subsume push/pull/cap_send/cap_recv/read_fd/write_fd/futex_wake | emulator | **done** (byte-fd + Register via write/read delegation; **SCM_RIGHTS caps over byte-fds landed** — `send`/`recv` carry caps over pipe/socket via the channel cap-FIFO, subsuming cap_send/cap_recv; commit 9cee1c0) |
| EP-H LLVM `.td` verbs + thin libc shims (read→recv, write→send, poll→wait, …) | compiler | **done** (`.td` backend + libc shims rewritten: read→`__lnp_recv`, write→`__lnp_send`, poll/epoll_wait→`__lnp_wait`, purely additive; commit 2110c4e. **Redis rebuilt on the verb-routed sysroot runs end-to-end on the verbs** — full smoke PASSED) |
| EP-I RTL endpoint/gate engine (only after EP-F) | rtl | **sanctioned to freeze** (M16 witness+Lean green; full M1–M16 gate green) |

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

## M16 Step 0 — M1–M15 regression gate: GREEN

All M1–M15 RTL witness/refinement gates pass (`run_rtl_m{1..15}_witness_gate.sh`
/ `run_rtl_m1_refinement_gate.sh`, `LNP64_RTL_FAST=1`). The session's silicon
edits (`enc_slots` padding, SIGACTION/KILL signal delivery) regress nothing —
confirmed structurally too: no `m{N}_filelist.f` compiles `lnp64_core_tile.sv`
(the M-series tbs exercise standalone engine modules), so those edits cannot
affect M1–M15.

Pre-existing breakage fixed en route (commit 65fca5f, from the sel.<cc> work
e1d82e5/ec11a84, not this session): `check_rtl_shared_schema.py` now strips `//`
comments in enum bodies, and the schema `lnp64_decode_t` gained `rs4/rs5` to
match the pkg. (The "PermissionError" failures on first run were just a
root-owned `build/` dir from prior docker runs; chowned, all green on re-run.)

**Flagged, out of scope (pre-existing, NOT M-series, NOT this session):** the
broader `run_rtl_proof_gates.sh` suite is red on `check_rtl_s0_contract.py` —
S0 still expects legacy `LNP64_OP_LI32` (+ a stale `lnp64_decode_t` shape) that
the ISA-v2 decode migration (9fca938) removed. Needs an ISA-contract decision;
independent of M16. The M1–M15 *witness/refinement* gates (Step 0's scope) are
all green.

## M16 endpoint typed-trace engine — COMPLETE; EP-I sanctioned to freeze

EP-F promoted from a standalone Lean file to the full M-series pipeline (M15
recipe), all green:
- **schema** (`lnp64_m16_endpoint_commit_t` / `lnp64_m16_state_projection_t`,
  `lnp64_m16_endpoint_op_e` / `lnp64_m16_backing_e`, `EMSGSIZE=90`); pkg+schema
  in lockstep (`check_rtl_shared_schema` green).
- **RTL** `rtl/engines/lnp64_m16_endpoint.sv` (+ assertions, tb, filelist):
  queue engine walking create/send/recv/full/empty/oversize/cap-send/cap-reject/
  notify, emitting a typed commit + invariant projection per op
  (`LNP64-RTL-M16 PASS`).
- **checker** `check_rtl_m16_typed_commit_trace.py` (+ offline `check_rtl_m16_witness.py`,
  self-tests) validating the four EP-F invariant classes; emits
  `lnp64_m16_endpoint_refinement_witness_v1` (13 records).
- **witness gate** `run_rtl_m16_witness_gate.sh` + seeded RTL↔model cosim
  (`formal/m16_endpoint_model.py`).
- **Lean** `formal/M16EndpointModel.lean` (promotes EP-F: generic
  bounded/fail-closed/cap-safety/framing theorems + packed-decode machinery) and
  `run_rtl_m16_lean_witness_gate.sh` (kernel-`decide` decode faithfulness).
  `#print axioms` = propext/Quot.sound only (no sorry/admit/custom axiom).

Acceptance met: M16 witness + Lean green; full **M1–M16** RTL witness/refinement
gates green; cargo green; Redis unaffected (no Rust change). → **EP-I (RTL
endpoint engine) is sanctioned to freeze.** Note: `EPEndpointModel.lean` is kept
as the EP-F design proof; `M16EndpointModel.lean` is its witness/refinement
promotion (same theorems, M16 namespace + packed-bit layout).

## Software collapse — EP-G + EP-H DONE; F1/F2 unblocked

The four verbs now subsume the legacy IPC/cap ops in the emulator **and**
software runs on them:
- **EP-G** (commit 9cee1c0): `send`/`recv` carry SCM_RIGHTS caps over
  Thread-backed byte-fds (pipe/socket) via the channel capability FIFO —
  resolve-against-sender, install-no-amplify, fail-closed — subsuming
  cap_send/cap_recv. Test + cargo 489 + cosim 35/35 + Redis green.
- **EP-H** (commit 2110c4e): libc `read`→`__lnp_recv`, `write`→`__lnp_send`,
  `poll`/`epoll_wait`→`__lnp_wait`, **purely additive** (legacy pull/push/await
  opcodes + handlers stay live). Fast equivalence gate green (write/read run-elf
  exit=0; poll byte-identical old-vs-new), cosim 35/35, cargo 489. Redis
  rebuilt on the verb-routed sysroot **runs end-to-end on the verbs** (full
  smoke PASSED — PING/SET/GET/DEL/INCR/RPUSH/LRANGE/HSET/SADD/SISMEMBER/SMEMBERS/
  KEYS).

**F1/F2 (the ISA collapse) is now unblocked**: the verbs subsume everything and
software (Redis) already runs on them, so the legacy `_dyn` twins
(0x3b/0x3c/0x70/0x72), `call_cap` (0x4e→gate_call), and the legacy
push/pull/read_fd/write_fd/cap_send/cap_recv/await*/waitable_probe* paths can be
removed — each removal gated on Redis green + cosim byte-exact + M1–M16 green.

## Collapse sequencing (HISTORICAL — superseded; see "Resolved decisions", "Tail sequencing", and "EP-I-full plan" below for the CURRENT plan)

> This section records the original EP-I-lite-era plan and is kept for the journey.
> The live plan is: EP-I-lite (done) → F1-step-1/2 (done) → F2 (done) → **EP-I-full
> a/b/c/d (in progress)** → byte-fd mop-up (push/pull, read_fd/write_fd; needs the
> libc→verb migration + Redis rebuild). No register-form slot (Resolved decision
> #4). The "EP-I-full (LAST)" / "F2 → step-3 → EP-I-full" lines below are stale.

Discovery during F1: the RTL has **no verb execution** (no SEND/RECV/WAIT in
pkg/decode/core-tile), so cosim programs can't migrate to the verbs until the RTL
runs them. Corrected order (additive → subtractive → heavyweight freeze):

1. **EP-I-lite (NEXT, one gated commit)** — RTL verb decode + backing-dispatch,
   **byte-fd arm only**. Add `LNP64_OP_SEND=0x83 / RECV=0x84 / WAIT=0x86` to the
   pkg enum, RTL decode, core-tile exec, and main.rs flat enc (verbs aren't
   flat-encodable today). **Structural correctness rule:** operand-sourcing is
   the *only* fork — static arms source fd/buf/len from dec.rd/regs; verb arms
   source ep=gpr[rs1], buf/len from the msg descriptor (gpr[rs2]+offsets) — and
   **both feed one shared pipe/SRAM/waitable store-load datapath** (a small
   operand mux, not a parallel reimplementation; the RTL analog of the shared
   write_fd_index helper). Behavior identical to read_fd/write_fd ⇒ stays
   cosim-byte-exact. No M16 typed-trace freeze needed. Validate with a
   verb-over-pipe smoke that **becomes** F1-step-2's migrated top_pipe_push_pull
   (one test, two jobs). Start from clean build + fresh server. Gate: clean-build
   cosim 35/35 + M1–M16 + Redis.
2. **F1/F2 collapse (resume)** — 0x3b/0x3c: migrate top_pipe_push_pull → verbs
   (the EP-I-lite validation test) + cargo tests (end-to-end → verbs;
   helper-semantics → call read_fd_index/write_fd_index directly), then free
   0x3b/0x3c. Then 0x4e (call_cap→gate_call). Then the step-3 legacy sweep
   (read_fd/write_fd/push/pull/cap_send/cap_recv/await*/waitable_probe* opcodes
   removed; *_index helpers kept private — RTL byte-fd now runs via the verb
   delegate). Each its own gated commit (Redis + cosim 35/35 + M1–M16).
   Forward note: when read_fd/write_fd come out, grep toolchain/ + demos/ for any
   static-mnemonic emitters (they'd break at link/run, not decode).
3. **EP-I-full (LAST)** — M16 endpoint-engine freeze (the Memory-backed arm of
   the same dispatch) against the final collapsed ISA, gated on M16 typed-trace +
   cosim.

F1 status: step 1 done (0x70/0x72 freed, bcc16a0); step 2 done (0x3b/0x3c freed).
F2 done (0x4e). EP-I-full-a done (wait verb in RTL). **Current next action:
EP-I-full-b** (retire await_ex/waitable_probe; see the EP-I-full plan section).

## B1 — ISA collapse burndown (the "fewer instructions" goal, measured)

Two metrics, recorded as each F1/F2 step lands. The shrink is in the **opcode
surface** (fewer distinct opcodes to implement + verify in silicon); per-call
**code size** can rise slightly because a verb carries its operands in an
in-memory msg descriptor instead of packed register fields — that setup cost is
built once per call site and amortizes across repeated transfers (e.g. a libc
read/write shim or a loop).

Opcode-surface metric (opcodes freed = removed from every decoder/encoder, value
reusable):

| step      | opcodes freed            | mnemonics retired                         | live RTL decode opcodes |
|-----------|--------------------------|-------------------------------------------|-------------------------|
| F1-step-1 | 0x70, 0x72               | POLL_FD_DYN, AWAIT_EX_DYN, WAITABLE_PROBE_DYN | (—)                 |
| F1-step-2 | 0x3b, 0x3c               | READ_FD_DYN, WRITE_FD_DYN, PULL_DYN, PUSH_DYN  | 125                 |
| F2        | 0x4e (CallCapDyn dup)    | (none — CallCapDyn had no asm mnemonic)   | 124                     |
| EP-I-full-a | (adds wait 0x86)       | (none — adds the unifying verb)           | 125 (+1, enabling)      |
| EP-I-full-b1 | 0x6f (waitable_probe) | WAITABLE_PROBE, POLL_FD                    | 124                     |
| EP-I-full-b2 | 0x71 (await_ex)       | AWAIT_EX                                   | 123                     |
| (remaining) | 0x51/0x52 cap_send/cap_recv (c); then the libc-batch (0x2b/0x2c/0x2d/0x57/0x2e + Redis rebuild) | … | → ~118 and down |

Running total freed after F2: **5 opcodes** (0x3b, 0x3c, 0x4e, 0x70, 0x72). F2 was
a pure dead-surface removal: CallCapDyn (0x4e) was a literal duplicate of
gate_call (0x2f/CallCap) with no asm mnemonic, no codegen, no test, no demo —
only the enum/encoder/decoder/handler existed. Zero call-site changes, zero
corpus cost (confirming "the code-size cost is already fully paid").
The D2 guard `scripts/check_retired_mnemonics.sh` enforces that no source emits a
retired mnemonic (decode removal alone breaks only at assemble/link/run time).

Corpus metric (instruction words, before = legacy form, after = verbs):

| corpus                        | before  | after   | Δ     | %       | note                                                  |
|-------------------------------|--------:|--------:|------:|--------:|-------------------------------------------------------|
| top_pipe_push_pull (1 send+1 recv) | 32 | 44 | +12 | +37%   | microbenchmark: ~12-word descriptor build, 0 amortization |
| Redis 7.0.15 + minilibc       | 351,210 | 351,328 | +118  | +0.034% | the real verdict — flat                               |

The Redis number is the one that answers "does this make real code longer or
shorter?" — and the answer is **flat (+0.034%)**. It's the *entire* corpus delta,
not a sample: the verb migration (EP-H, commit 2110c4e) changed only the libc
shim functions, and Redis's hundreds of read/write/poll call sites are unchanged
`call` instructions. The descriptor/waitset build is *inside* the shim (once per
function — read/write/poll/epoll/select/kqueue), so it is loop-invariant in the
hot paths and does not scale with call-site count. Measured by recompiling the
EP-H-parent shim sources vs current with the production flags:
`liblnp64_fd_min.o` 506→518 (+12), `liblnp64_poll_min.o` 1733→1839 (+106).

The microbenchmark's +37% is a fixed descriptor-build cost with nothing to
amortize against in a 32-word program that does one transfer; it is *not*
representative of real code, as the Redis row shows. The code-size cost of the
unified model is therefore already fully paid (it lived in this read/write→verb
migration); the remaining collapse steps (F2 0x4e dup-removal, the step-3 sweep
of already-migrated opcodes) are pure opcode-surface reduction with **zero**
further code-size cost. Reserve lever, deploy only if a future corpus row regresses
materially: a register-form fast path for the small-message case (fd+ptr+len in
registers) alongside the memory descriptor for the general case — recovers compact
encoding at the cost of re-introducing one encoding form. Not justified by the data
today.

## F1-step-2: DONE (0x3b/0x3c retired; byte-fd transfer is recv/send)

- **RTL:** removed `0x3b/0x3c` decode arms (`lnp64_decode.sv`) and the now-dead
  `raw_opcode == 8'h3b` dynamic-pull branches in `lnp64_core_tile.sv` (errno,
  result_value, sequential). `pipe_fd` etc. stay live via the static `0x2b` PULL.
- **emulator:** removed the `0x3b/0x3c` flat-decode arms and the
  PullDyn/PushDyn/ReadFdDyn/WriteFdDyn handlers. The private helpers
  `read_fd_index` / `write_fd_index` / `write_fd_index_to` stay — the verb byte
  path and the static fd ops call them (removing the opcode ≠ removing behavior).
  Inbox/MESSAGE_ENDPOINT_FD reads are preserved by the static `Pull` handler.
- **isa.rs:** removed the `ReadFdDyn`/`WriteFdDyn`/`PullDyn`/`PushDyn` variants
  (Rust's exhaustive match confirmed every site was updated).
- **toolchain:** removed `READ_FD_DYN`/`WRITE_FD_DYN` asm mnemonics + the
  `main.rs` flat-enc arms.
- **tests:** the ~11 capability cargo tests that read/wrote an fd via ReadFdDyn/
  WriteFdDyn now go through `recv`/`send` (new `exec_recv_fd`/`exec_send_fd` test
  helpers build a one-shot descriptor; result in r2) — verified byte-identical
  errno/r2 (stale→116, EPERM→1, success→count) since recv resolves the handle via
  `decode_fd_value` then `read_fd_index`, the same path. asm + main.rs encoding
  tests re-pointed to the verbs.
- **programs:** `top_pipe_push_pull.s` migrated in place to send/recv (the
  EP-I-lite `top_pipe_verb_push_pull.s` was folded into it — one test, two jobs);
  `demos/stale_fd_token.s` migrated to recv.
- **D2 guard:** `scripts/check_retired_mnemonics.sh` added and green.
- **Gate:** clean-build `flat_hex_programs` cosim 35/35 byte-exact; cargo 489;
  Redis green; M1–M16 unaffected (engine filelists exclude core_tile/decode).

## F2: DONE (0x4e CallCapDyn retired — pure dead-surface dup of gate_call)

`CallCapDyn` (0x4e) was a literal duplicate of `gate_call` (0x2f/`CallCap`) — same
`self.call_cap` handler, differing only in fd-operand sourcing — but with **no asm
mnemonic, no codegen/lowering, no cargo test, no demo**. Only the enum variant,
encoder, decoder, and handler existed. Removed all four + the smoke decode-map
entry. Zero call-site changes, zero corpus cost (confirming the code-size cost was
already fully paid in F1-step-2). Live RTL decode opcodes 125 → **124**. Gate:
clean-build cosim 35/35 byte-exact; cargo 489; Redis green; M1–M16 unaffected.

## step-3 sweep — FEASIBILITY FINDING: the remaining opcodes are LIVE, not dead

Greenlit as "pure surface reduction (already-migrated opcodes, dead branches)",
but measurement says otherwise — only the *_dyn twins (F1) and CallCapDyn (F2)
were dead; the legacy *static* opcodes are still emitted. Evidence (compiled
redis-server.elf + libc .c + cosim/demo .s):

| group                    | still emitted by                                              | blocker to removal                                  |
|--------------------------|--------------------------------------------------------------|-----------------------------------------------------|
| push / pull (0x2b/0x2c)  | libc meta/socket/time `__lnp_push`/`__lnp_pull`; LLVM ISel; Redis (2+2) | migrate those libc fns to send/recv + rebuild Redis |
| read_fd / write_fd (0x2d/0x57) | ~20 demos (`WRITE_FD fd1` = the console-write idiom), top_waitable_probe/top_await_ex (READ_FD), 3 cargo tests | migrate emitters; keep microcode (verbs reuse it) |
| cap_send / cap_recv (0x51/0x52) | 4 gated cosim programs, cargo cap tests, demos      | RTL verb **cap-FIFO path** (today emulator-only, EP-G) |
| await / await_ex / waitable_probe (0x2e/0x71/0x6f) | libc poll_min; Redis (5 await); 2 gated cosim programs | RTL **wait verb** (not in RTL — EP-I-lite deferred it) |

Consequence: the sweep **cannot precede EP-I-full** for caps/await/waitable — they
need RTL verb cap + wait support, which is EP-I-full. push/pull need a libc→verb
migration (+ Redis rebuild) first. read_fd/write_fd are the only group with no
compiler emitter, but freeing them still means migrating ~20 demos' console-write
idiom. So "F2 → step-3 → EP-I-full" is not executable as a quick sweep.

**Recommended re-sequencing:** EP-I-full (extend the verb_form mux with the wait
verb + the cap-FIFO path in RTL) **before** the bulk of step-3 — it is the
prerequisite that unblocks 3 of the 4 groups and is needed for the freeze anyway.
Then step-3 becomes mechanical opcode removal. The byte-fd-only groups
(push/pull via libc migration, read_fd/write_fd via demo migration) can be done
independently whenever, since the RTL verb byte path already covers them.

## Resolved decisions (locked — do not re-litigate)

1. **Operand-sourcing is the only fork.** The unified verbs reuse existing
   microcode (send→WRITE_FD, recv→READ_FD) with a `verb_form` operand mux; no new
   execute arms, no new opcodes for the byte-fd path. (EP-I-lite.)
2. **Verb result ABI = r2.** Byte-fd verbs return the transfer count in r2 (the v2
   return-value reg), matching the static fd ABI so the RTL sequential write and
   the retire trace agree. (EP-I-lite.)
3. **caps + wait/endpoint deferred to EP-I-full.** `wait` (waitset) and cap-passing
   are not pure operand-muxes of an existing arm, so they land with the M16
   endpoint engine — which is also when the cap_send/cap_recv/await/await_ex/
   waitable_probe opcodes retire. (EP-I-lite + step-3 feasibility finding.)
4. **Register-form fast path REJECTED — trades unification for density.** A 1-bit
   `form` field on send/recv (reg form `rd/rs1=fd/rs2=ptr/rs3=len` vs the memory
   descriptor) was specced to recover descriptor-build code size, then rejected:
   the B1 corpus measurement (Redis+libc verb migration **+0.034%, flat**) shows
   there is no code-size problem to fix. The form bit would re-introduce an
   encoding form (surface↑) to win back instructions the corpus shows we never
   lost. Keep send/recv single-form (memory descriptor). Reserve only if a future
   corpus row regresses materially — today's data says it won't. Do not re-propose.

## Tail sequencing: step-3 legacy sweep → EP-I-full freeze (one form)

The yardstick is **# unique opcodes/types**; every opcode the sweep retires is a
direct hit on it. Order: step-3 sweep → EP-I-full freeze — no register-form slot
(see Resolved decision #4). Per the feasibility finding above, the caps/await/
waitable opcodes retire *as part of* EP-I-full (that step adds the RTL wait +
cap-FIFO verb paths they need); the byte-fd groups (push/pull, read_fd/write_fd)
retire via emitter migration, independent of the freeze.

## EP-I-full plan (NEXT — decision A: freeze + biggest yardstick win together)

Adds the RTL `wait` + cap-FIFO verb paths and retires the 5 opcodes they subsume
(cap_send/cap_recv/await/await_ex/waitable_probe), then freezes M16. Guardrails:
freeze against the single descriptor form (Resolved decision #4); D2 guard + B1 row
per retired opcode (live decode opcodes 124 → ~119); confirm the later byte-fd
mop-up is dead-branch-only (no encoding change) so "M16 unaffected" holds.

**Key tractability insight:** every gated cosim program touches a *single* fd/cap
(`top_waitable_probe`/`top_await_ex` probe one fd; `top_cap_*` move one cap), so
the RTL needs only **single-entry waitset / single-cap** paths — reuse the
existing `await_fd_ready` readiness signals + add a revents memory writeback. The
general multi-entry waitset / multi-cap case stays **M16-engine-modeled** (the
emulator already handles it); document the RTL limitation. This mirrors EP-I-lite's
incremental scoping.

Sub-steps (each a gated commit — tasks #16–#19):
- **a. RTL wait verb (0x86). DONE.** New `LNP64_OP_WAIT` microcode (pkg+schema
  in lockstep); decode 0x86→WAIT; execute: waitset double-indirection
  (`gpr[rs1]`=waitset → `entries_ptr`[0]/`count`[8] → entry{handle@0, events@8,
  revents@16}), `fd=fdr_value_fd(handle)`, reuse `await_fd` readiness, **store
  revents to entry[16]**, result = ready count → `gpr[rd]`. The chained
  combinational SRAM reads (waitset → entries_ptr → entry fields) worked directly;
  POSIX revents semantics mirror `poll_fd_index_mask_raw` (POLLNVAL=32 for
  bad/closed/no-POLL-right; POLLIN=1 iff requested & read-ready). Single-entry,
  non-blocking (timeout=0); multi-entry/blocking stays M16-engine-modeled.
  Validated by `top_wait_poll.s` (poll a ready event-counter via a 1-entry
  waitset). Adds one opcode (live decode 124→125); the net reduction lands in
  b/c. Gate: flat_hex cosim 36/36 byte-exact; cargo 489; Redis green; M1+M16 green.
- **b. Retire await_ex/waitable_probe (0x71/0x6f) — −2 (125→123).** These have
  **no compiler emitter** (0 in Redis, no libc), so they free cleanly now: migrate
  top_waitable_probe/top_await_ex → wait + the cargo tests; free the opcodes +
  Instr variants (WaitableProbe/AwaitEx) + asm + decode + microcode. D2 + B1.
  **`await` (0x2e) is NOT freed here** — it is still compiler-emitted (5 in Redis,
  from `poll_min.c`'s `__lnp_await` fallback), so it joins the libc→verb migration
  + Redis-rebuild batch (with push/pull/read_fd/write_fd).
- **c. RTL cap-FIFO verb send/recv + retire cap_send/cap_recv (0x51/0x52) —
  STOP-AND-FLAG: frozen-contract fork (M1 capability refinement).** The emulator
  side is mechanical (cap_send ≡ send-with-caps: `cap_send_inner` pushes the src
  cap's `CapabilityPayload` to the channel cap FIFO, exactly like
  `ep_send_bytefd_caps`). But the RTL cap path is **M1-refinement-coupled** and the
  M1 *contract* hard-pins capability coverage to the CAP_SEND/CAP_RECV **arch
  opcodes**:
  - `check_rtl_top_level_program_manifest.py` `require(arch_opcode in top_text)` +
    `require(arch_opcode in smoke_text)` for `LNP64_OP_CAP_SEND/CAP_RECV`
    (M1_TOP_LEVEL_COVERED_KEYS), and the manifest `covered_real_instruction_ops`
    entries pin `{arch_opcode, commit_op, lean_step=capSend/capRecv}`.
  - the Lean model (`M1TransitionInvariantModel`) has `capSend`/`capRecv`
    transition steps; the witness/Lean gates prove cap commits decode under them.
  The M1 *proof* itself is commit-based (`LNP64_M1_COMMIT_CAP_SEND/RECV`, opcode-
  agnostic), so a verb that emits the same cap commit keeps M1 green — but retiring
  the arch opcodes breaks the contract's required opcode↔commit↔lean coupling.
  **Decision needed (frozen contract):** how does the M1 capability refinement
  re-couple when cap transfer becomes send/recv-with-caps? Options: (a) re-point
  the whole coupling (RTL verb emits the cap commits; manifest covered-ops + checker
  + Lean steps reference the verb path) — preserves the proof, re-architects the
  coupling; (b) keep cap_send/cap_recv as M1-proven RTL micro-primitives the verb
  cap-path delegates to (frees the ISA *mnemonic* surface, keeps the binary opcode +
  M1 coupling — "removing the opcode ≠ removing the behavior"); (c) defer cap
  retirement, leaving cap_send/cap_recv as the M1-anchored cap primitives. Until
  this is decided, **0x51/0x52 stay** and the byte-fd libc-batch (push/pull/read_fd/
  write_fd/await 0x2e) proceeds independently.
- **d. Freeze M16** against the final descriptor encoding. **Must prove + freeze
  the multi-entry + blocking wait** (per-entry revents writeback over N entries,
  parking/wake) in the engine — single-entry-non-blocking RTL is the fast path,
  NOT the ceiling; real poll/select-over-many-fds-with-block is the common case and
  must be proven-and-frozen, not deferred indefinitely. Confirm M16 covers both
  single-entry (RTL) and general (engine) transitions. Mark EP-I frozen.

**Open rung — A1 finite-timeout wakeup (track, don't drop):** the executing `wait`
(emulator + RTL) treats a nonzero finite timeout as block-until-edge (no timed
wakeup); the RTL cosim path is non-blocking (timeout=0). Real poll/select/
nanosleep finite timeouts need a genuine timed unblock returning revents=0 on
expiry. Covered by the M16 blocking model (d) for the engine; the emulator timed
wakeup is the A1 backlog item — keep it explicit, not silently absent behind a
green gate.

## EP-I-lite: DONE (byte-fd send/recv on the shared fd datapath)

Landed as one gated commit. The byte-fd IPC verbs now execute in the RTL:

- **No new pkg/schema opcodes.** Realizing the "operand-sourcing is the only
  fork" rule *means* reusing the existing microcode: `lnp64_decode.sv` maps raw
  `0x83→LNP64_OP_WRITE_FD` (send) and `0x84→LNP64_OP_READ_FD` (recv). Adding new
  opcodes would have forced new execute arms = a parallel datapath, which is
  exactly what the rule forbids.
- **The fork is three signals.** `lnp64_core_tile.sv` computes `verb_form` from
  the raw opcode and muxes `static_fd` (verb: `fdr_value_fd(gpr[rs1])`),
  `fd_buf_addr` (verb: descriptor `[0]=bytes_ptr`), and `fd_len` (verb:
  descriptor `[8]=bytes_len`). Every downstream consumer (errno, result_value,
  the sequential pipe/SRAM/event-counter/timer store-load) reads those three
  signals, so the datapath is shared verbatim with read_fd/write_fd.
- **Result ABI.** Byte-fd ops return the transfer count in r2 (the existing v2
  return-reg convention); RTL-targeted verb programs use `rd=2` so the trace
  result_reg (`flat_retire_result_reg(raw,rd)=rd`) and the sequential `gpr[2]`
  write agree, byte-exact with the emulator's `result=rd` write.
- **toolchain:** `main.rs` flat-enc arms for Send/Recv/Wait (`enc_rrr 0x83/0x84/
  0x86`); asm + emulator already supported the verbs.
- **Validation:** `tests/rtl/programs/top_pipe_verb_push_pull.s` (new, in
  `flat_hex_programs`) — send a byte over a pipe writer fd, recv it from the
  reader fd, both via descriptors. Byte-exact. This is the program F1-step-2
  migrates `top_pipe_push_pull` onto (one test, two jobs).
- **Gate:** clean-build `flat_hex_programs` cosim **36/36 byte-exact** (was 35),
  cargo **489**, Redis green (emulator path untouched). M1–M16 unaffected by
  construction: their engine filelists contain no core_tile/decode; the M1
  top-level refinement runs through the cosim and stayed green.

**Scope note — WAIT/ENDPOINT_CREATE deferred to EP-I-full (deliberate).** `wait`
(0x86) is *not* a pure operand-mux of `waitable_probe`: it iterates a waitset
descriptor `{entries_ptr,count}` and writes `revents` back into each entry — a
distinct side-effect shape. Forcing it through waitable_probe would violate the
shared-datapath invariant, so it lands with the M16 endpoint engine (which models
waitsets natively). `endpoint_create` (0x88) likewise belongs with the
Memory-backed arm. The byte-fd smoke needs neither; `main.rs` still encodes
`wait` (0x86) for the emulator/libc path (Redis), it just has no RTL decode yet.
