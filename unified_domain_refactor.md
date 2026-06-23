# Planned Refactor: Processes Are Uniform Resource Domains

Status: **planned design refactor**, not yet implemented. This is an architecture
change (touches `design.md`, `hardware_design.md`, `formal_theorems.md`, the
domain/capability engine, the shared schema, and the M-series confinement
invariants). It is *not* an OS/personality change and must not move any currently
frozen POSIX mechanism out of silicon.

**Companion:** [`unified_endpoint_ipc.md`](unified_endpoint_ipc.md) is the IPC/async
half of this thesis — one endpoint object, one `(bytes, caps)` message, four verbs
(`send`/`recv`/`call`/`wait`) and a frozen completion-ring. Its `call` verb *is*
track 3's migrating gate below; its endpoints are held capabilities of the domain
node defined here. The two compose into one machine. Both sit under the umbrella
roadmap in [`isa_v2_design.md`](isa_v2_design.md) §8.

## Scope: do the unification now; defer the scheduler and IPC tracks

This document covers **three separable tracks**. They are *not* one change, and only
the first is scoped to implement and freeze now:

1. **The unification (DO NOW).** Process = Resource Domain: one uniform node,
   address space as a held capability, fork/exec/clone as capability-sharing,
   names-are-data, re-prove confinement, and the cheap-leaf representation that
   keeps fork-heavy workloads cheap. This is the actual simplification — it removes
   an object kind and proves confinement once. Covered by sections *The idea*
   through *The conscious grafts*, plus *Cheap-leaf profile*, and work items N1–N5.
2. **The realtime scheduler model (FUTURE — DO NOT FREEZE YET).** Per-thread
   `SCHED_CLASS`, hierarchical/compositional CBS, the two-direction isolation
   theorem. A large independent subsystem carrying an open proof obligation
   (compositional schedulability). Captured here as intended design; gated on a
   paper proof of the composition bound **before** any RTL freeze.
3. **The migrating-IPC microarchitecture (FUTURE — DO NOT FREEZE YET).**
   Protection-vs-scheduling-context split, migrating/dispatched gates, the
   activation stack, tag pools, flush-free crossings. A good design (seL4-MCS
   lineage) but intricate greenfield machinery, independent of track 1. Captured
   here as intended design.

Tracks 2 and 3 do **not** depend on track 1 (the scheduler and IPC would be as large
either way), so bundling them under "the refactor" would inflate its risk. They are
recorded in full below so the vocabulary and obligations are fixed, but the
now-work is track 1 only. Everything in the *Scheduler model* and *IPC & fault
semantics* sections is **future intended design, not to be frozen in this refactor.**

## The idea

Collapse `process`, `container`, `VM`, `sandbox`, and `supervisor` into **one
recursive primitive: the Resource Domain.** Today `design.md` treats a *process*
(PID/PPID/PID-group, `CLONE profile=posix_fork`, per-process context) as a
distinct notion alongside Resource Domains (nested containment + accounting).
This refactor makes them the same object at the mechanism layer.

A **Resource Domain is one uniform node** that may *hold* any of:

- zero or more **threads** (the schedulable leaves — threads stay a separate axis,
  not domains),
- zero or more **address-space capabilities** (an address space is a *held
  capability*, not intrinsic to the node),
- zero or more **child domains**,
- other capabilities (FDRs, device/DMA authority, gates, …),
- an **accounting budget** with two parts: an always-present, cheap **limit**
  record (cgroup/rlimit style) that bounds resource *creation/consumption* charged
  to the domain (child domains, memory, fds), plus an *optional* **CPU bandwidth
  reservation** (see the scheduler model below). A limit is not a guarantee; only a
  reservation is. Most domains carry only the limit. The word "budget" therefore
  spans **three** distinct things across this document, and conflating them is the
  one mistake that breaks the model:
  1. a domain's **limit** — an accounting cap on resource creation; never a guarantee;
  2. a domain's optional **reservation** — a scheduling *guarantee*; an allocation
     threads are scheduled *against*;
  3. a thread's **scheduling context** — the CPU-time budget + priority the thread
     is *dispatched* under (see "protection context vs scheduling context" below).
  Senses 1–2 are properties of the *domain* (the **protection** context) and never
  migrate with a call; sense 3 is a property of the *thread* (the **scheduling**
  context) and migrates with it. That protection-vs-scheduling split — not a mere
  "two budgets" — is the keystone of the IPC model. **Scoping note:** only sense 1
  (the limit) is frozen by the now-refactor (track 1); senses 2–3 (domain
  reservations and thread scheduling contexts) belong to the deferred scheduler/IPC
  tracks. The node model names all three so the deferred work has a fixed
  vocabulary, but a domain built by the now-refactor carries only a limit — the
  reservation slot is defined-but-unpopulated until track 2 lands.

