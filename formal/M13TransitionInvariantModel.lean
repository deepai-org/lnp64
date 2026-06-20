/- LNP64 M13 transition-invariant model.

`M13PcieIommuModel.lean` is a bounded PCIe/IOMMU witness. This file adds a
transition-invariant proof slice for the SG-IO PCIe guarantees: typed state,
typed operations, a `Step` relation, `Reachable`, preservation, and theorems
over arbitrary reachable states (an unbound bus master is always rejected, a
stale BAR submit is always rejected, and raw PCIe DMA/interrupt authority is
never exposed in any reachable state).
-/

namespace Lnp64.M13Transition

structure State where
  ownerDomain : Nat
  unboundObserved : Bool
  unboundRejected : Bool
  staleBarObserved : Bool
  staleBarRejected : Bool
  malformedObserved : Bool
  malformedRejected : Bool
  rawPcieExposed : Bool
  completions : Nat
  faults : Nat
deriving DecidableEq, Repr

inductive Op
  | enumerate
  | iommuDma
  | msi
  | rejectUnboundBusMaster
  | rejectStaleBar
  | rejectMalformedConfig
  | retireRawAuthority
deriving DecidableEq, Repr

def reset : State :=
  { ownerDomain := 1
    unboundObserved := false
    unboundRejected := false
    staleBarObserved := false
    staleBarRejected := false
    malformedObserved := false
    malformedRejected := false
    rawPcieExposed := false
    completions := 0
    faults := 0 }

def unboundBusMasterFailsClosed (s : State) : Prop :=
  s.unboundObserved = true -> s.unboundRejected = true

def staleBarFailsClosed (s : State) : Prop :=
  s.staleBarObserved = true -> s.staleBarRejected = true

def malformedConfigFailsClosed (s : State) : Prop :=
  s.malformedObserved = true -> s.malformedRejected = true

def noRawPcieAuthority (s : State) : Prop :=
  s.rawPcieExposed = false

def invariant (s : State) : Prop :=
  unboundBusMasterFailsClosed s /\
  staleBarFailsClosed s /\
  malformedConfigFailsClosed s /\
  noRawPcieAuthority s

inductive Step : State -> Op -> State -> Prop
  | enumerate (s : State) :
      Step s Op.enumerate { s with completions := s.completions + 1 }
  | iommuDma (s : State) :
      Step s Op.iommuDma { s with completions := s.completions + 1 }
  | msi (s : State) :
      Step s Op.msi { s with completions := s.completions + 1 }
  | rejectUnboundBusMaster (s : State) :
      Step s Op.rejectUnboundBusMaster
        { s with unboundObserved := true, unboundRejected := true, faults := s.faults + 1 }
  | rejectStaleBar (s : State) :
      Step s Op.rejectStaleBar
        { s with staleBarObserved := true, staleBarRejected := true, faults := s.faults + 1 }
  | rejectMalformedConfig (s : State) :
      Step s Op.rejectMalformedConfig
        { s with malformedObserved := true, malformedRejected := true, faults := s.faults + 1 }
  | retireRawAuthority (s : State) :
      Step s Op.retireRawAuthority { s with rawPcieExposed := false }

inductive Reachable : State -> Prop
  | reset : Reachable reset
  | step {s t : State} {op : Op} :
      Reachable s -> Step s op t -> Reachable t

theorem invariant_reset : invariant reset := by
  simp [invariant, reset, unboundBusMasterFailsClosed, staleBarFailsClosed,
    malformedConfigFailsClosed, noRawPcieAuthority]

theorem invariant_step {s t : State} {op : Op} :
    invariant s -> Step s op t -> invariant t := by
  intro hInv hStep
  cases hStep <;>
    simp_all [invariant, unboundBusMasterFailsClosed, staleBarFailsClosed,
      malformedConfigFailsClosed, noRawPcieAuthority]

theorem reachable_invariant {s : State} :
    Reachable s -> invariant s := by
  intro hReach
  induction hReach with
  | reset => exact invariant_reset
  | step _ hStep ih => exact invariant_step ih hStep

theorem m13_t3_unbound_bus_master_fails_closed_for_all_reachable {s : State} :
    Reachable s -> unboundBusMasterFailsClosed s := by
  intro hReach
  exact (reachable_invariant hReach).1

theorem m13_t3_stale_bar_fails_closed_for_all_reachable {s : State} :
    Reachable s -> staleBarFailsClosed s := by
  intro hReach
  exact (reachable_invariant hReach).2.1

theorem m13_t3_no_raw_pcie_authority_for_all_reachable {s : State} :
    Reachable s -> noRawPcieAuthority s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2

end Lnp64.M13Transition
