import { z } from 'zod';

import type { ZodType } from 'zod';

import { WorkflowError } from './error.js';
import {
  archiveDefinitionResponseSchema,
  createDefinitionRequestSchema,
  createDefinitionResponseSchema,
  createDraftVersionRequestSchema,
  createDraftVersionResponseSchema,
  createWorkflowInstanceRequestSchema,
  createWorkflowInstanceResponseSchema,
  creatorDraftPageSchema,
  definitionDetailResponseSchema,
  definitionListPageSchema,
  definitionListQuerySchema,
  domainInstancePageSchema,
  domainInstanceQuerySchema,
  errorEnvelopeSchema,
  executeWorkflowTransitionRequestSchema,
  executeWorkflowTransitionResponseSchema,
  healthResponseSchema,
  idempotencyKeySchema,
  memberAddResponseSchema,
  memberListPageSchema,
  memberListQuerySchema,
  memberRemoveResponseSchema,
  publishVersionRequestSchema,
  publishVersionResponseSchema,
  replaceDraftGraphRequestSchema,
  selfProjectionResponseSchema,
  timelineQuerySchema,
  timelineResponseSchema,
  versionResponseSchema,
  workflowInstanceDetailResponseSchema,
  worklistPageSchema,
  worklistQuerySchema,
} from './schemas.js';
import type {
  ArchiveDefinitionResponse,
  CreateDefinitionRequest,
  CreateDefinitionResponse,
  CreateDraftVersionRequest,
  CreateDraftVersionResponse,
  CreateWorkflowInstanceRequest,
  CreateWorkflowInstanceResponse,
  CreatorDraftPage,
  DefinitionDetailResponse,
  DefinitionListPage,
  DefinitionListQuery,
  DomainInstancePage,
  DomainInstanceQuery,
  ExecuteWorkflowTransitionRequest,
  ExecuteWorkflowTransitionResponse,
  MemberAddResponse,
  MemberListPage,
  MemberListQuery,
  MemberRemoveResponse,
  PublishVersionRequest,
  PublishVersionResponse,
  ReplaceDraftGraphRequest,
  RequestOptions,
  SelfProjectionResponse,
  TimelineQuery,
  TimelineResponse,
  WorkflowClientConfig,
  WorkflowInstanceDetailResponse,
  WorklistPage,
  WorklistQuery,
  WriteOptions,
} from './types.js';

const DEFAULT_TIMEOUT_MS = 35_000;
const DEFAULT_MAX_ATTEMPTS = 3;
const DEFAULT_RETRY_DELAYS_MS = [250, 500] as const;

type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE';

interface RequestSpec<T> {
  method: HttpMethod;
  path: string;
  operation: string;
  successSchema: z.ZodType<T>;
  body?: unknown;
  authenticated?: boolean;
  idempotencyKey?: string;
  requestId?: string;
}

export class WorkflowClient {
  private readonly baseUrl: URL;
  private readonly requestTimeoutMs: number;
  private readonly maxAttempts: number;
  private readonly retryDelaysMs: readonly number[];
  private readonly fetchImplementation: typeof globalThis.fetch;

  constructor(private readonly config: WorkflowClientConfig) {
    this.baseUrl = parseBaseUrl(config.baseUrl);
    this.requestTimeoutMs = config.requestTimeoutMs ?? DEFAULT_TIMEOUT_MS;
    this.maxAttempts = config.maxAttempts ?? DEFAULT_MAX_ATTEMPTS;
    this.retryDelaysMs = config.retryDelaysMs ?? DEFAULT_RETRY_DELAYS_MS;
    this.fetchImplementation = config.fetchImplementation ?? globalThis.fetch;

    if (!Number.isInteger(this.requestTimeoutMs) || this.requestTimeoutMs <= 0) {
      throw configurationError('requestTimeoutMs must be a positive integer');
    }
    if (!Number.isInteger(this.maxAttempts) || this.maxAttempts < 1 || this.maxAttempts > 3) {
      throw configurationError('maxAttempts must be an integer from 1 to 3');
    }
    if (typeof this.fetchImplementation !== 'function') {
      throw configurationError('fetchImplementation is required');
    }
    if (this.retryDelaysMs.some((delay) => !Number.isInteger(delay) || delay < 0)) {
      throw configurationError('retryDelaysMs must contain non-negative integers');
    }
  }

