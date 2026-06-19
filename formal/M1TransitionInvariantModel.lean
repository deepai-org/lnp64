/- LNP64 M1 capability/FDR transition-invariant model.

This is the first SG-AUTH transition-invariant proof slice, not a mature
architecture-wide capability proof. It models a small capability-table boundary:
one root domain, one consumer domain, one main object, one optional created
object, one sent cap, and one minted cap.

The invariant is intentionally about authority, not a final scripted trace:
every reachable capability is tied to a known object/domain lineage, rights
never exceed the root authority unless the root mint path creates the object,
stale generations cannot authorize a live-object pull, and revoked generations
cannot revive.
-/

namespace Lnp64.M1Transition

-- The preservation lemmas below intentionally use one shared invariant
-- expansion vocabulary so each transition is checked against the same fields.
set_option linter.unusedSimpArgs false

structure Rights where
  push : Bool
  pull : Bool
  dup : Bool
  mint : Bool
deriving DecidableEq, Repr

def Rights.subset (child parent : Rights) : Prop :=
  (child.push = true -> parent.push = true) /\
  (child.pull = true -> parent.pull = true) /\
  (child.dup = true -> parent.dup = true) /\
  (child.mint = true -> parent.mint = true)

def allRights : Rights :=
  { push := true, pull := true, dup := true, mint := true }

def pullOnly : Rights :=
  { push := false, pull := true, dup := false, mint := false }

def noRights : Rights :=
  { push := false, pull := false, dup := false, mint := false }

theorem Rights.subset_refl (rights : Rights) :
    Rights.subset rights rights := by
  simp [Rights.subset]

structure Domain where
  id : Nat
  generation : Nat
  active : Bool
deriving DecidableEq, Repr

structure Capability where
  objectId : Nat
  generation : Nat
  rights : Rights
  ownerDomain : Nat
  lineageEpoch : Nat
  sealed : Bool
deriving DecidableEq, Repr

structure ObjectState where
  objectId : Nat
  generation : Nat
  ownerDomain : Nat
  created : Bool
deriving DecidableEq, Repr

inductive Location
  | runnable
  | running
  | parked
deriving DecidableEq, Repr

structure Thread where
  tid : Nat
  location : Location
  waitGeneration : Nat
deriving DecidableEq, Repr

structure State where
  rootDomain : Domain
  consumerDomain : Domain
  object : ObjectState
  createdObject : ObjectState
  rootCap : Capability
  consumerCap : Capability
  sentCap : Option Capability
  mintedCap : Option Capability
  consumer : Thread
  wakePending : Bool
  transferValid : Bool
  staleRejected : Bool
  revokedRejected : Bool
  failedNoAuthority : Bool
  fullWasExplicit : Bool
  hasRevokedGeneration : Bool
  revokedGeneration : Nat
deriving DecidableEq, Repr

inductive Op
  | capDup
  | capDupDenied
  | capSend
  | capRecv
  | capRecvEmpty
  | capRevoke
  | objectCreate
  | objectCreateDenied
  | rejectStale
  | rejectRevoked
  | await
  | awaitWakePending
  | push
  | pull
  | rejectFull
deriving DecidableEq, Repr

def rootDomain0 : Domain :=
  { id := 1, generation := 1, active := true }

def consumerDomain0 : Domain :=
  { id := 2, generation := 1, active := true }

def object0 : ObjectState :=
  { objectId := 1
    generation := 1
    ownerDomain := rootDomain0.id
    created := true }

def createdObject0 : ObjectState :=
  { objectId := 2
    generation := 1
    ownerDomain := rootDomain0.id
    created := false }

def rootCap0 : Capability :=
  { objectId := object0.objectId
    generation := object0.generation
    rights := allRights
    ownerDomain := rootDomain0.id
    lineageEpoch := 1
    sealed := false }

def consumer0 : Thread :=
  { tid := 2, location := Location.runnable, waitGeneration := object0.generation }

def reset : State :=
  { rootDomain := rootDomain0
    consumerDomain := consumerDomain0
    object := object0
    createdObject := createdObject0
    rootCap := rootCap0
    consumerCap := { rootCap0 with rights := noRights, ownerDomain := consumerDomain0.id }
    sentCap := none
    mintedCap := none
    consumer := consumer0
    wakePending := false
    transferValid := false
    staleRejected := false
    revokedRejected := false
    failedNoAuthority := false
    fullWasExplicit := false
    hasRevokedGeneration := false
    revokedGeneration := 0 }

def domainKnown (s : State) (domainId : Nat) : Prop :=
  domainId = s.rootDomain.id \/ domainId = s.consumerDomain.id

def objectKnown (s : State) (objectId : Nat) : Prop :=
  objectId = s.object.objectId \/ objectId = s.createdObject.objectId

def capLineageValid (s : State) (cap : Capability) : Prop :=
  objectKnown s cap.objectId /\
  domainKnown s cap.ownerDomain /\
  cap.lineageEpoch = s.rootCap.lineageEpoch /\
  cap.sealed = false /\
  Rights.subset cap.rights s.rootCap.rights

def capGenerationLive (s : State) (cap : Capability) : Prop :=
  (cap.objectId = s.object.objectId /\ cap.generation = s.object.generation) \/
  (s.createdObject.created = true /\
    cap.objectId = s.createdObject.objectId /\
    cap.generation = s.createdObject.generation)

def capCurrentlyAuthorizes (s : State) (cap : Capability) : Prop :=
  capLineageValid s cap /\ capGenerationLive s cap

abbrev canAuthorize (s : State) (cap : Capability) : Prop :=
  capCurrentlyAuthorizes s cap

def generationMatchesLiveObject (s : State) (cap : Capability) : Prop :=
  cap.objectId = s.object.objectId /\ cap.generation = s.object.generation

def rootCapCurrentlyAuthorizes (s : State) : Prop :=
  capCurrentlyAuthorizes s s.rootCap /\ s.rootCap.ownerDomain = s.rootDomain.id

def canRootDuplicate (s : State) : Prop :=
  rootCapCurrentlyAuthorizes s /\
  s.rootCap.rights.dup = true /\
  s.rootCap.rights.pull = true

def canRootMint (s : State) : Prop :=
  rootCapCurrentlyAuthorizes s /\ s.rootCap.rights.mint = true

def canRootPush (s : State) : Prop :=
  rootCapCurrentlyAuthorizes s /\ s.rootCap.rights.push = true

def canConsumerPullFromMainObject (s : State) (cap : Capability) : Prop :=
  capCurrentlyAuthorizes s cap /\
  generationMatchesLiveObject s cap /\
  cap.ownerDomain = s.consumerDomain.id /\
  cap.sealed = false /\
  cap.rights.pull = true

abbrev canAuthorizePullFromMainObject (s : State) (cap : Capability) : Prop :=
  canConsumerPullFromMainObject s cap

def noLostWakeupState (s : State) : Prop :=
  s.wakePending = true -> s.consumer.location ≠ Location.parked

def validTransferState (s : State) : Prop :=
  forall cap, s.sentCap = some cap -> s.transferValid = true /\ capLineageValid s cap

def mintedAuthorityState (s : State) : Prop :=
  forall cap, s.mintedCap = some cap ->
    s.createdObject.created = true /\
    cap.objectId = s.createdObject.objectId /\
    cap.generation = s.createdObject.generation /\
    cap.ownerDomain = s.rootDomain.id /\
    cap.lineageEpoch = s.rootCap.lineageEpoch /\
    cap.sealed = false /\
    Rights.subset cap.rights s.rootCap.rights

def rootAuthorityBounded (s : State) : Prop :=
  Rights.subset s.rootCap.rights allRights

def capStoredInState (s : State) (cap : Capability) : Prop :=
  cap = s.rootCap \/
  cap = s.consumerCap \/
  s.sentCap = some cap \/
  s.mintedCap = some cap

def authoritySlotsUnchanged (s t : State) : Prop :=
  t.rootCap = s.rootCap /\
  t.consumerCap = s.consumerCap /\
  t.sentCap = s.sentCap /\
  t.mintedCap = s.mintedCap

inductive FailedAuthorityOp : Op -> Prop
  | capDupDenied : FailedAuthorityOp Op.capDupDenied
  | capRecvEmpty : FailedAuthorityOp Op.capRecvEmpty
  | objectCreateDenied : FailedAuthorityOp Op.objectCreateDenied
  | rejectStale : FailedAuthorityOp Op.rejectStale
  | rejectRevoked : FailedAuthorityOp Op.rejectRevoked
  | rejectFull : FailedAuthorityOp Op.rejectFull

def revokedGenerationState (s : State) : Prop :=
  s.hasRevokedGeneration = true ->
    s.object.generation = s.revokedGeneration + 1

def invariant (s : State) : Prop :=
  s.rootCap.objectId = s.object.objectId /\
  s.rootCap.generation = s.object.generation /\
  s.rootCap.ownerDomain = s.rootDomain.id /\
  rootAuthorityBounded s /\
  s.rootCap.sealed = false /\
  s.object.ownerDomain = s.rootDomain.id /\
  s.createdObject.ownerDomain = s.rootDomain.id /\
  capLineageValid s s.rootCap /\
  capLineageValid s s.consumerCap /\
  (forall cap, s.sentCap = some cap -> capLineageValid s cap) /\
  mintedAuthorityState s /\
  validTransferState s /\
  revokedGenerationState s /\
  noLostWakeupState s /\
  s.object.objectId ≠ s.createdObject.objectId

def consumerPullCap (s : State) : Capability :=
  { s.rootCap with rights := pullOnly, ownerDomain := s.consumerDomain.id }

def mintedObjectCap (s : State) : Capability :=
  { objectId := s.createdObject.objectId
    generation := s.createdObject.generation
    rights := s.rootCap.rights
    ownerDomain := s.rootDomain.id
    lineageEpoch := s.rootCap.lineageEpoch
    sealed := false }

inductive Step : State -> Op -> State -> Prop
  | capDup (s : State) :
      canRootDuplicate s ->
      Step s Op.capDup { s with consumerCap := consumerPullCap s }
  | capDupDenied (s : State) :
      s.rootCap.rights.dup = false ->
      Step s Op.capDupDenied { s with failedNoAuthority := true }
  | capSend (s : State) :
      capCurrentlyAuthorizes s s.consumerCap ->
      Step s Op.capSend
        { s with sentCap := some s.consumerCap, transferValid := true }
  | capRecv (s : State) (cap : Capability) :
      s.sentCap = some cap ->
      capCurrentlyAuthorizes s cap ->
      Step s Op.capRecv
        { s with consumerCap := cap, sentCap := none, transferValid := true }
  | capRecvEmpty (s : State) :
      s.sentCap = none ->
      Step s Op.capRecvEmpty { s with failedNoAuthority := true }
  | capRevoke (s : State) :
      Step s Op.capRevoke
        { s with
          object := { s.object with generation := s.object.generation + 1 }
          rootCap := { s.rootCap with generation := s.object.generation + 1 }
          hasRevokedGeneration := true
          revokedGeneration := s.object.generation
          revokedRejected := true
          staleRejected := true }
  | objectCreate (s : State) :
      canRootMint s ->
      Step s Op.objectCreate
        { s with
          createdObject := { s.createdObject with created := true }
          mintedCap := some (mintedObjectCap s) }
  | objectCreateDenied (s : State) :
      s.rootCap.rights.mint = false ->
      Step s Op.objectCreateDenied { s with failedNoAuthority := true }
  | rejectStale (s : State) :
      s.consumerCap.generation ≠ s.object.generation ->
      Step s Op.rejectStale { s with staleRejected := true }
  | rejectRevoked (s : State) :
      s.hasRevokedGeneration = true ->
      Step s Op.rejectRevoked { s with revokedRejected := true }
  | await (s : State) :
      s.wakePending = false ->
      Step s Op.await
        { s with consumer := { s.consumer with
            location := Location.parked
            waitGeneration := s.object.generation } }
  | awaitWakePending (s : State) :
      s.wakePending = true ->
      Step s Op.awaitWakePending s
  | push (s : State) :
      canRootPush s ->
      Step s Op.push
        { s with
          object := { s.object with created := true }
          consumer := { s.consumer with location := Location.runnable }
          wakePending := true }
  | pull (s : State) :
      canConsumerPullFromMainObject s s.consumerCap ->
      Step s Op.pull
        { s with wakePending := false }
  | rejectFull (s : State) :
      Step s Op.rejectFull { s with fullWasExplicit := true }

inductive CommitOp
  | capDup
  | capSend
  | capRecv
  | capRevoke
  | rejectStale
  | push
  | pull
  | rejectFull
  | capDupDenied
  | objectCreate
deriving DecidableEq, Repr

inductive CommitStatus
  | ok
  | eperm
  | eagain
  | erevoked
deriving DecidableEq, Repr

structure CommitRecord where
  op : CommitOp
  objectId : Nat
  objectGeneration : Nat
  fdrGeneration : Nat
  domainId : Nat
  domainGeneration : Nat
  rights : Rights
  lineageEpoch : Nat
  sealed : Bool
  status : CommitStatus
deriving DecidableEq, Repr

structure RtlM1CommitProjection where
  op : CommitOp
  objectId : Nat
  objectGeneration : Nat
  fdrGeneration : Nat
  domainId : Nat
  domainGeneration : Nat
  rights : Rights
  lineageEpoch : Nat
  sealed : Bool
  status : CommitStatus
deriving DecidableEq, Repr

