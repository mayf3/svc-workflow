#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# verify-digests.sh — Workflow HTTP Contract V1 Bundle Digest Verifier
#
# Computes SCHEMA_DIGEST and BUNDLE_DIGEST per digest-spec.md and compares
# against manifest.json. Exits 0 on match, non-zero on mismatch.
#
# Prerequisites:
#   - python3, shasum (or sha256sum), jq
#
# Usage:
#   ./contracts/workflow-http/v1/verify-digests.sh
#
# Output:
#   SCHEMA_DIGEST=<sha256-hex>
#   BUNDLE_DIGEST=<sha256-hex>
#   DIGEST_VERIFICATION=PASS   (or FAIL)
# ---------------------------------------------------------------------------

set -euo pipefail
SELF="$0"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUNDLE_DIR="$SCRIPT_DIR"
MANIFEST="$BUNDLE_DIR/manifest.json"

# Verify prerequisites
command -v python3 >/dev/null 2>&1 || { echo "ERROR: python3 required"; exit 1; }

# -------------------------------------------------------------------------
# 1. Read manifest
# -------------------------------------------------------------------------
MANIFEST_JSON=$(python3 -c "
import json, sys
with open('$MANIFEST', 'rb') as f:
    print(f.read().decode('utf-8'))
")

MANIFEST_SCHEMA_DIGEST=$(echo "$MANIFEST_JSON" | python3 -c "
import json, sys
data = json.load(sys.stdin)
print(data.get('schema_digest', ''))
")

MANIFEST_BUNDLE_DIGEST=$(echo "$MANIFEST_JSON" | python3 -c "
import json, sys
data = json.load(sys.stdin)
print(data.get('bundle_digest', ''))
")

BUNDLE_FILE_SET=$(echo "$MANIFEST_JSON" | python3 -c "
import json, sys
data = json.load(sys.stdin)
files = data.get('bundle_file_set', [])
for f in sorted(files):
    print(f)
")

if [ -z "$MANIFEST_SCHEMA_DIGEST" ]; then
    echo "ERROR: manifest.json missing schema_digest"
    exit 1
fi
if [ -z "$MANIFEST_BUNDLE_DIGEST" ]; then
    echo "ERROR: manifest.json missing bundle_digest"
    exit 1
fi
if [ -z "$BUNDLE_FILE_SET" ]; then
    echo "ERROR: manifest.json missing bundle_file_set"
    exit 1
fi

# -------------------------------------------------------------------------
# 2. Compute SCHEMA_DIGEST = SHA-256(openapi.yaml raw bytes)
# -------------------------------------------------------------------------
COMPUTED_SCHEMA_DIGEST=$(
    shasum -a 256 "$BUNDLE_DIR/openapi.yaml" 2>/dev/null | awk '{print $1}' ||
    sha256sum "$BUNDLE_DIR/openapi.yaml" 2>/dev/null | awk '{print $1}'
)

if [ -z "$COMPUTED_SCHEMA_DIGEST" ]; then
    echo "ERROR: cannot compute SHA-256 of openapi.yaml"
    exit 1
fi

# -------------------------------------------------------------------------
# 3. Compute BUNDLE_DIGEST per spec
# -------------------------------------------------------------------------
COMPUTED_BUNDLE_DIGEST=$(python3 -c "
import hashlib, json, sys, os

bundle_dir = '$BUNDLE_DIR'
manifest_path = os.path.join(bundle_dir, 'manifest.json')

with open(manifest_path, 'rb') as f:
    manifest_raw = f.read()
manifest_data = json.loads(manifest_raw)

file_set = sorted(manifest_data.get('bundle_file_set', []))

# Normalize manifest: set BUNDLE_DIGEST to 64 zeros
normalized_manifest = dict(manifest_data)
normalized_manifest['bundle_digest'] = '0' * 64
normalized_manifest_bytes = json.dumps(normalized_manifest, indent=2, ensure_ascii=False).encode('utf-8') + b'\n'

hash_input = b''
for rel_path in file_set:
    if rel_path == 'manifest.json':
        file_bytes = normalized_manifest_bytes
    else:
        full_path = os.path.join(bundle_dir, rel_path)
        with open(full_path, 'rb') as f:
            file_bytes = f.read()
    entry = rel_path.encode('utf-8') + b'\x00' + file_bytes
    hash_input += entry

digest = hashlib.sha256(hash_input).hexdigest()
print(digest, end='')
")

# -------------------------------------------------------------------------
# 4. Compare and report
# -------------------------------------------------------------------------
SCHEMA_PASS=false
BUNDLE_PASS=false

[ "$COMPUTED_SCHEMA_DIGEST" = "$MANIFEST_SCHEMA_DIGEST" ] && SCHEMA_PASS=true
[ "$COMPUTED_BUNDLE_DIGEST" = "$MANIFEST_BUNDLE_DIGEST" ] && BUNDLE_PASS=true

echo "SCHEMA_DIGEST=$COMPUTED_SCHEMA_DIGEST"
echo "BUNDLE_DIGEST=$COMPUTED_BUNDLE_DIGEST"

if $SCHEMA_PASS && $BUNDLE_PASS; then
    echo "DIGEST_VERIFICATION=PASS"
    exit 0
else
    echo "DIGEST_VERIFICATION=FAIL"
    if ! $SCHEMA_PASS; then
        echo "  SCHEMA_DIGEST mismatch: manifest=$MANIFEST_SCHEMA_DIGEST computed=$COMPUTED_SCHEMA_DIGEST" >&2
    fi
    if ! $BUNDLE_PASS; then
        echo "  BUNDLE_DIGEST mismatch: manifest=$MANIFEST_BUNDLE_DIGEST computed=$COMPUTED_BUNDLE_DIGEST" >&2
    fi
    exit 1
fi