  async version(options: RequestOptions = {}) {
    return this.request({
      method: 'GET',
      path: '/version',
      operation: 'version',
      authenticated: false,
      successSchema: versionResponseSchema,
      requestId: options.requestId,
    });
  }

  async ready(options: RequestOptions = {}) {
    return this.request({
      method: 'GET',
      path: '/readyz',
      operation: 'ready',
      authenticated: false,
      successSchema: healthResponseSchema,
      requestId: options.requestId,
    });
  }

  async create(
    input: CreateWorkflowInstanceRequest,
    options: WriteOptions,
  ): Promise<CreateWorkflowInstanceResponse> {
    const body = parseInput(createWorkflowInstanceRequestSchema, input, 'create');
    const idempotencyKey = parseIdempotencyKey(options.idempotencyKey, 'create');
    return this.request({
      method: 'POST',
      path: '/internal/v1/workflow-instances',
      operation: 'create',
      body,
      idempotencyKey,
      successSchema: createWorkflowInstanceResponseSchema,
      requestId: options.requestId,
    });
  }

  async detail(
    workflowInstanceId: string,
    options: RequestOptions = {},
  ): Promise<WorkflowInstanceDetailResponse> {
    const id = parseUuid(workflowInstanceId, 'detail');
    return this.request({
      method: 'GET',
      path: `/internal/v1/workflow-instances/${encodeURIComponent(id)}`,
      operation: 'detail',
      successSchema: workflowInstanceDetailResponseSchema,
      requestId: options.requestId,
    });
  }

  async timeline(
    workflowInstanceId: string,
    query: TimelineQuery = {},
    options: RequestOptions = {},
  ): Promise<TimelineResponse> {
    const id = parseUuid(workflowInstanceId, 'timeline');
    const parsed = parseInput(timelineQuerySchema, query, 'timeline');
    const params = new URLSearchParams();
    setNumber(params, 'after', parsed.after);
    setNumber(params, 'limit', parsed.limit);
    return this.request({
      method: 'GET',
      path: withQuery(
        `/internal/v1/workflow-instances/${encodeURIComponent(id)}/timeline`,
        params,
      ),
      operation: 'timeline',
      successSchema: timelineResponseSchema,
      requestId: options.requestId,
    });
  }

  async transition(
    workflowInstanceId: string,
    input: ExecuteWorkflowTransitionRequest,
    options: WriteOptions,
  ): Promise<ExecuteWorkflowTransitionResponse> {
    const id = parseUuid(workflowInstanceId, 'transition');
    const body = parseInput(executeWorkflowTransitionRequestSchema, input, 'transition');
    const idempotencyKey = parseIdempotencyKey(options.idempotencyKey, 'transition');
    return this.request({
      method: 'POST',
      path: `/internal/v1/workflow-instances/${encodeURIComponent(id)}/transitions`,
      operation: 'transition',
      body,
      idempotencyKey,
      successSchema: executeWorkflowTransitionResponseSchema,
      requestId: options.requestId,
    });
  }

  async worklistAssignedToMe(
    query: WorklistQuery = {},
    options: RequestOptions = {},
  ): Promise<WorklistPage> {
    const params = worklistParams(parseInput(worklistQuerySchema, query, 'worklist'));
    return this.request({
      method: 'GET',
      path: withQuery('/internal/v1/worklists/assigned-to-me', params),
      operation: 'worklist-assigned-to-me',
      successSchema: worklistPageSchema,
      requestId: options.requestId,
    });
  }

  async worklistCreatorOwnedDrafts(
    query: WorklistQuery = {},
    options: RequestOptions = {},
  ): Promise<CreatorDraftPage> {
    const params = worklistParams(
      parseInput(worklistQuerySchema, query, 'creator-owned-drafts'),
    );
    return this.request({
      method: 'GET',
      path: withQuery('/internal/v1/worklists/creator-owned-drafts', params),
      operation: 'worklist-creator-owned-drafts',
      successSchema: creatorDraftPageSchema,
      requestId: options.requestId,
    });
  }

