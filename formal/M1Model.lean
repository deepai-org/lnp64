/- LNP64 M1 ping-pong checked model.

This bounded model names the proof targets exercised by `formal/m1_model.py`
and `rtl/engines/lnp64_m1_pingpong.sv`. Unlike the broader roadmap skeletons,
the M1 obligations below are proved for the concrete bounded trace.
-/

namespace Lnp64.M1

structure Capability where
  objectId : Nat
  generation : Nat
  rights : Nat
  lineageEpoch : Nat
  sealed : Bool
deriving Repr

structure Queue where
  objectId : Nat
  generation : Nat
  capacity : Nat
  contents : List Nat
deriving Repr

inductive Location
  | runnable
  | running
  | parked
deriving DecidableEq, Repr

structure Thread where
  tid : Nat
  location : Location
  waitGeneration : Nat
deriving Repr

structure Machine where
  producer : Thread
  consumer : Thread
  queue : Queue
  producerCap : Capability
  consumerCap : Capability
  sentCap : Option Capability
  receivedCap : Option Capability
  events : Nat
  sendCompleted : Bool
  recvCompleted : Bool
  revokeCompleted : Bool
  staleRejected : Bool
  fullWasExplicit : Bool
deriving Repr

def hasRight (cap : Capability) (right : Nat) : Prop :=
  cap.rights.land right = right

def capMatchesQueue (cap : Capability) (queue : Queue) : Prop :=
  cap.objectId = queue.objectId /\ cap.generation = queue.generation

def capAuthorizedOrRejected (cap : Capability) (queue : Queue) (rejected : Bool) : Prop :=
  cap.objectId = queue.objectId /\ (cap.generation = queue.generation \/ rejected = true)

def exactlyOneLocation (thread : Thread) : Prop :=
  thread.location = Location.runnable \/
  thread.location = Location.running \/
  thread.location = Location.parked

def rightPush : Nat := 1
def rightPull : Nat := 2
def rightDup : Nat := 4

def noForgedFdr (m : Machine) : Prop :=
  capAuthorizedOrRejected m.producerCap m.queue m.staleRejected /\
  capAuthorizedOrRejected m.consumerCap m.queue m.staleRejected

def rightsDoNotAmplify (child parent : Capability) : Prop :=
  child.rights = rightPull /\
  parent.rights = rightPush + rightPull + rightDup /\
  child.lineageEpoch = parent.lineageEpoch /\
  child.sealed = parent.sealed

def noAuthorityAmplification (m : Machine) : Prop :=
  rightsDoNotAmplify m.consumerCap m.producerCap

def capSendPreservesNarrowing (m : Machine) : Prop :=
  m.sendCompleted = true /\
  m.sentCap = some m.consumerCap /\
  noAuthorityAmplification m

def capRecvInstallsSentCap (m : Machine) : Prop :=
  m.recvCompleted = true /\
  m.receivedCap = m.sentCap

def capRevokeInvalidatesGeneration (m : Machine) : Prop :=
  m.revokeCompleted = true /\
  m.consumerCap.generation != m.queue.generation

def revokedAuthorityCannotStartNewWork (m : Machine) : Prop :=
  m.consumerCap.generation != m.queue.generation -> m.staleRejected = true

def noLostWakeup (m : Machine) : Prop :=
  m.events > 0 -> m.consumer.location != Location.parked

def explicitQueueFull (m : Machine) : Prop :=
  m.queue.contents.length = m.queue.capacity -> m.fullWasExplicit = true

def staleGenerationRejected (m : Machine) : Prop :=
  m.consumerCap.generation != m.queue.generation -> m.staleRejected = true

def producerThread : Thread :=
  { tid := 1, location := Location.runnable, waitGeneration := 1 }

def consumerThread : Thread :=
  { tid := 2, location := Location.runnable, waitGeneration := 1 }

def queue0 : Queue :=
  { objectId := 1, generation := 1, capacity := 1, contents := [] }

def producerCap0 : Capability :=
  { objectId := 1
    generation := 1
    rights := rightPush + rightPull + rightDup
    lineageEpoch := 1
    sealed := false }

def initialMachine : Machine :=
  { producer := producerThread
    consumer := consumerThread
    queue := queue0
    producerCap := producerCap0
    consumerCap := { producerCap0 with rights := 0 }
    sentCap := none
    receivedCap := none
    events := 0
    sendCompleted := false
    recvCompleted := false
    revokeCompleted := false
    staleRejected := false
    fullWasExplicit := false }

def capDupConsumer (m : Machine) : Machine :=
  { m with consumerCap := { m.producerCap with rights := rightPull } }

def capSend (m : Machine) : Machine :=
  { m with sentCap := some m.consumerCap, sendCompleted := true }

def capRecv (m : Machine) : Machine :=
  match m.sentCap with
  | some cap => { m with receivedCap := some cap, recvCompleted := true }
  | none => m

def capRevoke (m : Machine) : Machine :=
  { m with
    queue := { m.queue with generation := m.queue.generation + 1 }
    revokeCompleted := true }

def awaitEmptyQueue (m : Machine) : Machine :=
  { m with
    consumer := { m.consumer with
      location := Location.parked
      waitGeneration := m.queue.generation } }

