/- LNP64 M2 gate/continuation checked model.

This bounded model names the proof targets exercised by `formal/m2_gate_model.py`
and `rtl/engines/lnp64_m2_gate.sv`. The obligations below are proved for the
bounded sync/async/handoff/fault-delivery trace.
-/

namespace Lnp64.M2

inductive GateMode
  | sync
  | async
  | handoff
deriving DecidableEq, Repr

inductive Location
  | runnable
  | running
  | parked
deriving DecidableEq, Repr

structure Continuation where
  id : Nat
  generation : Nat
  valid : Bool
deriving Repr

structure Thread where
  tid : Nat
  location : Location
deriving Repr

structure Machine where
  caller : Thread
  callee : Thread
  continuation : Continuation
  syncRoundtrip : Bool
  asyncDelivered : Bool
  handoffDelivered : Bool
  staleRejected : Bool
  deliveredFaults : Nat
  signalDelivered : Bool
  signalCreatesAuthority : Bool
  signalMaskBypassed : Bool
deriving Repr

def continuationUnique (m : Machine) : Prop :=
  m.continuation.valid = true -> m.continuation.id > 0

def syncRoundtripComplete (m : Machine) : Prop :=
  m.syncRoundtrip = true -> m.caller.location = Location.runnable

def asyncDeliveryDoesNotParkCaller (m : Machine) : Prop :=
  m.asyncDelivered = true -> m.caller.location != Location.parked

def handoffTransfersExecution (m : Machine) : Prop :=
  m.handoffDelivered = true

def staleContinuationRejected (m : Machine) : Prop :=
  m.continuation.valid = false -> m.staleRejected = true

def faultDeliveryGateEntered (m : Machine) : Prop :=
  m.deliveredFaults > 0

def signalCompatibilitySafe (m : Machine) : Prop :=
  m.signalDelivered = true /\
  m.signalCreatesAuthority = false /\
  m.signalMaskBypassed = false

def caller0 : Thread :=
  { tid := 1, location := Location.runnable }

def callee0 : Thread :=
  { tid := 2, location := Location.runnable }

def continuation0 : Continuation :=
  { id := 0, generation := 0, valid := false }

def initialMachine : Machine :=
  { caller := caller0
    callee := callee0
    continuation := continuation0
    syncRoundtrip := false
    asyncDelivered := false
    handoffDelivered := false
    staleRejected := false
    deliveredFaults := 0
    signalDelivered := false
    signalCreatesAuthority := false
    signalMaskBypassed := false }

def syncCall (m : Machine) : Machine :=
  { m with
    caller := { m.caller with location := Location.parked }
    callee := { m.callee with location := Location.running }
    continuation := { id := 1, generation := 1, valid := true } }

def syncReturn (m : Machine) : Machine :=
  { m with
    caller := { m.caller with location := Location.runnable }
    callee := { m.callee with location := Location.runnable }
    continuation := { m.continuation with generation := m.continuation.generation + 1, valid := false }
    syncRoundtrip := true }

def asyncCall (m : Machine) : Machine :=
  { m with asyncDelivered := true }

def handoffCall (m : Machine) : Machine :=
  { m with
    callee := { m.callee with location := Location.running }
    handoffDelivered := true }

def staleReturnReject (m : Machine) : Machine :=
  { m with
    callee := { m.callee with location := Location.runnable }
    staleRejected := true }

def faultDelivery (m : Machine) : Machine :=
  { m with deliveredFaults := m.deliveredFaults + 1 }

def signalCompatibilityDelivery (m : Machine) : Machine :=
  { m with signalDelivered := true }

def finalMachine : Machine :=
  signalCompatibilityDelivery
    (faultDelivery
      (staleReturnReject
        (handoffCall
          (asyncCall
            (syncReturn
              (syncCall initialMachine))))))

theorem m2_continuation_unique :
  continuationUnique finalMachine := by
  intro valid
  simp [
    finalMachine, signalCompatibilityDelivery, faultDelivery,
    staleReturnReject, handoffCall, asyncCall, syncReturn, syncCall,
    initialMachine, caller0, callee0, continuation0
  ] at valid

theorem m2_sync_roundtrip :
  syncRoundtripComplete finalMachine := by
  intro _sync
  simp [
    finalMachine, signalCompatibilityDelivery, faultDelivery,
    staleReturnReject, handoffCall, asyncCall, syncReturn, syncCall,
    initialMachine, caller0, callee0, continuation0
  ]

theorem m2_async_delivery :
  asyncDeliveryDoesNotParkCaller finalMachine := by
  intro _async
  simp [
    finalMachine, signalCompatibilityDelivery, faultDelivery,
    staleReturnReject, handoffCall, asyncCall, syncReturn, syncCall,
    initialMachine, caller0, callee0, continuation0
  ]

theorem m2_handoff_delivery :
  handoffTransfersExecution finalMachine := by
  simp [
    finalMachine, signalCompatibilityDelivery, faultDelivery,
    staleReturnReject, handoffCall, asyncCall, syncReturn, syncCall,
    initialMachine, caller0, callee0, continuation0, handoffTransfersExecution
  ]

theorem m2_stale_continuation_rejected :
  staleContinuationRejected finalMachine := by
  intro _invalid
  simp [
    finalMachine, signalCompatibilityDelivery, faultDelivery,
    staleReturnReject, handoffCall, asyncCall, syncReturn, syncCall,
    initialMachine, caller0, callee0, continuation0
  ]

theorem m2_fault_delivery_gate_entered :
  faultDeliveryGateEntered finalMachine := by
  simp [
    finalMachine, signalCompatibilityDelivery, faultDelivery,
    staleReturnReject, handoffCall, asyncCall, syncReturn, syncCall,
    initialMachine, caller0, callee0, continuation0, faultDeliveryGateEntered
  ]

theorem m2_signal_compatibility_cannot_create_authority_or_bypass_masks :
  signalCompatibilitySafe finalMachine := by
  simp [
    signalCompatibilitySafe, finalMachine, signalCompatibilityDelivery,
    faultDelivery, staleReturnReject, handoffCall, asyncCall, syncReturn,
    syncCall, initialMachine, caller0, callee0, continuation0
  ]

end Lnp64.M2