  async listDomainInstances(
    query: DomainInstanceQuery,
    options: RequestOptions = {},
  ): Promise<DomainInstancePage> {
    const parsed = parseInput(domainInstanceQuerySchema, query, 'domain-list');
    const params = new URLSearchParams({ domainId: parsed.domainId });
    setString(params, 'beforeCreatedAt', parsed.beforeCreatedAt);
    setString(params, 'beforeId', parsed.beforeId);
    setNumber(params, 'limit', parsed.limit);
    setString(params, 'definitionKey', parsed.definitionKey);
    setString(params, 'lifecycle', parsed.lifecycle);
    setString(params, 'currentNodeKey', parsed.currentNodeKey);
    setString(params, 'assigneePrincipalId', parsed.assigneePrincipalId);
    return this.request({
      method: 'GET',
      path: `/internal/v1/workflow-instances/domain?${params.toString()}`,
      operation: 'domain-list',
      successSchema: domainInstancePageSchema,
      requestId: options.requestId,
    });
  }

  async selfProject(options: RequestOptions = {}): Promise<SelfProjectionResponse> {
    return this.request({
      method: 'PUT',
      path: '/internal/v1/principals/me',
      operation: 'self-project',
      successSchema: selfProjectionResponseSchema,
      requestId: options.requestId,
    });
  }

  async listDomainMembers(
    domainId: string,
    query: MemberListQuery = {},
    options: RequestOptions = {},
  ): Promise<MemberListPage> {
    const id = parseUuid(domainId, 'list-domain-members');
    const parsed = parseInput(memberListQuerySchema, query, 'list-domain-members');
    const params = new URLSearchParams();
    setString(params, 'beforeCreatedAt', parsed.beforeCreatedAt);
    setString(params, 'beforeId', parsed.beforeId);
    setNumber(params, 'limit', parsed.limit);
    return this.request({
      method: 'GET',
      path: withQuery(
        `/internal/v1/domains/${encodeURIComponent(id)}/members`,
        params,
      ),
      operation: 'list-domain-members',
      successSchema: memberListPageSchema,
      requestId: options.requestId,
    });
  }

  async addDomainMember(
    domainId: string,
    principalId: string,
    options: WriteOptions,
  ): Promise<MemberAddResponse> {
    const domainUuid = parseUuid(domainId, 'add-domain-member');
    const principalUuid = parseUuid(principalId, 'add-domain-member');
    const idempotencyKey = parseIdempotencyKey(options.idempotencyKey, 'add-domain-member');
    return this.request({
      method: 'PUT',
      path: `/internal/v1/domains/${encodeURIComponent(domainUuid)}/members/${encodeURIComponent(principalUuid)}`,
      operation: 'add-domain-member',
      body: {},
      idempotencyKey,
      successSchema: memberAddResponseSchema,
      requestId: options.requestId,
    });
  }

  async removeDomainMember(
    domainId: string,
    principalId: string,
    options: WriteOptions,
  ): Promise<MemberRemoveResponse> {
    const domainUuid = parseUuid(domainId, 'remove-domain-member');
    const principalUuid = parseUuid(principalId, 'remove-domain-member');
    const idempotencyKey = parseIdempotencyKey(options.idempotencyKey, 'remove-domain-member');
    return this.request({
      method: 'DELETE',
      path: `/internal/v1/domains/${encodeURIComponent(domainUuid)}/members/${encodeURIComponent(principalUuid)}`,
      operation: 'remove-domain-member',
      idempotencyKey,
      successSchema: memberRemoveResponseSchema,
      requestId: options.requestId,
    });
  }

  // ---------------------------------------------------------------------------
  // Domain Definition Governance
  // ---------------------------------------------------------------------------