There is **no `process` vs `container` type tag** and **no leaf-vs-interior
variant.** Where this document later says "interior domain" or "leaf," those are
*descriptive* of which slots a node currently populates (a node with child domains
vs. one with none), **never a stored type field** — the representation may tier
sparse-vs-materialized (see cheap-leaf below), but identity carries no variant tag.
What a domain *is* (process, container, VM) is just which slots it
populates: a "process" is a domain that holds an address-space capability + one
or more threads; a "container/VM" is a domain that holds child domains; a node
can do both at once (a shell runs threads *and* spawns children, like real Unix).

## Why this is a simplification, not a complication

- **Removes a hardware object type.** One uniform node instead of process-struct
  + domain — fewer *distinct* object types, and one lifecycle/accounting/confinement
  path proven once. The reduction this claims is in **concept and proof surface**,
  not a blanket promise that total gate count falls: the scheduler, IPC, and tagging
  machinery below *are* real silicon, but they are pre-existing realtime/IPC
  mechanisms unified onto **one** node type rather than new policy, and the
  degenerate fast path (below) keeps the common no-RT case cheap. Net: strictly
  fewer object *kinds* and proofs; roughly neutral-to-smaller gates.
- **One lifecycle, accounting, and confinement, proven once.** create / freeze /
  resume / destroy / death-event, budgets, attach/detach, capability scope —
  defined for domains, inherited by "processes." rlimits and cgroup limits become
  one budget mechanism at different nesting depths.
- **fork/exec/clone fall out of capability rules.** `fork` = spawn a child domain
  that COW-inherits the parent's address-space capability + a duplicated FDR table
  (plain POSIX `fork` copies the fd set unchanged; *narrowing* or *sharing* fds is
  a `clone`-spectrum choice, see below); `exec`
  = replace the image held by a domain; the whole `clone()` spectrum (share VM?
  fds? signal handlers?) = *which capabilities are shared* with the child domain.
  The process/thread continuum stops being a special case.
- **Address space as a held capability** makes shared-VM clones and vfork natural
  (two domains referencing the same address-space object) instead of special
  cases — strictly more expressive at no extra concept count.
- **Recursion is free.** "A process sub-delegates to child sandboxes" is the same
  nesting as the guest-can-be-a-host hypervisor model; no new machinery.

This completes a thesis already in the README: Resource Domains are "one nested
containment primitive" for containers/VMs/supervisors/cgroups. The process was
the missing case; this folds it in.

## Have cake and eat it: keep POSIX frozen in silicon

This refactor must **not** push more into the OS than is already there. Freezing a
stable POSIX subset in hardware is on-thesis (the core LNP64 bet). The refactor is
a **consolidation of hardware objects, not a migration of policy to software.**

- Every currently frozen POSIX mechanism stays frozen, now **domain-bound** instead
  of bound to a separate process struct: signal delivery via gates
  (`GATE_*`/`SIGRET`), futex wait/wake, FDR read/write, `CLONE`/`EXEC`/`EXIT`,
  `AWAIT_EX`, scheduler runqueue admission. Nothing leaves the chip.
- Process bookkeeping is **already domain-keyed** in the current design (the
  per-domain PID counter checked by `domain_nested_test`, and PCRs `PID`/`PPID`).
  This refactor extends that, it does not relocate it.
