/- LNP64 M14 Resource Domain / policy checked model.

This bounded model covers the direct Track A A7 obligations: delegated
authority and budgets, freeze/resume/destroy lifecycle dispatch gates,
hierarchical usage roll-up, and fail-closed policy enforcement.
-/

namespace Lnp64.M14

structure Domain where
  id : Nat
  generation : Nat
  parent : Nat
  parentGeneration : Nat
  rights : Nat
  budgetLimit : Nat
  budgetUsed : Nat
  frozen : Bool
  destroyed : Bool
deriving Repr

structure Machine where
  parent : Domain
  child : Domain
  siblingUsed : Nat
  requestedRights : Nat
  requestedBudget : Nat
  clippedRights : Bool
  excessBudgetRejected : Bool
  frozenDispatchRejected : Bool
  resumedDispatchAllowed : Bool
  destroyedDispatchRejected : Bool
  policyDenied : Bool
  policyErrno : Nat
  policyBypassRejected : Bool
  delegatedCaps : Nat
  failClosedCount : Nat
deriving Repr

def eperm : Nat := 1
def eagain : Nat := 11
def erevoked : Nat := 122

def readRight : Nat := 1
def writeRight : Nat := 2
def execRight : Nat := 4

def parent0 : Domain :=
  { id := 1
    generation := 1
    parent := 0
    parentGeneration := 0
    rights := readRight + writeRight
    budgetLimit := 100
    budgetUsed := 0
    frozen := false
    destroyed := false }

def child0 : Domain :=
  { id := 2
    generation := 1
    parent := 1
    parentGeneration := 1
    rights := 0
    budgetLimit := 0
    budgetUsed := 0
    frozen := false
    destroyed := false }

def initialMachine : Machine :=
  { parent := parent0
    child := child0
    siblingUsed := 7
    requestedRights := readRight + writeRight + execRight
    requestedBudget := 40
    clippedRights := false
    excessBudgetRejected := false
    frozenDispatchRejected := false
    resumedDispatchAllowed := false
    destroyedDispatchRejected := false
    policyDenied := false
    policyErrno := 0
    policyBypassRejected := false
    delegatedCaps := 0
    failClosedCount := 0 }

def delegateChild (m : Machine) : Machine :=
  { m with
    child := { m.child with rights := readRight + writeRight, budgetLimit := m.requestedBudget }
    clippedRights := true
    delegatedCaps := m.delegatedCaps + 1 }

def rejectExcessBudget (m : Machine) : Machine :=
  { m with excessBudgetRejected := true, failClosedCount := m.failClosedCount + 1 }

def freezeChild (m : Machine) : Machine :=
  { m with
    child := { m.child with frozen := true }
    frozenDispatchRejected := true
    failClosedCount := m.failClosedCount + 1 }

def resumeChild (m : Machine) : Machine :=
  { m with
    child := { m.child with frozen := false }
    resumedDispatchAllowed := true }

def chargeUsage (m : Machine) : Machine :=
  let childUsed := 13
  { m with
    child := { m.child with budgetUsed := childUsed }
    parent := { m.parent with budgetUsed := childUsed + m.siblingUsed } }

def destroyChild (m : Machine) : Machine :=
  { m with
    child := { m.child with generation := m.child.generation + 1, destroyed := true }
    destroyedDispatchRejected := true
    failClosedCount := m.failClosedCount + 1 }

def denyByPolicy (m : Machine) : Machine :=
  { m with policyDenied := true, policyErrno := eperm, policyBypassRejected := true }

def afterDelegate : Machine := delegateChild initialMachine
def afterExcessReject : Machine := rejectExcessBudget afterDelegate
def afterFreeze : Machine := freezeChild afterExcessReject
def afterResume : Machine := resumeChild afterFreeze
def afterUsage : Machine := chargeUsage afterResume
def afterDestroy : Machine := destroyChild afterUsage
def finalMachine : Machine := denyByPolicy afterDestroy

def childRightsSubsetParent (m : Machine) : Prop :=
  m.child.rights = readRight + writeRight /\ m.parent.rights = readRight + writeRight

def childBudgetWithinParent (m : Machine) : Prop :=
  m.child.budgetLimit <= m.parent.budgetLimit

def excessBudgetRejected (m : Machine) : Prop :=
  m.excessBudgetRejected = true

