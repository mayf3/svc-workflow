#!/usr/bin/env bash
set -euo pipefail

container="svc-workflow-cutover-v1-test"
port="55444"

cleanup() { docker rm -f "$container" >/dev/null 2>&1 || true; }
trap cleanup EXIT
cleanup
docker run -d --name "$container" -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=test-only \
  -e POSTGRES_DB=svc_workflow_dogfood_clean -p "127.0.0.1:${port}:5432" postgres:16 >/dev/null
for _ in $(seq 1 60); do
  docker exec "$container" pg_isready -U postgres -d svc_workflow_dogfood_clean >/dev/null 2>&1 && break
  sleep 1
done
export CUTOVER_TEST_DATABASE_URL="postgres://postgres:test-only@127.0.0.1:${port}/svc_workflow_dogfood_clean"
cargo test --test 36_legacy_successor_cutover_operator -- --nocapture --test-threads=1
