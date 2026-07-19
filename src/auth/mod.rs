//! Authentication for the internal HTTP API.
//!
//! Supports only Auth V1 RS256/JWKS verification.  Legacy HS256 and
//! `test_hs256` mode have been removed.

mod auth_context;
mod auth_mode;
mod canary;
mod claims;
mod jwks_verifier;
mod principal;

pub use auth_context::AuthContext;
pub use auth_mode::{validate_env, JwksConfig};
pub use canary::AuthV1CanaryConfig;
pub use claims::V1DirectMachineClaims;
pub use jwks_verifier::JwksVerifier;
pub use principal::AuthenticatedPrincipal;
