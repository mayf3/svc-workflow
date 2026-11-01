import { describe, expect, it } from 'vitest';

import {
  assignedWorkItemSchema,
  creatorDraftItemSchema,
  fullWorkflowInstanceDetailSchema,
  workflowInstanceDetailResponseSchema,
} from '../src/schemas.js';

const ID = '11111111-1111-4111-8111-111111111111';
const DOMAIN = '22222222-2222-4222-8222-222222222222';
const VERSION = '33333333-3333-4333-8333-333333333333';
const ACTOR = '44444444-4444-4444-8444-444444444444';
const CONTEXT = '55555555-5555-4555-8555-555555555555';
const VISIT = '66666666-6666-4666-8666-666666666666';
const NODE = '77777777-7777-4777-8777-777777777777';
const TRANSITION = '88888888-8888-4888-8888-888888888888';
const CASE = '99999999-9999-4999-8999-999999999999';
const TIME = '2026-11-01T00:00:00Z';
const node = { node_id: NODE, node_key: 'task', display_name: 'Task', node_type: 'TASK' };

const sharedDetail = {
  instance: {
    workflow_instance_id: ID,
    domain_id: DOMAIN,
    definition_version_id: VERSION,
    definition_version_status: 'PUBLISHED',
    created_by_principal_id: ACTOR,
    workflow_state_version: 1,
    external_reference: null,
    external_url: null,
    metadata: {},
    created_at: TIME,
    domain_enabled: true,
    is_terminal: false,
    current_node: node,
  },
  current_context_revision_id: CONTEXT,
  current_node_visit_id: VISIT,
  current_context: {
    context_revision_id: CONTEXT,
    workflow_instance_id: ID,
    revision_number: 1,
    previous_revision_id: null,
    payload: {},
    payload_digest: 'digest',
    created_by_principal_id: ACTOR,
    created_at: TIME,
  },
  current_visit: {
    node_visit_id: VISIT,
    workflow_instance_id: ID,
    node,
    visit_number: 1,
    assignee_principal_id: ACTOR,
    assignee_canonical_agent_id: null,
    eligibility: { classification: 'ACTIONABLE_NOW' },
    entered_by_transition_id: null,
    instructions: null,
    created_at: TIME,
  },
  outgoing_transitions: [],
};

const actions = [
  { operation: 'transition', transition_id: TRANSITION, transition_effect: 'ADVANCE', request_validation: 'REQUIRED_AT_EXECUTION' },
  { operation: 'transition', transition_id: TRANSITION, transition_effect: 'RETURN', request_validation: 'REQUIRED_AT_EXECUTION' },
  { operation: 'transition', transition_id: TRANSITION, transition_effect: 'TERMINATE', request_validation: 'REQUIRED_AT_EXECUTION' },
  { operation: 'request_assistance', request_validation: 'REQUIRED_AT_EXECUTION' },
  { operation: 'escalate_assistance', assistance_case_id: CASE, request_validation: 'REQUIRED_AT_EXECUTION' },
  { operation: 'resolve_assistance', assistance_case_id: CASE, request_validation: 'REQUIRED_AT_EXECUTION' },
  { operation: 'cancel', request_validation: 'REQUIRED_AT_EXECUTION' },
  { operation: 'archive', request_validation: 'REQUIRED_AT_EXECUTION' },
];

function fullResponse(overrides: Record<string, unknown> = {}) {
  return {
    visibility: 'full',
    detail: {
      ...sharedDetail,
      authorized_actions_scope: 'INSTANCE_TASK_AND_GOVERNANCE_V1',
      authorized_actions_status: 'AVAILABLE',
      authorized_actions: actions,
      current_task_type: { source: 'CURRENT_NODE_TYPE', node_type: 'TASK', business_task_type: null },
      ...overrides,
    },
  };
}

