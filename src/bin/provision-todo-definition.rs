//! Provision the agent_self_task_v1 workflow definition.
//!
//! Reads a JSON definition file with placeholders, resolves env vars,
//! validates the graph, and publishes the version.
//!
//! Usage:
//!   cargo run --bin provision-todo-definition -- <path-to-definition.json>
//!
//! Required env: DATABASE_URL, PROVISIONING_PRINCIPAL_ID, DOMAIN_ID,
//!   EFFICIENCY_MANAGER_PRINCIPAL_ID, LOBSTER_PARTNER_PRINCIPAL_ID

use std::collections::HashMap;
use std::env;
use std::fs;
use std::process;

use serde::Deserialize;
use uuid::Uuid;

use svc_workflow::application::definition::commands::{
    CreateDefinition, CreateDraftVersion, PublishVersion, RawNodeDefinition,
    RawTransitionDefinition, ReplaceDraftGraph,
};
use svc_workflow::application::definition::DefinitionService;
use svc_workflow::domain::definition::digest::{
    CanonicalDefinitionDocument, CanonicalNode, CanonicalTransition,
};
use svc_workflow::store::postgres::definition_repository::PgDefinitionRepository;

const EFFICIENCY_MANAGER_KEY: &str = "EFFICIENCY_MANAGER_PRINCIPAL_ID";
const LOBSTER_PARTNER_KEY: &str = "LOBSTER_PARTNER_PRINCIPAL_ID";
const DOMAIN_ID_KEY: &str = "DOMAIN_ID";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DefinitionFile {
    definition_key: String,
    display_name: String,
    description: Option<String>,
    version: DefinitionVersionConfig,
    domain: DomainConfig,
    nodes: Vec<NodeConfig>,
    transitions: Vec<TransitionConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DefinitionVersionConfig {
    version_number: i32,
    context_schema: Option<serde_json::Value>,
    json_schema_dialect: Option<String>,
    validator_version: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DomainConfig {
    domain_id: String,
    #[allow(dead_code)]
    domain_key: String,
    #[allow(dead_code)]
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NodeConfig {
    node_key: String,
    display_name: String,
    order_index: i32,
    node_type: String,
    assignee_ref_type: Option<String>,
    fixed_principal_id: Option<String>,
    instructions: Option<String>,
    primary_advance_transition_key: Option<String>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransitionConfig {
    transition_key: String,
    display_name: String,
    source_node_key: String,
    target_node_key: String,
    transition_effect: String,
    submission_schema: Option<serde_json::Value>,
    metadata: Option<serde_json::Value>,
}

struct ResolvedNode {
    node_key: String,
    display_name: String,
    order_index: i32,
    node_type: String,
    assignee_ref_type: Option<String>,
    fixed_principal_id: Option<Uuid>,
    instructions: Option<String>,
    primary_advance_transition_key: Option<String>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug)]
enum ProvisioningError {
    MissingEnv(String),
    InvalidUuid(String, String),
    UnknownPlaceholder(String),
    EmptyPlaceholder(String),
    Io(String),
    JsonParse(String),
    IoError(std::io::Error),
    SqlxError(sqlx::Error),
    ServiceError(String),
    DigestMismatch(String),
    ConfigConflict(String),
}
impl From<std::io::Error> for ProvisioningError {
    fn from(e: std::io::Error) -> Self {
        ProvisioningError::IoError(e)
    }
}
impl From<sqlx::Error> for ProvisioningError {
    fn from(e: sqlx::Error) -> Self {
        ProvisioningError::SqlxError(e)
    }
}
impl std::fmt::Display for ProvisioningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEnv(n) => write!(f, "required env {} is not set", n),
            Self::InvalidUuid(n, v) => write!(f, "{}='{}' is not a valid UUID", n, v),
            Self::UnknownPlaceholder(p) => write!(f, "unknown placeholder '{}'", p),
            Self::EmptyPlaceholder(p) => write!(f, "placeholder '{}' resolved to empty value", p),
            Self::Io(m) => write!(f, "I/O error: {}", m),
            Self::JsonParse(m) => write!(f, "JSON error: {}", m),
            Self::IoError(e) => write!(f, "I/O error: {}", e),
            Self::SqlxError(e) => write!(f, "DB error: {}", e),
            Self::ServiceError(m) => write!(f, "service error: {}", m),
            Self::DigestMismatch(m) => write!(f, "digest mismatch: {}", m),
            Self::ConfigConflict(m) => write!(f, "config conflict: {}", m),
        }
    }
}

