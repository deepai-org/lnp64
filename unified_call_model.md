# LNP64 Unified Call Model — one `call`, from function to RPC, on one continuation stack

Status: **Phase 4 — future intended design, NOT frozen.** This is the capstone of the
"everything is an endpoint" thesis in [`unified_object_model.md`](unified_object_model.md):
it erases the last special case — the line between an *intra-thread function call* and an
*inter-domain RPC* — by making both the **same `call` verb to an endpoint**, where the
endpoint's *distance* (its `Backing × Producer × ProtectionContext` type) dictates the cost.

> **NOT FROZEN. Phase 4, gated.** It touches the deepest part of the core (the
> continuation stack becomes a first-class protected structure). It must not be frozen
> before its own proofs land (continuation-stack integrity, the WCET bound, register-window
> non-leak). Recorded now so the design is fixed and the instruction-count win is on record.
> Current freeze scope is the endpoint work (EP-I / M16) and the domain track — not this.

## The thesis

> **`call` and `return` are the only control-transfer verbs. A "function", a "syscall", an
> "RPC", and a "coroutine resume" are one operation at four endpoint-distances, riding one
> unforgeable continuation stack.** The microarchitecture pays only for the distance the
> endpoint's type demands.

Steele's Lambda Papers (1976) already said it: *a procedure call is a message send is a
GOTO that passes arguments and optionally remembers a continuation.* Capabilities make it
safe; this machine puts it in silicon.

## The one verb, four distances

`call ep` (direct: PC-relative immediate, like `jal`; or indirect: an endpoint handle in a
register, like `jalr`). The endpoint type selects the mechanism — exactly as "send to a
Memory endpoint queues, send to a Thread endpoint rendezvouses" already does:

| Endpoint distance | What it is | Cost |
| --- | --- | --- |
| same thread, **same** context, local code | an ordinary **function call** | same tag → no flush, registers shared, ~1 cycle |
| same thread, **different** context | a **migrating gate** (syscall/driver RPC) | tag switch, message-window, cap check |
| **different** thread | a **dispatched RPC** | enqueue + wake the server thread |
| a **suspended** continuation | coroutine / async resume / `setjmp` restore | swap/restore a continuation cap |

The endpoint of a same-context call *is the gate endpoint* of `unified_object_model.md`;
the only difference is that its protection-context tag equals the caller's, so the hardware
takes the fast path. One model, one verb.

### Decision: always emit the unified `call`, even for known-local/direct

Per the explicit goal of **fewer instructions** and maximal uniformity, the compiler emits
the unified `call`/`return` for *every* call site — including statically-known direct local
calls — rather than a separate `jal` fast-encoding. We accept the per-call continuation-stack
push/pop everywhere in exchange for: (a) the instruction-count win below applying to *all*
code, (b) ROP/JOP immunity on *every* return, and (c) zero special cases in the backend.
(A later optimization may reintroduce a fused local-encoding if profiling demands it; not now.)

## The prize: one protected continuation stack subsumes six mechanisms

Making `call` push a return continuation onto a **hardware-managed, generation-guarded,
bounded protected stack** is the move. That single structure then *is*:

- the **call stack** (return addresses),
- the **shadow stack** (return-address integrity — but native, not bolted on),
- **exception unwinding** (the `.eh_frame` walk),
- the **gate activation stack** (cross-domain RPC),
- the **signal-frame stack**,
- **coroutines / async / `setjmp` / generators** (a continuation is a first-class capability
  you can suspend, resume, or *post to an endpoint*).

We already built **two instances** of exactly this — the gate continuation
(`lnp64_gate_continuation_t`) and the per-thread signal-frame stack — and M2 already proves
them compatible (`signal_compatibility_ok`). This unifies them into one and extends it to
every call. It is not greenfield; it is "the continuation stack you already have, generalized."

