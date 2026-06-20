/- LNP64 M14 transition-invariant model.

`M14ResourceDomainPolicyModel.lean` is a bounded Resource Domain / policy
witness. This file adds a small transition-invariant proof slice for Resource
Domain containment: typed domain state, typed operations, a `Step` relation,
`Reachable`, preservation, and theorems over arbitrary reachable states.
-/

namespace Lnp64.M14Transition

structure Rights where
  read : Bool
  write : Bool
  exec : Bool
deriving DecidableEq, Repr

def Rights.subset (child parent : Rights) : Prop :=
  (child.read = true -> parent.read = true) /\
  (child.write = true -> parent.write = true) /\
  (child.exec = true -> parent.exec = true)

def Rights.intersect (requested parent : Rights) : Rights :=
  { read := requested.read && parent.read
    write := requested.write && parent.write
    exec := requested.exec && parent.exec }

def parentRights0 : Rights :=
  { read := true, write := true, exec := false }

def noRights : Rights :=
  { read := false, write := false, exec := false }

structure Domain where
  id : Nat
  generation : Nat
  parent : Nat
  parentGeneration : Nat
  rights : Rights
  budgetLimit : Nat
  budgetUsed : Nat
  frozen : Bool
  destroyed : Bool
deriving DecidableEq, Repr

structure State where
  parent : Domain
  child : Domain
  siblingUsed : Nat
  frozenDispatchRejected : Bool
  destroyedDispatchRejected : Bool
  policyDenied : Bool
  policyErrno : Nat
  policyBypassRejected : Bool
  usageCharged : Bool
  delegatedCaps : Nat
  failClosedCount : Nat
deriving DecidableEq, Repr

inductive Op
  | delegateChild (requestedRights : Rights) (requestedBudget : Nat)
  | rejectExcessBudget
  | freezeChild
  | resumeChild
  | chargeUsage (childUsed siblingUsed : Nat)
  | destroyChild
  | denyByPolicy
deriving DecidableEq, Repr

def eperm : Nat := 1

def parent0 : Domain :=
  { id := 1
    generation := 1
    parent := 0
    parentGeneration := 0
    rights := parentRights0
    budgetLimit := 100
    budgetUsed := 0
    frozen := false
    destroyed := false }

def child0 : Domain :=
  { id := 2
    generation := 1
    parent := 1
    parentGeneration := 1
    rights := noRights
    budgetLimit := 0
    budgetUsed := 0
    frozen := false
    destroyed := false }

def reset : State :=
  { parent := parent0
    child := child0
    siblingUsed := 0
    frozenDispatchRejected := false
    destroyedDispatchRejected := false
    policyDenied := false
    policyErrno := 0
    policyBypassRejected := false
    usageCharged := false
    delegatedCaps := 0
    failClosedCount := 0 }

def lineageValid (s : State) : Prop :=
  s.child.parent = s.parent.id /\ s.child.parentGeneration = s.parent.generation

def childRightsSubsetParentState (s : State) : Prop :=
  Rights.subset s.child.rights s.parent.rights

def childBudgetWithinParentState (s : State) : Prop :=
  s.child.budgetLimit <= s.parent.budgetLimit

def frozenCannotDispatchState (s : State) : Prop :=
  s.child.frozen = true -> s.frozenDispatchRejected = true

def destroyedCannotDispatchState (s : State) : Prop :=
  s.child.destroyed = true -> s.destroyedDispatchRejected = true

def usageRollsUpState (s : State) : Prop :=
  s.usageCharged = true -> s.parent.budgetUsed = s.child.budgetUsed + s.siblingUsed

def policyFailClosedState (s : State) : Prop :=
  s.policyDenied = true -> s.policyErrno = eperm /\ s.policyBypassRejected = true

def invariant (s : State) : Prop :=
  lineageValid s /\
  childRightsSubsetParentState s /\
  childBudgetWithinParentState s /\
  frozenCannotDispatchState s /\
  destroyedCannotDispatchState s /\
  usageRollsUpState s /\
  policyFailClosedState s

theorem rights_intersect_subset_parent (requested parent : Rights) :
    Rights.subset (Rights.intersect requested parent) parent := by
  cases requested
  cases parent
  simp [Rights.subset, Rights.intersect]

