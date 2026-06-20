/- LNP64 M10 transition-invariant model.

`M10RasModel.lean` is a bounded RAS/attestation witness. This file adds a
transition-invariant proof slice for the SG-PROGRESS RAS guarantees: typed
state, typed operations, a `Step` relation, `Reachable`, preservation, and
theorems over arbitrary reachable states (a parity-poison fault always fails
closed, a watchdog timeout always reaches a degraded local reset, an MLS/debug
audit access is always denied, and no RAS path ever creates authority).
-/

namespace Lnp64.M10Transition

structure State where
  parityObserved : Bool
  parityFaultClosed : Bool
  watchdogObserved : Bool
  degradedReset : Bool
  auditObserved : Bool
  auditRecorded : Bool
  mlsDenied : Bool
  authorityCreated : Bool
  faults : Nat
deriving DecidableEq, Repr

inductive Op
  | bootMeasure
  | eccCorrect
  | parityPoison
  | watchdogTimeout
  | telemetryRead
  | auditMls
deriving DecidableEq, Repr

def reset : State :=
  { parityObserved := false
    parityFaultClosed := false
    watchdogObserved := false
    degradedReset := false
    auditObserved := false
    auditRecorded := false
    mlsDenied := false
    authorityCreated := false
    faults := 0 }

def parityFailsClosed (s : State) : Prop :=
  s.parityObserved = true -> s.parityFaultClosed = true

def watchdogReachesDegradedReset (s : State) : Prop :=
  s.watchdogObserved = true -> s.degradedReset = true

def auditMlsFailsClosed (s : State) : Prop :=
  s.auditObserved = true -> (s.auditRecorded = true /\ s.mlsDenied = true)

def noAuthorityCreated (s : State) : Prop :=
  s.authorityCreated = false

def invariant (s : State) : Prop :=
  parityFailsClosed s /\
  watchdogReachesDegradedReset s /\
  auditMlsFailsClosed s /\
  noAuthorityCreated s

inductive Step : State -> Op -> State -> Prop
  | bootMeasure (s : State) :
      Step s Op.bootMeasure s
  | eccCorrect (s : State) :
      Step s Op.eccCorrect s
  | parityPoison (s : State) :
      Step s Op.parityPoison
        { s with parityObserved := true, parityFaultClosed := true, faults := s.faults + 1 }
  | watchdogTimeout (s : State) :
      Step s Op.watchdogTimeout
        { s with watchdogObserved := true, degradedReset := true }
  | telemetryRead (s : State) :
      Step s Op.telemetryRead s
  | auditMls (s : State) :
      Step s Op.auditMls
        { s with auditObserved := true, auditRecorded := true, mlsDenied := true }

inductive Reachable : State -> Prop
  | reset : Reachable reset
  | step {s t : State} {op : Op} :
      Reachable s -> Step s op t -> Reachable t

theorem invariant_reset : invariant reset := by
  simp [invariant, reset, parityFailsClosed, watchdogReachesDegradedReset,
    auditMlsFailsClosed, noAuthorityCreated]

theorem invariant_step {s t : State} {op : Op} :
    invariant s -> Step s op t -> invariant t := by
  intro hInv hStep
  cases hStep <;>
    simp_all [invariant, parityFailsClosed, watchdogReachesDegradedReset,
      auditMlsFailsClosed, noAuthorityCreated]

theorem reachable_invariant {s : State} :
    Reachable s -> invariant s := by
  intro hReach
  induction hReach with
  | reset => exact invariant_reset
  | step _ hStep ih => exact invariant_step ih hStep

theorem m10_t3_parity_poison_fails_closed_for_all_reachable {s : State} :
    Reachable s -> parityFailsClosed s := by
  intro hReach
  exact (reachable_invariant hReach).1

theorem m10_t3_watchdog_reaches_degraded_reset_for_all_reachable {s : State} :
    Reachable s -> watchdogReachesDegradedReset s := by
  intro hReach
  exact (reachable_invariant hReach).2.1

theorem m10_t3_audit_mls_fails_closed_for_all_reachable {s : State} :
    Reachable s -> auditMlsFailsClosed s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.1

theorem m10_t3_no_authority_created_for_all_reachable {s : State} :
    Reachable s -> noAuthorityCreated s := by
  intro hReach
  exact (reachable_invariant hReach).2.2.2

end Lnp64.M10Transition
