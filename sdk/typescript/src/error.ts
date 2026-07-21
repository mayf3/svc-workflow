export type WorkflowErrorKind =
  | 'configuration'
  | 'input'
  | 'transport'
  | 'api'
  | 'protocol';

export interface WorkflowErrorOptions {
  kind: WorkflowErrorKind;
  operation?: string;
  status?: number;
  code?: string;
  details?: unknown;
  attempts?: number;
  requestId?: string;
  responseRequestId?: string;
  cause?: unknown;
}

export class WorkflowError extends Error {
  readonly kind: WorkflowErrorKind;
  readonly operation?: string;
  readonly status?: number;
  readonly code?: string;
  readonly details?: unknown;
  readonly attempts?: number;
  readonly requestId?: string;
  readonly responseRequestId?: string;

  constructor(message: string, options: WorkflowErrorOptions) {
    super(message, { cause: options.cause });
    this.name = 'WorkflowError';
    this.kind = options.kind;
    this.operation = options.operation;
    this.status = options.status;
    this.code = options.code;
    this.details = options.details;
    this.attempts = options.attempts;
    this.requestId = options.requestId;
    this.responseRequestId = options.responseRequestId;
  }
}
