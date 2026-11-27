import WakeReceipt

/-!
# Typed rejection receipts and commit-observation boundary

This module extends the Wake contract model with the five deterministic
rejections persisted by wake_transaction.rs. The stored failure payload is
still only `{error: code}`. In particular, invalid-cause detail is recovered
from the original command after request-hash equality, not from that payload.
`ContextMatches` states that necessary cause-binding premise explicitly.

The database state below is an abstraction. `settle` names the durable states
that an acknowledged commit, a known rollback, or an uncertain commit may
leave. It does not prove PostgreSQL, SQLx, lock scheduling or crash behavior.
The two unknown constructors are possible realities, not observations that
the caller is able to distinguish. Both return `unavailable`.

Rejection classification, authorization before entering the transaction,
canonicalization/hash collision resistance and actual JSON parsing remain
implementation obligations. `runRejection` begins at the receipt gate and
receives the already determined fresh-request rejection as a parameter;
an existing receipt takes precedence over that parameter. The separate
WakeReceipt module models fresh successful Applied/NoOp classification.

Only rejection-attempt audits are counted here. Conflict, successful-apply
and no-op audit categories are outside this counter. No production task
or Core execution attempt is represented by an audit increment.
-/

set_option warningAsError true

namespace SvcWorkflow.WakeDurability

open SvcWorkflow.WakeReceipt (Scope Binding Business Activation)

inductive Rejection where
  | invalidCause (detail : String)
  | principalNotFound
  | principalDisabled
  | instanceNotFound
  | dispatchIntentNotFound
  deriving DecidableEq, Repr

inductive Outcome where
  | success (value : WakeReceipt.Outcome)
  | rejected (value : Rejection)
  deriving DecidableEq, Repr

inductive Payload where
  | success (body : WakeReceipt.Body)
  | failure (code : String)
  | malformed
  deriving DecidableEq, Repr

structure Response where
  status : Nat
  payload : Payload
  deriving DecidableEq, Repr

def encodeRejection : Rejection → Response
  | .invalidCause _ => ⟨422, .failure "invalid_cause"⟩
  | .principalNotFound => ⟨404, .failure "principal_not_found"⟩
  | .principalDisabled => ⟨403, .failure "principal_disabled"⟩
  | .instanceNotFound => ⟨404, .failure "instance_not_found"⟩
  | .dispatchIntentNotFound => ⟨404, .failure "dispatch_intent_not_found"⟩

def encode : Outcome → Response
  | .success value =>
      let response := WakeReceipt.encode value
      ⟨response.status, .success response.body⟩
  | .rejected value => encodeRejection value

def decodeRejection (status : Nat) (code : String) (cause : Option String) :
    Option Rejection :=
  match status, code with
  | 422, "invalid_cause" => cause.map Rejection.invalidCause
  | 404, "principal_not_found" => some .principalNotFound
  | 403, "principal_disabled" => some .principalDisabled
  | 404, "instance_not_found" => some .instanceNotFound
  | 404, "dispatch_intent_not_found" => some .dispatchIntentNotFound
  | _, _ => none

def decode (response : Response) (cause : Option String) : Option Outcome :=
  match response.payload with
  | .success body =>
      (WakeReceipt.decodeTyped ⟨response.status, body⟩).map Outcome.success
  | .failure code => (decodeRejection response.status code cause).map Outcome.rejected
  | .malformed => none

/-- The request hash must bind the original cause. This is not a hash theorem. -/
def ContextMatches (outcome : Outcome) (cause : Option String) : Prop :=
  match outcome with
  | .rejected (.invalidCause detail) => cause = some detail
  | _ => True

theorem completed_outcome_roundtrip (outcome : Outcome) (cause : Option String)
    (context : ContextMatches outcome cause) :
    decode (encode outcome) cause = some outcome := by
  cases outcome with
  | success value =>
      simpa [encode, decode] using congrArg (Option.map Outcome.success)
        (WakeReceipt.typed_roundtrip value)
  | rejected value =>
      cases value <;>
        simp_all [ContextMatches, encode, encodeRejection, decode, decodeRejection]

theorem invalid_cause_payload_does_not_store_detail (first second : String) :
    encodeRejection (.invalidCause first) = encodeRejection (.invalidCause second) := by
  rfl