structure RtlM1StateProjection where
  objectGeneration : Nat
  createdObjectCreated : Bool
  createdObjectGeneration : Nat
  rootCap : Capability
  consumerCap : Capability
  sentCap : Option Capability
  mintedCap : Option Capability
  wakePending : Bool
  transferValid : Bool
  staleRejected : Bool
  revokedRejected : Bool
  failedNoAuthority : Bool
  fullWasExplicit : Bool
  hasRevokedGeneration : Bool
  revokedGeneration : Nat
deriving DecidableEq, Repr

def authoritySlotsProjectionUnchanged
    (pre post : RtlM1StateProjection) : Prop :=
  post.rootCap = pre.rootCap /\
  post.consumerCap = pre.consumerCap /\
  post.sentCap = pre.sentCap /\
  post.mintedCap = pre.mintedCap

structure PackedFieldLayout where
  name : String
  width : Nat
  lsb : Nat
  msb : Nat
deriving DecidableEq, Repr

-- Schema-owned packed RTL records are still checked by Python against
-- `rtl/schema/lnp64_shared_schema.json`; these Lean mirrors make the M1 model
-- name the exact bit-level projection it is willing to consume.
def rtlM1CommitPackedSchema : List (String × Nat) := [
  ("op", 8),
  ("object_id", 32),
  ("object_gen", 32),
  ("fdr_gen", 32),
  ("domain_id", 32),
  ("domain_gen", 32),
  ("rights_mask", 64),
  ("lineage_epoch", 32),
  ("sealed", 1),
  ("status", 16)
]

def rtlM1StateProjectionPackedSchema : List (String × Nat) := [
  ("op", 8),
  ("status", 16),
  ("object_gen", 32),
  ("created_object_created", 1),
  ("created_object_gen", 32),
  ("root_object_id", 32),
  ("root_generation", 32),
  ("root_domain_id", 32),
  ("root_lineage_epoch", 32),
  ("root_sealed", 1),
  ("root_rights", 64),
  ("consumer_object_id", 32),
  ("consumer_generation", 32),
  ("consumer_domain_id", 32),
  ("consumer_lineage_epoch", 32),
  ("consumer_sealed", 1),
  ("consumer_rights", 64),
  ("sent_valid", 1),
  ("sent_object_id", 32),
  ("sent_generation", 32),
  ("sent_domain_id", 32),
  ("sent_lineage_epoch", 32),
  ("sent_sealed", 1),
  ("sent_rights", 64),
  ("minted_valid", 1),
  ("minted_object_id", 32),
  ("minted_generation", 32),
  ("minted_domain_id", 32),
  ("minted_lineage_epoch", 32),
  ("minted_sealed", 1),
  ("minted_rights", 64),
  ("wake_pending", 1),
  ("transfer_valid", 1),
  ("stale_rejected", 1),
  ("revoked_rejected", 1),
  ("failed_no_authority", 1),
  ("full_was_explicit", 1),
  ("has_revoked_generation", 1),
  ("revoked_generation", 32)
]

-- The packed records are flat RTL bit layouts; these maps state the Lean
-- projection path each field is allowed to feed.
def rtlM1CommitSchemaToLeanProjection : List (String × String) := [
  ("op", "op"),
  ("object_id", "objectId"),
  ("object_gen", "objectGeneration"),
  ("fdr_gen", "fdrGeneration"),
  ("domain_id", "domainId"),
  ("domain_gen", "domainGeneration"),
  ("rights_mask", "rights"),
  ("lineage_epoch", "lineageEpoch"),
  ("sealed", "sealed"),
  ("status", "status")
]

def rtlM1StateProjectionSchemaToLeanProjection : List (String × String) := [
  ("op", "transitionTag.op"),
  ("status", "transitionTag.status"),
  ("object_gen", "objectGeneration"),
  ("created_object_created", "createdObjectCreated"),
  ("created_object_gen", "createdObjectGeneration"),
  ("root_object_id", "rootCap.objectId"),
  ("root_generation", "rootCap.generation"),
  ("root_domain_id", "rootCap.ownerDomain"),
  ("root_lineage_epoch", "rootCap.lineageEpoch"),
  ("root_sealed", "rootCap.sealed"),
  ("root_rights", "rootCap.rights"),
  ("consumer_object_id", "consumerCap.objectId"),
  ("consumer_generation", "consumerCap.generation"),
  ("consumer_domain_id", "consumerCap.ownerDomain"),
  ("consumer_lineage_epoch", "consumerCap.lineageEpoch"),
  ("consumer_sealed", "consumerCap.sealed"),
  ("consumer_rights", "consumerCap.rights"),
  ("sent_valid", "sentCap.valid"),
  ("sent_object_id", "sentCap.objectId"),
  ("sent_generation", "sentCap.generation"),
  ("sent_domain_id", "sentCap.ownerDomain"),
  ("sent_lineage_epoch", "sentCap.lineageEpoch"),
  ("sent_sealed", "sentCap.sealed"),
  ("sent_rights", "sentCap.rights"),
  ("minted_valid", "mintedCap.valid"),
  ("minted_object_id", "mintedCap.objectId"),
  ("minted_generation", "mintedCap.generation"),
  ("minted_domain_id", "mintedCap.ownerDomain"),
  ("minted_lineage_epoch", "mintedCap.lineageEpoch"),
  ("minted_sealed", "mintedCap.sealed"),
  ("minted_rights", "mintedCap.rights"),
  ("wake_pending", "wakePending"),
  ("transfer_valid", "transferValid"),
  ("stale_rejected", "staleRejected"),
  ("revoked_rejected", "revokedRejected"),
  ("failed_no_authority", "failedNoAuthority"),
  ("full_was_explicit", "fullWasExplicit"),
  ("has_revoked_generation", "hasRevokedGeneration"),
  ("revoked_generation", "revokedGeneration")
]

def packedSchemaWidth (schema : List (String × Nat)) : Nat :=
  schema.foldl (fun total field => total + field.2) 0

def packedSchemaFieldNames (schema : List (String × Nat)) : List String :=
  schema.map Prod.fst

def packedProjectionSchemaFieldNames (schema : List (String × String)) : List String :=
  schema.map Prod.fst

def packedProjectionLeanPaths (schema : List (String × String)) : List String :=
  schema.map Prod.snd

def packedSchemaLayoutFrom : Nat -> List (String × Nat) -> List PackedFieldLayout
  | _cursor, [] => []
  | cursor, field :: rest =>
      let lsb := cursor - field.2
      { name := field.1, width := field.2, lsb := lsb, msb := cursor - 1 } ::
        packedSchemaLayoutFrom lsb rest

def packedSchemaLayout (schema : List (String × Nat)) : List PackedFieldLayout :=
  packedSchemaLayoutFrom (packedSchemaWidth schema) schema

theorem rtlM1CommitPackedSchema_width :
    packedSchemaWidth rtlM1CommitPackedSchema = 281 := by
  rfl

theorem rtlM1StateProjectionPackedSchema_width :
    packedSchemaWidth rtlM1StateProjectionPackedSchema = 902 := by
  rfl

theorem rtlM1CommitSchemaToLeanProjection_covers_schema :
    packedProjectionSchemaFieldNames rtlM1CommitSchemaToLeanProjection =
      packedSchemaFieldNames rtlM1CommitPackedSchema := by
  rfl

theorem rtlM1CommitSchemaToLeanProjection_targets_commit_projection :
    packedProjectionLeanPaths rtlM1CommitSchemaToLeanProjection =
      [
        "op",
        "objectId",
        "objectGeneration",
        "fdrGeneration",
        "domainId",
        "domainGeneration",
        "rights",
        "lineageEpoch",
        "sealed",
        "status"
      ] := by
  rfl

theorem rtlM1StateProjectionSchemaToLeanProjection_covers_schema :
    packedProjectionSchemaFieldNames rtlM1StateProjectionSchemaToLeanProjection =
      packedSchemaFieldNames rtlM1StateProjectionPackedSchema := by
  rfl

theorem rtlM1StateProjectionSchemaToLeanProjection_targets_state_projection :
    packedProjectionLeanPaths rtlM1StateProjectionSchemaToLeanProjection =
      [
        "transitionTag.op",
        "transitionTag.status",
        "objectGeneration",
        "createdObjectCreated",
        "createdObjectGeneration",
        "rootCap.objectId",
        "rootCap.generation",
        "rootCap.ownerDomain",
        "rootCap.lineageEpoch",
        "rootCap.sealed",
        "rootCap.rights",
        "consumerCap.objectId",
        "consumerCap.generation",
        "consumerCap.ownerDomain",
        "consumerCap.lineageEpoch",
        "consumerCap.sealed",
        "consumerCap.rights",
        "sentCap.valid",
        "sentCap.objectId",
        "sentCap.generation",
        "sentCap.ownerDomain",
        "sentCap.lineageEpoch",
        "sentCap.sealed",
        "sentCap.rights",
        "mintedCap.valid",
        "mintedCap.objectId",
        "mintedCap.generation",
        "mintedCap.ownerDomain",
        "mintedCap.lineageEpoch",
        "mintedCap.sealed",
        "mintedCap.rights",
        "wakePending",
        "transferValid",
        "staleRejected",
        "revokedRejected",
        "failedNoAuthority",
        "fullWasExplicit",
        "hasRevokedGeneration",
        "revokedGeneration"
      ] := by
  rfl

def rtlM1CommitPackedLayout : List PackedFieldLayout := [
  { name := "op", width := 8, lsb := 273, msb := 280 },
  { name := "object_id", width := 32, lsb := 241, msb := 272 },
  { name := "object_gen", width := 32, lsb := 209, msb := 240 },
  { name := "fdr_gen", width := 32, lsb := 177, msb := 208 },
  { name := "domain_id", width := 32, lsb := 145, msb := 176 },
  { name := "domain_gen", width := 32, lsb := 113, msb := 144 },
  { name := "rights_mask", width := 64, lsb := 49, msb := 112 },
  { name := "lineage_epoch", width := 32, lsb := 17, msb := 48 },
  { name := "sealed", width := 1, lsb := 16, msb := 16 },
  { name := "status", width := 16, lsb := 0, msb := 15 }
]

def rtlM1StateProjectionPackedLayout : List PackedFieldLayout := [
  { name := "op", width := 8, lsb := 894, msb := 901 },
  { name := "status", width := 16, lsb := 878, msb := 893 },
  { name := "object_gen", width := 32, lsb := 846, msb := 877 },
  { name := "created_object_created", width := 1, lsb := 845, msb := 845 },
  { name := "created_object_gen", width := 32, lsb := 813, msb := 844 },
  { name := "root_object_id", width := 32, lsb := 781, msb := 812 },
  { name := "root_generation", width := 32, lsb := 749, msb := 780 },
  { name := "root_domain_id", width := 32, lsb := 717, msb := 748 },
  { name := "root_lineage_epoch", width := 32, lsb := 685, msb := 716 },
  { name := "root_sealed", width := 1, lsb := 684, msb := 684 },
  { name := "root_rights", width := 64, lsb := 620, msb := 683 },
  { name := "consumer_object_id", width := 32, lsb := 588, msb := 619 },
  { name := "consumer_generation", width := 32, lsb := 556, msb := 587 },
  { name := "consumer_domain_id", width := 32, lsb := 524, msb := 555 },
  { name := "consumer_lineage_epoch", width := 32, lsb := 492, msb := 523 },
  { name := "consumer_sealed", width := 1, lsb := 491, msb := 491 },
  { name := "consumer_rights", width := 64, lsb := 427, msb := 490 },
  { name := "sent_valid", width := 1, lsb := 426, msb := 426 },
  { name := "sent_object_id", width := 32, lsb := 394, msb := 425 },
  { name := "sent_generation", width := 32, lsb := 362, msb := 393 },
  { name := "sent_domain_id", width := 32, lsb := 330, msb := 361 },
  { name := "sent_lineage_epoch", width := 32, lsb := 298, msb := 329 },
  { name := "sent_sealed", width := 1, lsb := 297, msb := 297 },
  { name := "sent_rights", width := 64, lsb := 233, msb := 296 },
  { name := "minted_valid", width := 1, lsb := 232, msb := 232 },
  { name := "minted_object_id", width := 32, lsb := 200, msb := 231 },
  { name := "minted_generation", width := 32, lsb := 168, msb := 199 },
  { name := "minted_domain_id", width := 32, lsb := 136, msb := 167 },
  { name := "minted_lineage_epoch", width := 32, lsb := 104, msb := 135 },
  { name := "minted_sealed", width := 1, lsb := 103, msb := 103 },
  { name := "minted_rights", width := 64, lsb := 39, msb := 102 },
  { name := "wake_pending", width := 1, lsb := 38, msb := 38 },
  { name := "transfer_valid", width := 1, lsb := 37, msb := 37 },
  { name := "stale_rejected", width := 1, lsb := 36, msb := 36 },
  { name := "revoked_rejected", width := 1, lsb := 35, msb := 35 },
  { name := "failed_no_authority", width := 1, lsb := 34, msb := 34 },
  { name := "full_was_explicit", width := 1, lsb := 33, msb := 33 },
  { name := "has_revoked_generation", width := 1, lsb := 32, msb := 32 },
  { name := "revoked_generation", width := 32, lsb := 0, msb := 31 }
]

theorem rtlM1CommitPackedLayout_from_schema :
    packedSchemaLayout rtlM1CommitPackedSchema =
      rtlM1CommitPackedLayout := by
  rfl

theorem rtlM1StateProjectionPackedLayout_from_schema :
    packedSchemaLayout rtlM1StateProjectionPackedSchema =
      rtlM1StateProjectionPackedLayout := by
  rfl

