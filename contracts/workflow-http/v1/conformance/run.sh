#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Workflow HTTP Contract V1 — Real Process Conformance
#
# Starts the svc-workflow binary with an isolated database, seeds test data,
# runs black-box HTTP scenarios, then cleans up. Exits 0 on pass, non-zero
# on any conformance failure.
#
# Prerequisites:
#   - PostgreSQL running on localhost:5432 with trust/auth for postgres user
#   - psql, curl, jq, python3 (with PyJWT) on PATH
#   - compiled binary (target/release/svc-workflow or SVC_WORKFLOW_BIN)
#
# Usage:
#   SVC_WORKFLOW_BIN=./target/release/svc-workflow \
#     ./contracts/workflow-http/v1/conformance/run.sh
# ---------------------------------------------------------------------------

set -u
SELF="$0"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../../../../" && pwd)"
SVC_BIN="${SVC_WORKFLOW_BIN:-$REPO_DIR/target/release/svc-workflow}"

if [ ! -x "$SVC_BIN" ]; then
    echo "ERROR: binary not found or not executable: $SVC_BIN"
    echo "Set SVC_WORKFLOW_BIN or build with: cargo build --release"
    exit 1
fi

PASS=0
FAIL=0
pass() { PASS=$((PASS + 1)); echo "  PASS: $1"; }
fail() { FAIL=$((FAIL + 1)); echo "  FAIL: $1"; }

# Generate unique identifiers
SUFFIX="conf_$(date +%s)_$$"
DB_NAME="svc_workflow_${SUFFIX}"
OWNER_ID="11111111-1111-1111-1111-111111111111"
DOMAIN_ID="22222222-2222-2222-2222-222222222222"
CREATOR_ID="33333333-3333-3333-3333-333333333333"
DEF_ID="44444444-4444-4444-4444-444444444444"
VER_ID="55555555-5555-5555-5555-555555555555"
DRAFT_NODE_ID="66666666-6666-6666-6666-666666666666"
TERM_NODE_ID="77777777-7777-7777-7777-777777777777"
ADVANCE_ID="88888888-8888-8888-8888-888888888888"

cleanup() {
    set +e
    if [ -n "${SERVER_PID:-}" ]; then
        kill "$SERVER_PID" 2>/dev/null
        wait "$SERVER_PID" 2>/dev/null
    fi
    if [ -n "${JWKS_PID:-}" ]; then
        kill "$JWKS_PID" 2>/dev/null
        wait "$JWKS_PID" 2>/dev/null
    fi
    rm -f /tmp/conf_jwt_rsa_key.pem
    if [ -n "${DB_NAME:-}" ]; then
        psql -U postgres -c "DROP DATABASE IF EXISTS \"$DB_NAME\" WITH (FORCE)" 2>/dev/null
    fi
}
trap cleanup EXIT

# -------------------------------------------------------------------------
# 1. Create isolated database
# -------------------------------------------------------------------------
echo "Creating isolated database: $DB_NAME"
createdb -U postgres "$DB_NAME"

# -------------------------------------------------------------------------
# 2. Start server (it will auto-migrate on startup)
# -------------------------------------------------------------------------
PORT=$(( ((RANDOM << 10) | (RANDOM & 0x3FF)) % 40000 + 20000 ))
echo "Starting server on port $PORT"

export DATABASE_URL="postgres://postgres:postgres@localhost:5432/$DB_NAME"
export WORKFLOW_BIND_ADDR="127.0.0.1"
export WORKFLOW_PORT="$PORT"
export WORKFLOW_JWKS_URL="http://127.0.0.1:$((PORT + 1))/.well-known/jwks.json"
export WORKFLOW_JWT_ISSUER="auth-service"
export WORKFLOW_JWT_AUDIENCE="svc-workflow"
export WORKFLOW_JWT_CLOCK_SKEW="60"
export AUTH_V1_CANARY_ENABLED="true"
export AUTH_V1_CANARY_WRITE_ENABLED="true"
export AUTH_V1_CANARY_ALLOWED_CLIENT_ID="conformance-client"
export AUTH_V1_CANARY_ALLOWED_SUB=""
export WORKFLOW_REQUEST_TIMEOUT_SECS="30"
export WORKFLOW_REQUEST_BODY_MAX_BYTES="2097152"
export WORKFLOW_PROVISIONING_PRINCIPAL_IDS="$OWNER_ID"

