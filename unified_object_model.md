# LNP64 Unified Object Model — Domains and Endpoints are one held-capability table

Status: **the single design/thesis doc for the unification.** It replaces
`unified_domain_refactor.md` and `unified_endpoint_ipc.md` (both merged in here). To
stop re-deriving the same idea in parallel, each surviving artifact now has **one job**:

- **this doc** — the *design/thesis*: what the unified object is, why, and what is left.
- [`isa_v2_unification_impl_status.md`](isa_v2_unification_impl_status.md) — the *live
  tracker*: per-layer status and the assigned opcode slots. Don't duplicate it here.
- [`isa_v2_design.md`](isa_v2_design.md) §8 — the *phased roadmap* + the
  "what Phase 1 must not freeze shut" reconciliation.
- [`isa_v2_change_list.md`](isa_v2_change_list.md) — the *change ledger* (incl. §F, the
  IPC/async dedup).
- **M1–M15 typed-trace Lean models** — the *proofs*.

This doc is a **delta on a half-built system**, not a fresh proposal. Ground truth is the
emulator + shared schema + the M-series proofs; where this prose and the implementation
disagree, the implementation (and §0 below) wins.

## 0. Anchor — what is already built, proven, or in progress

The implementation already unified most of this. Read §0 before proposing anything.

- **Object model (built, emulator).** `ObjectKind` × `ObjectProfile` is the endpoint type
  system; **`OBJECT_CTL` (0x4b) is the endpoint factory** — `CREATE`, the full BSD-socket
  API (`BIND`/`LISTEN`/`CONNECT`/`ACCEPT`/`GETSOCKNAME`/`GET`/`SETSOCKOPT`), and
  `CLASSIFY`. `MESSAGE_ENDPOINT` exists; `pipe` is a `Queue`/`Pipe` object; sockets,
  completion-counters, and call-gates are object profiles. `FDR_KIND_CALL_GATE` makes a
  call-gate a capability kind — the gate *is* an endpoint.
- **The gate (built; M2-proven).** `GATE_CALL`/`GATE_RETURN` are the **canonical**
  mnemonics (`CALL_CAP`/`RET_CAP` are legacy aliases — `asm.rs:817,826`). Three modes —
  **sync** (migrating round-trip), **async** (completion posted to a counter object),
  **handoff** (continuation passed to a service routine) — see `demos/call_gate_modes.s`.
  A generation-guarded **continuation** (`lnp64_gate_continuation_t` =
  `{continuation_id, caller/callee pid+tid, domain_id+gen, generation, mode}`), **fault
  delivery** (`fault_delivery_gate_ok`/`delivered_faults`), and **signal compatibility**
  (`signal_compatibility_ok`) are already in `rtl/engines/lnp64_m2_gate.sv` + schema, with
  `sync_roundtrip_ok`/`async_delivery_ok`/`handoff_delivery_ok`/`stale_continuation_rejected`
  proved in the M2 Lean model.
- **Proofs (done).** M1–M15 typed-trace all complete (2026-06-20). **M2** = the gate,
  **M14** = domain delegate/budget, **M15** = gate profile/queue.
- **In progress.** An `Endpoint` held-cap kind, a frozen `(bytes, caps)` message-descriptor
  layout, and assigned opcode slots (`send`=0x83 …) are already being landed —
  `emulator.rs:589,607,6424`, `isa.rs:254`, tracked in the impl-status doc.

So the four-verb endpoint model, the gate (modes + continuation + fault + signal), the
object factory, and the domain delegate/budget are **substantially built and partly
proven**. The remaining work (§12) is naming, the full collapse, the ring, signal-fold,
and the deferred scheduler — not re-inventing the model.

## 1. The one thesis

> **Everything is a held capability in a domain's object table. Some held caps are child
> domains, some are endpoints, some are address spaces. A "process" is a domain holding an
> address-space cap + threads. You `send`/`recv`/`gate_call`/`wait` over endpoints. Names
> are data; authority is capability.**

This folds process/container/VM/sandbox/supervisor *and* the whole IPC/async surface into
one object table with one naming discipline. "Everything is a file" (Plan 9) upgraded to
"everything is a typed, capability-safe object," with io_uring's "one wait."

## 2. The objects (the taxonomy already in code)

- **Domain** — the node: held caps + threads + an accounting budget. A "process",
  "container", and "VM" differ only by which slots they populate; there is **no type tag**
  and **no leaf-vs-interior variant** (those words are *descriptive* of populated slots).
  [`domain_ctl` 0x4c; M14]
- **Endpoint** — an object you message/wait over. Kinds/profiles already present or slated:
  message-endpoint, pipe (`Queue`/`Pipe`), socket, **call-gate (3 modes)**, completion-
  counter, timer, signal, futex, fd/file, child-exit, thread-exit, and the **ring** (the
  one genuinely new profile, §10). [`OBJECT_CTL`/`ObjectProfile`]
