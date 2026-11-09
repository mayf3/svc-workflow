# Svc plan: workflow prerequisite orchestration

**Status:** proposed implementation plan only. No code, migration, acceptance,
merge, production apply, or live execution is authorized by this plan.

**Target:** Svc `c8937e567a429160779ee7db0cb7b53efbe8ed3e`.
**Authority:** proposed Svc/Core companion Specs, contingent on exact-head
acceptance of Product Boundary V9, Architecture V0_4_4, and WEP V2. The
paired acceptance record must pin both reviewed companion heads.

## Bounded phases

1. Add strict root-schema prerequisite opt-in and exact Context-value checks;
   revalidate same-Domain preexisting source/version/terminal transition in the
   create transaction. Require current dependent-Domain Owner plus fresh source
   visibility. Exact terminal Event/Submission proof is checked before active
   Visit reachability; without proof only an active reachable transition may
   wait. Test both serial transaction orderings. Reuse Context and canonical
   Instance facts; no migration or free-form predicate engine.
2. Compose existing create and GCR bind transaction helpers into one
   SERIALIZABLE create-and-bind command with one idempotency receipt. Prove
   all-or-nothing visibility, rollback, duplicate/replay, and unknown readback.
3. Derive typed readiness from stored Context and committed source facts.
   Expose typed readiness plus receipt IDs/digest only to the currently authorized
   dependent assignee/Domain Owner; status is visible for ordinary no-case
   workflows. Scheduler receives only `readinessStatus` with work coordinates.
   Add `GET /internal/v1/dispatch-intents/ready` and Core
   `workflow_dispatch_intents.list_ready`, filtering prerequisite READY AND
   time due before pagination. Preserve raw VAI API and WEP time semantics.
4. Run focused database/read/query tests and paired isolated Core acceptance.
   In the one-trigger native run, let the Owner first create an ordinary
   supplier via existing `create_instance`, then atomically create/bind the
   governance child using that returned upstream Instance ID while the
   supplier's normal Agent-owned work is still pending. Separately create a
   non-GCR dependent through ordinary `create_instance`. Demonstrate WAITING
   with zero attempt/Router delivery, normal supplier completion, READY receipt
   projection after restart/poll, and the GCR atomic create-bind path. This is
   fixture sequencing only, not a new product-selection rule.

## Touch points and stop gates

Expected Svc surfaces are the Assistance HTTP/application transaction,
transaction-scoped create/GCR helpers, Context validation, current-task and
Instance readiness reads, and a separate dispatchable-intent query/route.
Existing event, Submission, activation, receipt and Context records should
suffice; no table migration is planned. Fresh Core pre-plan dependent tuple,
runtime-config, Domain and authorization checks remain required; no Core wait
ledger, new lease, outbox, or raw-time fallback is permitted. CLOSED/UNKNOWN
remain visible stops; no recovery or indefinite-wait guarantee.

Stop if the transaction cannot prevent an unbound child from becoming visible,
if the source reference cannot be locked/revalidated, if readiness cannot be
filtered before page limits, or if ordinary create would need a new authority
scope. UNKNOWN remains a stop state; no automatic recovery or indefinite-wait
success claim. Production apply remains separately gated.
