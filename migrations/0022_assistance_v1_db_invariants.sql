-- Migration 0022: Workflow Assistance V1 DB-level invariants
--
-- Closes the two DB-integrity gaps surfaced by the independent Assistance V1
-- review (blockers 1 and 2). This migration is purely additive over 0021: it
-- adds a BEFORE INSERT trigger (initial-state + lifecycle binding) and a
-- DEFERRED constraint trigger (command-receipt / actor / status integrity).
--
-- It does NOT change Assistance business semantics, Instance status, assignee,
-- Human/HR boundaries, or Auth. Payload/digest correctness is intentionally
-- NOT re-validated here (see the rollout report for the rationale: there is no
-- DB-side RFC 8785/JCS canonicalizer, so a second canonicalization algorithm
-- would be riskier than the value it adds; payload+digest immutability is
-- already enforced by the existing 0021 BEFORE UPDATE trigger).

-- ============================================================
-- Blocker 1: initial-state + Visit/lifecycle invariants on INSERT
-- ============================================================
--
-- The 0021 BEFORE UPDATE trigger only governs transitions. A direct INSERT could
-- previously create an impossible AssistanceCase history (HUMAN_REQUIRED /
-- RESOLVED / VOIDED as the first row, or a case bound to a non-current /
-- terminal / cancelled / archived visit). This BEFORE INSERT trigger closes
-- that, and it takes the instance row lock itself (FOR UPDATE OF wi) so that
-- even a direct SQL INSERT serializes against transition / cancel / archive on
-- the same instance row — closing the read-skew window a plain SELECT would
-- leave. The normal request_assistance path already holds this lock, so
-- re-acquiring it within the same transaction is a no-op (lock order is
-- unchanged: every writer locks the instance row first).

CREATE OR REPLACE FUNCTION fn_check_assistance_case_insert()
RETURNS TRIGGER AS $$
DECLARE
    v_current_visit UUID;
    v_cancelled     BOOLEAN;
    v_archived_at   TIMESTAMPTZ;
    v_assignee      UUID;
    v_node_type     TEXT;
BEGIN
    -- A brand-new case can only start at OWNER_PENDING; the request stage is
    -- the only stage that may be populated on insert.
    IF NEW.status <> 'OWNER_PENDING' THEN
        RAISE EXCEPTION 'assistance case must start as OWNER_PENDING, got %',
            NEW.status
            USING ERRCODE = '23000';
    END IF;
    IF NEW.escalation_command_id IS NOT NULL
       OR NEW.resolution_command_id IS NOT NULL
       OR NEW.voided_by_command_id IS NOT NULL THEN
        RAISE EXCEPTION 'assistance case insert may not pre-set escalation/resolution/void command refs'
            USING ERRCODE = '23000';
    END IF;

    -- Bind to the instance's *current* visit and require the instance to be
    -- live and non-terminal. FOR UPDATE OF wi serializes this INSERT against
    -- any concurrent transition/cancel/archive on the same instance.
    SELECT wi.current_node_visit_id, wi.cancelled, wi.archived_at,
           nv.assignee_principal_id, nd.node_type::text
      INTO v_current_visit, v_cancelled, v_archived_at, v_assignee, v_node_type
      FROM workflow_instances wi
      LEFT JOIN workflow_node_visits nv
             ON nv.node_visit_id = wi.current_node_visit_id
      LEFT JOIN workflow_node_definitions nd
             ON nd.node_id = nv.node_id
     WHERE wi.workflow_instance_id = NEW.workflow_instance_id
     FOR UPDATE OF wi;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'assistance case references an unknown workflow_instance'
            USING ERRCODE = '23000';
    END IF;
    IF NEW.node_visit_id IS DISTINCT FROM v_current_visit THEN
        RAISE EXCEPTION 'assistance case node_visit_id must equal instance.current_node_visit_id'
            USING ERRCODE = '23000';
    END IF;
    IF v_cancelled THEN
        RAISE EXCEPTION 'cannot open an assistance case on a cancelled instance'
            USING ERRCODE = '23000';
    END IF;
    IF v_archived_at IS NOT NULL THEN
        RAISE EXCEPTION 'cannot open an assistance case on an archived instance'
            USING ERRCODE = '23000';
    END IF;
    -- Canonical terminal marker (migration 0010): a terminal visit has no
    -- assignee; node_type = 'TERMINAL' is checked as the belt-and-suspenders.
    IF v_assignee IS NULL OR v_node_type = 'TERMINAL' THEN
        RAISE EXCEPTION 'cannot open an assistance case on a terminal visit'
            USING ERRCODE = '23000';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_assistance_case_insert
    BEFORE INSERT ON workflow_assistance_cases
    FOR EACH ROW EXECUTE FUNCTION fn_check_assistance_case_insert();

