import { describe, it, expect, vi, beforeEach } from 'vitest';
import { applyDefinitionArtifact } from '../src/definition-apply.js';
import type { WorkflowClient } from '../src/client.js';
import type { DefinitionArtifactV1 } from '../src/definition-artifact.js';

// ---------------------------------------------------------------------------
// Helper: build a mock list item matching server response format
// ---------------------------------------------------------------------------

function mockDefItem(id: string, displayName: string, defKey: string, overrides?: Record<string, unknown>) {
  return {
    id,
    domain_id: '550e8400-e29b-41d4-a716-446655440000',
    definition_key: defKey,
    display_name: displayName,
    description: null,
    metadata: null,
    created_at: '2026-07-01T00:00:00Z',
    updated_at: '2026-07-01T00:00:00Z',
    archived: false,
    archived_at: null,
    archived_by_principal_id: null,
    ...overrides,
  };
}

function mockListItem(definition: ReturnType<typeof mockDefItem>, version?: Record<string, unknown> | null) {
  return {
    definition,
    nodes: [],
    transitions: [],
    version: version ?? null,
  };
}

function mockVersionItem(id: string, versionNumber: number, status: string, digest: string | null) {
  return {
    id,
    workflow_definition_id: 'def-1111',
    version_number: versionNumber,
    version_status: status,
    definition_digest: digest,
    json_schema_dialect: null,
    validator_version: null,
    context_schema: null,
    submission_schema: null,
    metadata: null,
    created_at: '2026-07-01T00:00:00Z',
    updated_at: '2026-07-01T00:00:00Z',
    published_at: status === 'PUBLISHED' ? '2026-07-01T00:00:00Z' : null,
    deprecated_at: null,
    revoked_at: null,
    published_by_principal_id: null,
    deprecated_by_principal_id: null,
    revoked_by_principal_id: null,
  };
}

function mockDetailResponse(def: ReturnType<typeof mockDefItem>, versions: Array<Record<string, unknown>>) {
  return {
    definition: def,
    versions: versions.map((v) => ({
      definition: def,
      nodes: [],
      transitions: [],
      version: v,
    })),
  };
}

// ---------------------------------------------------------------------------
// Test artifact
// ---------------------------------------------------------------------------

