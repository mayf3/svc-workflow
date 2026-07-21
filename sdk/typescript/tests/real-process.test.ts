import { readFile } from 'node:fs/promises';

import { describe, expect, it } from 'vitest';

import { WorkflowClient } from '../src/client.js';
import { WorkflowError } from '../src/error.js';
import type { CreateWorkflowInstanceRequest } from '../src/types.js';

const baseUrl = process.env.WORKFLOW_SDK_TEST_BASE_URL;
const token = process.env.WORKFLOW_SDK_TEST_TOKEN;
const readToken = process.env.WORKFLOW_SDK_TEST_READ_TOKEN;
const domainId = process.env.WORKFLOW_SDK_TEST_DOMAIN_ID;
const transitionId = process.env.WORKFLOW_SDK_TEST_TRANSITION_ID;
const enabled = [baseUrl, token, readToken, domainId, transitionId].every(Boolean);

describe.skipIf(!enabled)('WorkflowClient against real svc-workflow', () => {
  it('covers the Contract V1 lifecycle, pagination, errors, and request ID', async () => {
    const observedRequestIds: string[] = [];
    const instrumentedFetch: typeof globalThis.fetch = async (input, init) => {
      const sent = new Headers(init?.headers).get('x-request-id');
      const result = await globalThis.fetch(input, init);
      if (sent !== null) {
        observedRequestIds.push(sent);
        expect(result.headers.get('x-request-id')).toBe(sent);
      }
      return result;
    };
    const client = new WorkflowClient({
      baseUrl: baseUrl!,
      tokenProvider: () => token!,
      requestIdProvider: () => 'sdk-real-process-request',
      maxAttempts: 2,
      retryDelaysMs: [0],
      fetchImplementation: instrumentedFetch,
    });
    const fixture = JSON.parse(
      await readFile('contracts/workflow-http/v1/fixtures/create-request.json', 'utf8'),
    ) as CreateWorkflowInstanceRequest;

    await expect(client.ready()).resolves.toEqual({ status: 'ready' });
    const created = await client.create(fixture, { idempotencyKey: 'sdk-fixture-create' });
    const replayed = await client.create(fixture, { idempotencyKey: 'sdk-fixture-create' });
    expect(replayed).toEqual(created);

    const detail = await client.detail(created.workflowInstanceId);
    expect(detail.visibility).toBe('full');
    const assigned = await client.worklistAssignedToMe({ limit: 20 });
    const drafts = await client.worklistCreatorOwnedDrafts({ limit: 20 });
    expect(assigned.items.some((item) => item.detail.instance.workflow_instance_id === created.workflowInstanceId)).toBe(true);
    expect(drafts.items.some((item) => item.detail.instance.workflow_instance_id === created.workflowInstanceId)).toBe(true);

    const beforeTransition = await client.timeline(created.workflowInstanceId, { limit: 20 });
    expect(beforeTransition.items).toHaveLength(1);
    expect(beforeTransition.items[0]).toHaveProperty('event_id');
    expect(beforeTransition.items[0]).not.toHaveProperty('eventId');

    const additionalIds: string[] = [];
    for (const suffix of ['two', 'three']) {
      const result = await client.create(
        {
          ...fixture,
          externalReference: `sdk-fixture-${suffix}`,
          contextPayload: { title: `SDK ${suffix}` },
        },
        { idempotencyKey: `sdk-create-${suffix}` },
      );
      additionalIds.push(result.workflowInstanceId);
    }

    const seen = new Set<string>();
    let cursor: { created_at: string; id: string } | null = null;
    do {
      const page = await client.listDomainInstances({
        domainId: domainId!,
        limit: 1,
        beforeCreatedAt: cursor?.created_at,
        beforeId: cursor?.id,
      });
      for (const item of page.items) {
        expect(seen.has(item.workflow_instance_id)).toBe(false);
        seen.add(item.workflow_instance_id);
      }
      cursor = page.next_cursor;
    } while (cursor !== null);
    expect(seen).toEqual(new Set([created.workflowInstanceId, ...additionalIds]));

    await expect(
      client.create(
        { ...fixture, contextPayload: { title: 'different request' } },
        { idempotencyKey: 'sdk-fixture-create' },
      ),
    ).rejects.toMatchObject({ status: 409, code: 'idempotency_conflict' });

    const transitioned = await client.transition(
      created.workflowInstanceId,
      {
        transitionDefinitionId: transitionId!,
        expectedWorkflowStateVersion: created.workflowStateVersion,
      },
      { idempotencyKey: 'sdk-transition' },
    );
    expect(transitioned.workflowStateVersion).toBe(2);
    const afterTransition = await client.timeline(created.workflowInstanceId, {
      after: beforeTransition.items[0]!.event_sequence,
      limit: 20,
    });
    expect(afterTransition.items).toHaveLength(1);

    const unauthenticated = new WorkflowClient({
      baseUrl: baseUrl!,
      tokenProvider: () => 'not-a-token',
      maxAttempts: 1,
    });
    await expect(unauthenticated.detail(created.workflowInstanceId)).rejects.toMatchObject({
      status: 401,
      code: 'unauthenticated',
    });

    const readOnly = new WorkflowClient({
      baseUrl: baseUrl!,
      tokenProvider: () => readToken!,
      maxAttempts: 1,
    });
    await expect(
      readOnly.create(
        { ...fixture, externalReference: 'read-only-create' },
        { idempotencyKey: 'read-only-create' },
      ),
    ).rejects.toMatchObject({ status: 403, code: 'forbidden' });
    await expect(client.detail('99999999-9999-4999-8999-999999999999')).rejects.toMatchObject({
      status: 404,
      code: 'workflow_instance_not_found_or_not_visible',
    });
    await expect(
      client.listDomainInstances({ domainId: domainId!, limit: 101 }),
    ).rejects.toBeInstanceOf(WorkflowError);

    expect(observedRequestIds.length).toBeGreaterThan(0);
    expect(new Set(observedRequestIds)).toEqual(new Set(['sdk-real-process-request']));
  });
});