theorem invalid_cause_without_request_context_is_corrupt :
    decode ⟨422, .failure "invalid_cause"⟩ none = none := by
  rfl

theorem invalid_cause_context_restores_detail (detail : String) :
    decode ⟨422, .failure "invalid_cause"⟩ (some detail) =
      some (.rejected (.invalidCause detail)) := by
  rfl

theorem rejection_is_not_successful_noop (reason : Rejection) (cause : Option String)
    (binding : Binding) (noop : WakeReceipt.NoOpReason)
    (context : ContextMatches (.rejected reason) cause) :
    decode (encode (.rejected reason)) cause ≠ some (.success (.noOp binding noop)) := by
  rw [completed_outcome_roundtrip _ _ context]
  intro impossible
  cases impossible

theorem mismatched_failure_status_is_corrupt (cause : Option String) :
    decode ⟨200, .failure "instance_not_found"⟩ cause = none := by
  rfl

theorem unrecognized_failure_code_is_corrupt (cause : Option String) :
    decode ⟨404, .failure "unrecognized_error"⟩ cause = none := by
  rfl

inductive CommandType where
  | wake | other
  deriving DecidableEq, Repr

inductive ReceiptState where
  | processing
  | completed (response : Response)
  | malformed
  deriving DecidableEq, Repr

structure Receipt where
  commandType : CommandType
  requestHash : Nat
  state : ReceiptState
  deriving DecidableEq, Repr

structure Request where
  scope : Scope
  requestHash : Nat
  cause : Option String
  deriving DecidableEq, Repr

structure Database where
  business : Business
  activation : Activation
  rejectionAuditCount : Nat
  receipts : Scope → Option Receipt

/-- An uncommitted write set; durability is determined separately by settle. -/
def stageRejection (database : Database) (request : Request) (reason : Rejection) : Database :=
  { database with
    rejectionAuditCount := database.rejectionAuditCount + 1
    receipts := fun scope =>
      if scope = request.scope then
        some ⟨.wake, request.requestHash, .completed (encode (.rejected reason))⟩
      else database.receipts scope }

inductive Resolution where
  | acknowledgedCommit
  | knownRollback
  | unknownCommitted
  | unknownRolledBack
  deriving DecidableEq, Repr

inductive Reply where
  | original (outcome : Outcome)
  | unavailable
  | conflict
  | processing
  | corruptReceipt
  deriving DecidableEq, Repr

/-- The two UNKNOWN realities deliberately have the same caller observation. -/
def settle (before staged : Database) (outcome : Outcome) :
    Resolution → Database × Reply
  | .acknowledgedCommit => (staged, .original outcome)
  | .knownRollback => (before, .unavailable)
  | .unknownCommitted => (staged, .unavailable)
  | .unknownRolledBack => (before, .unavailable)

/-- Receipt-only reads can also fail to acknowledge their transaction commit. -/
def observeRead (reply : Reply) : Resolution → Reply
  | .acknowledgedCommit => reply
  | _ => .unavailable

def classifyReplay (request : Request) (receipt : Receipt) : Reply :=
  if receipt.commandType ≠ .wake ∨ receipt.requestHash ≠ request.requestHash then
    .conflict
  else match receipt.state with
    | .processing => .processing
    | .malformed => .corruptReceipt
    | .completed response =>
        match decode response request.cause with
        | some outcome => .original outcome
        | none => .corruptReceipt

def replay (database : Database) (request : Request) (receipt : Receipt)
    (resolution : Resolution) :
    Database × Reply :=
  (database, observeRead (classifyReplay request receipt) resolution)

/-- Receipt replay precedes fresh-request rejection classification and writes. -/
def runRejection (database : Database) (request : Request) (freshReason : Rejection)
    (resolution : Resolution) : Database × Reply :=
  match database.receipts request.scope with
  | some receipt => replay database request receipt resolution
  | none =>
      settle database (stageRejection database request freshReason)
        (.rejected freshReason) resolution

theorem staging_rejection_preserves_business_and_activation
    (database : Database) (request : Request) (reason : Rejection) :
    (stageRejection database request reason).business = database.business ∧
    (stageRejection database request reason).activation = database.activation := by
  exact ⟨rfl, rfl⟩

