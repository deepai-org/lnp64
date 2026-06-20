/- LNP64 M15 transition-invariant model.

`M15ObjectProfilesModel.lean` is a bounded object-profiles witness. This file
adds a transition-invariant proof slice for the SG-OBJECT guarantees: typed
state, typed operations, a `Step` relation, `Reachable`, preservation, and
theorems over arbitrary reachable states (a queue overflow is always signalled
explicitly and never silently dropped, a stale event-source generation is
always rejected, and a replayed gate continuation is always rejected).
-/

namespace Lnp64.M15Transition

structure State where
  overflowObserved : Bool
  overflowExplicit : Bool
  staleEventObserved : Bool
  staleEventRejected : Bool
  duplicateContinuationObserved : Bool
  duplicateContinuationRejected : Bool
  events : Nat
  failures : Nat
deriving DecidableEq, Repr

inductive Op
  | counterThreshold
  | queuePush
  | queueOverflow
  | eventEmit
  | rejectStaleEvent
  | rejectDuplicateContinuation
deriving DecidableEq, Repr

def reset : State :=
  { overflowObserved := false
    overflowExplicit := false
    staleEventObserved := false
    staleEventRejected := false
    duplicateContinuationObserved := false
    duplicateContinuationRejected := false
    events := 0
    failures := 0 }

def overflowSignalledExplicitly (s : State) : Prop :=
  s.overflowObserved = true -> s.overflowExplicit = true

def staleEventFailsClosed (s : State) : Prop :=
  s.staleEventObserved = true -> s.staleEventRejected = true

def gateContinuationUnique (s : State) : Prop :=
  s.duplicateContinuationObserved = true -> s.duplicateContinuationRejected = true

def invariant (s : State) : Prop :=
  overflowSignalledExplicitly s /\
  staleEventFailsClosed s /\
  gateContinuationUnique s

inductive Step : State -> Op -> State -> Prop
  | counterThreshold (s : State) :
      Step s Op.counterThreshold { s with events := s.events + 1 }
  | queuePush (s : State) :
      Step s Op.queuePush s
  | queueOverflow (s : State) :
      Step s Op.queueOverflow
        { s with overflowObserved := true, overflowExplicit := true, events := s.events + 1, failures := s.failures + 1 }
  | eventEmit (s : State) :
      Step s Op.eventEmit s
  | rejectStaleEvent (s : State) :
      Step s Op.rejectStaleEvent
        { s with staleEventObserved := true, staleEventRejected := true, failures := s.failures + 1 }
  | rejectDuplicateContinuation (s : State) :
      Step s Op.rejectDuplicateContinuation
        { s with duplicateContinuationObserved := true, duplicateContinuationRejected := true, failures := s.failures + 1 }

inductive Reachable : State -> Prop
  | reset : Reachable reset
  | step {s t : State} {op : Op} :
      Reachable s -> Step s op t -> Reachable t

theorem invariant_reset : invariant reset := by
  simp [invariant, reset, overflowSignalledExplicitly, staleEventFailsClosed,
    gateContinuationUnique]

theorem invariant_step {s t : State} {op : Op} :
    invariant s -> Step s op t -> invariant t := by
  intro hInv hStep
  cases hStep <;>
    simp_all [invariant, overflowSignalledExplicitly, staleEventFailsClosed,
      gateContinuationUnique]

theorem reachable_invariant {s : State} :
    Reachable s -> invariant s := by
  intro hReach
  induction hReach with
  | reset => exact invariant_reset
  | step _ hStep ih => exact invariant_step ih hStep

theorem m15_t3_queue_overflow_explicit_for_all_reachable {s : State} :
    Reachable s -> overflowSignalledExplicitly s := by
  intro hReach
  exact (reachable_invariant hReach).1

theorem m15_t3_stale_event_source_fails_closed_for_all_reachable {s : State} :
    Reachable s -> staleEventFailsClosed s := by
  intro hReach
  exact (reachable_invariant hReach).2.1

theorem m15_t3_gate_continuation_unique_for_all_reachable {s : State} :
    Reachable s -> gateContinuationUnique s := by
  intro hReach
  exact (reachable_invariant hReach).2.2

end Lnp64.M15Transition
