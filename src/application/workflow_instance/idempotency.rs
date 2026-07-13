//! Idempotency key and request hash computation.
//!
//! Implements the request hash algorithm per the frozen contract:
//!
//! ```json
//! JCS({
//!   "commandSchemaVersion": "...",
//!   "commandType": "CREATE_WORKFLOW_INSTANCE",
//!   "routeParameters": {},
//!   "requestBody": {
//!     "principalId": "...",
//!     "domainId": "...",
//!     "definitionVersionId": "...",
//!     "contextPayload": ...,
//!     "metadata": ...,
//!     "externalReference": null,
//!     "externalUrl": null
//!   }
//! }) → SHA-256
//! ```
//!
//! For `routeParameters`, we use a stable empty object `{}` since this
//! command has no HTTP route parameters.

use serde::Serialize;

use crate::domain::ids::{DefinitionVersionId, DomainId, PrincipalId};
use crate::domain::workflow_instance::errors::CreateWorkflowInstanceError;
use crate::domain::workflow_instance::events::COMMAND_TYPE_CREATE_INSTANCE;

/// The canonical request envelope used for hash computation.
///
/// Per the frozen contract, `requestBody` is a nested object containing
/// all request fields except the idempotency key.
#[derive(Debug, Clone, Serialize)]
struct RequestEnvelope {
    command_schema_version: String,
    command_type: String,
    route_parameters: serde_json::Value,
    request_body: RequestBody,
}

/// The body of the request without the idempotency key.
#[derive(Debug, Clone, Serialize)]
struct RequestBody {
    principal_id: String,
    domain_id: String,
    definition_version_id: String,
    context_payload: serde_json::Value,
    metadata: serde_json::Value,
    external_reference: Option<String>,
    external_url: Option<String>,
}

/// Compute the canonical request hash for idempotency.
///
/// The hash covers all command parameters except the idempotency key itself.
pub fn compute_request_hash(
    command_schema_version: &str,
    _idempotency_key: &str,
    principal_id: &PrincipalId,
    domain_id: &DomainId,
    definition_version_id: &DefinitionVersionId,
    context_payload: &serde_json::Value,
    metadata: &serde_json::Value,
    external_reference: &Option<String>,
    external_url: &Option<String>,
) -> Result<String, CreateWorkflowInstanceError> {
    let envelope = RequestEnvelope {
        command_schema_version: command_schema_version.to_string(),
        command_type: COMMAND_TYPE_CREATE_INSTANCE.to_string(),
        route_parameters: serde_json::json!({}),
        request_body: RequestBody {
            principal_id: principal_id.to_string(),
            domain_id: domain_id.to_string(),
            definition_version_id: definition_version_id.to_string(),
            context_payload: context_payload.clone(),
            metadata: metadata.clone(),
            external_reference: external_reference.clone(),
            external_url: external_url.clone(),
        },
    };

    jcs_canonicalize::sha256_jcs_hex(&envelope).map_err(|e| {
        CreateWorkflowInstanceError::StorageError(format!("request hash computation failed: {}", e))
    })
}
