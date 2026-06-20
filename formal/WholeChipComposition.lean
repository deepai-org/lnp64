/- LNP64 whole-chip composition.

Composes the fifteen per-engine transition-invariant models (M1-M15,
`formal/M*TransitionInvariantModel.lean`) into a single chip model and proves
that the conjunction of every engine's severe-goal invariant holds in EVERY
reachable whole-chip state. A `ChipStep` is one engine taking one of its typed
steps while all other engines hold their state (asynchronous interleaving);
`chipInvariant` is the conjunction of the fifteen per-engine invariants, and
`whole_chip_severe_goals_hold_for_all_reachable` lifts each engine's already-
proved `invariant_step` to the composite reachable set. Per-severe-goal
corollaries then project the specific guarantee for the whole chip.

This file is checked by concatenating the fifteen transition-invariant models
ahead of it (see scripts/run_rtl_whole_chip_composition_gate.sh); every name
resolves in its `Lnp64.M*Transition` namespace. Kernel tactics only -- no
axiom-introducing automation.
-/

namespace Lnp64.WholeChip

structure Chip where
  m1 : Lnp64.M1Transition.State
  m2 : Lnp64.M2Transition.State
  m3 : Lnp64.M3Transition.State
  m4 : Lnp64.M4Transition.State
  m5 : Lnp64.M5Transition.State
  m6 : Lnp64.M6Transition.State
  m7 : Lnp64.M7Transition.State
  m8 : Lnp64.M8Transition.State
  m9 : Lnp64.M9Transition.State
  m10 : Lnp64.M10Transition.State
  m11 : Lnp64.M11Transition.State
  m12 : Lnp64.M12Transition.State
  m13 : Lnp64.M13Transition.State
  m14 : Lnp64.M14Transition.State
  m15 : Lnp64.M15Transition.State
deriving DecidableEq

def chipReset : Chip :=
  { m1 := Lnp64.M1Transition.reset,
    m2 := Lnp64.M2Transition.reset,
    m3 := Lnp64.M3Transition.reset,
    m4 := Lnp64.M4Transition.reset,
    m5 := Lnp64.M5Transition.reset,
    m6 := Lnp64.M6Transition.reset,
    m7 := Lnp64.M7Transition.reset,
    m8 := Lnp64.M8Transition.reset,
    m9 := Lnp64.M9Transition.reset,
    m10 := Lnp64.M10Transition.reset,
    m11 := Lnp64.M11Transition.reset,
    m12 := Lnp64.M12Transition.reset,
    m13 := Lnp64.M13Transition.reset,
    m14 := Lnp64.M14Transition.reset,
    m15 := Lnp64.M15Transition.reset
  }