def commitOpToStepOp : CommitOp -> Op
  | CommitOp.capDup => Op.capDup
  | CommitOp.capSend => Op.capSend
  | CommitOp.capRecv => Op.capRecv
  | CommitOp.capRevoke => Op.capRevoke
  | CommitOp.rejectStale => Op.rejectStale
  | CommitOp.push => Op.push
  | CommitOp.pull => Op.pull
  | CommitOp.rejectFull => Op.rejectFull
  | CommitOp.capDupDenied => Op.capDupDenied
  | CommitOp.objectCreate => Op.objectCreate

def expectedCommitStatus : CommitOp -> CommitStatus
  | CommitOp.rejectStale => CommitStatus.erevoked
  | CommitOp.rejectFull => CommitStatus.eagain
  | CommitOp.capDupDenied => CommitStatus.eperm
  | _ => CommitStatus.ok

def commitFromCap
    (op : CommitOp)
    (cap : Capability)
    (objectGeneration : Nat)
    (status : CommitStatus) : CommitRecord :=
  { op := op
    objectId := cap.objectId
    objectGeneration := objectGeneration
    fdrGeneration := cap.generation
    domainId := cap.ownerDomain
    domainGeneration := 1
    rights := cap.rights
    lineageEpoch := cap.lineageEpoch
    sealed := cap.sealed
    status := status }

def commitProjectionToRecord (projection : RtlM1CommitProjection) : CommitRecord :=
  { op := projection.op
    objectId := projection.objectId
    objectGeneration := projection.objectGeneration
    fdrGeneration := projection.fdrGeneration
    domainId := projection.domainId
    domainGeneration := projection.domainGeneration
    rights := projection.rights
    lineageEpoch := projection.lineageEpoch
    sealed := projection.sealed
    status := projection.status }

def capabilityFromCommitProjection (projection : RtlM1CommitProjection) : Capability :=
  { objectId := projection.objectId
    generation := projection.fdrGeneration
    rights := projection.rights
    ownerDomain := projection.domainId
    lineageEpoch := projection.lineageEpoch
    sealed := projection.sealed }

def capabilityFromCommitRecord (commit : CommitRecord) : Capability :=
  { objectId := commit.objectId
    generation := commit.fdrGeneration
    rights := commit.rights
    ownerDomain := commit.domainId
    lineageEpoch := commit.lineageEpoch
    sealed := commit.sealed }

def commitMatchesRtlProjection
    (commit : CommitRecord)
    (projection : RtlM1CommitProjection) : Prop :=
  commit = commitProjectionToRecord projection

def stateProjectionOf (s : State) : RtlM1StateProjection :=
  { objectGeneration := s.object.generation
    createdObjectCreated := s.createdObject.created
    createdObjectGeneration := s.createdObject.generation
    rootCap := s.rootCap
    consumerCap := s.consumerCap
    sentCap := s.sentCap
    mintedCap := s.mintedCap
    wakePending := s.wakePending
    transferValid := s.transferValid
    staleRejected := s.staleRejected
    revokedRejected := s.revokedRejected
    failedNoAuthority := s.failedNoAuthority
    fullWasExplicit := s.fullWasExplicit
    hasRevokedGeneration := s.hasRevokedGeneration
    revokedGeneration := s.revokedGeneration }

def stateMatchesRtlProjection
    (s : State)
    (projection : RtlM1StateProjection) : Prop :=
  projection = stateProjectionOf s

def rtlM1ProjectionFaithful
    (s : State)
    (projection : RtlM1StateProjection) : Prop :=
  projection.objectGeneration = s.object.generation /\
  projection.createdObjectCreated = s.createdObject.created /\
  projection.createdObjectGeneration = s.createdObject.generation /\
  projection.rootCap = s.rootCap /\
  projection.consumerCap = s.consumerCap /\
  projection.sentCap = s.sentCap /\
  projection.mintedCap = s.mintedCap /\
  projection.wakePending = s.wakePending /\
  projection.transferValid = s.transferValid /\
  projection.staleRejected = s.staleRejected /\
  projection.revokedRejected = s.revokedRejected /\
  projection.failedNoAuthority = s.failedNoAuthority /\
  projection.fullWasExplicit = s.fullWasExplicit /\
  projection.hasRevokedGeneration = s.hasRevokedGeneration /\
  projection.revokedGeneration = s.revokedGeneration

theorem stateMatchesRtlProjection_projection_faithful
    {s : State}
    {projection : RtlM1StateProjection} :
    stateMatchesRtlProjection s projection ->
    rtlM1ProjectionFaithful s projection := by
  intro hProjection
  rw [stateMatchesRtlProjection] at hProjection
  rw [hProjection]
  simp [rtlM1ProjectionFaithful, stateProjectionOf]

def capDupCommit (s : State) : CommitRecord :=
  commitFromCap CommitOp.capDup (consumerPullCap s) s.object.generation CommitStatus.ok

def capSendCommit (s : State) : CommitRecord :=
  commitFromCap CommitOp.capSend s.consumerCap s.object.generation CommitStatus.ok

def capRecvCommit (s : State) (cap : Capability) : CommitRecord :=
  commitFromCap CommitOp.capRecv cap s.object.generation CommitStatus.ok

def capRevokeCommit (s : State) : CommitRecord :=
  commitFromCap
    CommitOp.capRevoke
    { s.rootCap with generation := s.object.generation }
    (s.object.generation + 1)
    CommitStatus.ok

def rejectStaleCommit (s : State) : CommitRecord :=
  commitFromCap CommitOp.rejectStale s.consumerCap s.object.generation CommitStatus.erevoked

def pushCommit (s : State) : CommitRecord :=
  commitFromCap CommitOp.push s.rootCap s.object.generation CommitStatus.ok

def pullCommit (s : State) : CommitRecord :=
  commitFromCap CommitOp.pull s.consumerCap s.object.generation CommitStatus.ok

def rejectFullCommit (s : State) : CommitRecord :=
  commitFromCap CommitOp.rejectFull s.rootCap s.object.generation CommitStatus.eagain

def capDupDeniedCommit (s : State) : CommitRecord :=
  commitFromCap CommitOp.capDupDenied s.rootCap s.object.generation CommitStatus.eperm

def objectCreateCommit (s : State) : CommitRecord :=
  commitFromCap CommitOp.objectCreate (mintedObjectCap s) s.createdObject.generation CommitStatus.ok

inductive TypedCommitTransition : State -> CommitRecord -> State -> Prop
  | capDup (s : State) :
      canRootDuplicate s ->
      TypedCommitTransition s (capDupCommit s) { s with consumerCap := consumerPullCap s }
  | capSend (s : State) :
      capCurrentlyAuthorizes s s.consumerCap ->
      TypedCommitTransition s (capSendCommit s)
        { s with sentCap := some s.consumerCap, transferValid := true }
  | capRecv (s : State) (cap : Capability) :
      s.sentCap = some cap ->
      capCurrentlyAuthorizes s cap ->
      TypedCommitTransition s (capRecvCommit s cap)
        { s with consumerCap := cap, sentCap := none, transferValid := true }
  | capRevoke (s : State) :
      TypedCommitTransition s (capRevokeCommit s)
        { s with
          object := { s.object with generation := s.object.generation + 1 }
          rootCap := { s.rootCap with generation := s.object.generation + 1 }
          hasRevokedGeneration := true
          revokedGeneration := s.object.generation
          revokedRejected := true
          staleRejected := true }
  | rejectStale (s : State) :
      s.consumerCap.generation ≠ s.object.generation ->
      TypedCommitTransition s (rejectStaleCommit s) { s with staleRejected := true }
  | push (s : State) :
      canRootPush s ->
      TypedCommitTransition s (pushCommit s)
        { s with
          object := { s.object with created := true }
          consumer := { s.consumer with location := Location.runnable }
          wakePending := true }
  | pull (s : State) :
      canConsumerPullFromMainObject s s.consumerCap ->
      TypedCommitTransition s (pullCommit s) { s with wakePending := false }
  | rejectFull (s : State) :
      TypedCommitTransition s (rejectFullCommit s) { s with fullWasExplicit := true }
  | capDupDenied (s : State) :
      s.rootCap.rights.dup = false ->
      TypedCommitTransition s (capDupDeniedCommit s) { s with failedNoAuthority := true }
  | objectCreate (s : State) :
      canRootMint s ->
      TypedCommitTransition s (objectCreateCommit s)
        { s with
          createdObject := { s.createdObject with created := true }
          mintedCap := some (mintedObjectCap s) }

def RtlM1RefinementStep
    (pre : RtlM1StateProjection)
    (commitProjection : RtlM1CommitProjection)
    (post : RtlM1StateProjection) : Prop :=
  exists s t commit,
    stateMatchesRtlProjection s pre /\
    commitMatchesRtlProjection commit commitProjection /\
    TypedCommitTransition s commit t /\
    stateMatchesRtlProjection t post

inductive Reachable : State -> Prop
  | reset : Reachable reset
  | step {s t : State} {op : Op} :
      Reachable s -> Step s op t -> Reachable t

theorem pullOnly_subset_all :
    Rights.subset pullOnly allRights := by
  simp [Rights.subset, pullOnly, allRights]

theorem noRights_subset_all :
    Rights.subset noRights allRights := by
  simp [Rights.subset, noRights, allRights]

theorem invariant_reset :
    invariant reset := by
  simp [
    invariant, reset, rootDomain0, consumerDomain0, object0, createdObject0,
    rootCap0, consumer0, capLineageValid, objectKnown, domainKnown,
    mintedAuthorityState, validTransferState, rootAuthorityBounded,
    revokedGenerationState, noLostWakeupState, Rights.subset, allRights,
    noRights
  ]

def rootWithoutDupAuthority : Rights :=
  { allRights with dup := false }

def deniedRootState : State :=
  { rootDomain := rootDomain0
    consumerDomain := consumerDomain0
    object := object0
    createdObject := createdObject0
    rootCap := { rootCap0 with rights := rootWithoutDupAuthority }
    consumerCap := { rootCap0 with rights := noRights, ownerDomain := consumerDomain0.id }
    sentCap := none
    mintedCap := none
    consumer := consumer0
    wakePending := false
    transferValid := false
    staleRejected := false
    revokedRejected := false
    failedNoAuthority := false
    fullWasExplicit := false
    hasRevokedGeneration := false
    revokedGeneration := 0 }

theorem denied_root_state_invariant :
    invariant deniedRootState := by
  repeat constructor <;> simp [
    deniedRootState, rootWithoutDupAuthority, rootDomain0, consumerDomain0,
    object0, createdObject0, rootCap0, consumer0, capLineageValid,
    objectKnown, domainKnown, mintedAuthorityState, validTransferState,
    rootAuthorityBounded, revokedGenerationState, noLostWakeupState,
    Rights.subset, allRights, noRights
  ]

theorem invariant_consumer_cap_lineage_valid {s : State} :
    invariant s -> capLineageValid s s.consumerCap := by
  intro hInv
  rcases hInv with
    ⟨_, _, _, _, _, _, _, _, hConsumerCapLineage, _⟩
  exact hConsumerCapLineage

theorem invariant_root_cap_lineage_valid {s : State} :
    invariant s -> capLineageValid s s.rootCap := by
  intro hInv
  rcases hInv with
    ⟨_, _, _, _, _, _, _, hRootCapLineage, _⟩
  exact hRootCapLineage

theorem invariant_sent_cap_lineage_valid {s : State} :
    invariant s ->
    forall cap, s.sentCap = some cap -> capLineageValid s cap := by
  intro hInv
  rcases hInv with
    ⟨_, _, _, _, _, _, _, _, _, hSentCapLineage, _⟩
  exact hSentCapLineage

theorem invariant_minted_authority_state {s : State} :
    invariant s -> mintedAuthorityState s := by
  intro hInv
  rcases hInv with
    ⟨_, _, _, _, _, _, _, _, _, _, hMintedAuthority, _⟩
  exact hMintedAuthority

theorem invariant_valid_transfer_state {s : State} :
    invariant s -> validTransferState s := by
  intro hInv
  rcases hInv with
    ⟨_, _, _, _, _, _, _, _, _, _, _, hValidTransfer, _⟩
  exact hValidTransfer

theorem invariant_revoked_generation_state {s : State} :
    invariant s -> revokedGenerationState s := by
  intro hInv
  rcases hInv with
    ⟨_, _, _, _, _, _, _, _, _, _, _, _, hRevokedGeneration, _⟩
  exact hRevokedGeneration

theorem invariant_no_lost_wakeup_state {s : State} :
    invariant s -> noLostWakeupState s := by
  intro hInv
  rcases hInv with
    ⟨_, _, _, _, _, _, _, _, _, _, _, _, _, hNoLostWakeup, _⟩
  exact hNoLostWakeup

theorem invariant_object_ids_distinct {s : State} :
    invariant s -> s.object.objectId ≠ s.createdObject.objectId := by
  intro hInv
  rcases hInv with
    ⟨_, _, _, _, _, _, _, _, _, _, _, _, _, _, hDistinct⟩
  exact hDistinct

theorem capLineageValid_rights_subset {s : State} {cap : Capability} :
    capLineageValid s cap -> Rights.subset cap.rights s.rootCap.rights := by
  intro hLineage
  rcases hLineage with
    ⟨_, _, _, _, hRightsSubset⟩
  exact hRightsSubset

