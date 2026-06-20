/- LNP64 M11 transition-invariant model.

`M11DdrMetadataModel.lean` is a bounded DDR/metadata witness. This file adds a
transition-invariant proof slice for the SG-MEM revocation/generation and
domain-confinement guarantees: typed metadata-line state, typed operations, a
`Step` relation, `Reachable`, preservation, and theorems over arbitrary
reachable states (a stale-generation submit is always rejected, a cross-domain
submit is always rejected, and the line stays validly bound).
-/

namespace Lnp64.M11Transition

structure State where
  lineGeneration : Nat
  ownerDomain : Nat
  staleObserved : Bool
  staleRejected : Bool
  crossObserved : Bool
  crossRejected : Bool
  completions : Nat
  faults : Nat
deriving DecidableEq, Repr

inductive Op
  | allocate (gen owner : Nat)
  | writeFresh (gen owner : Nat)
  | readFresh (gen owner : Nat)
  | submitStale (gen : Nat)
  | submitCrossDomain (owner : Nat)
  | eccScrub
  | barrier
deriving DecidableEq, Repr

def erevoked : Nat := 122
def eperm : Nat := 1

def reset : State :=
  { lineGeneration := 1
    ownerDomain := 1
    staleObserved := false
    staleRejected := false
    crossObserved := false
    crossRejected := false
    completions := 0
    faults := 0 }

/-- A stale-generation submit, once observed, is always rejected. -/
def staleFailsClosed (s : State) : Prop :=
  s.staleObserved = true -> s.staleRejected = true

/-- A cross-domain submit, once observed, is always rejected. -/
def crossDomainFailsClosed (s : State) : Prop :=
  s.crossObserved = true -> s.crossRejected = true

/-- The metadata line is always bound to a real owner domain and generation. -/
def lineValidlyBound (s : State) : Prop :=
  s.ownerDomain >= 1 /\ s.lineGeneration >= 1

def invariant (s : State) : Prop :=
  staleFailsClosed s /\
  crossDomainFailsClosed s /\
  lineValidlyBound s

inductive Step : State -> Op -> State -> Prop
  | allocate (s : State) (gen owner : Nat) :
      gen >= 1 -> owner >= 1 ->
      Step s (Op.allocate gen owner)
        { s with lineGeneration := gen, ownerDomain := owner }
  | writeFresh (s : State) (gen owner : Nat) :
      gen >= s.lineGeneration -> owner = s.ownerDomain ->
      Step s (Op.writeFresh gen owner)
        { s with completions := s.completions + 1 }
  | readFresh (s : State) (gen owner : Nat) :
      gen >= s.lineGeneration -> owner = s.ownerDomain ->
      Step s (Op.readFresh gen owner)
        { s with completions := s.completions + 1 }
  | submitStale (s : State) (gen : Nat) :
      gen < s.lineGeneration ->
      Step s (Op.submitStale gen)
        { s with staleObserved := true, staleRejected := true, faults := s.faults + 1 }
  | submitCrossDomain (s : State) (owner : Nat) :
      owner ≠ s.ownerDomain ->
      Step s (Op.submitCrossDomain owner)
        { s with crossObserved := true, crossRejected := true, faults := s.faults + 1 }
  | eccScrub (s : State) :
      Step s Op.eccScrub
        { s with faults := s.faults + 1 }
  | barrier (s : State) :
      Step s Op.barrier s

inductive Reachable : State -> Prop
  | reset : Reachable reset
  | step {s t : State} {op : Op} :
      Reachable s -> Step s op t -> Reachable t

theorem invariant_reset : invariant reset := by
  simp [invariant, reset, staleFailsClosed, crossDomainFailsClosed, lineValidlyBound]

theorem invariant_step {s t : State} {op : Op} :
    invariant s -> Step s op t -> invariant t := by
  intro hInv hStep
  cases hStep <;>
    simp_all [invariant, staleFailsClosed, crossDomainFailsClosed, lineValidlyBound]

theorem reachable_invariant {s : State} :
    Reachable s -> invariant s := by
  intro hReach
  induction hReach with
  | reset => exact invariant_reset
  | step _ hStep ih => exact invariant_step ih hStep

theorem m11_t3_stale_generation_fails_closed_for_all_reachable {s : State} :
    Reachable s -> staleFailsClosed s := by
  intro hReach
  exact (reachable_invariant hReach).1

theorem m11_t3_cross_domain_fails_closed_for_all_reachable {s : State} :
    Reachable s -> crossDomainFailsClosed s := by
  intro hReach
  exact (reachable_invariant hReach).2.1

theorem m11_t3_line_validly_bound_for_all_reachable {s : State} :
    Reachable s -> lineValidlyBound s := by
  intro hReach
  exact (reachable_invariant hReach).2.2

end Lnp64.M11Transition
