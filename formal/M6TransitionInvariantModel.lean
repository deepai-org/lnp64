/- LNP64 M6 transition-invariant model.

`M6ServiceModel.lean` is a bounded service/namespace witness. This file adds a
transition-invariant proof slice for the SG-AUTH no-forged-authority
guarantees: typed state, typed operations, a `Step` relation, `Reachable`,
preservation, and theorems over arbitrary reachable states (a stale service
request is always rejected, any installed capability is always the narrowed
one -- authority is never widened/forged on return -- and a service cancel
always reaches a terminal completion).
-/

namespace Lnp64.M6Transition

structure State where
  staleObserved : Bool
  staleRejected : Bool
  capabilityInstalled : Bool
  capabilityNarrowed : Bool
  cancelObserved : Bool
  cancelTerminal : Bool
  completions : Nat
  faults : Nat
deriving DecidableEq, Repr

inductive Op
  | validateEnvelope
  | dispatchNamespace
  | installNarrowedCapability
  | rejectStaleService
  | cancelService
deriving DecidableEq, Repr

def reset : State :=
  { staleObserved := false
    staleRejected := false
    capabilityInstalled := false
    capabilityNarrowed := false
    cancelObserved := false
    cancelTerminal := false
    completions := 0
    faults := 0 }

def staleServiceFailsClosed (s : State) : Prop :=
  s.staleObserved = true -> s.staleRejected = true

def installedCapabilityNarrowed (s : State) : Prop :=
  s.capabilityInstalled = true -> s.capabilityNarrowed = true

def serviceCancelTerminal (s : State) : Prop :=
  s.cancelObserved = true -> s.cancelTerminal = true

def invariant (s : State) : Prop :=
  staleServiceFailsClosed s /\
  installedCapabilityNarrowed s /\
  serviceCancelTerminal s

inductive Step : State -> Op -> State -> Prop
  | validateEnvelope (s : State) :
      Step s Op.validateEnvelope { s with completions := s.completions + 1 }
  | dispatchNamespace (s : State) :
      Step s Op.dispatchNamespace { s with completions := s.completions + 1 }
  | installNarrowedCapability (s : State) :
      Step s Op.installNarrowedCapability
        { s with capabilityInstalled := true, capabilityNarrowed := true, completions := s.completions + 1 }
  | rejectStaleService (s : State) :
      Step s Op.rejectStaleService
        { s with staleObserved := true, staleRejected := true, faults := s.faults + 1 }
  | cancelService (s : State) :
      Step s Op.cancelService
        { s with cancelObserved := true, cancelTerminal := true }

inductive Reachable : State -> Prop
  | reset : Reachable reset
  | step {s t : State} {op : Op} :
      Reachable s -> Step s op t -> Reachable t

theorem invariant_reset : invariant reset := by
  simp [invariant, reset, staleServiceFailsClosed, installedCapabilityNarrowed,
    serviceCancelTerminal]

theorem invariant_step {s t : State} {op : Op} :
    invariant s -> Step s op t -> invariant t := by
  intro hInv hStep
  cases hStep <;>
    simp_all [invariant, staleServiceFailsClosed, installedCapabilityNarrowed,
      serviceCancelTerminal]

theorem reachable_invariant {s : State} :
    Reachable s -> invariant s := by
  intro hReach
  induction hReach with
  | reset => exact invariant_reset
  | step _ hStep ih => exact invariant_step ih hStep

theorem m6_t3_stale_service_fails_closed_for_all_reachable {s : State} :
    Reachable s -> staleServiceFailsClosed s := by
  intro hReach
  exact (reachable_invariant hReach).1

theorem m6_t3_installed_capability_narrowed_for_all_reachable {s : State} :
    Reachable s -> installedCapabilityNarrowed s := by
  intro hReach
  exact (reachable_invariant hReach).2.1

theorem m6_t3_service_cancel_terminal_for_all_reachable {s : State} :
    Reachable s -> serviceCancelTerminal s := by
  intro hReach
  exact (reachable_invariant hReach).2.2

end Lnp64.M6Transition