fn read_env(name: &str) -> Result<String, ProvisioningError> {
    env::var(name).map_err(|_| ProvisioningError::MissingEnv(name.to_string()))
}
fn parse_uuid(name: &str, value: &str) -> Result<Uuid, ProvisioningError> {
    Uuid::parse_str(value)
        .map_err(|_| ProvisioningError::InvalidUuid(name.to_string(), value.to_string()))
}
fn resolve_placeholders(
    input: &str,
    values: &HashMap<String, String>,
) -> Result<String, ProvisioningError> {
    let mut result = input.to_string();
    loop {
        let pos = match result.find("${") {
            Some(p) => p,
            None => break Ok(result),
        };
        let end = match result[pos + 2..].find('}') {
            Some(e) => pos + 2 + e,
            None => break Ok(result),
        };
        let var = &result[pos + 2..end];
        let val = values
            .get(var)
            .ok_or_else(|| ProvisioningError::UnknownPlaceholder(var.to_string()))?;
        if val.is_empty() {
            return Err(ProvisioningError::EmptyPlaceholder(var.to_string()));
        }
        result = result.replace(&format!("${{{}}}", var), val);
    }
}

fn compute_digest(
    key: &str,
    ver: i32,
    ctx: &Option<serde_json::Value>,
    dialect: &Option<String>,
    validator: &Option<String>,
    nodes: &[ResolvedNode],
    trans: &[TransitionConfig],
) -> Result<String, ProvisioningError> {
    let cn: Vec<CanonicalNode> = nodes
        .iter()
        .map(|n| CanonicalNode {
            node_key: n.node_key.clone(),
            display_name: n.display_name.clone(),
            order_index: n.order_index,
            node_type: n.node_type.clone(),
            assignee_ref_type: n.assignee_ref_type.clone(),
            fixed_principal_id: n.fixed_principal_id.map(|id| id.to_string()),
            instructions: n.instructions.clone(),
            primary_advance_transition_key: n.primary_advance_transition_key.clone(),
            metadata: n.metadata.clone(),
        })
        .collect();
    let ct: Vec<CanonicalTransition> = trans
        .iter()
        .map(|t| CanonicalTransition {
            transition_key: t.transition_key.clone(),
            display_name: t.display_name.clone(),
            source_node_key: t.source_node_key.clone(),
            target_node_key: t.target_node_key.clone(),
            transition_effect: t.transition_effect.clone(),
            submission_schema: t.submission_schema.clone(),
            metadata: t.metadata.clone(),
        })
        .collect();
    let doc = CanonicalDefinitionDocument {
        definition_key: key.to_string(),
        version_number: ver,
        json_schema_dialect: dialect.clone(),
        validator_version: validator.clone(),
        context_schema: ctx.clone(),
        nodes: cn,
        transitions: ct,
    };
    let bytes =
        serde_json::to_vec(&doc).map_err(|e| ProvisioningError::JsonParse(e.to_string()))?;
    use sha2::{Digest, Sha256};
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("ERROR: {}", e);
        process::exit(1);
    }
    println!("OK: agent_self_task_v1 provisioned successfully");
}

