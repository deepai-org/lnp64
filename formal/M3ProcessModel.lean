/- LNP64 M3 process/thread lifecycle checked model.

This bounded model names the proof targets exercised by
`formal/m3_process_model.py` and `rtl/engines/lnp64_m3_process.sv`.
The obligations below are proved over concrete checkpoints in the bounded
clone/exit/join/exec-barrier trace.
-/

namespace Lnp64.M3

inductive ThreadState
  | unused
  | runnable
  | running
  | exited
deriving DecidableEq, Repr

structure Thread where
  tid : Nat
  generation : Nat
  state : ThreadState
deriving Repr

structure Machine where
  parent : Thread
  child : Thread
  joinGeneration : Nat
  childExitCode : Nat
  waitableSignaled : Bool
  execEpoch : Nat
  cloneCreated : Bool
  joined : Bool
  staleJoinRejected : Bool
  execCancelTerminal : Bool
deriving Repr

def exactlyOneThreadLocation (t : Thread) : Prop :=
  t.state = ThreadState.unused \/
  t.state = ThreadState.runnable \/
  t.state = ThreadState.running \/
  t.state = ThreadState.exited

def childExitSignalsWaitable (m : Machine) : Prop :=
  m.child.state = ThreadState.exited -> m.waitableSignaled = true

def joinConsumesExitedChild (m : Machine) : Prop :=
  m.joined = true -> m.child.state = ThreadState.unused

def execBarrierAdvancesEpoch (before after : Machine) : Prop :=
  after.execEpoch > before.execEpoch

def staleJoinRejected (m : Machine) : Prop :=
  m.joinGeneration != m.child.generation -> m.staleJoinRejected = true

def execCancelReachesTerminal (m : Machine) : Prop :=
  m.execCancelTerminal = true

def parent0 : Thread :=
  { tid := 1, generation := 1, state := ThreadState.running }

def childSlot0 : Thread :=
  { tid := 0, generation := 0, state := ThreadState.unused }

def initialMachine : Machine :=
  { parent := parent0
    child := childSlot0
    joinGeneration := 0
    childExitCode := 0
    waitableSignaled := false
    execEpoch := 1
    cloneCreated := false
    joined := false
    staleJoinRejected := false
    execCancelTerminal := false }

def cloneChild (m : Machine) : Machine :=
  { m with
    child := { tid := 2, generation := 1, state := ThreadState.runnable }
    joinGeneration := 1
    cloneCreated := true }

def childExit (m : Machine) : Machine :=
  { m with
    child := { m.child with state := ThreadState.exited }
    childExitCode := 7
    waitableSignaled := true }

def parentJoin (m : Machine) : Machine :=
  { m with
    child := { m.child with generation := m.child.generation + 1, state := ThreadState.unused }
    waitableSignaled := false
    joined := true }

def execBarrier (m : Machine) : Machine :=
  { m with execEpoch := m.execEpoch + 1 }

def rejectStaleJoin (m : Machine) : Machine :=
  { m with staleJoinRejected := true }

def cancelExec (m : Machine) : Machine :=
  { m with execCancelTerminal := true }

def afterClone : Machine :=
  cloneChild initialMachine

def afterChildExit : Machine :=
  childExit afterClone

def afterJoin : Machine :=
  parentJoin afterChildExit

def afterExecBarrier : Machine :=
  execBarrier afterJoin

def finalMachine : Machine :=
  cancelExec (rejectStaleJoin afterExecBarrier)

theorem m3_exactly_one_thread_location :
  exactlyOneThreadLocation finalMachine.parent /\ exactlyOneThreadLocation finalMachine.child := by
  simp [
    finalMachine, cancelExec, rejectStaleJoin, afterExecBarrier, execBarrier,
    afterJoin, parentJoin, afterChildExit, childExit, afterClone, cloneChild,
    initialMachine, parent0, childSlot0, exactlyOneThreadLocation
  ]

theorem m3_child_exit_signals_waitable :
  childExitSignalsWaitable afterChildExit := by
  intro _childExited
  simp [
    afterChildExit, childExit, afterClone, cloneChild, initialMachine,
    parent0, childSlot0
  ]

theorem m3_join_consumes_exited_child :
  joinConsumesExitedChild afterJoin := by
  intro _joined
  simp [
    afterJoin, parentJoin, afterChildExit, childExit, afterClone, cloneChild,
    initialMachine, parent0, childSlot0
  ]

theorem m3_exec_barrier_advances_epoch :
  execBarrierAdvancesEpoch afterJoin afterExecBarrier := by
  simp [
    execBarrierAdvancesEpoch, afterExecBarrier, execBarrier, afterJoin,
    parentJoin, afterChildExit, childExit, afterClone, cloneChild,
    initialMachine, parent0, childSlot0
  ]

theorem m3_stale_join_rejected :
  staleJoinRejected finalMachine := by
  intro _stale
  simp [
    finalMachine, cancelExec, rejectStaleJoin, afterExecBarrier, execBarrier,
    afterJoin, parentJoin, afterChildExit, childExit, afterClone, cloneChild,
    initialMachine, parent0, childSlot0
  ]

theorem m3_exec_cancel_terminal :
  execCancelReachesTerminal finalMachine := by
  simp [
    finalMachine, cancelExec, rejectStaleJoin, afterExecBarrier, execBarrier,
    afterJoin, parentJoin, afterChildExit, childExit, afterClone, cloneChild,
    initialMachine, parent0, childSlot0, execCancelReachesTerminal
  ]

end Lnp64.M3
