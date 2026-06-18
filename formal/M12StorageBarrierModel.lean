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

end Lnp64.M12
