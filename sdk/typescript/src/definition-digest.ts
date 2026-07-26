import { createHash } from 'node:crypto';
import { canonicalize } from 'json-canonicalize';
import type { NodeDefinition, TransitionDefinition } from './definition-artifact.js';

// ---------------------------------------------------------------------------
// artifactDigest — SHA-256 of the full artifact (all fields) under JCS
// ---------------------------------------------------------------------------

/**
 * Compute the artifact digest — SHA-256 of the full artifact JSON under JCS.
 * Format: `sha256:<64 lowercase hex>`
 *
 * All artifact fields are included: identity, definition metadata, version metadata,
 * and the full nodes/transitions arrays.
 */
export function computeArtifactDigest(
  artifact: Record<string, unknown>,
): string {
  const canon = canonicalize(artifact);
  const hash = createHash('sha256').update(canon, 'utf8').digest('hex');
  return `sha256:${hash}`;
}

// ---------------------------------------------------------------------------
// expectedDefinitionDigest — must match Rust `definition_digest` algorithm
// ---------------------------------------------------------------------------

interface CanonicalNode {
  node_key: string;
  display_name: string;
  order_index: number;
  node_type: string;
  assignee_ref_type: string | null;
  fixed_principal_id: string | null;
  assignee_input_key: string | null;
  instructions: string | null;
  primary_advance_transition_key: string | null;
  metadata: unknown | null;
}

interface CanonicalTransition {
  transition_key: string;
  display_name: string;
  source_node_key: string;
  target_node_key: string;
  transition_effect: string;
  submission_schema: unknown | null;
  metadata: unknown | null;
}

interface CanonicalDefinitionDocument {
  definition_key: string;
  version_number: number;
  json_schema_dialect: string | null;
  validator_version: string | null;
  context_schema: unknown;
  nodes: CanonicalNode[];
  transitions: CanonicalTransition[];
}

function toCanonicalNode(node: NodeDefinition): CanonicalNode {
  return {
    node_key: node.nodeKey,
    display_name: node.displayName,
    order_index: node.orderIndex,
    node_type: node.nodeType,
    assignee_ref_type: node.assigneeRefType ?? null,
    fixed_principal_id: node.fixedPrincipalId ?? null,
    assignee_input_key: node.assigneeInputKey ?? null,
    instructions: node.instructions ?? null,
    primary_advance_transition_key: node.primaryAdvanceTransitionKey ?? null,
    metadata: node.metadata ?? null,
  };
}

function toCanonicalTransition(
  transition: TransitionDefinition,
): CanonicalTransition {
  return {
    transition_key: transition.transitionKey,
    display_name: transition.displayName,
    source_node_key: transition.sourceNodeKey,
    target_node_key: transition.targetNodeKey,
    transition_effect: transition.transitionEffect ?? 'ADVANCE',
    submission_schema: transition.submissionSchema ?? null,
    metadata: transition.metadata ?? null,
  };
}

/**
 * Compute the expected digest matching Rust `definition_digest` algorithm.
 *
 * - Nodes sorted by node_key
 * - Transitions sorted by transition_key
 * - JCS canonicalized + SHA-256
 * - Returns 64-char lowercase hex (no `sha256:` prefix)
 */
export function computeExpectedDefinitionDigest(params: {
  definitionKey: string;
  versionNumber: number;
  jsonSchemaDialect?: string | null;
  validatorVersion?: string | null;
  contextSchema?: unknown;
  nodes: NodeDefinition[];
  transitions: TransitionDefinition[];
}): string {
  const sortedNodes = [...params.nodes]
    .map(toCanonicalNode)
    .sort((a, b) => a.node_key.localeCompare(b.node_key));

  const sortedTransitions = [...params.transitions]
    .map(toCanonicalTransition)
    .sort((a, b) => a.transition_key.localeCompare(b.transition_key));

  const doc: CanonicalDefinitionDocument = {
    definition_key: params.definitionKey,
    version_number: params.versionNumber,
    json_schema_dialect: params.jsonSchemaDialect ?? null,
    validator_version: params.validatorVersion ?? null,
    context_schema: params.contextSchema ?? null,
    nodes: sortedNodes,
    transitions: sortedTransitions,
  };

  const canon = canonicalize(doc);
  return createHash('sha256').update(canon, 'utf8').digest('hex');
}