A continuation frame holds `{return PC, caller protection-context tag, scheduling-context
marker, generation, register-window descriptor}` — the gate frame, generalized. `call`
pushes one (same-context push reuses the caller's tag); `return` pops one. O(1) each,
unforgeable by callees, depth-bounded → fail-closed on overflow.

## Why it is *fewer instructions* (the stated goal)

The win comes from where the return address lives.

- **Today (v2 ABI):** `r1` is a dedicated, *reserved* link register. Every non-leaf function
  spills it — prologue `sd r1,[sp,…]`, epilogue `ld r1,[sp,…]` then `ret` — so **2
  instructions per non-leaf function** go to return-address handling, and `r1` is unavailable
  to the allocator.
- **Unified:** the return address lives only on the protected continuation stack, never in a
  GPR. So:
  - the `sd r1`/`ld r1` pair **vanishes from every non-leaf prologue/epilogue** (−2 instr each);
  - `r1` is **reclaimed** as a general register → less spill pressure → fewer spill/reload
    instructions everywhere;
  - call sites and returns stay **1 instruction** (`call sym` ≈ `jal`, `return` ≈ `ret`);
  - **OS code shrinks more:** syscalls and RPC *become* `call` to a cross-context endpoint, so
    the trap-setup / arg-marshal / kernel save-restore sequences collapse into one `call`.

Estimate: low-single-digit % shorter in compute-heavy userspace (loops rarely call),
noticeably more in call/syscall/RPC-dense OS code. Leaf functions are unchanged.

**Honest caveat (cycles, not instructions):** today's GPR-link spills `r1` *once* and
amortizes it over many calls; the continuation stack pushes per call and pops per return. So
call-heavy code does *more memory ops* while running *fewer instructions*. Mitigation: the
continuation stack is a small, hot, on-chip structure (a purpose-built shadow stack, not the
DRAM-backed data stack) — each push/pop is ~1 cycle and never misses — and we **replace** the
software return address rather than duplicating it (unlike CET, which keeps both). Net: fewer
instructions, bounded/deterministic latency. Right tradeoff for an in-order realtime machine.

## Arguments are the message (and same-context pays nothing for it)

A function's argument list *is* a `(bytes, caps)` message. For a **same-context** endpoint the
message **is the register file** — zero copy, zero marshal, the ordinary calling convention;
the message abstraction only materializes (a window + grant) for cross-context calls, where
you would marshal anyway today. Capability arguments to a function (passing an fd/endpoint)
become first-class and *identical* to cap-passing over RPC (`SCM_RIGHTS`). One calling
convention, from `add(a,b)` to a cross-machine call.

## The teeth (why this is worth the silicon, beyond elegance)

- **ROP/JOP immunity on every call**, not just gates: no forgeable return address ever lives
  in writable memory.
- **One fault-unwinding mechanism**: a fault unwinds the continuation stack frame-by-frame
  whether frames are local calls or RPC hops — C++ exceptions, signal unwinding, and "a
  crashed RPC server returns an error to its caller" become *the same* unwind.
- **Native cross-domain backtraces**: walk one stack from `main()` straight through a gate
  into the FS server. Distributed stack traces in hardware.
- **Coroutines/async/`setjmp` as continuation ops**: async/await = "post my continuation to a
  Memory endpoint" (the ring); a coroutine switch = swap continuation caps; the gate's
  **handoff** mode = transfer a continuation. The whole concurrency zoo collapses into the
  object model.
- **Bounded stack depth for free**: the continuation stack's depth bound (already specified
  for gates) makes deep recursion fail closed — a realtime virtue.

## Honest costs & constraints

- **The hardware continuation stack is the load-bearing commitment** — a shadow stack with a
  write per call. Proven feasible (Intel CET), but a real microarch cost; keep it on-chip and
  small.
- **Bounded stack depth** changes unbounded-recursion software (must bound or heap-allocate an
  explicit continuation). Acceptable — even desirable — on this machine; a change nonetheless.
- **Register-window discipline is part of the endpoint type:** a same-context call shares the
  whole register file; a cross-context call clears all but the message window (no register
  leak across a protection boundary). The non-leak must be proven.
- **Keep the model semantic, not a runtime indirection:** even with the always-unified
  encoding, the *target* of a direct call is a static PC-relative immediate — `call` is not a
  dynamic endpoint lookup. The uniformity is in the model and the continuation stack, not an
  indirection on the hot path.

## Precedent (a convergence, not an invention)

Lambda Papers (call = send = goto-with-args); the Actor model; CHERI `CInvoke`
(domain-crossing call); the Mill's portal calls; Intel CET shadow stacks; the Burroughs
B5000 and iAPX-432 call gates. We sit at their centroid, with capabilities making it safe and
a continuation stack making it uniform.

## Relationship to the rest of the machine

- The same-context call's endpoint **is** the gate endpoint of `unified_object_model.md` §5;
  the continuation stack **is** the gate+signal continuation we already built (M2).
- `gate_call`/`gate_return` become the cross-context *cases* of `call`/`return` — not separate
  verbs. The four IPC/async verbs stay `send`/`recv`/`call`/`wait`; this just reveals that
  `call` was always the universal control-transfer verb.

## Non-goals

- Not freezing any of this now — Phase 4, gated on its own proofs.
- Not a dynamic endpoint lookup on the function-call hot path (the encoding stays direct).
- Not abandoning bounded stack depth for unbounded recursion — bounded is the point.
- Not exposing the continuation stack as forgeable software memory — it is hardware-owned.

## Work items (when scheduled — gated; do not freeze before the proofs)

C1. Generalize `lnp64_gate_continuation_t` + the signal-frame stack into **one** protected
    continuation stack; define the frame, the depth bound, the generation guard, the
    fail-closed overflow.
C2. Define `call`/`return` over endpoints (direct PC-rel + indirect handle forms); show the
    same-context fast path = no tag switch, registers shared, on-chip push/pop.
C3. Compiler: emit unified `call`/`return` for **all** call sites; drop the reserved link
    register (`r1` reclaimed), delete the prologue/epilogue ra spill/restore; measure the
    instruction-count delta on the OS + userspace corpus (target: net fewer).
C4. Proofs: continuation-stack integrity (returns land only on pushed frames; unforgeable),
    the WCET bound (bounded depth + on-chip push/pop ⇒ bounded call/return latency), and the
    cross-context register-window non-leak. **All before any RTL freeze.**
C5. Re-express signals, exceptions, coroutines/async, and `setjmp`/`longjmp` as continuation
    operations; confirm each is fewer instructions / one mechanism.
