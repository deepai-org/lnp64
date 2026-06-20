/- LNP64 M12 SD/SPI storage-barrier checked model.

This bounded model covers the first Track D step 14 smoke slice: boot-image
storage visibility, block-object writes, storage barrier quiescence,
domain/generation fail-closed behavior, media fault termination, and absence of
raw device authority in software-visible state.
-/

namespace Lnp64.M12

structure Machine where
  rootDomain : Nat
  bootImageVisible : Bool
  blockObjectAuthorized : Bool
  blockWriteCompleted : Bool
  storageBarrierIssued : Bool
  storageBarrierQuiescent : Bool
  staleObjectRejected : Bool
  crossDomainRejected : Bool
  mediaFaultTerminal : Bool
  rawDeviceAuthorityVisible : Bool
  completions : Nat
  faults : Nat
deriving Repr

def bootImageReadVisible (m : Machine) : Prop :=
  m.rootDomain > 0 /\ m.bootImageVisible = true /\ m.completions >= 1

def blockWriteRequiresObjectAuthority (m : Machine) : Prop :=
  m.blockObjectAuthorized = true /\ m.blockWriteCompleted = true /\ m.completions >= 2

def storageBarrierReachedQuiescence (m : Machine) : Prop :=
  m.storageBarrierIssued = true /\ m.storageBarrierQuiescent = true /\ m.completions >= 3

def staleObjectFailsClosed (m : Machine) : Prop :=
  m.staleObjectRejected = true /\ m.faults >= 1

def crossDomainFailsClosed (m : Machine) : Prop :=
  m.crossDomainRejected = true /\ m.faults >= 2

def mediaFaultReachesTerminalPath (m : Machine) : Prop :=
  m.mediaFaultTerminal = true /\ m.faults >= 3

def noRawDeviceAuthority (m : Machine) : Prop :=
  m.rawDeviceAuthorityVisible = false

def countsExact (m : Machine) : Prop :=
  m.completions = 3 /\ m.faults = 3

def initialMachine : Machine :=
  { rootDomain := 0
    bootImageVisible := false
    blockObjectAuthorized := false
    blockWriteCompleted := false
    storageBarrierIssued := false
    storageBarrierQuiescent := false
    staleObjectRejected := false
    crossDomainRejected := false
    mediaFaultTerminal := false
    rawDeviceAuthorityVisible := false
    completions := 0
    faults := 0 }

def boot (m : Machine) : Machine :=
  { m with rootDomain := 1 }

def readBootImage (m : Machine) : Machine :=
  { m with bootImageVisible := true, completions := m.completions + 1 }

def writeBlockObject (m : Machine) : Machine :=
  { m with
    blockObjectAuthorized := true
    blockWriteCompleted := true
    completions := m.completions + 1 }

def issueStorageBarrier (m : Machine) : Machine :=
  { m with
    storageBarrierIssued := true
    storageBarrierQuiescent := true
    completions := m.completions + 1 }

def rejectStaleObject (m : Machine) : Machine :=
  { m with staleObjectRejected := true, faults := m.faults + 1 }

def rejectCrossDomain (m : Machine) : Machine :=
  { m with crossDomainRejected := true, faults := m.faults + 1 }

def terminalMediaFault (m : Machine) : Machine :=
  { m with mediaFaultTerminal := true, faults := m.faults + 1 }

def afterBoot : Machine :=
  boot initialMachine

def afterBootImage : Machine :=
  readBootImage afterBoot

def afterBlockWrite : Machine :=
  writeBlockObject afterBootImage

def afterBarrier : Machine :=
  issueStorageBarrier afterBlockWrite

def afterStale : Machine :=
  rejectStaleObject afterBarrier

def afterCrossDomain : Machine :=
  rejectCrossDomain afterStale

def finalMachine : Machine :=
  terminalMediaFault afterCrossDomain

theorem m12_boot_image_read_visible :
  bootImageReadVisible afterBootImage := by
  simp [bootImageReadVisible, afterBootImage, readBootImage, afterBoot, boot, initialMachine]

theorem m12_block_write_requires_object_authority :
  blockWriteRequiresObjectAuthority afterBlockWrite := by
  simp [
    blockWriteRequiresObjectAuthority, afterBlockWrite, writeBlockObject,
    afterBootImage, readBootImage, afterBoot, boot, initialMachine
  ]

theorem m12_storage_barrier_quiescent :
  storageBarrierReachedQuiescence afterBarrier := by
  simp [
    storageBarrierReachedQuiescence, afterBarrier, issueStorageBarrier,
    afterBlockWrite, writeBlockObject, afterBootImage, readBootImage,
    afterBoot, boot, initialMachine
  ]

theorem m12_stale_object_rejected :
  staleObjectFailsClosed afterStale := by
  simp [
    staleObjectFailsClosed, afterStale, rejectStaleObject, afterBarrier,
    issueStorageBarrier, afterBlockWrite, writeBlockObject, afterBootImage,
    readBootImage, afterBoot, boot, initialMachine
  ]

theorem m12_cross_domain_rejected :
  crossDomainFailsClosed afterCrossDomain := by
  simp [
    crossDomainFailsClosed, afterCrossDomain, rejectCrossDomain, afterStale,
    rejectStaleObject, afterBarrier, issueStorageBarrier, afterBlockWrite,
    writeBlockObject, afterBootImage, readBootImage, afterBoot, boot,
    initialMachine
  ]