inductive ChipStep : Chip -> Chip -> Prop
  | m1 (c : Chip) (op : Lnp64.M1Transition.Op) (t : Lnp64.M1Transition.State) :
      Lnp64.M1Transition.Step c.m1 op t -> ChipStep c { c with m1 := t }
  | m2 (c : Chip) (op : Lnp64.M2Transition.Op) (t : Lnp64.M2Transition.State) :
      Lnp64.M2Transition.Step c.m2 op t -> ChipStep c { c with m2 := t }
  | m3 (c : Chip) (op : Lnp64.M3Transition.Op) (t : Lnp64.M3Transition.State) :
      Lnp64.M3Transition.Step c.m3 op t -> ChipStep c { c with m3 := t }
  | m4 (c : Chip) (op : Lnp64.M4Transition.Op) (t : Lnp64.M4Transition.State) :
      Lnp64.M4Transition.Step c.m4 op t -> ChipStep c { c with m4 := t }
  | m5 (c : Chip) (op : Lnp64.M5Transition.Op) (t : Lnp64.M5Transition.State) :
      Lnp64.M5Transition.Step c.m5 op t -> ChipStep c { c with m5 := t }
  | m6 (c : Chip) (op : Lnp64.M6Transition.Op) (t : Lnp64.M6Transition.State) :
      Lnp64.M6Transition.Step c.m6 op t -> ChipStep c { c with m6 := t }
  | m7 (c : Chip) (op : Lnp64.M7Transition.Op) (t : Lnp64.M7Transition.State) :
      Lnp64.M7Transition.Step c.m7 op t -> ChipStep c { c with m7 := t }
  | m8 (c : Chip) (op : Lnp64.M8Transition.Op) (t : Lnp64.M8Transition.State) :
      Lnp64.M8Transition.Step c.m8 op t -> ChipStep c { c with m8 := t }
  | m9 (c : Chip) (op : Lnp64.M9Transition.Op) (t : Lnp64.M9Transition.State) :
      Lnp64.M9Transition.Step c.m9 op t -> ChipStep c { c with m9 := t }
  | m10 (c : Chip) (op : Lnp64.M10Transition.Op) (t : Lnp64.M10Transition.State) :
      Lnp64.M10Transition.Step c.m10 op t -> ChipStep c { c with m10 := t }
  | m11 (c : Chip) (op : Lnp64.M11Transition.Op) (t : Lnp64.M11Transition.State) :
      Lnp64.M11Transition.Step c.m11 op t -> ChipStep c { c with m11 := t }
  | m12 (c : Chip) (op : Lnp64.M12Transition.Op) (t : Lnp64.M12Transition.State) :
      Lnp64.M12Transition.Step c.m12 op t -> ChipStep c { c with m12 := t }
  | m13 (c : Chip) (op : Lnp64.M13Transition.Op) (t : Lnp64.M13Transition.State) :
      Lnp64.M13Transition.Step c.m13 op t -> ChipStep c { c with m13 := t }
  | m14 (c : Chip) (op : Lnp64.M14Transition.Op) (t : Lnp64.M14Transition.State) :
      Lnp64.M14Transition.Step c.m14 op t -> ChipStep c { c with m14 := t }
  | m15 (c : Chip) (op : Lnp64.M15Transition.Op) (t : Lnp64.M15Transition.State) :
      Lnp64.M15Transition.Step c.m15 op t -> ChipStep c { c with m15 := t }

inductive ChipReachable : Chip -> Prop
  | reset : ChipReachable chipReset
  | step {c d : Chip} :
      ChipReachable c -> ChipStep c d -> ChipReachable d

def chipInvariant (c : Chip) : Prop :=
  Lnp64.M1Transition.invariant c.m1 /\
  Lnp64.M2Transition.invariant c.m2 /\
  Lnp64.M3Transition.invariant c.m3 /\
  Lnp64.M4Transition.invariant c.m4 /\
  Lnp64.M5Transition.invariant c.m5 /\
  Lnp64.M6Transition.invariant c.m6 /\
  Lnp64.M7Transition.invariant c.m7 /\
  Lnp64.M8Transition.invariant c.m8 /\
  Lnp64.M9Transition.invariant c.m9 /\
  Lnp64.M10Transition.invariant c.m10 /\
  Lnp64.M11Transition.invariant c.m11 /\
  Lnp64.M12Transition.invariant c.m12 /\
  Lnp64.M13Transition.invariant c.m13 /\
  Lnp64.M14Transition.invariant c.m14 /\
  Lnp64.M15Transition.invariant c.m15

theorem chipInvariant_reset : chipInvariant chipReset :=
  ⟨Lnp64.M1Transition.invariant_reset,
   Lnp64.M2Transition.invariant_reset,
   Lnp64.M3Transition.invariant_reset,
   Lnp64.M4Transition.invariant_reset,
   Lnp64.M5Transition.invariant_reset,
   Lnp64.M6Transition.invariant_reset,
   Lnp64.M7Transition.invariant_reset,
   Lnp64.M8Transition.invariant_reset,
   Lnp64.M9Transition.invariant_reset,
   Lnp64.M10Transition.invariant_reset,
   Lnp64.M11Transition.invariant_reset,
   Lnp64.M12Transition.invariant_reset,
   Lnp64.M13Transition.invariant_reset,
   Lnp64.M14Transition.invariant_reset,
   Lnp64.M15Transition.invariant_reset⟩

