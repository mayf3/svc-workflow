//! JWT claims types for Auth V1 DirectMachineAccess and OBO token verification.
//!
//! Direct access tokens are validated against the strict `V1DirectMachineClaims`
//! struct (RS256, `deny_unknown_fields`, all fields required).
//!
//! OBO tokens use the lenient `WorkflowClaims` struct (kept for future use).

use std::collections::HashSet;

use serde::Deserialize;
use uuid::Uuid;

use crate::domain::ids::PrincipalId;
use crate::http::error::ApiError;

// ---------------------------------------------------------------------------
// Auth V1 DirectMachineAccess claims (strict, deny_unknown_fields)
// ---------------------------------------------------------------------------

/// Narrow claims set matching the Auth V1 DirectMachineAccess profile.
///
/// All fields are required. `agent_id` is optional diagnostic metadata.
/// The sole principal identifier is `sub`.
///
/// Contract source: frozen Minimal Auth V1 Bundle.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct V1DirectMachineClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub principal_type: String,
    pub client_id: String,
    pub token_use: String,
    #[serde(rename = "type")]
    pub token_type: String,
    pub version: String,
    pub scope: String,
    pub agent_id: Option<String>,
    pub jti: String,
    pub iat: usize,
    pub nbf: usize,
    pub exp: usize,
}

// ---------------------------------------------------------------------------
// OBO claims (lenient, kept for future use)
// ---------------------------------------------------------------------------

/// Act claim for OBO delegation (RFC 8693 style).
#[derive(Debug, Deserialize)]
pub struct ActClaim {
    pub sub: Option<String>,
    /// Detect nested delegation — not allowed in V0.
    #[serde(rename = "act")]
    pub nested_act: Option<serde_json::Value>,
}

/// Lenient claims set for OBO token verification.
#[derive(Debug, Deserialize)]
pub struct WorkflowClaims {
    pub sub: Option<String>,
    pub iss: Option<String>,
    pub aud: Option<String>,
    pub exp: Option<usize>,
    pub iat: Option<usize>,
    pub nbf: Option<usize>,
    pub principal_type: Option<String>,
    #[serde(rename = "type")]
    pub token_type: Option<String>,
    pub version: Option<String>,
    pub scope: Option<String>,
    pub token_use: Option<String>,
    pub act: Option<ActClaim>,
    pub azp: Option<String>,
    pub jti: Option<String>,
    pub client_id: Option<String>,
}

// ---------------------------------------------------------------------------
// V1 validation helpers
// ---------------------------------------------------------------------------

/// Validate V1 scope against the ASCII-space wire format.
///
/// Rules:
/// - separator: U+0020 (ASCII space)
/// - case-sensitive
/// - no leading/trailing space
/// - no duplicates
/// - sorted: unsigned-ascii-byte-ascending
/// - each item matches `^[a-z][a-z0-9-]*\.[a-z][a-z0-9._-]*$`
pub fn validate_v1_scope(scope: &str) -> Result<(), ApiError> {
    if scope.is_empty() {
        return Err(ApiError::unauthorized(
            "invalid_scope",
            "scope must not be empty",
        ));
    }
    if !scope.is_ascii() {
        return Err(ApiError::unauthorized(
            "invalid_scope",
            "scope must be ASCII",
        ));
    }
    if scope.starts_with(' ') || scope.ends_with(' ') {
        return Err(ApiError::unauthorized(
            "invalid_scope",
            "scope must not have leading or trailing spaces",
        ));
    }
    let items: Vec<&str> = scope.split(' ').collect();
    if items.iter().any(|item| item.is_empty()) {
        return Err(ApiError::unauthorized(
            "invalid_scope",
            "scope items must be separated by single spaces",
        ));
    }
    let unique: HashSet<&str> = items.iter().copied().collect();
    if unique.len() != items.len() {
        return Err(ApiError::unauthorized(
            "invalid_scope",
            "scope items must not contain duplicates",
        ));
    }
    if items.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ApiError::unauthorized(
            "invalid_scope",
            "scope items must be sorted in ASCII ascending order",
        ));
    }
    for item in &items {
        if !is_valid_scope_item(item) {
            return Err(ApiError::unauthorized(
                "invalid_scope",
                "scope item has invalid format",
            ));
        }
    }
    Ok(())
}

