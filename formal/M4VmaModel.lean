/- LNP64 M4 VMA/MMU checked model.

This bounded model names the proof targets exercised by `formal/m4_vma_model.py`
and `rtl/engines/lnp64_m4_vma.sv`.
The obligations below are proved over the bounded VMA/MMU trace.
-/

namespace Lnp64.M4

inductive Fault
  | access
  | nx
  | guard
  | stale
deriving DecidableEq, Repr

structure Perms where
  read : Bool
  write : Bool
  execute : Bool
deriving Repr

structure Vma where
  id : Nat
  generation : Nat
  base : Nat
  pages : Nat
  perms : Perms
  guardPage : Bool
deriving Repr

structure Machine where
  mapping : Option Vma
  staleGeneration : Nat
  tlbValid : Bool
  loadCompleted : Bool
  storeRejected : Bool
  execFaulted : Bool
  guardFaulted : Bool
  staleRejected : Bool
  invalidationObserved : Bool
deriving Repr

def wxEnforced (p : Perms) : Prop :=
  p.write = true -> p.execute = false

def mappingCreated (m : Machine) : Prop :=
  exists v, m.mapping = some v /\ v.pages > 0

def readableLoadPermitted (m : Machine) : Prop :=
  forall v, m.mapping = some v -> v.perms.read = true -> m.loadCompleted = true

def writeWithoutPermissionRejected (m : Machine) : Prop :=
  forall v, m.mapping = some v -> v.perms.write = false -> m.storeRejected = true

def nxExecuteFaulted (m : Machine) : Prop :=
  forall v, m.mapping = some v -> v.perms.execute = false -> m.execFaulted = true

def guardAccessFaulted (m : Machine) : Prop :=
  forall v, m.mapping = some v -> v.guardPage = true -> m.guardFaulted = true

def staleGenerationRejected (m : Machine) : Prop :=
  forall v, m.mapping = some v -> m.staleGeneration != v.generation -> m.staleRejected = true

def tlbInvalidationObserved (m : Machine) : Prop :=
  m.invalidationObserved = true -> m.tlbValid = false

def noInvalidMemoryAccess (m : Machine) : Prop :=
  m.storeRejected = true /\
  m.execFaulted = true /\
  m.guardFaulted = true /\
  m.staleRejected = true

def wxNxGuardEnforced (m : Machine) : Prop :=
  (match m.mapping with
  | some v => wxEnforced v.perms
  | none => False) /\
  m.execFaulted = true /\
  m.guardFaulted = true

def noAccessAfterUnmapOrRevokeGenerationMismatch (m : Machine) : Prop :=
  m.staleRejected = true /\ m.tlbValid = false

def cacheTlbQuiescentBeforeAuthorityReuse (m : Machine) : Prop :=
  m.invalidationObserved = true /\ m.tlbValid = false

def rxPerms : Perms :=
  { read := true, write := false, execute := true }

def rPerms : Perms :=
  { read := true, write := false, execute := false }

def initialMachine : Machine :=
  { mapping := none
    staleGeneration := 0
    tlbValid := false
    loadCompleted := false
    storeRejected := false
    execFaulted := false
    guardFaulted := false
    staleRejected := false
    invalidationObserved := false }

def mapVma (m : Machine) : Machine :=
  { m with
    mapping := some
      { id := 1
        generation := 1
        base := 0x4000
        pages := 2
        perms := rxPerms
        guardPage := true }
    staleGeneration := 1
    tlbValid := true }

def permitLoad (m : Machine) : Machine :=
  { m with loadCompleted := true }

def rejectStore (m : Machine) : Machine :=
  { m with storeRejected := true }

def faultNxExec (m : Machine) : Machine :=
  match m.mapping with
  | none => { m with execFaulted := true }
  | some v =>
      { m with
        mapping := some { v with perms := rPerms }
        execFaulted := true }

def faultGuard (m : Machine) : Machine :=
  { m with guardFaulted := true }

def rejectStaleGeneration (m : Machine) : Machine :=
  match m.mapping with
  | none => { m with staleRejected := true }
  | some v =>
      { m with
        mapping := some { v with generation := v.generation + 1 }
        staleRejected := true }

def invalidateTlb (m : Machine) : Machine :=
  { m with tlbValid := false, invalidationObserved := true }

def afterMap : Machine :=
  mapVma initialMachine

def afterLoad : Machine :=
  permitLoad afterMap

def afterStoreDenied : Machine :=
  rejectStore afterLoad

def afterNxFault : Machine :=
  faultNxExec afterStoreDenied

def afterGuardFault : Machine :=
  faultGuard afterNxFault

def afterStaleReject : Machine :=
  rejectStaleGeneration afterGuardFault

def finalMachine : Machine :=
  invalidateTlb afterStaleReject

theorem m4_mapping_created :
  mappingCreated afterMap := by
  simp [afterMap, mapVma, initialMachine, mappingCreated, rxPerms]

theorem m4_readable_load_permitted :
  readableLoadPermitted afterLoad := by
  intro v mappingEq _readable
  simp [
    afterLoad, permitLoad, afterMap, mapVma, initialMachine, rxPerms
  ] at mappingEq
  simp [afterLoad, permitLoad, afterMap, mapVma, initialMachine, rxPerms]

