//! Test helper utilities for PostgreSQL-backed integration tests.
//!
//! The seed functions below may appear unused in individual test binaries
//! because they are compiled per test file. Allow dead_code at module level.

#![allow(dead_code)]

use sqlx::PgPool;

// ---------------------------------------------------------------------------
// RSA key pair helpers (runtime generation, no embedded private keys)
// ---------------------------------------------------------------------------

use base64::Engine as _;

/// An RSA key pair with its JWKS kid and base64url-encoded components.
#[derive(Clone)]
pub struct RsaTestKeyPair {
    pub private_key_pem: String,
    pub kid: String,
    pub n_base64url: String,
    pub e_base64url: String,
}

// ---------------------------------------------------------------------------
// Mock JWKS server (runtime RSA key generation)
// ---------------------------------------------------------------------------

use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

/// A mock JWKS server that serves a single RSA public key.
///
/// The key pair is generated at runtime — no embedded private keys.
pub struct MockJwksServer {
    pub url: String,
    pub key_pair: RsaTestKeyPair,
    shutdown: tokio::sync::oneshot::Sender<()>,
}

impl MockJwksServer {
    pub async fn start() -> Self {
        use tokio::io::AsyncWriteExt;

        let key_pair = generate_rsa_key_pair();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{addr}/.well-known/jwks.json");
        let (shutdown, mut rx) = tokio::sync::oneshot::channel::<()>();

        let body = serde_json::json!({
            "keys": [{
                "kty": "RSA",
                "use": "sig",
                "alg": "RS256",
                "kid": key_pair.kid,
                "n": key_pair.n_base64url,
                "e": key_pair.e_base64url,
            }]
        })
        .to_string();

        let body_clone = body.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        if let Ok((stream, _)) = result {
                            let resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                                body_clone.len(), body_clone
                            );
                            let mut writer = tokio::io::BufWriter::new(stream);
                            let _ = writer.write_all(resp.as_bytes()).await;
                            let _ = writer.flush().await;
                        }
                    }
                    _ = &mut rx => break,
                }
            }
        });

        Self {
            url,
            key_pair,
            shutdown,
        }
    }
}

// ---------------------------------------------------------------------------
// Auth V1 token factory (RS256, runtime keys, full V1DirectMachineClaims)
// ---------------------------------------------------------------------------

use jsonwebtoken::{encode as jwt_encode, Algorithm, EncodingKey, Header};

