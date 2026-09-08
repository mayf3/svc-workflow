//! Bounded offline identity repair operator (P9 historical identity repair).
//!
//! Command family: WORKFLOW_IDENTITY_REPAIR_V1
//! Governing authority:
//! `docs/specs/SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2.md`
//! CTR-CIR-001 (closed read-only plan), CTR-CIR-004 (preserved lineage),
//! CTR-CIR-006 (atomic apply and concurrency), CTR-CIR-007 (receipt,
//! idempotency and unknown outcome). Structural patterns (plan/apply/verify/
//! receipt/fail-closed, outcome classes, canonical hashing, one
//! SERIALIZABLE transaction, advisory repair lock, commit-outcome probing)
//! follow the accepted `trusted_fleet_principal_cutover_v1` operator; this is
//! intentionally an offline one-shot tool, not a reassignment API.
//!
//! Scope (bounded): map STALE auth naked-name Principals to their unique
//! canonical successors by inserting IMMUTABLE
//! `workflow_identity_successor_lines` rows (migration 0025). The sole
//! business write of this operator is INSERT into that lineage table.
//!
//! IMMUTABILITY BOUNDARY (hard):
//!   * no UPDATE/DELETE of `workflow_node_visits`, `workflow_events`, or any
//!     historical fact — the operator issues no such statement at all;
//!   * no fake equivalence record between old and new Principal: the stale
//!     Principal keeps its own history and gains no alias authority; the
//!     lineage row only enriches HR-facing read projections with the
//!     canonical agent id.
//!
//! Deliberate scope deltas from the parent spec's one-time production
//! reconciliation operator (documented, not silent):
//!   * the plan is compiled from a reviewed pairs file (exact
//!     source/successor UUID pairs) instead of a frozen artifact binary;
//!     there are no OLD/NEW/scope/discovery arguments;
//!   * the auth/dsh directory cross-check is OUT OF SCOPE here: it was
//!     performed mechanically by the census phase and the resulting evidence
//!     blob is recorded AS GIVEN in the lineage row (`evidence` jsonb);
//!   * the receipt is a JSON file (`--receipt-out`), not a
//!     `workflow_command_receipts` row — this tool bin performs no audit or
//!     receipt writes to the Workflow database;
//!   * a clean checkout is NOT required (tool bin, not a release artifact).
//!
//! Usage (hand-parsed, closed):
//!   identity_repair_v1 plan     --pairs <json-file> [--migrate]
//!   identity_repair_v1 apply    --pairs <json-file> --plan-sha256 <hex> \
//!                               --receipt-out <path> [--migrate]
//!   identity_repair_v1 verify   --pairs <json-file> --plan-sha256 <hex> [--migrate]
//!   identity_repair_v1 --migrate
//!
//! Env: DATABASE_URL (required) — the Workflow database.

use chrono::{SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Row, Transaction};
use std::{collections::BTreeMap, env, fs, process};
use uuid::Uuid;

const SCHEMA: &str = "workflow_identity_repair_plan_v1";
const RECEIPT_SCHEMA: &str = "workflow_identity_repair_receipt_v1";
const COMMAND_FAMILY: &str = "WORKFLOW_IDENTITY_REPAIR_V1";
const ALLOWED_CLASSIFICATIONS: [&str; 2] = [
    "STALE_PRINCIPAL_WITH_UNIQUE_REPAIR",
    "MECHANICALLY_PROVEN_SUCCESSOR",
];
/// Single concurrent production repair (CTR-CIR-006): one goal-specific
/// exclusive lock serializes every apply of this operator.
const REPAIR_LOCK: &str = "workflow_identity_repair_v1";

#[derive(Debug)]
struct Error(String);
type Result<T> = std::result::Result<T, Error>;
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}
impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self(format!("database conflict: {e}"))
    }
}
fn conflict(s: impl Into<String>) -> Error {
    Error(format!("CONFLICT: {}", s.into()))
}
fn outcome_error(outcome: &str, s: impl Into<String>) -> Error {
    Error(format!("{outcome}: {}", s.into()))
}

#[derive(Debug)]
enum Mode {
    Plan {
        pairs_path: String,
        migrate: bool,
    },
    Apply {
        pairs_path: String,
        plan_sha: String,
        receipt_out: String,
        migrate: bool,
    },
    Verify {
        pairs_path: String,
        plan_sha: String,
        migrate: bool,
    },
    MigrateOnly,
}

