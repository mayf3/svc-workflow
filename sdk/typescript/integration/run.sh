#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
SVC_BIN="${SVC_WORKFLOW_BIN:-$REPO_DIR/target/debug/svc-workflow}"

for command in createdb psql curl jq python3 npm; do
    command -v "$command" >/dev/null 2>&1 || {
        echo "ERROR: $command is required" >&2
        exit 1
    }
done
if [ ! -x "$SVC_BIN" ]; then
    echo "ERROR: svc-workflow binary not found: $SVC_BIN" >&2
    echo "Build it with: cargo build --bin svc-workflow" >&2
    exit 1
fi

SUFFIX="sdk_$(date +%s)_$$"
DB_NAME="svc_workflow_$SUFFIX"
PORT=$(( ((RANDOM << 10) | (RANDOM & 0x3FF)) % 40000 + 20000 ))
OWNER_ID="11111111-1111-4111-8111-111111111111"
DOMAIN_ID="550e8400-e29b-41d4-a716-446655440000"
DEFINITION_ID="550e8400-e29b-41d4-a716-446655440002"
VERSION_ID="550e8400-e29b-41d4-a716-446655440001"
DRAFT_NODE_ID="66666666-6666-4666-8666-666666666666"
TERMINAL_NODE_ID="77777777-7777-4777-8777-777777777777"
TRANSITION_ID="88888888-8888-4888-8888-888888888888"
JWT_SECRET="sdk-integration-secret-at-least-32-bytes-long!!"

cleanup() {
    set +e
    if [ -n "${SERVER_PID:-}" ]; then
        kill "$SERVER_PID" 2>/dev/null
        wait "$SERVER_PID" 2>/dev/null
    fi
    psql -U postgres -c "DROP DATABASE IF EXISTS \"$DB_NAME\" WITH (FORCE)" >/dev/null 2>&1
}
trap cleanup EXIT

createdb -U postgres "$DB_NAME"
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/$DB_NAME"
export WORKFLOW_BIND_ADDR="127.0.0.1"
export WORKFLOW_PORT="$PORT"
export WORKFLOW_AUTH_MODE="test_hs256"
export WORKFLOW_JWT_SECRET="$JWT_SECRET"
export WORKFLOW_JWT_ISSUER="auth-service"
export WORKFLOW_JWT_AUDIENCE="svc-workflow"
export WORKFLOW_JWT_CLOCK_SKEW="0"
export WORKFLOW_REQUEST_TIMEOUT_SECS="30"
export WORKFLOW_REQUEST_BODY_MAX_BYTES="2097152"
export WORKFLOW_PROVISIONING_PRINCIPAL_IDS="$OWNER_ID"

"$SVC_BIN" >"${TMPDIR:-/tmp}/svc-workflow-$SUFFIX.log" 2>&1 &
SERVER_PID=$!
for _ in $(seq 1 30); do
    if curl -sf "http://127.0.0.1:$PORT/readyz" >/dev/null 2>&1; then
        break
    fi
    sleep 1
done
curl -sf "http://127.0.0.1:$PORT/readyz" >/dev/null