- **Threads** — the scheduling axis. **Not** objects, **not** domains (a domain holds zero
  or more threads).
- **Held caps** — address-space, endpoint handles, device/DMA authority, gates. **One
  handle namespace**: the "capabilities are GPR handles" migration is dissolving the
  separate FDR file into GPR-named handles; RTL should treat the 256 FDR slots as a *cached
  view* of held caps, not an independent file (`isa_v2_design.md` §4.2/§8).

## 3. The verbs

- **`send(ep, msg=(bytes,caps))` / `recv(ep, buf)`** — message transfer; a message carries
  inline bytes *and* a capability-handle vector. Collapses `push`/`pull`(+`_dyn`)/
  `cap_send`/`cap_recv`/`read_fd`/`write_fd`/`futex_wake` (notify = empty msg) and `kill`
  (async signal = upcall-mode send, §6). Small payloads ride in registers (WCET-clean);
  large payloads ride as a page-grant cap (zero-copy).
- **`gate_call(ep, msg) -> reply` / `gate_return`** — the **cross-domain** gate (canonical
  `GATE_CALL`/`GATE_RETURN`; `call_cap`/`ret_cap` legacy aliases). Switches protection
  context, runs the migrating gate (priority/bandwidth inheritance), checks a capability.
  Three modes (§6). **Distinct from intra-domain `JAL`/`JALR` (`CALL`/`RET`)** — those are
  ordinary same-context control transfer, single-cycle, untouched. Keep the verb spelled
  `gate_call`, never bare `call`, to avoid colliding with the `CALL sym = JAL r1` pseudo.
- **`wait(waitset, timeout) -> ready`** — block until any edge in the set fires. Collapses
  `await`(+`_dyn`)/`await_ex`(+`_dyn`)/`waitable_probe`(+`_dyn`)/`futex_wait`/`thread_join`/
  `wait_pid`/`sleep`/`alarm`. Readiness mask = POSIX `revents` (already true:
  `POLLIN=1`/`POLLOUT=4` in the emulator). A ring is one waitable endpoint.
- **Lifecycle (orthogonal — keep):** `OBJECT_CTL` (create/socket/classify), `cap_dup`,
  `cap_revoke`, close. These manage the cap table, not transfer.
- **Signals = the async-upcall mode of an endpoint** (§6).

### The collapse (opcode → verb)

| Verb / role | Subsumes (current opcodes) | Status |
| --- | --- | --- |
| `send` | `push` 0x2c, `push_dyn` 0x3c, `cap_send` 0x51, `write_fd` 0x57, `gate_return`/`ret_cap` 0x4f, `futex_wake` 0xcc, `kill` 0x64 | verb pending; parts built |
| `recv` | `pull` 0x2b, `pull_dyn` 0x3b, `cap_recv` 0x52, `read_fd` 0x2d, `sigtimedwait`-class | verb pending |
| `gate_call`/`gate_return` | `GATE_CALL`/`call_cap` 0x2f, `call_cap_dyn` 0x4e (gate, 3 modes) | **built + M2-proven** |
| `wait` | `await` 0x2e, `await_dyn` 0x4d, `await_ex` 0x71/`_dyn` 0x72, `waitable_probe` 0x6f/0x70, `futex_wait` 0xcb, `thread_join` 0x5a, `wait_pid` 0x7e, `sleep` 0x07, `alarm` 0x68 | verb pending; sources built |
| `OBJECT_CTL` (factory/lifecycle) | create/socket/classify; `pipe`/socket/call-gate/counter profiles | **built** |
| `cap_dup` / `cap_revoke` | cap-table lifecycle | **built** |

~20 IPC/async opcodes → **4 verbs + the factory + 2 cap-table ops + the ring**. The
mechanical static/`_dyn` dedup (frees 0x3b/0x3c/0x70/0x72, then 0x4e) is `isa_v2_change_list.md` §F.

## 4. Protection context vs scheduling context (the keystone)

Unix conflates them; LNP64 splits them, and the split resolves every hard IPC question.
The word "budget" spans **three** things — do not conflate:

1. a domain's **limit** — accounting cap on resource creation; never a guarantee;
2. a domain's optional **reservation** — a scheduling *guarantee* threads schedule against;
3. a thread's **scheduling context** — the CPU-time budget + priority it is dispatched under.

Senses 1–2 belong to the **protection context** (the domain) and **never migrate** with a
call. Sense 3 belongs to the **scheduling context** (the thread) and **migrates** with a
`gate_call`. So: faults attribute to the protection context (the server); CPU time
attributes to the scheduling context (the client). This is already physical — the
continuation (`lnp64_gate_continuation_t`) carries the scheduling-context marker across the
hop.

## 5. Gate modes & fault containment (built; M2)