  async listDomainDefinitions(
    domainId: string,
    query: DefinitionListQuery = {},
    options: RequestOptions = {},
  ): Promise<DefinitionListPage> {
    const id = parseUuid(domainId, 'list-domain-definitions');
    const parsed = parseInput(definitionListQuerySchema, query, 'list-domain-definitions');
    const params = new URLSearchParams();
    setString(params, 'beforeCreatedAt', parsed.beforeCreatedAt);
    setString(params, 'beforeId', parsed.beforeId);
    setNumber(params, 'limit', parsed.limit);
    if (parsed.includeArchived === true) params.set('includeArchived', 'true');
    return this.request({
      method: 'GET',
      path: withQuery(
        `/internal/v1/domains/${encodeURIComponent(id)}/definitions`,
        params,
      ),
      operation: 'list-domain-definitions',
      successSchema: definitionListPageSchema,
      requestId: options.requestId,
    });
  }

  async getDomainDefinition(
    domainId: string,
    definitionId: string,
    options: RequestOptions = {},
  ): Promise<DefinitionDetailResponse> {
    const domainUuid = parseUuid(domainId, 'get-domain-definition');
    const defUuid = parseUuid(definitionId, 'get-domain-definition');
    return this.request({
      method: 'GET',
      path: `/internal/v1/domains/${encodeURIComponent(domainUuid)}/definitions/${encodeURIComponent(defUuid)}`,
      operation: 'get-domain-definition',
      successSchema: definitionDetailResponseSchema,
      requestId: options.requestId,
    });
  }

  async createDomainDefinition(
    domainId: string,
    input: CreateDefinitionRequest,
    options: WriteOptions,
  ): Promise<CreateDefinitionResponse> {
    const id = parseUuid(domainId, 'create-domain-definition');
    const body = parseInput(createDefinitionRequestSchema, input, 'create-domain-definition');
    const idempotencyKey = parseIdempotencyKey(options.idempotencyKey, 'create-domain-definition');
    return this.request({
      method: 'POST',
      path: `/internal/v1/domains/${encodeURIComponent(id)}/definitions`,
      operation: 'create-domain-definition',
      body,
      idempotencyKey,
      successSchema: createDefinitionResponseSchema,
      requestId: options.requestId,
    });
  }

  async createDefinitionVersion(
    domainId: string,
    definitionId: string,
    input: CreateDraftVersionRequest,
    options: WriteOptions,
  ): Promise<CreateDraftVersionResponse> {
    const domainUuid = parseUuid(domainId, 'create-definition-version');
    const defUuid = parseUuid(definitionId, 'create-definition-version');
    const body = parseInput(createDraftVersionRequestSchema, input, 'create-definition-version');
    const idempotencyKey = parseIdempotencyKey(options.idempotencyKey, 'create-definition-version');
    return this.request({
      method: 'POST',
      path: `/internal/v1/domains/${encodeURIComponent(domainUuid)}/definitions/${encodeURIComponent(defUuid)}/versions`,
      operation: 'create-definition-version',
      body,
      idempotencyKey,
      successSchema: createDraftVersionResponseSchema,
      requestId: options.requestId,
    });
  }

  async replaceDefinitionDraft(
    domainId: string,
    definitionId: string,
    input: ReplaceDraftGraphRequest,
    options: WriteOptions,
  ): Promise<{ status: string }> {
    const domainUuid = parseUuid(domainId, 'replace-definition-draft');
    const defUuid = parseUuid(definitionId, 'replace-definition-draft');
    const body = parseInput(replaceDraftGraphRequestSchema, input, 'replace-definition-draft');
    const idempotencyKey = parseIdempotencyKey(options.idempotencyKey, 'replace-definition-draft');
    return this.request({
      method: 'PUT',
      path: `/internal/v1/domains/${encodeURIComponent(domainUuid)}/definitions/${encodeURIComponent(defUuid)}/draft`,
      operation: 'replace-definition-draft',
      body,
      idempotencyKey,
      successSchema: z.object({ status: z.string() }).strict(),
      requestId: options.requestId,
    });
  }