describe('instance detail authorized action wire contract', () => {
  it('accepts the endpoint ten-field detail and all six exact operation variants', () => {
    const response = fullResponse();
    expect(workflowInstanceDetailResponseSchema.parse(response)).toEqual(response);
    expect(Object.keys(response.detail)).toHaveLength(10);
  });

  it('requires the four endpoint fields and rejects undeclared detail or envelope keys', () => {
    expect(workflowInstanceDetailResponseSchema.safeParse({ visibility: 'full', detail: sharedDetail }).success).toBe(false);
    for (const field of ['authorized_actions_scope', 'authorized_actions_status', 'authorized_actions', 'current_task_type']) {
      const response = fullResponse();
      delete (response.detail as Record<string, unknown>)[field];
      expect(workflowInstanceDetailResponseSchema.safeParse(response).success, field).toBe(false);
    }
    expect(workflowInstanceDetailResponseSchema.safeParse(fullResponse({ extra: true })).success).toBe(false);
    expect(workflowInstanceDetailResponseSchema.safeParse({ ...fullResponse(), extra: true }).success).toBe(false);
  });

  it('rejects mixed variant keys, missing keys, invented operations and unvalidated requests', () => {
    const invalid = [
      { ...actions[0], assistance_case_id: CASE },
      { ...actions[3], transition_id: TRANSITION },
      { ...actions[4], transition_effect: 'ADVANCE' },
      { ...actions[5], transition_id: TRANSITION },
      { ...actions[6], assistance_case_id: CASE },
      { ...actions[7], transition_id: TRANSITION },
      { operation: 'transition', transition_id: TRANSITION, request_validation: 'REQUIRED_AT_EXECUTION' },
      { operation: 'escalate_assistance', request_validation: 'REQUIRED_AT_EXECUTION' },
      { operation: 'resolve_assistance', assistance_case_id: CASE },
      { operation: 'wake', request_validation: 'REQUIRED_AT_EXECUTION' },
      { operation: 'cancel', request_validation: 'VALIDATED' },
      { ...actions[0], transition_effect: 'CANCEL' },
      { ...actions[0], transition_id: 'not-a-uuid' },
      { ...actions[4], assistance_case_id: null },
    ];
    for (const action of invalid) {
      expect(workflowInstanceDetailResponseSchema.safeParse(fullResponse({ authorized_actions: [action] })).success, JSON.stringify(action)).toBe(false);
    }
  });

  it('accepts only the named scope and status values with explicit empty arrays', () => {
    for (const status of ['NONE_FOR_ACTOR', 'MISSING_EXECUTE_SCOPE', 'WRITE_GATE_CLOSED', 'UNSUPPORTED_MODEL']) {
      const response = fullResponse({ authorized_actions_status: status, authorized_actions: [] });
      expect(workflowInstanceDetailResponseSchema.parse(response)).toEqual(response);
    }
    for (const overrides of [
      { authorized_actions_scope: 'ALL_ACTIONS' },
      { authorized_actions_status: 'UNKNOWN' },
      { authorized_actions: null },
    ]) {
      expect(workflowInstanceDetailResponseSchema.safeParse(fullResponse(overrides)).success).toBe(false);
    }
  });

  it('accepts only the closed current-node classification and null business task type', () => {
    for (const nodeType of ['DRAFT', 'NORMAL', 'TASK', 'TERMINAL']) {
      const response = fullResponse({ current_task_type: { source: 'CURRENT_NODE_TYPE', node_type: nodeType, business_task_type: null } });
      expect(workflowInstanceDetailResponseSchema.parse(response)).toEqual(response);
    }
    for (const taskType of [
      { source: 'INSTRUCTIONS', node_type: 'TASK', business_task_type: null },
      { source: 'CURRENT_NODE_TYPE', node_type: 'REVIEW', business_task_type: null },
      { source: 'CURRENT_NODE_TYPE', node_type: 'TASK', business_task_type: 'review' },
      { source: 'CURRENT_NODE_TYPE', node_type: 'TASK' },
      { source: 'CURRENT_NODE_TYPE', node_type: 'TASK', business_task_type: null, inferred: true },
    ]) {
      expect(workflowInstanceDetailResponseSchema.safeParse(fullResponse({ current_task_type: taskType })).success).toBe(false);
    }
  });

  it('keeps the shared six-field schema and both worklist detail schemas unchanged', () => {
    expect(fullWorkflowInstanceDetailSchema.parse(sharedDetail)).toEqual(sharedDetail);
    expect(fullWorkflowInstanceDetailSchema.safeParse(fullResponse().detail).success).toBe(false);
    const assigned = { detail: sharedDetail, upstream_submissions: [], return_feedback_events: [], submissions_truncated: false, return_events_truncated: false };
    const draft = { detail: sharedDetail, context_editable: false, combined_executable: false };
    expect(assignedWorkItemSchema.parse(assigned)).toEqual(assigned);
    expect(creatorDraftItemSchema.parse(draft)).toEqual(draft);
    expect(assignedWorkItemSchema.safeParse({ ...assigned, detail: fullResponse().detail }).success).toBe(false);
    expect(creatorDraftItemSchema.safeParse({ ...draft, detail: fullResponse().detail }).success).toBe(false);
  });

  it('preserves the restricted two-level shape and rejects new or private fields', () => {
    const restricted = {
      visibility: 'historical_participant',
      detail: {
        instance: {
          workflow_instance_id: ID,
          domain_id: DOMAIN,
          definition_version_id: VERSION,
          definition_version_status: 'PUBLISHED',
          workflow_state_version: 1,
          created_at: TIME,
          domain_enabled: true,
          is_terminal: false,
          current_node: node,
        },
      },
    };
    expect(workflowInstanceDetailResponseSchema.parse(restricted)).toEqual(restricted);
    expect(workflowInstanceDetailResponseSchema.safeParse({ ...restricted, detail: { ...restricted.detail, authorized_actions: [] } }).success).toBe(false);
    expect(workflowInstanceDetailResponseSchema.safeParse({ ...restricted, detail: { instance: { ...restricted.detail.instance, created_by_principal_id: ACTOR } } }).success).toBe(false);
  });
});