inductive Step : State -> Op -> State -> Prop
  | delegateChild (s : State) (requestedRights : Rights) (requestedBudget : Nat) :
      requestedBudget <= s.parent.budgetLimit ->
      Step s (Op.delegateChild requestedRights requestedBudget)
        { s with
          child := { s.child with
            rights := Rights.intersect requestedRights s.parent.rights
            budgetLimit := requestedBudget }
          delegatedCaps := s.delegatedCaps + 1 }
  | rejectExcessBudget (s : State) :
      Step s Op.rejectExcessBudget
        { s with failClosedCount := s.failClosedCount + 1 }
  | freezeChild (s : State) :
      Step s Op.freezeChild
        { s with
          child := { s.child with frozen := true }
          frozenDispatchRejected := true
          failClosedCount := s.failClosedCount + 1 }
  | resumeChild (s : State) :
      Step s Op.resumeChild
        { s with child := { s.child with frozen := false } }
  | chargeUsage (s : State) (childUsed siblingUsed : Nat) :
      childUsed + siblingUsed <= s.parent.budgetLimit ->
      Step s (Op.chargeUsage childUsed siblingUsed)
        { s with
          child := { s.child with budgetUsed := childUsed }
          parent := { s.parent with budgetUsed := childUsed + siblingUsed }
          siblingUsed := siblingUsed
          usageCharged := true }
  | destroyChild (s : State) :
      Step s Op.destroyChild
        { s with
          child := { s.child with generation := s.child.generation + 1, destroyed := true }
          destroyedDispatchRejected := true
          failClosedCount := s.failClosedCount + 1 }
  | denyByPolicy (s : State) :
      Step s Op.denyByPolicy
        { s with
          policyDenied := true
          policyErrno := eperm
          policyBypassRejected := true }

inductive Reachable : State -> Prop
  | reset : Reachable reset
  | step {s t : State} {op : Op} :
      Reachable s -> Step s op t -> Reachable t

theorem invariant_reset :
    invariant reset := by
  simp [
    invariant, reset, parent0, child0, parentRights0, noRights, lineageValid,
    childRightsSubsetParentState, childBudgetWithinParentState,
    frozenCannotDispatchState, destroyedCannotDispatchState,
    usageRollsUpState, policyFailClosedState, Rights.subset
  ]

theorem invariant_step {s t : State} {op : Op} :
    invariant s -> Step s op t -> invariant t := by
  intro hInv hStep
  cases hStep <;>
    simp_all [
      invariant, lineageValid, childRightsSubsetParentState,
      childBudgetWithinParentState, frozenCannotDispatchState,
      destroyedCannotDispatchState, usageRollsUpState, policyFailClosedState,
      rights_intersect_subset_parent
    ]

theorem reachable_invariant {s : State} :
    Reachable s -> invariant s := by
  intro hReach
  induction hReach with
  | reset => exact invariant_reset
  | step hPrev hStep ih => exact invariant_step ih hStep

theorem m14_t3_child_rights_subset_parent_for_all_reachable {s : State} :
    Reachable s -> childRightsSubsetParentState s := by
  intro hReach
  exact (reachable_invariant hReach).2.1

theorem m14_t3_child_budget_within_parent_for_all_reachable {s : State} :
    Reachable s -> childBudgetWithinParentState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.1

theorem m14_t3_frozen_domain_cannot_dispatch_for_all_reachable {s : State} :
    Reachable s -> frozenCannotDispatchState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.1

theorem m14_t3_destroyed_domain_cannot_dispatch_for_all_reachable {s : State} :
    Reachable s -> destroyedCannotDispatchState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.1

theorem m14_t3_usage_rolls_up_for_all_reachable {s : State} :
    Reachable s -> usageRollsUpState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.2.1

theorem m14_t3_policy_fail_closed_for_all_reachable {s : State} :
    Reachable s -> policyFailClosedState s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2.2.2.2

/- Packed-bit decode model for the M14 Resource Domain witness.

Mirrors the M1/M2/M4/M5/M7 packed-bit machinery so the emitted
lnp64_m14_domain_commit_t and lnp64_m14_state_projection_t bit vectors can be
decode-checked against this Lean model. Every M14 field is a plain scalar/bool
slice. -/

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

def rtlM14CommitPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("root_domain", 32)
  , ("child_domain", 32)
  , ("child_budget", 32)
  , ("parent_budget", 32)
  , ("requested_rights", 64)
  , ("delegated_rights", 64) ]

def rtlM14StateProjectionPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("root_domain", 32)
  , ("child_domain", 32)
  , ("delegated_caps", 32)
  , ("failures", 32)
  , ("parent_used", 32)
  , ("child_rights_subset_parent", 1)
  , ("child_budget_within_parent", 1)
  , ("excess_budget_rejected", 1)
  , ("frozen_dispatch_rejected", 1)
  , ("resumed_dispatch_allowed", 1)
  , ("destroyed_dispatch_rejected", 1)
  , ("usage_rollup_valid", 1)
  , ("policy_fail_closed", 1)
  , ("counts_exact", 1) ]

def rtlM14CommitPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM14CommitPackedSchema

def rtlM14StateProjectionPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM14StateProjectionPackedSchema

theorem rtlM14CommitPackedSchema_width :
    packedSchemaWidth rtlM14CommitPackedSchema = 280 := by
  decide

theorem rtlM14StateProjectionPackedSchema_width :
    packedSchemaWidth rtlM14StateProjectionPackedSchema = 193 := by
  decide

theorem rtlM14CommitPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM14CommitPackedSchema)
      rtlM14CommitPackedLayout = true := by
  decide

theorem rtlM14StateProjectionPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM14StateProjectionPackedSchema)
      rtlM14StateProjectionPackedLayout = true := by
  decide

end Lnp64.M14Transition
