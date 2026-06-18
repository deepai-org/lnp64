/- LNP64 M4 transition-invariant model.

`M4VmaModel.lean` is a bounded VMA/MMU witness. This file adds a small
transition-invariant proof slice for memory protection: typed VMA state, typed
operations, a `Step` relation, `Reachable`, preservation, and conditional
safety theorems over arbitrary reachable states.
-/

namespace Lnp64.M4Transition

structure Perms where
  read : Bool
  write : Bool
  execute : Bool
deriving DecidableEq, Repr

structure Vma where
  id : Nat
  generation : Nat
  base : Nat
  pages : Nat
  perms : Perms
  guardPage : Bool
deriving DecidableEq, Repr

structure State where
  mapping : Option Vma
  staleGeneration : Nat
  tlbValid : Bool
  mappingCreated : Bool
  loadCompleted : Bool
  storeChecked : Bool
  storeRejected : Bool
  execChecked : Bool
  execFaulted : Bool
  guardChecked : Bool
  guardFaulted : Bool
  staleChecked : Bool
  staleRejected : Bool
  invalidationObserved : Bool
deriving DecidableEq, Repr

inductive Op
  | mapVma
  | permitLoad
  | rejectStore
  | faultNxExec
  | faultGuard
  | rejectStaleGeneration
  | invalidateTlb
deriving DecidableEq, Repr

def rxPerms : Perms :=
  { read := true, write := false, execute := true }

def rPerms : Perms :=
  { read := true, write := false, execute := false }

def vma0 : Vma :=
  { id := 1
    generation := 1
    base := 0x4000
    pages := 2
    perms := rxPerms
    guardPage := true }

def reset : State :=
  { mapping := none
    staleGeneration := 0
    tlbValid := false
    mappingCreated := false
    loadCompleted := false
    storeChecked := false
    storeRejected := false
    execChecked := false
    execFaulted := false
    guardChecked := false
    guardFaulted := false
    staleChecked := false
    staleRejected := false
    invalidationObserved := false }

def wxEnforcedPerms (p : Perms) : Prop :=
  p.write = true -> p.execute = false

def wxEnforcedState (s : State) : Prop :=
  forall v, s.mapping = some v -> wxEnforcedPerms v.perms

def mappingCreatedState (s : State) : Prop :=
  s.mappingCreated = true -> exists v, s.mapping = some v /\ v.pages > 0

def readableLoadPermittedState (s : State) : Prop :=
  s.loadCompleted = true -> exists v, s.mapping = some v /\ v.perms.read = true

def writeWithoutPermissionRejectedState (s : State) : Prop :=
  s.storeChecked = true ->
    forall v, s.mapping = some v -> v.perms.write = false -> s.storeRejected = true

def nxExecuteFaultedState (s : State) : Prop :=
  s.execChecked = true ->
    forall v, s.mapping = some v -> v.perms.execute = false -> s.execFaulted = true

def guardAccessFaultedState (s : State) : Prop :=
  s.guardChecked = true ->
    forall v, s.mapping = some v -> v.guardPage = true -> s.guardFaulted = true

def staleGenerationRejectedState (s : State) : Prop :=
  s.staleChecked = true ->
    forall v, s.mapping = some v -> s.staleGeneration ≠ v.generation -> s.staleRejected = true

def tlbInvalidationObservedState (s : State) : Prop :=
  s.invalidationObserved = true -> s.tlbValid = false

def cacheTlbQuiescentBeforeAuthorityReuseState (s : State) : Prop :=
  s.invalidationObserved = true -> s.tlbValid = false /\ s.staleRejected = true

def invariant (s : State) : Prop :=
  wxEnforcedState s /\
  mappingCreatedState s /\
  readableLoadPermittedState s /\
  writeWithoutPermissionRejectedState s /\
  nxExecuteFaultedState s /\
  guardAccessFaultedState s /\
  staleGenerationRejectedState s /\
  tlbInvalidationObservedState s /\
  cacheTlbQuiescentBeforeAuthorityReuseState s

theorem rx_wx :
    wxEnforcedPerms rxPerms := by
  simp [wxEnforcedPerms, rxPerms]

theorem r_wx :
    wxEnforcedPerms rPerms := by
  simp [wxEnforcedPerms, rPerms]

