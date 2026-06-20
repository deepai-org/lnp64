/- LNP64 M6 typed-control/namespace/service-boundary checked model.

This bounded model names the proof targets exercised by
`formal/m6_service_model.py` and `rtl/engines/lnp64_m6_service.sv`.
The obligations below are proved over the bounded typed-control trace.
-/

namespace Lnp64.M6

structure Envelope where
  version : Nat
  profile : Nat
  valid : Bool
deriving Repr

structure CapabilityProposal where
  objectId : Nat
  requestedRights : Nat
  returnedRights : Nat
  installed : Bool
deriving Repr

structure Machine where
  envelope : Envelope
  namespaceDispatched : Bool
  serviceContinuation : Nat
  serviceGeneration : Nat
  staleServiceGeneration : Nat
  proposal : CapabilityProposal
  cancelTerminal : Bool
  staleServiceRejected : Bool
  crashCompleted : Bool
  installedCaps : Nat
  completions : Nat
deriving Repr

def namespaceProfile : Nat := 1
def readRight : Nat := 1
def writeRight : Nat := 2

def envelopeValidated (m : Machine) : Prop :=
  m.envelope.version = 1 /\ m.envelope.profile = namespaceProfile /\ m.envelope.valid = true

def namespaceDispatchRan (m : Machine) : Prop :=
  m.namespaceDispatched = true

def serviceContinuationCreated (m : Machine) : Prop :=
  m.serviceContinuation > 0

def returnedCapabilityNarrowed (m : Machine) : Prop :=
  m.proposal.returnedRights = readRight /\
  m.proposal.requestedRights = readRight + writeRight

def capabilityInstalled (m : Machine) : Prop :=
  m.proposal.installed = true /\ m.installedCaps = 1

def serviceCancelTerminal (m : Machine) : Prop :=
  m.cancelTerminal = true

def staleServiceRejected (m : Machine) : Prop :=
  m.staleServiceGeneration != m.serviceGeneration -> m.staleServiceRejected = true

def crashCompletionRecorded (m : Machine) : Prop :=
  m.crashCompleted = true /\ m.completions = 2

def envelope0 : Envelope :=
  { version := 0, profile := 0, valid := false }

def proposal0 : CapabilityProposal :=
  { objectId := 0, requestedRights := 0, returnedRights := 0, installed := false }

def initialMachine : Machine :=
  { envelope := envelope0
    namespaceDispatched := false
    serviceContinuation := 0
    serviceGeneration := 1
    staleServiceGeneration := 1
    proposal := proposal0
    cancelTerminal := false
    staleServiceRejected := false
    crashCompleted := false
    installedCaps := 0
    completions := 0 }

def validateEnvelope (m : Machine) : Machine :=
  { m with envelope := { version := 1, profile := namespaceProfile, valid := true } }

def dispatchNamespace (m : Machine) : Machine :=
  { m with namespaceDispatched := true }

def createServiceContinuation (m : Machine) : Machine :=
  { m with serviceContinuation := 1 }

def installReturnedCapability (m : Machine) : Machine :=
  { m with
    proposal :=
      { objectId := 9
        requestedRights := readRight + writeRight
        returnedRights := readRight
        installed := true }
    installedCaps := m.installedCaps + 1 }

def cancelService (m : Machine) : Machine :=
  { m with cancelTerminal := true, completions := m.completions + 1 }

def rejectStaleService (m : Machine) : Machine :=
  { m with
    serviceGeneration := m.serviceGeneration + 1
    staleServiceRejected := true }

def completeCrash (m : Machine) : Machine :=
  { m with crashCompleted := true, completions := m.completions + 1 }

def afterEnvelope : Machine :=
  validateEnvelope initialMachine

def afterDispatch : Machine :=
  dispatchNamespace afterEnvelope

def afterContinuation : Machine :=
  createServiceContinuation afterDispatch

def afterCapability : Machine :=
  installReturnedCapability afterContinuation

def afterCancel : Machine :=
  cancelService afterCapability

def afterStaleReject : Machine :=
  rejectStaleService afterCancel

def finalMachine : Machine :=
  completeCrash afterStaleReject

theorem m6_envelope_validated :
  envelopeValidated afterEnvelope := by
  simp [
    envelopeValidated, afterEnvelope, validateEnvelope, initialMachine,
    envelope0, proposal0, namespaceProfile
  ]

