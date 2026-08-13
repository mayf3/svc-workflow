import { readFile } from 'node:fs/promises';

import { describe, expect, it } from 'vitest';

import {
  BUNDLE_DIGEST,
  CONTRACT_MAINLINE_HEAD_SHA,
  CONTRACT_VERSION,
  OWNER_HEAD_SHA,
} from '../src/constants.js';
import {
  createWorkflowInstanceRequestSchema,
  escalateAssistanceRequestSchema,
  requestAssistanceRequestSchema,
  resolveAssistanceRequestSchema,
} from '../src/schemas.js';

describe('Contract V1 lock and fixture', () => {
  it('matches the committed manifest', async () => {
    const manifest = JSON.parse(
      await readFile('contracts/workflow-http/v1/manifest.json', 'utf8'),
    ) as Record<string, unknown>;

    expect(CONTRACT_VERSION).toBe(manifest.contract_version);
    expect(BUNDLE_DIGEST).toBe(manifest.bundle_digest);
    expect(OWNER_HEAD_SHA).toBe(manifest.owner_head_sha);
    expect(CONTRACT_MAINLINE_HEAD_SHA).toMatch(/^[0-9a-f]{40}$/);
  });

  it('accepts the committed create request fixture', async () => {
    const fixture = JSON.parse(
      await readFile('contracts/workflow-http/v1/fixtures/create-request.json', 'utf8'),
    );
    expect(createWorkflowInstanceRequestSchema.parse(fixture)).toEqual(fixture);
  });

  it('rejects undeclared actor fields before transport', () => {
    const result = createWorkflowInstanceRequestSchema.safeParse({
      domainId: '22222222-2222-2222-2222-222222222222',
      definitionVersionId: '55555555-5555-5555-5555-555555555555',
      metadata: {},
      contextPayload: {},
      principalId: '11111111-1111-1111-1111-111111111111',
    });
    expect(result.success).toBe(false);
  });

  it('rejects non-JSON numeric values before serialization', () => {
    const result = createWorkflowInstanceRequestSchema.safeParse({
      domainId: '22222222-2222-4222-8222-222222222222',
      definitionVersionId: '55555555-5555-4555-8555-555555555555',
      metadata: { score: Number.POSITIVE_INFINITY },
      contextPayload: {},
    });
    expect(result.success).toBe(false);
  });

  it('accepts the frozen Workflow Assistance V1 fixtures', async () => {
    const [request, escalate, resolve] = await Promise.all([
      readFile('contracts/workflow-http/v1/fixtures/assistance-request.json', 'utf8'),
      readFile('contracts/workflow-http/v1/fixtures/assistance-escalate.json', 'utf8'),
      readFile('contracts/workflow-http/v1/fixtures/assistance-resolve.json', 'utf8'),
    ]);
    expect(() => requestAssistanceRequestSchema.parse(JSON.parse(request))).not.toThrow();
    expect(() => escalateAssistanceRequestSchema.parse(JSON.parse(escalate))).not.toThrow();
    expect(() => resolveAssistanceRequestSchema.parse(JSON.parse(resolve))).not.toThrow();
  });
});
