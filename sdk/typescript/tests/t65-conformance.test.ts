/**
 * T65 regression — WF-SERVER-OPENAPI-SDK-SCHEMA-CONFORMANCE.
 * The body below is a REAL HTTP 200 DomainInstanceSummary serialized by the
 * current server (captured from GET /internal/v1/workflow-instances/global at
 * main cc006d9). The strict SDK decoder MUST accept it, unknown extra fields
 * MUST still be rejected, and wrong enum values MUST be rejected.
 */
import { describe, expect, it } from 'vitest';
import { domainInstancePageSchema } from '../src/schemas.js';

const REAL_SUMMARY = {"created_at": "2026-09-12T23:46:20.318374Z", "created_by_principal_id": "e0c683f1-cd0c-48e8-aa44-70a840c330b2", "current_assignee_canonical_agent_id": null, "current_assignee_principal_id": "e0c683f1-cd0c-48e8-aa44-70a840c330b2", "current_node": {"display_name": "Start", "node_id": "ded1f33c-4297-4e39-be13-49d046862646", "node_key": "start", "node_type": "TASK"}, "definition_key": "wec-test-f45d5008", "definition_version_id": "9c8b8e1b-cd45-4ff6-8722-62d29389a264", "domain_id": "0024452a-b196-4ea3-a865-4d38a1190427", "eligibility": {"classification": "ACTIONABLE_NOW"}, "execution_class": "BUSINESS", "is_terminal": false, "title": "t65-dump", "updated_at": "2026-09-12T23:46:20.318374Z", "workflow_instance_id": "bb239e03-92a2-48f1-9868-48e5857d88f7"};

describe('T65 SDK strict decode conformance', () => {
  it('accepts a real current server 200 summary', () => {
    const page = { items: [REAL_SUMMARY], next_cursor: null };
    const result = domainInstancePageSchema.safeParse(page);
    expect(result.success).toBe(true);
  });

  it('still rejects unknown fields (strict is preserved)', () => {
    const polluted = { ...REAL_SUMMARY, some_future_field: 1 };
    const result = domainInstancePageSchema.safeParse({ items: [polluted], next_cursor: null });
    expect(result.success).toBe(false);
  });

  it('rejects a wrong execution_class value', () => {
    const bad = { ...REAL_SUMMARY, execution_class: 'SOMETING_ELSE' };
    const result = domainInstancePageSchema.safeParse({ items: [bad], next_cursor: null });
    expect(result.success).toBe(false);
  });

  it('rejects a missing required server field', () => {
    const { execution_class, ...without } = REAL_SUMMARY;
    const result = domainInstancePageSchema.safeParse({ items: [without], next_cursor: null });
    expect(result.success).toBe(false);
  });
});
