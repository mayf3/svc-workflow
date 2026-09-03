-- Migration 0023: VISIT_ACTIVATION_V1 semantic model + canonical activation facts
--
-- Implements the storage layer of SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1 under
-- accepted SVC_WORKFLOW_ARCHITECTURE_V0_4_0 (Slice D phase 1):
--
--   1. semantic_model_version = 3 (VISIT_ACTIVATION_V1) becomes a defined,
--      accepted value. Values 1 (Legacy) and 2 (Minimal, descriptive) keep
--      their exact meaning; 2 is NOT aliased to 3.
--   2. Instances carry an immutable semantic_model_version with a
--      database-enforced equality to their Definition Version.
--   3. Canonical activation fact families (append-only, trigger-enforced):
--      workflow_activations (exactly one per Node Visit, DB-enforced),
--      workflow_activation_closures, workflow_dispatch_eligibility_events.
--
-- No existing row's meaning is changed. Legacy instances never gain
-- activation rows.

-- ------------------------------------------------------------------
-- 1. Semantic model 3 (VISIT_ACTIVATION_V1)
-- ------------------------------------------------------------------

ALTER TABLE workflow_definition_versions
    DROP CONSTRAINT workflow_definition_versions_semantic_model_version_check;

ALTER TABLE workflow_definition_versions
    ADD CONSTRAINT workflow_definition_versions_semantic_model_version_check
    CHECK (semantic_model_version IN (1, 2, 3));

-- DB-enforced equality between the Instance discriminator and its immutable
-- Definition Version: (definition_version_id, semantic_model_version) must
-- match exactly.
ALTER TABLE workflow_definition_versions
    ADD CONSTRAINT uq_definition_version_model
    UNIQUE (definition_version_id, semantic_model_version);

ALTER TABLE workflow_instances
    ADD COLUMN semantic_model_version SMALLINT NOT NULL DEFAULT 1;

-- Backfill from the immutable Definition Version (all existing instances
-- inherit the model of the version they already pin).
UPDATE workflow_instances wi
   SET semantic_model_version = wdv.semantic_model_version
  FROM workflow_definition_versions wdv
 WHERE wdv.definition_version_id = wi.definition_version_id
   AND wi.semantic_model_version <> wdv.semantic_model_version;

ALTER TABLE workflow_instances
    ADD CONSTRAINT workflow_instances_semantic_model_version_check
    CHECK (semantic_model_version IN (1, 2, 3));

-- Composite FK: the instance's model must equal its version's model.
ALTER TABLE workflow_instances
    ADD CONSTRAINT fk_instance_definition_version_model
    FOREIGN KEY (definition_version_id, semantic_model_version)
    REFERENCES workflow_definition_versions (definition_version_id, semantic_model_version);

-- The discriminator is immutable after creation (same style as 0006).
CREATE OR REPLACE FUNCTION fn_prevent_instance_model_change()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.semantic_model_version IS DISTINCT FROM OLD.semantic_model_version THEN
        RAISE EXCEPTION
            'immutable field: workflow_instances.semantic_model_version cannot be modified'
            USING ERRCODE = '23000';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_instance_semantic_model_immutable
    BEFORE UPDATE ON workflow_instances
    FOR EACH ROW EXECUTE FUNCTION fn_prevent_instance_model_change();

-- ------------------------------------------------------------------
-- 2. node_type gains TASK (VISIT_ACTIVATION_V1 node kinds: TASK | TERMINAL)
-- ------------------------------------------------------------------

ALTER TYPE node_type ADD VALUE IF NOT EXISTS 'TASK';

-- ------------------------------------------------------------------
-- 3. Canonical activation facts
-- ------------------------------------------------------------------

CREATE TYPE activation_kind AS ENUM ('HUMAN_WORK_ITEM', 'DISPATCH_INTENT');