fn parse_args() -> Result<Mode> {
    let args: Vec<String> = env::args().skip(1).collect();
    let take_flag = |args: &[String], name: &str| -> Result<Option<String>> {
        let mut it = args.iter();
        while let Some(a) = it.next() {
            if a == name {
                let value = it
                    .next()
                    .ok_or_else(|| conflict(format!("{name} requires a value")))?;
                return Ok(Some(value.clone()));
            }
        }
        Ok(None)
    };
    let has_flag = |args: &[String], name: &str| args.iter().any(|a| a == name);
    match args.first().map(String::as_str) {
        Some("--migrate") if args.len() == 1 => Ok(Mode::MigrateOnly),
        Some("plan") => {
            let pairs_path = take_flag(&args[1..], "--pairs")?
                .ok_or_else(|| conflict("plan requires --pairs <json-file>"))?;
            Ok(Mode::Plan { pairs_path, migrate: has_flag(&args[1..], "--migrate") })
        }
        Some("apply") => Ok(Mode::Apply {
            pairs_path: take_flag(&args[1..], "--pairs")?
                .ok_or_else(|| conflict("apply requires --pairs <json-file>"))?,
            plan_sha: take_flag(&args[1..], "--plan-sha256")?
                .ok_or_else(|| conflict("apply requires --plan-sha256 <hex>"))?,
            receipt_out: take_flag(&args[1..], "--receipt-out")?
                .ok_or_else(|| conflict("apply requires --receipt-out <path>"))?,
            migrate: has_flag(&args[1..], "--migrate"),
        }),
        Some("verify") => Ok(Mode::Verify {
            pairs_path: take_flag(&args[1..], "--pairs")?
                .ok_or_else(|| conflict("verify requires --pairs <json-file>"))?,
            plan_sha: take_flag(&args[1..], "--plan-sha256")?
                .ok_or_else(|| conflict("verify requires --plan-sha256 <hex>"))?,
            migrate: has_flag(&args[1..], "--migrate"),
        }),
        _ => Err(conflict(
            "use only: plan --pairs <file> | apply --pairs <file> --plan-sha256 <hex> --receipt-out <path> | verify --pairs <file> --plan-sha256 <hex> | --migrate; arbitrary IDs, scopes and subsets are forbidden",
        )),
    }
}

/// One reviewed repair pair. The pairs file is the reviewed authority for
/// this operator: exact UUIDs only, canonical agent-id grammar, closed
/// classification set. The directory cross-check evidence is recorded AS
/// GIVEN (produced by the census phase; out of scope here).
#[derive(Clone, Debug, Deserialize)]
struct PairInput {
    #[serde(rename = "sourcePrincipalId")]
    source_principal_id: Uuid,
    #[serde(rename = "successorPrincipalId")]
    successor_principal_id: Uuid,
    #[serde(rename = "legacyAgentId")]
    legacy_agent_id: String,
    #[serde(rename = "canonicalAgentId")]
    canonical_agent_id: String,
    #[serde(rename = "classification")]
    classification: String,
    #[serde(rename = "evidence")]
    evidence: Value,
    #[serde(rename = "repairReason")]
    repair_reason: String,
}