/// Create an RS256 JWT matching the Auth V1 DirectMachineAccess profile.
pub fn v1_token(
    subject: uuid::Uuid,
    scope: &str,
    client_id: &str,
    exp_offset: i64,
    key_pair: &RsaTestKeyPair,
) -> String {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = serde_json::json!({
        "iss": "auth-service",
        "sub": subject.to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "client_id": client_id,
        "token_use": "access",
        "type": "access",
        "version": "v1",
        "scope": scope,
        "jti": format!("test-{}", uuid::Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": (now as i64 + exp_offset) as usize,
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key = EncodingKey::from_rsa_pem(key_pair.private_key_pem.as_bytes()).unwrap();
    jwt_encode(&header, &claims, &key).unwrap()
}

/// Create an RS256 JWT matching the Auth V1 workflow_obo profile.
///
/// Includes `token_use=workflow_obo` and `act { sub: act_sub }`.
pub fn v1_obo_token(
    subject: uuid::Uuid,
    act_sub: uuid::Uuid,
    scope: &str,
    client_id: Option<&str>,
    exp_offset: i64,
    key_pair: &RsaTestKeyPair,
) -> String {
    let now = chrono::Utc::now().timestamp() as usize;
    let mut claims = serde_json::json!({
        "iss": "auth-service",
        "sub": subject.to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "token_use": "workflow_obo",
        "type": "access",
        "version": "v1",
        "scope": scope,
        "jti": format!("test-{}", uuid::Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": (now as i64 + exp_offset) as usize,
        "act": {
            "sub": act_sub.to_string()
        }
    });
    if let Some(cid) = client_id {
        claims["client_id"] = serde_json::json!(cid);
    }
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key = EncodingKey::from_rsa_pem(key_pair.private_key_pem.as_bytes()).unwrap();
    jwt_encode(&header, &claims, &key).unwrap()
}

/// Create an OBO token that is missing the `act` claim entirely.
pub fn v1_obo_token_missing_act(
    subject: uuid::Uuid,
    scope: &str,
    client_id: Option<&str>,
    exp_offset: i64,
    key_pair: &RsaTestKeyPair,
) -> String {
    let now = chrono::Utc::now().timestamp() as usize;
    let mut claims = serde_json::json!({
        "iss": "auth-service",
        "sub": subject.to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "token_use": "workflow_obo",
        "type": "access",
        "version": "v1",
        "scope": scope,
        "jti": format!("test-{}", uuid::Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": (now as i64 + exp_offset) as usize,
    });
    if let Some(cid) = client_id {
        claims["client_id"] = serde_json::json!(cid);
    }
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key = EncodingKey::from_rsa_pem(key_pair.private_key_pem.as_bytes()).unwrap();
    jwt_encode(&header, &claims, &key).unwrap()
}

/// Create an OBO token with an extra field at the top level.
pub fn v1_obo_token_with_extra_field(
    subject: uuid::Uuid,
    act_sub: uuid::Uuid,
    scope: &str,
    client_id: Option<&str>,
    exp_offset: i64,
    extra_key: &str,
    extra_value: &str,
    key_pair: &RsaTestKeyPair,
) -> String {
    let now = chrono::Utc::now().timestamp() as usize;
    let mut claims = serde_json::json!({
        "iss": "auth-service",
        "sub": subject.to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "token_use": "workflow_obo",
        "type": "access",
        "version": "v1",
        "scope": scope,
        "jti": format!("test-{}", uuid::Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": (now as i64 + exp_offset) as usize,
        "act": {
            "sub": act_sub.to_string()
        },
        extra_key: extra_value,
    });
    if let Some(cid) = client_id {
        claims["client_id"] = serde_json::json!(cid);
    }
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key = EncodingKey::from_rsa_pem(key_pair.private_key_pem.as_bytes()).unwrap();
    jwt_encode(&header, &claims, &key).unwrap()
}

/// Create a V1 token with an extra field (violates deny_unknown_fields).
pub fn v1_token_with_extra_field(
    subject: uuid::Uuid,
    scope: &str,
    client_id: &str,
    exp_offset: i64,
    extra_key: &str,
    extra_value: &str,
    key_pair: &RsaTestKeyPair,
) -> String {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = serde_json::json!({
        "iss": "auth-service",
        "sub": subject.to_string(),
        "aud": "svc-workflow",
        "principal_type": "agent",
        "client_id": client_id,
        "token_use": "access",
        "type": "access",
        "version": "v1",
        "scope": scope,
        "jti": format!("test-{}", uuid::Uuid::new_v4()),
        "iat": now,
        "nbf": now,
        "exp": (now as i64 + exp_offset) as usize,
        extra_key: extra_value,
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(key_pair.kid.clone());
    header.typ = Some("at+jwt".to_string());
    let key = EncodingKey::from_rsa_pem(key_pair.private_key_pem.as_bytes()).unwrap();
    jwt_encode(&header, &claims, &key).unwrap()
}

/// Generate a fresh 2048-bit RSA key pair at runtime.
///
/// The private key PEM can be used with `jsonwebtoken::EncodingKey::from_rsa_pem()`.
/// The `n_base64url` and `e_base64url` can be placed in a mock JWKS response.
pub fn generate_rsa_key_pair() -> RsaTestKeyPair {
    use rsa::pkcs8::EncodePrivateKey;
    use rsa::traits::PublicKeyParts;
    use rsa::RsaPrivateKey;

    let mut rng = rand::thread_rng();
    let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("generate RSA key pair");

    let kid = uuid::Uuid::new_v4().to_string();

    // PEM-encode the private key for jsonwebtoken
    let private_key_pem = private_key
        .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
        .expect("PEM-encode RSA private key")
        .to_string();

    // Extract n and e as base64url-encoded strings
    let n_bytes = private_key.n().to_bytes_be();
    let e_bytes = private_key.e().to_bytes_be();

    let n_base64url = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(n_bytes);
    let e_base64url = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(e_bytes);

    RsaTestKeyPair {
        private_key_pem,
        kid,
        n_base64url,
        e_base64url,
    }
}

/// Default test database URL.
const TEST_DATABASE_URL: &str = "postgres://postgres:postgres@localhost:5432/svc_workflow";

/// Create a new pool for a test and ensure migrations are applied.
///
/// SQLx migration tracking is idempotent: the `_sqlx_migrations` table records
/// which migrations have been applied. Calling `run()` multiple times is safe
/// and will only apply pending migrations.
pub async fn create_pool() -> PgPool {
    let pool = PgPool::connect(TEST_DATABASE_URL)
        .await
        .expect("failed to connect to test database");

    let migrator = sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
        .await
        .expect("failed to load migrations");
    migrator
        .run(&pool)
        .await
        .expect("failed to run migrations on test database");

    pool
}

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

/// Seed a minimal set of principals and a domain for tests.
/// Returns (principal_id, domain_id).
pub async fn seed_principal_and_domain(pool: &PgPool) -> (uuid::Uuid, uuid::Uuid) {
    let principal_id = uuid::Uuid::new_v4();
    let domain_id = uuid::Uuid::new_v4();
    let domain_key = format!("test-domain-{}", &uuid::Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        r#"
        INSERT INTO principals (principal_id, principal_type, display_name, email, enabled)
        VALUES ($1, 'HUMAN', 'Test User', 'test@example.com', TRUE)
        "#,
    )
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("failed to insert test principal");

    sqlx::query(
        r#"
        INSERT INTO domains (domain_id, domain_key, display_name, enabled)
        VALUES ($1, $2, 'Test Domain', TRUE)
        "#,
    )
    .bind(domain_id)
    .bind(&domain_key)
    .execute(pool)
    .await
    .expect("failed to insert test domain");

    (principal_id, domain_id)
}

/// Seed a principal, domain, and domain owner binding in one call.
/// Returns (principal_id, domain_id).
pub async fn seed_principal_domain_with_owner(pool: &PgPool) -> (uuid::Uuid, uuid::Uuid) {
    let (principal_id, domain_id) = seed_principal_and_domain(pool).await;
    seed_domain_owner(pool, domain_id, principal_id).await;
    (principal_id, domain_id)
}

/// Seed a second principal (for multiple-principal tests).
pub async fn seed_second_principal(pool: &PgPool) -> uuid::Uuid {
    let principal_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO principals (principal_id, principal_type, display_name, email, enabled)
        VALUES ($1, 'AGENT', 'Test Agent', 'agent@example.com', TRUE)
        "#,
    )
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("failed to insert second principal");
    principal_id
}

/// Seed a domain owner binding.
pub async fn seed_domain_owner(
    pool: &PgPool,
    domain_id: uuid::Uuid,
    principal_id: uuid::Uuid,
) -> uuid::Uuid {
    let binding_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
        VALUES ($1, $2, $3, 'DOMAIN_OWNER', TRUE)
        "#,
    )
    .bind(binding_id)
    .bind(domain_id)
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("failed to insert domain owner binding");
    binding_id
}

/// Seed a complete minimal workflow definition with one node and one transition.
/// Returns (definition_id, version_id, node_id, transition_id).
pub async fn seed_workflow_definition(
    pool: &PgPool,
    domain_id: uuid::Uuid,
) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid, uuid::Uuid) {
    let def_id = uuid::Uuid::new_v4();
    let ver_id = uuid::Uuid::new_v4();
    let node_id = uuid::Uuid::new_v4();
    let trans_id = uuid::Uuid::new_v4();

    let def_key = format!("test-def-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    sqlx::query(
        r#"
        INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name)
        VALUES ($1, $2, $3, 'Test Definition')
        "#,
    )
    .bind(def_id)
    .bind(domain_id)
    .bind(&def_key)
    .execute(pool)
    .await
    .expect("failed to insert workflow definition");

    sqlx::query(
        r#"
        INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema, submission_schema)
        VALUES ($1, $2, 1, 'DRAFT', '{"type":"object"}'::jsonb, '{"type":"object"}'::jsonb)
        "#,
    )
    .bind(ver_id)
    .bind(def_id)
    .execute(pool)
    .await
    .expect("failed to insert definition version");

    let principal_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO principals (principal_id, principal_type, display_name, email, enabled)
        VALUES ($1, 'HUMAN', 'Assignee', 'assignee@example.com', TRUE)
        "#,
    )
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("failed to insert assignee principal");

    sqlx::query(
        r#"
        INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id)
        VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'FIXED_PRINCIPAL', $3)
        "#,
    )
    .bind(node_id)
    .bind(ver_id)
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("failed to insert node definition");

    let terminal_node_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type)
        VALUES ($1, $2, 'done', 'Done', 1, 'TERMINAL', NULL)
        "#,
    )
    .bind(terminal_node_id)
    .bind(ver_id)
    .execute(pool)
    .await
    .expect("failed to insert terminal node");

    sqlx::query(
        r#"
        INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect)
        VALUES ($1, $2, 'advance-done', 'Complete', $3, $4, 'ADVANCE')
        "#,
    )
    .bind(trans_id)
    .bind(ver_id)
    .bind(node_id)
    .bind(terminal_node_id)
    .execute(pool)
    .await
    .expect("failed to insert transition");

    (def_id, ver_id, node_id, trans_id)
}
