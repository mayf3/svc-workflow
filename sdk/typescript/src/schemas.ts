import { z } from 'zod';

export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };

export const jsonValueSchema: z.ZodType<JsonValue> = z.lazy(() =>
  z.union([
    z.null(),
    z.boolean(),
    z.number().finite(),
    z.string(),
    z.array(jsonValueSchema),
    z.record(jsonValueSchema),
  ]),
);

const jsonObjectSchema = z.record(jsonValueSchema);
const uuidSchema = z.string().uuid();
const dateTimeSchema = z.string().datetime({ offset: true });

export const idempotencyKeySchema = z
  .string()
  .min(1)
  .max(128)
  .regex(/^[\x21-\x7e]+$/, 'must contain only visible ASCII characters');

export const createWorkflowInstanceRequestSchema = z
  .object({
    domainId: uuidSchema,
    definitionVersionId: uuidSchema,
    externalReference: z.string().max(512).optional(),
    externalUrl: z.string().url().optional(),
    metadata: jsonObjectSchema,
    contextPayload: jsonObjectSchema,
  })
  .strict();

export const createWorkflowInstanceResponseSchema = z
  .object({
    workflowInstanceId: uuidSchema,
    workflowStateVersion: z.number().int(),
    currentContextRevisionId: uuidSchema,
    currentNodeVisitId: uuidSchema,
    eventSequence: z.number().int(),
  })
  .strict();

export const executeWorkflowTransitionRequestSchema = z
  .object({
    transitionDefinitionId: uuidSchema,
    expectedWorkflowStateVersion: z.number().int(),
    submissionPayload: jsonObjectSchema.optional(),
  })
  .strict();

export const executeWorkflowTransitionResponseSchema = z
  .object({
    workflowInstanceId: uuidSchema,
    workflowStateVersion: z.number().int(),
    currentContextRevisionId: uuidSchema,
    sourceNodeVisitId: uuidSchema,
    currentNodeVisitId: uuidSchema,
    submissionId: uuidSchema.nullable(),
    eventSequence: z.number().int(),
  })
  .strict();

export const publicNodeSummarySchema = z
  .object({
    node_id: uuidSchema,
    node_key: z.string(),
    display_name: z.string(),
    node_type: z.string(),
  })
  .strict();

export const workflowInstanceSummarySchema = z
  .object({
    workflow_instance_id: uuidSchema,
    domain_id: uuidSchema,
    definition_version_id: uuidSchema,
    definition_version_status: z.string(),
    created_by_principal_id: uuidSchema,
    workflow_state_version: z.number().int(),
    external_reference: z.string().nullable(),
    external_url: z.string().nullable(),
    metadata: jsonValueSchema.nullable(),
    created_at: dateTimeSchema,
    domain_enabled: z.boolean(),
    is_terminal: z.boolean(),
    current_node: publicNodeSummarySchema,
  })
  .strict();

export const contextRevisionSchema = z
  .object({
    context_revision_id: uuidSchema,
    workflow_instance_id: uuidSchema,
    revision_number: z.number().int(),
    previous_revision_id: uuidSchema.nullable(),
    payload: jsonValueSchema,
    payload_digest: z.string(),
    created_by_principal_id: uuidSchema,
    created_at: dateTimeSchema,
  })
  .strict();

export const nodeVisitSchema = z
  .object({
    node_visit_id: uuidSchema,
    workflow_instance_id: uuidSchema,
    node: publicNodeSummarySchema,
    visit_number: z.number().int(),
    assignee_principal_id: uuidSchema.nullable(),
    entered_by_transition_id: uuidSchema.nullable(),
    instructions: z.string().nullable(),
    created_at: dateTimeSchema,
  })
  .strict();

export const outgoingTransitionSchema = z
  .object({
    transition_id: uuidSchema,
    transition_key: z.string(),
    display_name: z.string(),
    transition_effect: z.string(),
    target_node: publicNodeSummarySchema,
    submission_schema: jsonValueSchema.nullable(),
    executable_for_actor: z.boolean(),
    blocked_reason: z.string().nullable(),
  })
  .strict();

