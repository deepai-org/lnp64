# Planned Refactor: One Endpoint, One Message, Four Verbs (+ a frozen ring)

Status: **planned design refactor**, not yet implemented. This is the IPC/async
companion to [`unified_domain_refactor.md`](unified_domain_refactor.md): that doc
unifies process/container/VM into one **Resource Domain**; this one unifies the
~20 IPC/async opcodes into one object (the **endpoint**), one message shape
(`bytes + capabilities`), and four verbs (`send` / `recv` / `call` / `wait`),
with an **async completion-ring frozen into the ISA** as the substrate and the
synchronous **migrating call** as its depth-1 realtime fast path.

> **FUTURE INTENDED DESIGN — NOT TO BE FROZEN YET.** Recorded here so the object
> model, message shape, ring format, and POSIX lowering have a fixed vocabulary.
> This is the *target* IPC ISA. It composes with `unified_domain_refactor.md`
> **track 3** (the migrating-IPC microarchitecture): the `call` verb below *is*
> that track's migrating gate. Freeze is gated on (a) the track-3 design landing
> and (b) the bounded-ring WCET proof (work items below). Until then the existing
> opcodes stand; the Tier-1/2 mechanical dedup (see `isa_v2_change_list.md`) is the
> only near-term cleanup.

## The thesis

> **Everything you can transfer or wait on is an *endpoint capability*. A message
> is `(bytes, capabilities)`. The entire IPC/async surface is `send` / `recv` /
> `call` / `wait` over endpoints, submitted through one frozen completion ring.**

"Everything is a file" (Plan 9) upgraded to **"everything is an endpoint"** —
now typed and capability-safe — plus io_uring's **"there is only one wait."** fds,
pipes, sockets, futexes, call-gates, timers, child-exit, semaphores, and the ring
itself all become endpoint handles in the *one* handle namespace that the in-flight
"capabilities are GPR handles" migration is already building.

## Why there is "so much" today: a cartesian product, not a factoring

Every current IPC/async opcode is one cell of **(axis × source-type)**, enumerated
instead of factored:

- **Transfer axis** — move `(bytes, caps)` across a domain boundary; the only real
  variable is direction. Today: `push`, `pull`, `push_dyn`, `pull_dyn`, `cap_send`,
  `cap_recv`, `read_fd`, `write_fd`, `ret_cap`.
- **Wait axis** — block until an edge fires; the only variable is the edge *source*.
  Today: `await`, `await_dyn`, `await_ex`, `await_ex_dyn`, `waitable_probe`,
  `waitable_probe_dyn`, `futex_wait`, `thread_join`, `wait_pid`, `sleep`, `alarm`.

`push` vs `cap_send` is the same verb at two points of the message space (data-only
vs cap-only). `await` vs `wait_pid` vs `futex_wait` is the same verb over three edge
sources. The implementation is already drifting toward the unification: today's
`pull` handler already reads "from an fd **or** pops the MESSAGE_ENDPOINT inbox" —
a proto-`recv` straddling fd and endpoint.

## The unification: endpoint + message

- **One object — the endpoint.** A held capability you can `send`/`recv`/`call`/
  `wait` on. fds, pipes, sockets, futex words, call-gates, timers, child/thread
  handles, and rings are all endpoints (some kernel-backed, some
  domain-to-domain). This is just another **held capability slot** in the Resource
  Domain of `unified_domain_refactor.md` — the two refactors share one object table.