/// Canonical agent-id grammar: `^agt_[A-Za-z0-9_-]+$` (checked without a
/// regex dependency — the allowed set is a fixed ASCII class).
fn canonical_agent_id_valid(id: &str) -> bool {
    id.len() > 4
        && id.starts_with("agt_")
        && id[4..]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Pure, database-independent pair validation (grammar/classification/
/// shape). Database identity checks happen separately in observe/apply.
fn validate_pairs(pairs: &[PairInput]) -> Result<()> {
    if pairs.is_empty() {
        return Err(conflict("pairs file is empty"));
    }
    let mut sources = std::collections::HashSet::new();
    for (i, p) in pairs.iter().enumerate() {
        if p.source_principal_id == p.successor_principal_id {
            return Err(conflict(format!("pair {}: source equals successor", i + 1)));
        }
        if !canonical_agent_id_valid(&p.canonical_agent_id) {
            return Err(conflict(format!(
                "pair {}: canonicalAgentId {:?} does not match ^agt_[A-Za-z0-9_-]+$",
                i + 1,
                p.canonical_agent_id
            )));
        }
        if !ALLOWED_CLASSIFICATIONS.contains(&p.classification.as_str()) {
            return Err(conflict(format!(
                "pair {}: classification {:?} is outside the closed set",
                i + 1,
                p.classification
            )));
        }
        if p.legacy_agent_id.is_empty() || p.repair_reason.trim().is_empty() {
            return Err(conflict(format!(
                "pair {}: legacyAgentId and repairReason must be non-empty",
                i + 1
            )));
        }
        if !p.evidence.is_object() {
            return Err(conflict(format!(
                "pair {}: evidence must be a JSON object",
                i + 1
            )));
        }
        if !sources.insert(p.source_principal_id) {
            return Err(conflict(format!(
                "pair {}: duplicate sourcePrincipalId {}",
                i + 1,
                p.source_principal_id
            )));
        }
    }
    // Cross-pair ambiguity: the same successor Principal must never be
    // claimed with two different canonical agent ids inside one repair.
    let mut claims: BTreeMap<Uuid, String> = BTreeMap::new();
    for (i, p) in pairs.iter().enumerate() {
        match claims.get(&p.successor_principal_id) {
            Some(existing) if existing != &p.canonical_agent_id => {
                return Err(conflict(format!(
                    "pair {}: successor {} is claimed with conflicting canonical agent ids ({:?} vs {:?})",
                    i + 1,
                    p.successor_principal_id,
                    existing,
                    p.canonical_agent_id
                )));
            }
            _ => {
                claims.insert(p.successor_principal_id, p.canonical_agent_id.clone());
            }
        }
    }
    Ok(())
}

fn load_pairs(path: &str) -> Result<(Vec<PairInput>, String)> {
    let raw = fs::read(path)?;
    let file_sha = hex::encode(Sha256::digest(&raw));
    let pairs: Vec<PairInput> = serde_json::from_slice(&raw)
        .map_err(|e| conflict(format!("pairs file is invalid: {e}")))?;
    validate_pairs(&pairs)?;
    Ok((pairs, file_sha))
}

fn digest(v: &Value) -> Result<String> {
    let s = jcs_canonicalize::canonicalize(
        &serde_json::to_string(v).map_err(|e| conflict(e.to_string()))?,
    )
    .map_err(|e| conflict(e.to_string()))?;
    Ok(hex::encode(Sha256::digest(s.as_bytes())))
}

/// Existing lineage row projection used for identical/different decisions.
#[derive(Clone, Debug)]
struct LineageRow {
    successor_principal_id: Uuid,
    legacy_agent_id: String,
    canonical_agent_id: String,
    classification: String,
}

impl LineageRow {
    fn matches(&self, p: &PairInput) -> bool {
        self.successor_principal_id == p.successor_principal_id
            && self.legacy_agent_id == p.legacy_agent_id
            && self.canonical_agent_id == p.canonical_agent_id
            && self.classification == p.classification
    }
}

async fn existing_rows_for_sources(
    e: impl sqlx::PgExecutor<'_>,
    sources: &[Uuid],
) -> Result<BTreeMap<Uuid, LineageRow>> {
    let rows = sqlx::query(
        "SELECT source_principal_id, successor_principal_id, legacy_agent_id,
                canonical_agent_id, classification
         FROM workflow_identity_successor_lines
         WHERE source_principal_id = ANY($1)",
    )
    .bind(sources)
    .fetch_all(e)
    .await?;
    let mut map = BTreeMap::new();
    for r in rows {
        map.insert(
            r.get::<Uuid, _>("source_principal_id"),
            LineageRow {
                successor_principal_id: r.get("successor_principal_id"),
                legacy_agent_id: r.get("legacy_agent_id"),
                canonical_agent_id: r.get("canonical_agent_id"),
                classification: r.get("classification"),
            },
        );
    }
    Ok(map)
}

/// UNRELATED_DIGEST: md5 over all EXISTING lineage rows, ordered by source
/// principal id, excluding volatile timestamp text (timestamptz rendering is
/// session-dependent). Empty table -> md5 of the empty string. Computed in
/// SQL so no md5 dependency is added to this crate.
async fn unrelated_digest(e: impl sqlx::PgExecutor<'_>) -> Result<String> {
    let value: String = sqlx::query_scalar(
        "SELECT md5(COALESCE(string_agg(row_text, E'\\n' ORDER BY source_principal_id), ''))
         FROM (
           SELECT source_principal_id,
                  source_principal_id::text || '|' || successor_principal_id::text || '|'
                  || legacy_agent_id || '|' || canonical_agent_id || '|' || classification || '|'
                  || evidence::text || '|' || repair_reason AS row_text
           FROM workflow_identity_successor_lines
         ) t",
    )
    .fetch_one(e)
    .await?;
    Ok(value)
}

/// Successor-claim map from EXISTING lineage rows: successor principal ->
/// distinct canonical agent ids claimed by ANY source. Used for ambiguity
/// detection against the reviewed pairs.
async fn successor_claims(
    e: impl sqlx::PgExecutor<'_>,
    successors: &[Uuid],
) -> Result<BTreeMap<Uuid, Vec<String>>> {
    let rows = sqlx::query(
        "SELECT DISTINCT successor_principal_id, canonical_agent_id
         FROM workflow_identity_successor_lines
         WHERE successor_principal_id = ANY($1)",
    )
    .bind(successors)
    .fetch_all(e)
    .await?;
    let mut map: BTreeMap<Uuid, Vec<String>> = BTreeMap::new();
    for r in rows {
        map.entry(r.get("successor_principal_id"))
            .or_default()
            .push(r.get::<String, _>("canonical_agent_id"));
    }
    Ok(map)
}

/// Affected assignment counts per source (read-only):
///   AFFECTED_ASSIGNMENT_COUNT  - all visits ever assigned to the source
///   AFFECTED_ACTIONABLE_COUNT  - those whose instance is nonterminal
/// (no visit row is touched; this is a census, not a mutation).
async fn affected_counts(
    tx: &mut Transaction<'_, Postgres>,
    sources: &[Uuid],
) -> Result<BTreeMap<Uuid, (i64, i64)>> {
    let total: BTreeMap<Uuid, i64> = sqlx::query(
        "SELECT assignee_principal_id AS src, count(*) AS n
         FROM workflow_node_visits
         WHERE assignee_principal_id = ANY($1) GROUP BY 1",
    )
    .bind(sources)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|r| (r.get::<Uuid, _>("src"), r.get::<i64, _>("n")))
    .collect();
    let actionable: BTreeMap<Uuid, i64> = sqlx::query(
        "SELECT v.assignee_principal_id AS src, count(*) AS n
         FROM workflow_node_visits v
         JOIN workflow_instances wi ON wi.workflow_instance_id = v.workflow_instance_id
         WHERE v.assignee_principal_id = ANY($1)
           AND wi.cancelled = FALSE AND wi.archived_at IS NULL
         GROUP BY 1",
    )
    .bind(sources)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|r| (r.get::<Uuid, _>("src"), r.get::<i64, _>("n")))
    .collect();
    let mut map = BTreeMap::new();
    for src in sources {
        map.insert(
            *src,
            (
                total.get(src).copied().unwrap_or(0),
                actionable.get(src).copied().unwrap_or(0),
            ),
        );
    }
    Ok(map)
}

