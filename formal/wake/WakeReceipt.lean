import Std

/-!
# Wake receipt contract model and historical replay counterexample

Source coordinate: mayf3/svc-workflow@c50ab2ac98decceaa26138a282c34dd92ede0ddc.
Contract: SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V3 / CTR-VAI-008.

`decodeHistorical` abstracts the status-only replay dispatch at the source
coordinate above, before this repair. Its NoOp counterexample is retained as
a regression witness. `decodeTyped`, `run`, and their theorems model the
corrected contract. Source alignment and executable validation are recorded
separately; this module does not establish a Rust-to-Lean refinement theorem.

Abstraction boundary: one already resolved instance/activation pair.
`Resolved` explicitly records the instance/activation binding consistency
required for the state-to-Rust mapping. The total model functions also accept
other values, but no Rust correspondence is claimed for those values.
`Resolved` is NOT a new runtime guard: receipt replay still precedes fresh
instance/activation resolution, as in the source. Binding lookup correctness
itself remains a refinement obligation, not a theorem of this model.
authorization, existence checks, JSON/UUID/time parsing, hash collision
resistance, SQL transaction/lock/MVCC/crash semantics and error receipts are
outside this module. `WakeDurability` adds typed rejection and uncertain
commit observations as a second, explicitly abstract slice. A `run` is
one atomic, serialized transaction. IDs,
canonical-request hashes and timestamps are symbolic natural numbers.
Receipt keys are (principal,key), not just key. Counters count business
Events/facts; attempt-audit writes are deliberately not business changes.
NoOp receipt persistence is a change to receipts while business state stays
the same. i32 version overflow is explicitly rejected before an apply.
-/

set_option warningAsError true

namespace SvcWorkflow.WakeReceipt

structure Scope where
  principal : Nat
  key : Nat
  deriving DecidableEq, Repr

structure Binding where
  instanceId : Nat
  visit : Nat
  deriving DecidableEq, Repr

inductive NoOpReason where
  | instanceClosed | activationClosed | visitNotCurrent
  | versionMismatch | alreadyDue
  deriving DecidableEq, Repr

structure Applied where
  binding : Binding
  version : Nat
  eventSequence : Nat
  nextEligibleAt : Nat
  deriving DecidableEq, Repr

inductive Outcome where
  | applied (value : Applied)
  | noOp (binding : Binding) (reason : NoOpReason)
  deriving DecidableEq, Repr

/-- Optional fields model the distinction between the two stored JSON shapes. -/
structure Body where
  binding : Binding
  wakeApplied : Bool
  reason : Option NoOpReason
  version : Option Nat
  eventSequence : Option Nat
  nextEligibleAt : Option Nat
  deriving DecidableEq, Repr

structure Response where
  status : Nat
  body : Body
  deriving DecidableEq, Repr

def encode : Outcome → Response
  | .applied value =>
      ⟨200, ⟨value.binding, true, none, some value.version,
        some value.eventSequence, some value.nextEligibleAt⟩⟩
  | .noOp binding reason =>
      ⟨200, ⟨binding, false, some reason, none, none, none⟩⟩

/-- Required fields of Rust's WakeAppliedResult; unrelated fields are ignored. -/
def decodeApplied (body : Body) : Option Outcome :=
  match body.version, body.eventSequence, body.nextEligibleAt with
  | some version, some eventSequence, some nextEligibleAt =>
      some (.applied ⟨body.binding, version, eventSequence, nextEligibleAt⟩)
  | _, _, _ => none

def decodeNoOp (body : Body) : Option Outcome :=
  match body.reason with
  | some reason => some (.noOp body.binding reason)
  | none => none

/-- Historical acquire_receipt + replay dispatch at the pinned pre-repair SHA. -/
def decodeHistorical (response : Response) : Option Outcome :=
  if response.status = 200 then decodeApplied response.body
  else decodeNoOp response.body

/-- Distinguish variants using the already persisted wakeApplied field. -/
def decodeTyped (response : Response) : Option Outcome :=
  if response.status = 200 then
    if response.body.wakeApplied then decodeApplied response.body
    else decodeNoOp response.body
  else none

theorem typed_roundtrip (outcome : Outcome) :
    decodeTyped (encode outcome) = some outcome := by
  cases outcome <;> rfl

theorem historical_applied_roundtrip (value : Applied) :
    decodeHistorical (encode (.applied value)) = some (.applied value) := by
  rfl

/-- Historical code cannot replay any normally encoded NoOp as its original outcome. -/
theorem historical_noop_replay_fails (binding : Binding) (reason : NoOpReason) :
    decodeHistorical (encode (.noOp binding reason)) = none := by
  rfl

theorem historical_decoder_not_outcome_preserving :
    ¬ (∀ outcome, decodeHistorical (encode outcome) = some outcome) := by
  intro roundtrip
  have impossible := roundtrip (.noOp ⟨1, 2⟩ .alreadyDue)
  cases impossible

theorem typed_noop_replay_preserves_reason (binding : Binding) (reason : NoOpReason) :
    decodeTyped (encode (.noOp binding reason)) = some (.noOp binding reason) := by
  exact typed_roundtrip _

