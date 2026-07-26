import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { validateDefinition } from '../src/cli-definition.js';

// ---------------------------------------------------------------------------
// CLI definition validate command tests
// ---------------------------------------------------------------------------

describe('validateDefinition', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), 'cli-def-test-'));
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it('returns valid=false for a non-artifact JSON file', async () => {
    const filePath = join(tmpDir, 'not-a-def.json');
    writeFileSync(filePath, JSON.stringify({ not: 'an artifact' }), 'utf8');
    const result = await validateDefinition(filePath);
    expect(result.valid).toBe(false);
    expect(result.errors).toBeDefined();
    expect(result.errors!.length).toBeGreaterThan(0);
  });

  it('returns valid=false for non-JSON content', async () => {
    const filePath = join(tmpDir, 'bad.txt');
    writeFileSync(filePath, 'this is not json', 'utf8');
    const result = await validateDefinition(filePath);
    expect(result.valid).toBe(false);
  });

  it('returns valid=true with digest for a valid artifact', async () => {
    const artifact = {
      artifactVersion: 'definition-artifact-v1',
      domainId: '550e8400-e29b-41d4-a716-446655440000',
      definitionKey: 'test-def',
      displayName: 'Test Definition',
      versionNumber: 1,
      nodes: [
        {
          nodeKey: 'start',
          displayName: 'Start',
          orderIndex: 0,
          nodeType: 'DRAFT',
          assigneeRefType: 'WORKFLOW_CREATOR',
        },
        {
          nodeKey: 'end',
          displayName: 'End',
          orderIndex: 1,
          nodeType: 'TERMINAL',
        },
      ],
      transitions: [
        {
          transitionKey: 'advance',
          displayName: 'Advance',
          sourceNodeKey: 'start',
          targetNodeKey: 'end',
          transitionEffect: 'ADVANCE',
        },
      ],
    };
    const filePath = join(tmpDir, 'valid-artifact.json');
    writeFileSync(filePath, JSON.stringify(artifact), 'utf8');
    const result = await validateDefinition(filePath);
    expect(result.valid).toBe(true);
    expect(result.digest).toMatch(/^sha256:[0-9a-f]{64}$/);
    expect(result.expectedDefinitionDigest).toMatch(/^[0-9a-f]{64}$/);
  });

  it('returns valid=false for artifact with missing required fields', async () => {
    const artifact = {
      artifactVersion: 'definition-artifact-v1',
      // missing definitionKey
      displayName: 'Bad',
      versionNumber: 1,
      nodes: [],
      transitions: [],
    };
    const filePath = join(tmpDir, 'bad-artifact.json');
    writeFileSync(filePath, JSON.stringify(artifact), 'utf8');
    const result = await validateDefinition(filePath);
    expect(result.valid).toBe(false);
    expect(result.errors).toBeDefined();
  });
});