theorem chipInvariant_step {c d : Chip} :
    chipInvariant c -> ChipStep c d -> chipInvariant d := by
  intro h hstep
  obtain ⟨h1, h2, h3, h4, h5, h6, h7, h8, h9, h10, h11, h12, h13, h14, h15⟩ := h
  cases hstep with
  | m1 _ _ hst => exact ⟨Lnp64.M1Transition.invariant_step h1 hst, h2, h3, h4, h5, h6, h7, h8, h9, h10, h11, h12, h13, h14, h15⟩
  | m2 _ _ hst => exact ⟨h1, Lnp64.M2Transition.invariant_step h2 hst, h3, h4, h5, h6, h7, h8, h9, h10, h11, h12, h13, h14, h15⟩
  | m3 _ _ hst => exact ⟨h1, h2, Lnp64.M3Transition.invariant_step h3 hst, h4, h5, h6, h7, h8, h9, h10, h11, h12, h13, h14, h15⟩
  | m4 _ _ hst => exact ⟨h1, h2, h3, Lnp64.M4Transition.invariant_step h4 hst, h5, h6, h7, h8, h9, h10, h11, h12, h13, h14, h15⟩
  | m5 _ _ hst => exact ⟨h1, h2, h3, h4, Lnp64.M5Transition.invariant_step h5 hst, h6, h7, h8, h9, h10, h11, h12, h13, h14, h15⟩
  | m6 _ _ hst => exact ⟨h1, h2, h3, h4, h5, Lnp64.M6Transition.invariant_step h6 hst, h7, h8, h9, h10, h11, h12, h13, h14, h15⟩
  | m7 _ _ hst => exact ⟨h1, h2, h3, h4, h5, h6, Lnp64.M7Transition.invariant_step h7 hst, h8, h9, h10, h11, h12, h13, h14, h15⟩
  | m8 _ _ hst => exact ⟨h1, h2, h3, h4, h5, h6, h7, Lnp64.M8Transition.invariant_step h8 hst, h9, h10, h11, h12, h13, h14, h15⟩
  | m9 _ _ hst => exact ⟨h1, h2, h3, h4, h5, h6, h7, h8, Lnp64.M9Transition.invariant_step h9 hst, h10, h11, h12, h13, h14, h15⟩
  | m10 _ _ hst => exact ⟨h1, h2, h3, h4, h5, h6, h7, h8, h9, Lnp64.M10Transition.invariant_step h10 hst, h11, h12, h13, h14, h15⟩
  | m11 _ _ hst => exact ⟨h1, h2, h3, h4, h5, h6, h7, h8, h9, h10, Lnp64.M11Transition.invariant_step h11 hst, h12, h13, h14, h15⟩
  | m12 _ _ hst => exact ⟨h1, h2, h3, h4, h5, h6, h7, h8, h9, h10, h11, Lnp64.M12Transition.invariant_step h12 hst, h13, h14, h15⟩
  | m13 _ _ hst => exact ⟨h1, h2, h3, h4, h5, h6, h7, h8, h9, h10, h11, h12, Lnp64.M13Transition.invariant_step h13 hst, h14, h15⟩
  | m14 _ _ hst => exact ⟨h1, h2, h3, h4, h5, h6, h7, h8, h9, h10, h11, h12, h13, Lnp64.M14Transition.invariant_step h14 hst, h15⟩
  | m15 _ _ hst => exact ⟨h1, h2, h3, h4, h5, h6, h7, h8, h9, h10, h11, h12, h13, h14, Lnp64.M15Transition.invariant_step h15 hst⟩

theorem chipReachable_invariant {c : Chip} :
    ChipReachable c -> chipInvariant c := by
  intro hReach
  induction hReach with
  | reset => exact chipInvariant_reset
  | step _ hStep ih => exact chipInvariant_step ih hStep

/-- Master whole-chip guarantee: in every reachable whole-chip state, every
    engine's severe-goal transition invariant holds simultaneously. -/
theorem whole_chip_severe_goals_hold_for_all_reachable {c : Chip} :
    ChipReachable c -> chipInvariant c :=
  chipReachable_invariant