- The thin process service (path resolution, namespace policy) stays exactly as
  thin as today. No new OS surface.

## The one invariant: names are data, authority is capability

The Unix process bits that are *not* tree-shaped — PID-as-name, process
groups, sessions, wait/zombie/reaping, signal targeting by pgrp — **stay frozen in
silicon as domain-keyed indices/relations classified as data/observability, never
as authority.** This is already required by the **evidence-honesty** severe goal
("observability/naming are data paths only; cannot become hidden authority").

The single discipline that keeps the abstraction clean while POSIX stays frozen:

> A frozen name (PID, pgrp, session) is **addressing only**. The authority to act
> through it always rides a held capability. `kill(pid, sig)` resolves the PID in
> hardware **and** checks a capability to the target domain/group. No act-by-
> integer ambient authority, ever.

Hold that line and you get clean abstraction + frozen POSIX + compatibility
simultaneously. The *only* way to lose cleanliness is to let a frozen PID become
ambient authority the way stock Unix does — which this design refuses by an
existing severe goal.

## The conscious grafts (named, not rediscovered later)

A capability machine emulating Unix has a few impedance points. They are accepted
grafts, handled at the level already frozen, unchanged by this refactor:

- **setuid/setgid exec = authority amplification** (opposite of monotone
  capability narrowing). Handled as today: the loader confers a capability derived
  from the file's setuid bit; the binary's domain is pre-granted that authority.
- **Orphan reparenting to init/subreaper = tree mutation on death.** Modeled as a
  domain death-event plus a reparent policy, not arbitrary re-homing of authority.
- **PID reuse / `getpid` stability / kill-by-pid** = the ambient-naming graft,
  contained by the names-are-data invariant above.

## Scheduler model: multi-mode, time-share default, reservations opt-in

> **TRACK 2 — FUTURE INTENDED DESIGN, NOT TO BE FROZEN IN THIS REFACTOR.** Recorded
> here so the vocabulary and proof obligations are fixed. Implementation (work items
> D1–D2) is gated on a paper proof of the compositional-schedulability bound before
> any RTL freeze. The now-refactor (track 1) does not touch the scheduler beyond
> leaving the optional reservation slot defined-but-unpopulated. Read the prose
> below as the *target*, not as work scoped for this change.

Realtime is a pillar, so the scheduler is frozen in silicon — but as **analyzable
mechanism with per-thread policy modes**, not a single frozen discipline.
Bandwidth reservation is *one* mode, not the meaning of every domain budget;
traditional time-share is the easy default so the common case stays cheap.

### Per-thread scheduling class

Each thread carries exactly one `SCHED_CLASS` in its thread context:

- `TIMESHARE` — **default**. Weighted-fair / priority best-effort. No WCET claim.
- `FIXED_PRIO` — static-priority preemptive (RMA / response-time analyzable).
- `RESERVATION{budget, period}` — CBS / constant-bandwidth server: the *one*
  bandwidth-reservation mode, giving temporal isolation + a hard guarantee.

`CLONE` sets the class by a fixed precedence: an **explicit** class in the `CLONE`
profile wins; absent that, the child **inherits** the parent's class for
`TIMESHARE`/`FIXED_PRIO` (weight/priority carry down). A `RESERVATION` is **never
implicitly inherited** — its `{budget, period}` cannot be silently duplicated
without re-admission, which would double the reserved utilization (this is why
`SCHED_DEADLINE` forbids inherited-deadline fork) — so a child of a `RESERVATION`
thread is `TIMESHARE` unless the profile explicitly requests a class, and any
explicit `RESERVATION` must pass admission or `CLONE` fails closed. `TIMESHARE` is
thus the default both at a thread tree's root and as the safe fallback out of a
reservation. One class per thread — no blending.

### Two levels, with a degenerate flat fast path

- **Interior domains** may carry an optional **reservation** (compositional
  realtime — a reservation inside a reservation still composes); by default a
  domain has only a limit/weight, not a reservation.
- **Threads** carry the per-thread class above and are scheduled against their
  domain's allocation.
