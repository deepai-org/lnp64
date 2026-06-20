/- LNP64 M3 transition-invariant model.

`M3ProcessModel.lean` is a bounded process-lifecycle witness. This file adds a
transition-invariant proof slice for the SG-WAKE/SG-PROGRESS process
guarantees: typed state, typed operations, a `Step` relation, `Reachable`,
preservation, and theorems over arbitrary reachable states (a stale join is
always rejected, a successful join only ever consumes an exited child, and an
exec cancel always reaches a terminal state).
-/

namespace Lnp64.M3Transition

structure State where
  joinObserved : Bool
  childExited : Bool
  staleJoinObserved : Bool
  staleJoinRejected : Bool
  cancelObserved : Bool
  cancelTerminal : Bool
  completions : Nat
  faults : Nat
deriving DecidableEq, Repr

inductive Op
  | childExit
  | joinExitedChild
  | rejectStaleJoin
  | execCancel
deriving DecidableEq, Repr

def reset : State :=
  { joinObserved := false
    childExited := false
    staleJoinObserved := false
    staleJoinRejected := false
    cancelObserved := false
    cancelTerminal := false
    completions := 0
    faults := 0 }

def joinConsumesExitedChild (s : State) : Prop :=
  s.joinObserved = true -> s.childExited = true

def staleJoinFailsClosed (s : State) : Prop :=
  s.staleJoinObserved = true -> s.staleJoinRejected = true

def execCancelReachesTerminal (s : State) : Prop :=
  s.cancelObserved = true -> s.cancelTerminal = true

def invariant (s : State) : Prop :=
  joinConsumesExitedChild s /\
  staleJoinFailsClosed s /\
  execCancelReachesTerminal s

inductive Step : State -> Op -> State -> Prop
  | childExit (s : State) :
      Step s Op.childExit { s with childExited := true }
  | joinExitedChild (s : State) :
      s.childExited = true ->
      Step s Op.joinExitedChild
        { s with joinObserved := true, completions := s.completions + 1 }
  | rejectStaleJoin (s : State) :
      Step s Op.rejectStaleJoin
        { s with staleJoinObserved := true, staleJoinRejected := true, faults := s.faults + 1 }
  | execCancel (s : State) :
      Step s Op.execCancel
        { s with cancelObserved := true, cancelTerminal := true }

inductive Reachable : State -> Prop
  | reset : Reachable reset
  | step {s t : State} {op : Op} :
      Reachable s -> Step s op t -> Reachable t

theorem invariant_reset : invariant reset := by
  simp [invariant, reset, joinConsumesExitedChild, staleJoinFailsClosed,
    execCancelReachesTerminal]

theorem invariant_step {s t : State} {op : Op} :
    invariant s -> Step s op t -> invariant t := by
  intro hInv hStep
  cases hStep <;>
    simp_all [invariant, joinConsumesExitedChild, staleJoinFailsClosed,
      execCancelReachesTerminal]

theorem reachable_invariant {s : State} :
    Reachable s -> invariant s := by
  intro hReach
  induction hReach with
  | reset => exact invariant_reset
  | step _ hStep ih => exact invariant_step ih hStep

theorem m3_t3_join_consumes_exited_child_for_all_reachable {s : State} :
    Reachable s -> joinConsumesExitedChild s := by
  intro hReach
  exact (reachable_invariant hReach).1

theorem m3_t3_stale_join_fails_closed_for_all_reachable {s : State} :
    Reachable s -> staleJoinFailsClosed s := by
  intro hReach
  exact (reachable_invariant hReach).2.1

theorem m3_t3_exec_cancel_reaches_terminal_for_all_reachable {s : State} :
    Reachable s -> execCancelReachesTerminal s := by
  intro hReach
  exact (reachable_invariant hReach).2.2

end Lnp64.M3Transition
