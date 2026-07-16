//! JWT claims types for direct and OBO token verification.

use serde::Deserialize;
use uuid::Uuid;

use crate::domain::ids::PrincipalId;

/// Act claim for OBO delegation (RFC 8693 style).
#[derive(Debug, Deserialize)]
pub struct ActClaim {
    pub sub: Option<String>,
    /// Detect nested delegation — not allowed in V0.
    #[serde(rename = "act")]
    pub nested_act: Option<serde_json::Value>,
}

/// Full set of claims supported by svc-workflow authentication.
///
/// Supports both direct access tokens (`token_use: access`) and
/// on-behalf-of tokens (`token_use: workflow_obo`).
#[derive(Debug, Deserialize)]
pub struct WorkflowClaims {
    pub sub: Option<String>,
    pub iss: Option<String>,
    pub aud: Option<String>,
    pub exp: Option<usize>,
    pub iat: Option<usize>,
    pub nbf: Option<usize>,
    pub principal_type: Option<String>,
    /// Legacy `type` claim — required for backward compatibility.
    #[serde(rename = "type")]
    pub token_type: Option<String>,
    pub version: Option<String>,
    pub scope: Option<String>,
    /// Token use discriminator: `access` (direct) or `workflow_obo` (delegated).
    pub token_use: Option<String>,
    /// OBO: subject of the delegated authority (ADC service principal).
    pub act: Option<ActClaim>,
    /// OBO: authorized party (OAuth client ID).
    pub azp: Option<String>,
    /// OBO: unique token identifier for replay prevention.
    pub jti: Option<String>,
    /// Direct: OAuth client ID of the token issuer's client.
    pub client_id: Option<String>,
}

/// Result of parsing subject into a validated PrincipalId.
pub struct ParsedSubject {
    pub principal_id: PrincipalId,
    pub subject_uuid: Uuid,
}

/// Parse and validate the `sub` claim as a UUID.
pub fn parse_subject(sub: &Option<String>) -> Result<ParsedSubject, String> {
    let sub = sub.as_deref().ok_or("missing sub")?;
    let uuid = Uuid::parse_str(sub).map_err(|_| "sub must be a valid UUID".to_string())?;
    Ok(ParsedSubject {
        principal_id: PrincipalId::from_uuid(uuid),
        subject_uuid: uuid,
    })
}

/// Validate that a required non-empty string claim is present.
pub fn require_claim(value: &Option<String>, name: &str) -> Result<(), String> {
    match value.as_deref() {
        Some(v) if !v.is_empty() => Ok(()),
        _ => Err(format!("missing required claim: {name}")),
    }
}

/// Validate `principal_type` is exactly `agent` (V0 contract).
pub fn validate_principal_type(principal_type: &Option<String>) -> Result<(), String> {
    match principal_type.as_deref() {
        Some("agent") => Ok(()),
        Some(other) => Err(format!(
            "invalid principal_type '{other}': expected 'agent'"
        )),
        None => Err("missing principal_type".to_string()),
    }
}

/// Validate `token_use` is a known value.
pub fn validate_token_use(token_use: &Option<String>) -> Result<(), String> {
    match token_use.as_deref() {
        Some("access") | Some("workflow_obo") => Ok(()),
        Some(other) => Err(format!(
            "invalid token_use '{other}': expected 'access' or 'workflow_obo'"
        )),
        None => {
            // Default to "access" if missing (backward compat with existing tokens)
            Ok(())
        }
    }
}

/// Validate Direct token profile.
///
/// Direct tokens must not carry OBO markers:
/// - `act` must be absent
/// - `azp` must be absent
/// - `token_use` must not be `workflow_obo`
pub fn validate_direct_profile(claims: &WorkflowClaims) -> Result<(), String> {
    // Direct tokens must not have delegation claims.
    if claims.act.is_some() {
        return Err("direct token must not carry act claim".to_string());
    }
    if claims.azp.is_some() {
        return Err("direct token must not carry azp claim".to_string());
    }
    if claims.token_use.as_deref() == Some("workflow_obo") {
        return Err("direct token must not have token_use=workflow_obo".to_string());
    }
    Ok(())
}

/// Check if an ActClaim contains a nested `act` (delegation chain).
pub fn has_nested_act(act: &ActClaim) -> bool {
    act.nested_act.is_some()
}

