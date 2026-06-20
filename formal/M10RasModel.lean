/- LNP64 M10 RAS/observability/assurance checked model.

This bounded model names the proof targets exercised by
`formal/m10_ras_model.py` and `rtl/engines/lnp64_m10_ras.sv`.
The obligations below are proved over the bounded RAS trace.
-/

namespace Lnp64.M10

structure Machine where
  bootMeasured : Bool
  telemetryFdrPresent : Bool
  eccCorrected : Bool
  parityPoisonFaulted : Bool
  watchdogTimedOut : Bool
  localResetSeen : Bool
  degradedState : Bool
  telemetryScoped : Bool
  telemetryRedacted : Bool
  traceOverflowed : Bool
  quoteMeasurementBound : Bool
  quoteDevelopmentMarked : Bool
  adversarialInputRejected : Bool
  ownerEngineHung : Bool
  adversarialAuthorityCreated : Bool
  boundedLocalFaultTerminal : Bool
  unrelatedDomainIntact : Bool
  realtimeWorkAdmitted : Bool
  arbitrationBounded : Bool
  progressPathObserved : Bool
  auditRecorded : Bool
  mlsDenied : Bool
  debugDenied : Bool
  faultCount : Nat
  telemetryReads : Nat
  auditRecords : Nat
deriving Repr

def bootCreatesMeasuredObservability (m : Machine) : Prop :=
  m.bootMeasured = true /\ m.telemetryFdrPresent = true

def metadataFaultsContained (m : Machine) : Prop :=
  m.eccCorrected = true /\ m.parityPoisonFaulted = true /\ m.faultCount >= 1

def watchdogReachesDegradedReset (m : Machine) : Prop :=
  m.watchdogTimedOut = true /\ m.localResetSeen = true /\ m.degradedState = true

def telemetryReadScoped (m : Machine) : Prop :=
  m.telemetryScoped = true /\ m.telemetryRedacted = true /\ m.telemetryReads = 1

def traceOverflowVisible (m : Machine) : Prop :=
  m.traceOverflowed = true

def quoteStubBoundToMeasurement (m : Machine) : Prop :=
  m.quoteMeasurementBound = true /\ m.quoteDevelopmentMarked = true

def auditDebugMlsFailClosed (m : Machine) : Prop :=
  m.auditRecorded = true /\ m.mlsDenied = true /\ m.debugDenied = true /\ m.auditRecords = 1

def adversarialInputsCannotHangOrCreateAuthority (m : Machine) : Prop :=
  m.adversarialInputRejected = true /\
  m.ownerEngineHung = false /\
  m.adversarialAuthorityCreated = false

def boundedLocalFaultReachesTerminalPath (m : Machine) : Prop :=
  m.boundedLocalFaultTerminal = true /\ m.faultCount >= 1

def watchdogResetDoesNotCorruptUnrelatedDomains (m : Machine) : Prop :=
  m.watchdogTimedOut = true /\ m.localResetSeen = true /\ m.unrelatedDomainIntact = true

def realtimeWorkHasBoundedProgressPath (m : Machine) : Prop :=
  m.realtimeWorkAdmitted = true /\ m.arbitrationBounded = true /\ m.progressPathObserved = true

def countsExact (m : Machine) : Prop :=
  m.faultCount = 2 /\ m.telemetryReads = 1 /\ m.auditRecords = 1

def initialMachine : Machine :=
  { bootMeasured := false
    telemetryFdrPresent := false
    eccCorrected := false
    parityPoisonFaulted := false
    watchdogTimedOut := false
    localResetSeen := false
    degradedState := false
    telemetryScoped := false
    telemetryRedacted := false
    traceOverflowed := false
    quoteMeasurementBound := false
    quoteDevelopmentMarked := false
    adversarialInputRejected := false
    ownerEngineHung := false
    adversarialAuthorityCreated := false
    boundedLocalFaultTerminal := false
    unrelatedDomainIntact := true
    realtimeWorkAdmitted := false
    arbitrationBounded := false
    progressPathObserved := false
    auditRecorded := false
    mlsDenied := false
    debugDenied := false
    faultCount := 0
    telemetryReads := 0
    auditRecords := 0 }

def boot (m : Machine) : Machine :=
  { m with bootMeasured := true, telemetryFdrPresent := true }

def correctEcc (m : Machine) : Machine :=
  { m with eccCorrected := true }

def poisonParity (m : Machine) : Machine :=
  { m with
    parityPoisonFaulted := true
    boundedLocalFaultTerminal := true
    faultCount := m.faultCount + 1 }