- **Degenerate fast path:** when no `FIXED_PRIO`/`RESERVATION` threads or reserved
  domains exist, the engine collapses to a single flat time-share runqueue. You
  pay hierarchical-scheduler cost *only* when something opts into realtime — this
  is what makes "traditional is the easy default" true in silicon, not just API.
  The condition is evaluated **dynamically**: the first `CLONE` of a
  `FIXED_PRIO`/`RESERVATION` thread (or admission of a domain reservation) expands
  the engine into hierarchical mode, and it collapses back to the flat runqueue
  once the last such thread/reservation exits. The fast path is the steady state of
  a no-RT workload, not a one-time boot-time configuration.

Hard guarantees compose only along the chain of nodes that explicitly opted in;
everything else is best-effort. Default end-to-end = time-share, zero reservation
machinery engaged.

### Admission control, fail-closed

Setting a `RESERVATION` (thread or domain) goes through **hardware admission**: if
the utilization cannot be admitted under the compositional bound, it is
**refused**, never silently downgraded to best-effort (ties to the bounded-
behavior / fail-closed severe goal). The compositional bound reserves a
**non-zero time-share floor**: total admitted reservation utilization is capped at
`1 − floor`, so admission *itself* is what discharges the second direction of the
isolation theorem (below). A reservation that would eat into the floor is refused —
the floor is never overcommitted away.

### IPC carries priority/bandwidth inheritance (realtime correctness)

`GATE_CALL` into a server (driver / microkernel service) runs the server **on the
caller's reservation/priority** (bandwidth/priority inheritance), fast-path
bypassing the scheduler. Without this, cross-domain IPC reintroduces unbounded
**priority inversion** (the Mars Pathfinder failure) and "isolated drivers" would
contradict "realtime." So the migrating call-gate is a realtime *correctness*
requirement, not just a speed optimization.

### Realtime-contract scoping + isolation (proof obligations)

- WCET / bounded-dispatch guarantees apply **only** to `FIXED_PRIO` /
  `RESERVATION` threads and reserved domains. `TIMESHARE` is best-effort with **no**
  WCET claim (no accidental stronger claim).
- **Isolation theorem, both directions:** (1) time-share load cannot make a
  reservation miss — CBS temporal isolation bounds *upward* interference; (2) a
  reservation cannot starve time-share below the admission-reserved floor. The two
  directions rest on two distinct mechanisms that must both hold: CBS **throttling**
  bounds a *single* reservation (beyond its `{budget, period}` guarantee it runs
  only in slack, never preempting the floor), and **admission** (above) bounds the
  *sum* of guarantees at `1 − floor`. "Consumes only slack" is therefore precise
  only for demand *above* a reservation's guarantee; the guarantee itself is paid
  out of the `1 − floor` admitted envelope, not slack.
- **Compositional schedulability** (nested reservations still bound end-to-end
  latency — periodic resource model / CBS composition) is the named hard realtime
  proof frontier.

### POSIX mapping (falls out cleanly)

`SCHED_OTHER → TIMESHARE`, `SCHED_FIFO`/`SCHED_RR → FIXED_PRIO`,
`SCHED_DEADLINE → RESERVATION`. `SCHED_DEADLINE` *is* CBS, so the reservation mode
maps exactly. Three modes cover the POSIX scheduling surface — a frozen scheduler
does not mean a fixed discipline.

## Cheap-leaf profile: making a domain cost ~a fork

"Cheap security via resource domains" has two distinct cheapnesses; both are
required.

### Cheap to create / hold / destroy

- **Sparse uniform node.** Keep "no type tag," but tier the *representation*:
  optional slots materialize on demand. A leaf populates only `{id+generation,
  parent link, timeshare weight, one address-space cap handle, FDR table base,
  limit/budget}`. It does **not** allocate child-domain tables, device/DMA
  authority slots, IOMMU window sets, or `RESERVATION` CBS-server state. "Process
  profile" = the sparse population, not a distinct object type.
