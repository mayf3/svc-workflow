//! Authentication for the internal HTTP API.
//!
//! Supports two modes:
//! - `test_hs256`: HS256 shared-secret verification (local development, smoke tests).
//! - `jwks`: RS256 JWKS verification (staging, canary, production).
//!
//! ## Auth V1 Canary
//!
//! An additional single-agent read-only canary profile may be enabled via
//! `AuthV1CanaryConfig`.  When active, only the `assigned-to-me` worklist
//! endpoint accepts tokens validated against the frozen Minimal Auth V1
//! contract.  Write endpoints reject canary-authenticated requests.

mod auth_context;
mod auth_mode;
mod canary;
mod claims;
mod jwks_verifier;
mod principal;
mod verifier;

pub use auth_context::AuthContext;
pub use auth_mode::{validate_mode_gates, AuthMode, Hs256Config, JwksConfig};
pub use canary::{AuthV1CanaryConfig, AuthV1CanaryVerifier, CanaryAuthenticated, CanaryPrincipal};
pub use claims::WorkflowClaims;
pub use jwks_verifier::JwksVerifier;
pub use principal::AuthenticatedPrincipal;
pub use verifier::{require_legacy_claims, Hs256Verifier};