theorem m4_write_without_permission_rejected :
  writeWithoutPermissionRejected afterStoreDenied := by
  intro v mappingEq _notWritable
  simp [
    afterStoreDenied, rejectStore, afterLoad, permitLoad, afterMap, mapVma,
    initialMachine, rxPerms
  ] at mappingEq
  simp [
    afterStoreDenied, rejectStore, afterLoad, permitLoad, afterMap, mapVma,
    initialMachine, rxPerms
  ]

theorem m4_nx_execute_faulted :
  nxExecuteFaulted afterNxFault := by
  intro v mappingEq _notExecutable
  simp [
    afterNxFault, faultNxExec, afterStoreDenied, rejectStore, afterLoad,
    permitLoad, afterMap, mapVma, initialMachine, rxPerms, rPerms
  ] at mappingEq
  simp [
    afterNxFault, faultNxExec, afterStoreDenied, rejectStore, afterLoad,
    permitLoad, afterMap, mapVma, initialMachine, rxPerms, rPerms
  ]

theorem m4_guard_access_faulted :
  guardAccessFaulted afterGuardFault := by
  intro v mappingEq _guarded
  simp [
    afterGuardFault, faultGuard, afterNxFault, faultNxExec, afterStoreDenied,
    rejectStore, afterLoad, permitLoad, afterMap, mapVma, initialMachine,
    rxPerms, rPerms
  ] at mappingEq
  simp [
    afterGuardFault, faultGuard, afterNxFault, faultNxExec, afterStoreDenied,
    rejectStore, afterLoad, permitLoad, afterMap, mapVma, initialMachine,
    rxPerms, rPerms
  ]

theorem m4_stale_generation_rejected :
  staleGenerationRejected afterStaleReject := by
  intro v mappingEq _stale
  simp [
    afterStaleReject, rejectStaleGeneration, afterGuardFault, faultGuard,
    afterNxFault, faultNxExec, afterStoreDenied, rejectStore, afterLoad,
    permitLoad, afterMap, mapVma, initialMachine, rxPerms, rPerms
  ] at mappingEq
  simp [
    afterStaleReject, rejectStaleGeneration, afterGuardFault, faultGuard,
    afterNxFault, faultNxExec, afterStoreDenied, rejectStore, afterLoad,
    permitLoad, afterMap, mapVma, initialMachine, rxPerms, rPerms
  ]

theorem m4_tlb_invalidation_observed :
  tlbInvalidationObserved finalMachine := by
  intro _observed
  simp [
    finalMachine, invalidateTlb, afterStaleReject, rejectStaleGeneration,
    afterGuardFault, faultGuard, afterNxFault, faultNxExec, afterStoreDenied,
    rejectStore, afterLoad, permitLoad, afterMap, mapVma, initialMachine,
    rxPerms, rPerms
  ]

theorem m4_no_invalid_memory_access :
  noInvalidMemoryAccess finalMachine := by
  simp [
    noInvalidMemoryAccess, finalMachine, invalidateTlb, afterStaleReject,
    rejectStaleGeneration, afterGuardFault, faultGuard, afterNxFault,
    faultNxExec, afterStoreDenied, rejectStore, afterLoad, permitLoad,
    afterMap, mapVma, initialMachine, rxPerms, rPerms
  ]

theorem m4_wx_nx_guard_enforced :
  wxNxGuardEnforced finalMachine := by
  simp [
    wxNxGuardEnforced, wxEnforced, finalMachine, invalidateTlb,
    afterStaleReject, rejectStaleGeneration, afterGuardFault, faultGuard,
    afterNxFault, faultNxExec, afterStoreDenied, rejectStore, afterLoad,
    permitLoad, afterMap, mapVma, initialMachine, rxPerms, rPerms
  ]

theorem m4_no_access_after_unmap_or_revoke_generation_mismatch :
  noAccessAfterUnmapOrRevokeGenerationMismatch finalMachine := by
  simp [
    noAccessAfterUnmapOrRevokeGenerationMismatch, finalMachine, invalidateTlb,
    afterStaleReject, rejectStaleGeneration, afterGuardFault, faultGuard,
    afterNxFault, faultNxExec, afterStoreDenied, rejectStore, afterLoad,
    permitLoad, afterMap, mapVma, initialMachine, rxPerms, rPerms
  ]

theorem m4_cache_tlb_quiescent_before_authority_reuse :
  cacheTlbQuiescentBeforeAuthorityReuse finalMachine := by
  simp [
    cacheTlbQuiescentBeforeAuthorityReuse, finalMachine, invalidateTlb,
    afterStaleReject, rejectStaleGeneration, afterGuardFault, faultGuard,
    afterNxFault, faultNxExec, afterStoreDenied, rejectStore, afterLoad,
    permitLoad, afterMap, mapVma, initialMachine, rxPerms, rPerms
  ]

theorem m4_wx_enforced :
  match finalMachine.mapping with
  | some v => wxEnforced v.perms
  | none => False := by
  simp [
    finalMachine, invalidateTlb, afterStaleReject, rejectStaleGeneration,
    afterGuardFault, faultGuard, afterNxFault, faultNxExec, afterStoreDenied,
    rejectStore, afterLoad, permitLoad, afterMap, mapVma, initialMachine,
    rxPerms, rPerms, wxEnforced
  ]

end Lnp64.M4