CREATE TABLE workflow_activations (
    activation_id            UUID           NOT NULL PRIMARY KEY,
    workflow_instance_id     UUID           NOT NULL REFERENCES workflow_instances(workflow_instance_id),
    node_visit_id            UUID           NOT NULL REFERENCES workflow_node_visits(node_visit_id),
    activation_kind          activation_kind NOT NULL,
    owner_principal_id       UUID           NOT NULL REFERENCES principals(principal_id),
    activation_at            TIMESTAMPTZ    NOT NULL,
    initial_next_eligible_at TIMESTAMPTZ,
    command_id               UUID           NOT NULL,
    created_at               TIMESTAMPTZ    NOT NULL DEFAULT now(),

    -- Exactly-one canonical activation per Node Visit (mechanical, not
    -- best-effort): one Visit can never have two activations of any kind.
    CONSTRAINT uq_activation_node_visit UNIQUE (node_visit_id),
    -- DISPATCH_INTENT carries the server-authored initial wait timestamp;
    -- HUMAN_WORK_ITEM has none.
    CONSTRAINT chk_activation_kind_eligibility CHECK (
        (activation_kind = 'DISPATCH_INTENT' AND initial_next_eligible_at IS NOT NULL)
        OR
        (activation_kind = 'HUMAN_WORK_ITEM' AND initial_next_eligible_at IS NULL)
    )
);

CREATE INDEX idx_activation_instance
    ON workflow_activations (workflow_instance_id);

CREATE INDEX idx_activation_kind
    ON workflow_activations (activation_kind)
    WHERE activation_kind = 'DISPATCH_INTENT';

CREATE TABLE workflow_activation_closures (
    activation_id UUID        NOT NULL PRIMARY KEY REFERENCES workflow_activations(activation_id),
    closed_at     TIMESTAMPTZ NOT NULL,
    -- 1..128: closed set of lifecycle reasons (TRANSITIONED, CANCELLED,
    -- ADMIN_MOVE, ADMIN_TERMINATE, ...)
    closure_reason TEXT       NOT NULL CHECK (char_length(closure_reason) >= 1 AND char_length(closure_reason) <= 128),
    command_id    UUID        NOT NULL,
    event_id      UUID,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- The closure may be written before its Event in the same transaction
-- (activation closure precedes the Event insert in the command order), so
-- the FK is deferred to commit time -- same style as the other composite
-- FKs in this schema.
ALTER TABLE workflow_activation_closures
    ADD CONSTRAINT fk_activation_closure_event
    FOREIGN KEY (event_id) REFERENCES workflow_events(event_id)
    DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE workflow_dispatch_eligibility_events (
    eligibility_event_id      UUID        NOT NULL PRIMARY KEY,
    activation_id             UUID        NOT NULL REFERENCES workflow_activations(activation_id),
    previous_next_eligible_at TIMESTAMPTZ NOT NULL,
    new_next_eligible_at      TIMESTAMPTZ NOT NULL,
    -- 1..64 closed cause classes (WAKE, SCHEDULER_DEFER, ...). The concrete
    -- value set is validated at the API layer per the implementation Spec.
    cause_class               TEXT        NOT NULL CHECK (char_length(cause_class) >= 1 AND char_length(cause_class) <= 64),
    command_id                UUID        NOT NULL,
    created_at                TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_eligibility_activation
    ON workflow_dispatch_eligibility_events (activation_id, created_at, eligibility_event_id);

-- Append-only enforcement for all three fact families (same style as 0006).
CREATE TRIGGER trg_workflow_activations_immutable
    BEFORE UPDATE OR DELETE ON workflow_activations
    FOR EACH ROW EXECUTE FUNCTION fn_prevent_modification();

CREATE TRIGGER trg_activation_closures_immutable
    BEFORE UPDATE OR DELETE ON workflow_activation_closures
    FOR EACH ROW EXECUTE FUNCTION fn_prevent_modification();

CREATE TRIGGER trg_dispatch_eligibility_immutable
    BEFORE UPDATE OR DELETE ON workflow_dispatch_eligibility_events
    FOR EACH ROW EXECUTE FUNCTION fn_prevent_modification();

-- NOTE: EXPECTED_MIGRATION_VERSION / SCHEMA_VERSION must be updated from
-- 22/0022 to 23/0023 in src/http/mod.rs for /readyz to return "ready".
