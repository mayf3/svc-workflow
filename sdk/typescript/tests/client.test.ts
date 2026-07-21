import { describe, expect, it, vi } from 'vitest';

import { WorkflowClient } from '../src/client.js';
import { WorkflowError } from '../src/error.js';

const INSTANCE_ID = '11111111-1111-4111-8111-111111111111';
const DOMAIN_ID = '22222222-2222-4222-8222-222222222222';
const DEFINITION_ID = '33333333-3333-4333-8333-333333333333';

function response(status: number, body: unknown, requestId = 'response-id'): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json', 'x-request-id': requestId },
  });
}

function createResponse() {
  return {
    workflowInstanceId: INSTANCE_ID,
    workflowStateVersion: 1,
    currentContextRevisionId: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',
    currentNodeVisitId: 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',
    eventSequence: 1,
  };
}

describe('WorkflowClient transport', () => {
  it('sends bearer, idempotency key, and one request ID', async () => {
    const fetchImplementation = vi.fn(async (_input: URL | RequestInfo, init?: RequestInit) => {
      const headers = new Headers(init?.headers);
      expect(headers.get('authorization')).toBe('Bearer test-token');
      expect(headers.get('idempotency-key')).toBe('create-key');
      expect(headers.get('x-request-id')).toBe('logical-request-id');
      return response(201, createResponse(), 'logical-request-id');
    });
    const client = new WorkflowClient({
      baseUrl: 'http://127.0.0.1:8989',
      tokenProvider: () => 'test-token',
      requestIdProvider: () => 'logical-request-id',
      maxAttempts: 1,
      fetchImplementation,
    });

    await expect(
      client.create(
        {
          domainId: DOMAIN_ID,
          definitionVersionId: DEFINITION_ID,
          metadata: {},
          contextPayload: {},
        },
        { idempotencyKey: 'create-key' },
      ),
    ).resolves.toEqual(createResponse());
  });

  it('retries 504 request_timeout with the same key and request ID', async () => {
    const seen: Array<{ key: string | null; requestId: string | null }> = [];
    const fetchImplementation = vi.fn(async (_input: URL | RequestInfo, init?: RequestInit) => {
      const headers = new Headers(init?.headers);
      seen.push({
        key: headers.get('idempotency-key'),
        requestId: headers.get('x-request-id'),
      });
      if (seen.length === 1) {
        return response(504, {
          error: { code: 'request_timeout', message: 'request timed out' },
        });
      }
      return response(201, createResponse());
    });
    const client = new WorkflowClient({
      baseUrl: 'http://127.0.0.1:8989',
      tokenProvider: () => 'test-token',
      requestIdProvider: () => 'retry-request-id',
      maxAttempts: 2,
      retryDelaysMs: [0],
      fetchImplementation,
    });

    await client.create(
      {
        domainId: DOMAIN_ID,
        definitionVersionId: DEFINITION_ID,
        metadata: {},
        contextPayload: {},
      },
      { idempotencyKey: 'same-key' },
    );
    expect(seen).toEqual([
      { key: 'same-key', requestId: 'retry-request-id' },
      { key: 'same-key', requestId: 'retry-request-id' },
    ]);
  });

  it('retries a response-body transport failure', async () => {
    let calls = 0;
    const client = new WorkflowClient({
      baseUrl: 'http://127.0.0.1:8989',
      tokenProvider: () => 'test-token',
      maxAttempts: 2,
      retryDelaysMs: [0],
      fetchImplementation: async () => {
        calls += 1;
        if (calls === 1) {
          return {
            ok: true,
            status: 201,
            headers: new Headers(),
            text: async () => {
              throw new TypeError('connection closed while reading body');
            },
          } as Response;
        }
        return response(201, createResponse());
      },
    });

    await expect(
      client.create(
        {
          domainId: DOMAIN_ID,
          definitionVersionId: DEFINITION_ID,
          metadata: {},
          contextPayload: {},
        },
        { idempotencyKey: 'body-retry-key' },
      ),
    ).resolves.toEqual(createResponse());
    expect(calls).toBe(2);
  });

  it('preserves the real Domain List next_cursor wire shape', async () => {
    const cursor = {
      created_at: '2026-07-18T12:00:00Z',
      id: INSTANCE_ID,
    };
    const client = new WorkflowClient({
      baseUrl: 'http://127.0.0.1:8989',
      tokenProvider: () => 'test-token',
      maxAttempts: 1,
      fetchImplementation: async () =>
        response(200, { items: [], next_cursor: cursor }),
    });

    const page = await client.listDomainInstances({ domainId: DOMAIN_ID });
    expect(page.next_cursor).toEqual(cursor);
    expect(page).not.toHaveProperty('nextCursor');
  });

  it('rejects a camelCase cursor alias as protocol drift', async () => {
    const client = new WorkflowClient({
      baseUrl: 'http://127.0.0.1:8989',
      tokenProvider: () => 'test-token',
      maxAttempts: 1,
      fetchImplementation: async () =>
        response(200, { items: [], nextCursor: null }),
    });

    await expect(
      client.listDomainInstances({ domainId: DOMAIN_ID }),
    ).rejects.toMatchObject({ kind: 'protocol' });
  });

  it('parses API errors with details and propagated request ID', async () => {
    const client = new WorkflowClient({
      baseUrl: 'http://127.0.0.1:8989',
      tokenProvider: () => 'test-token',
      maxAttempts: 1,
      requestIdProvider: () => 'request-id',
      fetchImplementation: async () =>
        response(
          409,
          {
            error: {
              code: 'workflow_state_version_conflict',
              message: 'workflow state version does not match',
              details: { expected: 1, actual: 2 },
            },
          },
          'request-id',
        ),
    });

    await expect(client.detail(INSTANCE_ID)).rejects.toMatchObject({
      kind: 'api',
      status: 409,
      code: 'workflow_state_version_conflict',
      details: { expected: 1, actual: 2 },
      requestId: 'request-id',
      responseRequestId: 'request-id',
    });
  });

  it('fails closed when the token provider returns empty', async () => {
    const fetchImplementation = vi.fn();
    const client = new WorkflowClient({
      baseUrl: 'http://127.0.0.1:8989',
      tokenProvider: () => '',
      fetchImplementation,
    });

    await expect(client.detail(INSTANCE_ID)).rejects.toBeInstanceOf(WorkflowError);
    expect(fetchImplementation).not.toHaveBeenCalled();
  });
});