export const fullWorkflowInstanceDetailSchema = z
  .object({
    instance: workflowInstanceSummarySchema,
    current_context_revision_id: uuidSchema,
    current_node_visit_id: uuidSchema,
    current_context: contextRevisionSchema,
    current_visit: nodeVisitSchema,
    outgoing_transitions: z.array(outgoingTransitionSchema),
  })
  .strict();

export const historicalParticipantSummarySchema = z
  .object({
    workflow_instance_id: uuidSchema,
    domain_id: uuidSchema,
    definition_version_id: uuidSchema,
    definition_version_status: z.string(),
    workflow_state_version: z.number().int(),
    created_at: dateTimeSchema,
    domain_enabled: z.boolean(),
    is_terminal: z.boolean(),
    current_node: publicNodeSummarySchema,
  })
  .strict();

export const workflowInstanceDetailResponseSchema = z.discriminatedUnion('visibility', [
  z
    .object({
      visibility: z.literal('full'),
      detail: fullWorkflowInstanceDetailSchema,
    })
    .strict(),
  z
    .object({
      visibility: z.literal('historical_participant'),
      detail: z.object({ instance: historicalParticipantSummarySchema }).strict(),
    })
    .strict(),
]);

export const workflowEventSchema = z
  .object({
    event_id: uuidSchema,
    workflow_instance_id: uuidSchema,
    event_sequence: z.number().int(),
    event_schema_version: z.string(),
    command_id: uuidSchema.nullable(),
    causation_id: uuidSchema.nullable(),
    correlation_id: uuidSchema.nullable(),
    event_type: z.string(),
    transition_effect: z.string().nullable(),
    source_node_visit_id: uuidSchema.nullable(),
    target_node_visit_id: uuidSchema.nullable(),
    context_revision_id: uuidSchema.nullable(),
    submission_id: uuidSchema.nullable(),
    event_data: jsonValueSchema.nullable(),
    event_data_digest: z.string().nullable(),
    actor_principal_id: uuidSchema,
    from_node_id: uuidSchema.nullable(),
    to_node_id: uuidSchema.nullable(),
    old_workflow_state_version: z.number().int(),
    new_workflow_state_version: z.number().int(),
    created_at: dateTimeSchema,
  })
  .strict();

export const timelineResponseSchema = z
  .object({
    items: z.array(workflowEventSchema),
    nextCursor: z.number().int().nullable(),
  })
  .strict();

export const timelineQuerySchema = z
  .object({
    after: z.number().int().nonnegative().optional(),
    limit: z.number().int().min(1).max(100).optional(),
  })
  .strict();

export const submissionHistoryItemSchema = z
  .object({
    submission_id: uuidSchema,
    workflow_instance_id: uuidSchema,
    source_node_visit_id: uuidSchema,
    source_node: publicNodeSummarySchema,
    context_revision_id: uuidSchema,
    author_principal_id: uuidSchema,
    transition_id: uuidSchema,
    transition_effect: z.string(),
    payload: jsonValueSchema,
    payload_digest: z.string(),
    schema_version: z.string(),
    created_at: dateTimeSchema,
  })
  .strict();

export const assignedWorkItemSchema = z
  .object({
    detail: fullWorkflowInstanceDetailSchema,
    upstream_submissions: z.array(submissionHistoryItemSchema),
    return_feedback_events: z.array(workflowEventSchema),
    submissions_truncated: z.boolean(),
    return_events_truncated: z.boolean(),
  })
  .strict();

export const creatorDraftItemSchema = z
  .object({
    detail: fullWorkflowInstanceDetailSchema,
    context_editable: z.boolean(),
    combined_executable: z.boolean(),
  })
  .strict();

export const cursorSchema = z
  .object({
    created_at: dateTimeSchema,
    id: uuidSchema,
  })
  .strict();

export const worklistPageSchema = z
  .object({
    items: z.array(assignedWorkItemSchema),
    next_cursor: cursorSchema.nullable(),
  })
  .strict();

