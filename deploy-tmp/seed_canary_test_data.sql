-- Create test principals
INSERT INTO principals (principal_id, principal_type, display_name, email, enabled)
VALUES ('aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa', 'AGENT', 'Auth Canary Test Principal', 'auth-canary@test', TRUE);
INSERT INTO principals (principal_id, principal_type, display_name, email, enabled)
VALUES ('bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb', 'AGENT', 'Other Test Principal', 'other@test', TRUE);

-- Create test domain
INSERT INTO domains (domain_id, domain_key, display_name, enabled)
VALUES ('cccccccc-cccc-4ccc-cccc-cccccccccccc', 'canary-test-domain', 'Canary Test Domain', TRUE);

-- Grant domain roles
INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
VALUES (gen_random_uuid(), 'cccccccc-cccc-4ccc-cccc-cccccccccccc', 'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa', 'MEMBER', TRUE);
INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
VALUES (gen_random_uuid(), 'cccccccc-cccc-4ccc-cccc-cccccccccccc', 'bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb', 'MEMBER', TRUE);

-- Create a simple workflow definition
INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name)
VALUES ('dddddddd-dddd-4ddd-dddd-dddddddddddd', 'cccccccc-cccc-4ccc-cccc-cccccccccccc', 'canary-review-def', 'Canary Review');

INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema)
VALUES ('eeeeeeee-eeee-4eee-eeee-eeeeeeeeeeee', 'dddddddd-dddd-4ddd-dddd-dddddddddddd', 1, 'PUBLISHED', '{"type":"object"}'::jsonb);

INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type)
VALUES ('ffffffff-ffff-4fff-ffff-ffffffffffff', 'eeeeeeee-eeee-4eee-eeee-eeeeeeeeeeee', 'review', 'Review', 0, 'HUMAN_TASK', 'FIXED_PRINCIPAL');

UPDATE workflow_node_definitions SET fixed_principal_id = 'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa' WHERE node_id = 'ffffffff-ffff-4fff-ffff-ffffffffffff';

-- Task visible to canary principal: assigned to 'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa'
INSERT INTO workflow_instances (workflow_instance_id, domain_id, definition_version_id, created_by_principal_id)
VALUES ('11111111-1111-4111-1111-111111111111', 'cccccccc-cccc-4ccc-cccc-cccccccccccc', 'eeeeeeee-eeee-4eee-eeee-eeeeeeeeeeee', 'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa');

INSERT INTO workflow_context_revisions (context_revision_id, workflow_instance_id, revision_number, context_payload, created_by_principal_id)
VALUES ('22222222-2222-4222-2222-222222222222', '11111111-1111-4111-1111-111111111111', 1, '{}'::jsonb, 'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa');

INSERT INTO workflow_node_visits (node_visit_id, workflow_instance_id, node_id, visit_number, assignee_principal_id)
VALUES ('33333333-3333-4333-3333-333333333333', '11111111-1111-4111-1111-111111111111', 'ffffffff-ffff-4fff-ffff-ffffffffffff', 1, 'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa');

UPDATE workflow_instances SET current_context_revision_id = '22222222-2222-4222-2222-222222222222', current_node_visit_id = '33333333-3333-4333-3333-333333333333' WHERE workflow_instance_id = '11111111-1111-4111-1111-111111111111';

-- Task hidden from canary principal: assigned to 'bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb'
INSERT INTO workflow_instances (workflow_instance_id, domain_id, definition_version_id, created_by_principal_id)
VALUES ('44444444-4444-4444-4444-444444444444', 'cccccccc-cccc-4ccc-cccc-cccccccccccc', 'eeeeeeee-eeee-4eee-eeee-eeeeeeeeeeee', 'bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb');

INSERT INTO workflow_context_revisions (context_revision_id, workflow_instance_id, revision_number, context_payload, created_by_principal_id)
VALUES ('55555555-5555-4555-5555-555555555555', '44444444-4444-4444-4444-444444444444', 1, '{}'::jsonb, 'bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb');

INSERT INTO workflow_node_visits (node_visit_id, workflow_instance_id, node_id, visit_number, assignee_principal_id)
VALUES ('66666666-6666-4666-6666-666666666666', '44444444-4444-4444-4444-444444444444', 'ffffffff-ffff-4fff-ffff-ffffffffffff', 1, 'bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb');

UPDATE workflow_instances SET current_context_revision_id = '55555555-5555-4555-5555-555555555555', current_node_visit_id = '66666666-6666-4666-6666-666666666666' WHERE workflow_instance_id = '44444444-4444-4444-4444-444444444444';
