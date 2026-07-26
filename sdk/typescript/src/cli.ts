#!/usr/bin/env node

import { readFile } from 'node:fs/promises';

import { WorkflowClient } from './client.js';
import { WorkflowError } from './error.js';
import type {
  CreateWorkflowInstanceRequest,
  DomainInstanceQuery,
  ExecuteWorkflowTransitionRequest,
  TimelineQuery,
  WorklistQuery,
} from './types.js';
import { validateDefinition, applyDefinitionFromFile } from './cli-definition.js';
import { applyDefinitionArtifact } from './definition-apply.js';

async function main(): Promise<void> {
  const [command, ...args] = process.argv.slice(2);
  if (command === undefined || command === '--help' || command === 'help') {
    showUsage();
    return;
  }

  // Local-only commands (no server needed)
  if (command === 'definition' && args[0] === 'validate') {
    const filePath = requiredValue(args, '--file');
    const result = await validateDefinition(filePath);
    writeJson(result);
    if (!result.valid) process.exitCode = 1;
    return;
  }

  const client = new WorkflowClient({
    baseUrl: requiredEnv('SVC_WORKFLOW_BASE_URL'),
    tokenProvider: () => requiredEnv('SVC_WORKFLOW_ACCESS_TOKEN'),
    requestTimeoutMs: optionalIntegerEnv('SVC_WORKFLOW_REQUEST_TIMEOUT_MS'),
    maxAttempts: optionalIntegerEnv('SVC_WORKFLOW_MAX_ATTEMPTS'),
  });
  const requestId = optionalValue(args, '--request-id');

  switch (command) {
    case 'create': {
      const input = await readJsonInput<CreateWorkflowInstanceRequest>(
        requiredValue(args, '--input'),
      );
      const result = await client.create(input, {
        idempotencyKey: requiredValue(args, '--idempotency-key'),
        requestId,
      });
      writeJson(result);
      return;
    }
    case 'list': {
      const query: DomainInstanceQuery = compact({
        domainId: requiredValue(args, '--domain-id'),
        beforeCreatedAt: optionalValue(args, '--before-created-at'),
        beforeId: optionalValue(args, '--before-id'),
        limit: optionalInteger(args, '--limit'),
        definitionKey: optionalValue(args, '--definition-key'),
        lifecycle: optionalLifecycle(args),
        currentNodeKey: optionalValue(args, '--current-node-key'),
        assigneePrincipalId: optionalValue(args, '--assignee-principal-id'),
      });
      writeJson(await client.listDomainInstances(query, { requestId }));
      return;
    }
    case 'worklist': {
      const query: WorklistQuery = compact({
        beforeCreatedAt: optionalValue(args, '--before-created-at'),
        beforeId: optionalValue(args, '--before-id'),
        limit: optionalInteger(args, '--limit'),
      });
      const kind = optionalValue(args, '--kind') ?? 'assigned';
      if (kind === 'assigned') {
        writeJson(await client.worklistAssignedToMe(query, { requestId }));
      } else if (kind === 'creator-drafts') {
        writeJson(await client.worklistCreatorOwnedDrafts(query, { requestId }));
      } else {
        throw new WorkflowError('--kind must be assigned or creator-drafts', {
          kind: 'input',
          operation: 'worklist',
        });
      }
      return;
    }
    case 'detail': {
      writeJson(
        await client.detail(requiredValue(args, '--instance-id'), { requestId }),
      );
      return;
    }
    case 'timeline': {
      const query: TimelineQuery = compact({
        after: optionalInteger(args, '--after'),
        limit: optionalInteger(args, '--limit'),
      });
      writeJson(
        await client.timeline(requiredValue(args, '--instance-id'), query, { requestId }),
      );
      return;
    }
    case 'transition': {
      const input = await readJsonInput<ExecuteWorkflowTransitionRequest>(
        requiredValue(args, '--input'),
      );
      const result = await client.transition(requiredValue(args, '--instance-id'), input, {
        idempotencyKey: requiredValue(args, '--idempotency-key'),
        requestId,
      });
      writeJson(result);
      return;
    }
    case 'definition': {
      const subcommand = args[0];
      if (subcommand === undefined) {
        throw new WorkflowError('definition requires a subcommand: apply | validate', {
          kind: 'input',
          operation: 'definition',
        });
      }
      switch (subcommand) {
        case 'apply': {
          const filePath = requiredValue(args, '--file');
          const result = await applyDefinitionFromFile(client, filePath, applyDefinitionArtifact);
          writeJson(result);
          return;
        }
        default:
          throw new WorkflowError('unknown definition subcommand', {
            kind: 'input',
            operation: `definition ${subcommand}`,
          });
      }
    }
    default:
      throw new WorkflowError(`unknown command: ${command}`, {
        kind: 'input',
        operation: command,
      });
  }
}

