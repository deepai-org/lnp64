/- LNP64 S0 abstract machine skeleton.

This file is a lightweight Lean-style proof artifact for the first RTL
milestone. It fixes the state names and theorem targets used by the RTL
assertions and simulation gate; later work can replace the admitted proof bodies
with machine-checked proofs without changing the public obligations.
-/

namespace Lnp64.S0

inductive Terminal
  | response
  | error
  | event
  | cancellation
  | fault
  | degraded
deriving DecidableEq, Repr

inductive SchedLocation
  | none
  | runnable
  | running
  | parked
  | faulted
deriving DecidableEq, Repr

structure Domain where
  id : Nat
  generation : Nat
  parent : Nat
  parentGeneration : Nat
deriving Repr

structure Capability where
  objectId : Nat
  objectGeneration : Nat
  fdrGeneration : Nat
  domainId : Nat
  domainGeneration : Nat
  rightsMask : Nat
  lineageEpoch : Nat
  sealed : Bool
  narrowable : Bool
deriving Repr

structure Thread where
  pid : Nat
  tid : Nat
  domainId : Nat
  domainGeneration : Nat
  location : SchedLocation
  parkedSource : Option Nat
deriving Repr

structure Command where
  opId : Nat
  opcode : Nat
  pid : Nat
  tid : Nat
  domainId : Nat
  domainGeneration : Nat
  terminal : Option Terminal
  mintsAuthority : Bool
  exposesRawAuthority : Bool
deriving Repr

structure Machine where
  bootFault : Bool
  measuredBootFault : Bool
  rootDomain : Option Domain
  pid1 : Option Thread
  rootFdr : Option Capability
  commands : List Command
  unsupportedFailClosed : Bool
  rawAuthorityVisible : Bool
deriving Repr

def validRoot (d : Domain) : Prop :=
  d.id = 1 /\ d.generation = 1 /\ d.parent = 0

def validPid1 (t : Thread) : Prop :=
  t.pid = 1 /\ t.tid = 1 /\ t.domainId = 1 /\ t.domainGeneration = 1

def exactlyOneLocation (t : Thread) : Prop :=
  t.location != SchedLocation.none

def parkedHasSource (t : Thread) : Prop :=
  t.location = SchedLocation.parked -> t.parkedSource.isSome

def terminalPath (c : Command) : Prop :=
  c.terminal.isSome

def noStubAuthority (c : Command) : Prop :=
  c.mintsAuthority = false

def noRawAuthority (m : Machine) : Prop :=
  m.rawAuthorityVisible = false /\ forall c in m.commands, c.exposesRawAuthority = false

def validInitialState (m : Machine) : Prop :=
  (exists d, m.rootDomain = some d /\ validRoot d) /\
  (exists t, m.pid1 = some t /\ validPid1 t /\ exactlyOneLocation t) /\
  (exists c, m.rootFdr = some c /\ c.domainId = 1 /\ c.domainGeneration = 1)

def resetObligation (m : Machine) : Prop :=
  validInitialState m \/ (m.bootFault = true /\ m.measuredBootFault = true)

def schedulerObligation (m : Machine) : Prop :=
  forall t, m.pid1 = some t -> exactlyOneLocation t /\ parkedHasSource t

def terminalObligation (m : Machine) : Prop :=
  forall c in m.commands, terminalPath c

def authorityObligation (m : Machine) : Prop :=
  (forall c in m.commands, noStubAuthority c) /\ noRawAuthority m

def unsupportedObligation (m : Machine) : Prop :=
  m.unsupportedFailClosed = true

axiom s0_reset_produces_valid_initial_state_or_measured_fault :
  forall m : Machine, resetObligation m

axiom s0_every_live_thread_has_exactly_one_scheduler_location :
  forall m : Machine, schedulerObligation m

axiom s0_every_accepted_command_has_terminal_path :
  forall m : Machine, terminalObligation m

axiom s0_stubs_do_not_create_authority :
  forall m : Machine, authorityObligation m

axiom s0_unsupported_operations_fail_closed :
  forall m : Machine, unsupportedObligation m

end Lnp64.S0