theorem preserve_cap_dup {s : State} :
    invariant s ->
    canRootDuplicate s ->
    invariant { s with consumerCap := consumerPullCap s } := by
  intro hInv hRootDup
  simp_all [
    invariant, consumerPullCap, mintedObjectCap, capLineageValid, capGenerationLive,
    objectKnown, domainKnown, mintedAuthorityState, validTransferState, rootAuthorityBounded,
    revokedGenerationState, noLostWakeupState, Rights.subset,
    capCurrentlyAuthorizes, rootCapCurrentlyAuthorizes, canRootDuplicate,
    pullOnly, allRights
  ]

theorem preserve_cap_dup_denied {s : State} :
    invariant s ->
    s.rootCap.rights.dup = false ->
    invariant { s with failedNoAuthority := true } := by
  intro hInv hDup
  simp_all [
    invariant, consumerPullCap, mintedObjectCap, capLineageValid, capGenerationLive,
    objectKnown, domainKnown, mintedAuthorityState, validTransferState, rootAuthorityBounded,
    revokedGenerationState, noLostWakeupState, Rights.subset, pullOnly,
    allRights
  ]

theorem denied_root_cap_dup_preserves_invariant :
    invariant { deniedRootState with failedNoAuthority := true } := by
  exact preserve_cap_dup_denied denied_root_state_invariant rfl

theorem preserve_cap_send {s : State} :
    invariant s ->
    capCurrentlyAuthorizes s s.consumerCap ->
    invariant { s with sentCap := some s.consumerCap, transferValid := true } := by
  intro hInv hCap
  simp_all [
    invariant, consumerPullCap, mintedObjectCap, capLineageValid, capGenerationLive,
    objectKnown, domainKnown, mintedAuthorityState, validTransferState, rootAuthorityBounded,
    revokedGenerationState, noLostWakeupState, Rights.subset, pullOnly,
    allRights
  ]

theorem preserve_cap_recv {s : State} {cap : Capability} :
    invariant s ->
    s.sentCap = some cap ->
    capCurrentlyAuthorizes s cap ->
    invariant { s with consumerCap := cap, sentCap := none, transferValid := true } := by
  intro hInv hSent hCap
  simp_all [
    invariant, consumerPullCap, mintedObjectCap, capLineageValid, capGenerationLive,
    objectKnown, domainKnown, mintedAuthorityState, validTransferState, rootAuthorityBounded,
    revokedGenerationState, noLostWakeupState, Rights.subset, pullOnly,
    allRights
  ]

theorem preserve_cap_recv_empty {s : State} :
    invariant s ->
    s.sentCap = none ->
    invariant { s with failedNoAuthority := true } := by
  intro hInv hEmpty
  simp_all [
    invariant, consumerPullCap, mintedObjectCap, capLineageValid, capGenerationLive,
    objectKnown, domainKnown, mintedAuthorityState, validTransferState, rootAuthorityBounded,
    revokedGenerationState, noLostWakeupState, Rights.subset, pullOnly,
    allRights
  ]

theorem preserve_cap_revoke {s : State} :
    invariant s ->
    invariant
      { s with
        object := { s.object with generation := s.object.generation + 1 }
        rootCap := { s.rootCap with generation := s.object.generation + 1 }
        hasRevokedGeneration := true
        revokedGeneration := s.object.generation
        revokedRejected := true
        staleRejected := true } := by
  intro hInv
  simp_all [
    invariant, consumerPullCap, mintedObjectCap, capLineageValid, capGenerationLive,
    objectKnown, domainKnown, mintedAuthorityState, validTransferState, rootAuthorityBounded,
    revokedGenerationState, noLostWakeupState, Rights.subset, pullOnly,
    allRights
  ]

theorem preserve_object_create {s : State} :
    invariant s ->
    canRootMint s ->
    invariant
      { s with
        createdObject := { s.createdObject with created := true }
        mintedCap := some (mintedObjectCap s) } := by
  intro hInv hRootMint
  simp_all [
    invariant, consumerPullCap, mintedObjectCap, capLineageValid, capGenerationLive,
    objectKnown, domainKnown, mintedAuthorityState, validTransferState, rootAuthorityBounded,
    revokedGenerationState, noLostWakeupState, Rights.subset,
    capCurrentlyAuthorizes, rootCapCurrentlyAuthorizes, canRootMint,
    pullOnly, allRights
  ]

theorem preserve_object_create_denied {s : State} :
    invariant s ->
    s.rootCap.rights.mint = false ->
    invariant { s with failedNoAuthority := true } := by
  intro hInv hMint
  simp_all [
    invariant, consumerPullCap, mintedObjectCap, capLineageValid, capGenerationLive,
    objectKnown, domainKnown, mintedAuthorityState, validTransferState, rootAuthorityBounded,
    revokedGenerationState, noLostWakeupState, Rights.subset, pullOnly,
    allRights
  ]

theorem preserve_reject_stale {s : State} :
    invariant s ->
    s.consumerCap.generation ≠ s.object.generation ->
    invariant { s with staleRejected := true } := by
  intro hInv hStale
  simp_all [
    invariant, consumerPullCap, mintedObjectCap, capLineageValid, capGenerationLive,
    objectKnown, domainKnown, mintedAuthorityState, validTransferState, rootAuthorityBounded,
    revokedGenerationState, noLostWakeupState, Rights.subset, pullOnly,
    allRights
  ]

theorem preserve_reject_revoked {s : State} :
    invariant s ->
    s.hasRevokedGeneration = true ->
    invariant { s with revokedRejected := true } := by
  intro hInv hRevoked
  simp_all [
    invariant, consumerPullCap, mintedObjectCap, capLineageValid, capGenerationLive,
    objectKnown, domainKnown, mintedAuthorityState, validTransferState, rootAuthorityBounded,
    revokedGenerationState, noLostWakeupState, Rights.subset, pullOnly,
    allRights
  ]

theorem preserve_await {s : State} :
    invariant s ->
    s.wakePending = false ->
    invariant
      { s with consumer := { s.consumer with
          location := Location.parked
          waitGeneration := s.object.generation } } := by
  intro hInv hWake
  simp_all [
    invariant, consumerPullCap, mintedObjectCap, capLineageValid, capGenerationLive,
    objectKnown, domainKnown, mintedAuthorityState, validTransferState, rootAuthorityBounded,
    revokedGenerationState, noLostWakeupState, Rights.subset, pullOnly,
    allRights
  ]

theorem preserve_await_wake_pending {s : State} :
    invariant s ->
    s.wakePending = true ->
    invariant s := by
  intro hInv hWake
  exact hInv

theorem preserve_push {s : State} :
    invariant s ->
    canRootPush s ->
    invariant
      { s with
        object := { s.object with created := true }
        consumer := { s.consumer with location := Location.runnable }
        wakePending := true } := by
  intro hInv hRootPush
  simp_all [
    invariant, consumerPullCap, mintedObjectCap, capLineageValid, capGenerationLive,
    objectKnown, domainKnown, mintedAuthorityState, validTransferState, rootAuthorityBounded,
    revokedGenerationState, noLostWakeupState, Rights.subset,
    capCurrentlyAuthorizes, rootCapCurrentlyAuthorizes, canRootPush,
    pullOnly, allRights
  ]

theorem preserve_pull {s : State} :
    invariant s ->
    canConsumerPullFromMainObject s s.consumerCap ->
    invariant { s with wakePending := false } := by
  intro hInv hConsumerPull
  simp_all [
    invariant, consumerPullCap, mintedObjectCap, capLineageValid, capGenerationLive,
    objectKnown, domainKnown, mintedAuthorityState, validTransferState, rootAuthorityBounded,
    revokedGenerationState, noLostWakeupState, Rights.subset,
    capCurrentlyAuthorizes, canConsumerPullFromMainObject,
    generationMatchesLiveObject, pullOnly, allRights
  ]

theorem preserve_reject_full {s : State} :
    invariant s ->
    invariant { s with fullWasExplicit := true } := by
  intro hInv
  simp_all [
    invariant, consumerPullCap, mintedObjectCap, capLineageValid, capGenerationLive,
    objectKnown, domainKnown, mintedAuthorityState, validTransferState, rootAuthorityBounded,
    revokedGenerationState, noLostWakeupState, Rights.subset, pullOnly,
    allRights
  ]

theorem invariant_step {s t : State} {op : Op} :
    invariant s -> Step s op t -> invariant t := by
  intro hInv hStep
  cases hStep with
  | capDup hRootDup => exact preserve_cap_dup hInv hRootDup
  | capDupDenied hDup => exact preserve_cap_dup_denied hInv hDup
  | capSend hCap => exact preserve_cap_send hInv hCap
  | capRecv cap hSent hCap => exact preserve_cap_recv hInv hSent hCap
  | capRecvEmpty hEmpty => exact preserve_cap_recv_empty hInv hEmpty
  | capRevoke => exact preserve_cap_revoke hInv
  | objectCreate hRootMint => exact preserve_object_create hInv hRootMint
  | objectCreateDenied hMint => exact preserve_object_create_denied hInv hMint
  | rejectStale hStale => exact preserve_reject_stale hInv hStale
  | rejectRevoked hRevoked => exact preserve_reject_revoked hInv hRevoked
  | await hWake => exact preserve_await hInv hWake
  | awaitWakePending hWake => exact preserve_await_wake_pending hInv hWake
  | push hRootPush => exact preserve_push hInv hRootPush
  | pull hConsumerPull => exact preserve_pull hInv hConsumerPull
  | rejectFull => exact preserve_reject_full hInv

theorem typed_commit_transition_refines_step
    {s t : State} {commit : CommitRecord} :
    TypedCommitTransition s commit t ->
    Step s (commitOpToStepOp commit.op) t := by
  intro hCommit
  cases hCommit with
  | capDup hRootDup =>
      exact Step.capDup _ hRootDup
  | capSend hCap =>
      exact Step.capSend _ hCap
  | capRecv cap hSent hCap =>
      exact Step.capRecv _ cap hSent hCap
  | capRevoke =>
      exact Step.capRevoke _
  | rejectStale hStale =>
      exact Step.rejectStale _ hStale
  | push hRootPush =>
      exact Step.push _ hRootPush
  | pull hConsumerPull =>
      exact Step.pull _ hConsumerPull
  | rejectFull =>
      exact Step.rejectFull _
  | capDupDenied hDup =>
      exact Step.capDupDenied _ hDup
  | objectCreate hRootMint =>
      exact Step.objectCreate _ hRootMint

theorem typed_commit_transition_preserves_invariant
    {s t : State} {commit : CommitRecord} :
    invariant s ->
    TypedCommitTransition s commit t ->
    invariant t := by
  intro hInv hCommit
  exact invariant_step hInv (typed_commit_transition_refines_step hCommit)

theorem typed_commit_transition_status_matches_op
    {s t : State} {commit : CommitRecord} :
    TypedCommitTransition s commit t ->
    commit.status = expectedCommitStatus commit.op := by
  intro hCommit
  cases hCommit with
  | capDup hRootDup =>
      simp [capDupCommit, commitFromCap, expectedCommitStatus]
  | capSend hCap =>
      simp [capSendCommit, commitFromCap, expectedCommitStatus]
  | capRecv cap hSent hCap =>
      simp [capRecvCommit, commitFromCap, expectedCommitStatus]
  | capRevoke =>
      simp [capRevokeCommit, commitFromCap, expectedCommitStatus]
  | rejectStale hStale =>
      simp [rejectStaleCommit, commitFromCap, expectedCommitStatus]
  | push hRootPush =>
      simp [pushCommit, commitFromCap, expectedCommitStatus]
  | pull hConsumerPull =>
      simp [pullCommit, commitFromCap, expectedCommitStatus]
  | rejectFull =>
      simp [rejectFullCommit, commitFromCap, expectedCommitStatus]
  | capDupDenied hDup =>
      simp [capDupDeniedCommit, commitFromCap, expectedCommitStatus]
  | objectCreate hRootMint =>
      simp [objectCreateCommit, commitFromCap, expectedCommitStatus]

theorem rtl_m1_refinement_step_refines_lean_step
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    exists s t commit,
      stateMatchesRtlProjection s pre /\
      commitMatchesRtlProjection commit commitProjection /\
      Step s (commitOpToStepOp commit.op) t /\
      stateMatchesRtlProjection t post := by
  intro hRefine
  rcases hRefine with ⟨s, t, commit, hPre, hCommitProjection, hCommit, hPost⟩
  exact ⟨s, t, commit, hPre, hCommitProjection,
    typed_commit_transition_refines_step hCommit, hPost⟩

theorem rtl_m1_refinement_step_refines_commit_projection_op
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    exists s t,
      stateMatchesRtlProjection s pre /\
      Step s (commitOpToStepOp commitProjection.op) t /\
      stateMatchesRtlProjection t post := by
  intro hRefine
  rcases hRefine with ⟨s, t, commit, hPre, hCommitProjection, hCommit, hPost⟩
  refine ⟨s, t, hPre, ?_, hPost⟩
  rw [commitMatchesRtlProjection, commitProjectionToRecord] at hCommitProjection
  subst commit
  simpa using typed_commit_transition_refines_step hCommit

theorem rtl_m1_refinement_step_status_matches_op
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    commitProjection.status = expectedCommitStatus commitProjection.op := by
  intro hRefine
  rcases hRefine with ⟨_s, _t, commit, _hPre, hCommitProjection, hCommit, _hPost⟩
  have hStatus := typed_commit_transition_status_matches_op hCommit
  rw [commitMatchesRtlProjection, commitProjectionToRecord] at hCommitProjection
  subst commit
  simpa [commitProjectionToRecord] using hStatus