theorem staging_rejection_records_exactly_one_audit
    (database : Database) (request : Request) (reason : Rejection) :
    (stageRejection database request reason).rejectionAuditCount =
      database.rejectionAuditCount + 1 := by
  rfl

theorem staging_rejection_binds_receipt_to_scope_and_hash
    (database : Database) (request : Request) (reason : Rejection) :
    (stageRejection database request reason).receipts request.scope =
      some ⟨.wake, request.requestHash, .completed (encode (.rejected reason))⟩ := by
  simp [stageRejection]

theorem staging_rejection_preserves_other_scope
    (database : Database) (request : Request) (reason : Rejection) (scope : Scope)
    (different : scope ≠ request.scope) :
    (stageRejection database request reason).receipts scope = database.receipts scope := by
  simp [stageRejection, different]

theorem acknowledged_rejection_is_durable
    (database : Database) (request : Request) (reason : Rejection)
    (fresh : database.receipts request.scope = none) :
    runRejection database request reason .acknowledgedCommit =
      (stageRejection database request reason, .original (.rejected reason)) := by
  simp [runRejection, fresh, settle]

theorem historical_drop_loses_staged_failure_receipt
    (database : Database) (request : Request) (reason : Rejection)
    (fresh : database.receipts request.scope = none) :
    (runRejection database request reason .knownRollback).1.receipts request.scope = none := by
  simp [runRejection, fresh, settle]

theorem fresh_rejection_never_changes_business_even_when_commit_is_unknown
    (database : Database) (request : Request) (reason : Rejection) (resolution : Resolution)
    (fresh : database.receipts request.scope = none) :
    (runRejection database request reason resolution).1.business = database.business ∧
    (runRejection database request reason resolution).1.activation = database.activation := by
  cases resolution <;> simp [runRejection, fresh, settle, stageRejection]

theorem recorded_outcome_replays_before_fresh_rejection
    (database : Database) (request : Request) (outcome : Outcome)
    (freshReason : Rejection) (resolution : Resolution)
    (stored : database.receipts request.scope =
      some ⟨.wake, request.requestHash, .completed (encode outcome)⟩)
    (context : ContextMatches outcome request.cause) :
    runRejection database request freshReason resolution =
      (database, observeRead (.original outcome) resolution) := by
  simp [runRejection, stored, replay, classifyReplay, completed_outcome_roundtrip _ _ context]

theorem processing_receipt_never_returns_a_completed_outcome
    (database : Database) (request : Request) (freshReason : Rejection)
    (resolution : Resolution)
    (stored : database.receipts request.scope = some ⟨.wake, request.requestHash, .processing⟩) :
    runRejection database request freshReason resolution =
      (database, observeRead .processing resolution) := by
  simp [runRejection, stored, replay, classifyReplay]

theorem other_command_receipt_conflicts
    (database : Database) (request : Request) (state : ReceiptState)
    (freshReason : Rejection) (resolution : Resolution)
    (stored : database.receipts request.scope = some ⟨.other, request.requestHash, state⟩) :
    runRejection database request freshReason resolution =
      (database, observeRead .conflict resolution) := by
  simp [runRejection, stored, replay, classifyReplay]

theorem changed_hash_conflicts
    (database : Database) (request : Request) (receipt : Receipt)
    (freshReason : Rejection) (resolution : Resolution)
    (stored : database.receipts request.scope = some receipt)
    (different : receipt.requestHash ≠ request.requestHash) :
    runRejection database request freshReason resolution =
      (database, observeRead .conflict resolution) := by
  simp [runRejection, stored, replay, classifyReplay, different]

/-- Subsequent validation results and transport outcomes may vary arbitrarily. -/
def retryTrace : List (Rejection × Resolution) → Database → Request → Database × List Reply
  | [], database, _ => (database, [])
  | (reason, resolution) :: rest, database, request =>
      let (next, reply) := runRejection database request reason resolution
      let (final, replies) := retryTrace rest next request
      (final, reply :: replies)

