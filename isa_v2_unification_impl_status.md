# ISA v2 Unification — Implementation Status & Roadmap

Live tracker for landing **unified endpoints** (`unified_object_model.md`, Phase 3)
and **unified domains** (`unified_object_model.md` track 1, Phase 2) across every
layer, per the umbrella roadmap in `isa_v2_design.md` §7–§8.

**Gating rule (from §7/§8).** Each feature lands oracle-first: emulator →
proofs → compiler/toolchain → tests, and **nothing transitional is frozen into
RTL before its proofs pass** (endpoints/ring need the bounded-ring WCET +
ring-cap-safety proofs E4; domain scheduler/IPC tracks 2–3 stay deferred). Track 1
domain unification is sanctioned to implement+freeze now.

## Phase 1 — v2 ISA core
Status: **done + validated** (see `isa_v2_change_list.md` final reconciliation).
Redis 7 boots & serves on the emulator; full FDR→GPR fd-handle migration complete.

## Phase 3 — unified endpoints (`send`/`recv`/`call`/`wait` + ring)

| Item | Layer | Status |
| --- | --- | --- |
| EP-A endpoint object: `Endpoint` held-cap kind, `(bytes,caps)` message queue + mode | emulator | **done** |
| EP-B `send` / `recv` verbs over endpoint handles (non-blocking; byte-fd delegation TBD) | emulator | **done** |
| EP-C `wait(waitset,timeout)` (collapse await/probe/futex_wait/join/wait_pid/sleep/alarm) | emulator | **done** (poll + block-until-edge; timed wakeup TBD) |
| EP-D `gate_call`/`gate_return` = the `call` verb (cross-domain migrating gate) | emulator | **built + M2-proven** (existing 0x2f) |
| EP-E async completion ring: SQE/CQE layout freeze + ring engine + `ring_setup`/`ring_enter` | emulator | **done** (emulator; schema/RTL gated on EP-F) |
| EP-F bounded-ring WCET + ring cap-safety proofs (E4 — **gate before RTL**) | formal | pending |
| EP-G LLVM `.td` verbs + thin libc shims (read→recv, write→send, poll→wait, …) | compiler | pending |
| EP-H RTL IPC/Gate engine (only after EP-F) | rtl | blocked on EP-F |

Opcode assignments (free slots `0x83-0x9f`): `send`=0x83, `recv`=0x84 (done),
`endpoint_create`=0x88 (done), `wait`=0x86 (pending), `ring_enter`=0x87
(pending). The `call` verb **is** the existing M2-proven `GATE_CALL` (0x2f) —
no new opcode; `0x85` left free.

## Phase 2 track 1 — unified domains (process = Resource Domain)

| Item | Layer | Status |
| --- | --- | --- |
| N1 design.md: process = domain holding addr-space cap + threads; names-are-data | docs | pending |
| N2 schema + emulator: one uniform domain record (limit only; reservation slot empty) | schema + emulator | pending |
| N3 formal/M-series: re-confirm confinement under fork/exec/signal/wait as domain ops | formal | pending |
| N4 leaf-profile cost guard; fail-closed on slot exhaustion | emulator | pending |
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
- EP-E: completion ring (`ring_setup` 0x89, `ring_enter` 0x87) — frozen 64-byte
  SQE / 32-byte CQE; bounded drain (depth ≤ 1024); each SQE is a deferred
  send/recv dispatched via ep_send_inner/ep_recv_inner, so ring cap-safety is
  inherited (handles resolve against the submitter's table). 2 unit tests
  (drain+deliver+CQE, oversize-batch EINVAL); 482 cargo pass. Schema freeze +
  RTL gated on the EP-F proofs.
