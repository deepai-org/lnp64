/- LNP64 M9 transition-invariant model.

`M9ClassifierServiceletModel.lean` is a bounded classifier/servicelet witness.
This file adds a transition-invariant proof slice for the SG-PROGRESS
servicelet-containment guarantees: typed state, typed operations, a `Step`
relation, `Reachable`, preservation, and theorems over arbitrary reachable
states (an over-budget servicelet is always terminated/charged, a stale
attachment is always rejected, and a servicelet never creates authority).
-/

namespace Lnp64.M9Transition

structure State where
  budgetExceededObserved : Bool
  budgetEnforced : Bool
  staleAttachObserved : Bool
  staleAttachRejected : Bool
  authorityCreated : Bool
  completions : Nat
  faults : Nat
deriving DecidableEq, Repr

inductive Op
  | verifierAccept
  | steerPacket
  | steerIpc
  | enforceBudget
  | rejectStaleAttachment
deriving DecidableEq, Repr

def reset : State :=
  { budgetExceededObserved := false
    budgetEnforced := false
    staleAttachObserved := false
    staleAttachRejected := false
    authorityCreated := false
    completions := 0
    faults := 0 }

def budgetEnforcedFailsClosed (s : State) : Prop :=
  s.budgetExceededObserved = true -> s.budgetEnforced = true

def staleAttachmentFailsClosed (s : State) : Prop :=
  s.staleAttachObserved = true -> s.staleAttachRejected = true

def noAuthorityCreation (s : State) : Prop :=
  s.authorityCreated = false

def invariant (s : State) : Prop :=
  budgetEnforcedFailsClosed s /\
  staleAttachmentFailsClosed s /\
  noAuthorityCreation s

inductive Step : State -> Op -> State -> Prop
  | verifierAccept (s : State) :
      Step s Op.verifierAccept { s with completions := s.completions + 1 }
  | steerPacket (s : State) :
      Step s Op.steerPacket { s with completions := s.completions + 1 }
  | steerIpc (s : State) :
      Step s Op.steerIpc { s with completions := s.completions + 1 }
  | enforceBudget (s : State) :
      Step s Op.enforceBudget
        { s with budgetExceededObserved := true, budgetEnforced := true, faults := s.faults + 1 }
  | rejectStaleAttachment (s : State) :
      Step s Op.rejectStaleAttachment
        { s with staleAttachObserved := true, staleAttachRejected := true, faults := s.faults + 1 }

inductive Reachable : State -> Prop
  | reset : Reachable reset
  | step {s t : State} {op : Op} :
      Reachable s -> Step s op t -> Reachable t

theorem invariant_reset : invariant reset := by
  simp [invariant, reset, budgetEnforcedFailsClosed, staleAttachmentFailsClosed,
    noAuthorityCreation]

theorem invariant_step {s t : State} {op : Op} :
    invariant s -> Step s op t -> invariant t := by
  intro hInv hStep
  cases hStep <;>
    simp_all [invariant, budgetEnforcedFailsClosed, staleAttachmentFailsClosed,
      noAuthorityCreation]

theorem reachable_invariant {s : State} :
    Reachable s -> invariant s := by
  intro hReach
  induction hReach with
  | reset => exact invariant_reset
  | step _ hStep ih => exact invariant_step ih hStep

theorem m9_t3_budget_enforced_for_all_reachable {s : State} :
    Reachable s -> budgetEnforcedFailsClosed s := by
  intro hReach
  exact (reachable_invariant hReach).1

theorem m9_t3_stale_attachment_fails_closed_for_all_reachable {s : State} :
    Reachable s -> staleAttachmentFailsClosed s := by
  intro hReach
  exact (reachable_invariant hReach).2.1

theorem m9_t3_no_authority_creation_for_all_reachable {s : State} :
    Reachable s -> noAuthorityCreation s := by
  intro hReach
  exact (reachable_invariant hReach).2.2

end Lnp64.M9Transition
