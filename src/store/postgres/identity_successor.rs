//! Canonical identity successor-lineage read surface (migration 0025).
//!
//! Governing authority:
//! `docs/specs/SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2.md`
//! (CTR-CIR-001/004/006/007). P9 historical identity repair maps STALE auth
//! naked-name Principals to their unique canonical successors through the
//! IMMUTABLE `workflow_identity_successor_lines` table so current resolution
//! paths uniquely yield the canonical Agent WITHOUT rewriting history.
//!
//! Hard boundary: this module is READ-ONLY. It never mutates
//! `workflow_node_visits`, `workflow_events`, or any historical fact, and it
//! creates no equivalence record between old and new Principals — the stale
//! Principal keeps its own history and gains no alias authority. Rows are
//! written exactly once by the offline `identity_repair_v1` operator; the
//! table is append-only at the database level.

use sqlx::PgExecutor;
use uuid::Uuid;

/// Resolve the current canonical Principal for `source_principal_id`.
///
/// Follows AT MOST ONE edge of `workflow_identity_successor_lines`:
///
/// * a lineage row exists            -> `Some(successor_principal_id)`
/// * no lineage row (the common case) -> the source itself is returned
/// * the source Principal is absent from the Workflow `principals` projection
///   entirely                         -> `None`
///
/// There is deliberately no second hop, transitive closure, fuzzy fallback,
/// or display-name aliasing: one immutable edge per stale source, enforced by
/// `UNIQUE (source_principal_id)`.
pub async fn resolve_current_principal<'e, E>(
    executor: E,
    source_principal_id: Uuid,
) -> Result<Option<Uuid>, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    let (source_exists, successor): (bool, Option<Uuid>) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM principals WHERE principal_id = $1),
                (SELECT successor_principal_id FROM workflow_identity_successor_lines
                  WHERE source_principal_id = $1)",
    )
    .bind(source_principal_id)
    .fetch_one(executor)
    .await?;
    if !source_exists {
        return Ok(None);
    }
    Ok(Some(successor.unwrap_or(source_principal_id)))
}