- **sync (migrating, default):** the server is passive code+caps; the thread runs server
  code **on the caller's budget/priority** (bandwidth/priority inheritance — avoids the
  Mars-Pathfinder priority inversion). *Calling a service spends your own time, like a
  function call.*
- **async (dispatched):** completion is posted to a counter/endpoint; the server has its own
  thread+budget; charged to the server. Use for cost isolation / background work.
- **handoff:** the continuation is handed to a service routine.
- **Faults are contained:** a fault while migrated into the server converts the in-flight
  `gate_call` into a **failed return to the client** (`ESRVFAULT`-class); the client's
  thread/continuation is preserved — *a crashed RPC server returns an error; the caller
  does not crash.* Transient caps granted for the call are revoked on the failed return.
  This is the driver-restart model: buggy driver faults → callers get errors → supervisor
  restarts it → nothing else dies. (`fault_delivery_gate_ok`, M2.)
- **The continuation stack is bounded + generation-guarded:** `gate_return` pops O(1);
  depth bounded ⇒ WCET + fault containment + reentrancy safety; stale frames rejected
  (`stale_continuation_rejected`, M2).

## 6. Signals — the async-upcall mode of an endpoint

A signal is the **asynchronous-upcall** dual of a synchronous `recv` — same signal endpoint,
two delivery modes. The emulator already delivers signals by pushing a **signal frame** onto
a per-thread, generation-checked **signal-frame stack** (`deliver_signal_if_needed`
`emulator.rs:10187`; `SIGRET` pops it, `:4144`) — structurally the **same continuation
stack** as the gate's, which is why M2 carries `signal_compatibility_ok`. So:

- `kill(pid, sig)` = **`send`** to the target domain's signal endpoint (names-are-data:
  resolve the PID, check a capability — never act-by-integer). `alarm`/timer = a timer
  endpoint that sends SIGALRM.
- `sigaction` = **register the upcall handler** — the same object as a domain's **fault
  gate** (a fault is an involuntary signal). `SIG_DFL`/`SIG_IGN` are dispositions.
- `SIGRET` = **`gate_return` for the upcall.** `sigprocmask`/`SIGMASK` PCR = endpoint
  **mask**; `SIGPENDING` PCR = the **pending set**.

This is seL4/Zircon exactly (a notification can be waited-on or bound for async delivery; a
fault is delivered to a registered gate). Signals do **not** move to software — the frozen
mechanism is *re-expressed* as an endpoint mode.

## 7. Flush-free crossing + tagging

With **(a) PIPT caches, (b) a fully tagged TLB, (c) the in-order non-speculative core**, a
normal `gate_call` needs **no flush** — only refills. Narrow exceptions: tag recycling under
exhaustion (avoidable for RT via a pinned pool) and concurrent revocation. Tag space
partitions into a **pinned pool for RT domains** (never shot down; admission also reserves a
pinned tag and **fails closed** if none free) and a **recyclable best-effort pool** (victim
shoot-down, generation-guarded). The in-order core also eliminates the Spectre-class
predictor flush across the trust boundary. `design.md` line-635 invalidations scope to
**exec-replace + revocation**, not gate calls.

## 8. Cheap-leaf: a domain costs ~a fork

Per-request / per-connection / per-driver isolation is only the default if a domain is cheap.

- **Sparse uniform node:** a leaf populates only `{id+gen, parent, timeshare weight,
  address-space cap, FDR/endpoint table base, limit}`; it does **not** materialize
  child-domain tables, DMA slots, IOMMU windows, or CBS-server state.
- **DDR-backed Domain Descriptor Table** with a small on-chip hot cache (running/ready
  domains) — millions of domains DDR-bound, on-chip cost cache-bound, like FDR tables.
- **`fork ≈ O(1)`:** COW the address-space object, share-or-COW the FDR/endpoint table, alloc
  a DDT entry, spawn one thread. **Fork-bomb safety for free:** child creation charged to the
  parent's mandatory limit; exceeding it makes `CLONE` fail closed. Generation-checked id
  reuse.

## 9. Scheduler model (DEFERRED track — proof-gated)

Realtime is a pillar, so the scheduler is frozen as **analyzable mechanism with per-thread
modes**, time-share default, reservations opt-in. **Not frozen in this pass** — gated on the
compositional-schedulability proof before any RTL freeze.

- Per-thread `SCHED_CLASS`: `TIMESHARE` (default, no WCET claim), `FIXED_PRIO`
  (RMA-analyzable), `RESERVATION{budget,period}` (CBS). `CLONE` precedence: explicit wins;
  else inherit `TIMESHARE`/`FIXED_PRIO`; a `RESERVATION` is **never implicitly inherited**
  (would double utilization; mirrors `SCHED_DEADLINE`) — child of a reservation is
  `TIMESHARE` unless explicit, and any explicit reservation must pass admission.
