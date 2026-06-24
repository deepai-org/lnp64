# LNP64 Unified Memory Model — memory is an endpoint (and `design.md` already says so)

Status: **reconciliation, not a new proposal.** This doc does **not** add a mechanism. It
names what `design.md` §4/§6/§7 and [`unified_object_model.md`](unified_object_model.md)
*already* imply as **one model**, and retires the vocabulary scatter that currently spreads
the same idea across six sections under different names. The fast path stays a plain `LD`/`ST`
(zero performance change); the slow path is the **Object-Backed Page Transaction Protocol**
that `design.md` already specifies. Memory was an endpoint all along — we just had two
vocabularies for it.

## The thesis (one sentence)

> A `LD`/`ST` is the **fast path of `recv`/`send` on a Memory-backed endpoint**: when the
> page is `RESIDENT`, it's a 1-cycle load; when it's `NONRESIDENT_OBJECT`, the faulting
> thread **parks** (a blocking `recv`), the owning object/service **sends** the page (the
> Object-Backed Page Transaction), and the thread resumes. Same object family, same readiness
> machine, same Gate/Continuation Engine — `design.md` already builds all of it.

## Rosetta stone: `design.md` term ↔ endpoint term

The two vocabularies describe the *same* hardware. This is the scatter, named once:

| `design.md` (§) | `unified_object_model.md` | What it is |
| --- | --- | --- |
| `memory_object` (`OBJECT_CTL` profile, §2) | a **Memory-backed endpoint** | the object a mapping/load reads from |
| **Object-Backed Page Transaction Protocol** (§4) | `send`(page-request) → `recv`(page) | demand-fill = a bounded request/reply over the endpoint |
| page states `NONRESIDENT_OBJECT` / `FILL_PENDING` / `RESIDENT_*` (§4) | endpoint **readiness**: not-ready / recv-in-flight / ready | the endpoint's queue/readiness state machine |
| faulting `LD` **parks** the thread, no kernel trap (§6) | a **blocking `recv`** that resumes | precise, restartable fault = the recv unblock |
| `MMAP` object-backed mapping (§4) | **map an endpoint** into the address space | binds the endpoint to a VA range |
| `pcie_bar` `MMAP` + plain `LD`/`ST` (§7) | a **device-backed** Memory endpoint, load fast-path | MMIO is memory with a *device producer*, **not** a message verb |
| `DMA_CTL` + `dma_completion` (§5) | bulk `send`/`recv` over endpoints + a **completion endpoint** | device-producer transfer + event/counter completion |
| Gate/Continuation Engine: call/fault/timer/signal (§5) | one **continuation stack** for all upcalls | the fault/recv-unblock and the gate share it |
| `ALLOC` heap engine (§4) | a process-local Memory endpoint with a hardware producer | anonymous memory is the zero/COW producer |

**Takeaway:** the Object-Backed Page Transaction Protocol is the load-bearing 90% of
"memory = endpoint," and it is **already in silicon-design**. Adopting the endpoint
vocabulary changes *no mechanism* — it removes the scatter and connects §4 to the endpoint
model so future docs stop re-deriving it.

## `Backing × Producer` is just `design.md`'s object-profile list, factored

`unified_object_model.md` types endpoints by `Backing {Thread, Memory, Register} × Producer
{software, hardware}`. `design.md`'s `OBJECT_CTL` profiles are points in that grid — two
lists for one taxonomy:

| `design.md` profile | Backing | Producer |
| --- | --- | --- |
| `memory_object`, `queue`, `pipe` | **Memory** | software (or hardware for anon zero/COW) |
| `counter`, `event/completion` | **Register** | software or hardware |
| `call_gate` | **Thread** | software |
| `timer`, `dma_completion`, `irq_event` | Register/Memory | **hardware** |
| `pcie_bar` (MMIO) | **Memory** (device) | **hardware** (the device) |

