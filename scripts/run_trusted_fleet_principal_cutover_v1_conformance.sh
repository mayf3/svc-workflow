#!/usr/bin/env bash
# Disposable PostgreSQL conformance for the exact trusted-fleet cutover.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NAME="svc-workflow-fleet-cutover-$$"
TARGET="${TRUSTED_FLEET_CARGO_TARGET_DIR:-$ROOT/target/trusted-fleet-conformance}"

cleanup() {
  set +e
  docker rm -f "$NAME" >/dev/null 2>&1
}
trap cleanup EXIT

docker run -d --rm --name "$NAME" \
  -e POSTGRES_PASSWORD=postgres -e POSTGRES_USER=postgres \
  -P postgres:16-alpine >/dev/null
PORT="$(docker port "$NAME" 5432/tcp | awk -F: 'NR==1{print $NF}')"
BASE="postgres://postgres:postgres@127.0.0.1:${PORT}"
for _ in $(seq 1 60); do
  if docker exec "$NAME" pg_isready -U postgres >/dev/null 2>&1; then break; fi
  sleep 1
done
docker exec "$NAME" pg_isready -U postgres >/dev/null

docker exec "$NAME" createdb -U postgres svc_workflow_cutover_test
docker exec "$NAME" createdb -U postgres auth_cutover_test
for migration in "$ROOT"/migrations/*.sql; do
  psql "${BASE}/svc_workflow_cutover_test" -v ON_ERROR_STOP=1 -f "$migration" >/dev/null
done

export TEST_WORKFLOW_DATABASE_URL="${BASE}/svc_workflow_cutover_test"
export TEST_AUTH_DATABASE_URL="${BASE}/auth_cutover_test"
export CARGO_TARGET_DIR="$TARGET"
export RUSTFLAGS="${RUSTFLAGS:-} --cfg trusted_fleet_cutover_conformance --check-cfg=cfg(trusted_fleet_cutover_conformance)"

cd "$ROOT"
cargo test --locked --test 27_trusted_fleet_principal_cutover_v1 -- --test-threads=1
printf '[trusted-fleet-cutover] PASS plan_sha256=%s\n' \
  '0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606'