# Generate RSA key pair and start mock JWKS server
JWKS_PORT=$((PORT + 1))

# Write the Python script to a temp file to avoid heredoc quoting issues
cat > /tmp/conf_jwks_server.py << 'PYEOF'
import json, socketserver, threading, http.server, base64, os, sys
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.hazmat.primitives import serialization

port = int(sys.argv[1])
key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
pub = key.public_key()

priv_pem = key.private_bytes(
    encoding=serialization.Encoding.PEM,
    format=serialization.PrivateFormat.PKCS8,
    encryption_algorithm=serialization.NoEncryption()
).decode()
os.makedirs('/tmp', exist_ok=True)
with open('/tmp/conf_jwt_rsa_key.pem', 'w') as f:
    f.write(priv_pem)

pub_nums = pub.public_numbers()
n_bytes = pub_nums.n.to_bytes((pub_nums.n.bit_length() + 7) // 8, 'big')
e_bytes = pub_nums.e.to_bytes((pub_nums.e.bit_length() + 7) // 8, 'big')
n_b64 = base64.urlsafe_b64encode(n_bytes).rstrip(b'=').decode()
e_b64 = base64.urlsafe_b64encode(e_bytes).rstrip(b'=').decode()

jwks_body = json.dumps({'keys': [{'kty': 'RSA', 'use': 'sig', 'alg': 'RS256', 'kid': 'conf-key-v1', 'n': n_b64, 'e': e_b64}]})

class JwksHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(jwks_body)))
        self.end_headers()
        self.wfile.write(jwks_body.encode())
    def log_message(self, *a): pass

httpd = socketserver.TCPServer(('127.0.0.1', port), JwksHandler)
t = threading.Thread(target=httpd.serve_forever, daemon=True)
t.start()
sys.stdout.flush()
print('JWKS ready on port ' + str(port))
httpd.serve_forever()
PYEOF

python3 /tmp/conf_jwks_server.py $JWKS_PORT &
JWKS_PID=$!

# Wait for JWKS server to be ready
for i in $(seq 1 10); do
    if curl -s -o /dev/null "http://127.0.0.1:$JWKS_PORT/.well-known/jwks.json" 2>/dev/null; then
        break
    fi
    sleep 1
done

"$SVC_BIN" &
SERVER_PID=$!

# Wait for server to be fully ready (DB migrated + JWKS cached)
for i in $(seq 1 30); do
    if curl -s -o /dev/null "http://127.0.0.1:$PORT/readyz" 2>/dev/null; then
        break
    fi
    sleep 1
done

if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "ERROR: Server failed to start"
    exit 1
fi
echo "Server is ready (readyz OK)"

# -------------------------------------------------------------------------
# 3. Seed test data via direct DB (after server has applied migrations)
# -------------------------------------------------------------------------
echo "Seeding test data..."
psql -U postgres -d "$DB_NAME" <<SQL
INSERT INTO principals (principal_id, principal_type, display_name, email, enabled)
VALUES ('$OWNER_ID', 'HUMAN', 'Owner', 'owner@test', TRUE);
INSERT INTO principals (principal_id, principal_type, display_name, email, enabled)
VALUES ('$CREATOR_ID', 'AGENT', 'Creator', 'creator@test', TRUE);
INSERT INTO domains (domain_id, domain_key, display_name, enabled)
VALUES ('$DOMAIN_ID', 'conformance-test-$SUFFIX', 'Conformance Test', TRUE);
INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
VALUES (gen_random_uuid(), '$DOMAIN_ID', '$OWNER_ID', 'DOMAIN_OWNER', TRUE);
INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
VALUES (gen_random_uuid(), '$DOMAIN_ID', '$CREATOR_ID', 'MEMBER', TRUE);

INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name)
VALUES ('$DEF_ID', '$DOMAIN_ID', 'conformance-def-$SUFFIX', 'Conformance Def');
INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema)
VALUES ('$VER_ID', '$DEF_ID', 1, 'DRAFT', '{"type":"object"}'::jsonb);
INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type)
VALUES ('$DRAFT_NODE_ID', '$VER_ID', 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR');
INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type)
VALUES ('$TERM_NODE_ID', '$VER_ID', 'done', 'Done', 1, 'TERMINAL', NULL);
INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect)
VALUES ('$ADVANCE_ID', '$VER_ID', 'advance', 'Advance', '$DRAFT_NODE_ID', '$TERM_NODE_ID', 'ADVANCE');
UPDATE workflow_node_definitions SET primary_advance_transition_id = '$ADVANCE_ID' WHERE node_id = '$DRAFT_NODE_ID';
UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = '$VER_ID';
SQL

# -------------------------------------------------------------------------
# 4. Generate RS256 JWT tokens using runtime-generated RSA key
# -------------------------------------------------------------------------
# Wait for Python script and JWKS mock server
sleep 2

TOKEN_OWNER=$(python3 -c "
import jwt, time
with open('/tmp/conf_jwt_rsa_key.pem') as f: key = f.read()
now = int(time.time())
TTL=300
print(jwt.encode({
    'sub': '$OWNER_ID', 'iss': 'auth-service', 'aud': 'svc-workflow',
    'exp': now + TTL, 'iat': now, 'nbf': now,
    'principal_type': 'agent', 'client_id': 'conformance-client',
    'token_use': 'access', 'type': 'access', 'version': 'v1',
    'scope': 'workflow.execute workflow.read',
    'jti': 'conf-jti-' + str(now)
}, key, algorithm='RS256', headers={'kid': 'conf-key-v1', 'typ': 'at+jwt'}))
")
TOKEN_CREATOR_RO=$(python3 -c "
import jwt, time
with open('/tmp/conf_jwt_rsa_key.pem') as f: key = f.read()
now = int(time.time())
TTL=300
print(jwt.encode({
    'sub': '$CREATOR_ID', 'iss': 'auth-service', 'aud': 'svc-workflow',
    'exp': now + TTL, 'iat': now, 'nbf': now,
    'principal_type': 'agent', 'client_id': 'conformance-client',
    'token_use': 'access', 'type': 'access', 'version': 'v1',
    'scope': 'workflow.read',
    'jti': 'conf-jti-' + str(now)
}, key, algorithm='RS256', headers={'kid': 'conf-key-v1', 'typ': 'at+jwt'}))
")
TOKEN_HUMAN=$(python3 -c "
import jwt, time
with open('/tmp/conf_jwt_rsa_key.pem') as f: key = f.read()
now = int(time.time())
TTL=300
print(jwt.encode({
    'sub': '$OWNER_ID', 'iss': 'auth-service', 'aud': 'svc-workflow',
    'exp': now + TTL, 'iat': now, 'nbf': now,
    'principal_type': 'human', 'client_id': 'conformance-client',
    'token_use': 'access', 'type': 'access', 'version': 'v1',
    'scope': 'workflow.read',
    'jti': 'conf-jti-' + str(now)
}, key, algorithm='RS256', headers={'kid': 'conf-key-v1', 'typ': 'at+jwt'}))
")

BASE_URL="http://127.0.0.1:$PORT"

# -------------------------------------------------------------------------
# 5. Conformance checks  (set +e so each check is independent)
# -------------------------------------------------------------------------
set +e
echo ""
echo "========== WORKFLOW HTTP CONTRACT V1 CONFORMANCE =========="
echo ""

# 5.1 Health / Readiness
echo "--- health/ready/version ---"
HTTP_HEALTH=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/healthz")
[ "$HTTP_HEALTH" = "200" ] && pass "healthz 200" || fail "healthz: $HTTP_HEALTH"

HTTP_READY=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/readyz")
[ "$HTTP_READY" = "200" ] && pass "readyz 200" || fail "readyz: $HTTP_READY"

	VERSION_JSON=$(curl -s "$BASE_URL/version")
	echo "$VERSION_JSON" | jq -e '.service == "svc-workflow"' >/dev/null 2>&1 && pass "version.service == svc-workflow" || fail "version.service: $(echo $VERSION_JSON | head -c 200)"
	echo "$VERSION_JSON" | jq -e '.version == "0.3.1"' >/dev/null 2>&1 && pass "version.version == 0.3.1" || fail "version.version: $(echo $VERSION_JSON | head -c 200)"
	echo "$VERSION_JSON" | jq -e '.schemaVersion == "0014"' >/dev/null 2>&1 && pass "version.schemaVersion == 0014" || fail "version.schemaVersion: $(echo $VERSION_JSON | head -c 200)"
	echo "$VERSION_JSON" | jq -e '.apiContractVersion == "internal-v0"' >/dev/null 2>&1 && pass "version.apiContractVersion == internal-v0" || fail "version.apiContractVersion: $(echo $VERSION_JSON | head -c 200)"

# 5.2 Create
echo "--- create ---"
CREATE_KEY="conf-create-$SUFFIX"
CREATE_RESP=$(curl -s -X POST "$BASE_URL/internal/v1/workflow-instances" \
    -H "Authorization: Bearer $TOKEN_OWNER" \
    -H "Idempotency-Key: $CREATE_KEY" \
    -H "Content-Type: application/json" \
    -d '{
        "domainId": "'"$DOMAIN_ID"'",
        "definitionVersionId": "'"$VER_ID"'",
        "metadata": {"source": "conformance"},
        "contextPayload": {"title": "conformance"}
    }')
INSTANCE_ID=$(echo "$CREATE_RESP" | jq -r '.workflowInstanceId // empty')
[ -n "$INSTANCE_ID" ] && pass "create: $INSTANCE_ID" || fail "create: $(echo $CREATE_RESP | head -c 200)"

# If create failed, skip remaining checks that depend on instance_id
if [ -z "$INSTANCE_ID" ]; then
    fail "Skipping downstream checks due to create failure"
else
    # 5.3 Detail
    echo "--- detail ---"
    DETAIL_RESP=$(curl -s "$BASE_URL/internal/v1/workflow-instances/$INSTANCE_ID" \
        -H "Authorization: Bearer $TOKEN_OWNER")
    VISIBILITY=$(echo "$DETAIL_RESP" | jq -r '.visibility // empty')
    [ "$VISIBILITY" = "full" ] && pass "detail visibility=full" || fail "detail: $(echo $DETAIL_RESP | head -c 200)"

    # 5.4 Transition
    echo "--- transition ---"
    TRANSITION_KEY="conf-trn-$SUFFIX"
    TRANSITION_RESP=$(curl -s -X POST "$BASE_URL/internal/v1/workflow-instances/$INSTANCE_ID/transitions" \
        -H "Authorization: Bearer $TOKEN_OWNER" \
        -H "Idempotency-Key: $TRANSITION_KEY" \
        -H "Content-Type: application/json" \
        -d '{
            "transitionDefinitionId": "'"$ADVANCE_ID"'",
            "expectedWorkflowStateVersion": 1
        }')
    NEW_VERSION=$(echo "$TRANSITION_RESP" | jq -r '.workflowStateVersion // empty')
    [ "$NEW_VERSION" = "2" ] && pass "transition stateVersion=2" || fail "transition: $(echo $TRANSITION_RESP | head -c 200)"

    # 5.5 Timeline
    echo "--- timeline ---"
    TIMELINE_RESP=$(curl -s "$BASE_URL/internal/v1/workflow-instances/$INSTANCE_ID/timeline" \
        -H "Authorization: Bearer $TOKEN_OWNER")
    EVENT_COUNT=$(echo "$TIMELINE_RESP" | jq '.items | length')
    [ "$EVENT_COUNT" -ge 2 ] && pass "timeline: $EVENT_COUNT events" || fail "timeline: $(echo $TIMELINE_RESP | head -c 200)"
    echo "$TIMELINE_RESP" | jq -e '
        .items[0]
        | has("event_id")
          and has("workflow_instance_id")
          and has("event_sequence")
          and has("event_schema_version")
          and has("actor_principal_id")
          and has("created_at")
          and (has("eventId") | not)
    ' >/dev/null 2>&1 \
        && pass "timeline event uses frozen snake_case fields" \
        || fail "timeline event field mismatch: $(echo $TIMELINE_RESP | head -c 300)"

    echo "--- worklist ---"
    WL_RESP=$(curl -s "$BASE_URL/internal/v1/worklists/assigned-to-me" \
        -H "Authorization: Bearer $TOKEN_OWNER")
    WL_COUNT=$(echo "$WL_RESP" | jq '.items | length')
    echo "  assigned-to-me items: $WL_COUNT"

    echo "--- creator drafts ---"
    CD_RESP=$(curl -s "$BASE_URL/internal/v1/worklists/creator-owned-drafts" \
        -H "Authorization: Bearer $TOKEN_OWNER")
    CD_COUNT=$(echo "$CD_RESP" | jq '.items | length')
    echo "  creator-owned-drafts items: $CD_COUNT"
fi

# 5.6 Domain Owner list
echo "--- domain list ---"
DL_RESP=$(curl -s "$BASE_URL/internal/v1/workflow-instances/domain?domainId=$DOMAIN_ID" \
    -H "Authorization: Bearer $TOKEN_OWNER")
DL_COUNT=$(echo "$DL_RESP" | jq '.items | length')
[ "$DL_COUNT" -ge 1 ] && pass "domain list: $DL_COUNT items" || fail "domain list: $(echo $DL_RESP | head -c 200)"

# 5.7 Domain list pagination
echo "--- domain list pagination ---"
# Create 2 more instances for pagination test
for i in 0 1; do
    curl -s -X POST "$BASE_URL/internal/v1/workflow-instances" \
        -H "Authorization: Bearer $TOKEN_OWNER" \
        -H "Idempotency-Key: conf-page-$i-$SUFFIX" \
        -H "Content-Type: application/json" \
        -d '{"domainId":"'"$DOMAIN_ID"'","definitionVersionId":"'"$VER_ID"'","metadata":{"source":"pagination"},"contextPayload":{"title":"pagination-'$i'"}}' >/dev/null
done

# Sleep briefly so instances have distinct created_at
sleep 0.1

DL_PAGE1=$(curl -s "$BASE_URL/internal/v1/workflow-instances/domain?domainId=$DOMAIN_ID&limit=1" \
    -H "Authorization: Bearer $TOKEN_OWNER")
DL_CURSOR_JSON=$(echo "$DL_PAGE1" | jq -r '.next_cursor.created_at // empty')
[ -n "$DL_CURSOR_JSON" ] && pass "domain list page 1 has next_cursor" || fail "domain list no cursor on page 1: $(echo $DL_PAGE1 | head -c 200)"
CA=$(echo "$DL_PAGE1" | jq -r '.next_cursor.created_at // ""')
CID=$(echo "$DL_PAGE1" | jq -r '.next_cursor.id // ""')
if [ -n "$CA" ] && [ -n "$CID" ]; then
    DL_PAGE2=$(curl -s "$BASE_URL/internal/v1/workflow-instances/domain?domainId=$DOMAIN_ID&limit=1&beforeCreatedAt=$CA&beforeId=$CID" \
        -H "Authorization: Bearer $TOKEN_OWNER")
    DL_PAGE2_COUNT=$(echo "$DL_PAGE2" | jq '.items | length')
    echo "  page 2 items: $DL_PAGE2_COUNT"
    [ "$DL_PAGE2_COUNT" -ge 1 ] && pass "domain list page 2 returns items" || fail "domain list page 2 empty when expected items"
fi

# -------------------------------------------------------------------------
# Negative path checks
# -------------------------------------------------------------------------
echo ""
echo "--- negative path ---"

# 5.8 401 Missing Bearer
MISSING_AUTH_RESP=$(curl -s "$BASE_URL/internal/v1/workflow-instances/domain?domainId=$DOMAIN_ID")
MISSING_AUTH_CODE=$(echo "$MISSING_AUTH_RESP" | jq -r '.error.code // empty')
[ "$MISSING_AUTH_CODE" = "unauthenticated" ] \
    && pass "401 unauthenticated wire code" \
    || fail "expected unauthenticated, got $(echo $MISSING_AUTH_RESP | head -c 200)"

# 5.9 401 Invalid principal type
HUMAN_RESP=$(curl -s "$BASE_URL/internal/v1/workflow-instances/domain?domainId=$DOMAIN_ID" \
    -H "Authorization: Bearer $TOKEN_HUMAN")
HUMAN_CODE=$(echo "$HUMAN_RESP" | jq -r '.error.code // empty')
[ "$HUMAN_CODE" = "invalid_principal_type" ] \
    && pass "401 invalid_principal_type for human token" \
    || fail "expected invalid_principal_type, got $(echo $HUMAN_RESP | head -c 200)"

# 5.10 403 Forbidden (no workflow.execute scope for create)
HTTP_403=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/internal/v1/workflow-instances" \
    -H "Authorization: Bearer $TOKEN_CREATOR_RO" \
    -H "Idempotency-Key: conf-403-$SUFFIX" \
    -H "Content-Type: application/json" \
    -d '{"domainId":"'"$DOMAIN_ID"'","definitionVersionId":"'"$VER_ID"'","metadata":{},"contextPayload":{}}')
[ "$HTTP_403" = "403" ] && pass "403 forbidden" || fail "expected 403, got $HTTP_403"

# 5.11 Unknown request fields remain rejected by the strict DTO contract
UNKNOWN_FIELD_RESP=$(curl -s -X POST "$BASE_URL/internal/v1/workflow-instances" \
    -H "Authorization: Bearer $TOKEN_OWNER" \
    -H "Idempotency-Key: conf-unknown-$SUFFIX" \
    -H "Content-Type: application/json" \
    -d '{"domainId":"'"$DOMAIN_ID"'","definitionVersionId":"'"$VER_ID"'","metadata":{},"contextPayload":{},"principalId":"'"$OWNER_ID"'"}')
UNKNOWN_FIELD_CODE=$(echo "$UNKNOWN_FIELD_RESP" | jq -r '.error.code // empty')
[ "$UNKNOWN_FIELD_CODE" = "unknown_field" ] \
    && pass "400 unknown_field for undeclared request property" \
    || fail "expected unknown_field, got $(echo $UNKNOWN_FIELD_RESP | head -c 200)"

# 5.12 409 Idempotency Conflict (if we have a transition key)
if [ -n "${TRANSITION_KEY:-}" ]; then
    CONFLICT_RESP=$(curl -s -X POST "$BASE_URL/internal/v1/workflow-instances/$INSTANCE_ID/transitions" \
        -H "Authorization: Bearer $TOKEN_OWNER" \
        -H "Idempotency-Key: $TRANSITION_KEY" \
        -H "Content-Type: application/json" \
        -d '{"transitionDefinitionId":"'"$ADVANCE_ID"'","expectedWorkflowStateVersion":999}')
    CONFLICT_CODE=$(echo "$CONFLICT_RESP" | jq -r '.error.code // empty')
    [ "$CONFLICT_CODE" = "idempotency_conflict" ] && pass "409 idempotency_conflict" || fail "expected idempotency_conflict, got $(echo $CONFLICT_RESP | head -c 200)"
fi

# 5.13 422 Invalid Cursor
HTTP_422=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/internal/v1/workflow-instances/domain?domainId=$DOMAIN_ID&beforeCreatedAt=not-a-date&beforeId=not-a-uuid" \
    -H "Authorization: Bearer $TOKEN_OWNER")
[ "$HTTP_422" = "422" ] && pass "422 invalid_cursor" || fail "expected 422, got $HTTP_422"

# -------------------------------------------------------------------------
# Summary
# -------------------------------------------------------------------------
echo ""
echo "========== CONFORMANCE RESULTS =========="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
if [ "$FAIL" -eq 0 ]; then
    echo "STATUS: ALL PASSED"
    echo "=========================================="
    exit 0
else
    echo "STATUS: $FAIL FAILURES"
    echo "=========================================="
    exit 1
fi