structure Business where
  instanceId : Nat
  currentVisit : Option Nat
  owner : Nat
  closed : Bool
  version : Nat
  eventCount : Nat
  eligibilityFactCount : Nat
  deriving DecidableEq, Repr

structure Activation where
  visit : Nat
  closed : Bool
  nextEligibleAt : Option Nat
  deriving DecidableEq, Repr

structure Request where
  scope : Scope
  requestHash : Nat
  binding : Binding
  expectedVersion : Nat
  now : Nat
  deriving DecidableEq, Repr

structure Receipt where
  requestHash : Nat
  response : Response
  deriving DecidableEq, Repr

structure State where
  business : Business
  activation : Activation
  receipts : Scope → Option Receipt

/-- Source lookup precondition for the state-to-Rust correspondence, not an
extra check in `run`. A historical target Visit may differ from currentVisit. -/
def Resolved (state : State) (request : Request) : Prop :=
  state.business.instanceId = request.binding.instanceId ∧
  state.activation.visit = request.binding.visit

inductive Decision where
  | apply
  | noOp (reason : NoOpReason)
  deriving DecidableEq, Repr

/-- Same ordered NoOp guards as wake_transaction.rs, after activation resolution. -/
def classify (state : State) (request : Request) : Decision :=
  if state.business.closed then .noOp .instanceClosed
  else if state.activation.closed then .noOp .activationClosed
  else if state.business.currentVisit ≠ some request.binding.visit then .noOp .visitNotCurrent
  else if request.expectedVersion ≠ state.business.version then .noOp .versionMismatch
  else match state.activation.nextEligibleAt with
    | none => .noOp .activationClosed
    | some instant => if instant ≤ request.now then .noOp .alreadyDue else .apply

def maxStateVersion : Nat := 2147483647

/-- A fresh AGENT activation can be due at creation; no malformed data is needed. -/
def dueWitnessState : State :=
  ⟨⟨1, some 2, 7, false, 1, 1, 0⟩, ⟨2, false, some 10⟩, fun _ => none⟩

def dueWitnessRequest : Request :=
  ⟨⟨5, 11⟩, 123, ⟨1, 2⟩, 1, 10⟩

/-- A nonempty positive domain: a future effective time in a resolved pair.
Producing that future time (for example SCHEDULER_DEFER) is outside this model. -/
def futureWitnessState : State :=
  { dueWitnessState with activation := ⟨2, false, some 20⟩ }

theorem resolved_future_witness_is_applicable :
    Resolved futureWitnessState dueWitnessRequest ∧
    classify futureWitnessState dueWitnessRequest = .apply ∧
    futureWitnessState.business.version < maxStateVersion := by
  exact ⟨⟨rfl, rfl⟩, rfl, by decide⟩

theorem due_noop_counterexample_is_reachable_in_classifier :
    classify dueWitnessState dueWitnessRequest = .noOp .alreadyDue ∧
    decodeHistorical (encode (.noOp dueWitnessRequest.binding .alreadyDue)) = none := by
  exact ⟨rfl, rfl⟩

def appliedState (state : State) (request : Request) : State :=
  { state with
    business := { state.business with
      version := state.business.version + 1
      eventCount := state.business.eventCount + 1
      eligibilityFactCount := state.business.eligibilityFactCount + 1 }
    activation := { state.activation with nextEligibleAt := some request.now } }

def appliedOutcome (state : State) (request : Request) : Outcome :=
  .applied ⟨request.binding, state.business.version + 1,
    state.business.version + 1, request.now⟩

theorem apply_preserves_resolved_binding (state : State) (request : Request)
    (resolved : Resolved state request) :
    Resolved (appliedState state request) request := by
  exact resolved

/-- This changes the selected receipt cell, not the business state. -/
def record (state : State) (request : Request) (outcome : Outcome) : State :=
  { state with receipts := fun scope =>
      if scope = request.scope then some ⟨request.requestHash, encode outcome⟩
      else state.receipts scope }

inductive Reply where
  | returned (outcome : Outcome) (replayed : Bool)
  | conflict
  | corruptReceipt
  | versionOverflow
  deriving DecidableEq, Repr

/-- Candidate atomic transition; it replays a completed receipt before new guards. -/
def run (state : State) (request : Request) : State × Reply :=
  match state.receipts request.scope with
  | some receipt =>
      if receipt.requestHash = request.requestHash then
        match decodeTyped receipt.response with
        | some outcome => (state, .returned outcome true)
        | none => (state, .corruptReceipt)
      else (state, .conflict)
  | none =>
      match classify state request with
      | .noOp reason =>
          let outcome := Outcome.noOp request.binding reason
          (record state request outcome, .returned outcome false)
      | .apply =>
          if state.business.version < maxStateVersion then
            let outcome := appliedOutcome state request
            (record (appliedState state request) request outcome, .returned outcome false)
          else (state, .versionOverflow)

theorem record_preserves_business (state : State) (request : Request) (outcome : Outcome) :
    (record state request outcome).business = state.business := by
  rfl

