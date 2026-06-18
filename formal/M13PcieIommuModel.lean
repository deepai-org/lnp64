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

end Lnp64.M13
