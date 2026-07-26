import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { canonicalize } from 'json-canonicalize';

import { WorkflowError } from './error.js';
import { definitionArtifactV1Schema } from './definition-artifact.js';
import { computeExpectedDefinitionDigest } from './definition-digest.js';

// ---------------------------------------------------------------------------
// CLI command: definition validate
// ---------------------------------------------------------------------------

export interface ValidateResult {
  valid: boolean;
  digest?: string;
  expectedDefinitionDigest?: string;
  errors?: Array<{ path: string; message: string }>;
}

/**
 * Validate a DefinitionArtifactV1 locally.
 * Does NOT access the server.
 */
export async function validateDefinition(filePath: string): Promise<ValidateResult> {
  const text = await readFile(filePath, 'utf8');
  let parsed: unknown;
  try {
    parsed = JSON.parse(text) as unknown;
  } catch {
    return { valid: false, errors: [{ path: '<root>', message: 'File is not valid JSON' }] };
  }

  const result = definitionArtifactV1Schema.safeParse(parsed);
  if (!result.success) {
    return {
      valid: false,
      errors: result.error.issues.map((issue) => ({
        path: issue.path.join('.'),
        message: issue.message,
      })),
    };
  }

  const artifact = result.data;
  const artifactDigest = `sha256:${createHash('sha256').update(canonicalize(artifact), 'utf8').digest('hex')}`;
  const expectedDefinitionDigest = computeExpectedDefinitionDigest(artifact);

  return {
    valid: true,
    digest: artifactDigest,
    expectedDefinitionDigest,
  };
}

// ---------------------------------------------------------------------------
// CLI command: definition apply
// ---------------------------------------------------------------------------

/**
 * Run the definition-apply command.
 * Called from the main CLI dispatcher which provides the client.
 */
export async function applyDefinitionFromFile(
  client: import('./client.js').WorkflowClient,
  filePath: string,
  applyFn: typeof import('./definition-apply.js').applyDefinitionArtifact,
): Promise<Record<string, unknown>> {
  const text = await readFile(filePath, 'utf8');
  let parsed: unknown;
  try {
    parsed = JSON.parse(text) as unknown;
  } catch {
    throw new WorkflowError('File is not valid JSON', { kind: 'input', operation: 'definition-apply' });
  }

  const result = definitionArtifactV1Schema.safeParse(parsed);
  if (!result.success) {
    throw new WorkflowError('Definition artifact validation failed', {
      kind: 'input',
      operation: 'definition-apply',
      details: result.error.issues,
    });
  }

  const applyResult = await applyFn(client, result.data);
  return applyResult as unknown as Record<string, unknown>;
}
