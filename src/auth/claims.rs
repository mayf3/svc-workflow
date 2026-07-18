//! JWT claims types for direct and OBO token verification.

use serde::Deserialize;
use uuid::Uuid;

use crate::domain::ids::PrincipalId;

/// Act claim for OBO delegation (RFC 8693 style).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActClaim {
    pub sub: Option<String>,
    /// Detect nested delegation — not allowed by the frozen V1 profile.
    #[serde(rename = "act")]
    pub nested_act: Option<serde_json::Value>,
}

/// Full set of claims supported by svc-workflow authentication.
///
/// Supports both direct access tokens (`token_use: access`) and
/// on-behalf-of tokens (`token_use: workflow_obo`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// Optional canonical agent identifier; never authoritative for resource authorization.
    pub agent_id: Option<String>,
    /// Token use discriminator: `access` (direct) or `workflow_obo` (delegated).
    pub token_use: Option<String>,
    /// OBO: subject of the delegated authority (ADC service principal).
    pub act: Option<ActClaim>,
    /// OBO: authorized party (OAuth client ID).
    pub azp: Option<String>,
    /// Unique token identifier for audit correlation.
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
        None => Err("missing token_use".to_string()),
    }
}

/// Validate Direct token profile.
///
/// Direct tokens must not carry OBO markers:
/// - `act` must be absent
/// - `azp` must be absent
/// - `token_use` must not be `workflow_obo`
pub fn validate_direct_profile(claims: &WorkflowClaims) -> Result<(), String> {
    if claims.token_use.as_deref() != Some("access") {
        return Err("direct token must have token_use=access".to_string());
    }
    // Direct tokens must not have delegation claims.
    if claims.act.is_some() {
        return Err("direct token must not carry act claim".to_string());
    }
    if claims.azp.is_some() {
        return Err("direct token must not carry azp claim".to_string());
    }
    require_claim(&claims.client_id, "client_id")?;
    if claims.agent_id.as_deref().is_some_and(str::is_empty) {
        return Err("direct token agent_id must not be empty".to_string());
    }
    require_token_id(&claims.jti)?;
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
    require_token_id(&claims.jti)?;
    if claims.agent_id.as_deref().is_some_and(str::is_empty) {
        return Err("OBO token agent_id must not be empty".to_string());
    }
    Ok(())
}

fn require_token_id(jti: &Option<String>) -> Result<(), String> {
    if jti.as_deref().is_none_or(|value| value.len() < 16) {
        return Err("token jti must contain at least 16 characters".to_string());
    }
    Ok(())
}

/// Validate the frozen ASCII-space scope wire format and return exact items.
pub fn canonical_scope(scope: &Option<String>) -> Result<Vec<String>, String> {
    let scope = scope.as_deref().ok_or("missing scope")?;
    if scope.is_empty() || !scope.is_ascii() {
        return Err("scope must be a non-empty ASCII string".to_string());
    }
    let items = scope.split(' ').collect::<Vec<_>>();
    if items.iter().any(|item| item.is_empty()) {
        return Err("scope must use one ASCII space between items".to_string());
    }
    if items.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("scope items must be unique and sorted".to_string());
    }
    if items.iter().any(|item| !valid_workflow_scope_item(item)) {
        return Err("scope item is outside the workflow namespace".to_string());
    }
    Ok(items.into_iter().map(str::to_owned).collect())
}

fn valid_workflow_scope_item(item: &str) -> bool {
    let Some(action) = item.strip_prefix("workflow.") else {
        return false;
    };
    let Some(first) = action.as_bytes().first() else {
        return false;
    };
    first.is_ascii_lowercase()
        && action.as_bytes().iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

/// Enforce frozen NumericDate ordering, skew and profile TTL bounds.
pub fn validate_time_claims(
    claims: &WorkflowClaims,
    is_obo: bool,
    now: usize,
    clock_skew_seconds: u64,
) -> Result<(), String> {
    let iat = claims.iat.ok_or("missing iat")?;
    let nbf = claims.nbf.ok_or("missing nbf")?;
    let exp = claims.exp.ok_or("missing exp")?;
    let skew = usize::try_from(clock_skew_seconds).unwrap_or(usize::MAX);
    let maximum_ttl = if is_obo { 300 } else { 600 };
    if nbf > iat || exp <= iat || exp - iat > maximum_ttl {
        return Err("invalid token time ordering or TTL".to_string());
    }
    if iat > now.saturating_add(skew)
        || nbf > now.saturating_add(skew)
        || exp <= now.saturating_sub(skew)
    {
        return Err("token is outside the accepted time window".to_string());
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
    fn token_use_is_required_and_exact() {
        assert!(validate_token_use(&None).is_err());
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
            agent_id: Some("test-agent".to_string()),
            token_use: Some("access".to_string()),
            act: Some(ActClaim {
                sub: Some(Uuid::new_v4().to_string()),
                nested_act: None,
            }),
            azp: None,
            jti: Some("direct-token-id-01".to_string()),
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
            agent_id: Some("test-agent".to_string()),
            token_use: Some("access".to_string()),
            act: None,
            azp: Some("some-azp".to_string()),
            jti: Some("direct-token-id-02".to_string()),
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
            agent_id: Some("test-agent".to_string()),
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
            agent_id: Some("test-agent".to_string()),
            token_use: Some("access".to_string()),
            act: None,
            azp: None,
            jti: Some("direct-token-id-03".to_string()),
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
            agent_id: Some("test-agent".to_string()),
            token_use: Some("workflow_obo".to_string()),
            act: Some(ActClaim {
                sub: Some(act_sub.to_string()),
                nested_act: None,
            }),
            azp: Some("test-client".to_string()),
            jti: Some("unique-token-id-01".to_string()),
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
        claims.jti = Some("unique-token-id-02".to_string());

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
    fn scope_wire_format_is_canonical() {
        assert_eq!(
            canonical_scope(&Some("workflow.execute workflow.read".to_string())).unwrap(),
            vec!["workflow.execute", "workflow.read"]
        );
        for invalid in [
            "workflow.read workflow.execute",
            "workflow.read workflow.read",
            " workflow.read",
            "workflow.read ",
            "workflow.read  workflow.write",
            "okr.read",
            "workflow.Read",
        ] {
            assert!(
                canonical_scope(&Some(invalid.to_string())).is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn time_claims_enforce_order_and_profile_ttl() {
        let mut claims = WorkflowClaims {
            sub: Some(Uuid::new_v4().to_string()),
            iss: Some("auth-service".to_string()),
            aud: Some("svc-workflow".to_string()),
            exp: Some(1_500),
            iat: Some(1_000),
            nbf: Some(1_000),
            principal_type: Some("agent".to_string()),
            token_type: Some("access".to_string()),
            version: Some("v1".to_string()),
            scope: Some("workflow.read".to_string()),
            agent_id: Some("test-agent".to_string()),
            token_use: Some("access".to_string()),
            act: None,
            azp: None,
            jti: Some("direct-token-id-04".to_string()),
            client_id: Some("test-client".to_string()),
        };
        assert!(validate_time_claims(&claims, false, 1_100, 60).is_ok());
        claims.exp = Some(1_601);
        assert!(validate_time_claims(&claims, false, 1_100, 60).is_err());
        claims.exp = Some(1_200);
        claims.nbf = Some(1_001);
        assert!(validate_time_claims(&claims, false, 1_100, 60).is_err());
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
