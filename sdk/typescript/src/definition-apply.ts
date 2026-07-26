import { createHash } from 'node:crypto';
import { canonicalize } from 'json-canonicalize';
import type { WorkflowClient } from './client.js';
import type { DefinitionArtifactV1, NodeDefinition, TransitionDefinition } from './definition-artifact.js';
import { computeExpectedDefinitionDigest } from './definition-digest.js';
import { WorkflowError } from './error.js';
import type { DefinitionItem, PublishVersionResponse } from './types.js';

// ---------------------------------------------------------------------------
// Idempotency-Key generation
// ---------------------------------------------------------------------------

function computeOperationKey(
  operation: string,
  endpoint: string,
  body: Record<string, unknown>,
  extra?: string,
): string {
  const payload = operation + endpoint + canonicalize(body) + (extra ?? '');
  const hash = createHash('sha256').update(payload, 'utf8').digest('hex');
  return `definition-apply-v1:${operation}:${hash}`;
}

// ---------------------------------------------------------------------------
// Apply result type (client-side only)
// ---------------------------------------------------------------------------

export type ApplyStatus = 'APPLIED' | 'ALREADY_APPLIED';

export interface DefinitionApplyResultV1 {
  status: ApplyStatus;
  artifactDigest: string;
  expectedDefinitionDigest: string;
  publishedDigest: string | null;
  workflowDefinitionId: string;
  definitionVersionId: string | null;
  versionNumber: number;
  idempotencyKeys: string[];
  operations: string[];
}

export interface DefinitionApplyOptions {
  signal?: AbortSignal;
}

// ---------------------------------------------------------------------------
// Pagination helper
// ---------------------------------------------------------------------------

async function collectAllDefinitions(
  client: WorkflowClient,
  domainId: string,
  includeArchived: boolean,
): Promise<DefinitionItem[]> {
  const all: DefinitionItem[] = [];
  let cursor: { created_at: string; id: string } | null = null;
  const limit = 100;

  for (let page = 0; page < 100; page += 1) {
    const result = await client.listDomainDefinitions(domainId, {
      ...(cursor !== null ? { beforeCreatedAt: cursor.created_at, beforeId: cursor.id } : {}),
      limit,
      includeArchived,
    });
    // Server wraps each item in { definition, nodes, transitions, version }
    for (const item of result.items) {
      all.push(item.definition);
    }
    if (result.next_cursor === null) break;
    cursor = result.next_cursor;
  }

  return all;
}

// ---------------------------------------------------------------------------
// Body builders
// ---------------------------------------------------------------------------

function buildCreateDefBody(artifact: DefinitionArtifactV1): Record<string, unknown> {
  const body: Record<string, unknown> = {
    definitionKey: artifact.definitionKey,
    displayName: artifact.displayName,
  };
  if (artifact.description != null) body.description = artifact.description;
  if (artifact.definitionMetadata != null) body.metadata = artifact.definitionMetadata;
  return body;
}

function buildCreateVersionBody(artifact: DefinitionArtifactV1): Record<string, unknown> {
  const body: Record<string, unknown> = {};
  if (artifact.contextSchema != null) body.contextSchema = artifact.contextSchema;
  if (artifact.jsonSchemaDialect != null) body.jsonSchemaDialect = artifact.jsonSchemaDialect;
  if (artifact.validatorVersion != null) body.validatorVersion = artifact.validatorVersion;
  if (artifact.versionMetadata != null) body.metadata = artifact.versionMetadata;
  return body;
}

function buildReplaceGraphBody(
  definitionVersionId: string,
  artifact: DefinitionArtifactV1,
): Record<string, unknown> {
  // Server expects snake_case for RawNodeDefinition / RawTransitionDefinition fields
  const nodes = artifact.nodes.map((n: NodeDefinition) => {
    const node: Record<string, unknown> = {
      node_key: n.nodeKey,
      display_name: n.displayName,
      order_index: n.orderIndex,
      node_type: n.nodeType,
    };
    if (n.assigneeRefType != null) node.assignee_ref_type = n.assigneeRefType;
    if (n.fixedPrincipalId != null) node.fixed_principal_id = n.fixedPrincipalId;
    if (n.assigneeInputKey != null) node.assignee_input_key = n.assigneeInputKey;
    if (n.instructions != null) node.instructions = n.instructions;
    if (n.primaryAdvanceTransitionKey != null) {
      node.primary_advance_transition_key = n.primaryAdvanceTransitionKey;
    }
    if (n.metadata != null) node.metadata = n.metadata;
    return node;
  });

  const transitions = artifact.transitions.map((t: TransitionDefinition) => {
    const transition: Record<string, unknown> = {
      transition_key: t.transitionKey,
      display_name: t.displayName,
      source_node_key: t.sourceNodeKey,
      target_node_key: t.targetNodeKey,
      transition_effect: t.transitionEffect ?? 'ADVANCE',
    };
    if (t.submissionSchema != null) transition.submission_schema = t.submissionSchema;
    if (t.metadata != null) transition.metadata = t.metadata;
    return transition;
  });

  // Top-level fields use camelCase (ReplaceDraftGraphBody has rename_all = "camelCase")
  return {
    definitionVersionId,
    ...(artifact.contextSchema != null ? { contextSchema: artifact.contextSchema } : {}),
    nodes,
    transitions,
  };
}

