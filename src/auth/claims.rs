//! JWT claims types for Auth V1 DirectMachineAccess and OBO token verification.
//!
//! Direct access tokens are validated against the strict `V1DirectMachineClaims`
//! struct (RS256, `deny_unknown_fields`, all fields required).
//!
//! OBO tokens use the strict `V1OboMachineClaims` struct (RS256,
//! `deny_unknown_fields`, `act.sub` required).

use std::collections::HashSet;

use serde::Deserialize;

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
// OBO claims — production struct (strict, deny_unknown_fields)
// ---------------------------------------------------------------------------

/// Strict `act` claim for OBO delegation (RFC 8693 style).
///
/// `sub` is required.  Nested `act` (chained delegation) is implicitly
/// rejected by `deny_unknown_fields`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OboActClaim {
    /// The proxy service principal (e.g. ADC) that initiated the delegation.
    pub sub: String,
}

/// Strict claims set matching the Auth V1 `workflow_obo` profile.
///
/// Contract: `token_use=workflow_obo`, `act` required, `act.sub` required.
/// `client_id`, `azp`, and `agent_id` are optional diagnostic fields —
/// the Auth V1 contract permits them but they never participate in
/// domain authorization (see spec section 4).
///
/// Unknown fields are rejected via `deny_unknown_fields`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct V1OboMachineClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub principal_type: String,
    pub client_id: Option<String>,
    pub token_use: String,
    #[serde(rename = "type")]
    pub token_type: String,
    pub version: String,
    pub scope: String,
    pub agent_id: Option<String>,
    pub azp: Option<String>,
    pub jti: String,
    pub iat: usize,
    pub nbf: usize,
    pub exp: usize,
    pub act: OboActClaim,
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
}