  async publishDefinitionVersion(
    domainId: string,
    definitionId: string,
    input: PublishVersionRequest,
    options: WriteOptions,
  ): Promise<PublishVersionResponse> {
    const domainUuid = parseUuid(domainId, 'publish-definition-version');
    const defUuid = parseUuid(definitionId, 'publish-definition-version');
    const body = parseInput(publishVersionRequestSchema, input, 'publish-definition-version');
    const idempotencyKey = parseIdempotencyKey(options.idempotencyKey, 'publish-definition-version');
    return this.request({
      method: 'POST',
      path: `/internal/v1/domains/${encodeURIComponent(domainUuid)}/definitions/${encodeURIComponent(defUuid)}/publish`,
      operation: 'publish-definition-version',
      body,
      idempotencyKey,
      successSchema: publishVersionResponseSchema,
      requestId: options.requestId,
    });
  }

  async archiveDomainDefinition(
    domainId: string,
    definitionId: string,
    options: WriteOptions,
  ): Promise<ArchiveDefinitionResponse> {
    const domainUuid = parseUuid(domainId, 'archive-domain-definition');
    const defUuid = parseUuid(definitionId, 'archive-domain-definition');
    const idempotencyKey = parseIdempotencyKey(options.idempotencyKey, 'archive-domain-definition');
    return this.request({
      method: 'POST',
      path: `/internal/v1/domains/${encodeURIComponent(domainUuid)}/definitions/${encodeURIComponent(defUuid)}/archive`,
      operation: 'archive-domain-definition',
      body: {},
      idempotencyKey,
      successSchema: archiveDefinitionResponseSchema,
      requestId: options.requestId,
    });
  }

  private async request<T>(spec: RequestSpec<T>): Promise<T> {
    const requestId = resolveRequestId(spec.requestId, this.config.requestIdProvider);
    const token = spec.authenticated === false ? undefined : await this.resolveToken(spec.operation);
    const url = new URL(spec.path, this.baseUrl);
    const headers = new Headers({ Accept: 'application/json', 'X-Request-Id': requestId });
    if (token !== undefined) headers.set('Authorization', `Bearer ${token}`);
    if (spec.body !== undefined) headers.set('Content-Type', 'application/json');
    if (spec.idempotencyKey !== undefined) {
      headers.set('Idempotency-Key', spec.idempotencyKey);
    }
    const body = spec.body === undefined ? undefined : JSON.stringify(spec.body);

    for (let attempt = 1; attempt <= this.maxAttempts; attempt += 1) {
      let response: Response;
      let text: string;
      try {
        response = await this.fetchImplementation(url, {
          method: spec.method,
          headers,
          body,
          signal: AbortSignal.timeout(this.requestTimeoutMs),
        });
        text = await response.text();
      } catch (cause) {
        if (attempt < this.maxAttempts) {
          await this.waitBeforeRetry(attempt);
          continue;
        }
        const timeout = isTimeout(cause);
        throw new WorkflowError(
          timeout ? 'svc-workflow request timed out' : 'svc-workflow network failure',
          {
            kind: 'transport',
            operation: spec.operation,
            attempts: attempt,
            requestId,
            code: timeout ? 'timeout' : 'network_error',
            cause,
          },
        );
      }

      const responseRequestId = response.headers.get('x-request-id') ?? undefined;
      const parsed = parseJson(text);

      if (response.ok) {
        if (!parsed.ok) {
          throw protocolError(spec.operation, response.status, requestId, responseRequestId, attempt);
        }
        const validated = spec.successSchema.safeParse(parsed.value);
        if (!validated.success) {
          throw protocolError(
            spec.operation,
            response.status,
            requestId,
            responseRequestId,
            attempt,
            validated.error.issues,
          );
        }
        return validated.data;
      }

      const envelope = parsed.ok ? errorEnvelopeSchema.safeParse(parsed.value) : undefined;
      const error = envelope?.success ? envelope.data.error : undefined;
      if (attempt < this.maxAttempts && isRetryable(response.status, error?.code)) {
        await this.waitBeforeRetry(attempt);
        continue;
      }
      if (error !== undefined) {
        throw new WorkflowError(error.message, {
          kind: 'api',
          operation: spec.operation,
          status: response.status,
          code: error.code,
          details: error.details,
          attempts: attempt,
          requestId,
          responseRequestId,
        });
      }
      throw protocolError(
        spec.operation,
        response.status,
        requestId,
        responseRequestId,
        attempt,
      );
    }

    throw new WorkflowError('svc-workflow request attempts exhausted', {
      kind: 'protocol',
      operation: spec.operation,
      attempts: this.maxAttempts,
      requestId,
    });
  }