- **Domain Descriptor Table cached like a TLB.** A large **DDR-backed** descriptor
  table with a small **on-chip hot cache** (running/ready domains). Create = alloc
  id + write DDR descriptor + COW caps; the descriptor pages on-chip when
  scheduled; idle/blocked domains evict to DDR. This decouples *how many domains
  exist* (millions, DDR-bound) from *on-chip cost* (cache-bound), exactly as FDR
  tables are already DDR-backed with an fd0–255 fast bank.
- **fork ≈ O(1).** COW the address-space *object* (one mark; VMA engine does lazy
  per-page COW), share-or-COW the FDR-table object, alloc a DDT entry, spawn one
  thread. No per-page or per-fd copy at fork time.
- **Fork-bomb safety for free.** Child creation is charged against the parent's
  mandatory budget/limit; exceeding it makes `CLONE` fail closed. cgroup-style
  containment of fork storms falls out of the budget being non-optional — this is
  both the cost guard and a security property.
- **Generation-checked id reuse** for safe, cheap recycling.

### Cheap to enforce per access

- Capability + domain checks are O(1) hardware in the load/store and engine-submit
  pipeline, parallel with address translation — no software ACL lookups
  (guaranteed by the names-are-data invariant).
- A domain *switch* is a tag change, **not a flush** (see IPC below — same tagging
  substrate serves cheap enforcement and fast IPC).

Payoff: when a domain costs ~a fork and enforcement is free, per-request /
per-connection / per-driver isolation becomes the **default**. Cheap-leaf is the
mechanism that delivers the cheap-security goal, not a side optimization.

## IPC & fault semantics

> **TRACK 3 — FUTURE INTENDED DESIGN, NOT TO BE FROZEN IN THIS REFACTOR.** Recorded
> here so the protection-vs-scheduling split, gate profiles, fault semantics, and
> tagging substrate have a fixed design (work items D3–D4). Independent of track 1;
> sequence on its own go/no-go. Read the prose below as the *target*, not as work
> scoped for this change. The migrating `GATE_CALL` described here is the **`call`
> verb** of [`unified_endpoint_ipc.md`](unified_endpoint_ipc.md) — the two docs
> specify one mechanism (the synchronous, realtime, flush-free face of `send`+`recv`);
> keep them in sync.

### The key: protection context vs scheduling context

Unix conflates them; LNP64 splits them, and that split resolves every hard IPC
question:

> A **protection context** is a domain (memory + capabilities, including its limit
> and any domain reservation — budget senses 1–2 above). A **scheduling context**
> is a **CPU-time budget + priority** (the thread's `SCHED_CLASS` allocation —
> budget sense 3, *not* the domain's limit or reservation). A migrating `GATE_CALL`
> switches the protection context, but the thread **carries its scheduling context
> with it.** Faults attribute to the protection context (the server); CPU time
> attributes to the scheduling context (the client). The domain's limit and
> reservation, by contrast, stay charged to whichever domain *creates/holds* a
> resource and never migrate with the call.

### CPU-time budget: the client is charged (migrating); dispatched is the opt-out

(This section is about the **scheduling-context** CPU budget — budget sense 3, not
the domain's limit or reservation; "budget" below means CPU time + priority.)

- **Migrating gate (fast path):** the server is passive — code+caps, no scheduling
  context. The thread runs server code **on the caller's budget and priority**
  (this *is* the bandwidth/priority inheritance). Charging the server would
  reintroduce priority inversion and force every server to size a budget for
  aggregate client demand. User model: *calling a service spends your own time,
  like a function call.*
- **Dispatched gate (opt-out):** server has its own thread + budget; charged to
  the server. Use when you want cost isolation, protection from client
  exhaustion, or async/background work.

### Faults: contained to the server; the client never dies

This section describes the **migrating** gate (the thread is in-flight inside the
server, so a fault converts an active activation into a failed return). The
**dispatched** gate differs: the server runs on its own thread/budget, so a server
fault is not an in-flight unwind of the caller — it surfaces to the client as a
failed/aborted request (or timeout) on the pending reply, and is handled by the
server's own fault gate / supervisor as below. The containment guarantee (client
never dies, transient caps revoked) holds in both modes; only the unwind mechanism
differs.

> A fault while migrated into the server is attributed to the **server domain** and
> converts the in-flight `GATE_CALL` into a **failed return to the client** —
> exactly as if the server had `GATE_RETURN`ed an error. The client's thread and
> continuation (saved at the gate) are preserved; the client just sees an error
> (`ESRVFAULT`-class). *A crashed RPC server returns an error to its caller; the
> caller does not crash.*

- **Handler:** if the server registered a fault gate (its supervisor/debugger), the
  fault is delivered there (log/restart). Default = fail the call + mark the
  server domain faulted. The eject-call-vs-restart-server choice is server policy;
  hardware guarantees only containment + a defined error to the client.
- **Cleanup:** caps / buffer mappings transiently granted to the server for this
  call are revoked on the failed return (`forced_cancel` / `synchronous_quiesce`
  revocation classes) — a crashed server cannot retain the client's buffer.
- **Server-internal consistency** is the server's burden (structure critical
  sections, or opt to restart-whole-server). This is the driver-restart model that
  makes isolated drivers practical: a buggy driver faults → callers get errors →
  supervisor restarts it → nothing else dies.

