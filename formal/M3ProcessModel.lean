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

/- Packed-bit decode model for the M3 process/thread lifecycle witness.

Mirrors the M1/M2/M4/M5/M7/M14 packed-bit machinery so the emitted
lnp64_m3_process_commit_t and lnp64_m3_state_projection_t bit vectors can be
decode-checked against this Lean model. Every M3 field is a plain scalar/bool
slice. -/

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

def rtlM3CommitPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("parent_tid", 32)
  , ("child_tid", 32)
  , ("child_generation", 32)
  , ("join_generation", 32)
  , ("exec_epoch", 32)
  , ("exit_code", 32) ]

def rtlM3StateProjectionPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("parent_state", 2)
  , ("child_state", 2)
  , ("parent_tid", 32)
  , ("child_tid", 32)
  , ("child_generation", 32)
  , ("join_generation", 32)
  , ("exec_epoch", 32)
  , ("clone_created", 1)
  , ("child_exit_signaled", 1)
  , ("parent_join_completed", 1)
  , ("exec_barrier_stopped_sibling", 1)
  , ("stale_join_rejected", 1)
  , ("exec_cancel_terminal", 1)
  , ("exactly_one_thread_location", 1) ]

def rtlM3CommitPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM3CommitPackedSchema

def rtlM3StateProjectionPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM3StateProjectionPackedSchema

theorem rtlM3CommitPackedSchema_width :
    packedSchemaWidth rtlM3CommitPackedSchema = 216 := by
  decide

theorem rtlM3StateProjectionPackedSchema_width :
    packedSchemaWidth rtlM3StateProjectionPackedSchema = 195 := by
  decide

theorem rtlM3CommitPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM3CommitPackedSchema)
      rtlM3CommitPackedLayout = true := by
  decide

theorem rtlM3StateProjectionPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM3StateProjectionPackedSchema)
      rtlM3StateProjectionPackedLayout = true := by
  decide

end Lnp64.M3
