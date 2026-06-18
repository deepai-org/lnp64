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

inductive CommitStatus
  | ok
  | eagain
  | erevoked
deriving DecidableEq, Repr

structure CommitRecord where
  op : Op
  status : CommitStatus
  tid : Nat
  beforeLocation : Location
  afterLocation : Location
  waitGeneration : Nat
  addressGeneration : Nat
deriving DecidableEq, Repr

structure RtlM7CommitProjection where
  op : Op
  status : CommitStatus
  tid : Nat
  beforeLocation : Location
  afterLocation : Location
  waitGeneration : Nat
  addressGeneration : Nat
deriving DecidableEq, Repr

structure RtlM7StateProjection where
  op : Op
  status : CommitStatus
  tid : Nat
  location : Location
  waitGeneration : Nat
  atomicWord : Nat
  atomicCount : Nat
  cmpxchgFailureExplicit : Bool
  addressGeneration : Nat
  staleAddressGeneration : Nat
  domainBudget : Nat
  waitCost : Nat
  wakePending : Bool
  futexWakeDelivered : Bool
  timerWakeDelivered : Bool
  staleAddressRejected : Bool
deriving DecidableEq, Repr

def commitProjectionToRecord (projection : RtlM7CommitProjection) : CommitRecord :=
  { op := projection.op
    status := projection.status
    tid := projection.tid
    beforeLocation := projection.beforeLocation
    afterLocation := projection.afterLocation
    waitGeneration := projection.waitGeneration
    addressGeneration := projection.addressGeneration }

def commitMatchesRtlProjection
    (commit : CommitRecord)
    (projection : RtlM7CommitProjection) : Prop :=
  commit = commitProjectionToRecord projection

def stateProjectionOf
    (s : State)
    (op : Op)
    (status : CommitStatus) : RtlM7StateProjection :=
  { op := op
    status := status
    tid := s.thread.tid
    location := s.thread.location
    waitGeneration := s.thread.waitGeneration
    atomicWord := s.atomicWord
    atomicCount := s.atomicCount
    cmpxchgFailureExplicit := s.cmpxchgFailureExplicit
    addressGeneration := s.addressGeneration
    staleAddressGeneration := s.staleAddressGeneration
    domainBudget := s.domainBudget
    waitCost := s.waitCost
    wakePending := s.wakePending
    futexWakeDelivered := s.futexWakeDelivered
    timerWakeDelivered := s.timerWakeDelivered
    staleAddressRejected := s.staleAddressRejected }

def stateMatchesRtlProjection
    (s : State)
    (op : Op)
    (status : CommitStatus)
    (projection : RtlM7StateProjection) : Prop :=
  projection = stateProjectionOf s op status

def commitFor
    (s t : State)
    (op : Op)
    (status : CommitStatus) : CommitRecord :=
  { op := op
    status := status
    tid := s.thread.tid
    beforeLocation := s.thread.location
    afterLocation := t.thread.location
    waitGeneration := s.thread.waitGeneration
    addressGeneration := s.addressGeneration }

inductive TypedCommitTransition : State -> CommitRecord -> State -> Prop
  | cmpxchgSuccess (s : State) :
      s.atomicCount = 0 ->
      TypedCommitTransition s
        (commitFor s { s with atomicWord := 1, atomicCount := 1 } Op.cmpxchgSuccess CommitStatus.ok)
        { s with atomicWord := 1, atomicCount := 1 }
  | cmpxchgFail (s : State) :
      s.atomicCount = 1 ->
      s.atomicWord = 1 ->
      TypedCommitTransition s
        (commitFor s
          { s with atomicCount := 2, cmpxchgFailureExplicit := true }
          Op.cmpxchgFail
          CommitStatus.eagain)
        { s with atomicCount := 2, cmpxchgFailureExplicit := true }
  | futexWait (s : State) :
      s.wakePending = false ->
      TypedCommitTransition s
        (commitFor s
          { s with thread := { s.thread with
            location := Location.parked
            waitGeneration := s.addressGeneration } }
          Op.futexWait
          CommitStatus.ok)
        { s with thread := { s.thread with
          location := Location.parked
          waitGeneration := s.addressGeneration } }
  | futexWake (s : State) :
      s.thread.location = Location.parked ->
      s.thread.waitGeneration = s.addressGeneration ->
      TypedCommitTransition s
        (commitFor s
          { s with
            thread := { s.thread with location := Location.runnable }
            wakePending := true
            futexWakeDelivered := true }
          Op.futexWake
          CommitStatus.ok)
        { s with
          thread := { s.thread with location := Location.runnable }
          wakePending := true
          futexWakeDelivered := true }
  | timerWait (s : State) :
      s.wakePending = false ->
      TypedCommitTransition s
        (commitFor s
          { s with thread := { s.thread with
            location := Location.parked
            waitGeneration := s.addressGeneration } }
          Op.timerWait
          CommitStatus.ok)
        { s with thread := { s.thread with
          location := Location.parked
          waitGeneration := s.addressGeneration } }
  | timerExpire (s : State) :
      s.thread.location = Location.parked ->
      s.thread.waitGeneration = s.addressGeneration ->
      TypedCommitTransition s
        (commitFor s
          { s with
            thread := { s.thread with location := Location.runnable }
            wakePending := true
            timerWakeDelivered := true }
          Op.timerExpire
          CommitStatus.ok)
        { s with
          thread := { s.thread with location := Location.runnable }
          wakePending := true
          timerWakeDelivered := true }
  | consumeWake (s : State) :
      s.wakePending = true ->
      TypedCommitTransition s
        (commitFor s { s with wakePending := false } Op.consumeWake CommitStatus.ok)
        { s with wakePending := false }
  | rejectStaleAddress (s : State) :
      s.staleAddressGeneration ≠ s.addressGeneration ->
      TypedCommitTransition s
        (commitFor s
          { s with staleAddressRejected := true }
          Op.rejectStaleAddress
          CommitStatus.erevoked)
        { s with staleAddressRejected := true }