### Tag exhaustion (ASID / domain tags): recycle by shoot-down

Tags are a finite hardware *cache* of recently-active domains over a DDR-backed
universe of millions. On exhaustion: pick a victim and **shoot down its TLB +
capability-cache entries** (bounded flush-by-tag), then reassign. Refinements:

- **Generation-guarded.** Identity is (id, generation); the short tag is a recycled
  handle; generation catches any stale reference racing the recycle.
- **Partition the tag space:** a **pinned pool for realtime domains**
  (`RESERVATION`/`FIXED_PRIO` — never shot down under them, so RT IPC stays
  predictable) and a **recyclable pool for best-effort** timeshare domains. A
  pinned tag is a reserved resource exactly like CPU utilization: **reservation
  admission also reserves a pinned tag**, and if the pinned pool has none free the
  reservation is **refused** down the same fail-closed path as utilization
  admission. Because the pinned pool is never overcommitted, it never needs
  shoot-down; all shoot-down churn is confined to the recyclable best-effort pool.
- **Hardware-managed / software-invisible** (like ASIDs). User code sees only
  domain capabilities, never tags.

### Flush-free gate calls (design rule)

With **(a) PIPT caches, (b) a fully tagged TLB, and (c) the in-order
non-speculative core**, a normal migrating `GATE_CALL` needs **no flush** — only
refills (normal misses, not flushes). The narrow unavoidable exceptions:

- **Tag recycling under exhaustion** (above) — bounded victim shoot-down,
  *avoidable for RT via the pinned pool*.
- **Concurrent revocation** of a capability involved in the call
  (`synchronous_quiesce`) — a revocation event, not the call.

Convergence worth recording: the in-order core (chosen for WCET) also **eliminates
the Spectre-class predictor flush** across a trust boundary that normally makes
secure domain crossing expensive. If speculation is ever added, predictor
*partitioning by domain tag* (or a flush) returns as both a realtime and a
security cost — another reason V1 stays in-order. `design.md` line-635's
"TLB/I-cache invalidations before resume" must be scoped to **exec-replace and
revocation**, not normal gate calls.

### Nested migration (Client → FS → Disk) + the return stack

The same migrating thread hops protection contexts; manage the chain with a
**protected per-thread activation (continuation) stack**, owned by the Gate Engine
and **unforgeable by called domains**:

- Each `GATE_CALL` pushes a frame `{caller domain tag, return PC, saved
  message-register window, one-shot reply token, scheduling-context marker}`;
  `GATE_RETURN` pops one frame — **O(1)**. The stack lives in memory the
  scheduling context owns, writable only by the Gate Engine, so a callee cannot
  tamper with the return path or see frames above it.
- **Depth is bounded** (configured max per thread/reservation); exceeding it
  **fails closed**. Bounding pays for three things at once: **WCET** (bounded
  unwind + bounded stack reservation), **fault containment** (a crash unwinds at
  most N frames), and **cycle/reentrancy safety** (Disk calling back into FS
  cannot recurse unbounded).