/// Validate OBO-specific claims with strict profile enforcement.
///
/// Requirements:
/// - `act` present with valid `act.sub` (UUID)
/// - No nested `act` (delegation chain not supported in V0)
/// - `azp` present and non-empty
/// - `client_id` present, non-empty, and equal to `azp`
/// - `jti` present and non-empty
pub fn validate_obo(claims: &WorkflowClaims) -> Result<(), String> {
    // OBO token: act must be present
    let act = claims.act.as_ref().ok_or("OBO token missing act")?;

    // act.sub must be present and a valid UUID
    let act_sub = act.sub.as_deref().ok_or("OBO token missing act.sub")?;
    Uuid::parse_str(act_sub).map_err(|_| "OBO act.sub must be a valid UUID".to_string())?;

    // Reject nested delegation (V0 does not support chain)
    if has_nested_act(act) {
        return Err("OBO token must not contain nested act".to_string());
    }

    // azp must be present and non-empty
    if claims.azp.as_deref().is_none_or(str::is_empty) {
        return Err("OBO token missing azp".to_string());
    }

    // client_id must be present and equal to azp
    let client_id = claims
        .client_id
        .as_deref()
        .ok_or("OBO token missing client_id")?;
    if client_id.is_empty() {
        return Err("OBO token client_id must not be empty".to_string());
    }
    let azp = claims.azp.as_deref().unwrap_or("");
    if client_id != azp {
        return Err("OBO token client_id must equal azp".to_string());
    }

    // jti must be present and non-empty
    if claims.jti.as_deref().is_none_or(str::is_empty) {
        return Err("OBO token missing jti".to_string());
    }
    Ok(())
}

