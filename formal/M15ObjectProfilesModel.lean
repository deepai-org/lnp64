/- LNP64 M15 object-profile checked model.

This bounded model covers the direct Track A A4 object-profile obligations:
counter threshold events, queue rights and explicit overflow, event-source
generation safety, and call-gate continuation uniqueness.
-/

namespace Lnp64.M15

structure QueueProfile where
  generation : Nat
  rights : Nat
  capacity : Nat
  depth : Nat
  overflowExplicit : Bool
deriving Repr

structure CounterProfile where
  value : Nat
  threshold : Nat
  eventGeneration : Nat
  thresholdEvent : Bool
deriving Repr

structure EventProfile where
  sourceGeneration : Nat
  eventGeneration : Nat
  delivered : Bool
  staleRejected : Bool
deriving Repr

structure GateProfile where
  continuationId : Nat
  duplicateContinuationRejected : Bool
deriving Repr

structure Machine where
  queue : QueueProfile
  counter : CounterProfile
  eventQueue : EventProfile
  gate : GateProfile
  failures : Nat
deriving Repr

def eagain : Nat := 11
def erevoked : Nat := 122
def rightPush : Nat := 1
def rightPull : Nat := 2
def rightEventEmit : Nat := 4

def initialMachine : Machine :=
  { queue := { generation := 1, rights := rightPush + rightPull, capacity := 1, depth := 0, overflowExplicit := false }
    counter := { value := 0, threshold := 3, eventGeneration := 1, thresholdEvent := false }
    eventQueue := { sourceGeneration := 1, eventGeneration := 1, delivered := false, staleRejected := false }
    gate := { continuationId := 0, duplicateContinuationRejected := false }
    failures := 0 }

def incrementCounterToThreshold (m : Machine) : Machine :=
  { m with counter := { m.counter with value := 3, thresholdEvent := true } }

def pushQueueWithRights (m : Machine) : Machine :=
  { m with queue := { m.queue with depth := 1 } }

def rejectQueueOverflow (m : Machine) : Machine :=
  { m with queue := { m.queue with overflowExplicit := true }, failures := m.failures + 1 }

def deliverEventWithMatchingGeneration (m : Machine) : Machine :=
  { m with eventQueue := { m.eventQueue with delivered := true } }

def rejectStaleEventSource (m : Machine) : Machine :=
  { m with
    eventQueue :=
      { m.eventQueue with
        sourceGeneration := m.eventQueue.sourceGeneration + 1
        staleRejected := true }
    failures := m.failures + 1 }

def allocateGateContinuation (m : Machine) : Machine :=
  { m with gate := { m.gate with continuationId := 1 } }

def rejectDuplicateContinuation (m : Machine) : Machine :=
  { m with
    gate := { m.gate with duplicateContinuationRejected := true }
    failures := m.failures + 1 }

def afterCounter : Machine :=
  incrementCounterToThreshold initialMachine

def afterQueuePush : Machine :=
  pushQueueWithRights afterCounter

def afterOverflow : Machine :=
  rejectQueueOverflow afterQueuePush

def afterEvent : Machine :=
  deliverEventWithMatchingGeneration afterOverflow

def afterStaleEvent : Machine :=
  rejectStaleEventSource afterEvent

def afterGate : Machine :=
  allocateGateContinuation afterStaleEvent

def finalMachine : Machine :=
  rejectDuplicateContinuation afterGate

def counterThresholdEvent (m : Machine) : Prop :=
  m.counter.value = m.counter.threshold /\ m.counter.thresholdEvent = true

def queueRightsAllowPush (m : Machine) : Prop :=
  m.queue.rights.land rightPush = rightPush /\ m.queue.depth = 1

def queueOverflowExplicit (m : Machine) : Prop :=
  m.queue.depth = m.queue.capacity -> m.queue.overflowExplicit = true

def eventSourceGenerationSafe (m : Machine) : Prop :=
  m.eventQueue.delivered = true /\ m.eventQueue.staleRejected = true

def gateContinuationUnique (m : Machine) : Prop :=
  m.gate.continuationId = 1 /\ m.gate.duplicateContinuationRejected = true

def countsExact (m : Machine) : Prop :=
  m.failures = 3

theorem m15_counter_threshold_event :
  counterThresholdEvent afterCounter := by
  simp [
    counterThresholdEvent, afterCounter, incrementCounterToThreshold,
    initialMachine, rightPush, rightPull
  ]

theorem m15_queue_rights_allow_push :
  queueRightsAllowPush afterQueuePush := by
  simp [
    queueRightsAllowPush, afterQueuePush, pushQueueWithRights, afterCounter,
    incrementCounterToThreshold, initialMachine, rightPush, rightPull
  ]

theorem m15_queue_overflow_explicit :
  queueOverflowExplicit afterOverflow := by
  intro _full
  simp [
    afterOverflow, rejectQueueOverflow, afterQueuePush, pushQueueWithRights,
    afterCounter, incrementCounterToThreshold, initialMachine, rightPush,
    rightPull
  ]

theorem m15_event_source_generation_safe :
  eventSourceGenerationSafe afterStaleEvent := by
  simp [
    eventSourceGenerationSafe, afterStaleEvent, rejectStaleEventSource,
    afterEvent, deliverEventWithMatchingGeneration, afterOverflow,
    rejectQueueOverflow, afterQueuePush, pushQueueWithRights, afterCounter,
    incrementCounterToThreshold, initialMachine, rightPush, rightPull
  ]

theorem m15_gate_continuation_unique :
  gateContinuationUnique finalMachine := by
  simp [
    gateContinuationUnique, finalMachine, rejectDuplicateContinuation,
    afterGate, allocateGateContinuation, afterStaleEvent,
    rejectStaleEventSource, afterEvent, deliverEventWithMatchingGeneration,
    afterOverflow, rejectQueueOverflow, afterQueuePush, pushQueueWithRights,
    afterCounter, incrementCounterToThreshold, initialMachine, rightPush,
    rightPull
  ]

theorem m15_counts_exact :
  countsExact finalMachine := by
  simp [
    countsExact, finalMachine, rejectDuplicateContinuation, afterGate,
    allocateGateContinuation, afterStaleEvent, rejectStaleEventSource,
    afterEvent, deliverEventWithMatchingGeneration, afterOverflow,
    rejectQueueOverflow, afterQueuePush, pushQueueWithRights, afterCounter,
    incrementCounterToThreshold, initialMachine, rightPush, rightPull
  ]

end Lnp64.M15