- **Faults unwind frame-by-frame:** a fault in Disk returns an error to FS, which
  may handle or propagate it up — nested service errors propagate like nested
  function-call error returns; the client sees an error only if no level handled
  it.
- **CPU-time budget is charged transitively** along the chain (the client's
  scheduling-context reservation pays for FS *and* Disk), so admission/WCET
  analysis must use the **whole call
  chain's demand**, not just the first hop. Where per-server budgeting is wanted
  instead, use a **dispatched** gate at that hop (which also caps migration depth
  there).

## Work items

### Now — track 1, the unification (implement + freeze)

N1. `design.md`: replace the separate "process" notion with "a process is a
    Resource Domain holding an address-space capability + threads"; define the
    uniform node and its held slots; state the names-are-data invariant.
N2. `hardware_design.md` + shared schema: one uniform domain record (drop any
    process/domain split); address space as a held capability; domain-keyed
    PID/pgrp/session/reaping indices marked data-only. The budget record carries
    only the **limit** (sense 1); leave the reservation slot defined-but-unpopulated
    for track 2.
N3. `formal_theorems.md` / M-series: re-confirm domain-confinement invariants hold
    when `fork`/`exec`/`signal`/`wait` are expressed as domain operations (more
    proof *leverage* — prove containment once — but real schema/Lean churn).
N4. Cost guard: a **lightweight leaf profile** so fork-heavy workloads
    (`make -j`, pipelines, fork storms) do not pay container-weight cost or exhaust
    a finite domain table; define fail-closed behavior on slot exhaustion.
N5. Cheap-leaf representation: sparse uniform-node layout, the DDR-backed Domain
    Descriptor Table with on-chip hot cache, O(1) COW fork, and budget-bounded
    `CLONE` (fork-bomb fail-closed).

### Deferred — track 2, the realtime scheduler (future intended design; do not freeze yet)

D1. `design.md` §2 + `hardware_design.md` + schema: add the per-thread `SCHED_CLASS`
    (default `TIMESHARE`), the optional domain reservation, the degenerate flat
    time-share fast path, reservation admission/fail-closed, and `GATE_CALL`
    priority/bandwidth inheritance. **Blocked on D2 — no RTL freeze before the proof.**
D2. `formal_theorems.md` / `formal_rtl_codesign_roadmap.md`: realtime-contract
    scoping, the two-direction isolation theorem, and the compositional
    schedulability proof (the hard realtime frontier). **Must land before any RTL
    freeze of D1** — freezing a guarantee in silicon before its proof closes is the
    one risk this scoping exists to avoid.

### Deferred — track 3, the migrating-IPC microarchitecture (future intended design; do not freeze yet)

D3. IPC microarchitecture: protection-vs-scheduling-context split; migrating vs
    dispatched gate profiles; client-charged budget on migrating calls; fault →
    failed-return-to-client with server-supervisor restart and transient-cap
    revocation; the protected per-thread activation stack with bounded depth.
D4. Tagging substrate: tagged TLB + domain-tagged capability caches, ASID/domain
    tag recycle by shoot-down, pinned-vs-recyclable tag pools, PIPT caches +
    in-order core = flush-free gate fast path. Scope `design.md` line-635
    invalidations to exec-replace/revocation only.

## Non-goals

- Not moving any frozen POSIX mechanism into the OS/personality.
- Not making threads into domains (threads stay the scheduling axis).
- Not adding ambient (capability-free) authority via PIDs/pgrps.
- Not making bandwidth reservation the default or the meaning of every budget —
  it is one opt-in per-thread/per-domain mode; time-share is the default.
- Not charging the server on a migrating gate (that reintroduces priority
  inversion); not letting a server fault crash its client; not exposing
  ASID/domain tags to software.
- Not freezing the scheduler model (track 2) or the IPC microarchitecture (track 3)
  in this refactor — both are future intended design, captured here for a fixed
  vocabulary but gated on their own go/no-go (and, for track 2, a
  compositional-schedulability proof before any RTL).
