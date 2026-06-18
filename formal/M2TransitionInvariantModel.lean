/- LNP64 M2 transition-invariant model.

`M2GateModel.lean` is the bounded gate/continuation witness used by the RTL
smoke trace. This file adds the transition-proof slice: typed protocol state,
typed operations, a `Step` relation, reachability from reset, preservation, and
reachable-state theorems for continuation, gate delivery, fault delivery, and
signal-compatibility safety.
-/

namespace Lnp64.M2Transition

inductive Location
  | runnable
  | running
  | parked
deriving DecidableEq, Repr

structure Continuation where
  id : Nat
  generation : Nat
  valid : Bool
deriving DecidableEq, Repr

structure Thread where
  tid : Nat
  location : Location
deriving DecidableEq, Repr

structure State where
  booted : Bool
  caller : Thread
  callee : Thread
  continuation : Continuation
  syncAccepted : Bool
  syncRoundtrip : Bool
  asyncDelivered : Bool
  handoffDelivered : Bool
  staleChecked : Bool
  staleRejected : Bool
  faultDeliveryChecked : Bool
  deliveredFaults : Nat
  signalDelivered : Bool
  signalCreatesAuthority : Bool
  signalMaskBypassed : Bool
deriving DecidableEq, Repr

inductive Op
  | boot
  | syncCall
  | syncReturn
  | asyncCall
  | handoffCall
  | rejectStaleContinuation
  | faultDelivery
  | signalCompatibilityDelivery
deriving DecidableEq, Repr

def caller0 : Thread :=
  { tid := 1, location := Location.runnable }

def callee0 : Thread :=
  { tid := 2, location := Location.runnable }

def continuation0 : Continuation :=
  { id := 0, generation := 0, valid := false }

def reset : State :=
  { booted := false
    caller := caller0
    callee := callee0
    continuation := continuation0
    syncAccepted := false
    syncRoundtrip := false
    asyncDelivered := false
    handoffDelivered := false
    staleChecked := false
    staleRejected := false
    faultDeliveryChecked := false
    deliveredFaults := 0
    signalDelivered := false
    signalCreatesAuthority := false
    signalMaskBypassed := false }

def continuationUniqueState (s : State) : Prop :=
  s.continuation.valid = true -> s.continuation.id > 0

def syncRoundtripState (s : State) : Prop :=
  s.syncAccepted = true ->
    s.continuation.valid = true \/ s.syncRoundtrip = true

def syncReturnWakesCallerState (s : State) : Prop :=
  s.syncRoundtrip = true -> s.caller.location = Location.runnable

def asyncDeliveryDoesNotParkCallerState (s : State) : Prop :=
  s.asyncDelivered = true -> s.caller.location ≠ Location.parked

def handoffDeliveryRecordedState (s : State) : Prop :=
  s.handoffDelivered = true -> s.booted = true

def staleContinuationRejectedState (s : State) : Prop :=
  s.staleChecked = true -> s.continuation.valid = false -> s.staleRejected = true

def faultDeliveryGateEnteredState (s : State) : Prop :=
  s.faultDeliveryChecked = true -> s.deliveredFaults > 0

def signalCompatibilitySafeState (s : State) : Prop :=
  s.signalDelivered = true ->
    s.signalCreatesAuthority = false /\ s.signalMaskBypassed = false

def invariant (s : State) : Prop :=
  continuationUniqueState s /\
  syncRoundtripState s /\
  syncReturnWakesCallerState s /\
  asyncDeliveryDoesNotParkCallerState s /\
  handoffDeliveryRecordedState s /\
  staleContinuationRejectedState s /\
  faultDeliveryGateEnteredState s /\
  signalCompatibilitySafeState s