/// Read-only observation of the full preimage. Emits the canonical plan
/// document and its SHA-256 (over the JCS-canonicalized document bytes).
/// One REPEATABLE READ transaction, zero writes, no mutation lock, no
/// audit/receipt/database writes (CTR-CIR-001).
///
/// Fail-closed per-pair conditions:
///   * source exists in principals (this database);
///   * successor exists AND enabled AND principal_type = 'AGENT';
///   * source != successor (also checked purely in validate_pairs);
///   * no conflicting lineage row for the source — an IDENTICAL existing row
///     sets LINEAGE_ALREADY_EXISTS=true (plan still emits; apply becomes a
///     NOOP), a DIFFERENT one is a hard conflict;
///   * canonicalAgentId grammar ^agt_[A-Za-z0-9_-]+$ (pure, validated above);
///   * cross-pair/existing-row successor ambiguity -> AMBIGUITY > 0 -> abort.
async fn observe(w: &PgPool, pairs: &[PairInput], pairs_file_sha: &str) -> Result<(Value, String)> {
    let mut tx = w.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    let (doc, _) = observe_tx(&mut tx, pairs, pairs_file_sha).await?;
    tx.commit().await?;
    let plan_hash = digest(&doc)?;
    Ok((doc, plan_hash))
}

async fn observe_tx(
    tx: &mut Transaction<'_, Postgres>,
    pairs: &[PairInput],
    pairs_file_sha: &str,
) -> Result<(Value, Vec<(String, String, bool)>)> {
    let sources: Vec<Uuid> = pairs.iter().map(|p| p.source_principal_id).collect();
    let successors: Vec<Uuid> = pairs.iter().map(|p| p.successor_principal_id).collect();

    let existing = existing_rows_for_sources(&mut **tx, &sources).await?;
    let claims = successor_claims(&mut **tx, &successors).await?;
    let counts = affected_counts(tx, &sources).await?;
    let unrelated = unrelated_digest(&mut **tx).await?;

    // Principal identity census for all referenced UUIDs (single query).
    let mut wanted: Vec<Uuid> = sources.clone();
    wanted.extend(successors.iter().copied());
    wanted.sort();
    wanted.dedup();
    let mut principal_state: BTreeMap<Uuid, (String, bool, Option<Value>)> = BTreeMap::new();
    for r in sqlx::query(
        "SELECT principal_id, principal_type::text AS t, enabled, metadata
         FROM principals WHERE principal_id = ANY($1)",
    )
    .bind(&wanted)
    .fetch_all(&mut **tx)
    .await?
    {
        let metadata: Option<Value> = r.try_get("metadata").ok();
        principal_state.insert(
            r.get("principal_id"),
            (
                r.get::<String, _>("t"),
                r.get::<bool, _>("enabled"),
                metadata,
            ),
        );
    }

    let mut entries = Vec::new();
    let mut per_pair_outcomes: Vec<(String, String, bool)> = Vec::new();
    let mut total_assignments = 0i64;
    let mut total_actionable = 0i64;
    let mut ambiguity_count = 0usize;
    let mut conflict_count = 0usize;
    let mut already_exists_count = 0usize;

    for p in pairs {
        let mut conflicts: Vec<String> = Vec::new();
        let (assignment, actionable) = counts
            .get(&p.source_principal_id)
            .copied()
            .unwrap_or((0, 0));
        total_assignments += assignment;
        total_actionable += actionable;

        match principal_state.get(&p.source_principal_id) {
            None => conflicts.push("source principal does not exist in this database".into()),
            Some(_) => {}
        }
        match principal_state.get(&p.successor_principal_id) {
            None => conflicts.push("successor principal does not exist in this database".into()),
            Some((t, enabled, _)) => {
                if !enabled {
                    conflicts.push("successor principal is not enabled".into());
                }
                if t != "AGENT" {
                    conflicts.push(format!("successor principal_type is {t}, not AGENT"));
                }
            }
        }

        let mut lineage_already_exists = false;
        match existing.get(&p.source_principal_id) {
            Some(row) if row.matches(p) => lineage_already_exists = true,
            Some(row) => conflicts.push(format!(
                "conflicting lineage row for source: existing successor={} canonical={:?} classification={:?}",
                row.successor_principal_id, row.canonical_agent_id, row.classification
            )),
            None => {}
        }

        // AMBIGUITY: this successor already claimed by ANOTHER source's
        // lineage with a DIFFERENT canonical agent id (many-to-one with the
        // SAME canonical id is legitimate; a second distinct claim is not).
        for claimed in claims.get(&p.successor_principal_id).into_iter().flatten() {
            if claimed != &p.canonical_agent_id {
                conflicts.push(format!(
                    "ambiguous successor: already claimed with canonical agent id {claimed:?}"
                ));
                ambiguity_count += 1;
            }
        }

        if conflicts.is_empty() {
            if lineage_already_exists {
                already_exists_count += 1;
            }
        } else {
            conflict_count += 1;
        }
        let outcome = if !conflicts.is_empty() {
            "CONFLICT"
        } else if lineage_already_exists {
            "NOOP"
        } else {
            "PLANNED"
        };
        per_pair_outcomes.push((
            p.source_principal_id.to_string(),
            outcome.to_string(),
            lineage_already_exists,
        ));
        entries.push(json!({
            "sourcePrincipalId": p.source_principal_id,
            "successorPrincipalId": p.successor_principal_id,
            "legacyAgentId": p.legacy_agent_id,
            "canonicalAgentId": p.canonical_agent_id,
            "classification": p.classification,
            "evidence": p.evidence,
            "repairReason": p.repair_reason,
            "lineageAlreadyExists": lineage_already_exists,
            "affectedAssignmentCount": assignment,
            "affectedActionableCount": actionable,
            "outcome": outcome,
            "conflicts": conflicts,
        }));
    }

    // Deterministic order: by source principal id.
    entries.sort_by(|a, b| {
        a.get("sourcePrincipalId")
            .and_then(Value::as_str)
            .cmp(&b.get("sourcePrincipalId").and_then(Value::as_str))
    });

    let write_count: usize = pairs
        .len()
        .saturating_sub(already_exists_count)
        .saturating_sub(conflict_count.min(pairs.len()));
    let doc = json!({
        "schema": SCHEMA,
        "mode": "READ_ONLY_CANONICAL_PLAN",
        "productionChange": "LINEAGE_ROWS_ONLY",
        "commandFamily": COMMAND_FAMILY,
        "authorityNotes": {
            "governingSpec": "SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2",
            "contracts": ["CTR-CIR-001", "CTR-CIR-004", "CTR-CIR-006", "CTR-CIR-007"],
            "immutability": "no UPDATE/DELETE of workflow_node_visits, workflow_events, or any historical fact; the lineage table is append-only",
            "directoryCrossCheck": "OUT OF SCOPE here - performed mechanically by the census phase; the evidence blob is recorded as given"
        },
        "pairsFileSha256": pairs_file_sha,
        "pairs": entries,
        "summary": {
            "pairCount": pairs.len(),
            "affectedAssignmentCount": total_assignments,
            "affectedActionableCount": total_actionable,
            "ambiguityCount": ambiguity_count,
            "conflictCount": conflict_count,
            "lineageAlreadyExistsCount": already_exists_count,
            "plannedInsertCount": write_count,
            "visitRowsMutated": 0,
            "instanceRowsMutated": 0,
            "eventRowsMutated": 0,
            "historyRewriteCount": 0
        },
        "unrelatedDigest": unrelated,
    });
    Ok((doc, per_pair_outcomes))
}