/// Check if the claims indicate an OBO token.
pub fn is_obo(claims: &WorkflowClaims) -> bool {
    claims.act.is_some() || matches!(claims.token_use.as_deref(), Some("workflow_obo"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_valid_subject() {
        let uuid = Uuid::new_v4();
        let result = parse_subject(&Some(uuid.to_string())).unwrap();
        assert_eq!(result.subject_uuid, uuid);
    }

    #[test]
    fn rejects_invalid_subject() {
        assert!(parse_subject(&Some("not-a-uuid".to_string())).is_err());
        assert!(parse_subject(&None).is_err());
    }

    #[test]
    fn accepts_agent_only() {
        assert!(validate_principal_type(&Some("agent".to_string())).is_ok());
        assert!(validate_principal_type(&Some("human".to_string())).is_err());
        assert!(validate_principal_type(&Some("service".to_string())).is_err());
        assert!(validate_principal_type(&None).is_err());
    }

    #[test]
    fn token_use_defaults_to_access() {
        assert!(validate_token_use(&None).is_ok());
        assert!(validate_token_use(&Some("access".to_string())).is_ok());
        assert!(validate_token_use(&Some("workflow_obo".to_string())).is_ok());
        assert!(validate_token_use(&Some("invalid".to_string())).is_err());
    }

    #[test]
    fn direct_profile_rejects_obo_markers() {
        // Direct token with act → reject
        let claims = WorkflowClaims {
            sub: Some(Uuid::new_v4().to_string()),
            iss: Some("auth-service".to_string()),
            aud: Some("svc-workflow".to_string()),
            exp: Some(9999999999),
            iat: Some(1000000000),
            nbf: None,
            principal_type: Some("agent".to_string()),
            token_type: Some("access".to_string()),
            version: Some("v1".to_string()),
            scope: Some("workflow.read".to_string()),
            token_use: None,
            act: Some(ActClaim {
                sub: Some(Uuid::new_v4().to_string()),
                nested_act: None,
            }),
            azp: None,
            jti: None,
            client_id: Some("test-client".to_string()),
        };
        assert!(
            validate_direct_profile(&claims).is_err(),
            "direct token with act must be rejected"
        );

        // Direct token with azp → reject
        let claims = WorkflowClaims {
            sub: Some(Uuid::new_v4().to_string()),
            iss: Some("auth-service".to_string()),
            aud: Some("svc-workflow".to_string()),
            exp: Some(9999999999),
            iat: Some(1000000000),
            nbf: None,
            principal_type: Some("agent".to_string()),
            token_type: Some("access".to_string()),
            version: Some("v1".to_string()),
            scope: Some("workflow.read".to_string()),
            token_use: None,
            act: None,
            azp: Some("some-azp".to_string()),
            jti: None,
            client_id: Some("test-client".to_string()),
        };
        assert!(
            validate_direct_profile(&claims).is_err(),
            "direct token with azp must be rejected"
        );

        // Direct token with token_use=workflow_obo → reject
        let claims = WorkflowClaims {
            sub: Some(Uuid::new_v4().to_string()),
            iss: Some("auth-service".to_string()),
            aud: Some("svc-workflow".to_string()),
            exp: Some(9999999999),
            iat: Some(1000000000),
            nbf: None,
            principal_type: Some("agent".to_string()),
            token_type: Some("access".to_string()),
            version: Some("v1".to_string()),
            scope: Some("workflow.read".to_string()),
            token_use: Some("workflow_obo".to_string()),
            act: None,
            azp: None,
            jti: None,
            client_id: Some("test-client".to_string()),
        };
        assert!(
            validate_direct_profile(&claims).is_err(),
            "direct token with workflow_obo must be rejected"
        );

        // Valid direct token → accept
        let claims = WorkflowClaims {
            sub: Some(Uuid::new_v4().to_string()),
            iss: Some("auth-service".to_string()),
            aud: Some("svc-workflow".to_string()),
            exp: Some(9999999999),
            iat: Some(1000000000),
            nbf: None,
            principal_type: Some("agent".to_string()),
            token_type: Some("access".to_string()),
            version: Some("v1".to_string()),
            scope: Some("workflow.read".to_string()),
            token_use: None,
            act: None,
            azp: None,
            jti: None,
            client_id: Some("test-client".to_string()),
        };
        assert!(
            validate_direct_profile(&claims).is_ok(),
            "valid direct token must be accepted"
        );
    }

    #[test]
    fn obo_validation() {
        let uuid = Uuid::new_v4();
        let act_sub = Uuid::new_v4();
        let mut claims = WorkflowClaims {
            sub: Some(uuid.to_string()),
            iss: Some("auth-service".to_string()),
            aud: Some("svc-workflow".to_string()),
            exp: Some(9999999999),
            iat: Some(1000000000),
            nbf: None,
            principal_type: Some("agent".to_string()),
            token_type: Some("access".to_string()),
            version: Some("v1".to_string()),
            scope: Some("workflow.execute".to_string()),
            token_use: Some("workflow_obo".to_string()),
            act: Some(ActClaim {
                sub: Some(act_sub.to_string()),
                nested_act: None,
            }),
            azp: Some("test-client".to_string()),
            jti: Some("unique-token-id".to_string()),
            client_id: Some("test-client".to_string()),
        };
        assert!(
            validate_obo(&claims).is_ok(),
            "valid OBO token must be accepted"
        );

        // client_id != azp → reject
        claims.client_id = Some("other-client".to_string());
        assert!(
            validate_obo(&claims).is_err(),
            "client_id != azp must be rejected"
        );
        claims.client_id = Some("test-client".to_string());

        // Missing jti → reject
        claims.jti = None;
        assert!(validate_obo(&claims).is_err());
        claims.jti = Some("tid".to_string());

        // Missing azp → reject
        claims.azp = None;
        assert!(validate_obo(&claims).is_err());
        claims.azp = Some("test-client".to_string());

        // Missing client_id → reject
        claims.client_id = None;
        assert!(validate_obo(&claims).is_err());
        claims.client_id = Some("test-client".to_string());

        // Invalid act.sub → reject
        claims.act = Some(ActClaim {
            sub: Some("not-uuid".to_string()),
            nested_act: None,
        });
        assert!(validate_obo(&claims).is_err());

        // Restore valid act.sub
        claims.act = Some(ActClaim {
            sub: Some(act_sub.to_string()),
            nested_act: None,
        });
        assert!(validate_obo(&claims).is_ok());

        // Nested act → reject
        claims.act = Some(ActClaim {
            sub: Some(act_sub.to_string()),
            nested_act: Some(serde_json::json!({"sub": Uuid::new_v4().to_string()})),
        });
        assert!(
            validate_obo(&claims).is_err(),
            "nested act must be rejected"
        );
    }

    #[test]
    fn has_nested_act_detection() {
        let no_nest = ActClaim {
            sub: Some(Uuid::new_v4().to_string()),
            nested_act: None,
        };
        assert!(!has_nested_act(&no_nest));

        let with_nest = ActClaim {
            sub: Some(Uuid::new_v4().to_string()),
            nested_act: Some(serde_json::json!({"sub": "uuid"})),
        };
        assert!(has_nested_act(&with_nest));
    }
}
