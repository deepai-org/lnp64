/- LNP64 M1 ping-pong proof skeleton.

This bounded model names the proof targets exercised by `formal/m1_model.py`
and `rtl/engines/lnp64_m1_pingpong.sv`.
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
  events : Nat
  staleRejected : Bool
  fullWasExplicit : Bool
deriving Repr

def hasRight (cap : Capability) (right : Nat) : Prop :=
  cap.rights.land right = right

def capMatchesQueue (cap : Capability) (queue : Queue) : Prop :=
  cap.objectId = queue.objectId /\ cap.generation = queue.generation

def exactlyOneLocation (_thread : Thread) : Prop := True

def noForgedFdr (m : Machine) : Prop :=
  capMatchesQueue m.producerCap m.queue /\
  (m.consumerCap.generation = m.queue.generation \/ m.staleRejected)

def noLostWakeup (m : Machine) : Prop :=
  m.events > 0 -> m.consumer.location != Location.parked

def explicitQueueFull (m : Machine) : Prop :=
  m.queue.contents.length = m.queue.capacity -> m.fullWasExplicit = true

def staleGenerationRejected (m : Machine) : Prop :=
  m.consumerCap.generation != m.queue.generation -> m.staleRejected = true

axiom m1_no_forged_fdr :
  forall m : Machine, noForgedFdr m

axiom m1_no_lost_wakeup :
  forall m : Machine, noLostWakeup m

axiom m1_exactly_one_scheduler_location :
  forall m : Machine, exactlyOneLocation m.producer /\ exactlyOneLocation m.consumer

axiom m1_queue_full_behavior_is_explicit :
  forall m : Machine, explicitQueueFull m

axiom m1_stale_generation_rejected :
  forall m : Machine, staleGenerationRejected m

end Lnp64.M1