  private async resolveToken(operation: string): Promise<string> {
    let token: string;
    try {
      token = await this.config.tokenProvider();
    } catch (cause) {
      throw new WorkflowError('token provider failed', {
        kind: 'configuration',
        operation,
        cause,
      });
    }
    if (typeof token !== 'string' || token.length === 0) {
      throw new WorkflowError('token provider returned an empty token', {
        kind: 'configuration',
        operation,
      });
    }
    return token;
  }

  private async waitBeforeRetry(attempt: number): Promise<void> {
    const fallback = this.retryDelaysMs.at(-1) ?? 0;
    const delay = this.retryDelaysMs[attempt - 1] ?? fallback;
    if (delay > 0) await new Promise((resolve) => setTimeout(resolve, delay));
  }
}

function parseBaseUrl(value: string): URL {
  try {
    const url = new URL(value);
    if (url.protocol !== 'http:' && url.protocol !== 'https:') throw new Error('invalid protocol');
    return url;
  } catch (cause) {
    throw new WorkflowError('baseUrl must be an absolute HTTP(S) URL', {
      kind: 'configuration',
      cause,
    });
  }
}

function configurationError(message: string): WorkflowError {
  return new WorkflowError(message, { kind: 'configuration' });
}

function parseInput<T>(schema: z.ZodType<T>, value: unknown, operation: string): T {
  const result = schema.safeParse(value);
  if (!result.success) {
    throw new WorkflowError('request does not match Workflow Contract V1', {
      kind: 'input',
      operation,
      details: result.error.issues,
    });
  }
  return result.data;
}

function parseUuid(value: string, operation: string): string {
  return parseInput(z.string().uuid(), value, operation);
}

function parseIdempotencyKey(value: string, operation: string): string {
  return parseInput(idempotencyKeySchema, value, operation);
}

function resolveRequestId(value: string | undefined, provider: (() => string) | undefined): string {
  const requestId = value ?? provider?.() ?? globalThis.crypto.randomUUID();
  if (typeof requestId !== 'string' || requestId.length === 0) {
    throw new WorkflowError('request ID must be a non-empty string', { kind: 'input' });
  }
  try {
    new Headers({ 'X-Request-Id': requestId });
  } catch (cause) {
    throw new WorkflowError('request ID is not a valid HTTP header value', {
      kind: 'input',
      cause,
    });
  }
  return requestId;
}

function worklistParams(query: WorklistQuery): URLSearchParams {
  const params = new URLSearchParams();
  setString(params, 'beforeCreatedAt', query.beforeCreatedAt);
  setString(params, 'beforeId', query.beforeId);
  setNumber(params, 'limit', query.limit);
  return params;
}

function setString(params: URLSearchParams, key: string, value: string | undefined): void {
  if (value !== undefined) params.set(key, value);
}

function setNumber(params: URLSearchParams, key: string, value: number | undefined): void {
  if (value !== undefined) params.set(key, String(value));
}

function withQuery(path: string, params: URLSearchParams): string {
  const query = params.toString();
  return query.length === 0 ? path : `${path}?${query}`;
}

function parseJson(text: string): { ok: true; value: unknown } | { ok: false } {
  try {
    return { ok: true, value: JSON.parse(text) };
  } catch {
    return { ok: false };
  }
}

function isRetryable(status: number, code: string | undefined): boolean {
  return (
    status === 503 ||
    (status === 425 && code === 'command_still_processing') ||
    (status === 504 && code === 'request_timeout')
  );
}

function isTimeout(cause: unknown): boolean {
  return cause instanceof DOMException && (cause.name === 'AbortError' || cause.name === 'TimeoutError');
}

function protocolError(
  operation: string,
  status: number,
  requestId: string,
  responseRequestId: string | undefined,
  attempts: number,
  details?: unknown,
): WorkflowError {
  return new WorkflowError('svc-workflow response does not match Workflow Contract V1', {
    kind: 'protocol',
    operation,
    status,
    attempts,
    requestId,
    responseRequestId,
    details,
  });
}