inductive Step : State -> Op -> State -> Prop
  | boot (s : State) :
      s.booted = false ->
      s.syncAccepted = false ->
      s.asyncDelivered = false ->
      s.handoffDelivered = false ->
      s.staleChecked = false ->
      s.faultDeliveryChecked = false ->
      s.signalDelivered = false ->
      Step s Op.boot { s with booted := true }
  | syncCall (s : State) :
      s.booted = true ->
      s.continuation.valid = false ->
      s.syncRoundtrip = false ->
      s.asyncDelivered = false ->
      s.caller.location = Location.runnable ->
      s.callee.location = Location.runnable ->
      Step s Op.syncCall
        { s with
          caller := { s.caller with location := Location.parked }
          callee := { s.callee with location := Location.running }
          continuation := { id := 1, generation := s.continuation.generation + 1, valid := true }
          syncAccepted := true }
  | syncReturn (s : State) :
      s.continuation.valid = true ->
      s.continuation.id > 0 ->
      s.staleChecked = false ->
      Step s Op.syncReturn
        { s with
          caller := { s.caller with location := Location.runnable }
          callee := { s.callee with location := Location.runnable }
          continuation := { s.continuation with generation := s.continuation.generation + 1, valid := false }
          syncRoundtrip := true }
  | asyncCall (s : State) :
      s.booted = true ->
      s.continuation.valid = false ->
      s.caller.location ≠ Location.parked ->
      Step s Op.asyncCall
        { s with asyncDelivered := true }
  | handoffCall (s : State) :
      s.booted = true ->
      s.continuation.valid = false ->
      Step s Op.handoffCall
        { s with
          callee := { s.callee with location := Location.running }
          handoffDelivered := true }
  | rejectStaleContinuation (s : State) :
      s.booted = true ->
      s.continuation.valid = false ->
      Step s Op.rejectStaleContinuation
        { s with
          callee := { s.callee with location := Location.runnable }
          staleChecked := true
          staleRejected := true }
  | faultDelivery (s : State) :
      s.booted = true ->
      Step s Op.faultDelivery
        { s with
          faultDeliveryChecked := true
          deliveredFaults := s.deliveredFaults + 1 }
  | signalCompatibilityDelivery (s : State) :
      s.booted = true ->
      Step s Op.signalCompatibilityDelivery
        { s with
          signalDelivered := true
          signalCreatesAuthority := false
          signalMaskBypassed := false }

inductive Reachable : State -> Prop
  | reset : Reachable reset
  | step {s t : State} {op : Op} :
      Reachable s -> Step s op t -> Reachable t

theorem invariant_reset :
    invariant reset := by
  simp [
    invariant, reset, caller0, callee0, continuation0,
    continuationUniqueState, syncRoundtripState, syncReturnWakesCallerState,
    asyncDeliveryDoesNotParkCallerState, handoffDeliveryRecordedState,
    staleContinuationRejectedState, faultDeliveryGateEnteredState,
    signalCompatibilitySafeState
  ]

theorem invariant_step {s t : State} {op : Op} :
    invariant s -> Step s op t -> invariant t := by
  intro hInv hStep
  cases hStep <;>
    simp_all [
      invariant, continuationUniqueState, syncRoundtripState,
      syncReturnWakesCallerState, asyncDeliveryDoesNotParkCallerState,
      handoffDeliveryRecordedState, staleContinuationRejectedState,
      faultDeliveryGateEnteredState, signalCompatibilitySafeState
    ]

theorem reachable_invariant {s : State} :
    Reachable s -> invariant s := by
  intro hReach
  induction hReach with
  | reset => exact invariant_reset
  | step hPrev hStep ih => exact invariant_step ih hStep

theorem m2_t3_continuation_unique_for_all_reachable {s : State} :
    Reachable s -> continuationUniqueState s := by
  intro hReach
  exact (reachable_invariant hReach).1

theorem m2_t3_sync_roundtrip_or_live_continuation_for_all_reachable {s : State} :
    Reachable s -> syncRoundtripState s := by
  intro hReach
  exact (reachable_invariant hReach).2.1

theorem m2_t3_sync_return_wakes_caller_for_all_reachable {s : State} :
    Reachable s -> syncReturnWakesCallerState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.1

theorem m2_t3_async_delivery_does_not_park_caller_for_all_reachable {s : State} :
    Reachable s -> asyncDeliveryDoesNotParkCallerState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.1

theorem m2_t3_handoff_delivery_recorded_for_all_reachable {s : State} :
    Reachable s -> handoffDeliveryRecordedState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.1

theorem m2_t3_stale_continuation_rejected_for_all_reachable {s : State} :
    Reachable s -> staleContinuationRejectedState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.2.1

theorem m2_t3_fault_delivery_gate_entered_for_all_reachable {s : State} :
    Reachable s -> faultDeliveryGateEnteredState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.2.2.1

theorem m2_t3_signal_compatibility_safe_for_all_reachable {s : State} :
    Reachable s -> signalCompatibilitySafeState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.2.2.2

end Lnp64.M2Transition
