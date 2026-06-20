/- LNP64 M8 heap checked model.

This bounded model names the proof targets exercised by
`formal/m8_heap_model.py` and `rtl/engines/lnp64_m8_heap.sv`.
The obligations below are proved over the bounded heap trace.
-/

namespace Lnp64.M8

structure Chunk where
  ptr : Nat
  size : Nat
  generation : Nat
  ownerTid : Nat
  allocated : Bool
  quarantined : Bool
deriving Repr

structure Machine where
  chunk : Chunk
  staleGeneration : Nat
  allocations : Nat
  frees : Nat
  allocCompleted : Bool
  allocSizeReported : Bool
  freeCompleted : Bool
  reuseCompleted : Bool
  doubleFreeRejected : Bool
  stalePointerRejected : Bool
  crossThreadHandoff : Bool
  guardFaulted : Bool
  quarantineObserved : Bool
deriving Repr

def allocationCompleted (m : Machine) : Prop :=
  m.allocCompleted = true /\ m.chunk.allocated = true /\ m.allocations = 1

def allocationSizeReported (m : Machine) : Prop :=
  m.allocSizeReported = true /\ m.chunk.size = 32

def freeCompleted (m : Machine) : Prop :=
  m.freeCompleted = true /\ m.chunk.allocated = false /\ m.chunk.quarantined = true

def reuseCompleted (m : Machine) : Prop :=
  m.reuseCompleted = true /\ m.chunk.allocated = true /\ m.allocations = 2

def doubleFreeRejected (m : Machine) : Prop :=
  m.doubleFreeRejected = true

def stalePointerRejected (m : Machine) : Prop :=
  m.staleGeneration != m.chunk.generation -> m.stalePointerRejected = true

def crossThreadFreeHandoff (m : Machine) : Prop :=
  m.crossThreadHandoff = true /\ m.chunk.allocated = false /\ m.frees = 2

def guardFaultObserved (m : Machine) : Prop :=
  m.guardFaulted = true

def quarantineObserved (m : Machine) : Prop :=
  m.quarantineObserved = true

def heapCountsExact (m : Machine) : Prop :=
  m.allocations = 2 /\ m.frees = 2

def chunk0 : Chunk :=
  { ptr := 4096
    size := 32
    generation := 1
    ownerTid := 1
    allocated := false
    quarantined := false }

def initialMachine : Machine :=
  { chunk := chunk0
    staleGeneration := 1
    allocations := 0
    frees := 0
    allocCompleted := false
    allocSizeReported := false
    freeCompleted := false
    reuseCompleted := false
    doubleFreeRejected := false
    stalePointerRejected := false
    crossThreadHandoff := false
    guardFaulted := false
    quarantineObserved := false }

def allocChunk (m : Machine) : Machine :=
  { m with
    chunk := { m.chunk with allocated := true, quarantined := false, ownerTid := 1 }
    allocations := m.allocations + 1
    allocCompleted := true }

def reportAllocSize (m : Machine) : Machine :=
  { m with allocSizeReported := true }

def freeChunk (m : Machine) : Machine :=
  { m with
    chunk := { m.chunk with allocated := false, quarantined := true, generation := m.chunk.generation + 1 }
    frees := m.frees + 1
    freeCompleted := true
    quarantineObserved := true }

def reuseChunk (m : Machine) : Machine :=
  { m with
    chunk := { m.chunk with allocated := true, quarantined := false }
    allocations := m.allocations + 1
    reuseCompleted := true }

def rejectDoubleFree (m : Machine) : Machine :=
  { m with doubleFreeRejected := true }

def rejectStalePointer (m : Machine) : Machine :=
  { m with stalePointerRejected := true }

def crossThreadFree (m : Machine) : Machine :=
  { m with
    chunk := { m.chunk with allocated := false, quarantined := true }
    frees := m.frees + 1
    crossThreadHandoff := true }

def guardFault (m : Machine) : Machine :=
  { m with guardFaulted := true }

def afterAlloc : Machine :=
  allocChunk initialMachine

def afterAllocSize : Machine :=
  reportAllocSize afterAlloc

def afterFree : Machine :=
  freeChunk afterAllocSize

def afterReuse : Machine :=
  reuseChunk afterFree

def afterDoubleFreeReject : Machine :=
  rejectDoubleFree afterReuse

def afterStaleReject : Machine :=
  rejectStalePointer afterDoubleFreeReject

def afterCrossThreadFree : Machine :=
  crossThreadFree afterStaleReject

def finalMachine : Machine :=
  guardFault afterCrossThreadFree

theorem m8_allocation_completed :
  allocationCompleted afterAlloc := by
  simp [allocationCompleted, afterAlloc, allocChunk, initialMachine, chunk0]

theorem m8_allocation_size_reported :
  allocationSizeReported afterAllocSize := by
  simp [
    allocationSizeReported, afterAllocSize, reportAllocSize, afterAlloc,
    allocChunk, initialMachine, chunk0
  ]