async fn run_migrate(w: &PgPool) -> Result<()> {
    let migrator = sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
        .await
        .map_err(|e| conflict(format!("cannot load migrations: {e}")))?;
    migrator
        .run(w)
        .await
        .map_err(|e| conflict(format!("migration failed: {e}")))?;
    Ok(())
}

/// Terminal-state probe used by apply's idempotent short-circuit and by
/// verify: every pair has an IDENTICAL lineage row and the read-path
/// projection resolves the canonical agent id for every affected instance.
async fn terminal_exact(w: &PgPool, pairs: &[PairInput]) -> Result<(bool, Vec<Value>)> {
    let sources: Vec<Uuid> = pairs.iter().map(|p| p.source_principal_id).collect();
    let existing = existing_rows_for_sources(w, &sources).await?;
    let mut all_ok = true;
    let mut details = Vec::new();
    for p in pairs {
        let identical = existing
            .get(&p.source_principal_id)
            .is_some_and(|r| r.matches(p));
        let mut proof_instances = 0i64;
        let mut proof_failures: Vec<String> = Vec::new();
        if identical {
            let instances: Vec<Uuid> = sqlx::query_scalar(
                "SELECT DISTINCT workflow_instance_id FROM workflow_node_visits
                 WHERE assignee_principal_id = $1",
            )
            .bind(p.source_principal_id)
            .fetch_all(w)
            .await?;
            // Re-run the enriched read-path projection for each affected
            // instance: every visit row assigned to the source must resolve
            // the canonical agent id through the lineage join.
            for instance in instances {
                let rows: Vec<(Uuid, Option<String>)> = sqlx::query_as(
                    "SELECT v.node_visit_id, wisl.canonical_agent_id
                     FROM workflow_node_visits v
                     LEFT JOIN workflow_identity_successor_lines wisl
                       ON wisl.source_principal_id = v.assignee_principal_id
                     WHERE v.workflow_instance_id = $1
                       AND v.assignee_principal_id = $2",
                )
                .bind(instance)
                .bind(p.source_principal_id)
                .fetch_all(w)
                .await?;
                for (visit, canonical) in rows {
                    if canonical.as_deref() != Some(p.canonical_agent_id.as_str()) {
                        proof_failures.push(format!(
                            "instance {instance} visit {visit} resolved {canonical:?}"
                        ));
                    }
                    proof_instances += 1;
                }
            }
            if !proof_failures.is_empty() {
                all_ok = false;
            }
        } else {
            all_ok = false;
        }
        details.push(json!({
            "sourcePrincipalId": p.source_principal_id,
            "successorPrincipalId": p.successor_principal_id,
            "canonicalAgentId": p.canonical_agent_id,
            "lineageRowIdentical": identical,
            "projectionProofVisits": proof_instances,
            "projectionFailures": proof_failures,
        }));
    }
    Ok((all_ok, details))
}