/// Check that a scope item matches `^[a-z][a-z0-9-]*\.[a-z][a-z0-9._-]*$`
fn is_valid_scope_item(item: &str) -> bool {
    let Some(dot_pos) = item.find('.') else {
        return false;
    };
    let prefix = &item[..dot_pos];
    let suffix = &item[dot_pos + 1..];
    if prefix.is_empty() || suffix.is_empty() {
        return false;
    }
    prefix
        .as_bytes()
        .first()
        .is_some_and(|b| b.is_ascii_lowercase())
        && prefix
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && suffix
            .as_bytes()
            .first()
            .is_some_and(|b| b.is_ascii_lowercase())
        && suffix.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        })
}

/// Validate time claims against the contract rules.
///
/// - clock_skew_tolerance_seconds: configured value
/// - machine_access_ttl_seconds: 600
/// - `nbf ≤ iat`
/// - `exp > iat` and `exp - iat ≤ machine_access_ttl_seconds`
pub fn validate_v1_time_claims(
    iat: usize,
    nbf: usize,
    exp: usize,
    clock_skew_seconds: u64,
) -> Result<(), ApiError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as usize;
    let skew = clock_skew_seconds as usize;
    let machine_ttl = 600;

    if nbf > iat {
        return Err(ApiError::unauthorized(
            "invalid_time_claims",
            "nbf must not be later than iat",
        ));
    }
    if exp <= iat {
        return Err(ApiError::unauthorized(
            "invalid_time_claims",
            "exp must be after iat",
        ));
    }
    if exp - iat > machine_ttl {
        return Err(ApiError::unauthorized(
            "token_ttl_exceeded",
            "token TTL must not exceed the maximum allowed duration",
        ));
    }
    if iat > now.saturating_add(skew) {
        return Err(ApiError::unauthorized(
            "invalid_time_claims",
            "iat is too far in the future",
        ));
    }
    if nbf > now.saturating_add(skew) {
        return Err(ApiError::unauthorized(
            "token_not_yet_valid",
            "token is not yet valid (nbf in the future)",
        ));
    }
    if exp <= now.saturating_sub(skew) {
        return Err(ApiError::unauthorized(
            "token_expired",
            "access token has expired",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// OBO validation helpers (kept for future use)
// ---------------------------------------------------------------------------

/// Validate `principal_type` is exactly `agent`.
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
        None => Ok(()),
    }
}

/// Validate Direct token profile.
pub fn validate_direct_profile(claims: &WorkflowClaims) -> Result<(), String> {
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
pub fn validate_obo(claims: &WorkflowClaims) -> Result<(), String> {
    let act = claims.act.as_ref().ok_or("OBO token missing act")?;
    let act_sub = act.sub.as_deref().ok_or("OBO token missing act.sub")?;
    Uuid::parse_str(act_sub).map_err(|_| "OBO act.sub must be a valid UUID".to_string())?;
    if has_nested_act(act) {
        return Err("OBO token must not contain nested act".to_string());
    }
    if claims.azp.as_deref().is_none_or(str::is_empty) {
        return Err("OBO token missing azp".to_string());
    }
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
    if claims.jti.as_deref().is_none_or(str::is_empty) {
        return Err("OBO token missing jti".to_string());
    }
    Ok(())
}

/// Check if the claims indicate an OBO token.
pub fn is_obo(claims: &WorkflowClaims) -> bool {
    claims.act.is_some() || matches!(claims.token_use.as_deref(), Some("workflow_obo"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- V1 scope validation ---

    #[test]
    fn v1_scope_accepts_valid() {
        assert!(validate_v1_scope("workflow.execute workflow.read").is_ok());
        assert!(validate_v1_scope("workflow.read").is_ok());
    }

    #[test]
    fn v1_scope_rejects_empty() {
        assert!(validate_v1_scope("").is_err());
    }

    #[test]
    fn v1_scope_rejects_duplicates() {
        assert!(validate_v1_scope("workflow.read workflow.read").is_err());
    }

    #[test]
    fn v1_scope_rejects_unsorted() {
        assert!(validate_v1_scope("workflow.read workflow.execute").is_err());
    }

    #[test]
    fn v1_scope_rejects_trailing_space() {
        assert!(validate_v1_scope("workflow.read ").is_err());
    }

    #[test]
    fn v1_scope_rejects_leading_space() {
        assert!(validate_v1_scope(" workflow.read").is_err());
    }

    #[test]
    fn v1_scope_rejects_consecutive_spaces() {
        assert!(validate_v1_scope("workflow.read  workflow.execute").is_err());
    }

    #[test]
    fn v1_scope_rejects_non_ascii() {
        assert!(validate_v1_scope("workflow.读取").is_err());
    }

    #[test]
    fn v1_scope_accepts_unknown_scopes() {
        assert!(validate_v1_scope("custom.namespace workflow.read").is_ok());
    }

    // --- V1 time validation ---

    #[test]
    fn v1_time_valid() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        assert!(validate_v1_time_claims(now, now, now + 300, 60).is_ok());
        assert!(validate_v1_time_claims(now - 30, now - 30, now + 570, 60).is_ok());
    }

    #[test]
    fn v1_time_rejects_future_nbf() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        assert!(validate_v1_time_claims(now, now + 10, now + 300, 60).is_err());
    }

    #[test]
    fn v1_time_rejects_expired() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        assert!(validate_v1_time_claims(now - 600, now - 600, now - 1, 0).is_err());
    }

    #[test]
    fn v1_time_rejects_excessive_ttl() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        assert!(validate_v1_time_claims(now, now, now + 601, 60).is_err());
    }

    #[test]
    fn v1_time_rejects_exp_equal_iat() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        assert!(validate_v1_time_claims(now, now, now, 60).is_err());
    }

    #[test]
    fn v1_time_rejects_exp_before_iat() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        assert!(validate_v1_time_claims(now, now, now - 1, 60).is_err());
    }

    #[test]
    fn v1_time_rejects_future_iat_beyond_skew() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        assert!(validate_v1_time_claims(now + 120, now, now + 600, 60).is_err());
    }

    #[test]
    fn v1_time_rejects_expired_beyond_skew() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        assert!(validate_v1_time_claims(now - 120, now - 120, now - 61, 60).is_err());
    }

    #[test]
    fn v1_time_accepts_within_skew() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        assert!(validate_v1_time_claims(now - 60, now - 60, now + 540, 60).is_ok());
    }

    #[test]
    fn scope_item_pattern() {
        assert!(is_valid_scope_item("workflow.read"));
        assert!(is_valid_scope_item("workflow.execute"));
        assert!(is_valid_scope_item("adc.read"));
        assert!(!is_valid_scope_item("read"));
        assert!(!is_valid_scope_item(".read"));
        assert!(!is_valid_scope_item("workflow."));
        assert!(!is_valid_scope_item("Workflow.read"));
    }

    // --- OBO validation (preserved) ---

    #[test]
    fn obo_validation_preserved() {
        let act_sub = Uuid::new_v4();
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
        assert!(validate_obo(&claims).is_ok());
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

    #[test]
    fn validate_principal_type_works() {
        assert!(validate_principal_type(&Some("agent".to_string())).is_ok());
        assert!(validate_principal_type(&Some("human".to_string())).is_err());
        assert!(validate_principal_type(&None).is_err());
    }

    #[test]
    fn validate_token_use_works() {
        assert!(validate_token_use(&None).is_ok());
        assert!(validate_token_use(&Some("access".to_string())).is_ok());
        assert!(validate_token_use(&Some("workflow_obo".to_string())).is_ok());
        assert!(validate_token_use(&Some("invalid".to_string())).is_err());
    }
}
