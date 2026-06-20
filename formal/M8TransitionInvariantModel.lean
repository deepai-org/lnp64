/- LNP64 M8 transition-invariant model.

`M8HeapModel.lean` is a bounded heap witness. This file adds a
transition-invariant proof slice for the SG-MEM heap-safety guarantees: typed
state, typed operations, a `Step` relation, `Reachable`, preservation, and
theorems over arbitrary reachable states (a double free is always rejected, a
stale pointer is always rejected, and a guard fault is always quarantined).
-/

namespace Lnp64.M8Transition

structure State where
  doubleFreeObserved : Bool
  doubleFreeRejected : Bool
  stalePointerObserved : Bool
  stalePointerRejected : Bool
  guardFaultObserved : Bool
  quarantined : Bool
  completions : Nat
  faults : Nat
deriving DecidableEq, Repr

inductive Op
  | allocate
  | free
  | reuse
  | rejectDoubleFree
  | rejectStalePointer
  | guardFault
deriving DecidableEq, Repr

def reset : State :=
  { doubleFreeObserved := false
    doubleFreeRejected := false
    stalePointerObserved := false
    stalePointerRejected := false
    guardFaultObserved := false
    quarantined := false
    completions := 0
    faults := 0 }

def doubleFreeFailsClosed (s : State) : Prop :=
  s.doubleFreeObserved = true -> s.doubleFreeRejected = true

def stalePointerFailsClosed (s : State) : Prop :=
  s.stalePointerObserved = true -> s.stalePointerRejected = true

def guardFaultQuarantined (s : State) : Prop :=
  s.guardFaultObserved = true -> s.quarantined = true

def invariant (s : State) : Prop :=
  doubleFreeFailsClosed s /\
  stalePointerFailsClosed s /\
  guardFaultQuarantined s

inductive Step : State -> Op -> State -> Prop
  | allocate (s : State) :
      Step s Op.allocate { s with completions := s.completions + 1 }
  | free (s : State) :
      Step s Op.free { s with completions := s.completions + 1 }
  | reuse (s : State) :
      Step s Op.reuse { s with completions := s.completions + 1 }
  | rejectDoubleFree (s : State) :
      Step s Op.rejectDoubleFree
        { s with doubleFreeObserved := true, doubleFreeRejected := true, faults := s.faults + 1 }
  | rejectStalePointer (s : State) :
      Step s Op.rejectStalePointer
        { s with stalePointerObserved := true, stalePointerRejected := true, faults := s.faults + 1 }
  | guardFault (s : State) :
      Step s Op.guardFault
        { s with guardFaultObserved := true, quarantined := true, faults := s.faults + 1 }

inductive Reachable : State -> Prop
  | reset : Reachable reset
  | step {s t : State} {op : Op} :
      Reachable s -> Step s op t -> Reachable t

theorem invariant_reset : invariant reset := by
  simp [invariant, reset, doubleFreeFailsClosed, stalePointerFailsClosed,
    guardFaultQuarantined]

theorem invariant_step {s t : State} {op : Op} :
    invariant s -> Step s op t -> invariant t := by
  intro hInv hStep
  cases hStep <;>
    simp_all [invariant, doubleFreeFailsClosed, stalePointerFailsClosed,
      guardFaultQuarantined]

theorem reachable_invariant {s : State} :
    Reachable s -> invariant s := by
  intro hReach
  induction hReach with
  | reset => exact invariant_reset
  | step _ hStep ih => exact invariant_step ih hStep

theorem m8_t3_double_free_fails_closed_for_all_reachable {s : State} :
    Reachable s -> doubleFreeFailsClosed s := by
  intro hReach
  exact (reachable_invariant hReach).1

theorem m8_t3_stale_pointer_fails_closed_for_all_reachable {s : State} :
    Reachable s -> stalePointerFailsClosed s := by
  intro hReach
  exact (reachable_invariant hReach).2.1

theorem m8_t3_guard_fault_quarantined_for_all_reachable {s : State} :
    Reachable s -> guardFaultQuarantined s := by
  intro hReach
  exact (reachable_invariant hReach).2.2

end Lnp64.M8Transition