async fn write_receipt(path: &str, receipt: &Value) -> Result<()> {
    let canonical = jcs_canonicalize::canonicalize(
        &serde_json::to_string(receipt).map_err(|e| conflict(e.to_string()))?,
    )
    .map_err(|e| conflict(e.to_string()))?;
    fs::write(path, canonical.as_bytes())?;
    Ok(())
}

async fn apply(
    w: &PgPool,
    pairs: &[PairInput],
    pairs_file_sha: &str,
    plan_sha: &str,
    receipt_out: &str,
) -> Result<()> {
    if plan_sha.len() != 64 || !plan_sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(conflict("--plan-sha256 must be a 64-char hex SHA-256"));
    }

    // Idempotent short-circuit FIRST (same shape as the accepted cutover
    // operator): when the exact terminal state is already present, apply is
    // a NOOP regardless of the preimage plan hash, because the preimage is
    // no longer recomputable after the lineage rows exist. The terminal
    // probe itself re-proves every row identical plus the read-path
    // projection, so a stale hash can never mask drifted state.
    let (terminal, terminal_details) = terminal_exact(w, pairs).await?;
    if terminal {
        let receipt = json!({
            "schema": RECEIPT_SCHEMA,
            "commandFamily": COMMAND_FAMILY,
            "mode": "apply",
            "outcome": "NOOP",
            "writes": 0,
            "insertedCount": 0,
            "skippedCount": pairs.len(),
            "planSha256": plan_sha,
            "pairsFileSha256": pairs_file_sha,
            "completedAtUtc": Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true),
            "immutability": {
                "visitRowsMutated": 0,
                "instanceRowsMutated": 0,
                "eventRowsMutated": 0,
                "historyRewriteCount": 0
            },
            "pairs": terminal_details,
        });
        write_receipt(receipt_out, &receipt).await?;
        println!(
            "{}",
            json!({
                "outcome": "NOOP",
                "writes": 0,
                "insertedCount": 0,
                "skippedCount": pairs.len(),
                "planSha256": plan_sha,
                "receiptOut": receipt_out,
            })
        );
        return Ok(());
    }

    // Fresh preimage: the plan hash must match the recomputed observation
    // exactly (changed current tuples are conflict; the frozen plan is never
    // updated during apply — CTR-CIR-001/006).
    let (doc, recomputed_hash) = observe(w, pairs, pairs_file_sha).await?;
    if recomputed_hash != plan_sha {
        return Err(conflict(format!(
            "plan hash mismatch: provided {plan_sha}, recomputed {recomputed_hash} (state drifted since plan; re-run plan)"
        )));
    }
    let summary = doc.get("summary").cloned().unwrap_or(Value::Null);
    if summary
        .get("conflictCount")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        != 0
        || summary
            .get("ambiguityCount")
            .and_then(Value::as_i64)
            .unwrap_or(1)
            != 0
    {
        return Err(conflict(
            "plan has conflicts or ambiguities; nothing to apply",
        ));
    }

    let mut tx = w.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(REPAIR_LOCK)
        .execute(&mut *tx)
        .await?;

    // Fresh fail-closed re-verification INSIDE the committing transaction
    // (CTR-CIR-006: all fresh preconditions in the one transaction).
    let (doc_tx, _) = observe_tx(&mut tx, pairs, pairs_file_sha).await?;
    if digest(&doc_tx)? != plan_sha {
        return Err(conflict(
            "plan state drifted inside the committing transaction; rolled back",
        ));
    }
    let summary_tx = doc_tx.get("summary").cloned().unwrap_or(Value::Null);
    if summary_tx
        .get("conflictCount")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        != 0
        || summary_tx
            .get("ambiguityCount")
            .and_then(Value::as_i64)
            .unwrap_or(1)
            != 0
    {
        return Err(conflict(
            "fresh plan state has conflicts or ambiguities; rolled back",
        ));
    }

    // The sole business write: INSERT of lineage rows. No visit, instance,
    // or event row is touched by any statement of this command family.
    let mut inserted = 0usize;
    let mut skipped = 0usize;
    for p in pairs {
        let existing = existing_rows_for_sources(&mut *tx, &[p.source_principal_id]).await?;
        match existing.get(&p.source_principal_id) {
            Some(row) if row.matches(p) => {
                skipped += 1;
            }
            Some(row) => {
                return Err(conflict(format!(
                    "conflicting lineage row for source {}: existing successor={} canonical={:?}",
                    p.source_principal_id, row.successor_principal_id, row.canonical_agent_id
                )));
            }
            None => {
                // Lock both principals against concurrent disable while the
                // lineage row commits.
                let locked: Vec<Uuid> = sqlx::query_scalar(
                    "SELECT principal_id FROM principals
                     WHERE principal_id = ANY($1) FOR UPDATE",
                )
                .bind(&[p.source_principal_id, p.successor_principal_id][..])
                .fetch_all(&mut *tx)
                .await?;
                if locked.len() != 2 {
                    return Err(conflict(format!(
                        "principal vanished during apply for pair source={}",
                        p.source_principal_id
                    )));
                }
                sqlx::query(
                    "INSERT INTO workflow_identity_successor_lines
                       (source_principal_id, successor_principal_id, legacy_agent_id,
                        canonical_agent_id, classification, evidence, repair_reason)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)",
                )
                .bind(p.source_principal_id)
                .bind(p.successor_principal_id)
                .bind(&p.legacy_agent_id)
                .bind(&p.canonical_agent_id)
                .bind(&p.classification)
                .bind(&p.evidence)
                .bind(&p.repair_reason)
                .execute(&mut *tx)
                .await?;
                inserted += 1;
            }
        }
    }

    // Postcondition verification inside the same transaction (CTR-CIR-006).
    for p in pairs {
        let ok: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM workflow_identity_successor_lines
             WHERE source_principal_id = $1 AND successor_principal_id = $2
               AND legacy_agent_id = $3 AND canonical_agent_id = $4
               AND classification = $5",
        )
        .bind(p.source_principal_id)
        .bind(p.successor_principal_id)
        .bind(&p.legacy_agent_id)
        .bind(&p.canonical_agent_id)
        .bind(&p.classification)
        .fetch_one(&mut *tx)
        .await?;
        if ok != 1 {
            return Err(conflict(format!(
                "postcondition failed for source {}: planned lineage row not exactly present",
                p.source_principal_id
            )));
        }
    }

    if let Err(commit_error) = tx.commit().await {
        // Outcome-unknown protocol (CTR-CIR-007): never replay blindly;
        // probe the authoritative durable state instead (same shape as the
        // accepted cutover operator: exact terminal state after a commit
        // error is reported as COMMITTED with the probe as evidence).
        return match terminal_exact(w, pairs).await {
            Ok((true, terminal_details)) => {
                let receipt = json!({
                    "schema": RECEIPT_SCHEMA,
                    "commandFamily": COMMAND_FAMILY,
                    "mode": "apply",
                    "outcome": "COMMITTED",
                    "writes": inserted,
                    "insertedCount": inserted,
                    "skippedCount": skipped,
                    "planSha256": plan_sha,
                    "pairsFileSha256": pairs_file_sha,
                    "commitErrorNote": commit_error.to_string(),
                    "completedAtUtc": Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true),
                    "immutability": {
                        "visitRowsMutated": 0,
                        "instanceRowsMutated": 0,
                        "eventRowsMutated": 0,
                        "historyRewriteCount": 0
                    },
                    "pairs": terminal_details,
                });
                write_receipt(receipt_out, &receipt).await?;
                println!(
                    "{}",
                    json!({
                        "outcome": "COMMITTED",
                        "writes": inserted,
                        "insertedCount": inserted,
                        "skippedCount": skipped,
                        "planSha256": plan_sha,
                        "receiptOut": receipt_out,
                    })
                );
                Ok(())
            }
            Ok((false, _)) => Err(conflict(format!(
                "commit failed with no durable delta: {commit_error}"
            ))),
            Err(read_error) => Err(outcome_error(
                "OUTCOME_UNKNOWN",
                format!("commit error {commit_error}; terminal re-read failed: {read_error} - inspect the durable lineage state before any resubmit"),
            )),
        };
    }

    // Post-commit terminal re-read proves the committed state.
    let (terminal, terminal_details) = terminal_exact(w, pairs).await?;
    if !terminal {
        return Err(conflict(
            "committed but terminal re-read disagrees with the planned lineage",
        ));
    }

    let receipt = json!({
        "schema": RECEIPT_SCHEMA,
        "commandFamily": COMMAND_FAMILY,
        "mode": "apply",
        "outcome": "COMMITTED",
        "writes": inserted,
        "insertedCount": inserted,
        "skippedCount": skipped,
        "planSha256": plan_sha,
        "pairsFileSha256": pairs_file_sha,
        "unrelatedDigestAtPlan": doc.get("unrelatedDigest").cloned().unwrap_or(Value::Null),
        "affectedAssignmentCount": summary.get("affectedAssignmentCount").cloned().unwrap_or(Value::Null),
        "affectedActionableCount": summary.get("affectedActionableCount").cloned().unwrap_or(Value::Null),
        "completedAtUtc": Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true),
        "immutability": {
            "visitRowsMutated": 0,
            "instanceRowsMutated": 0,
            "eventRowsMutated": 0,
            "historyRewriteCount": 0
        },
        "pairs": terminal_details,
    });
    write_receipt(receipt_out, &receipt).await?;
    println!(
        "{}",
        json!({
            "outcome": "COMMITTED",
            "writes": inserted,
            "insertedCount": inserted,
            "skippedCount": skipped,
            "planSha256": plan_sha,
            "receiptOut": receipt_out,
        })
    );
    Ok(())
}