def RtlM7RefinementStep
    (pre : RtlM7StateProjection)
    (commitProjection : RtlM7CommitProjection)
    (post : RtlM7StateProjection) : Prop :=
  exists s t commit,
    stateMatchesRtlProjection s commit.op commit.status pre /\
    commitMatchesRtlProjection commit commitProjection /\
    TypedCommitTransition s commit t /\
    stateMatchesRtlProjection t commit.op commit.status post

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

theorem typed_commit_transition_refines_step
    {s t : State} {commit : CommitRecord} :
    TypedCommitTransition s commit t -> Step s commit.op t := by
  intro hCommit
  cases hCommit <;> simp [commitFor] <;> constructor <;> assumption

theorem typed_commit_transition_preserves_invariant
    {s t : State} {commit : CommitRecord} :
    invariant s ->
    TypedCommitTransition s commit t ->
    invariant t := by
  intro hInv hCommit
  exact invariant_step hInv (typed_commit_transition_refines_step hCommit)

theorem rtl_m7_refinement_step_refines_lean_step
    {pre : RtlM7StateProjection}
    {commitProjection : RtlM7CommitProjection}
    {post : RtlM7StateProjection} :
    RtlM7RefinementStep pre commitProjection post ->
    exists s t op status,
      stateMatchesRtlProjection s op status pre /\
      Step s op t /\
      stateMatchesRtlProjection t op status post := by
  intro hRefine
  rcases hRefine with ⟨s, t, commit, hPre, _hCommitProjection, hCommit, hPost⟩
  exact ⟨s, t, commit.op, commit.status, hPre, typed_commit_transition_refines_step hCommit, hPost⟩

theorem rtl_m7_refinement_step_preserves_scheduler_invariant
    {pre : RtlM7StateProjection}
    {commitProjection : RtlM7CommitProjection}
    {post : RtlM7StateProjection} :
    RtlM7RefinementStep pre commitProjection post ->
    (forall s op status, stateMatchesRtlProjection s op status pre -> invariant s) ->
    exists t op status, stateMatchesRtlProjection t op status post /\ invariant t := by
  intro hRefine hPreInvariant
  rcases hRefine with ⟨s, t, commit, hPre, _hCommitProjection, hCommit, hPost⟩
  exact ⟨t, commit.op, commit.status, hPost,
    typed_commit_transition_preserves_invariant (hPreInvariant s commit.op commit.status hPre) hCommit⟩

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

theorem m7_t3_typed_commit_transition_refines_step_for_reachable
    {s t : State} {commit : CommitRecord} :
    Reachable s ->
    TypedCommitTransition s commit t ->
    Step s commit.op t := by
  intro _hReach hCommit
  exact typed_commit_transition_refines_step hCommit

theorem m7_t3_typed_commit_transition_preserves_invariant_for_reachable
    {s t : State} {commit : CommitRecord} :
    Reachable s ->
    TypedCommitTransition s commit t ->
    invariant t := by
  intro hReach hCommit
  exact typed_commit_transition_preserves_invariant (reachable_invariant hReach) hCommit

theorem m7_t3_rtl_m7_refinement_step_preserves_scheduler_invariant_for_reachable
    {pre : RtlM7StateProjection}
    {commitProjection : RtlM7CommitProjection}
    {post : RtlM7StateProjection}
    {s t : State}
    {commit : CommitRecord} :
    Reachable s ->
    stateMatchesRtlProjection s commit.op commit.status pre ->
    commitMatchesRtlProjection commit commitProjection ->
    TypedCommitTransition s commit t ->
    stateMatchesRtlProjection t commit.op commit.status post ->
    RtlM7RefinementStep pre commitProjection post /\
      Step s commit.op t /\
      invariant t := by
  intro hReach hPre hCommitProjection hCommit hPost
  exact ⟨
    ⟨s, t, commit, hPre, hCommitProjection, hCommit, hPost⟩,
    typed_commit_transition_refines_step hCommit,
    typed_commit_transition_preserves_invariant (reachable_invariant hReach) hCommit
  ⟩

end Lnp64.M7Transition
