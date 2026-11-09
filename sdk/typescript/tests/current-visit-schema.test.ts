import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import {
  fullWorkflowInstanceDetailSchema,
  nodeVisitSchema,
  workflowInstanceDetailResponseSchema,
} from '../src/schemas.js';

// Captured from the isolated real HTTP fixture; the adjacent provenance file
// binds the unedited response to its original capture and implementation base.
const captured = JSON.parse(
  readFileSync('sdk/typescript/tests/fixtures/current-detail.actual.json', 'utf8'),
);

function endpointResponse() {
  return {
    ...structuredClone(captured),
    detail: {
      ...structuredClone(captured.detail),
      authorized_actions_scope: 'INSTANCE_TASK_AND_GOVERNANCE_V1',
      authorized_actions_status: 'NONE_FOR_ACTOR',
      authorized_actions: [],
      current_task_type: {
        source: 'CURRENT_NODE_TYPE',
        node_type: 'TASK',
        business_task_type: null,
      },
    },
  };
}

describe('current visit SDK schema matches the existing detail response', () => {
  it('parses the captured full detail without dropping existing visit fields', () => {
    expect(fullWorkflowInstanceDetailSchema.parse(captured.detail)).toEqual(captured.detail);
  });

  it('parses the captured current visit through the authorized-actions endpoint schema', () => {
    const response = endpointResponse();
    expect(workflowInstanceDetailResponseSchema.parse(response)).toEqual(response);
  });

  it('rejects a genuinely unknown current visit field', () => {
    const response = endpointResponse();
    response.detail.current_visit.undeclared_visit_field = true;
    const result = workflowInstanceDetailResponseSchema.safeParse(response);
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.issues).toContainEqual(expect.objectContaining({
        code: 'unrecognized_keys',
        keys: expect.arrayContaining(['undeclared_visit_field']),
        path: ['detail', 'current_visit'],
      }));
    }
  });

  it('keeps eligibility closed to unknown fields and classifications', () => {
    const visit = captured.detail.current_visit;
    for (const eligibility of [
      { classification: 'ACTIONABLE_NOW', undeclared_eligibility_field: true },
      { classification: 'UNKNOWN' },
      { classification: 'WAITING_FOR_TIME' },
      { classification: 'WAITING_FOR_TIME', nextEligibleAt: 'not-a-time' },
    ]) {
      expect(nodeVisitSchema.safeParse({ ...visit, eligibility }).success).toBe(false);
    }
  });
});