def pushValue (m : Machine) (value : Nat) : Machine :=
  { m with
    queue := { m.queue with contents := [value] }
    consumer := { m.consumer with location := Location.runnable }
    events := m.events + 1 }

def pullValue (m : Machine) : Machine :=
  { m with queue := { m.queue with contents := [] } }

def refillQueue (m : Machine) (value : Nat) : Machine :=
  { m with queue := { m.queue with contents := [value] } }

def rejectFullQueue (m : Machine) : Machine :=
  { m with fullWasExplicit := true }

def revokeQueue (m : Machine) : Machine :=
  { m with queue := { m.queue with generation := m.queue.generation + 1 } }

def rejectStalePull (m : Machine) : Machine :=
  { m with staleRejected := true }

def afterCapSend : Machine :=
  capSend (capDupConsumer initialMachine)

def afterCapRecv : Machine :=
  capRecv afterCapSend

def afterCapRevoke : Machine :=
  capRevoke afterCapRecv

def finalMachine : Machine :=
  rejectStalePull
    (revokeQueue
      (rejectFullQueue
        (refillQueue
          (pullValue
            (pushValue
              (awaitEmptyQueue
                (capDupConsumer initialMachine))
              42))
          7)))

theorem m1_no_forged_fdr :
  noForgedFdr finalMachine := by
  simp [
    finalMachine, rejectStalePull, revokeQueue, rejectFullQueue, refillQueue,
    pullValue, pushValue, awaitEmptyQueue, capDupConsumer, initialMachine,
    producerThread, consumerThread, queue0, producerCap0, noForgedFdr,
    capAuthorizedOrRejected
  ]

theorem m1_no_authority_amplification :
  noAuthorityAmplification finalMachine := by
  simp [
    finalMachine, rejectStalePull, revokeQueue, rejectFullQueue, refillQueue,
    pullValue, pushValue, awaitEmptyQueue, capDupConsumer, initialMachine,
    producerThread, consumerThread, queue0, producerCap0,
    noAuthorityAmplification, rightsDoNotAmplify
  ]

theorem m1_cap_send_preserves_narrowing :
  capSendPreservesNarrowing afterCapSend := by
  simp [
    afterCapSend, capSend, capDupConsumer, initialMachine, producerThread,
    consumerThread, queue0, producerCap0, capSendPreservesNarrowing,
    noAuthorityAmplification, rightsDoNotAmplify
  ]

theorem m1_cap_recv_installs_sent_cap :
  capRecvInstallsSentCap afterCapRecv := by
  simp [
    afterCapRecv, afterCapSend, capRecv, capSend, capDupConsumer,
    initialMachine, producerThread, consumerThread, queue0, producerCap0,
    capRecvInstallsSentCap
  ]

theorem m1_cap_revoke_invalidates_generation :
  capRevokeInvalidatesGeneration afterCapRevoke := by
  simp [
    afterCapRevoke, afterCapRecv, afterCapSend, capRevoke, capRecv, capSend,
    capDupConsumer, initialMachine, producerThread, consumerThread, queue0,
    producerCap0, capRevokeInvalidatesGeneration
  ]

theorem m1_revoked_authority_cannot_start_new_work :
  revokedAuthorityCannotStartNewWork finalMachine := by
  intro _revoked
  simp [
    finalMachine, rejectStalePull, revokeQueue, rejectFullQueue, refillQueue,
    pullValue, pushValue, awaitEmptyQueue, capDupConsumer, initialMachine,
    producerThread, consumerThread, queue0, producerCap0
  ]

theorem m1_no_lost_wakeup :
  noLostWakeup finalMachine := by
  intro _eventsPositive
  simp [
    finalMachine, rejectStalePull, revokeQueue, rejectFullQueue, refillQueue,
    pullValue, pushValue, awaitEmptyQueue, capDupConsumer, initialMachine,
    producerThread, consumerThread, queue0, producerCap0
  ]

theorem m1_exactly_one_scheduler_location :
  exactlyOneLocation finalMachine.producer /\ exactlyOneLocation finalMachine.consumer := by
  simp [
    finalMachine, rejectStalePull, revokeQueue, rejectFullQueue, refillQueue,
    pullValue, pushValue, awaitEmptyQueue, capDupConsumer, initialMachine,
    producerThread, consumerThread, queue0, producerCap0, exactlyOneLocation
  ]

theorem m1_queue_full_behavior_is_explicit :
  explicitQueueFull finalMachine := by
  intro _queueFull
  simp [
    finalMachine, rejectStalePull, revokeQueue, rejectFullQueue, refillQueue,
    pullValue, pushValue, awaitEmptyQueue, capDupConsumer, initialMachine,
    producerThread, consumerThread, queue0, producerCap0
  ]

theorem m1_stale_generation_rejected :
  staleGenerationRejected finalMachine := by
  intro _stale
  simp [
    finalMachine, rejectStalePull, revokeQueue, rejectFullQueue, refillQueue,
    pullValue, pushValue, awaitEmptyQueue, capDupConsumer, initialMachine,
    producerThread, consumerThread, queue0, producerCap0
  ]

end Lnp64.M1