- **Degenerate flat fast path:** with no RT threads/reservations the engine is one flat
  time-share runqueue, expanding to hierarchical only when something opts in (dynamic).
- **Admission, fail-closed, reserves a time-share floor:** total admitted reservation
  utilization ≤ `1 − floor`; a reservation that would eat the floor is refused.
- **Isolation theorem, both directions:** time-share can't make a reservation miss (CBS);
  a reservation can't starve time-share below the floor (CBS throttling bounds one;
  admission bounds the sum). **Compositional schedulability** of nested reservations is the
  hard proof frontier.
- POSIX: `SCHED_OTHER→TIMESHARE`, `FIFO/RR→FIXED_PRIO`, `DEADLINE→RESERVATION`.

## 10. The completion-ring (the one genuinely-new piece — frozen, proof-gated)

The async face of the same verbs, **frozen in the ISA** (per decision) — not a second IPC
ABI. A **ring is an endpoint** whose inbox is completion entries, so `wait` on a ring needs
no new concept. An **SQE is a deferred verb**, a **CQE its result**; the SQE/CQE binary
layout and fixed-depth ring are frozen. `ring_enter(ring_ep, n_submit, min_complete,
timeout)` = `wait` generalized. **Capability-safe** because SQE cap fields are cap-table
indices the engine resolves against the *submitter's* table (names-are-data); received caps
are installed by the engine. **WCET-bounded:** fixed depth + bounded drain; RT paths bypass
the ring via the synchronous migrating `gate_call`. A **proto already exists** — the
async-gate **completion-counter**. Gated on the **bounded-ring WCET** + **ring
capability-safety** proofs before any RTL freeze.

## 11. POSIX legality (the frozen subset still lowers cleanly)

A re-expression of existing semantics (the emulator already uses POSIX poll bits and has
`futex`/`thread_join`/the gate). Shims get *thinner*, nothing moves to software.

| POSIX | Lowers to |
| --- | --- |
| `read`/`recv*` · `write`/`send*` | `recv` / `send` (`SCM_RIGHTS` = the caps vector) |
| `poll`/`select`/`epoll_wait`/`futex(WAIT)`/`nanosleep`/`wait4`/`pthread_join`/`sigtimedwait` | `wait(waitset, timeout)` |
| `futex(WAKE)` | `send(futex-ep, empty)` |
| `kill`/`raise` · `sigaction` · `sigreturn` · `sigprocmask` | `send(signal-ep)` · register upcall · `gate_return` · mask |
| `pipe`/`socket`/`bind`/`listen`/`connect`/`accept`/sockopt | `OBJECT_CTL` (already implemented) |
| `eventfd`/`timerfd`/`signalfd` | endpoint profiles; `send`/`recv`/`wait` |
| io_uring | a ring endpoint — native |

## 12. What's actually LEFT (the deltas)

Everything in §2–§8 is built or proven except naming. The open work, each gated, nothing
transitional frozen into RTL before its proof (live status: the impl-status tracker):

- **Canonical naming + dedup** — `gate_call`/`gate_return` canonical; the four verbs;
  retire `call_cap`/`ret_cap`; static/`_dyn` Tier-1/2 collapse (change-list §F).
- **Endpoint kind + `send`/`recv`/`wait` verbs** — in progress (tracker EP-A…EP-G).
- **Completion-ring** (EP-E) + **its proofs** (EP-F: bounded-ring WCET + cap-safety — gate
  before RTL).
- **Signal-fold** — `kill`=`send`, `sigaction`=handler, `SIGRET`=`gate_return`; reuse the
  existing signal-frame/continuation stack + `signal_compatibility_ok`.
- **Scheduler model** (§9) + the compositional-schedulability proof (gate before RTL).
- **Cheap-leaf representation** (§8): sparse node + DDR-backed DDT + O(1) COW fork.
- **Coq read/fetch permission** (the `design.md` §3.2 gap).

## Conscious grafts (named, not rediscovered)

- **setuid/setgid exec = authority amplification** — the loader confers a capability from the
  file's setuid bit.
- **Orphan reparenting** — a domain death-event + reparent policy, not arbitrary re-homing.
- **PID reuse / kill-by-pid** — the ambient-naming graft, contained by names-are-data.

## Non-goals

- Not moving any frozen POSIX mechanism (incl. signal delivery) into software — re-expressed,
  not relocated; libc shims get thinner.
- Not making threads into domains (threads are the scheduling axis).
- Not adding ambient (capability-free) authority via PIDs/pgrps.
- Not a second IPC ABI — the ring and the migrating `gate_call` are one verb set, two faces.
- Not conflating the cross-domain `gate_call` with the intra-domain `JAL`/`JALR`
  (`CALL`/`RET`).
- Not making bandwidth reservation the default; not exposing ASID/domain tags to software.
