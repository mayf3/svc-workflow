//! Authentication for the internal HTTP API.

mod principal;
mod verifier;

pub use principal::AuthenticatedPrincipal;
pub use verifier::{JwtConfig, JwtVerifier};
