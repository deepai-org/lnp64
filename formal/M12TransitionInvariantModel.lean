/- LNP64 M12 transition-invariant model.

`M12StorageBarrierModel.lean` is a bounded storage-barrier witness. This file
adds a transition-invariant proof slice for the SG-MEM storage guarantees:
typed state, typed operations, a `Step` relation, `Reachable`, preservation,
and theorems over arbitrary reachable states (a stale-object submit is always
rejected, a cross-domain submit is always rejected, and raw block-device
authority is never exposed in any reachable state).
-/

namespace Lnp64.M12Transition

structure State where
  ownerDomain : Nat
  staleObserved : Bool
  staleRejected : Bool
  crossObserved : Bool
  crossRejected : Bool
  rawAuthorityExposed : Bool
  completions : Nat
  faults : Nat
deriving DecidableEq, Repr

inductive Op
  | bootImage
  | blockWrite
  | barrier
  | submitStale
  | submitCrossDomain
  | mediaFault
  | retireRawAuthority
deriving DecidableEq, Repr

def reset : State :=
  { ownerDomain := 1
    staleObserved := false
    staleRejected := false
    crossObserved := false
    crossRejected := false
    rawAuthorityExposed := false
    completions := 0
    faults := 0 }

def staleFailsClosed (s : State) : Prop :=
  s.staleObserved = true -> s.staleRejected = true

def crossDomainFailsClosed (s : State) : Prop :=
  s.crossObserved = true -> s.crossRejected = true

def noRawDeviceAuthority (s : State) : Prop :=
  s.rawAuthorityExposed = false

def invariant (s : State) : Prop :=
  staleFailsClosed s /\ crossDomainFailsClosed s /\ noRawDeviceAuthority s

inductive Step : State -> Op -> State -> Prop
  | bootImage (s : State) :
      Step s Op.bootImage { s with completions := s.completions + 1 }
  | blockWrite (s : State) :
      Step s Op.blockWrite { s with completions := s.completions + 1 }
  | barrier (s : State) :
      Step s Op.barrier { s with completions := s.completions + 1 }
  | submitStale (s : State) :
      Step s Op.submitStale
        { s with staleObserved := true, staleRejected := true, faults := s.faults + 1 }
  | submitCrossDomain (s : State) :
      Step s Op.submitCrossDomain
        { s with crossObserved := true, crossRejected := true, faults := s.faults + 1 }
  | mediaFault (s : State) :
      Step s Op.mediaFault { s with faults := s.faults + 1 }
  | retireRawAuthority (s : State) :
      Step s Op.retireRawAuthority { s with rawAuthorityExposed := false }

inductive Reachable : State -> Prop
  | reset : Reachable reset
  | step {s t : State} {op : Op} :
      Reachable s -> Step s op t -> Reachable t

theorem invariant_reset : invariant reset := by
  simp [invariant, reset, staleFailsClosed, crossDomainFailsClosed, noRawDeviceAuthority]

theorem invariant_step {s t : State} {op : Op} :
    invariant s -> Step s op t -> invariant t := by
  intro hInv hStep
  cases hStep <;>
    simp_all [invariant, staleFailsClosed, crossDomainFailsClosed, noRawDeviceAuthority]

theorem reachable_invariant {s : State} :
    Reachable s -> invariant s := by
  intro hReach
  induction hReach with
  | reset => exact invariant_reset
  | step _ hStep ih => exact invariant_step ih hStep

theorem m12_t3_stale_object_fails_closed_for_all_reachable {s : State} :
    Reachable s -> staleFailsClosed s := by
  intro hReach
  exact (reachable_invariant hReach).1

theorem m12_t3_cross_domain_fails_closed_for_all_reachable {s : State} :
    Reachable s -> crossDomainFailsClosed s := by
  intro hReach
  exact (reachable_invariant hReach).2.1

theorem m12_t3_no_raw_device_authority_for_all_reachable {s : State} :
    Reachable s -> noRawDeviceAuthority s := by
  intro hReach
  exact (reachable_invariant hReach).2.2

end Lnp64.M12Transition