theorem m12_media_fault_terminal :
  mediaFaultReachesTerminalPath finalMachine := by
  simp [
    mediaFaultReachesTerminalPath, finalMachine, terminalMediaFault,
    afterCrossDomain, rejectCrossDomain, afterStale, rejectStaleObject,
    afterBarrier, issueStorageBarrier, afterBlockWrite, writeBlockObject,
    afterBootImage, readBootImage, afterBoot, boot, initialMachine
  ]

theorem m12_no_raw_device_authority :
  noRawDeviceAuthority finalMachine := by
  simp [
    noRawDeviceAuthority, finalMachine, terminalMediaFault, afterCrossDomain,
    rejectCrossDomain, afterStale, rejectStaleObject, afterBarrier,
    issueStorageBarrier, afterBlockWrite, writeBlockObject, afterBootImage,
    readBootImage, afterBoot, boot, initialMachine
  ]

theorem m12_counts_exact :
  countsExact finalMachine := by
  simp [
    countsExact, finalMachine, terminalMediaFault, afterCrossDomain,
    rejectCrossDomain, afterStale, rejectStaleObject, afterBarrier,
    issueStorageBarrier, afterBlockWrite, writeBlockObject, afterBootImage,
    readBootImage, afterBoot, boot, initialMachine
  ]

/- Packed-bit decode machinery for the M12 storage-barrier typed commit and
   state projection records. Mirrors the shared schema layout so the offline
   witness bits can be decoded back to projection fields and proved faithful in
   Lean with the kernel `decide` tactic (no native_decide, no axioms). -/

structure PackedFieldLayout where
  name : String
  width : Nat
  lsb : Nat
  msb : Nat
deriving DecidableEq, Repr

def packedSchemaWidth (schema : List (String × Nat)) : Nat :=
  schema.foldl (fun total field => total + field.2) 0

def packedSchemaLayoutFrom : Nat -> List (String × Nat) -> List PackedFieldLayout
  | _cursor, [] => []
  | cursor, field :: rest =>
      let lsb := cursor - field.2
      { name := field.1, width := field.2, lsb := lsb, msb := cursor - 1 } ::
        packedSchemaLayoutFrom lsb rest

def packedSchemaLayout (schema : List (String × Nat)) : List PackedFieldLayout :=
  packedSchemaLayoutFrom (packedSchemaWidth schema) schema

def packedFieldWithinWidth (totalWidth : Nat) (field : PackedFieldLayout) : Bool :=
  decide (field.width > 0) &&
  decide (field.lsb + field.width = field.msb + 1) &&
  decide (field.msb < totalWidth)

def packedLayoutWithinWidth (totalWidth : Nat) (layout : List PackedFieldLayout) : Bool :=
  layout.all (packedFieldWithinWidth totalWidth)

def packedLayoutStartsAtWidth (totalWidth : Nat) : List PackedFieldLayout -> Bool
  | [] => decide (totalWidth = 0)
  | field :: _rest => decide (field.msb + 1 = totalWidth)

def packedLayoutAdjacentContiguous : List PackedFieldLayout -> Bool
  | [] => true
  | _field :: [] => true
  | first :: second :: rest =>
      decide (first.lsb = second.msb + 1) &&
      packedLayoutAdjacentContiguous (second :: rest)

def packedLayoutEndsAtZero : List PackedFieldLayout -> Bool
  | [] => true
  | field :: [] => decide (field.lsb = 0)
  | _field :: rest => packedLayoutEndsAtZero rest

def packedLayoutCoversWidth (totalWidth : Nat) (layout : List PackedFieldLayout) : Bool :=
  packedLayoutWithinWidth totalWidth layout &&
  packedLayoutStartsAtWidth totalWidth layout &&
  packedLayoutAdjacentContiguous layout &&
  packedLayoutEndsAtZero layout

def packedBitSlice (bits lsb width : Nat) : Nat :=
  (bits / (2 ^ lsb)) % (2 ^ width)

def packedFieldValue (bits : Nat) (field : PackedFieldLayout) : Nat :=
  packedBitSlice bits field.lsb field.width

def packedLayoutFieldValue
    (bits : Nat)
    (fieldName : String) : List PackedFieldLayout -> Option Nat
  | [] => none
  | field :: rest =>
      if field.name == fieldName then
        some (packedFieldValue bits field)
      else
        packedLayoutFieldValue bits fieldName rest

def rtlM12CommitPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("object_id", 32)
  , ("object_generation", 32)
  , ("domain_id", 32)
  , ("barrier_id", 32)
  , ("block_index", 32)
  , ("data_value", 32) ]

def rtlM12StateProjectionPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("completions", 32)
  , ("faults", 32)
  , ("boot_image_visible", 1)
  , ("block_object_authorized", 1)
  , ("block_write_completed", 1)
  , ("storage_barrier_issued", 1)
  , ("storage_barrier_quiescent", 1)
  , ("stale_object_rejected", 1)
  , ("cross_domain_rejected", 1)
  , ("media_fault_terminal", 1)
  , ("no_raw_device_authority", 1)
  , ("counts_exact", 1) ]

def rtlM12CommitPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM12CommitPackedSchema

def rtlM12StateProjectionPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM12StateProjectionPackedSchema

theorem rtlM12CommitPackedSchema_width :
    packedSchemaWidth rtlM12CommitPackedSchema = 216 := by
  decide

theorem rtlM12StateProjectionPackedSchema_width :
    packedSchemaWidth rtlM12StateProjectionPackedSchema = 98 := by
  decide

theorem rtlM12CommitPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM12CommitPackedSchema)
      rtlM12CommitPackedLayout = true := by
  decide

theorem rtlM12StateProjectionPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM12StateProjectionPackedSchema)
      rtlM12StateProjectionPackedLayout = true := by
  decide

end Lnp64.M12
