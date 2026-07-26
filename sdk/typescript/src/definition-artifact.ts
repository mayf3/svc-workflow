import { z } from 'zod';
import { jsonValueSchema, jsonObjectSchema } from './schemas.js';

// ---------------------------------------------------------------------------
// Explicit interfaces (not z.infer — avoids refine() type inference issues)
// ---------------------------------------------------------------------------

export interface NodeDefinition {
  nodeKey: string;
  displayName: string;
  orderIndex: number;
  nodeType: 'DRAFT' | 'NORMAL' | 'TERMINAL';
  assigneeRefType?: 'WORKFLOW_CREATOR' | 'FIXED_PRINCIPAL' | 'INSTANCE_INPUT_PRINCIPAL' | null;
  fixedPrincipalId?: string | null;
  assigneeInputKey?: string | null;
  instructions?: string | null;
  primaryAdvanceTransitionKey?: string | null;
  metadata?: Record<string, unknown> | null;
}

export interface TransitionDefinition {
  transitionKey: string;
  displayName: string;
  sourceNodeKey: string;
  targetNodeKey: string;
  transitionEffect?: 'ADVANCE' | 'RETURN' | 'SPLIT' | 'REVIEW';
  submissionSchema?: Record<string, unknown> | null;
  metadata?: Record<string, unknown> | null;
}

export interface DefinitionArtifactV1 {
  artifactVersion: 'definition-artifact-v1';
  domainId: string;
  definitionKey: string;
  displayName: string;
  description?: string | null;
  definitionMetadata?: Record<string, unknown> | null;
  versionNumber: number;
  versionMetadata?: Record<string, unknown> | null;
  jsonSchemaDialect?: string | null;
  validatorVersion?: string | null;
  contextSchema?: unknown | null;
  nodes: NodeDefinition[];
  transitions: TransitionDefinition[];
}

// ---------------------------------------------------------------------------
// Zod schemas for runtime validation
// ---------------------------------------------------------------------------

const nodeAssigneeRefTypeEnum = z.enum([
  'WORKFLOW_CREATOR',
  'FIXED_PRINCIPAL',
  'INSTANCE_INPUT_PRINCIPAL',
]);

const nodeTypeEnum = z.enum(['DRAFT', 'NORMAL', 'TERMINAL']);

const nodeDefFields = {
  nodeKey: z.string().min(1),
  displayName: z.string().min(1),
  orderIndex: z.number().int().nonnegative(),
  nodeType: nodeTypeEnum,
  assigneeRefType: nodeAssigneeRefTypeEnum.nullable().optional(),
  fixedPrincipalId: z.string().uuid().nullable().optional(),
  assigneeInputKey: z.string().nullable().optional(),
  instructions: z.string().nullable().optional(),
  primaryAdvanceTransitionKey: z.string().nullable().optional(),
  metadata: jsonObjectSchema.nullable().optional(),
};

export const nodeDefinitionSchema = z
  .object(nodeDefFields)
  .strict()
  .refine(
    (node) => {
      if (node.nodeType === 'TERMINAL') {
        return (
          node.assigneeRefType == null &&
          node.fixedPrincipalId == null &&
          node.assigneeInputKey == null
        );
      }
      return true;
    },
    { message: 'TERMINAL nodes must not have assignee fields' },
  )
  .refine(
    (node) => {
      if (node.nodeType !== 'TERMINAL') {
        if (node.assigneeRefType === 'FIXED_PRINCIPAL') return node.fixedPrincipalId != null;
        if (node.assigneeRefType === 'INSTANCE_INPUT_PRINCIPAL')
          return node.assigneeInputKey != null;
        return node.assigneeRefType != null;
      }
      return true;
    },
    { message: 'Non-TERMINAL nodes must have a valid assignee configuration' },
  );

const transitionEffectEnum = z
  .enum(['ADVANCE', 'RETURN', 'SPLIT', 'REVIEW'])
  .optional()
  .default('ADVANCE');

export const transitionDefinitionSchema = z
  .object({
    transitionKey: z.string().min(1),
    displayName: z.string().min(1),
    sourceNodeKey: z.string().min(1),
    targetNodeKey: z.string().min(1),
    transitionEffect: transitionEffectEnum,
    submissionSchema: jsonObjectSchema.nullable().optional(),
    metadata: jsonObjectSchema.nullable().optional(),
  })
  .strict();

export const definitionArtifactV1Schema = z
  .object({
    artifactVersion: z.literal('definition-artifact-v1'),
    domainId: z.string().uuid(),
    definitionKey: z.string().min(1).max(128),
    displayName: z.string().min(1).max(256),
    description: z.string().nullable().optional(),
    definitionMetadata: jsonObjectSchema.nullable().optional(),
    versionNumber: z.number().int().positive(),
    versionMetadata: jsonObjectSchema.nullable().optional(),
    jsonSchemaDialect: z.string().nullable().optional(),
    validatorVersion: z.string().nullable().optional(),
    contextSchema: jsonValueSchema.nullable().optional(),
    nodes: z.array(nodeDefinitionSchema).min(1),
    transitions: z.array(transitionDefinitionSchema).min(1),
  })
  .strict();
