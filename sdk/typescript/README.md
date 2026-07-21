# Workflow SDK and CLI Starter V1

This package is a thin TypeScript client for the frozen svc-workflow HTTP
Contract V1. It contains no Workflow Definition, product object, state-machine,
domain-authorization, token-signing, cache, or UI behavior.

## Contract lock

```text
CONTRACT_VERSION=1.0.0
BUNDLE_DIGEST=aff4f35b09b887eb8e83ffcd44eb4d487099d5f8911027cda172fce317dc9715
OWNER_HEAD_SHA=2dff1320d1488ff4d2137795df1622d61d01c00c
CONTRACT_MAINLINE_HEAD_SHA=ae81f1e04d41abba3e2cb957da30fbad4607b43d
```

Consumers must pin the exact independently audited SDK Git SHA. Do not depend
on a moving branch.

## Client

```ts
import { WorkflowClient } from '@workflow-foundation/sdk';

const client = new WorkflowClient({
  baseUrl: process.env.SVC_WORKFLOW_BASE_URL!,
  tokenProvider: () => process.env.SVC_WORKFLOW_ACCESS_TOKEN!,
});

const page = await client.worklistAssignedToMe({ limit: 20 });
const next = page.next_cursor;
```

Worklist and Domain List preserve their real composite cursor without mapping:

```json
{"created_at":"2026-07-18T00:00:00Z","id":"..."}
```

Timeline uses its separate numeric `after` / `nextCursor` event-sequence cursor.

Write methods require an explicit Idempotency-Key. The Token Provider only
returns a bearer token; the SDK never signs, exchanges, or interprets claims.
Each logical request sends one `X-Request-Id`, preserved across finite retries.

## CLI Starter

Build and show usage:

```bash
npm run build:sdk
node sdk/typescript/dist/cli.js --help
```

The CLI supports only generic `create`, `list`, `worklist`, `detail`, `timeline`,
and `transition` commands and writes structured JSON. Access tokens are read from
the environment, never from command-line flags.

## Verification

```bash
npm ci
npm run build:sdk
npm run test:sdk
cargo build --bin svc-workflow
sdk/typescript/integration/run.sh
```

The integration harness starts the real service against an isolated PostgreSQL
database and removes the database on exit.
