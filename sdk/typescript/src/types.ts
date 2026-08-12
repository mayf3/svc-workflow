import type { z } from 'zod';

import type {
  assistanceCaseDetailSchema,
  assistanceCaseDetailResponseSchema,
  assistanceCasePageSchema,
  assistanceCommandResponseSchema,
  assistanceInboxQuerySchema,
  archiveDefinitionResponseSchema,
  createDefinitionRequestSchema,
  createDefinitionResponseSchema,
  createDraftVersionRequestSchema,
  createDraftVersionResponseSchema,
  createWorkflowInstanceRequestSchema,
  createWorkflowInstanceResponseSchema,
  creatorDraftPageSchema,
  definitionDetailResponseSchema,
  definitionItemSchema,
  definitionListPageSchema,
  definitionListQuerySchema,
  definitionVersionSummarySchema,
  domainInstancePageSchema,
  domainInstanceQuerySchema,
  escalateAssistanceRequestSchema,
  executeWorkflowTransitionRequestSchema,
  executeWorkflowTransitionResponseSchema,
  humanRequiredAssistancePageSchema,
  humanRequiredAssistanceQuerySchema,
  memberAddResponseSchema,
  memberItemSchema,
  memberListPageSchema,
  memberListQuerySchema,
  memberRemoveResponseSchema,
  publishVersionRequestSchema,
  publishVersionResponseSchema,
  replaceDraftGraphRequestSchema,
  requestAssistanceRequestSchema,
  resolveAssistanceRequestSchema,
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

export type DefinitionItem = z.infer<typeof definitionItemSchema>;
export type DefinitionVersionSummary = z.infer<typeof definitionVersionSummarySchema>;
export type DefinitionDetailResponse = z.infer<typeof definitionDetailResponseSchema>;
export type DefinitionListPage = z.infer<typeof definitionListPageSchema>;
export type DefinitionListQuery = z.infer<typeof definitionListQuerySchema>;
export type CreateDefinitionRequest = z.infer<typeof createDefinitionRequestSchema>;
export type CreateDefinitionResponse = z.infer<typeof createDefinitionResponseSchema>;
export type CreateDraftVersionRequest = z.infer<typeof createDraftVersionRequestSchema>;
export type CreateDraftVersionResponse = z.infer<typeof createDraftVersionResponseSchema>;
export type ReplaceDraftGraphRequest = z.infer<typeof replaceDraftGraphRequestSchema>;
export type PublishVersionRequest = z.infer<typeof publishVersionRequestSchema>;
export type PublishVersionResponse = z.infer<typeof publishVersionResponseSchema>;
export type ArchiveDefinitionResponse = z.infer<typeof archiveDefinitionResponseSchema>;

export type RequestAssistanceRequest = z.infer<typeof requestAssistanceRequestSchema>;
export type EscalateAssistanceRequest = z.infer<typeof escalateAssistanceRequestSchema>;
export type ResolveAssistanceRequest = z.infer<typeof resolveAssistanceRequestSchema>;
export type AssistanceCommandResponse = z.infer<typeof assistanceCommandResponseSchema>;
export type AssistanceCaseDetail = z.infer<typeof assistanceCaseDetailSchema>;
export type AssistanceCaseDetailResponse = z.infer<
  typeof assistanceCaseDetailResponseSchema
>;
export type AssistanceCasePage = z.infer<typeof assistanceCasePageSchema>;
export type HumanRequiredAssistancePage = z.infer<
  typeof humanRequiredAssistancePageSchema
>;
export type AssistanceInboxQuery = z.infer<typeof assistanceInboxQuerySchema>;
export type HumanRequiredAssistanceQuery = z.infer<
  typeof humanRequiredAssistanceQuerySchema
>;
