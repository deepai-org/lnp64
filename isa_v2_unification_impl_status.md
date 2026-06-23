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
| EP-H LLVM `.td` verbs + thin libc shims (read→recv, write→send, poll→wait, …) | compiler | pending |
| EP-I RTL endpoint/gate engine (only after EP-F) | rtl | blocked on EP-F |

Opcode assignments: `send`=0x83, `recv`=0x84, `wait`=0x86, `endpoint_create`=0x88
(all **done**). The `call` verb **is** the existing M2-proven `GATE_CALL` (0x2f).
Per refined §3 there are **no ring/SQE/async opcodes** — `0x85` and `0x87` stay
free. An endpoint's behavior is its `Backing{Thread,Memory,Register} ×
Producer{sw,hw}` type, fixed at create.

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