export const creatorDraftPageSchema = z
  .object({
    items: z.array(creatorDraftItemSchema),
    next_cursor: cursorSchema.nullable(),
  })
  .strict();

export const worklistQuerySchema = z
  .object({
    beforeCreatedAt: dateTimeSchema.optional(),
    beforeId: uuidSchema.optional(),
    limit: z.number().int().min(1).max(100).optional(),
  })
  .strict()
  .refine(
    (query) => Boolean(query.beforeCreatedAt) === Boolean(query.beforeId),
    'beforeCreatedAt and beforeId must be provided together',
  );

export const domainInstanceSummarySchema = z
  .object({
    workflow_instance_id: uuidSchema,
    domain_id: uuidSchema,
    definition_version_id: uuidSchema,
    definition_key: z.string(),
    created_by_principal_id: uuidSchema,
    current_assignee_principal_id: uuidSchema.nullable(),
    current_node: publicNodeSummarySchema,
    is_terminal: z.boolean(),
    title: z.string().nullable(),
    created_at: dateTimeSchema,
    updated_at: dateTimeSchema,
  })
  .strict();

export const domainInstancePageSchema = z
  .object({
    items: z.array(domainInstanceSummarySchema),
    next_cursor: cursorSchema.nullable(),
  })
  .strict();

export const domainInstanceQuerySchema = z
  .object({
    domainId: uuidSchema,
    beforeCreatedAt: dateTimeSchema.optional(),
    beforeId: uuidSchema.optional(),
    limit: z.number().int().min(1).max(100).optional(),
    definitionKey: z.string().optional(),
    lifecycle: z.enum(['active', 'terminal', 'all']).optional(),
    currentNodeKey: z.string().optional(),
    assigneePrincipalId: uuidSchema.optional(),
  })
  .strict()
  .refine(
    (query) => Boolean(query.beforeCreatedAt) === Boolean(query.beforeId),
    'beforeCreatedAt and beforeId must be provided together',
  );

// ---------------------------------------------------------------------------
// Self-Projection & Domain Member schemas
// ---------------------------------------------------------------------------

export const selfProjectionResponseSchema = z
  .object({
    principalId: uuidSchema,
    created: z.boolean(),
  })
  .strict();

export const memberItemSchema = z
  .object({
    principalId: uuidSchema,
    principalType: z.string(),
    displayName: z.string(),
    role: z.literal('DOMAIN_MEMBER'),
    bindingCreatedAt: dateTimeSchema,
  })
  .strict();

export const memberListCursorSchema = z
  .object({
    created_at: dateTimeSchema,
    id: uuidSchema,
  })
  .strict();

export const memberListPageSchema = z
  .object({
    items: z.array(memberItemSchema),
    next_cursor: memberListCursorSchema.nullable(),
  })
  .strict();

export const memberListQuerySchema = z
  .object({
    beforeCreatedAt: dateTimeSchema.optional(),
    beforeId: uuidSchema.optional(),
    limit: z.number().int().min(1).max(100).optional(),
  })
  .strict()
  .refine(
    (query) => Boolean(query.beforeCreatedAt) === Boolean(query.beforeId),
    'beforeCreatedAt and beforeId must be provided together',
  );

export const memberAddResponseSchema = z
  .object({
    domainId: uuidSchema,
    principalId: uuidSchema,
    role: z.literal('DOMAIN_MEMBER'),
  })
  .strict();

export const memberRemoveResponseSchema = z
  .object({
    domainId: uuidSchema,
    principalId: uuidSchema,
    role: z.literal('DOMAIN_MEMBER'),
    enabled: z.literal(false),
  })
  .strict();

export const errorEnvelopeSchema = z
  .object({
    error: z
      .object({
        code: z.string(),
        message: z.string(),
        details: z.unknown().optional(),
      })
      .strict(),
  })
  .strict();

export const versionResponseSchema = z
  .object({
    service: z.literal('svc-workflow'),
    version: z.string(),
    gitSha: z.string(),
    schemaVersion: z.string(),
    apiContractVersion: z.string(),
  })
  .strict();

export const healthResponseSchema = z
  .object({ status: z.string() })
  .strict();
