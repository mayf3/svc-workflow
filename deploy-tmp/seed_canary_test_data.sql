-- Canary test data seed script
--
-- Idempotent: all INSERTs use ON CONFLICT DO NOTHING so that re-running
-- this script does not create duplicate rows.  Each workflow instance now
-- also inserts its initial WORKFLOW_INSTANCE_CREATED event so that the
-- event-based consistency checks (event_count == workflow_state_version,
-- min_event_sequence == 1, etc.) pass without manual intervention.

-- ============================================================
-- Principals
-- ============================================================
INSERT INTO principals (principal_id, principal_type, display_name, email, enabled)
VALUES ('aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa', 'AGENT', 'Auth Canary Test Principal', 'auth-canary@test', TRUE)
ON CONFLICT (principal_id) DO NOTHING;

INSERT INTO principals (principal_id, principal_type, display_name, email, enabled)
VALUES ('bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb', 'AGENT', 'Other Test Principal', 'other@test', TRUE)
ON CONFLICT (principal_id) DO NOTHING;

-- ============================================================
-- Domain
-- ============================================================
INSERT INTO domains (domain_id, domain_key, display_name, enabled)
VALUES ('cccccccc-cccc-4ccc-cccc-cccccccccccc', 'canary-test-domain', 'Canary Test Domain', TRUE)
ON CONFLICT (domain_id) DO NOTHING;

-- ============================================================
-- Domain role bindings
-- ============================================================
INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
VALUES (gen_random_uuid(), 'cccccccc-cccc-4ccc-cccc-cccccccccccc', 'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa', 'MEMBER', TRUE)
ON CONFLICT (domain_id, principal_id, role_key) DO NOTHING;

INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
VALUES (gen_random_uuid(), 'cccccccc-cccc-4ccc-cccc-cccccccccccc', 'bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb', 'MEMBER', TRUE)
ON CONFLICT (domain_id, principal_id, role_key) DO NOTHING;

-- ============================================================
-- Workflow definition
-- ============================================================
INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name)
VALUES ('dddddddd-dddd-4ddd-dddd-dddddddddddd', 'cccccccc-cccc-4ccc-cccc-cccccccccccc', 'canary-review-def', 'Canary Review')
ON CONFLICT (workflow_definition_id) DO NOTHING;

INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema)
VALUES ('eeeeeeee-eeee-4eee-eeee-eeeeeeeeeeee', 'dddddddd-dddd-4ddd-dddd-dddddddddddd', 1, 'DRAFT', '{"type":"object"}'::jsonb)
ON CONFLICT (definition_version_id) DO NOTHING;

-- Insert node definitions while version is still DRAFT.
-- Use a DO block with an existence check instead of ON CONFLICT because the
-- graph_immutability trigger blocks INSERTs when the version is PUBLISHED,
-- which would cause ON CONFLICT to fail before it can detect the conflict.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM workflow_node_definitions
        WHERE node_id = 'ffffffff-ffff-4fff-ffff-ffffffffffff'
    ) THEN
        INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id)
        VALUES ('ffffffff-ffff-4fff-ffff-ffffffffffff', 'eeeeeeee-eeee-4eee-eeee-eeeeeeeeeeee', 'review', 'Review', 0, 'NORMAL', 'FIXED_PRINCIPAL', 'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa');
    END IF;
END;
$$;

-- Now publish the version (idempotent: only updates if status is DRAFT)
UPDATE workflow_definition_versions
SET version_status = 'PUBLISHED'
WHERE definition_version_id = 'eeeeeeee-eeee-4eee-eeee-eeeeeeeeeeee'
  AND version_status = 'DRAFT';

-- ============================================================
-- Instance 1: visible to canary principal
--   assigned to 'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa'
-- ============================================================
INSERT INTO workflow_instances (
    workflow_instance_id, domain_id, definition_version_id,
    created_by_principal_id
)
VALUES (
    '11111111-1111-4111-1111-111111111111',
    'cccccccc-cccc-4ccc-cccc-cccccccccccc',
    'eeeeeeee-eeee-4eee-eeee-eeeeeeeeeeee',
    'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa'
)
ON CONFLICT (workflow_instance_id) DO NOTHING;

INSERT INTO workflow_context_revisions (
    context_revision_id, workflow_instance_id, revision_number,
    previous_revision_id, payload, payload_digest, created_by_principal_id
)
VALUES (
    '22222222-2222-4222-2222-222222222222',
    '11111111-1111-4111-1111-111111111111',
    1, NULL,
    '{}'::jsonb,
    '44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a',
    'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa'
)
ON CONFLICT (context_revision_id) DO NOTHING;

INSERT INTO workflow_node_visits (
    node_visit_id, workflow_instance_id, node_id, visit_number,
    assignee_principal_id
)
VALUES (
    '33333333-3333-4333-3333-333333333333',
    '11111111-1111-4111-1111-111111111111',
    'ffffffff-ffff-4fff-ffff-ffffffffffff',
    1,
    'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa'
)
ON CONFLICT (node_visit_id) DO NOTHING;