-- ============================================================
-- Blocker 2: command-receipt / actor / status integrity
-- ============================================================
--
-- A direct INSERT/UPDATE could previously bind a stage command_id to a receipt
-- of the wrong command_type, the wrong actor, or a receipt that never reached
-- COMPLETED, and could reuse one command_id across stages. The write path
-- acquires the receipt as PROCESSING and only completes it later in the same
-- transaction, so an immediate check would reject the legitimate path. This
-- constraint trigger is therefore DEFERRABLE INITIALLY DEFERRED: it fires at
-- COMMIT, once every receipt has reached its final COMPLETED state.
--
-- Cross-stage reuse (e.g. request_command_id == resolution_command_id) is
-- prevented without an extra cross-column constraint: each receipt has exactly
-- one command_type, and each stage below requires its own distinct command_type,
-- so the same command_id cannot satisfy two stages. Per-column uniqueness is
-- already provided by 0021 (UNIQUE(request_command_id) and the partial unique
-- indexes on escalation_command_id / resolution_command_id).

CREATE OR REPLACE FUNCTION fn_validate_assistance_command_refs()
RETURNS TRIGGER AS $$
DECLARE
    r_command_type TEXT;
    r_status       TEXT;
    r_principal    UUID;
BEGIN
    -- Request stage (always populated).
    SELECT cr.command_type, cr.receipt_status::text, cr.principal_id
      INTO r_command_type, r_status, r_principal
      FROM workflow_command_receipts cr
     WHERE cr.command_id = NEW.request_command_id;
    IF NOT FOUND
       OR r_command_type <> 'REQUEST_WORKFLOW_ASSISTANCE'
       OR r_principal IS DISTINCT FROM NEW.requested_by_principal_id
       OR r_status <> 'COMPLETED' THEN
        RAISE EXCEPTION
            'request_command_id % must reference a COMPLETED REQUEST_WORKFLOW_ASSISTANCE receipt by the requesting principal',
            NEW.request_command_id
            USING ERRCODE = '23000';
    END IF;

    -- Escalation stage (populated once the case is escalated).
    IF NEW.escalation_command_id IS NOT NULL THEN
        SELECT cr.command_type, cr.receipt_status::text, cr.principal_id
          INTO r_command_type, r_status, r_principal
          FROM workflow_command_receipts cr
         WHERE cr.command_id = NEW.escalation_command_id;
        IF NOT FOUND
           OR r_command_type <> 'ESCALATE_WORKFLOW_ASSISTANCE_TO_HUMAN'
           OR r_principal IS DISTINCT FROM NEW.escalated_by_principal_id
           OR r_status <> 'COMPLETED' THEN
            RAISE EXCEPTION
                'escalation_command_id % must reference a COMPLETED ESCALATE_WORKFLOW_ASSISTANCE_TO_HUMAN receipt by the escalating principal',
                NEW.escalation_command_id
                USING ERRCODE = '23000';
        END IF;
    END IF;

    -- Resolution stage (populated once the case is resolved).
    IF NEW.resolution_command_id IS NOT NULL THEN
        SELECT cr.command_type, cr.receipt_status::text, cr.principal_id
          INTO r_command_type, r_status, r_principal
          FROM workflow_command_receipts cr
         WHERE cr.command_id = NEW.resolution_command_id;
        IF NOT FOUND
           OR r_command_type <> 'RESOLVE_WORKFLOW_ASSISTANCE'
           OR r_principal IS DISTINCT FROM NEW.resolved_by_principal_id
           OR r_status <> 'COMPLETED' THEN
            RAISE EXCEPTION
                'resolution_command_id % must reference a COMPLETED RESOLVE_WORKFLOW_ASSISTANCE receipt by the resolving principal',
                NEW.resolution_command_id
                USING ERRCODE = '23000';
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE CONSTRAINT TRIGGER trg_assistance_command_refs
    AFTER INSERT OR UPDATE ON workflow_assistance_cases
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION fn_validate_assistance_command_refs();
