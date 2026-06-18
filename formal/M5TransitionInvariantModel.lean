/- LNP64 M5 transition-invariant model.

`M5DmaModel.lean` is a bounded DMA/memory-object witness. This file adds a
small transition-invariant proof slice for DMA confinement: typed buffer state,
typed operations, a `Step` relation, `Reachable`, preservation, and conditional
safety theorems over arbitrary reachable states.
-/

namespace Lnp64.M5Transition

structure Rights where
  read : Bool
  write : Bool
deriving DecidableEq, Repr

structure DmaBuffer where
  id : Nat
  generation : Nat
  domainId : Nat
  rights : Rights
  visible : Bool
  pinned : Bool
deriving DecidableEq, Repr

structure State where
  requesterDomain : Nat
  dst : DmaBuffer
  staleDstGeneration : Nat
  pinCompleted : Bool
  copyCompleted : Bool
  fillCompleted : Bool
  unpinCompleted : Bool
  permissionFaulted : Bool
  revokedRejected : Bool
  domainIsolationEnforced : Bool
  coherenceObserved : Bool
  completions : Nat
deriving DecidableEq, Repr

inductive Op
  | pinBuffer
  | dmaCopy
  | dmaFill
  | unpinBuffer
  | faultMissingWrite
  | rejectRevokedSubmit
  | rejectCrossDomain (isolationDomain : Nat)
  | observeCoherence
deriving DecidableEq, Repr

def rwRights : Rights :=
  { read := true, write := true }

def readOnlyRights : Rights :=
  { read := true, write := false }

def dst0 : DmaBuffer :=
  { id := 2
    generation := 1
    domainId := 1
    rights := rwRights
    visible := false
    pinned := false }

def reset : State :=
  { requesterDomain := 1
    dst := dst0
    staleDstGeneration := 1
    pinCompleted := false
    copyCompleted := false
    fillCompleted := false
    unpinCompleted := false
    permissionFaulted := false
    revokedRejected := false
    domainIsolationEnforced := false
    coherenceObserved := false
    completions := 0 }

def sameDomain (s : State) : Prop :=
  s.requesterDomain = s.dst.domainId

def writePermitted (s : State) : Prop :=
  s.dst.rights.write = true

def completionCountMatches (s : State) : Prop :=
  s.completions =
    (if s.copyCompleted then 1 else 0) + (if s.fillCompleted then 1 else 0)

def copyRequiresPinState (s : State) : Prop :=
  s.copyCompleted = true -> s.pinCompleted = true

def fillRequiresCopyState (s : State) : Prop :=
  s.fillCompleted = true -> s.copyCompleted = true

def unpinClearsPinnedState (s : State) : Prop :=
  s.unpinCompleted = true -> s.dst.pinned = false

def missingWritePermissionFaultsState (s : State) : Prop :=
  s.dst.rights.write = false -> s.permissionFaulted = true

def revokedGenerationRejectedState (s : State) : Prop :=
  s.staleDstGeneration ≠ s.dst.generation -> s.revokedRejected = true

def crossDomainRejectedState (s : State) : Prop :=
  s.requesterDomain ≠ s.dst.domainId -> s.domainIsolationEnforced = true

def coherentVisibilityObservedState (s : State) : Prop :=
  s.coherenceObserved = true -> s.dst.visible = true

def completionsAreExactState (s : State) : Prop :=
  s.copyCompleted = true -> s.fillCompleted = true -> s.completions = 2

def invariant (s : State) : Prop :=
  completionCountMatches s /\
  copyRequiresPinState s /\
  fillRequiresCopyState s /\
  unpinClearsPinnedState s /\
  missingWritePermissionFaultsState s /\
  revokedGenerationRejectedState s /\
  crossDomainRejectedState s /\
  coherentVisibilityObservedState s