// ---------------------------------------------------------------------------
// Apply orchestration
// ---------------------------------------------------------------------------

export async function applyDefinitionArtifact(
  client: WorkflowClient,
  artifact: DefinitionArtifactV1,
  options?: DefinitionApplyOptions,
): Promise<DefinitionApplyResultV1> {
  const { domainId, definitionKey, versionNumber } = artifact;

  // Compute digests
  const expectedDefinitionDigest = computeExpectedDefinitionDigest(artifact);
  const artifactDigest = `sha256:${expectedDefinitionDigest}`;

  // -----------------------------------------------------------------------
  // 1. Find existing definition (full pagination, include archived)
  // -----------------------------------------------------------------------
  const existingDefs = await collectAllDefinitions(client, domainId, true);
  const existingDef = existingDefs.find(
    (d: DefinitionItem) => d.definition_key === definitionKey,
  );

  if (existingDef !== undefined) {
    // Definition identity check
    if (existingDef.display_name !== artifact.displayName) {
      throw new WorkflowError('Definition displayName mismatch', {
        kind: 'api',
        operation: 'definition-identity-check',
        code: 'DEFINITION_IDENTITY_MISMATCH',
        details: {
          expected: artifact.displayName,
          actual: existingDef.display_name,
        },
      });
    }

    if (existingDef.archived) {
      throw new WorkflowError('Definition is archived', {
        kind: 'api',
        operation: 'definition-archived-check',
        code: 'DEFINITION_ARCHIVED',
        details: { workflowDefinitionId: existingDef.id },
      });
    }

    const expectedDesc = artifact.description ?? null;
    const actualDesc = existingDef.description ?? null;
    const expectedMeta = artifact.definitionMetadata ?? null;
    const actualMeta = existingDef.metadata ?? null;

    if (
      expectedDesc !== actualDesc ||
      JSON.stringify(expectedMeta) !== JSON.stringify(actualMeta)
    ) {
      throw new WorkflowError('Definition identity metadata mismatch', {
        kind: 'api',
        operation: 'definition-identity-check',
        code: 'DEFINITION_IDENTITY_MISMATCH',
        details: {
          description: { expected: expectedDesc, actual: actualDesc },
        },
      });
    }
  }

  // -----------------------------------------------------------------------
  // 2. Check existing versions for the target versionNumber
  // -----------------------------------------------------------------------
  let workflowDefinitionId: string = existingDef?.id ?? '';
  type VersionInfo = { id: string; versionNumber: number; versionStatus: string; digest: string | null };
  let existingVersions: VersionInfo[] = [];

  if (workflowDefinitionId !== '') {
    const detail = await client.getDomainDefinition(domainId, workflowDefinitionId) as unknown as {
      definition: Record<string, unknown>;
      versions: Array<{ version: Record<string, unknown> | null }>;
    };
    existingVersions = detail.versions
      .filter((v): v is { version: Record<string, unknown> } => v.version !== null)
      .map((v) => ({
        id: (v.version as { id: string }).id,
        versionNumber: (v.version as { version_number: number }).version_number,
        versionStatus: (v.version as { version_status: string }).version_status,
        digest: (v.version as { definition_digest: string | null }).definition_digest,
      }));
  }

  const existingTargetVersion = existingVersions.find(
    (v: VersionInfo) => v.versionNumber === versionNumber,
  );

  if (existingTargetVersion !== undefined) {
    if (
      existingTargetVersion.versionStatus === 'PUBLISHED' &&
      existingTargetVersion.digest === expectedDefinitionDigest
    ) {
      return {
        status: 'ALREADY_APPLIED' as const,
        artifactDigest,
        expectedDefinitionDigest,
        publishedDigest: existingTargetVersion.digest,
        workflowDefinitionId,
        definitionVersionId: existingTargetVersion.id,
        versionNumber,
        idempotencyKeys: [],
        operations: [],
      };
    }

    if (
      existingTargetVersion.versionStatus !== 'DRAFT' &&
      existingTargetVersion.digest !== expectedDefinitionDigest
    ) {
      throw new WorkflowError('Definition version digest mismatch', {
        kind: 'api',
        operation: 'definition-apply',
        code: 'DEFINITION_VERSION_DIGEST_MISMATCH',
        details: {
          definitionKey,
          versionNumber,
          expectedDigest: expectedDefinitionDigest,
          actualDigest: existingTargetVersion.digest,
        },
      });
    }
  }

  // -----------------------------------------------------------------------
  // 3. Create or reuse definition
  // -----------------------------------------------------------------------
  const idempotencyKeys: string[] = [];
  const operations: string[] = [];

  if (workflowDefinitionId === '') {
    const createDefBody = buildCreateDefBody(artifact);
    const createKey = computeOperationKey(
      'create-definition',
      `POST /internal/v1/domains/${domainId}/definitions`,
      createDefBody,
    );
    idempotencyKeys.push(createKey);
    operations.push('create-definition');

    // Use type assertion for the create body — the data is valid at runtime
    const created = await client.createDomainDefinition(
      domainId,
      createDefBody as unknown as Parameters<typeof client.createDomainDefinition>[1],
      { idempotencyKey: createKey },
    );
    workflowDefinitionId = created.workflowDefinitionId;
    if (options?.signal?.aborted) {
      throw new WorkflowError('Apply cancelled', { kind: 'configuration', operation: 'definition-apply' });
    }
  }

  // -----------------------------------------------------------------------
  // 4. Version sequence check
  // -----------------------------------------------------------------------
  if (existingTargetVersion === undefined) {
    const maxExistingVersion = existingVersions.reduce(
      (max: number, v: VersionInfo) => Math.max(max, v.versionNumber),
      0,
    );
    const expectedNextVersion = maxExistingVersion + 1;

    if (versionNumber !== expectedNextVersion) {
      throw new WorkflowError('Definition version sequence mismatch', {
        kind: 'api',
        operation: 'definition-apply',
        code: 'DEFINITION_VERSION_SEQUENCE_MISMATCH',
        details: { definitionKey, expectedNextVersion, requestedVersion: versionNumber },
      });
    }
  }

  // -----------------------------------------------------------------------
  // 5. Create or reuse draft version
  // -----------------------------------------------------------------------
  let definitionVersionId: string;

  if (existingTargetVersion !== undefined && existingTargetVersion.versionStatus === 'DRAFT') {
    definitionVersionId = existingTargetVersion.id;
  } else {
    const createVersionBody = buildCreateVersionBody(artifact);
    const createVersionKey = computeOperationKey(
      'create-version',
      `POST /internal/v1/domains/${domainId}/definitions/${workflowDefinitionId}/versions`,
      createVersionBody,
      String(versionNumber), // differentiate versions even when body is empty
    );
    idempotencyKeys.push(createVersionKey);
    operations.push('create-version');

    const draftVersion = await client.createDefinitionVersion(
      domainId,
      workflowDefinitionId,
      createVersionBody as unknown as Parameters<typeof client.createDefinitionVersion>[2],
      { idempotencyKey: createVersionKey },
    );

    definitionVersionId = draftVersion.definitionVersionId;

    if (draftVersion.versionNumber !== versionNumber) {
      throw new WorkflowError('Server assigned unexpected version number', {
        kind: 'api',
        operation: 'definition-apply',
        code: 'DEFINITION_VERSION_SEQUENCE_MISMATCH',
        details: { expected: versionNumber, actual: draftVersion.versionNumber, definitionVersionId },
      });
    }

    if (options?.signal?.aborted) {
      throw new WorkflowError('Apply cancelled', { kind: 'configuration', operation: 'definition-apply' });
    }
  }

  // -----------------------------------------------------------------------
  // 6. Replace draft graph
  // -----------------------------------------------------------------------
  const graphBody = buildReplaceGraphBody(definitionVersionId, artifact);
  const replaceKey = computeOperationKey(
    'replace-draft',
    `PUT /internal/v1/domains/${domainId}/definitions/${workflowDefinitionId}/draft`,
    graphBody,
  );
  idempotencyKeys.push(replaceKey);
  operations.push('replace-draft');

  await client.replaceDefinitionDraft(
    domainId,
    workflowDefinitionId,
    graphBody as unknown as Parameters<typeof client.replaceDefinitionDraft>[2],
    { idempotencyKey: replaceKey },
  );

  if (options?.signal?.aborted) {
    throw new WorkflowError('Apply cancelled', { kind: 'configuration', operation: 'definition-apply' });
  }

  // -----------------------------------------------------------------------
  // 7. Publish version
  // -----------------------------------------------------------------------
  const publishBody: Record<string, unknown> = {
    versionId: definitionVersionId,
    expectedRevision: expectedDefinitionDigest,
  };
  const publishKey = computeOperationKey(
    'publish-version',
    `POST /internal/v1/domains/${domainId}/definitions/${workflowDefinitionId}/publish`,
    publishBody,
  );
  idempotencyKeys.push(publishKey);
  operations.push('publish-version');

  const publishResponse = await client.publishDefinitionVersion(
    domainId,
    workflowDefinitionId,
    publishBody as unknown as Parameters<typeof client.publishDefinitionVersion>[2],
    { idempotencyKey: publishKey },
  );

  if (options?.signal?.aborted) {
    throw new WorkflowError('Apply cancelled', { kind: 'configuration', operation: 'definition-apply' });
  }

  return {
    status: 'APPLIED',
    artifactDigest,
    expectedDefinitionDigest,
    publishedDigest: publishResponse.digest,
    workflowDefinitionId,
    definitionVersionId,
    versionNumber,
    idempotencyKeys,
    operations,
  };
}