async fn run() -> Result<(), ProvisioningError> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <definition.json>", args[0]);
        process::exit(1);
    }
    let path = &args[1];

    let db = read_env("DATABASE_URL")?;
    let actor = parse_uuid(
        "PROVISIONING_PRINCIPAL_ID",
        &read_env("PROVISIONING_PRINCIPAL_ID")?,
    )?;
    let did = parse_uuid("DOMAIN_ID", &read_env("DOMAIN_ID")?)?;
    let em = read_env(EFFICIENCY_MANAGER_KEY)?;
    let lp = read_env(LOBSTER_PARTNER_KEY)?;
    let _eid = parse_uuid(EFFICIENCY_MANAGER_KEY, &em)?;
    let _lid = parse_uuid(LOBSTER_PARTNER_KEY, &lp)?;

    let mut vals = HashMap::new();
    vals.insert(EFFICIENCY_MANAGER_KEY.to_string(), em);
    vals.insert(LOBSTER_PARTNER_KEY.to_string(), lp);
    vals.insert(DOMAIN_ID_KEY.to_string(), did.to_string());

    let content = fs::read_to_string(path).map_err(|e| ProvisioningError::Io(e.to_string()))?;
    let resolved = resolve_placeholders(content.trim_start_matches('\u{feff}'), &vals)?;
    let file: DefinitionFile =
        serde_json::from_str(&resolved).map_err(|e| ProvisioningError::JsonParse(e.to_string()))?;

    let fdid = parse_uuid("domainId", &file.domain.domain_id)?;
    if fdid != did {
        return Err(ProvisioningError::ConfigConflict(
            "DOMAIN_ID mismatch".to_string(),
        ));
    }

    let mut rnodes = Vec::new();
    for n in &file.nodes {
        let fp = match &n.fixed_principal_id {
            Some(s) if s.starts_with("${") && s.ends_with('}') => {
                let k = &s[2..s.len() - 1];
                Some(parse_uuid(
                    k,
                    vals.get(k)
                        .ok_or_else(|| ProvisioningError::UnknownPlaceholder(k.to_string()))?,
                )?)
            }
            Some(s) => Some(parse_uuid("fixed_principal_id", s)?),
            None => None,
        };
        rnodes.push(ResolvedNode {
            node_key: n.node_key.clone(),
            display_name: n.display_name.clone(),
            order_index: n.order_index,
            node_type: n.node_type.clone(),
            assignee_ref_type: n.assignee_ref_type.clone(),
            fixed_principal_id: fp,
            instructions: n.instructions.clone(),
            primary_advance_transition_key: n.primary_advance_transition_key.clone(),
            metadata: n.metadata.clone(),
        });
    }

    let digest = compute_digest(
        &file.definition_key,
        file.version.version_number,
        &file.version.context_schema,
        &file.version.json_schema_dialect,
        &file.version.validator_version,
        &rnodes,
        &file.transitions,
    )?;

    let pool = sqlx::PgPool::connect(&db).await?;

    // Idempotency check: compare against existing version with same key/version.
    if let Some((_, stored_digest, vs)) = sqlx::query_as::<_, (String, Option<String>, String)>(
        "SELECT dv.definition_version_id::text, dv.definition_digest, dv.version_status::text
         FROM workflow_definition_versions dv JOIN workflow_definitions d ON d.workflow_definition_id = dv.workflow_definition_id
         WHERE d.definition_key = $1 AND d.domain_id = $2 AND dv.version_number = $3 ORDER BY dv.created_at DESC LIMIT 1",
    ).bind(&file.definition_key).bind(did).bind(file.version.version_number).fetch_optional(&pool).await? {
        match vs.as_str() {
            "PUBLISHED" | "DEPRECATED" => {
                if stored_digest.as_deref() == Some(&digest) {
                    println!("ALREADY_PROVISIONED: definition '{}' version {} already exists with matching digest", file.definition_key, file.version.version_number);
                    return Ok(());
                }
                // Different content for same key/version — fail closed
                eprintln!("DEFINITION_VERSION_DIGEST_MISMATCH: definition '{}' version {} exists with different content.\n  stored digest: {}\n  requested digest: {}\n  Refusing to overwrite.",
                    file.definition_key, file.version.version_number,
                    stored_digest.as_deref().unwrap_or("(null)"), &digest);
                return Err(ProvisioningError::DigestMismatch(format!(
                    "DEFINITION_VERSION_DIGEST_MISMATCH: version {} of '{}' already published with different content",
                    file.version.version_number, file.definition_key)));
            }
            "DRAFT" => {
                if stored_digest.as_deref() == Some(&digest) {
                    println!("Draft version {} already exists with matching digest, will replace graph", file.version.version_number);
                } else {
                    println!("Draft version {} exists with different digest, will replace graph", file.version.version_number);
                }
                // Proceed — draft replacement is allowed
            }
            _ => {
                // REVOKED — fall through to recreate
            }
        }
    }

    let repo = PgDefinitionRepository::new(pool.clone());
    let svc = DefinitionService::new(repo);

    let def_id = match sqlx::query_as::<_, (String,)>(
        "SELECT workflow_definition_id::text FROM workflow_definitions WHERE definition_key = $1 AND domain_id = $2",
    ).bind(&file.definition_key).bind(did).fetch_optional(&pool).await? {
        Some((id,)) => { println!("Definition '{}' reusing id={}", file.definition_key, id); Uuid::parse_str(&id).unwrap() }
        None => { let d = svc.create_definition(CreateDefinition {
            actor_principal_id: actor, owner_domain_id: did, definition_key: file.definition_key.clone(),
            display_name: file.display_name.clone(), description: file.description.clone(), metadata: None,
        }).await.map_err(|e| ProvisioningError::ServiceError(e.to_string()))?;
            let id = d.id.into_uuid(); println!("Created definition id={}", id); id }
    };

    let ver_id = match sqlx::query_as::<_, (String,)>(
        "SELECT definition_version_id::text FROM workflow_definition_versions WHERE workflow_definition_id = $1 AND version_number = $2 AND version_status = 'DRAFT'",
    ).bind(def_id).bind(file.version.version_number).fetch_optional(&pool).await? {
        Some((id,)) => { println!("Draft {} reusing id={}", file.version.version_number, id); Uuid::parse_str(&id).unwrap() }
        None => { let v = svc.create_draft_version(CreateDraftVersion {
            actor_principal_id: actor, workflow_definition_id: def_id, context_schema: file.version.context_schema.clone(),
            json_schema_dialect: file.version.json_schema_dialect.clone(), validator_version: file.version.validator_version.clone(), metadata: None,
        }).await.map_err(|e| ProvisioningError::ServiceError(e.to_string()))?;
            let id = v.id.into_uuid(); println!("Created draft id={}", id); id }
    };

    svc.replace_draft_graph(ReplaceDraftGraph {
        actor_principal_id: actor,
        definition_version_id: ver_id,
        context_schema: file.version.context_schema.clone(),
        nodes: rnodes
            .iter()
            .map(|n| RawNodeDefinition {
                node_key: n.node_key.clone(),
                display_name: n.display_name.clone(),
                order_index: n.order_index,
                node_type: n.node_type.clone(),
                assignee_ref_type: n.assignee_ref_type.clone(),
                fixed_principal_id: n.fixed_principal_id,
                instructions: n.instructions.clone(),
                primary_advance_transition_key: n.primary_advance_transition_key.clone(),
                metadata: n.metadata.clone(),
            })
            .collect(),
        transitions: file
            .transitions
            .iter()
            .map(|t| RawTransitionDefinition {
                transition_key: t.transition_key.clone(),
                display_name: t.display_name.clone(),
                source_node_key: t.source_node_key.clone(),
                target_node_key: t.target_node_key.clone(),
                transition_effect: t.transition_effect.clone(),
                submission_schema: t.submission_schema.clone(),
                metadata: t.metadata.clone(),
            })
            .collect(),
    })
    .await
    .map_err(|e| ProvisioningError::ServiceError(e.to_string()))?;
    println!("Graph replaced");

    if sqlx::query_as::<_, (String,)>(
        "SELECT definition_version_id::text FROM workflow_definition_versions WHERE workflow_definition_id = $1 AND version_number = $2 AND version_status = 'PUBLISHED'",
    ).bind(def_id).bind(file.version.version_number).fetch_optional(&pool).await?.is_some() {
        println!("Already PUBLISHED");
    } else {
        svc.publish_version(PublishVersion { actor_principal_id: actor, definition_version_id: ver_id })
            .await.map_err(|e| ProvisioningError::ServiceError(e.to_string()))?;
        println!("Published version {}", file.version.version_number);
    }

    // Digest is set during replace_draft_graph; skip post-publish update.
    println!("Digest: {}", digest);
    Ok(())
}