psql -U postgres -d "$DB_NAME" >/dev/null <<SQL
INSERT INTO principals (principal_id, principal_type, display_name, email, enabled)
VALUES ('$OWNER_ID', 'AGENT', 'SDK Owner', 'sdk-owner@test', TRUE);
INSERT INTO domains (domain_id, domain_key, display_name, enabled)
VALUES ('$DOMAIN_ID', 'sdk-$SUFFIX', 'SDK Integration', TRUE);
INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
VALUES (gen_random_uuid(), '$DOMAIN_ID', '$OWNER_ID', 'DOMAIN_OWNER', TRUE);
INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name)
VALUES ('$DEFINITION_ID', '$DOMAIN_ID', 'sdk-integration-v1', 'SDK Integration');
INSERT INTO workflow_definition_versions (
    definition_version_id, workflow_definition_id, version_number, version_status, context_schema
) VALUES ('$VERSION_ID', '$DEFINITION_ID', 1, 'DRAFT', '{"type":"object"}'::jsonb);
INSERT INTO workflow_node_definitions (
    node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type
) VALUES ('$DRAFT_NODE_ID', '$VERSION_ID', 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR');
INSERT INTO workflow_node_definitions (
    node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type
) VALUES ('$TERMINAL_NODE_ID', '$VERSION_ID', 'done', 'Done', 1, 'TERMINAL', NULL);
INSERT INTO workflow_transition_definitions (
    transition_id, definition_version_id, transition_key, display_name,
    source_node_id, target_node_id, transition_effect
) VALUES (
    '$TRANSITION_ID', '$VERSION_ID', 'advance', 'Advance',
    '$DRAFT_NODE_ID', '$TERMINAL_NODE_ID', 'ADVANCE'
);
UPDATE workflow_node_definitions
SET primary_advance_transition_id = '$TRANSITION_ID'
WHERE node_id = '$DRAFT_NODE_ID';
UPDATE workflow_definition_versions
SET version_status = 'PUBLISHED'
WHERE definition_version_id = '$VERSION_ID';
SQL

make_token() {
    python3 - "$OWNER_ID" "$JWT_SECRET" "$1" <<'PY'
import sys
import time

import jwt

subject, secret, scope = sys.argv[1:]
print(jwt.encode({
    "sub": subject,
    "iss": "auth-service",
    "aud": "svc-workflow",
    "exp": int(time.time()) + 3600,
    "iat": int(time.time()),
    "principal_type": "agent",
    "type": "access",
    "version": "v1",
    "scope": scope,
}, secret, algorithm="HS256"))
PY
}

export WORKFLOW_SDK_TEST_TOKEN="$(make_token 'workflow.execute workflow.read')"
export WORKFLOW_SDK_TEST_READ_TOKEN="$(make_token 'workflow.read')"
test -n "$WORKFLOW_SDK_TEST_TOKEN"
test -n "$WORKFLOW_SDK_TEST_READ_TOKEN"
export WORKFLOW_SDK_TEST_BASE_URL="http://127.0.0.1:$PORT"
export WORKFLOW_SDK_TEST_DOMAIN_ID="$DOMAIN_ID"
export WORKFLOW_SDK_TEST_TRANSITION_ID="$TRANSITION_ID"

cd "$REPO_DIR"
npm run build:sdk
npm run test:sdk:integration

INSTANCE_ID="$(psql -U postgres -d "$DB_NAME" -Atc \
    "SELECT workflow_instance_id FROM workflow_instances WHERE external_reference = 'fixture-ref-001'")"
test -n "$INSTANCE_ID"
export SVC_WORKFLOW_BASE_URL="$WORKFLOW_SDK_TEST_BASE_URL"
export SVC_WORKFLOW_ACCESS_TOKEN="$WORKFLOW_SDK_TEST_TOKEN"
CLI=(node "$REPO_DIR/sdk/typescript/dist/cli.js")

"${CLI[@]}" create \
    --input "$REPO_DIR/contracts/workflow-http/v1/fixtures/create-request.json" \
    --idempotency-key sdk-fixture-create \
    --request-id sdk-cli-create \
    | jq -e --arg id "$INSTANCE_ID" '.workflowInstanceId == $id' >/dev/null
"${CLI[@]}" list --domain-id "$DOMAIN_ID" --limit 20 --request-id sdk-cli-list \
    | jq -e --arg id "$INSTANCE_ID" \
        'any(.items[]; .workflow_instance_id == $id)' >/dev/null
"${CLI[@]}" worklist --kind assigned --limit 20 --request-id sdk-cli-worklist \
    | jq -e '.items | length >= 2' >/dev/null
"${CLI[@]}" detail --instance-id "$INSTANCE_ID" --request-id sdk-cli-detail \
    | jq -e --arg id "$INSTANCE_ID" '.detail.instance.workflow_instance_id == $id' >/dev/null
"${CLI[@]}" timeline --instance-id "$INSTANCE_ID" --limit 20 --request-id sdk-cli-timeline \
    | jq -e '.items | length == 2' >/dev/null
jq -nc --arg transition_id "$TRANSITION_ID" \
    '{transitionDefinitionId: $transition_id, expectedWorkflowStateVersion: 1}' \
    | "${CLI[@]}" transition --instance-id "$INSTANCE_ID" --input - \
        --idempotency-key sdk-transition --request-id sdk-cli-transition \
    | jq -e '.workflowStateVersion == 2' >/dev/null

echo "SDK real-process and CLI six-command smoke: PASS"
