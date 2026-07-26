#!/usr/bin/env bash
set -u
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../../../" && pwd)"
SVC_BIN="${SVC_WORKFLOW_BIN:-$REPO_DIR/target/release/svc-workflow}"
[ ! -x "$SVC_BIN" ] && { echo "ERROR: binary not found: $SVC_BIN"; exit 1; }
PASS=0; FAIL=0
pass() { PASS=$((PASS+1)); echo "  PASS: $1"; }
fail() { FAIL=$((FAIL+1)); echo "  FAIL: $1"; }
SUFFIX="defa_$(date +%s)_$$"
DB_NAME="svc_workflow_${SUFFIX}"
OWNER_ID="11111111-1111-1111-1111-111111111111"
DOMAIN_ID="22222222-2222-2222-2222-222222222222"
cleanup() { set +e; [ -n "${SERVER_PID:-}" ] && { kill "$SERVER_PID" 2>/dev/null; wait "$SERVER_PID" 2>/dev/null; }; [ -n "${JWKS_PID:-}" ] && { kill "$JWKS_PID" 2>/dev/null; wait "$JWKS_PID" 2>/dev/null; }; rm -f /tmp/defa_jwt_key.pem; [ -n "${DB_NAME:-}" ] && psql -U postgres -c "DROP DATABASE IF EXISTS \"$DB_NAME\" WITH (FORCE)" 2>/dev/null; }; trap cleanup EXIT
echo "Creating database: $DB_NAME"
createdb -U postgres "$DB_NAME" 2>/dev/null
PORT=$(( ((RANDOM<<10)|(RANDOM&0x3FF))%40000+20000 )); JWKS_PORT=$((PORT+1))
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/$DB_NAME" WORKFLOW_BIND_ADDR="127.0.0.1" WORKFLOW_PORT="$PORT" WORKFLOW_JWKS_URL="http://127.0.0.1:$JWKS_PORT/.well-known/jwks.json" WORKFLOW_JWT_ISSUER="auth-service" WORKFLOW_JWT_AUDIENCE="svc-workflow" WORKFLOW_JWT_CLOCK_SKEW="60" AUTH_V1_CANARY_ENABLED="true" AUTH_V1_CANARY_WRITE_ENABLED="true" AUTH_V1_CANARY_ALLOWED_CLIENT_ID="defa-c" AUTH_V1_CANARY_ALLOWED_SUB="" WORKFLOW_REQUEST_TIMEOUT_SECS="30" WORKFLOW_REQUEST_BODY_MAX_BYTES="2097152" WORKFLOW_PROVISIONING_PRINCIPAL_IDS="$OWNER_ID"
python3 -c "
import json,socketserver,threading,http.server,base64,os,sys
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.hazmat.primitives import serialization
port=int(sys.argv[1]);key=rsa.generate_private_key(public_exponent=65537,key_size=2048);pub=key.public_key()
priv_pem=key.private_bytes(encoding=serialization.Encoding.PEM,format=serialization.PrivateFormat.PKCS8,encryption_algorithm=serialization.NoEncryption()).decode()
with open('/tmp/defa_jwt_key.pem','w') as f: f.write(priv_pem)
n=pub.public_numbers();n64=base64.urlsafe_b64encode(n.n.to_bytes((n.n.bit_length()+7)//8,'big')).rstrip(b'=').decode()
e64=base64.urlsafe_b64encode(n.e.to_bytes((n.e.bit_length()+7)//8,'big')).rstrip(b'=').decode()
jb=json.dumps({'keys':[{'kty':'RSA','use':'sig','alg':'RS256','kid':'dk1','n':n64,'e':e64}]})
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self): self.send_response(200);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(jb)));self.end_headers();self.wfile.write(jb.encode())
    def log_message(self,*a): pass
s=socketserver.TCPServer(('127.0.0.1',port),H);t=threading.Thread(target=s.serve_forever,daemon=True);t.start();sys.stdout.flush();s.serve_forever()
" "$JWKS_PORT" &>/dev/null & JWKS_PID=$!; sleep 1
"$SVC_BIN" &>/tmp/defa_server.log & SERVER_PID=$!; sleep 3
psql -U postgres -d "$DB_NAME" -c "INSERT INTO principals VALUES('$OWNER_ID','AGENT','DefA','d@t',TRUE); INSERT INTO domains VALUES('$DOMAIN_ID','defa-$SUFFIX','DefA',TRUE); INSERT INTO domain_role_bindings VALUES(gen_random_uuid(),'$DOMAIN_ID','$OWNER_ID','DOMAIN_OWNER',TRUE);" 2>/dev/null
TOKEN=$(python3 -c "
import jwt,time;f=open('/tmp/defa_jwt_key.pem');key=f.read();f.close();n=int(time.time())
print(jwt.encode({'sub':'$OWNER_ID','iss':'auth-service','aud':'svc-workflow','exp':n+600,'iat':n,'nbf':n,'principal_type':'agent','client_id':'defa-c','token_use':'access','type':'access','version':'v1','scope':'workflow.execute workflow.read','jti':'defa-apply-integration-'+str(n)},key,algorithm='RS256',headers={'kid':'dk1','typ':'at+jwt'}))
")
export SVC_WORKFLOW_BASE_URL="http://127.0.0.1:$PORT" SVC_WORKFLOW_ACCESS_TOKEN="$TOKEN"
npx tsc -p tsconfig.sdk.json 2>/dev/null; CLI="node $REPO_DIR/sdk/typescript/dist/cli.js"
set +e
echo ""; echo "========== DEFINITION APPLY INTEGRATION =========="; echo ""

# Step 1: First apply
DEF_ART="/tmp/defa_${SUFFIX}.json"
cat > "$DEF_ART" << JSONEOF
{"artifactVersion":"definition-artifact-v1","domainId":"$DOMAIN_ID","definitionKey":"defa-integ-$SUFFIX","displayName":"Integration Test Def","versionNumber":1,"nodes":[{"nodeKey":"draft","displayName":"Draft","orderIndex":0,"nodeType":"DRAFT","assigneeRefType":"WORKFLOW_CREATOR","primaryAdvanceTransitionKey":"finish"},{"nodeKey":"done","displayName":"Done","orderIndex":1,"nodeType":"TERMINAL"}],"transitions":[{"transitionKey":"finish","displayName":"Finish","sourceNodeKey":"draft","targetNodeKey":"done","transitionEffect":"ADVANCE"}]}
JSONEOF
echo "--- 1. First apply ---"
FIRST=$($CLI definition apply --file "$DEF_ART" 2>&1); FIRST_STATUS=$(echo "$FIRST"|jq -r '.status//empty'); echo "  first: $(echo $FIRST|head -c 200)"
[ "$FIRST_STATUS" = "APPLIED" ] && pass "First apply returns APPLIED" || fail "First apply: $(echo $FIRST|head -c 200)"
WF_ID=$(echo "$FIRST"|jq -r '.workflowDefinitionId//empty'); VER_ID=$(echo "$FIRST"|jq -r '.definitionVersionId//empty')
[ -n "$WF_ID" ] && pass "Has workflowDefinitionId" || fail "No workflowDefinitionId"
[ -n "$VER_ID" ] && pass "Has definitionVersionId" || fail "No definitionVersionId"

# Step 2: Idempotent second apply
echo "--- 2. Second apply (idempotent) ---"
SECOND=$($CLI definition apply --file "$DEF_ART" 2>&1); S2=$(echo "$SECOND"|jq -r '.status//empty'); echo "  second: $S2"
[ "$S2" = "ALREADY_APPLIED" ] && pass "Second returns ALREADY_APPLIED" || fail "Second: $S2"

# Step 3: Version 2 apply
echo "--- 3. Version 2 ---"
cat > "$DEF_ART" << JSONEOF
{"artifactVersion":"definition-artifact-v1","domainId":"$DOMAIN_ID","definitionKey":"defa-integ-$SUFFIX","displayName":"Integration Test Def","versionNumber":2,"nodes":[{"nodeKey":"a","displayName":"A","orderIndex":0,"nodeType":"DRAFT","assigneeRefType":"WORKFLOW_CREATOR","primaryAdvanceTransitionKey":"ab"},{"nodeKey":"b","displayName":"B","orderIndex":1,"nodeType":"TERMINAL"}],"transitions":[{"transitionKey":"ab","displayName":"A->B","sourceNodeKey":"a","targetNodeKey":"b","transitionEffect":"ADVANCE"}]}
JSONEOF
V2=$($CLI definition apply --file "$DEF_ART" 2>&1); V2S=$(echo "$V2"|jq -r '.status//empty')
[ "$V2S" = "APPLIED" ] && pass "Version 2 APPLIED" || fail "Version 2: $(echo $V2|head -c 200)"

# Step 4: Same version, different digest
echo "--- 4. Same version diff digest ---"
cat > "$DEF_ART" << JSONEOF
{"artifactVersion":"definition-artifact-v1","domainId":"$DOMAIN_ID","definitionKey":"defa-integ-$SUFFIX","displayName":"Integration Test Def","versionNumber":2,"nodes":[{"nodeKey":"x","displayName":"X","orderIndex":0,"nodeType":"DRAFT","assigneeRefType":"WORKFLOW_CREATOR","primaryAdvanceTransitionKey":"xy"},{"nodeKey":"y","displayName":"Y","orderIndex":1,"nodeType":"TERMINAL"}],"transitions":[{"transitionKey":"xy","displayName":"X->Y","sourceNodeKey":"x","targetNodeKey":"y","transitionEffect":"ADVANCE"}]}
JSONEOF
V2B=$($CLI definition apply --file "$DEF_ART" 2>&1); V2BC=$(echo "$V2B"|jq -r '.error.code//empty')
[ "$V2BC" = "DEFINITION_VERSION_DIGEST_MISMATCH" ] && pass "Digest mismatch caught" || fail "Expected DIGEST_MISMATCH got $V2BC"

# Step 5: Version sequence skip
echo "--- 5. Version sequence skip ---"
cat > "$DEF_ART" << JSONEOF
{"artifactVersion":"definition-artifact-v1","domainId":"$DOMAIN_ID","definitionKey":"defa-integ-$SUFFIX","displayName":"Integration Test Def","versionNumber":5,"nodes":[{"nodeKey":"p","displayName":"P","orderIndex":0,"nodeType":"DRAFT","assigneeRefType":"WORKFLOW_CREATOR","primaryAdvanceTransitionKey":"pq"},{"nodeKey":"q","displayName":"Q","orderIndex":1,"nodeType":"TERMINAL"}],"transitions":[{"transitionKey":"pq","displayName":"P->Q","sourceNodeKey":"p","targetNodeKey":"q","transitionEffect":"ADVANCE"}]}
JSONEOF
V5=$($CLI definition apply --file "$DEF_ART" 2>&1); V5C=$(echo "$V5"|jq -r '.error.code//empty')
[ "$V5C" = "DEFINITION_VERSION_SEQUENCE_MISMATCH" ] && pass "Sequence mismatch caught" || fail "Expected SEQUENCE_MISMATCH got $V5C"

# Step 6: Identity mismatch (different displayName)
echo "--- 6. Identity mismatch ---"
cat > "$DEF_ART" << JSONEOF
{"artifactVersion":"definition-artifact-v1","domainId":"$DOMAIN_ID","definitionKey":"defa-integ-mismatch-$SUFFIX","displayName":"Original Name","versionNumber":1,"nodes":[{"nodeKey":"d","displayName":"D","orderIndex":0,"nodeType":"DRAFT","assigneeRefType":"WORKFLOW_CREATOR","primaryAdvanceTransitionKey":"dd"},{"nodeKey":"e","displayName":"E","orderIndex":1,"nodeType":"TERMINAL"}],"transitions":[{"transitionKey":"dd","displayName":"D->E","sourceNodeKey":"d","targetNodeKey":"e","transitionEffect":"ADVANCE"}]}
JSONEOF
$CLI definition apply --file "$DEF_ART" >/dev/null 2>&1
# Now try with different displayName
cat > "$DEF_ART" << JSONEOF
{"artifactVersion":"definition-artifact-v1","domainId":"$DOMAIN_ID","definitionKey":"defa-integ-mismatch-$SUFFIX","displayName":"Different Name","versionNumber":1,"nodes":[{"nodeKey":"d","displayName":"D","orderIndex":0,"nodeType":"DRAFT","assigneeRefType":"WORKFLOW_CREATOR","primaryAdvanceTransitionKey":"dd"},{"nodeKey":"e","displayName":"E","orderIndex":1,"nodeType":"TERMINAL"}],"transitions":[{"transitionKey":"dd","displayName":"D->E","sourceNodeKey":"d","targetNodeKey":"e","transitionEffect":"ADVANCE"}]}
JSONEOF
MIS=$($CLI definition apply --file "$DEF_ART" 2>&1); MISC=$(echo "$MIS"|jq -r '.error.code//empty')
[ "$MISC" = "DEFINITION_IDENTITY_MISMATCH" ] && pass "Identity mismatch caught" || fail "Expected IDENTITY_MISMATCH got $MISC"

echo ""; echo "========== RESULTS =========="; echo "PASS: $PASS"; echo "FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && echo "STATUS: ALL PASSED" || echo "STATUS: $FAIL FAILURES"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
