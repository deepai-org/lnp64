/- LNP64 EP-F: bounded Memory-backed endpoint — the freeze gate for the
   unified `send`/`recv`/`wait` verbs (unified_object_model.md §3, §10).

   Two obligations the design names as the gate before any RTL freeze:

   1. **Bounded latency / fail-closed.** A Memory-backed endpoint is a
      fixed-depth queue. `send` is one bounded step: enqueue when there is room,
      else fail closed with EAGAIN (never an unbounded block). `recv` is one
      bounded step: dequeue when non-empty, else EAGAIN. Depth never exceeds the
      frozen capacity, so draining all messages takes at most `capacity` recv
      steps — a deterministic WCET, with queue non-determinism contained as a
      memory abstraction rather than baked into instruction timing.

   2. **Capability-safety (names-are-data).** A message names capabilities by
      index into the *sender's* cap table. An index the sender does not hold, or
      a revoked cap, resolves to nothing — a message can forge no authority the
      sender lacks. The engine installs a received cap into the *receiver's*
      table carrying authority verbatim or narrowed, never amplified.
-/

namespace Lnp64.EPEndpoint

def eagain : Nat := 11

/-- A Memory-backed endpoint: a fixed-depth message queue. -/
structure Endpoint where
  capacity : Nat
  depth : Nat
deriving Repr, DecidableEq

/-- `send`: enqueue when there is room, else fail closed. Returns the next state
    and a status (0 ok / EAGAIN). A single bounded step. -/
def epSend (e : Endpoint) : Endpoint × Nat :=
  if e.depth < e.capacity then ({ e with depth := e.depth + 1 }, 0) else (e, eagain)

/-- `recv`: dequeue when non-empty, else fail closed. A single bounded step. -/
def epRecv (e : Endpoint) : Endpoint × Nat :=
  if 0 < e.depth then ({ e with depth := e.depth - 1 }, 0) else (e, eagain)

/- ---- Obligation 1: bounded latency / fail-closed ---------------------- -/

theorem ep_send_full_fails_closed (e : Endpoint) (h : e.depth = e.capacity) :
    (epSend e).2 = eagain ∧ (epSend e).1 = e := by
  unfold epSend
  have : ¬ e.depth < e.capacity := by omega
  simp [this]

theorem ep_send_room_enqueues (e : Endpoint) (h : e.depth < e.capacity) :
    (epSend e).2 = 0 ∧ (epSend e).1.depth = e.depth + 1 := by
  unfold epSend; simp [h]

theorem ep_recv_empty_fails_closed (e : Endpoint) (h : e.depth = 0) :
    (epRecv e).2 = eagain ∧ (epRecv e).1 = e := by
  unfold epRecv
  have : ¬ 0 < e.depth := by omega
  simp [this]

theorem ep_recv_nonempty_dequeues (e : Endpoint) (h : 0 < e.depth) :
    (epRecv e).2 = 0 ∧ (epRecv e).1.depth = e.depth - 1 := by
  unfold epRecv; simp [h]

/-- Depth never exceeds the frozen capacity — preserved by both verbs. This is
    the WCET hook: the queue can hold at most `capacity` messages, so a full
    drain is bounded by `capacity` `recv` steps. -/
theorem ep_send_preserves_bound (e : Endpoint) (h : e.depth ≤ e.capacity) :
    (epSend e).1.depth ≤ e.capacity := by
  unfold epSend; split <;> simp <;> omega

theorem ep_recv_preserves_bound (e : Endpoint) (h : e.depth ≤ e.capacity) :
    (epRecv e).1.depth ≤ e.capacity := by
  unfold epRecv; split <;> simp <;> omega

/- ---- Obligation 2: capability-safety (names-are-data) ------------------ -/

/-- Resolve a message cap-handle (an index into the *sender's* table of `len`
    held caps). Succeeds only if the index is in range and the cap is live;
    otherwise `none` — the message can name nothing the sender does not hold. -/
def resolve (len : Nat) (idx : Nat) (rights : Nat) (revoked : Bool) : Option Nat :=
  if idx < len ∧ revoked = false then some rights else none

/-- The engine installs a received cap, carrying authority verbatim or narrowed
    by the receiver's request — never amplified. -/
def install (sourceRights requestedRights : Nat) : Nat :=
  Nat.min sourceRights requestedRights

/-- Out-of-range handle: a message cannot forge a cap the sender does not hold. -/
theorem cap_no_forge_out_of_range
    (len idx rights : Nat) (revoked : Bool) (h : len ≤ idx) :
    resolve len idx rights revoked = none := by
  unfold resolve
  have : ¬ idx < len := by omega
  simp [this]

/-- A revoked cap resolves to nothing (regardless of index). -/
theorem cap_no_forge_revoked (len idx rights : Nat) :
    resolve len idx rights true = none := by
  unfold resolve; simp

/-- A live, in-range handle resolves to exactly the sender's rights — no more. -/
theorem cap_resolve_exact (len idx rights : Nat) (h : idx < len) :
    resolve len idx rights false = some rights := by
  unfold resolve; simp [h]

/-- No amplification: the installed authority never exceeds the source's. -/
theorem cap_install_no_amplify (sourceRights requestedRights : Nat) :
    install sourceRights requestedRights ≤ sourceRights := by
  unfold install; exact Nat.min_le_left _ _

/- ---- Concrete witnesses (decidable sanity) ----------------------------- -/

/-- A depth-2 ring: two sends fill it, the third fails closed; two recvs drain. -/
def ring0 : Endpoint := { capacity := 2, depth := 0 }

example : (epSend ring0).1.depth = 1 := by decide
example : (epSend (epSend ring0).1).1.depth = 2 := by decide
example : (epSend (epSend (epSend ring0).1).1).2 = eagain := by decide
example : resolve 3 5 7 false = none := by decide   -- forge attempt blocked
example : resolve 3 1 7 false = some 7 := by decide  -- held cap resolves
example : install 7 3 = 3 := by decide               -- narrowed, not amplified

end Lnp64.EPEndpoint
