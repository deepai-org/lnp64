/- LNP64 M13 PCIe/IOMMU/MSI checked model.

This bounded model covers the first Track D step 17 smoke slice: PCIe root
enumeration, BAR capability metadata, IOMMU-scoped DMA admission, MSI event
delivery, fail-closed rejection for unbound bus mastering and stale BAR
generations, malformed config-space termination, and absence of raw DMA or raw
interrupt authority in software-visible state.
-/

namespace Lnp64.M13

structure Machine where
  rootDomain : Nat
  deviceEnumerated : Bool
  barCapabilityCreated : Bool
  iommuBoundToDomain : Bool
  scopedDmaCompleted : Bool
  msiEventDelivered : Bool
  unboundBusMasterRejected : Bool
  staleBarRejected : Bool
  malformedConfigRejected : Bool
  rawDmaAuthorityVisible : Bool
  rawInterruptVisible : Bool
  completions : Nat
  faults : Nat
deriving Repr

def enumerationSafe (m : Machine) : Prop :=
  m.rootDomain > 0 /\ m.deviceEnumerated = true /\ m.barCapabilityCreated = true

def iommuScopedDmaSafe (m : Machine) : Prop :=
  m.iommuBoundToDomain = true /\ m.scopedDmaCompleted = true /\ m.completions >= 2

def msiDeliveredAsEvent (m : Machine) : Prop :=
  m.msiEventDelivered = true /\ m.completions >= 3

def unboundBusMasterFailsClosed (m : Machine) : Prop :=
  m.unboundBusMasterRejected = true /\ m.faults >= 1

def staleBarFailsClosed (m : Machine) : Prop :=
  m.staleBarRejected = true /\ m.faults >= 2

def malformedConfigFailsClosed (m : Machine) : Prop :=
  m.malformedConfigRejected = true /\ m.faults >= 3

def noRawPcieAuthority (m : Machine) : Prop :=
  m.rawDmaAuthorityVisible = false /\ m.rawInterruptVisible = false

def countsExact (m : Machine) : Prop :=
  m.completions = 3 /\ m.faults = 3

def initialMachine : Machine :=
  { rootDomain := 0
    deviceEnumerated := false
    barCapabilityCreated := false
    iommuBoundToDomain := false
    scopedDmaCompleted := false
    msiEventDelivered := false
    unboundBusMasterRejected := false
    staleBarRejected := false
    malformedConfigRejected := false
    rawDmaAuthorityVisible := false
    rawInterruptVisible := false
    completions := 0
    faults := 0 }

def boot (m : Machine) : Machine :=
  { m with rootDomain := 1 }

def enumerateDevice (m : Machine) : Machine :=
  { m with
    deviceEnumerated := true
    barCapabilityCreated := true
    completions := m.completions + 1 }

def bindIommuAndDma (m : Machine) : Machine :=
  { m with
    iommuBoundToDomain := true
    scopedDmaCompleted := true
    completions := m.completions + 1 }

def deliverMsi (m : Machine) : Machine :=
  { m with msiEventDelivered := true, completions := m.completions + 1 }

def rejectUnboundBusMaster (m : Machine) : Machine :=
  { m with unboundBusMasterRejected := true, faults := m.faults + 1 }

def rejectStaleBar (m : Machine) : Machine :=
  { m with staleBarRejected := true, faults := m.faults + 1 }

def rejectMalformedConfig (m : Machine) : Machine :=
  { m with malformedConfigRejected := true, faults := m.faults + 1 }

def afterBoot : Machine :=
  boot initialMachine

def afterEnumeration : Machine :=
  enumerateDevice afterBoot

def afterDma : Machine :=
  bindIommuAndDma afterEnumeration

def afterMsi : Machine :=
  deliverMsi afterDma

def afterUnbound : Machine :=
  rejectUnboundBusMaster afterMsi

def afterStale : Machine :=
  rejectStaleBar afterUnbound

def finalMachine : Machine :=
  rejectMalformedConfig afterStale

theorem m13_enumeration_safe :
  enumerationSafe afterEnumeration := by
  simp [enumerationSafe, afterEnumeration, enumerateDevice, afterBoot, boot, initialMachine]