async fn verify(w: &PgPool, pairs: &[PairInput], plan_sha: &str) -> Result<()> {
    if plan_sha.len() != 64 || !plan_sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(conflict("--plan-sha256 must be a 64-char hex SHA-256"));
    }
    // Store-level resolution helper must agree with the SQL projection:
    // resolve_current_principal follows at most one lineage edge.
    for p in pairs {
        let resolved =
            svc_workflow::store::postgres::identity_successor::resolve_current_principal(
                w,
                p.source_principal_id,
            )
            .await?;
        if resolved != Some(p.successor_principal_id) {
            return Err(conflict(format!(
                "verify failed for source {}: resolve_current_principal returned {resolved:?}, expected {}",
                p.source_principal_id, p.successor_principal_id
            )));
        }
    }
    let (terminal, details) = terminal_exact(w, pairs).await?;
    if !terminal {
        return Err(conflict(
            "verify failed: lineage rows are missing or drifted, or the read-path projection does not resolve the canonical agent id (was the plan applied?)",
        ));
    }
    println!(
        "{}",
        json!({
            "outcome": "VERIFIED",
            "writes": 0,
            "planSha256": plan_sha,
            "pairs": details,
        })
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        let message = e.to_string();
        let outcome = message
            .split_once(':')
            .map(|(prefix, _)| prefix)
            .filter(|value| matches!(*value, "CONFLICT" | "ROLLED_BACK" | "OUTCOME_UNKNOWN"))
            .unwrap_or("CONFLICT");
        println!(
            "{}",
            json!({"outcome": outcome, "writes": 0, "error": message})
        );
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let mode = parse_args()?;
    let database_url =
        env::var("DATABASE_URL").map_err(|_| conflict("DATABASE_URL is required"))?;
    let workflow = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await?;
    if matches!(mode, Mode::MigrateOnly) {
        run_migrate(&workflow).await?;
        println!("{}", json!({"outcome": "MIGRATED", "writes": 0}));
        return Ok(());
    }
    let (pairs_path, migrate) = match &mode {
        Mode::Plan {
            pairs_path,
            migrate,
        } => (pairs_path, *migrate),
        Mode::Apply {
            pairs_path,
            migrate,
            ..
        } => (pairs_path, *migrate),
        Mode::Verify {
            pairs_path,
            migrate,
            ..
        } => (pairs_path, *migrate),
        Mode::MigrateOnly => unreachable!(),
    };
    if migrate {
        run_migrate(&workflow).await?;
    }
    let (pairs, pairs_file_sha) = load_pairs(pairs_path)?;
    match mode {
        Mode::Plan { .. } => {
            let (doc, plan_hash) = observe(&workflow, &pairs, &pairs_file_sha).await?;
            let conflicts = doc
                .get("summary")
                .and_then(|s| s.get("conflictCount"))
                .and_then(Value::as_i64)
                .unwrap_or(0);
            println!(
                "{}",
                json!({
                    "outcome": if conflicts == 0 { "PLANNED" } else { "CONFLICT" },
                    "writes": 0,
                    "planHash": plan_hash,
                    "plan": doc,
                })
            );
            if conflicts == 0 {
                Ok(())
            } else {
                Err(conflict(format!(
                    "{conflicts} pair(s) have fail-closed conflicts; nothing was written"
                )))
            }
        }
        Mode::Apply {
            plan_sha,
            receipt_out,
            ..
        } => apply(&workflow, &pairs, &pairs_file_sha, &plan_sha, &receipt_out).await,
        Mode::Verify { plan_sha, .. } => verify(&workflow, &pairs, &plan_sha).await,
        Mode::MigrateOnly => unreachable!(),
    }
}
