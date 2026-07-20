import type { z } from 'zod';

import type {
  createWorkflowInstanceRequestSchema,
  createWorkflowInstanceResponseSchema,
  creatorDraftPageSchema,
  domainInstancePageSchema,
  domainInstanceQuerySchema,
  executeWorkflowTransitionRequestSchema,
  executeWorkflowTransitionResponseSchema,
  memberAddResponseSchema,
  memberItemSchema,
  memberListPageSchema,
  memberListQuerySchema,
  memberRemoveResponseSchema,
  selfProjectionResponseSchema,
  timelineQuerySchema,
  timelineResponseSchema,
  workflowInstanceDetailResponseSchema,
  worklistPageSchema,
  worklistQuerySchema,
} from './schemas.js';

export type TokenProvider = () => string | Promise<string>;
export type RequestIdProvider = () => string;

export interface WorkflowClientConfig {
  baseUrl: string;
  tokenProvider: TokenProvider;
  requestTimeoutMs?: number;
  maxAttempts?: number;
  retryDelaysMs?: readonly number[];
  requestIdProvider?: RequestIdProvider;
  fetchImplementation?: typeof globalThis.fetch;
}

export interface RequestOptions {
  requestId?: string;
}

export interface WriteOptions extends RequestOptions {
  idempotencyKey: string;
}

export type CreateWorkflowInstanceRequest = z.infer<
  typeof createWorkflowInstanceRequestSchema
>;
export type CreateWorkflowInstanceResponse = z.infer<
  typeof createWorkflowInstanceResponseSchema
>;
export type ExecuteWorkflowTransitionRequest = z.infer<
  typeof executeWorkflowTransitionRequestSchema
>;
export type ExecuteWorkflowTransitionResponse = z.infer<
  typeof executeWorkflowTransitionResponseSchema
>;
export type WorkflowInstanceDetailResponse = z.infer<
  typeof workflowInstanceDetailResponseSchema
>;
export type TimelineQuery = z.infer<typeof timelineQuerySchema>;
export type TimelineResponse = z.infer<typeof timelineResponseSchema>;
export type WorklistQuery = z.infer<typeof worklistQuerySchema>;
export type WorklistPage = z.infer<typeof worklistPageSchema>;
export type CreatorDraftPage = z.infer<typeof creatorDraftPageSchema>;
export type DomainInstanceQuery = z.infer<typeof domainInstanceQuerySchema>;
export type DomainInstancePage = z.infer<typeof domainInstancePageSchema>;

export type SelfProjectionResponse = z.infer<typeof selfProjectionResponseSchema>;
export type MemberItem = z.infer<typeof memberItemSchema>;
export type MemberListPage = z.infer<typeof memberListPageSchema>;
export type MemberListQuery = z.infer<typeof memberListQuerySchema>;
export type MemberAddResponse = z.infer<typeof memberAddResponseSchema>;
export type MemberRemoveResponse = z.infer<typeof memberRemoveResponseSchema>;