theorem sg_no_forged_authority_whole_chip {c : Chip} :
    ChipReachable c -> Lnp64.M6Transition.installedCapabilityNarrowed c.m6 := by
  intro h
  have H := chipReachable_invariant h
  obtain ⟨_, _, _, _, _, hh, _, _, _, _, _, _, _, _, _⟩ := H
  exact hh.2.1

theorem sg_revocation_generation_safety_whole_chip {c : Chip} :
    ChipReachable c -> Lnp64.M11Transition.staleFailsClosed c.m11 := by
  intro h
  have H := chipReachable_invariant h
  obtain ⟨_, _, _, _, _, _, _, _, _, _, hh, _, _, _, _⟩ := H
  exact hh.1

theorem sg_domain_containment_whole_chip {c : Chip} :
    ChipReachable c -> Lnp64.M14Transition.childRightsSubsetParentState c.m14 := by
  intro h
  have H := chipReachable_invariant h
  obtain ⟨_, _, _, _, _, _, _, _, _, _, _, _, _, hh, _⟩ := H
  exact hh.2.1

theorem sg_vma_memory_safety_whole_chip {c : Chip} :
    ChipReachable c -> Lnp64.M4Transition.wxEnforcedState c.m4 := by
  intro h
  have H := chipReachable_invariant h
  obtain ⟨_, _, _, hh, _, _, _, _, _, _, _, _, _, _, _⟩ := H
  exact hh.1

theorem sg_heap_double_free_safety_whole_chip {c : Chip} :
    ChipReachable c -> Lnp64.M8Transition.doubleFreeFailsClosed c.m8 := by
  intro h
  have H := chipReachable_invariant h
  obtain ⟨_, _, _, _, _, _, _, hh, _, _, _, _, _, _, _⟩ := H
  exact hh.1

theorem sg_dma_confined_whole_chip {c : Chip} :
    ChipReachable c -> Lnp64.M13Transition.noRawPcieAuthority c.m13 := by
  intro h
  have H := chipReachable_invariant h
  obtain ⟨_, _, _, _, _, _, _, _, _, _, _, _, hh, _, _⟩ := H
  exact hh.2.2.2

theorem sg_scheduler_single_location_whole_chip {c : Chip} :
    ChipReachable c -> Lnp64.M7Transition.exactlyOneSchedulerLocationState c.m7 := by
  intro h
  have H := chipReachable_invariant h
  obtain ⟨_, _, _, _, _, _, hh, _, _, _, _, _, _, _, _⟩ := H
  exact hh.1

theorem sg_no_lost_wakeups_whole_chip {c : Chip} :
    ChipReachable c -> Lnp64.M15Transition.overflowSignalledExplicitly c.m15 := by
  intro h
  have H := chipReachable_invariant h
  obtain ⟨_, _, _, _, _, _, _, _, _, _, _, _, _, _, hh⟩ := H
  exact hh.1

theorem sg_gate_continuation_unique_whole_chip {c : Chip} :
    ChipReachable c -> Lnp64.M2Transition.continuationUniqueState c.m2 := by
  intro h
  have H := chipReachable_invariant h
  obtain ⟨_, hh, _, _, _, _, _, _, _, _, _, _, _, _, _⟩ := H
  exact hh.1

theorem sg_servicelets_terminate_contained_whole_chip {c : Chip} :
    ChipReachable c -> Lnp64.M9Transition.budgetEnforcedFailsClosed c.m9 := by
  intro h
  have H := chipReachable_invariant h
  obtain ⟨_, _, _, _, _, _, _, _, hh, _, _, _, _, _, _⟩ := H
  exact hh.1

theorem sg_faults_terminal_progress_whole_chip {c : Chip} :
    ChipReachable c -> Lnp64.M10Transition.watchdogReachesDegradedReset c.m10 := by
  intro h
  have H := chipReachable_invariant h
  obtain ⟨_, _, _, _, _, _, _, _, _, hh, _, _, _, _, _⟩ := H
  exact hh.2.1

end Lnp64.WholeChip