theorem record_stores_original_outcome (state : State) (request : Request) (outcome : Outcome) :
    (record state request outcome).receipts request.scope =
      some ⟨request.requestHash, encode outcome⟩ := by
  simp [record]

theorem record_preserves_other_scope (state : State) (request : Request) (outcome : Outcome)
    (scope : Scope) (different : scope ≠ request.scope) :
    (record state request outcome).receipts scope = state.receipts scope := by
  simp [record, different]

/-- Applies to any business snapshot, even if its visit/version changed after receipt creation. -/
theorem same_request_replays_without_writes (state : State) (request : Request) (outcome : Outcome)
    (stored : state.receipts request.scope = some ⟨request.requestHash, encode outcome⟩) :
    run state request = (state, .returned outcome true) := by
  simp [run, stored, typed_roundtrip]

theorem different_hash_conflicts_without_writes (state : State) (request : Request)
    (receipt : Receipt)
    (stored : state.receipts request.scope = some receipt)
    (different : receipt.requestHash ≠ request.requestHash) :
    run state request = (state, .conflict) := by
  simp [run, stored, different]

theorem fresh_noop_changes_only_receipt (state : State) (request : Request) (reason : NoOpReason)
    (fresh : state.receipts request.scope = none)
    (noop : classify state request = .noOp reason) :
    run state request =
      (record state request (.noOp request.binding reason),
        .returned (.noOp request.binding reason) false) := by
  simp [run, fresh, noop]

theorem fresh_apply_records_once (state : State) (request : Request)
    (fresh : state.receipts request.scope = none)
    (applicable : classify state request = .apply)
    (withinBound : state.business.version < maxStateVersion) :
    run state request =
      (record (appliedState state request) request (appliedOutcome state request),
        .returned (appliedOutcome state request) false) := by
  simp [run, fresh, applicable, withinBound]

theorem overflow_does_not_change_state (state : State) (request : Request)
    (fresh : state.receipts request.scope = none)
    (applicable : classify state request = .apply)
    (atLimit : ¬ state.business.version < maxStateVersion) :
    run state request = (state, .versionOverflow) := by
  simp [run, fresh, applicable, atLimit]

theorem apply_increments_version_event_and_fact_together (state : State) (request : Request) :
    (appliedState state request).business.version = state.business.version + 1 ∧
    (appliedState state request).business.eventCount = state.business.eventCount + 1 ∧
    (appliedState state request).business.eligibilityFactCount =
      state.business.eligibilityFactCount + 1 := by
  exact ⟨rfl, rfl, rfl⟩

theorem apply_preserves_instance_visit_owner (state : State) (request : Request) :
    (appliedState state request).business.instanceId = state.business.instanceId ∧
    (appliedState state request).business.currentVisit = state.business.currentVisit ∧
    (appliedState state request).business.owner = state.business.owner := by
  exact ⟨rfl, rfl, rfl⟩

/-- Replay the same request any finite number of times and retain every reply. -/
def replayTrace : Nat → State → Request → State × List Reply
  | 0, state, _ => (state, [])
  | count + 1, state, request =>
      let (next, reply) := run state request
      let (final, replies) := replayTrace count next request
      (final, reply :: replies)

/-- Induction over arbitrary finite retry traces, not a finite test enumeration. -/
theorem any_number_of_replays_preserves_state_and_outcome
    (state : State) (request : Request) (outcome : Outcome)
    (stored : state.receipts request.scope = some ⟨request.requestHash, encode outcome⟩)
    (count : Nat) :
    replayTrace count state request =
      (state, List.replicate count (.returned outcome true)) := by
  induction count with
  | zero => rfl
  | succ count ih =>
      simp [replayTrace, same_request_replays_without_writes state request outcome stored,
        ih, List.replicate_succ]

theorem apply_then_arbitrary_replays_never_duplicate_business_effects
    (state : State) (request : Request) (count : Nat)
    (fresh : state.receipts request.scope = none)
    (applicable : classify state request = .apply)
    (withinBound : state.business.version < maxStateVersion) :
    (replayTrace count (run state request).1 request).1.business =
      (appliedState state request).business := by
  rw [fresh_apply_records_once state request fresh applicable withinBound]
  have trace := any_number_of_replays_preserves_state_and_outcome
    (record (appliedState state request) request (appliedOutcome state request))
    request (appliedOutcome state request)
    (record_stores_original_outcome _ _ _) count
  simp only [trace, record_preserves_business]

theorem noop_then_arbitrary_replays_never_change_business
    (state : State) (request : Request) (reason : NoOpReason) (count : Nat)
    (fresh : state.receipts request.scope = none)
    (noop : classify state request = .noOp reason) :
    (replayTrace count (run state request).1 request).1.business = state.business := by
  rw [fresh_noop_changes_only_receipt state request reason fresh noop]
  have trace := any_number_of_replays_preserves_state_and_outcome
    (record state request (.noOp request.binding reason)) request
    (.noOp request.binding reason) (record_stores_original_outcome _ _ _) count
  simp only [trace, record_preserves_business]

end SvcWorkflow.WakeReceipt