theorem arbitrary_replays_preserve_state_and_only_observe_original_or_unavailable
    (database : Database) (request : Request) (outcome : Outcome)
    (attempts : List (Rejection × Resolution))
    (stored : database.receipts request.scope =
      some ⟨.wake, request.requestHash, .completed (encode outcome)⟩)
    (context : ContextMatches outcome request.cause) :
    retryTrace attempts database request =
      (database, attempts.map (fun attempt => observeRead (.original outcome) attempt.2)) := by
  induction attempts with
  | nil => rfl
  | cons attempt rest ih =>
      rcases attempt with ⟨reason, resolution⟩
      simp [retryTrace, recorded_outcome_replays_before_fresh_rejection
        database request outcome reason resolution stored context, ih]

theorem rejection_then_replays_never_duplicate_rejection_audit
    (database : Database) (request : Request) (reason : Rejection)
    (attempts : List (Rejection × Resolution))
    (fresh : database.receipts request.scope = none)
    (context : ContextMatches (.rejected reason) request.cause) :
    (retryTrace attempts
      (runRejection database request reason .acknowledgedCommit).1 request).1.rejectionAuditCount =
      database.rejectionAuditCount + 1 := by
  rw [acknowledged_rejection_is_durable database request reason fresh]
  rw [arbitrary_replays_preserve_state_and_only_observe_original_or_unavailable
    (stageRejection database request reason) request (.rejected reason) attempts
    (staging_rejection_binds_receipt_to_scope_and_hash database request reason) context]
  rfl

theorem unknown_commit_does_not_reveal_durability
    (before staged : Database) (outcome : Outcome) :
    (settle before staged outcome .unknownCommitted).2 =
      (settle before staged outcome .unknownRolledBack).2 := by
  rfl

theorem unknown_committed_rejection_is_recoverable_by_same_request
    (database : Database) (request : Request) (reason nextReason : Rejection)
    (fresh : database.receipts request.scope = none)
    (context : ContextMatches (.rejected reason) request.cause) :
    runRejection (runRejection database request reason .unknownCommitted).1
      request nextReason .acknowledgedCommit =
      (stageRejection database request reason, .original (.rejected reason)) := by
  simp only [runRejection, fresh, settle]
  simp [staging_rejection_binds_receipt_to_scope_and_hash, replay, classifyReplay, observeRead,
    completed_outcome_roundtrip _ _ context]

def witnessDatabase : Database :=
  ⟨WakeReceipt.futureWitnessState.business, WakeReceipt.futureWitnessState.activation,
    0, fun _ => none⟩

def witnessRequest : Request :=
  ⟨WakeReceipt.dueWitnessRequest.scope, WakeReceipt.dueWitnessRequest.requestHash, none⟩

def appliedWitnessOutcome : Outcome :=
  .success (WakeReceipt.appliedOutcome WakeReceipt.futureWitnessState
    WakeReceipt.dueWitnessRequest)

def appliedWitnessDatabase : Database :=
  { witnessDatabase with
    business := (WakeReceipt.appliedState WakeReceipt.futureWitnessState
      WakeReceipt.dueWitnessRequest).business
    activation := (WakeReceipt.appliedState WakeReceipt.futureWitnessState
      WakeReceipt.dueWitnessRequest).activation
    receipts := fun scope =>
      if scope = witnessRequest.scope then
        some ⟨.wake, witnessRequest.requestHash, .completed (encode appliedWitnessOutcome)⟩
      else none }

/-- Same failure observation can accompany either zero or one business increment.
The write set here is an apply witness, not a rejection write set. -/
theorem unknown_apply_can_hide_either_business_result :
    (settle witnessDatabase appliedWitnessDatabase appliedWitnessOutcome .unknownCommitted).2 =
      .unavailable ∧
    (settle witnessDatabase appliedWitnessDatabase appliedWitnessOutcome .unknownRolledBack).2 =
      .unavailable ∧
    (settle witnessDatabase appliedWitnessDatabase appliedWitnessOutcome .unknownCommitted).1.business.version = 2 ∧
    (settle witnessDatabase appliedWitnessDatabase appliedWitnessOutcome .unknownRolledBack).1.business.version = 1 := by
  exact ⟨rfl, rfl, rfl, rfl⟩

end SvcWorkflow.WakeDurability