def watchdogTimeout (m : Machine) : Machine :=
  { m with
    watchdogTimedOut := true
    localResetSeen := true
    degradedState := true
    faultCount := m.faultCount + 1 }

def scopedTelemetryRead (m : Machine) : Machine :=
  { m with
    telemetryScoped := true
    telemetryRedacted := true
    telemetryReads := m.telemetryReads + 1 }

def traceOverflow (m : Machine) : Machine :=
  { m with traceOverflowed := true }

def quoteStub (m : Machine) : Machine :=
  { m with quoteMeasurementBound := true, quoteDevelopmentMarked := true }

def rejectAdversarialInput (m : Machine) : Machine :=
  { m with adversarialInputRejected := true }

def admitRealtimeWork (m : Machine) : Machine :=
  { m with
    realtimeWorkAdmitted := true
    arbitrationBounded := true
    progressPathObserved := true }

def auditMlsDeny (m : Machine) : Machine :=
  { m with
    auditRecorded := true
    mlsDenied := true
    debugDenied := true
    auditRecords := m.auditRecords + 1 }

def afterBoot : Machine :=
  boot initialMachine

def afterEcc : Machine :=
  correctEcc afterBoot

def afterPoison : Machine :=
  poisonParity afterEcc

def afterWatchdog : Machine :=
  watchdogTimeout afterPoison

def afterTelemetry : Machine :=
  scopedTelemetryRead afterWatchdog

def afterTraceOverflow : Machine :=
  traceOverflow afterTelemetry

def afterQuote : Machine :=
  quoteStub afterTraceOverflow

def afterAdversarialInput : Machine :=
  rejectAdversarialInput afterQuote

def afterRealtimeWork : Machine :=
  admitRealtimeWork afterAdversarialInput

def finalMachine : Machine :=
  auditMlsDeny afterRealtimeWork

theorem m10_boot_measured_observability :
  bootCreatesMeasuredObservability afterBoot := by
  simp [bootCreatesMeasuredObservability, afterBoot, boot, initialMachine]

theorem m10_metadata_faults_contained :
  metadataFaultsContained afterPoison := by
  simp [
    metadataFaultsContained, afterPoison, poisonParity, afterEcc, correctEcc,
    afterBoot, boot, initialMachine
  ]

theorem m10_watchdog_reaches_degraded_reset :
  watchdogReachesDegradedReset afterWatchdog := by
  simp [
    watchdogReachesDegradedReset, afterWatchdog, watchdogTimeout, afterPoison,
    poisonParity, afterEcc, correctEcc, afterBoot, boot, initialMachine
  ]

theorem m10_telemetry_read_scoped :
  telemetryReadScoped afterTelemetry := by
  simp [
    telemetryReadScoped, afterTelemetry, scopedTelemetryRead, afterWatchdog,
    watchdogTimeout, afterPoison, poisonParity, afterEcc, correctEcc,
    afterBoot, boot, initialMachine
  ]

theorem m10_trace_overflow_visible :
  traceOverflowVisible afterTraceOverflow := by
  simp [
    traceOverflowVisible, afterTraceOverflow, traceOverflow, afterTelemetry,
    scopedTelemetryRead, afterWatchdog, watchdogTimeout, afterPoison,
    poisonParity, afterEcc, correctEcc, afterBoot, boot, initialMachine
  ]

theorem m10_quote_stub_bound_to_measurement :
  quoteStubBoundToMeasurement afterQuote := by
  simp [
    quoteStubBoundToMeasurement, afterQuote, quoteStub, afterTraceOverflow,
    traceOverflow, afterTelemetry, scopedTelemetryRead, afterWatchdog,
    watchdogTimeout, afterPoison, poisonParity, afterEcc, correctEcc,
    afterBoot, boot, initialMachine
  ]

theorem m10_audit_debug_mls_fail_closed :
  auditDebugMlsFailClosed finalMachine := by
  simp [
    auditDebugMlsFailClosed, finalMachine, auditMlsDeny, afterQuote, quoteStub,
    afterRealtimeWork, admitRealtimeWork, afterAdversarialInput,
    rejectAdversarialInput, afterTraceOverflow, traceOverflow, afterTelemetry,
    scopedTelemetryRead, afterWatchdog, watchdogTimeout, afterPoison,
    poisonParity, afterEcc, correctEcc, afterBoot, boot, initialMachine
  ]

