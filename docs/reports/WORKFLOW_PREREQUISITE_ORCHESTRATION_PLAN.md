# Svc plan: workflow prerequisite orchestration

**Status:** proposed implementation plan only. No code, migration, acceptance,
merge, production apply, or live execution is authorized by this plan.

**Target:** Svc `c8937e567a429160779ee7db0cb7b53efbe8ed3e`.
**Authority:** `SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1` (proposed), paired
with Core `AGENT_CORE_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1` (proposed).

## Bounded phases

1. Add strict root-schema prerequisite opt-in and exact Context-value checks;
   revalidate same-Domain preexisting source/version/terminal transition in
   the create transaction. Reuse Context and canonical Instance facts; no
   migration or free-form predicate engine.
2. Compose existing create and GCR bind transaction helpers into one
   SERIALIZABLE create-and-bind command with one idempotency receipt. Prove
   all-or-nothing visibility, rollback, duplicate/replay, and unknown readback.
3. Derive typed readiness from stored Context and committed source facts.
   Expose it on existing authorized current-task/Instance reads and linked
   case detail. Add a separate scheduler query that filters READY before
   pagination; preserve raw VAI due and WEP semantics.
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
suffice; no table migration is planned.

Stop if the transaction cannot prevent an unbound child from becoming visible,
if the source reference cannot be locked/revalidated, if readiness cannot be
filtered before page limits, or if ordinary create would need a new authority
scope. UNKNOWN remains a stop state; no automatic recovery or indefinite-wait
success claim. Production apply remains separately gated.