theorem rtl_m1_refinement_step_projection_faithful
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
      exists s t commit,
        rtlM1ProjectionFaithful s pre /\
        commitMatchesRtlProjection commit commitProjection /\
        TypedCommitTransition s commit t /\
        rtlM1ProjectionFaithful t post := by
  intro hRefine
  rcases hRefine with ⟨s, t, commit, hPre, hCommitProjection, hCommit, hPost⟩
  exact ⟨
    s,
    t,
    commit,
    stateMatchesRtlProjection_projection_faithful hPre,
    hCommitProjection,
    hCommit,
    stateMatchesRtlProjection_projection_faithful hPost
  ⟩

theorem rtl_m1_refinement_step_preserves_sg_auth_invariant
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    (forall s, stateMatchesRtlProjection s pre -> invariant s) ->
    exists t, stateMatchesRtlProjection t post /\ invariant t := by
  intro hRefine hPreInvariant
  rcases hRefine with ⟨s, t, commit, hPre, _hCommitProjection, hCommit, hPost⟩
  exact ⟨t, hPost, typed_commit_transition_preserves_invariant (hPreInvariant s hPre) hCommit⟩

theorem step_minted_cap_created_only_by_authorized_object_create
    {s t : State} {op : Op} {cap : Capability} :
    Step s op t ->
    s.mintedCap = none ->
    t.mintedCap = some cap ->
    op = Op.objectCreate /\ canRootMint s := by
  intro hStep hNoMinted hMinted
  cases hStep with
  | capDup hRootDup => simp_all
  | capDupDenied hDup => simp_all
  | capSend hCap => simp_all
  | capRecv cap hSent hCap => simp_all
  | capRecvEmpty hEmpty => simp_all
  | capRevoke => simp_all
  | objectCreate hRootMint => exact ⟨rfl, hRootMint⟩
  | objectCreateDenied hMint => simp_all
  | rejectStale hStale => simp_all
  | rejectRevoked hRevoked => simp_all
  | await hWake => simp_all
  | awaitWakePending hWake => simp_all
  | push hRootPush => simp_all
  | pull hConsumerPull => simp_all
  | rejectFull => simp_all

theorem step_sent_cap_created_only_by_valid_send
    {s t : State} {op : Op} {cap : Capability} :
    invariant s ->
    Step s op t ->
    s.sentCap = none ->
    t.sentCap = some cap ->
    op = Op.capSend /\
      cap = s.consumerCap /\
      capCurrentlyAuthorizes s cap /\
      t.transferValid = true := by
  intro _hInv hStep hNoSent hSent
  cases hStep with
  | capDup hRootDup => simp_all
  | capDupDenied hDup => simp_all
  | capSend hCap =>
      simp_all
  | capRecv cap hSentPrev hCap => simp_all
  | capRecvEmpty hEmpty => simp_all
  | capRevoke => simp_all
  | objectCreate hRootMint => simp_all
  | objectCreateDenied hMint => simp_all
  | rejectStale hStale => simp_all
  | rejectRevoked hRevoked => simp_all
  | await hWake => simp_all
  | awaitWakePending hWake => simp_all
  | push hRootPush => simp_all
  | pull hConsumerPull => simp_all
  | rejectFull => simp_all

theorem step_consumer_cap_changed_only_by_authorized_transfer
    {s t : State} {op : Op} :
    invariant s ->
    Step s op t ->
    t.consumerCap ≠ s.consumerCap ->
    (op = Op.capDup /\
      canRootDuplicate s /\
      t.consumerCap = consumerPullCap s /\
      capLineageValid t t.consumerCap) \/
    (exists cap,
      op = Op.capRecv /\
        s.sentCap = some cap /\
        capCurrentlyAuthorizes s cap /\
        t.consumerCap = cap /\
        capLineageValid t t.consumerCap) := by
  intro hInv hStep hChanged
  cases hStep with
  | capDup hRootDup =>
      left
      have hPostLineage :
          capLineageValid
            { s with consumerCap := consumerPullCap s }
            ({ s with consumerCap := consumerPullCap s }).consumerCap :=
        invariant_consumer_cap_lineage_valid (preserve_cap_dup hInv hRootDup)
      exact ⟨rfl, hRootDup, rfl, hPostLineage⟩
  | capDupDenied hDup =>
      exfalso
      exact hChanged rfl
  | capSend hCap =>
      exfalso
      exact hChanged rfl
  | capRecv cap hSent hCap =>
      right
      have hPostLineage :
          capLineageValid
            { s with consumerCap := cap, sentCap := none, transferValid := true }
            ({ s with consumerCap := cap, sentCap := none, transferValid := true }).consumerCap :=
        invariant_consumer_cap_lineage_valid (preserve_cap_recv hInv hSent hCap)
      exact ⟨cap, rfl, hSent, hCap, rfl, hPostLineage⟩
  | capRecvEmpty hEmpty =>
      exfalso
      exact hChanged rfl
  | capRevoke =>
      exfalso
      exact hChanged rfl
  | objectCreate hRootMint =>
      exfalso
      exact hChanged rfl
  | objectCreateDenied hMint =>
      exfalso
      exact hChanged rfl
  | rejectStale hStale =>
      exfalso
      exact hChanged rfl
  | rejectRevoked hRevoked =>
      exfalso
      exact hChanged rfl
  | await hWake =>
      exfalso
      exact hChanged rfl
  | awaitWakePending hWake =>
      exfalso
      exact hChanged rfl
  | push hRootPush =>
      exfalso
      exact hChanged rfl
  | pull hConsumerPull =>
      exfalso
      exact hChanged rfl
  | rejectFull =>
      exfalso
      exact hChanged rfl

theorem step_cap_send_requires_current_authority
    {s t : State} :
    Step s Op.capSend t ->
    capCurrentlyAuthorizes s s.consumerCap := by
  intro hStep
  cases hStep with
  | capSend hAuth => exact hAuth

theorem step_cap_recv_requires_current_authority
    {s t : State} {cap : Capability} :
    Step s Op.capRecv t ->
    s.sentCap = some cap ->
    capCurrentlyAuthorizes s cap := by
  intro hStep hSent
  cases hStep with
  | capRecv cap' hSent' hAuth => simp_all

theorem step_cap_revoke_invalidates_outstanding_main_object_transfer
    {s t : State} {cap : Capability} :
    invariant s ->
    Step s Op.capRevoke t ->
    s.sentCap = some cap ->
    cap.objectId = s.object.objectId ->
    cap.generation = s.object.generation ->
    ¬ capCurrentlyAuthorizes t cap := by
  intro hInv hStep _hSent hObject hGeneration hAuth
  cases hStep with
  | capRevoke =>
      cases hAuth.2 with
      | inl hLive =>
          have hLiveGeneration : cap.generation = s.object.generation + 1 := by
            simpa using hLive.2
          have hImpossible : s.object.generation = s.object.generation + 1 :=
            hGeneration.symm.trans hLiveGeneration
          omega
      | inr hCreated =>
          have hDistinct := invariant_object_ids_distinct hInv
          exact hDistinct (hObject.symm.trans hCreated.2.1)

theorem step_cap_dup_denied_preserves_authority_slots
    {s t : State} :
    Step s Op.capDupDenied t ->
    authoritySlotsUnchanged s t := by
  intro hStep
  cases hStep with
  | capDupDenied hDup => simp [authoritySlotsUnchanged]

theorem step_cap_recv_empty_preserves_authority_slots
    {s t : State} :
    Step s Op.capRecvEmpty t ->
    authoritySlotsUnchanged s t := by
  intro hStep
  cases hStep with
  | capRecvEmpty hEmpty => simp [authoritySlotsUnchanged]

theorem step_object_create_denied_preserves_authority_slots
    {s t : State} :
    Step s Op.objectCreateDenied t ->
    authoritySlotsUnchanged s t := by
  intro hStep
  cases hStep with
  | objectCreateDenied hMint => simp [authoritySlotsUnchanged]

theorem step_reject_stale_preserves_authority_slots
    {s t : State} :
    Step s Op.rejectStale t ->
    authoritySlotsUnchanged s t := by
  intro hStep
  cases hStep with
  | rejectStale hStale => simp [authoritySlotsUnchanged]

theorem step_reject_revoked_preserves_authority_slots
    {s t : State} :
    Step s Op.rejectRevoked t ->
    authoritySlotsUnchanged s t := by
  intro hStep
  cases hStep with
  | rejectRevoked hRevoked => simp [authoritySlotsUnchanged]

theorem step_reject_full_preserves_authority_slots
    {s t : State} :
    Step s Op.rejectFull t ->
    authoritySlotsUnchanged s t := by
  intro hStep
  cases hStep with
  | rejectFull => simp [authoritySlotsUnchanged]

theorem step_failed_authority_operation_preserves_authority_slots
    {s t : State} {op : Op} :
    Step s op t ->
    FailedAuthorityOp op ->
    authoritySlotsUnchanged s t := by
  intro hStep hFailed
  cases hFailed with
  | capDupDenied => exact step_cap_dup_denied_preserves_authority_slots hStep
  | capRecvEmpty => exact step_cap_recv_empty_preserves_authority_slots hStep
  | objectCreateDenied => exact step_object_create_denied_preserves_authority_slots hStep
  | rejectStale => exact step_reject_stale_preserves_authority_slots hStep
  | rejectRevoked => exact step_reject_revoked_preserves_authority_slots hStep
  | rejectFull => exact step_reject_full_preserves_authority_slots hStep

theorem typed_commit_failed_authority_transition_preserves_authority_slots
    {s t : State} {commit : CommitRecord} :
    TypedCommitTransition s commit t ->
    FailedAuthorityOp (commitOpToStepOp commit.op) ->
    authoritySlotsUnchanged s t := by
  intro hCommit hFailed
  exact step_failed_authority_operation_preserves_authority_slots
    (typed_commit_transition_refines_step hCommit) hFailed

theorem typed_commit_non_ok_status_preserves_authority_slots
    {s t : State} {commit : CommitRecord} :
    TypedCommitTransition s commit t ->
    commit.status ≠ CommitStatus.ok ->
    authoritySlotsUnchanged s t := by
  intro hCommit hStatus
  cases hCommit with
  | capDup hRootDup =>
      simp [capDupCommit, commitFromCap] at hStatus
  | capSend hCap =>
      simp [capSendCommit, commitFromCap] at hStatus
  | capRecv cap hSent hCap =>
      simp [capRecvCommit, commitFromCap] at hStatus
  | capRevoke =>
      simp [capRevokeCommit, commitFromCap] at hStatus
  | rejectStale hStale =>
      simp [rejectStaleCommit, commitFromCap, authoritySlotsUnchanged]
  | push hRootPush =>
      simp [pushCommit, commitFromCap] at hStatus
  | pull hConsumerPull =>
      simp [pullCommit, commitFromCap] at hStatus
  | rejectFull =>
      simp [rejectFullCommit, commitFromCap, authoritySlotsUnchanged]
  | capDupDenied hDup =>
      simp [capDupDeniedCommit, commitFromCap, authoritySlotsUnchanged]
  | objectCreate hRootMint =>
      simp [objectCreateCommit, commitFromCap] at hStatus

theorem state_projection_authority_slots_unchanged
    {s t : State}
    {pre post : RtlM1StateProjection} :
    stateMatchesRtlProjection s pre ->
    stateMatchesRtlProjection t post ->
    authoritySlotsUnchanged s t ->
    authoritySlotsProjectionUnchanged pre post := by
  intro hPre hPost hSlots
  rcases hSlots with ⟨hRoot, hConsumer, hSent, hMinted⟩
  rw [hPre, hPost]
  simp [
    stateProjectionOf, authoritySlotsProjectionUnchanged,
    hRoot, hConsumer, hSent, hMinted
  ]

theorem rtl_m1_refinement_failed_authority_transition_preserves_authority_projection
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    FailedAuthorityOp (commitOpToStepOp commitProjection.op) ->
    authoritySlotsProjectionUnchanged pre post := by
  intro hRefine hFailed
  rcases hRefine with ⟨s, t, commit, hPre, hCommitProjection, hCommit, hPost⟩
  have hFailedCommit : FailedAuthorityOp (commitOpToStepOp commit.op) := by
    rw [commitMatchesRtlProjection, commitProjectionToRecord] at hCommitProjection
    rw [hCommitProjection]
    exact hFailed
  exact state_projection_authority_slots_unchanged hPre hPost
    (typed_commit_failed_authority_transition_preserves_authority_slots hCommit hFailedCommit)

theorem rtl_m1_refinement_non_ok_status_preserves_authority_projection
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    commitProjection.status ≠ CommitStatus.ok ->
    authoritySlotsProjectionUnchanged pre post := by
  intro hRefine hStatus
  rcases hRefine with ⟨s, t, commit, hPre, hCommitProjection, hCommit, hPost⟩
  have hStatusCommit : commit.status ≠ CommitStatus.ok := by
    rw [commitMatchesRtlProjection, commitProjectionToRecord] at hCommitProjection
    rw [hCommitProjection]
    exact hStatus
  exact state_projection_authority_slots_unchanged hPre hPost
    (typed_commit_non_ok_status_preserves_authority_slots hCommit hStatusCommit)

theorem commit_projection_op_matches_commit
    {commit : CommitRecord}
    {commitProjection : RtlM1CommitProjection} :
    commitMatchesRtlProjection commit commitProjection ->
    commit.op = commitProjection.op := by
  intro hCommitProjection
  rw [commitMatchesRtlProjection, commitProjectionToRecord] at hCommitProjection
  rw [hCommitProjection]

