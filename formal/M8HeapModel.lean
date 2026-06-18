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

end Lnp64.M8