inductive Step : State -> Op -> State -> Prop
  | pinBuffer (s : State) :
      sameDomain s ->
      writePermitted s ->
      s.unpinCompleted = false ->
      Step s Op.pinBuffer
        { s with dst := { s.dst with pinned := true }, pinCompleted := true }
  | dmaCopy (s : State) :
      s.dst.pinned = true ->
      s.pinCompleted = true ->
      sameDomain s ->
      writePermitted s ->
      s.copyCompleted = false ->
      Step s Op.dmaCopy
        { s with copyCompleted := true, completions := s.completions + 1 }
  | dmaFill (s : State) :
      s.dst.pinned = true ->
      sameDomain s ->
      writePermitted s ->
      s.copyCompleted = true ->
      s.fillCompleted = false ->
      Step s Op.dmaFill
        { s with fillCompleted := true, completions := s.completions + 1 }
  | unpinBuffer (s : State) :
      s.dst.pinned = true ->
      Step s Op.unpinBuffer
        { s with dst := { s.dst with pinned := false }, unpinCompleted := true }
  | faultMissingWrite (s : State) :
      Step s Op.faultMissingWrite
        { s with
          dst := { s.dst with rights := readOnlyRights }
          permissionFaulted := true }
  | rejectRevokedSubmit (s : State) :
      Step s Op.rejectRevokedSubmit
        { s with
          dst := { s.dst with generation := s.dst.generation + 1 }
          revokedRejected := true }
  | rejectCrossDomain (s : State) (isolationDomain : Nat) :
      isolationDomain ≠ s.requesterDomain ->
      Step s (Op.rejectCrossDomain isolationDomain)
        { s with
          dst := { s.dst with domainId := isolationDomain }
          domainIsolationEnforced := true }
  | observeCoherence (s : State) :
      Step s Op.observeCoherence
        { s with
          dst := { s.dst with visible := true }
          coherenceObserved := true }

inductive Reachable : State -> Prop
  | reset : Reachable reset
  | step {s t : State} {op : Op} :
      Reachable s -> Step s op t -> Reachable t

theorem invariant_reset :
    invariant reset := by
  simp [
    invariant, reset, dst0, rwRights, completionCountMatches,
    copyRequiresPinState, fillRequiresCopyState, unpinClearsPinnedState,
    missingWritePermissionFaultsState, revokedGenerationRejectedState,
    crossDomainRejectedState, coherentVisibilityObservedState
  ]

theorem invariant_step {s t : State} {op : Op} :
    invariant s -> Step s op t -> invariant t := by
  intro hInv hStep
  cases hStep <;>
    simp_all [
      invariant, sameDomain, writePermitted, readOnlyRights,
      completionCountMatches, copyRequiresPinState, fillRequiresCopyState,
      unpinClearsPinnedState, missingWritePermissionFaultsState,
      revokedGenerationRejectedState, crossDomainRejectedState,
      coherentVisibilityObservedState
    ]

theorem reachable_invariant {s : State} :
    Reachable s -> invariant s := by
  intro hReach
  induction hReach with
  | reset => exact invariant_reset
  | step hPrev hStep ih => exact invariant_step ih hStep

theorem m5_t3_missing_write_permission_faults_for_all_reachable {s : State} :
    Reachable s -> missingWritePermissionFaultsState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.1

theorem m5_t3_revoked_generation_rejected_for_all_reachable {s : State} :
    Reachable s -> revokedGenerationRejectedState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.2.1

theorem m5_t3_cross_domain_rejected_for_all_reachable {s : State} :
    Reachable s -> crossDomainRejectedState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.2.2.1

theorem m5_t3_coherent_visibility_observed_for_all_reachable {s : State} :
    Reachable s -> coherentVisibilityObservedState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.2.2.2

theorem m5_t3_unpin_clears_pinned_state_for_all_reachable {s : State} :
    Reachable s -> unpinClearsPinnedState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.1

theorem m5_t3_completions_are_exact_for_all_reachable {s : State} :
    Reachable s -> completionsAreExactState s := by
  intro hReach hCopy hFill
  have hCount := (reachable_invariant hReach).1
  simp [completionCountMatches, hCopy, hFill] at hCount
  exact hCount

end Lnp64.M5Transition