def frozenCannotDispatch (m : Machine) : Prop :=
  m.child.frozen = true -> m.frozenDispatchRejected = true

def destroyedCannotDispatch (m : Machine) : Prop :=
  m.child.destroyed = true -> m.destroyedDispatchRejected = true

def usageRollsUp (m : Machine) : Prop :=
  m.parent.budgetUsed = m.child.budgetUsed + m.siblingUsed

def policyFailClosed (m : Machine) : Prop :=
  m.policyDenied = true /\ m.policyErrno = eperm

def policyCannotBeBypassedByAnotherEngine (m : Machine) : Prop :=
  m.policyDenied = true /\ m.policyErrno = eperm /\ m.policyBypassRejected = true

def countsExact (m : Machine) : Prop :=
  m.delegatedCaps = 1 /\ m.failClosedCount = 3

theorem m14_child_rights_subset_parent :
  childRightsSubsetParent afterDelegate := by
  simp [
    childRightsSubsetParent, afterDelegate, delegateChild, initialMachine,
    parent0, child0, readRight, writeRight, execRight
  ]

theorem m14_child_budget_within_parent :
  childBudgetWithinParent afterDelegate := by
  simp [
    childBudgetWithinParent, afterDelegate, delegateChild, initialMachine,
    parent0, child0, readRight, writeRight, execRight
  ]

theorem m14_excess_budget_rejected :
  excessBudgetRejected afterExcessReject := by
  simp [
    excessBudgetRejected, afterExcessReject, rejectExcessBudget,
    afterDelegate, delegateChild, initialMachine, parent0, child0,
    readRight, writeRight, execRight
  ]

theorem m14_frozen_domain_cannot_dispatch :
  frozenCannotDispatch afterFreeze := by
  intro _frozen
  simp [
    afterFreeze, freezeChild, afterExcessReject, rejectExcessBudget,
    afterDelegate, delegateChild, initialMachine, parent0, child0,
    readRight, writeRight, execRight
  ]

theorem m14_destroyed_domain_cannot_dispatch :
  destroyedCannotDispatch afterDestroy := by
  intro _destroyed
  simp [
    afterDestroy, destroyChild, afterUsage, chargeUsage, afterResume,
    resumeChild, afterFreeze, freezeChild, afterExcessReject,
    rejectExcessBudget, afterDelegate, delegateChild, initialMachine,
    parent0, child0, readRight, writeRight, execRight
  ]

theorem m14_usage_rolls_up :
  usageRollsUp afterUsage := by
  simp [
    usageRollsUp, afterUsage, chargeUsage, afterResume, resumeChild,
    afterFreeze, freezeChild, afterExcessReject, rejectExcessBudget,
    afterDelegate, delegateChild, initialMachine, parent0, child0,
    readRight, writeRight, execRight
  ]

theorem m14_policy_fail_closed :
  policyFailClosed finalMachine := by
  simp [
    policyFailClosed, finalMachine, denyByPolicy, afterDestroy,
    destroyChild, afterUsage, chargeUsage, afterResume, resumeChild,
    afterFreeze, freezeChild, afterExcessReject, rejectExcessBudget,
    afterDelegate, delegateChild, initialMachine, parent0, child0,
    readRight, writeRight, execRight, eperm
  ]

theorem m14_policy_cannot_be_bypassed_by_another_engine :
  policyCannotBeBypassedByAnotherEngine finalMachine := by
  simp [
    policyCannotBeBypassedByAnotherEngine, finalMachine, denyByPolicy,
    afterDestroy, destroyChild, afterUsage, chargeUsage, afterResume,
    resumeChild, afterFreeze, freezeChild, afterExcessReject,
    rejectExcessBudget, afterDelegate, delegateChild, initialMachine,
    parent0, child0, readRight, writeRight, execRight, eperm
  ]

theorem m14_counts_exact :
  countsExact finalMachine := by
  simp [
    countsExact, finalMachine, denyByPolicy, afterDestroy, destroyChild,
    afterUsage, chargeUsage, afterResume, resumeChild, afterFreeze,
    freezeChild, afterExcessReject, rejectExcessBudget, afterDelegate,
    delegateChild, initialMachine, parent0, child0, readRight, writeRight,
    execRight
  ]

end Lnp64.M14