theorem m13_iommu_scoped_dma_safe :
  iommuScopedDmaSafe afterDma := by
  simp [
    iommuScopedDmaSafe, afterDma, bindIommuAndDma, afterEnumeration,
    enumerateDevice, afterBoot, boot, initialMachine
  ]

theorem m13_msi_delivered_as_event :
  msiDeliveredAsEvent afterMsi := by
  simp [
    msiDeliveredAsEvent, afterMsi, deliverMsi, afterDma, bindIommuAndDma,
    afterEnumeration, enumerateDevice, afterBoot, boot, initialMachine
  ]

theorem m13_unbound_bus_master_rejected :
  unboundBusMasterFailsClosed afterUnbound := by
  simp [
    unboundBusMasterFailsClosed, afterUnbound, rejectUnboundBusMaster,
    afterMsi, deliverMsi, afterDma, bindIommuAndDma, afterEnumeration,
    enumerateDevice, afterBoot, boot, initialMachine
  ]

theorem m13_stale_bar_rejected :
  staleBarFailsClosed afterStale := by
  simp [
    staleBarFailsClosed, afterStale, rejectStaleBar, afterUnbound,
    rejectUnboundBusMaster, afterMsi, deliverMsi, afterDma, bindIommuAndDma,
    afterEnumeration, enumerateDevice, afterBoot, boot, initialMachine
  ]

theorem m13_malformed_config_rejected :
  malformedConfigFailsClosed finalMachine := by
  simp [
    malformedConfigFailsClosed, finalMachine, rejectMalformedConfig,
    afterStale, rejectStaleBar, afterUnbound, rejectUnboundBusMaster,
    afterMsi, deliverMsi, afterDma, bindIommuAndDma, afterEnumeration,
    enumerateDevice, afterBoot, boot, initialMachine
  ]

theorem m13_no_raw_pcie_authority :
  noRawPcieAuthority finalMachine := by
  simp [
    noRawPcieAuthority, finalMachine, rejectMalformedConfig, afterStale,
    rejectStaleBar, afterUnbound, rejectUnboundBusMaster, afterMsi,
    deliverMsi, afterDma, bindIommuAndDma, afterEnumeration, enumerateDevice,
    afterBoot, boot, initialMachine
  ]

theorem m13_counts_exact :
  countsExact finalMachine := by
  simp [
    countsExact, finalMachine, rejectMalformedConfig, afterStale,
    rejectStaleBar, afterUnbound, rejectUnboundBusMaster, afterMsi,
    deliverMsi, afterDma, bindIommuAndDma, afterEnumeration, enumerateDevice,
    afterBoot, boot, initialMachine
  ]

/- Packed-bit decode machinery for the M13 PCIe/IOMMU typed commit and state
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

def rtlM13CommitPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("requester_id", 32)
  , ("bar_id", 32)
  , ("bar_generation", 32)
  , ("domain_id", 32)
  , ("iommu_context", 32)
  , ("dma_bytes", 32) ]

def rtlM13StateProjectionPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("completions", 32)
  , ("faults", 32)
  , ("device_enumerated", 1)
  , ("bar_capability_created", 1)
  , ("iommu_bound_to_domain", 1)
  , ("scoped_dma_completed", 1)
  , ("msi_event_delivered", 1)
  , ("unbound_bus_master_rejected", 1)
  , ("stale_bar_rejected", 1)
  , ("malformed_config_rejected", 1)
  , ("no_raw_pcie_authority", 1)
  , ("counts_exact", 1) ]

def rtlM13CommitPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM13CommitPackedSchema

def rtlM13StateProjectionPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM13StateProjectionPackedSchema

theorem rtlM13CommitPackedSchema_width :
    packedSchemaWidth rtlM13CommitPackedSchema = 216 := by
  decide

theorem rtlM13StateProjectionPackedSchema_width :
    packedSchemaWidth rtlM13StateProjectionPackedSchema = 98 := by
  decide

theorem rtlM13CommitPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM13CommitPackedSchema)
      rtlM13CommitPackedLayout = true := by
  decide

theorem rtlM13StateProjectionPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM13StateProjectionPackedSchema)
      rtlM13StateProjectionPackedLayout = true := by
  decide

end Lnp64.M13