theorem m6_namespace_dispatch_ran :
  namespaceDispatchRan afterDispatch := by
  simp [
    namespaceDispatchRan, afterDispatch, dispatchNamespace, afterEnvelope,
    validateEnvelope, initialMachine, envelope0, proposal0, namespaceProfile
  ]

theorem m6_service_continuation_created :
  serviceContinuationCreated afterContinuation := by
  simp [
    serviceContinuationCreated, afterContinuation, createServiceContinuation,
    afterDispatch, dispatchNamespace, afterEnvelope, validateEnvelope,
    initialMachine, envelope0, proposal0, namespaceProfile
  ]

theorem m6_returned_capability_narrowed :
  returnedCapabilityNarrowed afterCapability := by
  simp [
    returnedCapabilityNarrowed, afterCapability, installReturnedCapability,
    afterContinuation, createServiceContinuation, afterDispatch,
    dispatchNamespace, afterEnvelope, validateEnvelope, initialMachine,
    envelope0, proposal0, namespaceProfile, readRight, writeRight
  ]

theorem m6_capability_installed :
  capabilityInstalled afterCapability := by
  simp [
    capabilityInstalled, afterCapability, installReturnedCapability,
    afterContinuation, createServiceContinuation, afterDispatch,
    dispatchNamespace, afterEnvelope, validateEnvelope, initialMachine,
    envelope0, proposal0, namespaceProfile, readRight, writeRight
  ]

theorem m6_service_cancel_terminal :
  serviceCancelTerminal afterCancel := by
  simp [
    serviceCancelTerminal, afterCancel, cancelService, afterCapability,
    installReturnedCapability, afterContinuation, createServiceContinuation,
    afterDispatch, dispatchNamespace, afterEnvelope, validateEnvelope,
    initialMachine, envelope0, proposal0, namespaceProfile, readRight,
    writeRight
  ]

theorem m6_stale_service_rejected :
  staleServiceRejected afterStaleReject := by
  intro _stale
  simp [
    afterStaleReject, rejectStaleService, afterCancel, cancelService,
    afterCapability, installReturnedCapability, afterContinuation,
    createServiceContinuation, afterDispatch, dispatchNamespace,
    afterEnvelope, validateEnvelope, initialMachine, envelope0, proposal0,
    namespaceProfile, readRight, writeRight
  ]

theorem m6_crash_completion_recorded :
  crashCompletionRecorded finalMachine := by
  simp [
    crashCompletionRecorded, finalMachine, completeCrash, afterStaleReject,
    rejectStaleService, afterCancel, cancelService, afterCapability,
    installReturnedCapability, afterContinuation, createServiceContinuation,
    afterDispatch, dispatchNamespace, afterEnvelope, validateEnvelope,
    initialMachine, envelope0, proposal0, namespaceProfile, readRight,
    writeRight
  ]

/- Packed-bit decode model for the M6 service-boundary witness.

Mirrors the M1/M2/M3/M4/M5/M7/M14 packed-bit machinery so the emitted
lnp64_m6_service_commit_t and lnp64_m6_state_projection_t bit vectors can be
decode-checked against this Lean model. Every M6 field is a plain scalar/bool
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

def rtlM6CommitPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("service_id", 32)
  , ("op_id", 32)
  , ("continuation_generation", 32)
  , ("service_generation", 32)
  , ("requested_rights", 64)
  , ("returned_rights", 64) ]

def rtlM6StateProjectionPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("service_generation", 32)
  , ("continuation_generation", 32)
  , ("installed_caps", 32)
  , ("completions", 32)
  , ("envelope_validated", 1)
  , ("namespace_dispatched", 1)
  , ("service_continuation_created", 1)
  , ("cap_return_installed", 1)
  , ("returned_cap_narrowed", 1)
  , ("cancel_terminal", 1)
  , ("stale_service_rejected", 1)
  , ("crash_completed", 1) ]

def rtlM6CommitPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM6CommitPackedSchema

def rtlM6StateProjectionPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM6StateProjectionPackedSchema

theorem rtlM6CommitPackedSchema_width :
    packedSchemaWidth rtlM6CommitPackedSchema = 280 := by
  decide

theorem rtlM6StateProjectionPackedSchema_width :
    packedSchemaWidth rtlM6StateProjectionPackedSchema = 160 := by
  decide

theorem rtlM6CommitPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM6CommitPackedSchema)
      rtlM6CommitPackedLayout = true := by
  decide

theorem rtlM6StateProjectionPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM6StateProjectionPackedSchema)
      rtlM6StateProjectionPackedLayout = true := by
  decide

end Lnp64.M6