-- Initial WORKFLOW_INSTANCE_CREATED event (matches runtime semantics)
INSERT INTO workflow_events (
    event_id, workflow_instance_id, event_sequence, event_schema_version,
    event_type, source_node_visit_id, target_node_visit_id,
    context_revision_id, event_data, event_data_digest,
    actor_principal_id, old_workflow_state_version, new_workflow_state_version
)
VALUES (
    '00000000-0000-4000-a000-000000000001',
    '11111111-1111-4111-1111-111111111111',
    1, 'v1',
    'WORKFLOW_INSTANCE_CREATED',
    NULL,
    '33333333-3333-4333-3333-333333333333',
    '22222222-2222-4222-2222-222222222222',
    '{
        "definition_version_id": "eeeeeeee-eeee-4eee-eeee-eeeeeeeeeeee",
        "definition_digest": "b66962237378fcca2ac804b4a821cc0b36803a3b325d677d6aafd90a7eab4f99",
        "initial_node_id": "ffffffff-ffff-4fff-ffff-ffffffffffff",
        "assignee_resolution_type": "FIXED_PRINCIPAL"
    }'::jsonb,
    'e0d94e66307dc20d8e32cb7788399d87729ba01fd4d1755e4fbffac8402e30bb',
    'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa',
    0, 1
)
ON CONFLICT (event_id) DO NOTHING;

UPDATE workflow_instances
SET current_context_revision_id = '22222222-2222-4222-2222-222222222222',
    current_node_visit_id = '33333333-3333-4333-3333-333333333333'
WHERE workflow_instance_id = '11111111-1111-4111-1111-111111111111'
  AND current_context_revision_id IS NULL;

-- ============================================================
-- Instance 2: hidden from canary principal
--   assigned to 'bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb'
-- ============================================================
INSERT INTO workflow_instances (
    workflow_instance_id, domain_id, definition_version_id,
    created_by_principal_id
)
VALUES (
    '44444444-4444-4444-4444-444444444444',
    'cccccccc-cccc-4ccc-cccc-cccccccccccc',
    'eeeeeeee-eeee-4eee-eeee-eeeeeeeeeeee',
    'bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb'
)
ON CONFLICT (workflow_instance_id) DO NOTHING;

INSERT INTO workflow_context_revisions (
    context_revision_id, workflow_instance_id, revision_number,
    previous_revision_id, payload, payload_digest, created_by_principal_id
)
VALUES (
    '55555555-5555-4555-5555-555555555555',
    '44444444-4444-4444-4444-444444444444',
    1, NULL,
    '{}'::jsonb,
    '44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a',
    'bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb'
)
ON CONFLICT (context_revision_id) DO NOTHING;

INSERT INTO workflow_node_visits (
    node_visit_id, workflow_instance_id, node_id, visit_number,
    assignee_principal_id
)
VALUES (
    '66666666-6666-4666-6666-666666666666',
    '44444444-4444-4444-4444-444444444444',
    'ffffffff-ffff-4fff-ffff-ffffffffffff',
    1,
    'bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb'
)
ON CONFLICT (node_visit_id) DO NOTHING;

-- Initial WORKFLOW_INSTANCE_CREATED event (matches runtime semantics)
INSERT INTO workflow_events (
    event_id, workflow_instance_id, event_sequence, event_schema_version,
    event_type, source_node_visit_id, target_node_visit_id,
    context_revision_id, event_data, event_data_digest,
    actor_principal_id, old_workflow_state_version, new_workflow_state_version
)
VALUES (
    '00000000-0000-4000-a000-000000000002',
    '44444444-4444-4444-4444-444444444444',
    1, 'v1',
    'WORKFLOW_INSTANCE_CREATED',
    NULL,
    '66666666-6666-4666-6666-666666666666',
    '55555555-5555-4555-5555-555555555555',
    '{
        "definition_version_id": "eeeeeeee-eeee-4eee-eeee-eeeeeeeeeeee",
        "definition_digest": "b66962237378fcca2ac804b4a821cc0b36803a3b325d677d6aafd90a7eab4f99",
        "initial_node_id": "ffffffff-ffff-4fff-ffff-ffffffffffff",
        "assignee_resolution_type": "FIXED_PRINCIPAL"
    }'::jsonb,
    'e0d94e66307dc20d8e32cb7788399d87729ba01fd4d1755e4fbffac8402e30bb',
    'bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb',
    0, 1
)
ON CONFLICT (event_id) DO NOTHING;

UPDATE workflow_instances
SET current_context_revision_id = '55555555-5555-4555-5555-555555555555',
    current_node_visit_id = '66666666-6666-4666-6666-666666666666'
WHERE workflow_instance_id = '44444444-4444-4444-4444-444444444444'
  AND current_context_revision_id IS NULL;