function requiredEnv(name: string): string {
  const value = process.env[name];
  if (value === undefined || value.length === 0) {
    throw new WorkflowError(`${name} is required`, { kind: 'configuration' });
  }
  return value;
}

function optionalIntegerEnv(name: string): number | undefined {
  const value = process.env[name];
  if (value === undefined) return undefined;
  return parseInteger(value, name);
}

function requiredValue(args: string[], name: string): string {
  const value = optionalValue(args, name);
  if (value === undefined) {
    throw new WorkflowError(`${name} is required`, { kind: 'input' });
  }
  return value;
}

function optionalValue(args: string[], name: string): string | undefined {
  const index = args.indexOf(name);
  if (index < 0) return undefined;
  const value = args[index + 1];
  if (value === undefined || value.startsWith('--')) {
    throw new WorkflowError(`${name} requires a value`, { kind: 'input' });
  }
  return value;
}

function optionalInteger(args: string[], name: string): number | undefined {
  const value = optionalValue(args, name);
  return value === undefined ? undefined : parseInteger(value, name);
}

function parseInteger(value: string, name: string): number {
  if (!/^-?\d+$/.test(value)) {
    throw new WorkflowError(`${name} must be an integer`, { kind: 'input' });
  }
  return Number(value);
}

function optionalLifecycle(args: string[]): DomainInstanceQuery['lifecycle'] {
  const value = optionalValue(args, '--lifecycle');
  if (value === undefined) return undefined;
  if (value === 'active' || value === 'terminal' || value === 'all') return value;
  throw new WorkflowError('--lifecycle must be active, terminal, or all', {
    kind: 'input',
  });
}

async function readJsonInput<T>(path: string): Promise<T> {
  const text = path === '-' ? await readStdin() : await readFile(path, 'utf8');
  try {
    return JSON.parse(text) as T;
  } catch (cause) {
    throw new WorkflowError('input is not valid JSON', { kind: 'input', cause });
  }
}

async function readStdin(): Promise<string> {
  let text = '';
  process.stdin.setEncoding('utf8');
  for await (const chunk of process.stdin) text += chunk;
  return text;
}

function compact<T extends Record<string, unknown>>(value: T): T {
  return Object.fromEntries(
    Object.entries(value).filter(([, entry]) => entry !== undefined),
  ) as T;
}

function writeJson(value: unknown): void {
  process.stdout.write(`${JSON.stringify(value)}\n`);
}

function showUsage(): void {
  process.stdout.write(`Workflow CLI Starter V1 (JSON output only)

Environment:
  SVC_WORKFLOW_BASE_URL
  SVC_WORKFLOW_ACCESS_TOKEN
  SVC_WORKFLOW_REQUEST_TIMEOUT_MS (optional)
  SVC_WORKFLOW_MAX_ATTEMPTS (optional, 1-3)

Commands:
  workflow-cli create --input <json-file|-> --idempotency-key <key>
  workflow-cli list --domain-id <uuid> [--before-created-at <rfc3339> --before-id <uuid>] [--limit <n>] [--definition-key <key>] [--lifecycle active|terminal|all] [--current-node-key <key>] [--assignee-principal-id <uuid>]
  workflow-cli worklist [--kind assigned|creator-drafts] [--before-created-at <rfc3339> --before-id <uuid>] [--limit <n>]
  workflow-cli detail --instance-id <uuid>
  workflow-cli timeline --instance-id <uuid> [--after <event-sequence>] [--limit <n>]
	  workflow-cli transition --instance-id <uuid> --input <json-file|-> --idempotency-key <key>

	  workflow-cli definition validate --file <definition-artifact.json>
	  workflow-cli definition apply --file <definition-artifact.json>

All commands accept --request-id <value>. Subcommands accept --file <json-file>.
`);
}

main().catch((error: unknown) => {
  if (error instanceof WorkflowError) {
    process.stderr.write(
      `${JSON.stringify({
        error: {
          message: error.message,
          kind: error.kind,
          operation: error.operation,
          status: error.status,
          code: error.code,
          details: error.details,
          attempts: error.attempts,
          requestId: error.requestId,
          responseRequestId: error.responseRequestId,
        },
      })}\n`,
    );
  } else {
    process.stderr.write(
      `${JSON.stringify({ error: { message: 'unexpected CLI failure', kind: 'protocol' } })}\n`,
    );
  }
  process.exitCode = 1;
});