theorem m10_adversarial_inputs_cannot_hang_owner_or_create_authority :
  adversarialInputsCannotHangOrCreateAuthority afterAdversarialInput := by
  simp [
    adversarialInputsCannotHangOrCreateAuthority, afterAdversarialInput,
    rejectAdversarialInput, afterQuote, quoteStub, afterTraceOverflow,
    traceOverflow, afterTelemetry, scopedTelemetryRead, afterWatchdog,
    watchdogTimeout, afterPoison, poisonParity, afterEcc, correctEcc,
    afterBoot, boot, initialMachine
  ]

theorem m10_bounded_local_fault_reaches_terminal_path :
  boundedLocalFaultReachesTerminalPath afterPoison := by
  simp [
    boundedLocalFaultReachesTerminalPath, afterPoison, poisonParity, afterEcc,
    correctEcc, afterBoot, boot, initialMachine
  ]

theorem m10_watchdog_reset_preserves_unrelated_domains :
  watchdogResetDoesNotCorruptUnrelatedDomains afterWatchdog := by
  simp [
    watchdogResetDoesNotCorruptUnrelatedDomains, afterWatchdog,
    watchdogTimeout, afterPoison, poisonParity, afterEcc, correctEcc,
    afterBoot, boot, initialMachine
  ]

theorem m10_realtime_work_has_bounded_arbitration_progress :
  realtimeWorkHasBoundedProgressPath afterRealtimeWork := by
  simp [
    realtimeWorkHasBoundedProgressPath, afterRealtimeWork, admitRealtimeWork,
    afterAdversarialInput, rejectAdversarialInput, afterQuote, quoteStub,
    afterTraceOverflow, traceOverflow, afterTelemetry, scopedTelemetryRead,
    afterWatchdog, watchdogTimeout, afterPoison, poisonParity, afterEcc,
    correctEcc, afterBoot, boot, initialMachine
  ]

theorem m10_counts_exact :
  countsExact finalMachine := by
  simp [
    countsExact, finalMachine, auditMlsDeny, afterQuote, quoteStub,
    afterRealtimeWork, admitRealtimeWork, afterAdversarialInput,
    rejectAdversarialInput, afterTraceOverflow, traceOverflow, afterTelemetry,
    scopedTelemetryRead, afterWatchdog, watchdogTimeout, afterPoison,
    poisonParity, afterEcc, correctEcc, afterBoot, boot, initialMachine
  ]

/- Packed-bit decode model for the M10 RAS witness.

Mirrors the M1..M9/M14 packed-bit machinery so the emitted lnp64_m10_ras_commit_t
and lnp64_m10_state_projection_t bit vectors can be decode-checked against this
Lean model. Every M10 field is a plain scalar/bool slice. -/

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

def rtlM10CommitPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("root_domain", 32)
  , ("fault_count", 32)
  , ("telemetry_reads", 32)
  , ("audit_records", 32)
  , ("quote_id", 32)
  , ("reset_id", 32) ]

def rtlM10StateProjectionPackedSchema : List (String × Nat) :=
  [ ("op", 8)
  , ("status", 16)
  , ("fault_count", 32)
  , ("telemetry_reads", 32)
  , ("audit_records", 32)
  , ("trace_writes", 32)
  , ("trace_capacity", 32)
  , ("boot_measured", 1)
  , ("telemetry_fdr_present", 1)
  , ("ecc_corrected", 1)
  , ("parity_poison_faulted", 1)
  , ("watchdog_timed_out", 1)
  , ("local_reset_seen", 1)
  , ("degraded_state", 1)
  , ("telemetry_scoped", 1)
  , ("telemetry_redacted", 1)
  , ("trace_overflowed", 1)
  , ("quote_measurement_bound", 1)
  , ("quote_development_marked", 1)
  , ("audit_recorded", 1)
  , ("mls_denied", 1)
  , ("debug_denied", 1)
  , ("counts_exact", 1) ]

def rtlM10CommitPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM10CommitPackedSchema

def rtlM10StateProjectionPackedLayout : List PackedFieldLayout :=
  packedSchemaLayout rtlM10StateProjectionPackedSchema

theorem rtlM10CommitPackedSchema_width :
    packedSchemaWidth rtlM10CommitPackedSchema = 216 := by
  decide

theorem rtlM10StateProjectionPackedSchema_width :
    packedSchemaWidth rtlM10StateProjectionPackedSchema = 200 := by
  decide

theorem rtlM10CommitPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM10CommitPackedSchema)
      rtlM10CommitPackedLayout = true := by
  decide

theorem rtlM10StateProjectionPackedLayout_covers_schema_width :
    packedLayoutCoversWidth
      (packedSchemaWidth rtlM10StateProjectionPackedSchema)
      rtlM10StateProjectionPackedLayout = true := by
  decide

end Lnp64.M10
