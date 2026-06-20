/- LNP64 M11 DDR/metadata broker checked model.

This bounded model covers the first Track D step 10 smoke slice: an external
DDR line interface and shared metadata broker with domain/generation checks,
visibility after a write/read roundtrip, stale generation rejection, cross
domain rejection, ECC scrub visibility, and a quiescent barrier.
-/

namespace Lnp64.M11

structure Machine where
  rootDomain : Nat
  lineAllocated : Bool
  lineId : Nat
  lineGeneration : Nat
  metadataEpoch : Nat
  metadataBoundToDomain : Bool
  ddrWriteCompleted : Bool
  ddrReadCompleted : Bool
  readMatchesWrite : Bool
  staleGenerationRejected : Bool
  crossDomainRejected : Bool
  eccScrubbed : Bool
  barrierQuiescent : Bool
  completions : Nat
  faults : Nat
deriving Repr

def metadataAllocationSafe (m : Machine) : Prop :=
  m.lineAllocated = true /\
    m.metadataBoundToDomain = true /\
    m.rootDomain > 0 /\
    m.lineGeneration > 0 /\
    m.metadataEpoch > 0

def ddrReadAfterWriteVisible (m : Machine) : Prop :=
  m.ddrWriteCompleted = true /\ m.ddrReadCompleted = true /\ m.readMatchesWrite = true

def staleGenerationFailsClosed (m : Machine) : Prop :=
  m.staleGenerationRejected = true /\ m.faults >= 1

def crossDomainMetadataRejected (m : Machine) : Prop :=
  m.crossDomainRejected = true /\ m.faults >= 2

def eccScrubTerminal (m : Machine) : Prop :=
  m.eccScrubbed = true /\ m.faults >= 3

def metadataBarrierQuiescent (m : Machine) : Prop :=
  m.barrierQuiescent = true

def countsExact (m : Machine) : Prop :=
  m.completions = 2 /\ m.faults = 3

def initialMachine : Machine :=
  { rootDomain := 0
    lineAllocated := false
    lineId := 0
    lineGeneration := 0
    metadataEpoch := 0
    metadataBoundToDomain := false
    ddrWriteCompleted := false
    ddrReadCompleted := false
    readMatchesWrite := false
    staleGenerationRejected := false
    crossDomainRejected := false
    eccScrubbed := false
    barrierQuiescent := false
    completions := 0
    faults := 0 }

def boot (m : Machine) : Machine :=
  { m with rootDomain := 1 }

def allocateMetadata (m : Machine) : Machine :=
  { m with
    lineAllocated := true
    lineId := 1
    lineGeneration := 1
    metadataEpoch := 1
    metadataBoundToDomain := true }

def ddrWrite (m : Machine) : Machine :=
  { m with ddrWriteCompleted := true, completions := m.completions + 1 }

def ddrRead (m : Machine) : Machine :=
  { m with
    ddrReadCompleted := true
    readMatchesWrite := true
    completions := m.completions + 1 }

def rejectStaleGeneration (m : Machine) : Machine :=
  { m with staleGenerationRejected := true, faults := m.faults + 1 }

def rejectCrossDomain (m : Machine) : Machine :=
  { m with crossDomainRejected := true, faults := m.faults + 1 }

def eccScrub (m : Machine) : Machine :=
  { m with eccScrubbed := true, faults := m.faults + 1 }

def barrierFlush (m : Machine) : Machine :=
  { m with barrierQuiescent := true }

def afterBoot : Machine :=
  boot initialMachine

def afterMetadata : Machine :=
  allocateMetadata afterBoot

def afterWrite : Machine :=
  ddrWrite afterMetadata

def afterRead : Machine :=
  ddrRead afterWrite

def afterStale : Machine :=
  rejectStaleGeneration afterRead

def afterCrossDomain : Machine :=
  rejectCrossDomain afterStale

def afterEcc : Machine :=
  eccScrub afterCrossDomain

def finalMachine : Machine :=
  barrierFlush afterEcc

theorem m11_metadata_allocation_safe :
  metadataAllocationSafe afterMetadata := by
  simp [metadataAllocationSafe, afterMetadata, allocateMetadata, afterBoot, boot, initialMachine]

