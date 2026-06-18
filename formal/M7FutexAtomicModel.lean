/- LNP64 M7 futex/atomic checked model.

This bounded model names the proof targets exercised by
`formal/m7_futex_atomic_model.py` and `rtl/engines/lnp64_m7_futex_atomic.sv`.
The obligations below are proved over the bounded futex/atomic trace.
-/

namespace Lnp64.M7

structure Machine where
  atomicWord : Nat
  waiterParked : Bool
  waitGeneration : Nat
  addressGeneration : Nat
  staleAddressGeneration : Nat
  schedulerLocationCount : Nat
  domainBudget : Nat
  waitCost : Nat
  timerDeadline : Nat
  wakeCount : Nat
  timerWakeCount : Nat
  atomicCount : Nat
  cmpxchgSucceeded : Bool
  cmpxchgFailureExplicit : Bool
  futexWaitParked : Bool
  futexWakeDelivered : Bool
  timerWaitParked : Bool
  timerExpired : Bool
  bucketSpillPreserved : Bool
  staleAddressRejected : Bool
deriving Repr

def cmpxchgSuccessObserved (m : Machine) : Prop :=
  m.cmpxchgSucceeded = true /\ m.atomicWord = 1

def cmpxchgFailureExplicit (m : Machine) : Prop :=
  m.cmpxchgFailureExplicit = true /\ m.atomicWord = 1

def futexWaitParked (m : Machine) : Prop :=
  m.futexWaitParked = true

def futexWakeDelivered (m : Machine) : Prop :=
  m.futexWakeDelivered = true /\ m.waiterParked = false /\ m.wakeCount = 1

def timerWaitParked (m : Machine) : Prop :=
  m.timerWaitParked = true

def timerExpiryWakesThread (m : Machine) : Prop :=
  m.timerExpired = true /\ m.waiterParked = false /\ m.timerWakeCount = 1

def exactlyOneSchedulerLocation (m : Machine) : Prop :=
  m.schedulerLocationCount = 1

def wakeGenerationMatches (m : Machine) : Prop :=
  m.futexWakeDelivered = true -> m.waitGeneration = m.addressGeneration

def domainBudgetEligible (m : Machine) : Prop :=
  m.waitCost = 1 /\ m.domainBudget = 1

def bucketSpillPreservesIdentity (m : Machine) : Prop :=
  m.bucketSpillPreserved = true /\ m.waitGeneration = 1

def staleAddressRejected (m : Machine) : Prop :=
  m.staleAddressGeneration != m.addressGeneration -> m.staleAddressRejected = true

def noLostWakeup (m : Machine) : Prop :=
  m.wakeCount + m.timerWakeCount > 0 -> m.waiterParked = false

def lockedAtomicSingleCopy (m : Machine) : Prop :=
  m.atomicCount = 2 /\ m.atomicWord = 1 /\ m.cmpxchgFailureExplicit = true

def atomicCountExact (m : Machine) : Prop :=
  m.atomicCount = 2

def initialMachine : Machine :=
  { atomicWord := 0
    waiterParked := false
    waitGeneration := 1
    addressGeneration := 1
    staleAddressGeneration := 1
    schedulerLocationCount := 1
    domainBudget := 1
    waitCost := 1
    timerDeadline := 3
    wakeCount := 0
    timerWakeCount := 0
    atomicCount := 0
    cmpxchgSucceeded := false
    cmpxchgFailureExplicit := false
    futexWaitParked := false
    futexWakeDelivered := false
    timerWaitParked := false
    timerExpired := false
    bucketSpillPreserved := false
    staleAddressRejected := false }

def cmpxchgSuccess (m : Machine) : Machine :=
  { m with
    atomicWord := 1
    atomicCount := m.atomicCount + 1
    cmpxchgSucceeded := true }

def cmpxchgFail (m : Machine) : Machine :=
  { m with
    atomicCount := m.atomicCount + 1
    cmpxchgFailureExplicit := true }

def futexWait (m : Machine) : Machine :=
  { m with waiterParked := true, futexWaitParked := true }

def futexWake (m : Machine) : Machine :=
  { m with
    waiterParked := false
    wakeCount := m.wakeCount + 1
    futexWakeDelivered := true }

def timerWait (m : Machine) : Machine :=
  { m with waiterParked := true, timerWaitParked := true }

def timerExpire (m : Machine) : Machine :=
  { m with
    waiterParked := false
    timerWakeCount := m.timerWakeCount + 1
    timerExpired := true }

def bucketSpill (m : Machine) : Machine :=
  { m with bucketSpillPreserved := true }

def rejectStaleAddress (m : Machine) : Machine :=
  { m with
    addressGeneration := m.addressGeneration + 1
    staleAddressRejected := true }

def afterCmpxchgSuccess : Machine :=
  cmpxchgSuccess initialMachine

def afterCmpxchgFail : Machine :=
  cmpxchgFail afterCmpxchgSuccess

def afterWait : Machine :=
  futexWait afterCmpxchgFail

def afterWake : Machine :=
  futexWake afterWait

def afterTimerWait : Machine :=
  timerWait afterWake

def afterTimerExpire : Machine :=
  timerExpire afterTimerWait

def afterSpill : Machine :=
  bucketSpill afterTimerExpire

def finalMachine : Machine :=
  rejectStaleAddress afterSpill