theorem commit_projection_object_generation_matches_commit
    {commit : CommitRecord}
    {commitProjection : RtlM1CommitProjection} :
    commitMatchesRtlProjection commit commitProjection ->
    commit.objectGeneration = commitProjection.objectGeneration := by
  intro hCommitProjection
  rw [commitMatchesRtlProjection, commitProjectionToRecord] at hCommitProjection
  rw [hCommitProjection]

theorem commit_projection_fdr_generation_matches_commit
    {commit : CommitRecord}
    {commitProjection : RtlM1CommitProjection} :
    commitMatchesRtlProjection commit commitProjection ->
    commit.fdrGeneration = commitProjection.fdrGeneration := by
  intro hCommitProjection
  rw [commitMatchesRtlProjection, commitProjectionToRecord] at hCommitProjection
  rw [hCommitProjection]

theorem capability_from_commit_projection_matches_commit
    {commit : CommitRecord}
    {commitProjection : RtlM1CommitProjection} :
    commitMatchesRtlProjection commit commitProjection ->
    capabilityFromCommitProjection commitProjection =
      capabilityFromCommitRecord commit := by
  intro hCommitProjection
  rw [commitMatchesRtlProjection, commitProjectionToRecord] at hCommitProjection
  rw [hCommitProjection]
  rfl

theorem rtl_m1_refinement_cap_dup_post_consumer_matches_commit_projection
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    commitProjection.op = CommitOp.capDup ->
    post.consumerCap = capabilityFromCommitProjection commitProjection := by
  intro hRefine hOp
  rcases hRefine with ⟨s, t, commit, _hPre, hCommitProjection, hCommit, hPost⟩
  have hCommitOp := commit_projection_op_matches_commit hCommitProjection
  have hProjectionCap :=
    capability_from_commit_projection_matches_commit hCommitProjection
  cases hCommit with
  | capDup hRootDup =>
      rw [hPost]
      exact (by
        simpa [
          stateMatchesRtlProjection, stateProjectionOf, capabilityFromCommitRecord,
          capDupCommit, commitFromCap, consumerPullCap
        ] using hProjectionCap.symm)
  | capSend hCap =>
      simp [capSendCommit, commitFromCap, hOp] at hCommitOp
  | capRecv cap hSent hCap =>
      simp [capRecvCommit, commitFromCap, hOp] at hCommitOp
  | capRevoke =>
      simp [capRevokeCommit, commitFromCap, hOp] at hCommitOp
  | rejectStale hStale =>
      simp [rejectStaleCommit, commitFromCap, hOp] at hCommitOp
  | push hRootPush =>
      simp [pushCommit, commitFromCap, hOp] at hCommitOp
  | pull hConsumerPull =>
      simp [pullCommit, commitFromCap, hOp] at hCommitOp
  | rejectFull =>
      simp [rejectFullCommit, commitFromCap, hOp] at hCommitOp
  | capDupDenied hDup =>
      simp [capDupDeniedCommit, commitFromCap, hOp] at hCommitOp
  | objectCreate hRootMint =>
      simp [objectCreateCommit, commitFromCap, hOp] at hCommitOp

theorem rtl_m1_refinement_cap_send_post_sent_matches_commit_projection
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    commitProjection.op = CommitOp.capSend ->
    post.sentCap = some (capabilityFromCommitProjection commitProjection) := by
  intro hRefine hOp
  rcases hRefine with ⟨s, t, commit, _hPre, hCommitProjection, hCommit, hPost⟩
  have hCommitOp := commit_projection_op_matches_commit hCommitProjection
  have hProjectionCap :=
    capability_from_commit_projection_matches_commit hCommitProjection
  cases hCommit with
  | capDup hRootDup =>
      simp [capDupCommit, commitFromCap, hOp] at hCommitOp
  | capSend hCap =>
      rw [hPost]
      exact (by
        simpa [
          stateMatchesRtlProjection, stateProjectionOf, capabilityFromCommitRecord,
          capSendCommit, commitFromCap
        ] using congrArg some hProjectionCap.symm)
  | capRecv cap hSent hCap =>
      simp [capRecvCommit, commitFromCap, hOp] at hCommitOp
  | capRevoke =>
      simp [capRevokeCommit, commitFromCap, hOp] at hCommitOp
  | rejectStale hStale =>
      simp [rejectStaleCommit, commitFromCap, hOp] at hCommitOp
  | push hRootPush =>
      simp [pushCommit, commitFromCap, hOp] at hCommitOp
  | pull hConsumerPull =>
      simp [pullCommit, commitFromCap, hOp] at hCommitOp
  | rejectFull =>
      simp [rejectFullCommit, commitFromCap, hOp] at hCommitOp
  | capDupDenied hDup =>
      simp [capDupDeniedCommit, commitFromCap, hOp] at hCommitOp
  | objectCreate hRootMint =>
      simp [objectCreateCommit, commitFromCap, hOp] at hCommitOp

theorem rtl_m1_refinement_cap_recv_post_consumer_matches_commit_projection
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    commitProjection.op = CommitOp.capRecv ->
    post.consumerCap = capabilityFromCommitProjection commitProjection /\
      post.sentCap = none := by
  intro hRefine hOp
  rcases hRefine with ⟨s, t, commit, _hPre, hCommitProjection, hCommit, hPost⟩
  have hCommitOp := commit_projection_op_matches_commit hCommitProjection
  have hProjectionCap :=
    capability_from_commit_projection_matches_commit hCommitProjection
  cases hCommit with
  | capDup hRootDup =>
      simp [capDupCommit, commitFromCap, hOp] at hCommitOp
  | capSend hCap =>
      simp [capSendCommit, commitFromCap, hOp] at hCommitOp
  | capRecv cap hSent hCap =>
      rw [hPost]
      constructor
      · exact (by
          simpa [
            stateMatchesRtlProjection, stateProjectionOf, capabilityFromCommitRecord,
            capRecvCommit, commitFromCap
          ] using hProjectionCap.symm)
      · simp [stateMatchesRtlProjection, stateProjectionOf]
  | capRevoke =>
      simp [capRevokeCommit, commitFromCap, hOp] at hCommitOp
  | rejectStale hStale =>
      simp [rejectStaleCommit, commitFromCap, hOp] at hCommitOp
  | push hRootPush =>
      simp [pushCommit, commitFromCap, hOp] at hCommitOp
  | pull hConsumerPull =>
      simp [pullCommit, commitFromCap, hOp] at hCommitOp
  | rejectFull =>
      simp [rejectFullCommit, commitFromCap, hOp] at hCommitOp
  | capDupDenied hDup =>
      simp [capDupDeniedCommit, commitFromCap, hOp] at hCommitOp
  | objectCreate hRootMint =>
      simp [objectCreateCommit, commitFromCap, hOp] at hCommitOp

theorem rtl_m1_refinement_object_create_post_minted_matches_commit_projection
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    commitProjection.op = CommitOp.objectCreate ->
    post.mintedCap = some (capabilityFromCommitProjection commitProjection) /\
      post.createdObjectCreated = true := by
  intro hRefine hOp
  rcases hRefine with ⟨s, t, commit, _hPre, hCommitProjection, hCommit, hPost⟩
  have hCommitOp := commit_projection_op_matches_commit hCommitProjection
  have hProjectionCap :=
    capability_from_commit_projection_matches_commit hCommitProjection
  cases hCommit with
  | capDup hRootDup =>
      simp [capDupCommit, commitFromCap, hOp] at hCommitOp
  | capSend hCap =>
      simp [capSendCommit, commitFromCap, hOp] at hCommitOp
  | capRecv cap hSent hCap =>
      simp [capRecvCommit, commitFromCap, hOp] at hCommitOp
  | capRevoke =>
      simp [capRevokeCommit, commitFromCap, hOp] at hCommitOp
  | rejectStale hStale =>
      simp [rejectStaleCommit, commitFromCap, hOp] at hCommitOp
  | push hRootPush =>
      simp [pushCommit, commitFromCap, hOp] at hCommitOp
  | pull hConsumerPull =>
      simp [pullCommit, commitFromCap, hOp] at hCommitOp
  | rejectFull =>
      simp [rejectFullCommit, commitFromCap, hOp] at hCommitOp
  | capDupDenied hDup =>
      simp [capDupDeniedCommit, commitFromCap, hOp] at hCommitOp
  | objectCreate hRootMint =>
      rw [hPost]
      constructor
      · exact (by
          simpa [
            stateMatchesRtlProjection, stateProjectionOf, capabilityFromCommitRecord,
            objectCreateCommit, commitFromCap, mintedObjectCap
          ] using congrArg some hProjectionCap.symm)
      · simp [stateMatchesRtlProjection, stateProjectionOf]

theorem rtl_m1_refinement_push_post_wake_matches_commit_projection
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    commitProjection.op = CommitOp.push ->
    post.rootCap = capabilityFromCommitProjection commitProjection /\
      post.wakePending = true := by
  intro hRefine hOp
  rcases hRefine with ⟨s, t, commit, _hPre, hCommitProjection, hCommit, hPost⟩
  have hCommitOp := commit_projection_op_matches_commit hCommitProjection
  have hProjectionCap :=
    capability_from_commit_projection_matches_commit hCommitProjection
  cases hCommit with
  | capDup hRootDup =>
      simp [capDupCommit, commitFromCap, hOp] at hCommitOp
  | capSend hCap =>
      simp [capSendCommit, commitFromCap, hOp] at hCommitOp
  | capRecv cap hSent hCap =>
      simp [capRecvCommit, commitFromCap, hOp] at hCommitOp
  | capRevoke =>
      simp [capRevokeCommit, commitFromCap, hOp] at hCommitOp
  | rejectStale hStale =>
      simp [rejectStaleCommit, commitFromCap, hOp] at hCommitOp
  | push hRootPush =>
      rw [hPost]
      constructor
      · exact (by
          simpa [
            stateMatchesRtlProjection, stateProjectionOf, capabilityFromCommitRecord,
            pushCommit, commitFromCap
          ] using hProjectionCap.symm)
      · simp [stateMatchesRtlProjection, stateProjectionOf]
  | pull hConsumerPull =>
      simp [pullCommit, commitFromCap, hOp] at hCommitOp
  | rejectFull =>
      simp [rejectFullCommit, commitFromCap, hOp] at hCommitOp
  | capDupDenied hDup =>
      simp [capDupDeniedCommit, commitFromCap, hOp] at hCommitOp
  | objectCreate hRootMint =>
      simp [objectCreateCommit, commitFromCap, hOp] at hCommitOp

theorem rtl_m1_refinement_pull_post_wake_matches_commit_projection
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    commitProjection.op = CommitOp.pull ->
    post.consumerCap = capabilityFromCommitProjection commitProjection /\
      post.wakePending = false := by
  intro hRefine hOp
  rcases hRefine with ⟨s, t, commit, _hPre, hCommitProjection, hCommit, hPost⟩
  have hCommitOp := commit_projection_op_matches_commit hCommitProjection
  have hProjectionCap :=
    capability_from_commit_projection_matches_commit hCommitProjection
  cases hCommit with
  | capDup hRootDup =>
      simp [capDupCommit, commitFromCap, hOp] at hCommitOp
  | capSend hCap =>
      simp [capSendCommit, commitFromCap, hOp] at hCommitOp
  | capRecv cap hSent hCap =>
      simp [capRecvCommit, commitFromCap, hOp] at hCommitOp
  | capRevoke =>
      simp [capRevokeCommit, commitFromCap, hOp] at hCommitOp
  | rejectStale hStale =>
      simp [rejectStaleCommit, commitFromCap, hOp] at hCommitOp
  | push hRootPush =>
      simp [pushCommit, commitFromCap, hOp] at hCommitOp
  | pull hConsumerPull =>
      rw [hPost]
      constructor
      · exact (by
          simpa [
            stateMatchesRtlProjection, stateProjectionOf, capabilityFromCommitRecord,
            pullCommit, commitFromCap
          ] using hProjectionCap.symm)
      · simp [stateMatchesRtlProjection, stateProjectionOf]
  | rejectFull =>
      simp [rejectFullCommit, commitFromCap, hOp] at hCommitOp
  | capDupDenied hDup =>
      simp [capDupDeniedCommit, commitFromCap, hOp] at hCommitOp
  | objectCreate hRootMint =>
      simp [objectCreateCommit, commitFromCap, hOp] at hCommitOp

theorem rtl_m1_refinement_reject_full_post_failure_matches_commit_projection
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    commitProjection.op = CommitOp.rejectFull ->
    post.fullWasExplicit = true /\
      authoritySlotsProjectionUnchanged pre post /\
      commitProjection.status = CommitStatus.eagain := by
  intro hRefine hOp
  rcases hRefine with ⟨s, t, commit, hPre, hCommitProjection, hCommit, hPost⟩
  have hCommitOp := commit_projection_op_matches_commit hCommitProjection
  cases hCommit with
  | capDup hRootDup =>
      simp [capDupCommit, commitFromCap, hOp] at hCommitOp
  | capSend hCap =>
      simp [capSendCommit, commitFromCap, hOp] at hCommitOp
  | capRecv cap hSent hCap =>
      simp [capRecvCommit, commitFromCap, hOp] at hCommitOp
  | capRevoke =>
      simp [capRevokeCommit, commitFromCap, hOp] at hCommitOp
  | rejectStale hStale =>
      simp [rejectStaleCommit, commitFromCap, hOp] at hCommitOp
  | push hRootPush =>
      simp [pushCommit, commitFromCap, hOp] at hCommitOp
  | pull hConsumerPull =>
      simp [pullCommit, commitFromCap, hOp] at hCommitOp
  | rejectFull =>
      constructor
      · rw [hPost]
        simp [stateMatchesRtlProjection, stateProjectionOf]
      constructor
      · exact state_projection_authority_slots_unchanged hPre hPost
          (typed_commit_non_ok_status_preserves_authority_slots
            (TypedCommitTransition.rejectFull s) (by simp [rejectFullCommit, commitFromCap]))
      · rw [commitMatchesRtlProjection, commitProjectionToRecord] at hCommitProjection
        have hStatus := congrArg CommitRecord.status hCommitProjection
        simp [rejectFullCommit, commitFromCap] at hStatus
        exact hStatus.symm
  | capDupDenied hDup =>
      simp [capDupDeniedCommit, commitFromCap, hOp] at hCommitOp
  | objectCreate hRootMint =>
      simp [objectCreateCommit, commitFromCap, hOp] at hCommitOp