- **One message — `(bytes, caps)`.** A message carries an inline byte payload *and*
  a vector of capability handles. Small byte payloads ride in **registers** (bounded,
  flush-free — WCET-clean); large payloads ride as a **page-grant capability** (the
  message's `caps` carries a handle to a granted page → zero-copy). Capability
  handles in a message are **cap-table indices, never raw authority** — see ring
  safety below.
- **Four verbs:**
  - `send(ep, msg)` — deliver a message to an endpoint (queued or rendezvous per the
    endpoint's mode). A *notification* is `send` of an empty message that just raises
    the endpoint's edge.
  - `recv(ep, buf)` — receive the next message; blocks per the endpoint's mode.
  - `call(ep, msg) -> reply` — fused `send` + `wait`-for-reply; the kernel mints a
    one-shot **reply endpoint** and threads it in. **This is the migrating gate**
    (priority/bandwidth inheritance, protected activation stack, flush-free crossing
    — `unified_domain_refactor.md` track 3). Reply = `send` on the reply endpoint.
  - `wait(waitset, timeout) -> ready` — block until any edge in the set fires, or the
    timeout. The waitset is a set of endpoint handles (a ring is one endpoint).

### The collapse, concretely

| Galaxy-brain verb | Subsumes (current opcodes) |
| --- | --- |
| `send(ep, (bytes,caps))` | `push` 0x2c, `push_dyn` 0x3c, `cap_send` 0x51, `write_fd` 0x57, `ret_cap` 0x4f, `futex_wake` 0xcc (notify = empty msg) |
| `recv(ep, buf)` | `pull` 0x2b, `pull_dyn` 0x3b, `cap_recv` 0x52, `read_fd` 0x2d |
| `call(ep, msg) -> reply` | `call_cap` 0x2f, `call_cap_dyn` 0x4e (+ reply via `send` on reply ep) |
| `wait(waitset, timeout)` | `await` 0x2e, `await_dyn` 0x4d, `await_ex` 0x71/`await_ex_dyn` 0x72, `waitable_probe` 0x6f/0x70, `futex_wait` 0xcb, `thread_join` 0x5a, `wait_pid` 0x7e, `sleep` 0x07, `alarm` 0x68 |
| `cap_dup` / `cap_revoke` (**orthogonal — keep**) | capability-table lifecycle, not transfer |

~20 IPC/async opcodes → **4 transfer/wait verbs + 2 cap-table verbs + the ring ops.**

## The frozen async completion-ring (the substrate, not a bolt-on)

The ring is **the asynchronous face of the same four verbs**, frozen into the ISA —
*not* a second IPC mechanism. The synchronous inline verbs are its depth-1 special
case. This is the io_uring insight made first-class and capability-safe.

- **A ring is an endpoint.** Its inbox is **completion entries**; `wait` on a ring
  is "wait for ≥N completions." So the ring needs no new wait concept — it folds
  into `wait`. (Uniformity: the ring is an endpoint whose messages are CQEs.)
- **SQE = a deferred verb; CQE = its result.** A **Submission Queue Entry** is a
  message describing `{verb, target-endpoint handle, inline bytes | buffer-cap,
  cap-handle vector, user-tag}`. A **Completion Queue Entry** carries
  `{user-tag, status, result bytes, installed cap-handles}`. The SQE/CQE binary
  layout is **frozen in the ISA** — the stable contract all layers implement, exactly
  as the opcode table is today.
- **Frozen ISA surface (minimal):** the SQ/CQ live in memory shared with the IPC
  engine (mmap'd, io_uring-style). The instruction surface is one doorbell that
  *is* `wait` generalized: `ring_enter(ring_ep, n_submit, min_complete, timeout)` —
  submit pending SQEs and reap ≥`min_complete` CQEs. (Open decision: keep this as a
  distinct opcode, or fold it into `wait` with a submit count when the waited
  endpoint is a ring. Leaning fold-in: fewer opcodes, same mechanism.)
- **Capability safety on a memory-resident ring** — inherits the domain refactor's
  *names-are-data, authority-is-capability* invariant verbatim. An SQE names caps by
  **cap-table index**, never by raw authority; the engine resolves every handle
  against the **submitter's** cap table, so writing an SQE can forge nothing you
  don't already hold. On completion, received caps are **installed by the engine**
  into the submitter's cap table and the CQE reports the new handle. The ring is
  data; authority still rides the cap table, checked in hardware.
- **WCET-bounded.** Rings are **fixed-depth** (frozen max); submit is O(1)
  (write SQE + doorbell); the engine drains a **bounded batch** per dispatch, charged
  to the submitter's **scheduling context** (the migrating-budget model). Realtime
  threads do **not** depend on ring-drain latency — they use the synchronous
  migrating `call` (below), which bypasses the buffer entirely.

### Sync and async are the same verbs, chosen by need

| | Mechanism | When |
| --- | --- | --- |
| **Synchronous** | inline `call`/`send`/`recv`/`wait` = depth-1 submit+block, and `call` = the **migrating gate** (flush-free, priority-inherited) | realtime RPC, driver calls, the WCET path |
| **Asynchronous** | the **ring**: batch many SQEs, reap CQEs with one `wait` | throughput servers (Redis), batching, hiding latency |

Same `(bytes,caps)` message, same endpoints, same four verbs. You pick the ring
for throughput or the migrating call for bounded latency — **not two IPC ABIs, one
with a fast path.** This mirrors the scheduler model's "flat fast path until you opt
into the heavy machinery."

## Prior art: this is a convergence, not an invention

Every serious capability/microkernel **and** the modern async story land here:

- **seL4** — endpoints (`Send`/`Recv`/`Call`/`Reply`) + notifications; caps ride in
  message registers. `Call` auto-mints a reply cap — exactly our `call`.
- **QNX** (shipped, realtime) — `MsgSend`/`MsgReceive`/`MsgReply` *is* the OS; drivers
  and fs are message passing. Validates the realtime synchronous core.
- **Fuchsia/Zircon** — channels carry **bytes + handles together** (our `(bytes,caps)`
  message verbatim); `port` + `object_wait_async` is the unified `wait` over any
  object.
- **io_uring** — collapses read/write/poll/timeout/send/recv/**futex** into ring ops
  with **one** wait. This is Linux performing our Tier-3 unification today; we freeze
  its capability-safe form.
- **Plan 9** — "everything is a file." The elegance; capabilities + typed messages
  make it finally safe.

## POSIX legality (the frozen subset still lowers cleanly)

The current emulator already uses **real Linux poll bits** (`POLLIN=1`,
`POLLOUT=4`) for `await` readiness and already implements `FUTEX_WAIT/WAKE`,
`THREAD_JOIN`, `CLONE_SPAWN` — so most of this is a *re-expression* of existing
semantics, not new behavior. Lowering map:

| POSIX | Lowers to |
| --- | --- |
| `read`/`recv`/`recvfrom`/`recvmsg` | `recv(fd-ep, buf)` (caps vector empty for byte fds) |
| `write`/`send`/`sendto`/`sendmsg` | `send(fd-ep, (bytes, caps))` (`SCM_RIGHTS` = the caps vector) |
| `poll`/`select`/`pselect`/`epoll_wait` | `wait(waitset, timeout)`; readiness mask = POSIX revents |
| `epoll_ctl` | add/remove an endpoint to/from a waitset (a kernel waitset object) |
| `futex(WAIT)` / `futex(WAKE)` | `wait(futex-ep)` / `send(futex-ep, empty)` |
| `nanosleep`/`clock_nanosleep`/`sleep` | `wait(∅, timeout)` (timer-only waitset) |
| `alarm`/`setitimer`/`timerfd` | a **timer endpoint**; arm = `send`, fire = edge; `wait`/`recv` |
| `wait4`/`waitpid` | `wait(child-exit-ep)` then `recv` status; child-exit is an endpoint |
| `pthread_join` | `wait(thread-exit-ep)` then `recv` retval |
| `sigtimedwait`/`signalfd` | a **signal endpoint**; `wait`/`recv` |
| `eventfd` | an endpoint with a counter payload; `send`/`recv` |
| `accept`/`connect` | a listening endpoint yields a new connection **endpoint cap** via `recv` |
| io_uring itself | a ring endpoint — native, not emulated |

Capability-passing fds (`SCM_RIGHTS`) stop being a graft: they're just the message's
`caps` vector, which the engine installs into the receiver's cap table — the same
path every cap transfer uses. The **frozen-POSIX-in-silicon** thesis is preserved:
nothing moves to software; the thin libc shims (`read`→`recv`, `poll`→`wait`, …) get
*thinner*, and the names-are-data invariant carries over unchanged.

## Open decisions

1. **Ring-enter as its own opcode vs folded into `wait`.** Leaning fold-in (`wait`
   on a ring endpoint with a submit count) to avoid a near-duplicate; confirm the
   encoding carries `n_submit`/`min_complete` cleanly.
2. **Endpoint mode taxonomy.** Rendezvous (seL4/QNX — no buffering, cheapest, WCET-
   clean) vs queued (throughput). Mode is a property of the **endpoint object**, not
   a different verb. Decide the fixed set of modes and which is default.
3. **Message register window size.** How many bytes/caps ride in registers before
   spilling to a page-grant — a direct WCET ↔ ergonomics knob (seL4 picks 120 bytes;
   we should pick from the migrating-gate frame budget in track 3).
4. **Reply-endpoint lifetime.** One-shot (seL4 reply cap) is the WCET-clean default;
   confirm it composes with the bounded activation stack (track 3).
5. **Ring + migrating call interaction.** Can an SQE carry a `call` (deferred RPC)?
   If yes, its completion is the reply; priority inheritance for a *ringed* call needs
   a rule (likely: ringed calls are best-effort; RT calls must be inline/migrating).

## Non-goals

- Not moving any frozen POSIX mechanism into the OS/personality (libc shims get
  thinner, not fatter).
- Not adding a second IPC ABI — the ring and the migrating call are one verb set with
  two faces.
- Not exposing raw authority through the memory-resident ring — SQE cap fields are
  cap-table indices resolved by the engine (names-are-data).
- Not making the asynchronous ring mandatory for the realtime path — RT uses the
  synchronous migrating `call`; the ring is the throughput face.
- Not unifying `cap_dup`/`cap_revoke` into transfer — they are cap-table lifecycle,
  orthogonal to messaging.

## Work items (when scheduled — gated; do not freeze before the proofs)

E1. `isa_v2_design.md` + `isa_v2_opcodes.md` + schema: define the **endpoint** object
    (a held-capability kind), the `(bytes, caps)` **message** shape, and the four
    verbs `send`/`recv`/`call`/`wait`. Mark `call` = track-3 migrating gate.
E2. Freeze the **SQE/CQE binary layout** and the ring object (fixed depth) in the
    shared schema; define `ring_enter`/`wait`-on-ring and the bounded-batch drain.
E3. Emulator: re-express `push`/`pull`/`cap_send`/`cap_recv`/`read_fd`/`write_fd`/
    `ret_cap` as `send`/`recv`; collapse the wait family into `wait`; add the ring
    engine with cap-table-mediated SQE/CQE handling.
E4. `formal_theorems.md`: **bounded-ring WCET** (fixed depth + bounded drain ⇒ bounded
    submit/reap latency) and **ring capability-safety** (SQE handles resolve only
    against the submitter's cap table; received caps install only via the engine) —
    **both must land before any RTL freeze.**
E5. LLVM `.td` + toolchain libc shims: lower `read`/`write`/`poll`/`epoll`/`futex`/
    `nanosleep`/`wait4`/`pthread_join`/`sigtimedwait`/`accept` onto the verbs per the
    POSIX map; keep the shims thin.
E6. RTL: the IPC/Gate engine processes endpoints + ring; PIPT + tagged-TLB + in-order
    ⇒ flush-free `call`; bounded ring drain. Composes with track-3 / track-9 tagging.

## How it composes with the domain refactor

One sentence is now the whole process/IPC/async model:

> **A Resource Domain holds endpoint capabilities; it `send`/`recv`/`call`s over
> them and `wait`s on them — synchronously via the migrating gate, or asynchronously
> via the frozen ring.**

Endpoints are domain slots (`unified_domain_refactor.md`). `call` is its track-3
migrating gate. The ring's cap safety is its names-are-data invariant. The three
tracks are one machine: **everything is a held capability; some held capabilities are
endpoints; you message and wait over endpoints.**