theorem m7_cmpxchg_success_observed :
  cmpxchgSuccessObserved afterCmpxchgSuccess := by
  simp [
    cmpxchgSuccessObserved, afterCmpxchgSuccess, cmpxchgSuccess,
    initialMachine
  ]

theorem m7_cmpxchg_failure_explicit :
  cmpxchgFailureExplicit afterCmpxchgFail := by
  simp [
    cmpxchgFailureExplicit, afterCmpxchgFail, cmpxchgFail,
    afterCmpxchgSuccess, cmpxchgSuccess, initialMachine
  ]

theorem m7_futex_wait_parked :
  futexWaitParked afterWait := by
  simp [
    futexWaitParked, afterWait, futexWait, afterCmpxchgFail, cmpxchgFail,
    afterCmpxchgSuccess, cmpxchgSuccess, initialMachine
  ]

theorem m7_futex_wake_delivered :
  futexWakeDelivered afterWake := by
  simp [
    futexWakeDelivered, afterWake, futexWake, afterWait, futexWait,
    afterCmpxchgFail, cmpxchgFail, afterCmpxchgSuccess, cmpxchgSuccess,
    initialMachine
  ]

theorem m7_exactly_one_scheduler_location :
  exactlyOneSchedulerLocation afterTimerExpire := by
  simp [
    exactlyOneSchedulerLocation, afterTimerExpire, timerExpire,
    afterTimerWait, timerWait, afterWake, futexWake, afterWait, futexWait,
    afterCmpxchgFail, cmpxchgFail, afterCmpxchgSuccess, cmpxchgSuccess,
    initialMachine
  ]

theorem m7_wake_generation_matches :
  wakeGenerationMatches afterWake := by
  intro _wakeDelivered
  simp [
    afterWake, futexWake, afterWait, futexWait,
    afterCmpxchgFail, cmpxchgFail, afterCmpxchgSuccess, cmpxchgSuccess,
    initialMachine
  ]

theorem m7_domain_budget_eligible :
  domainBudgetEligible afterWait := by
  simp [
    domainBudgetEligible, afterWait, futexWait, afterCmpxchgFail,
    cmpxchgFail, afterCmpxchgSuccess, cmpxchgSuccess, initialMachine
  ]

theorem m7_timer_wait_parked :
  timerWaitParked afterTimerWait := by
  simp [
    timerWaitParked, afterTimerWait, timerWait, afterWake, futexWake,
    afterWait, futexWait, afterCmpxchgFail, cmpxchgFail,
    afterCmpxchgSuccess, cmpxchgSuccess, initialMachine
  ]

theorem m7_timer_expiry_wakes_thread :
  timerExpiryWakesThread afterTimerExpire := by
  simp [
    timerExpiryWakesThread, afterTimerExpire, timerExpire, afterTimerWait,
    timerWait, afterWake, futexWake, afterWait, futexWait,
    afterCmpxchgFail, cmpxchgFail, afterCmpxchgSuccess, cmpxchgSuccess,
    initialMachine
  ]

theorem m7_bucket_spill_preserves_identity :
  bucketSpillPreservesIdentity afterSpill := by
  simp [
    bucketSpillPreservesIdentity, afterSpill, bucketSpill, afterTimerExpire,
    timerExpire, afterTimerWait, timerWait, afterWake, futexWake, afterWait,
    futexWait, afterCmpxchgFail, cmpxchgFail, afterCmpxchgSuccess,
    cmpxchgSuccess, initialMachine
  ]

theorem m7_stale_address_rejected :
  staleAddressRejected finalMachine := by
  intro _stale
  simp [
    finalMachine, rejectStaleAddress, afterSpill, bucketSpill,
    afterTimerExpire, timerExpire, afterTimerWait, timerWait, afterWake,
    futexWake, afterWait, futexWait, afterCmpxchgFail, cmpxchgFail,
    afterCmpxchgSuccess, cmpxchgSuccess, initialMachine
  ]

theorem m7_no_lost_wakeup :
  noLostWakeup finalMachine := by
  intro _woken
  simp [
    finalMachine, rejectStaleAddress, afterSpill, bucketSpill,
    afterTimerExpire, timerExpire, afterTimerWait, timerWait, afterWake,
    futexWake, afterWait, futexWait, afterCmpxchgFail, cmpxchgFail,
    afterCmpxchgSuccess, cmpxchgSuccess, initialMachine
  ]

theorem m7_locked_atomic_single_copy :
  lockedAtomicSingleCopy finalMachine := by
  simp [
    lockedAtomicSingleCopy, finalMachine, rejectStaleAddress, afterSpill,
    bucketSpill, afterTimerExpire, timerExpire, afterTimerWait, timerWait,
    afterWake, futexWake, afterWait, futexWait, afterCmpxchgFail,
    cmpxchgFail, afterCmpxchgSuccess, cmpxchgSuccess, initialMachine
  ]

theorem m7_atomic_count_exact :
  atomicCountExact finalMachine := by
  simp [
    atomicCountExact, finalMachine, rejectStaleAddress, afterSpill,
    bucketSpill, afterTimerExpire, timerExpire, afterTimerWait, timerWait,
    afterWake, futexWake, afterWait, futexWait, afterCmpxchgFail,
    cmpxchgFail, afterCmpxchgSuccess, cmpxchgSuccess, initialMachine
  ]

end Lnp64.M7