theorem rtl_m1_refinement_reject_stale_post_failure_matches_commit_projection
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    commitProjection.op = CommitOp.rejectStale ->
    post.staleRejected = true /\
      authoritySlotsProjectionUnchanged pre post /\
      commitProjection.status = CommitStatus.erevoked := by
  intro hRefine hOp
  rcases hRefine with ⟨s, t, commit, hPre, hCommitProjection, hCommit, hPost⟩
  have hCommitOp := commit_projection_op_matches_commit hCommitProjection
  cases hCommit with
  | capDup hRootDup =>
      simp [capDupCommit, commitFromCap, hOp] at hCommitOp
  | capSend hCap =>
      simp [capSendCommit, commitFromCap, hOp] at hCommitOp
  | capRecv cap hSent hCap =>
      simp [capRecvCommit, commitFromCap, hOp] at hCommitOp
  | capRevoke =>
      simp [capRevokeCommit, commitFromCap, hOp] at hCommitOp
  | rejectStale hStale =>
      constructor
      · rw [hPost]
        simp [stateMatchesRtlProjection, stateProjectionOf]
      constructor
      · exact state_projection_authority_slots_unchanged hPre hPost
          (typed_commit_non_ok_status_preserves_authority_slots
            (TypedCommitTransition.rejectStale s hStale) (by simp [rejectStaleCommit, commitFromCap]))
      · rw [commitMatchesRtlProjection, commitProjectionToRecord] at hCommitProjection
        have hStatus := congrArg CommitRecord.status hCommitProjection
        simp [rejectStaleCommit, commitFromCap] at hStatus
        exact hStatus.symm
  | push hRootPush =>
      simp [pushCommit, commitFromCap, hOp] at hCommitOp
  | pull hConsumerPull =>
      simp [pullCommit, commitFromCap, hOp] at hCommitOp
  | rejectFull =>
      simp [rejectFullCommit, commitFromCap, hOp] at hCommitOp
  | capDupDenied hDup =>
      simp [capDupDeniedCommit, commitFromCap, hOp] at hCommitOp
  | objectCreate hRootMint =>
      simp [objectCreateCommit, commitFromCap, hOp] at hCommitOp

theorem rtl_m1_refinement_cap_dup_denied_post_failure_matches_commit_projection
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    commitProjection.op = CommitOp.capDupDenied ->
    post.failedNoAuthority = true /\
      authoritySlotsProjectionUnchanged pre post /\
      commitProjection.status = CommitStatus.eperm := by
  intro hRefine hOp
  rcases hRefine with ⟨s, t, commit, hPre, hCommitProjection, hCommit, hPost⟩
  have hCommitOp := commit_projection_op_matches_commit hCommitProjection
  cases hCommit with
  | capDup hRootDup =>
      simp [capDupCommit, commitFromCap, hOp] at hCommitOp
  | capSend hCap =>
      simp [capSendCommit, commitFromCap, hOp] at hCommitOp
  | capRecv cap hSent hCap =>
      simp [capRecvCommit, commitFromCap, hOp] at hCommitOp
  | capRevoke =>
      simp [capRevokeCommit, commitFromCap, hOp] at hCommitOp
  | rejectStale hStale =>
      simp [rejectStaleCommit, commitFromCap, hOp] at hCommitOp
  | push hRootPush =>
      simp [pushCommit, commitFromCap, hOp] at hCommitOp
  | pull hConsumerPull =>
      simp [pullCommit, commitFromCap, hOp] at hCommitOp
  | rejectFull =>
      simp [rejectFullCommit, commitFromCap, hOp] at hCommitOp
  | capDupDenied hDup =>
      constructor
      · rw [hPost]
        simp [stateMatchesRtlProjection, stateProjectionOf]
      constructor
      · exact state_projection_authority_slots_unchanged hPre hPost
          (typed_commit_non_ok_status_preserves_authority_slots
            (TypedCommitTransition.capDupDenied s hDup) (by simp [capDupDeniedCommit, commitFromCap]))
      · rw [commitMatchesRtlProjection, commitProjectionToRecord] at hCommitProjection
        have hStatus := congrArg CommitRecord.status hCommitProjection
        simp [capDupDeniedCommit, commitFromCap] at hStatus
        exact hStatus.symm
  | objectCreate hRootMint =>
      simp [objectCreateCommit, commitFromCap, hOp] at hCommitOp

theorem rtl_m1_refinement_cap_revoke_post_generation_matches_commit_projection
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    commitProjection.op = CommitOp.capRevoke ->
    post.objectGeneration = commitProjection.objectGeneration /\
      post.rootCap.generation = commitProjection.objectGeneration /\
      post.revokedGeneration = commitProjection.fdrGeneration /\
      post.hasRevokedGeneration = true := by
  intro hRefine hOp
  rcases hRefine with ⟨s, t, commit, _hPre, hCommitProjection, hCommit, hPost⟩
  have hCommitOp := commit_projection_op_matches_commit hCommitProjection
  have hObjectGeneration :=
    commit_projection_object_generation_matches_commit hCommitProjection
  have hFdrGeneration :=
    commit_projection_fdr_generation_matches_commit hCommitProjection
  cases hCommit with
  | capDup hRootDup =>
      simp [capDupCommit, commitFromCap, hOp] at hCommitOp
  | capSend hCap =>
      simp [capSendCommit, commitFromCap, hOp] at hCommitOp
  | capRecv cap hSent hCap =>
      simp [capRecvCommit, commitFromCap, hOp] at hCommitOp
  | capRevoke =>
      rw [hPost]
      constructor
      · exact (by
          simpa [
            stateMatchesRtlProjection, stateProjectionOf, capRevokeCommit,
            commitFromCap
          ] using hObjectGeneration)
      constructor
      · exact (by
          simpa [
            stateMatchesRtlProjection, stateProjectionOf, capRevokeCommit,
            commitFromCap
          ] using hObjectGeneration)
      constructor
      · exact (by
          simpa [
            stateMatchesRtlProjection, stateProjectionOf, capRevokeCommit,
            commitFromCap
          ] using hFdrGeneration)
      · simp [stateMatchesRtlProjection, stateProjectionOf]
  | rejectStale hStale =>
      simp [rejectStaleCommit, commitFromCap, hOp] at hCommitOp
  | push hRootPush =>
      simp [pushCommit, commitFromCap, hOp] at hCommitOp
  | pull hConsumerPull =>
      simp [pullCommit, commitFromCap, hOp] at hCommitOp
  | rejectFull =>
      simp [rejectFullCommit, commitFromCap, hOp] at hCommitOp
  | capDupDenied hDup =>
      simp [capDupDeniedCommit, commitFromCap, hOp] at hCommitOp
  | objectCreate hRootMint =>
      simp [objectCreateCommit, commitFromCap, hOp] at hCommitOp

def RtlM1RefinementPostcondition
    (commitProjection : RtlM1CommitProjection)
    (pre post : RtlM1StateProjection) : Prop :=
  match commitProjection.op with
  | CommitOp.capDup =>
      post.consumerCap = capabilityFromCommitProjection commitProjection
  | CommitOp.capSend =>
      post.sentCap = some (capabilityFromCommitProjection commitProjection)
  | CommitOp.capRecv =>
      post.consumerCap = capabilityFromCommitProjection commitProjection /\
        post.sentCap = none
  | CommitOp.capRevoke =>
      post.objectGeneration = commitProjection.objectGeneration /\
        post.rootCap.generation = commitProjection.objectGeneration /\
        post.revokedGeneration = commitProjection.fdrGeneration /\
        post.hasRevokedGeneration = true
  | CommitOp.rejectStale =>
      post.staleRejected = true /\
        authoritySlotsProjectionUnchanged pre post /\
        commitProjection.status = CommitStatus.erevoked
  | CommitOp.push =>
      post.rootCap = capabilityFromCommitProjection commitProjection /\
        post.wakePending = true
  | CommitOp.pull =>
      post.consumerCap = capabilityFromCommitProjection commitProjection /\
        post.wakePending = false
  | CommitOp.rejectFull =>
      post.fullWasExplicit = true /\
        authoritySlotsProjectionUnchanged pre post /\
        commitProjection.status = CommitStatus.eagain
  | CommitOp.capDupDenied =>
      post.failedNoAuthority = true /\
        authoritySlotsProjectionUnchanged pre post /\
        commitProjection.status = CommitStatus.eperm
  | CommitOp.objectCreate =>
      post.mintedCap = some (capabilityFromCommitProjection commitProjection) /\
        post.createdObjectCreated = true

theorem rtl_m1_refinement_step_satisfies_postcondition
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection} :
    RtlM1RefinementStep pre commitProjection post ->
    RtlM1RefinementPostcondition commitProjection pre post := by
  intro hRefine
  cases hOp : commitProjection.op <;>
    simp [RtlM1RefinementPostcondition, hOp]
  · exact rtl_m1_refinement_cap_dup_post_consumer_matches_commit_projection hRefine hOp
  · exact rtl_m1_refinement_cap_send_post_sent_matches_commit_projection hRefine hOp
  · exact rtl_m1_refinement_cap_recv_post_consumer_matches_commit_projection hRefine hOp
  · exact rtl_m1_refinement_cap_revoke_post_generation_matches_commit_projection hRefine hOp
  · exact rtl_m1_refinement_reject_stale_post_failure_matches_commit_projection hRefine hOp
  · exact rtl_m1_refinement_push_post_wake_matches_commit_projection hRefine hOp
  · exact rtl_m1_refinement_pull_post_wake_matches_commit_projection hRefine hOp
  · exact rtl_m1_refinement_reject_full_post_failure_matches_commit_projection hRefine hOp
  · exact rtl_m1_refinement_cap_dup_denied_post_failure_matches_commit_projection hRefine hOp
  · exact rtl_m1_refinement_object_create_post_minted_matches_commit_projection hRefine hOp

theorem reachable_invariant {s : State} :
    Reachable s -> invariant s := by
  intro hReach
  induction hReach with
  | reset => exact invariant_reset
  | step hPrev hStep ih => exact invariant_step ih hStep

theorem m1_t3_typed_commit_transition_refines_step_for_reachable
    {s t : State} {commit : CommitRecord} :
    Reachable s ->
    TypedCommitTransition s commit t ->
    Step s (commitOpToStepOp commit.op) t := by
  intro _hReach hCommit
  exact typed_commit_transition_refines_step hCommit

theorem m1_t3_typed_commit_transition_preserves_invariant_for_reachable
    {s t : State} {commit : CommitRecord} :
    Reachable s ->
    TypedCommitTransition s commit t ->
    invariant t := by
  intro hReach hCommit
  exact typed_commit_transition_preserves_invariant
    (reachable_invariant hReach) hCommit

theorem m1_t3_typed_commit_transition_status_matches_op_for_reachable
    {s t : State} {commit : CommitRecord} :
    Reachable s ->
    TypedCommitTransition s commit t ->
    commit.status = expectedCommitStatus commit.op := by
  intro _hReach hCommit
  exact typed_commit_transition_status_matches_op hCommit

theorem m1_t3_rtl_m1_refinement_step_preserves_sg_auth_invariant_for_reachable
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection}
    {s t : State}
    {commit : CommitRecord} :
    Reachable s ->
    stateMatchesRtlProjection s pre ->
    commitMatchesRtlProjection commit commitProjection ->
    TypedCommitTransition s commit t ->
    stateMatchesRtlProjection t post ->
    RtlM1RefinementStep pre commitProjection post /\
      Step s (commitOpToStepOp commit.op) t /\
      invariant t := by
  intro hReach hPre hCommitProjection hCommit hPost
  exact ⟨
    ⟨s, t, commit, hPre, hCommitProjection, hCommit, hPost⟩,
    typed_commit_transition_refines_step hCommit,
    typed_commit_transition_preserves_invariant (reachable_invariant hReach) hCommit
  ⟩

theorem m1_t3_rtl_m1_refinement_step_refines_commit_projection_op_for_reachable
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection}
    {s t : State}
    {commit : CommitRecord} :
    Reachable s ->
    stateMatchesRtlProjection s pre ->
    commitMatchesRtlProjection commit commitProjection ->
    TypedCommitTransition s commit t ->
    stateMatchesRtlProjection t post ->
    RtlM1RefinementStep pre commitProjection post /\
      Step s (commitOpToStepOp commitProjection.op) t /\
      invariant t := by
  intro hReach hPre hCommitProjection hCommit hPost
  constructor
  · exact ⟨s, t, commit, hPre, hCommitProjection, hCommit, hPost⟩
  constructor
  · rw [commitMatchesRtlProjection, commitProjectionToRecord] at hCommitProjection
    subst commit
    simpa using typed_commit_transition_refines_step hCommit
  · exact typed_commit_transition_preserves_invariant
      (reachable_invariant hReach) hCommit

