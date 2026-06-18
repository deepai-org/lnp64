/- LNP64 M7 transition-invariant model.

`M7FutexAtomicModel.lean` is a bounded futex/atomic witness. This file adds a
small transition-invariant proof slice for the waitable scheduler path: typed
state, typed operations, a `Step` relation, `Reachable`, preservation, and
theorems over arbitrary reachable states of this slice.
-/

namespace Lnp64.M7Transition

inductive Location
  | runnable
  | running
  | parked
deriving DecidableEq, Repr

structure Thread where
  tid : Nat
  location : Location
  waitGeneration : Nat
deriving DecidableEq, Repr

structure State where
  atomicWord : Nat
  atomicCount : Nat
  cmpxchgFailureExplicit : Bool
  thread : Thread
  addressGeneration : Nat
  staleAddressGeneration : Nat
  domainBudget : Nat
  waitCost : Nat
  wakePending : Bool
  futexWakeDelivered : Bool
  timerWakeDelivered : Bool
  staleAddressRejected : Bool
deriving DecidableEq, Repr

inductive Op
  | cmpxchgSuccess
  | cmpxchgFail
  | futexWait
  | futexWake
  | timerWait
  | timerExpire
  | consumeWake
  | rejectStaleAddress
deriving DecidableEq, Repr

def initialThread : Thread :=
  { tid := 2, location := Location.runnable, waitGeneration := 1 }

def reset : State :=
  { atomicWord := 0
    atomicCount := 0
    cmpxchgFailureExplicit := false
    thread := initialThread
    addressGeneration := 1
    staleAddressGeneration := 0
    domainBudget := 1
    waitCost := 1
    wakePending := false
    futexWakeDelivered := false
    timerWakeDelivered := false
    staleAddressRejected := false }

def runnableLocationCount (s : State) : Nat :=
  match s.thread.location with
  | Location.runnable => 1
  | _ => 0

def runningLocationCount (s : State) : Nat :=
  match s.thread.location with
  | Location.running => 1
  | _ => 0

def parkedLocationCount (s : State) : Nat :=
  match s.thread.location with
  | Location.parked => 1
  | _ => 0

def schedulerLocationCount (s : State) : Nat :=
  runnableLocationCount s + runningLocationCount s + parkedLocationCount s

def exactlyOneSchedulerLocationState (s : State) : Prop :=
  schedulerLocationCount s = 1

def wakeGenerationMatchesState (s : State) : Prop :=
  s.futexWakeDelivered = true \/ s.timerWakeDelivered = true ->
    s.thread.waitGeneration = s.addressGeneration

def noLostWakeupState (s : State) : Prop :=
  s.wakePending = true -> s.thread.location ≠ Location.parked

def domainBudgetEligibleState (s : State) : Prop :=
  s.waitCost <= s.domainBudget

def explicitCmpxchgFailureState (s : State) : Prop :=
  s.cmpxchgFailureExplicit = true -> s.atomicWord = 1

def invariant (s : State) : Prop :=
  exactlyOneSchedulerLocationState s /\
  wakeGenerationMatchesState s /\
  noLostWakeupState s /\
  domainBudgetEligibleState s /\
  explicitCmpxchgFailureState s

inductive Step : State -> Op -> State -> Prop
  | cmpxchgSuccess (s : State) :
      s.atomicCount = 0 ->
      Step s Op.cmpxchgSuccess
        { s with atomicWord := 1, atomicCount := 1 }
  | cmpxchgFail (s : State) :
      s.atomicCount = 1 ->
      s.atomicWord = 1 ->
      Step s Op.cmpxchgFail
        { s with atomicCount := 2, cmpxchgFailureExplicit := true }
  | futexWait (s : State) :
      s.wakePending = false ->
      Step s Op.futexWait
        { s with thread := { s.thread with
            location := Location.parked
            waitGeneration := s.addressGeneration } }
  | futexWake (s : State) :
      s.thread.location = Location.parked ->
      s.thread.waitGeneration = s.addressGeneration ->
      Step s Op.futexWake
        { s with
          thread := { s.thread with location := Location.runnable }
          wakePending := true
          futexWakeDelivered := true }
  | timerWait (s : State) :
      s.wakePending = false ->
      Step s Op.timerWait
        { s with thread := { s.thread with
            location := Location.parked
            waitGeneration := s.addressGeneration } }
  | timerExpire (s : State) :
      s.thread.location = Location.parked ->
      s.thread.waitGeneration = s.addressGeneration ->
      Step s Op.timerExpire
        { s with
          thread := { s.thread with location := Location.runnable }
          wakePending := true
          timerWakeDelivered := true }
  | consumeWake (s : State) :
      s.wakePending = true ->
      Step s Op.consumeWake { s with wakePending := false }
  | rejectStaleAddress (s : State) :
      s.staleAddressGeneration ≠ s.addressGeneration ->
      Step s Op.rejectStaleAddress { s with staleAddressRejected := true }

inductive Reachable : State -> Prop
  | reset : Reachable reset
  | step {s t : State} {op : Op} :
      Reachable s -> Step s op t -> Reachable t

theorem exactly_one_location_by_construction (s : State) :
    exactlyOneSchedulerLocationState s := by
  cases hLoc : s.thread.location <;>
    simp [
      exactlyOneSchedulerLocationState, schedulerLocationCount,
      runnableLocationCount, runningLocationCount, parkedLocationCount, hLoc
    ]

theorem invariant_reset :
    invariant reset := by
  simp [
    invariant, reset, initialThread, exactly_one_location_by_construction,
    wakeGenerationMatchesState, noLostWakeupState, domainBudgetEligibleState,
    explicitCmpxchgFailureState
  ]

theorem invariant_step {s t : State} {op : Op} :
    invariant s -> Step s op t -> invariant t := by
  intro hInv hStep
  cases hStep <;>
    simp_all [
      invariant, exactly_one_location_by_construction,
      wakeGenerationMatchesState, noLostWakeupState,
      domainBudgetEligibleState, explicitCmpxchgFailureState
    ]

theorem reachable_invariant {s : State} :
    Reachable s -> invariant s := by
  intro hReach
  induction hReach with
  | reset => exact invariant_reset
  | step hPrev hStep ih => exact invariant_step ih hStep

theorem m7_t3_exactly_one_scheduler_location_for_all_reachable {s : State} :
    Reachable s -> exactlyOneSchedulerLocationState s := by
  intro hReach
  exact (reachable_invariant hReach).1

theorem m7_t3_wake_generation_matches_for_all_reachable {s : State} :
    Reachable s -> wakeGenerationMatchesState s := by
  intro hReach
  exact (reachable_invariant hReach).2.1

theorem m7_t3_no_lost_wakeup_for_all_reachable {s : State} :
    Reachable s -> noLostWakeupState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.1

theorem m7_t3_domain_budget_eligible_for_all_reachable {s : State} :
    Reachable s -> domainBudgetEligibleState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.1

theorem m7_t3_explicit_cmpxchg_failure_for_all_reachable {s : State} :
    Reachable s -> explicitCmpxchgFailureState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2

end Lnp64.M7Transition