inductive Step : State -> Op -> State -> Prop
  | mapVma (s : State) :
      s.mapping = none ->
      s.loadCompleted = false ->
      s.storeChecked = false ->
      s.execChecked = false ->
      s.guardChecked = false ->
      s.staleChecked = false ->
      s.invalidationObserved = false ->
      Step s Op.mapVma
        { s with
          mapping := some vma0
          staleGeneration := vma0.generation
          tlbValid := true
          mappingCreated := true }
  | permitLoad (s : State) (v : Vma) :
      s.mapping = some v ->
      s.tlbValid = true ->
      v.perms.read = true ->
      Step s Op.permitLoad { s with loadCompleted := true }
  | rejectStore (s : State) (v : Vma) :
      s.mapping = some v ->
      v.perms.write = false ->
      Step s Op.rejectStore
        { s with storeChecked := true, storeRejected := true }
  | faultNxExec (s : State) (v : Vma) :
      s.mapping = some v ->
      (s.storeChecked = true -> s.storeRejected = true) ->
      Step s Op.faultNxExec
        { s with
          mapping := some { v with perms := rPerms }
          execChecked := true
          execFaulted := true }
  | faultGuard (s : State) (v : Vma) :
      s.mapping = some v ->
      v.guardPage = true ->
      Step s Op.faultGuard
        { s with guardChecked := true, guardFaulted := true }
  | rejectStaleGeneration (s : State) (v : Vma) :
      s.mapping = some v ->
      Step s Op.rejectStaleGeneration
        { s with
          mapping := some { v with generation := v.generation + 1 }
          staleChecked := true
          staleRejected := true }
  | invalidateTlb (s : State) :
      s.staleRejected = true ->
      Step s Op.invalidateTlb
        { s with tlbValid := false, invalidationObserved := true }

inductive Reachable : State -> Prop
  | reset : Reachable reset
  | step {s t : State} {op : Op} :
      Reachable s -> Step s op t -> Reachable t

theorem invariant_reset :
    invariant reset := by
  simp [
    invariant, reset, wxEnforcedState, mappingCreatedState,
    readableLoadPermittedState, writeWithoutPermissionRejectedState,
    nxExecuteFaultedState, guardAccessFaultedState,
    staleGenerationRejectedState, tlbInvalidationObservedState,
    cacheTlbQuiescentBeforeAuthorityReuseState
  ]

theorem invariant_step {s t : State} {op : Op} :
    invariant s -> Step s op t -> invariant t := by
  intro hInv hStep
  cases hStep <;>
    simp_all [
      invariant, vma0, rxPerms, rPerms, wxEnforcedState, wxEnforcedPerms,
      mappingCreatedState, readableLoadPermittedState,
      writeWithoutPermissionRejectedState, nxExecuteFaultedState,
      guardAccessFaultedState, staleGenerationRejectedState,
      tlbInvalidationObservedState, cacheTlbQuiescentBeforeAuthorityReuseState
    ]

theorem reachable_invariant {s : State} :
    Reachable s -> invariant s := by
  intro hReach
  induction hReach with
  | reset => exact invariant_reset
  | step hPrev hStep ih => exact invariant_step ih hStep

theorem m4_t3_wx_enforced_for_all_reachable {s : State} :
    Reachable s -> wxEnforcedState s := by
  intro hReach
  exact (reachable_invariant hReach).1

theorem m4_t3_mapping_created_for_all_reachable {s : State} :
    Reachable s -> mappingCreatedState s := by
  intro hReach
  exact (reachable_invariant hReach).2.1

theorem m4_t3_write_without_permission_rejected_for_all_reachable {s : State} :
    Reachable s -> writeWithoutPermissionRejectedState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.1

theorem m4_t3_nx_execute_faulted_for_all_reachable {s : State} :
    Reachable s -> nxExecuteFaultedState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.1

theorem m4_t3_guard_access_faulted_for_all_reachable {s : State} :
    Reachable s -> guardAccessFaultedState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.2.1

theorem m4_t3_stale_generation_rejected_for_all_reachable {s : State} :
    Reachable s -> staleGenerationRejectedState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.2.2.1

theorem m4_t3_tlb_invalidation_observed_for_all_reachable {s : State} :
    Reachable s -> tlbInvalidationObservedState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.2.2.2.1

theorem m4_t3_cache_tlb_quiescent_before_authority_reuse_for_all_reachable {s : State} :
    Reachable s -> cacheTlbQuiescentBeforeAuthorityReuseState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.2.2.2.2

end Lnp64.M4Transition