theorem m1_t3_rtl_m1_refinement_step_refines_preserves_and_satisfies_postcondition_for_reachable
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection}
    {s t : State}
    {commit : CommitRecord} :
    Reachable s ->
    stateMatchesRtlProjection s pre ->
    commitMatchesRtlProjection commit commitProjection ->
    TypedCommitTransition s commit t ->
    stateMatchesRtlProjection t post ->
    RtlM1RefinementStep pre commitProjection post /\
      Step s (commitOpToStepOp commitProjection.op) t /\
      invariant t /\
      RtlM1RefinementPostcondition commitProjection pre post := by
  intro hReach hPre hCommitProjection hCommit hPost
  have hRefine : RtlM1RefinementStep pre commitProjection post :=
    ⟨s, t, commit, hPre, hCommitProjection, hCommit, hPost⟩
  have hBundle :=
    m1_t3_rtl_m1_refinement_step_refines_commit_projection_op_for_reachable
      hReach hPre hCommitProjection hCommit hPost
  exact ⟨
    hRefine,
    hBundle.2.1,
    hBundle.2.2,
    rtl_m1_refinement_step_satisfies_postcondition hRefine
  ⟩

theorem m1_t3_rtl_m1_refinement_step_status_matches_op_for_reachable
    {pre : RtlM1StateProjection}
    {commitProjection : RtlM1CommitProjection}
    {post : RtlM1StateProjection}
    {s t : State}
    {commit : CommitRecord} :
    Reachable s ->
    stateMatchesRtlProjection s pre ->
    commitMatchesRtlProjection commit commitProjection ->
    TypedCommitTransition s commit t ->
    stateMatchesRtlProjection t post ->
    commitProjection.status = expectedCommitStatus commitProjection.op := by
  intro _hReach hPre hCommitProjection hCommit hPost
  have hRefine : RtlM1RefinementStep pre commitProjection post :=
    ⟨s, t, commit, hPre, hCommitProjection, hCommit, hPost⟩
  exact rtl_m1_refinement_step_status_matches_op hRefine

theorem m1_t3_typed_commit_failed_authority_transition_preserves_authority_slots_for_reachable
    {s t : State} {commit : CommitRecord} :
    Reachable s ->
    TypedCommitTransition s commit t ->
    FailedAuthorityOp (commitOpToStepOp commit.op) ->
    authoritySlotsUnchanged s t := by
  intro _hReach hCommit hFailed
  exact typed_commit_failed_authority_transition_preserves_authority_slots hCommit hFailed

theorem m1_t3_typed_commit_non_ok_status_preserves_authority_slots_for_reachable
    {s t : State} {commit : CommitRecord} :
    Reachable s ->
    TypedCommitTransition s commit t ->
    commit.status ≠ CommitStatus.ok ->
    authoritySlotsUnchanged s t := by
  intro _hReach hCommit hStatus
  exact typed_commit_non_ok_status_preserves_authority_slots hCommit hStatus

theorem m1_t3_consumer_cap_lineage_valid_for_all_reachable {s : State} :
    Reachable s -> capLineageValid s s.consumerCap := by
  intro hReach
  exact invariant_consumer_cap_lineage_valid (reachable_invariant hReach)

-- Compatibility name for older manifests. This proves lineage validity for the
-- modeled consumer cap, not a full architecture-wide non-forgeability theorem.
theorem m1_t3_no_forged_fdr_for_all_reachable {s : State} :
    Reachable s -> capLineageValid s s.consumerCap := by
  exact m1_t3_consumer_cap_lineage_valid_for_all_reachable

theorem m1_t3_consumer_cap_rights_subset_root_for_all_reachable {s : State} :
    Reachable s -> Rights.subset s.consumerCap.rights s.rootCap.rights := by
  intro hReach
  exact capLineageValid_rights_subset
    (m1_t3_consumer_cap_lineage_valid_for_all_reachable hReach)

theorem m1_t3_no_authority_amplification_for_all_reachable {s : State} :
    Reachable s -> Rights.subset s.consumerCap.rights s.rootCap.rights := by
  exact m1_t3_consumer_cap_rights_subset_root_for_all_reachable

theorem m1_t3_sent_cap_lineage_valid_for_all_reachable {s : State} {cap : Capability} :
    Reachable s -> s.sentCap = some cap -> capLineageValid s cap := by
  intro hReach hSent
  exact invariant_sent_cap_lineage_valid (reachable_invariant hReach) cap hSent

-- Compatibility name for older manifests. This is lineage validity for the sent
-- cap; `validTransferState` carries the transfer-validity bit.
theorem m1_t3_sent_cap_authority_for_all_reachable {s : State} {cap : Capability} :
    Reachable s -> s.sentCap = some cap -> capLineageValid s cap := by
  exact m1_t3_sent_cap_lineage_valid_for_all_reachable

theorem m1_t3_valid_transfer_for_all_reachable {s : State} :
    Reachable s -> validTransferState s := by
  intro hReach
  exact invariant_valid_transfer_state (reachable_invariant hReach)

theorem m1_t3_minted_caps_authorized_for_all_reachable {s : State} :
    Reachable s -> mintedAuthorityState s := by
  intro hReach
  exact invariant_minted_authority_state (reachable_invariant hReach)

theorem m1_t3_minted_caps_currently_authorize_created_object_for_all_reachable {s : State} {cap : Capability} :
    Reachable s ->
    s.mintedCap = some cap ->
    capCurrentlyAuthorizes s cap /\
      cap.objectId = s.createdObject.objectId /\
      cap.generation = s.createdObject.generation /\
      cap.ownerDomain = s.rootDomain.id := by
  intro hReach hMinted
  have hMintedAuthority := m1_t3_minted_caps_authorized_for_all_reachable hReach cap hMinted
  rcases hMintedAuthority with
    ⟨hCreated, hObject, hGeneration, hOwner, hLineage, hSealed, hRights⟩
  have hAuth : capCurrentlyAuthorizes s cap := by
    constructor
    · exact ⟨Or.inr hObject, Or.inl hOwner, hLineage, hSealed, hRights⟩
    · exact Or.inr ⟨hCreated, hObject, hGeneration⟩
  exact ⟨hAuth, hObject, hGeneration, hOwner⟩

theorem m1_t3_minted_cap_created_only_by_authorized_object_create_for_reachable
    {s t : State} {op : Op} {cap : Capability} :
    Reachable s ->
    Step s op t ->
    s.mintedCap = none ->
    t.mintedCap = some cap ->
    op = Op.objectCreate /\ canRootMint s := by
  intro _hReach hStep hNoMinted hMinted
  exact step_minted_cap_created_only_by_authorized_object_create hStep hNoMinted hMinted

theorem m1_t3_sent_cap_created_only_by_valid_send_for_reachable
    {s t : State} {op : Op} {cap : Capability} :
    Reachable s ->
    Step s op t ->
    s.sentCap = none ->
    t.sentCap = some cap ->
    op = Op.capSend /\
      cap = s.consumerCap /\
      capCurrentlyAuthorizes s cap /\
      t.transferValid = true := by
  intro hReach hStep hNoSent hSent
  exact step_sent_cap_created_only_by_valid_send
    (reachable_invariant hReach) hStep hNoSent hSent

theorem m1_t3_consumer_cap_changed_only_by_authorized_transfer_for_reachable
    {s t : State} {op : Op} :
    Reachable s ->
    Step s op t ->
    t.consumerCap ≠ s.consumerCap ->
    (op = Op.capDup /\
      canRootDuplicate s /\
      t.consumerCap = consumerPullCap s /\
      capLineageValid t t.consumerCap) \/
    (exists cap,
        op = Op.capRecv /\
        s.sentCap = some cap /\
        capCurrentlyAuthorizes s cap /\
        t.consumerCap = cap /\
        capLineageValid t t.consumerCap) := by
  intro hReach hStep hChanged
  exact step_consumer_cap_changed_only_by_authorized_transfer
    (reachable_invariant hReach) hStep hChanged

theorem m1_t3_cap_send_requires_current_authority_for_reachable
    {s t : State} :
    Reachable s ->
    Step s Op.capSend t ->
    capCurrentlyAuthorizes s s.consumerCap := by
  intro _hReach hStep
  exact step_cap_send_requires_current_authority hStep

theorem m1_t3_cap_recv_requires_current_authority_for_reachable
    {s t : State} {cap : Capability} :
    Reachable s ->
    Step s Op.capRecv t ->
    s.sentCap = some cap ->
    capCurrentlyAuthorizes s cap := by
  intro _hReach hStep hSent
  exact step_cap_recv_requires_current_authority hStep hSent

theorem m1_t3_revoke_invalidates_outstanding_main_object_transfer_for_reachable
    {s t : State} {cap : Capability} :
    Reachable s ->
    Step s Op.capRevoke t ->
    s.sentCap = some cap ->
    cap.objectId = s.object.objectId ->
    cap.generation = s.object.generation ->
    ¬ capCurrentlyAuthorizes t cap := by
  intro hReach hStep hSent hObject hGeneration
  exact step_cap_revoke_invalidates_outstanding_main_object_transfer
    (reachable_invariant hReach) hStep hSent hObject hGeneration

theorem m1_t3_failed_authority_operations_preserve_authority_slots_for_reachable
    {s t : State} {op : Op} :
    Reachable s ->
    Step s op t ->
    FailedAuthorityOp op ->
    authoritySlotsUnchanged s t := by
  intro _hReach hStep hFailed
  exact step_failed_authority_operation_preserves_authority_slots hStep hFailed

theorem m1_t3_stored_cap_lineage_valid_for_all_reachable {s : State} {cap : Capability} :
    Reachable s ->
    capStoredInState s cap ->
    capLineageValid s cap := by
  intro hReach hStored
  have hInv := reachable_invariant hReach
  rcases hStored with hRoot | hConsumer | hSent | hMinted
  · rw [hRoot]
    exact invariant_root_cap_lineage_valid hInv
  · rw [hConsumer]
    exact invariant_consumer_cap_lineage_valid hInv
  · exact invariant_sent_cap_lineage_valid hInv cap hSent
  · have hMintedAuth :=
      m1_t3_minted_caps_currently_authorize_created_object_for_all_reachable hReach hMinted
    exact hMintedAuth.1.1

theorem m1_t3_no_authority_amplification_for_all_stored_caps {s : State} {cap : Capability} :
    Reachable s ->
    capStoredInState s cap ->
    Rights.subset cap.rights s.rootCap.rights := by
  intro hReach hStored
  exact capLineageValid_rights_subset
    (m1_t3_stored_cap_lineage_valid_for_all_reachable hReach hStored)

theorem m1_t3_stale_generation_cannot_consumer_pull_from_main_object {s : State} :
    Reachable s ->
    s.consumerCap.generation ≠ s.object.generation ->
    ¬ canAuthorizePullFromMainObject s s.consumerCap := by
  intro _hReach hStale hStart
  exact hStale hStart.2.1.2

theorem m1_t3_stale_generation_cannot_authorize_pull_from_main_object {s : State} :
    Reachable s ->
    s.consumerCap.generation ≠ s.object.generation ->
    ¬ canAuthorizePullFromMainObject s s.consumerCap := by
  exact m1_t3_stale_generation_cannot_consumer_pull_from_main_object

theorem m1_t3_stale_generation_cannot_match_live_object {s : State} :
    Reachable s ->
    s.consumerCap.generation ≠ s.object.generation ->
    ¬ generationMatchesLiveObject s s.consumerCap := by
  intro _hReach hStale hMatch
  exact hStale hMatch.2

theorem m1_t3_stale_generation_cannot_be_current_main_object_authority {s : State} :
    Reachable s ->
    s.consumerCap.objectId = s.object.objectId ->
    s.consumerCap.generation ≠ s.object.generation ->
    ¬ canAuthorize s s.consumerCap := by
  intro hReach hObject hStale hAuth
  cases hAuth.2 with
  | inl hLive => exact hStale hLive.2
  | inr hCreated =>
      have hDistinct := invariant_object_ids_distinct (reachable_invariant hReach)
      exact hDistinct (hObject.symm.trans hCreated.2.1)

theorem m1_t3_stale_generation_cannot_authorize_work_for_all_reachable {s : State} :
    Reachable s ->
    s.consumerCap.objectId = s.object.objectId ->
    s.consumerCap.generation ≠ s.object.generation ->
    ¬ canAuthorize s s.consumerCap := by
  exact m1_t3_stale_generation_cannot_be_current_main_object_authority

theorem m1_t3_revoked_caps_cannot_revive_for_all_reachable {s : State} {cap : Capability} :
    Reachable s ->
    s.hasRevokedGeneration = true ->
    cap.objectId = s.object.objectId ->
    cap.generation = s.revokedGeneration ->
    ¬ canAuthorize s cap := by
  intro hReach hHasRevoked hObject hGeneration hAuth
  have hRevoked : s.object.generation = s.revokedGeneration + 1 :=
    invariant_revoked_generation_state (reachable_invariant hReach) hHasRevoked
  cases hAuth.2 with
  | inl hLive =>
      omega
  | inr hCreated =>
      have hDistinct := invariant_object_ids_distinct (reachable_invariant hReach)
      exact hDistinct (hObject.symm.trans hCreated.2.1)

theorem m1_t3_no_lost_wakeup_for_all_reachable {s : State} :
    Reachable s -> noLostWakeupState s := by
  intro hReach
  exact invariant_no_lost_wakeup_state (reachable_invariant hReach)

end Lnp64.M1Transition