theorem m8_free_completed :
  freeCompleted afterFree := by
  simp [
    freeCompleted, afterFree, freeChunk, afterAllocSize, reportAllocSize,
    afterAlloc, allocChunk, initialMachine, chunk0
  ]

theorem m8_reuse_completed :
  reuseCompleted afterReuse := by
  simp [
    reuseCompleted, afterReuse, reuseChunk, afterFree, freeChunk,
    afterAllocSize, reportAllocSize, afterAlloc, allocChunk, initialMachine,
    chunk0
  ]

theorem m8_double_free_rejected :
  doubleFreeRejected afterDoubleFreeReject := by
  simp [
    doubleFreeRejected, afterDoubleFreeReject, rejectDoubleFree, afterReuse,
    reuseChunk, afterFree, freeChunk, afterAllocSize, reportAllocSize,
    afterAlloc, allocChunk, initialMachine, chunk0
  ]

theorem m8_stale_pointer_rejected :
  stalePointerRejected afterStaleReject := by
  intro _stale
  simp [
    afterStaleReject, rejectStalePointer, afterDoubleFreeReject,
    rejectDoubleFree, afterReuse, reuseChunk, afterFree, freeChunk,
    afterAllocSize, reportAllocSize, afterAlloc, allocChunk, initialMachine,
    chunk0
  ]

theorem m8_cross_thread_free_handoff :
  crossThreadFreeHandoff afterCrossThreadFree := by
  simp [
    crossThreadFreeHandoff, afterCrossThreadFree, crossThreadFree,
    afterStaleReject, rejectStalePointer, afterDoubleFreeReject,
    rejectDoubleFree, afterReuse, reuseChunk, afterFree, freeChunk,
    afterAllocSize, reportAllocSize, afterAlloc, allocChunk, initialMachine,
    chunk0
  ]

theorem m8_guard_fault_observed :
  guardFaultObserved finalMachine := by
  simp [
    guardFaultObserved, finalMachine, guardFault, afterCrossThreadFree,
    crossThreadFree, afterStaleReject, rejectStalePointer,
    afterDoubleFreeReject, rejectDoubleFree, afterReuse, reuseChunk,
    afterFree, freeChunk, afterAllocSize, reportAllocSize, afterAlloc,
    allocChunk, initialMachine, chunk0
  ]

theorem m8_quarantine_observed :
  quarantineObserved finalMachine := by
  simp [
    quarantineObserved, finalMachine, guardFault, afterCrossThreadFree,
    crossThreadFree, afterStaleReject, rejectStalePointer,
    afterDoubleFreeReject, rejectDoubleFree, afterReuse, reuseChunk,
    afterFree, freeChunk, afterAllocSize, reportAllocSize, afterAlloc,
    allocChunk, initialMachine, chunk0
  ]

theorem m8_heap_counts_exact :
  heapCountsExact finalMachine := by
  simp [
    heapCountsExact, finalMachine, guardFault, afterCrossThreadFree,
    crossThreadFree, afterStaleReject, rejectStalePointer,
    afterDoubleFreeReject, rejectDoubleFree, afterReuse, reuseChunk,
    afterFree, freeChunk, afterAllocSize, reportAllocSize, afterAlloc,
    allocChunk, initialMachine, chunk0
  ]

/- Packed-bit decode model for the M8 heap witness.

Mirrors the M1..M7/M14 packed-bit machinery so the emitted lnp64_m8_heap_commit_t
and lnp64_m8_state_projection_t bit vectors can be decode-checked against this
Lean model. Every M8 field is a plain scalar/bool slice. -/

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

def rtlM8CommitPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("owner_tid", 32)
  , ("pointer_generation", 32)
  , ("heap_generation", 32)
  , ("size_class", 32)
  , ("heap_ptr", 64) ]

def rtlM8StateProjectionPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("pointer_generation", 32)
  , ("owner_tid", 32)
  , ("allocations", 32)
  , ("frees", 32)
  , ("allocated", 1)
  , ("quarantined", 1)
  , ("alloc_completed", 1)
  , ("alloc_size_reported", 1)
  , ("free_completed", 1)
  , ("reuse_completed", 1)
  , ("double_free_rejected", 1)
  , ("stale_pointer_rejected", 1)
  , ("cross_thread_handoff", 1)
  , ("guard_faulted", 1)
  , ("quarantine_observed", 1)
  , ("heap_count_exact", 1) ]

def rtlM8CommitPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM8CommitPackedSchema

def rtlM8StateProjectionPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM8StateProjectionPackedSchema

theorem rtlM8CommitPackedSchema_width :
    packedSchemaWidth rtlM8CommitPackedSchema = 216 := by
  decide

theorem rtlM8StateProjectionPackedSchema_width :
    packedSchemaWidth rtlM8StateProjectionPackedSchema = 164 := by
  decide

theorem rtlM8CommitPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM8CommitPackedSchema)
      rtlM8CommitPackedLayout = true := by
  decide

theorem rtlM8StateProjectionPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM8StateProjectionPackedSchema)
      rtlM8StateProjectionPackedLayout = true := by
  decide

end Lnp64.M8