theorem m11_ddr_read_after_write_visible :
  ddrReadAfterWriteVisible afterRead := by
  simp [
    ddrReadAfterWriteVisible, afterRead, ddrRead, afterWrite, ddrWrite,
    afterMetadata, allocateMetadata, afterBoot, boot, initialMachine
  ]

theorem m11_stale_generation_fails_closed :
  staleGenerationFailsClosed afterStale := by
  simp [
    staleGenerationFailsClosed, afterStale, rejectStaleGeneration, afterRead,
    ddrRead, afterWrite, ddrWrite, afterMetadata, allocateMetadata, afterBoot,
    boot, initialMachine
  ]

theorem m11_cross_domain_metadata_rejected :
  crossDomainMetadataRejected afterCrossDomain := by
  simp [
    crossDomainMetadataRejected, afterCrossDomain, rejectCrossDomain, afterStale,
    rejectStaleGeneration, afterRead, ddrRead, afterWrite, ddrWrite,
    afterMetadata, allocateMetadata, afterBoot, boot, initialMachine
  ]

theorem m11_ecc_scrub_terminal :
  eccScrubTerminal afterEcc := by
  simp [
    eccScrubTerminal, afterEcc, eccScrub, afterCrossDomain, rejectCrossDomain,
    afterStale, rejectStaleGeneration, afterRead, ddrRead, afterWrite, ddrWrite,
    afterMetadata, allocateMetadata, afterBoot, boot, initialMachine
  ]

theorem m11_metadata_barrier_quiescent :
  metadataBarrierQuiescent finalMachine := by
  simp [
    metadataBarrierQuiescent, finalMachine, barrierFlush, afterEcc, eccScrub,
    afterCrossDomain, rejectCrossDomain, afterStale, rejectStaleGeneration,
    afterRead, ddrRead, afterWrite, ddrWrite, afterMetadata, allocateMetadata,
    afterBoot, boot, initialMachine
  ]

theorem m11_counts_exact :
  countsExact finalMachine := by
  simp [
    countsExact, finalMachine, barrierFlush, afterEcc, eccScrub,
    afterCrossDomain, rejectCrossDomain, afterStale, rejectStaleGeneration,
    afterRead, ddrRead, afterWrite, ddrWrite, afterMetadata, allocateMetadata,
    afterBoot, boot, initialMachine
  ]

/- Packed-bit decode machinery for the M11 DDR/metadata typed commit and state
   projection records. Mirrors the shared schema layout so the offline witness
   bits can be decoded back to projection fields and proved faithful in Lean
   with the kernel `decide` tactic (no native_decide, no axioms). -/

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

def rtlM11CommitPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("line_id", 32)
  , ("line_generation", 32)
  , ("domain_id", 32)
  , ("metadata_epoch", 32)
  , ("byte_len", 32)
  , ("data_value", 32) ]

def rtlM11StateProjectionPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("completions", 32)
  , ("faults", 32)
  , ("metadata_allocated", 1)
  , ("metadata_domain_bound", 1)
  , ("ddr_write_completed", 1)
  , ("ddr_read_completed", 1)
  , ("read_matches_write", 1)
  , ("stale_generation_rejected", 1)
  , ("cross_domain_rejected", 1)
  , ("ecc_scrubbed", 1)
  , ("barrier_quiescent", 1)
  , ("counts_exact", 1) ]

def rtlM11CommitPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM11CommitPackedSchema

def rtlM11StateProjectionPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM11StateProjectionPackedSchema

theorem rtlM11CommitPackedSchema_width :
    packedSchemaWidth rtlM11CommitPackedSchema = 216 := by
  decide

theorem rtlM11StateProjectionPackedSchema_width :
    packedSchemaWidth rtlM11StateProjectionPackedSchema = 98 := by
  decide

theorem rtlM11CommitPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM11CommitPackedSchema)
      rtlM11CommitPackedLayout = true := by
  decide

theorem rtlM11StateProjectionPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM11StateProjectionPackedSchema)
      rtlM11StateProjectionPackedLayout = true := by
  decide

end Lnp64.M11