So "memory_object vs counter vs call_gate vs pcie_bar" isn't four ad-hoc modules (as §2 reads
today) — it's four cells of one `Backing × Producer` type, which `design.md` line 246 already
hints at ("…are runtime or acceleration profiles over these primitives, not separate ad hoc
hardware modules").

## The page fault, stated once

A demand fault is the endpoint mechanism, not a special case:

1. VA in a `NONRESIDENT_OBJECT` page → the load is a `recv` on that page's Memory endpoint.
2. The faulting thread **parks** (`FILL_PENDING`); the Page/VMA Engine `send`s a bounded,
   capability-authorized page-request to the owning object/service.
3. The owner replies `page` / `zero` / `shared` / `error` / `retry`; hardware installs only
   if VMA-gen, object-gen, lineage-epoch, perms, memory-type, and domain policy still match.
4. Thread resumes; the load re-runs against the now-`RESIDENT` page (a plain load).

This is verbatim `design.md` §4 — re-expressed as `recv`/`send`. The **in-order,
non-speculative core** (chosen for WCET) is what makes step 4 clean: architectural state at
the faulting load is already precise, so "resume and re-run the load" needs no squash
machinery. The resume point lives on the same continuation stack as gates and signals.

**Honest latency note (carry it, don't bury it):** a page-endpoint whose producer is a
*userspace* service (file-backed mmap, the rump fs) is **unbounded** — so such a thread is
best-effort, not a hard reservation, unless the pages are pinned/pre-faulted. The endpoint
model makes this *explicit and typed* (a `recv` on a software-produced Memory endpoint is
visibly unbounded; a `RESIDENT` page is a 1-cycle load) rather than hiding it in a pager.
This matches the `netbsd_rump_port_roadmap.md` R1.5 named risk exactly.

## The one genuine (small, optional) unification opportunity: servicelets as page producers

Today `servicelet_program` (§8) is a network-only concept: a bounded, verified program
attached to a classifier/queue. But the Object-Backed Page Transaction's "owning
object/service" is *exactly* a producer for a Memory-backed endpoint. So a servicelet could
generalize from "network classifier" to **"the bounded producer of a Memory endpoint's
pages"** — deterministic/cheap fills (a zero/pattern generator, a decompressor, a
checksum-verified page source) served *in hardware* with a bounded WCET, instead of a
round-trip to a userspace service. That would give a *bounded-latency* fill class (eligible
for realtime) distinct from the unbounded userspace-service class. This is the only part that
isn't already in `design.md`; it's **future, gated** (it needs the servicelet verifier's WCET
bound to extend to the fill path), and it's a natural widening of an existing primitive, not
a new module.

## What this actually changes (the action — it's small)

1. **One vocabulary.** Connect `unified_object_model.md` §2 (`Backing × Producer`) to
   `design.md` §4 (VMA/page states) via the Rosetta stone above, so the page-transaction
   protocol, `memory_object`, `MMAP`, and MMIO are documented as one endpoint model — not
   re-derived per doc. (Fixes the scatter the way the IPC merge fixed the IPC scatter.)
2. **Correct the MMIO framing** wherever the earlier "MMIO = send/recv" pitch leaked: MMIO is
   a *device-backed Memory endpoint accessed by loads* (`design.md` §7), consistent with
   "fast path = load." No message verb per register.
3. **Mark the port retrofit.** `netbsd_rump_port_roadmap.md` R1.5's file-backed-fault handshake
   **is** the Object-Backed Page Transaction = a Memory-endpoint `recv`; build it as that from
   day one so it's a substitution, not a bespoke pager. Same for the Linux external-memory-
   provider endgame (L4) — "hardware owns frames, exposed as Memory endpoints."

## Non-goals / caveats

- **No new hardware** beyond the optional servicelet-as-page-producer (gated). This is naming
  + consolidation of `design.md` §1/§2/§4/§5/§6/§7.
- **No performance change.** The fast path is and stays a plain `LD`/`ST`; the endpoint
  framing is semantic. Unmodified OS VM/VFS code above the seam can't tell.
- **DMA physics don't collapse.** Coherency, bounce buffers, scatter-gather, IOMMU scoping
  still live in the `DMA_CTL`/grant contract; the endpoint unifies the *dispatch*, not the
  device math.
- **Latency truth stays explicit**, not hidden (above).

## Cross-references

- [`unified_object_model.md`](unified_object_model.md) — endpoints + `Backing × Producer` +
  the four verbs. This doc adds the **Memory** backing's relationship to the VMA/page engine.
- [`unified_call_model.md`](unified_call_model.md) — the shared continuation stack that holds
  the faulting thread's resume point (Phase 4, not frozen).
- `design.md` §4 (Memory Management / VMAs), §6 (Load/Store + faulting-load park), §7 (MMIO),
  §5 (Gate/Continuation Engine, `DMA_CTL`), §8 (servicelets) — the existing silicon-design
  this reconciles.
- `netbsd_rump_port_roadmap.md` R1.5 + `linux_enlightened_port_roadmap.md` L4 — the ports that
  consume the page-fault-as-`recv` and the memory-endpoint frame model.