function createTestArtifact(overrides?: Partial<DefinitionArtifactV1>): DefinitionArtifactV1 {
  return {
    artifactVersion: 'definition-artifact-v1',
    domainId: '550e8400-e29b-41d4-a716-446655440000',
    definitionKey: 'test-def',
    displayName: 'Test Definition',
    versionNumber: 1,
    nodes: [
      { nodeKey: 'start', displayName: 'Start', orderIndex: 0, nodeType: 'DRAFT', assigneeRefType: 'WORKFLOW_CREATOR', primaryAdvanceTransitionKey: 'adv' },
      { nodeKey: 'end', displayName: 'End', orderIndex: 1, nodeType: 'TERMINAL' },
    ],
    transitions: [
      { transitionKey: 'adv', displayName: 'Advance', sourceNodeKey: 'start', targetNodeKey: 'end', transitionEffect: 'ADVANCE' },
    ],
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Mock client
// ---------------------------------------------------------------------------

function createMockClient(): WorkflowClient {
  return {
    listDomainDefinitions: vi.fn(),
    getDomainDefinition: vi.fn(),
    createDomainDefinition: vi.fn(),
    createDefinitionVersion: vi.fn(),
    replaceDefinitionDraft: vi.fn(),
    publishDefinitionVersion: vi.fn(),
  } as unknown as WorkflowClient;
}

describe('applyDefinitionArtifact', () => {
  let client: WorkflowClient;

  beforeEach(() => {
    client = createMockClient();
  });

  it('APPLIES a new definition artifact', async () => {
    vi.mocked(client.listDomainDefinitions).mockResolvedValue({
      items: [],
      next_cursor: null,
    });

    vi.mocked(client.createDomainDefinition).mockResolvedValue({
      workflowDefinitionId: 'def-1111',
      domainId: '550e8400-e29b-41d4-a716-446655440000',
      definitionKey: 'test-def',
      displayName: 'Test Definition',
      createdAt: '2026-07-26T00:00:00Z',
    });

    vi.mocked(client.createDefinitionVersion).mockResolvedValue({
      definitionVersionId: 'ver-1111',
      workflowDefinitionId: 'def-1111',
      versionNumber: 1,
      versionStatus: 'DRAFT',
      createdAt: '2026-07-26T00:00:00Z',
    });

    vi.mocked(client.replaceDefinitionDraft).mockResolvedValue({ status: 'ok' });

    vi.mocked(client.publishDefinitionVersion).mockResolvedValue({
      definitionVersionId: 'ver-1111',
      versionNumber: 1,
      versionStatus: 'PUBLISHED',
      digest: 'a'.repeat(64),
      publishedAt: '2026-07-26T00:00:01Z',
    });

    const result = await applyDefinitionArtifact(client, createTestArtifact());
    expect(result.status).toBe('APPLIED');
    expect(result.workflowDefinitionId).toBe('def-1111');
    expect(result.definitionVersionId).toBe('ver-1111');
    expect(result.versionNumber).toBe(1);
    expect(result.idempotencyKeys).toHaveLength(4);
    expect(result.operations).toEqual(['create-definition', 'create-version', 'replace-draft', 'publish-version']);
  });

  it('returns ALREADY_APPLIED when same version and digest exists', async () => {
    const artifact = createTestArtifact();
    const { computeExpectedDefinitionDigest } = await import('../src/definition-digest.js');
    const digest = computeExpectedDefinitionDigest(artifact);

    const defItem = mockDefItem('def-1111', 'Test Definition', 'test-def');
    const verItem = mockVersionItem('ver-1111', 1, 'PUBLISHED', digest);

    vi.mocked(client.listDomainDefinitions).mockResolvedValue({
      items: [mockListItem(defItem, verItem)],
      next_cursor: null,
    });

    vi.mocked(client.getDomainDefinition).mockResolvedValue(mockDetailResponse(defItem, [verItem]) as never);

    const result = await applyDefinitionArtifact(client, artifact);
    expect(result.status).toBe('ALREADY_APPLIED');
    expect(result.workflowDefinitionId).toBe('def-1111');
    expect(result.definitionVersionId).toBe('ver-1111');
    expect(result.versionNumber).toBe(1);
    expect(vi.mocked(client.createDomainDefinition)).not.toHaveBeenCalled();
    expect(vi.mocked(client.createDefinitionVersion)).not.toHaveBeenCalled();
    expect(vi.mocked(client.replaceDefinitionDraft)).not.toHaveBeenCalled();
    expect(vi.mocked(client.publishDefinitionVersion)).not.toHaveBeenCalled();
  });

  it('throws DEFINITION_VERSION_DIGEST_MISMATCH when same version has different digest', async () => {
    const defItem = mockDefItem('def-1111', 'Test Definition', 'test-def');
    const verItem = mockVersionItem('ver-1111', 1, 'PUBLISHED', 'a'.repeat(64));

    vi.mocked(client.listDomainDefinitions).mockResolvedValue({
      items: [mockListItem(defItem, verItem)],
      next_cursor: null,
    });

    vi.mocked(client.getDomainDefinition).mockResolvedValue(mockDetailResponse(defItem, [verItem]) as never);

    await expect(applyDefinitionArtifact(client, createTestArtifact())).rejects.toMatchObject({
      code: 'DEFINITION_VERSION_DIGEST_MISMATCH',
    });
  });

  it('throws DEFINITION_VERSION_SEQUENCE_MISMATCH when version number is wrong', async () => {
    const artifact = createTestArtifact({ versionNumber: 5 });
    const defItem = mockDefItem('def-1111', 'Test Definition', 'test-def');
    const ver1 = mockVersionItem('ver-1', 1, 'PUBLISHED', 'b'.repeat(64));

    vi.mocked(client.listDomainDefinitions).mockResolvedValue({
      items: [mockListItem(defItem, ver1)],
      next_cursor: null,
    });

    vi.mocked(client.getDomainDefinition).mockResolvedValue(mockDetailResponse(defItem, [ver1]) as never);

    await expect(applyDefinitionArtifact(client, artifact)).rejects.toMatchObject({
      code: 'DEFINITION_VERSION_SEQUENCE_MISMATCH',
    });
  });

  it('throws DEFINITION_IDENTITY_MISMATCH when displayName differs', async () => {
    const defItem = mockDefItem('def-1111', 'Old Name', 'test-def');

    vi.mocked(client.listDomainDefinitions).mockResolvedValue({
      items: [mockListItem(defItem)],
      next_cursor: null,
    });

    await expect(applyDefinitionArtifact(client, createTestArtifact({ displayName: 'New Name' }))).rejects.toMatchObject({
      code: 'DEFINITION_IDENTITY_MISMATCH',
    });
  });

  it('throws DEFINITION_ARCHIVED when definition is archived', async () => {
    const defItem = mockDefItem('def-1111', 'Test Definition', 'test-def', { archived: true, archived_at: '2026-07-02T00:00:00Z', archived_by_principal_id: '550e8400-e29b-41d4-a716-446655449999' });

    vi.mocked(client.listDomainDefinitions).mockResolvedValue({
      items: [mockListItem(defItem)],
      next_cursor: null,
    });

    await expect(applyDefinitionArtifact(client, createTestArtifact())).rejects.toMatchObject({
      code: 'DEFINITION_ARCHIVED',
    });
  });

  it('recovers from a failed replace-draft by reusing the existing DRAFT version', async () => {
    const defItem = mockDefItem('def-1111', 'Test Definition', 'test-def');
    const draftVer = mockVersionItem('ver-1111', 1, 'DRAFT', null);

    vi.mocked(client.listDomainDefinitions).mockResolvedValue({
      items: [mockListItem(defItem, draftVer)],
      next_cursor: null,
    });

    vi.mocked(client.getDomainDefinition).mockResolvedValue(mockDetailResponse(defItem, [draftVer]) as never);

    vi.mocked(client.replaceDefinitionDraft).mockResolvedValue({ status: 'ok' });
    vi.mocked(client.publishDefinitionVersion).mockResolvedValue({
      definitionVersionId: 'ver-1111',
      versionNumber: 1,
      versionStatus: 'PUBLISHED',
      digest: 'a'.repeat(64),
      publishedAt: '2026-07-26T00:00:01Z',
    });

    const result = await applyDefinitionArtifact(client, createTestArtifact());
    expect(result.status).toBe('APPLIED');
    expect(vi.mocked(client.createDefinitionVersion)).not.toHaveBeenCalled();
    expect(vi.mocked(client.replaceDefinitionDraft)).toHaveBeenCalled();
    expect(vi.mocked(client.publishDefinitionVersion)).toHaveBeenCalled();
  });
});
