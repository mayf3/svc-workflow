//! Domain-wide instance listing for authorized domain owners.
//!
//! Returns a paginated, filtered projection of all instances within a single
//! domain. Authorization (DOMAIN_OWNER check) is handled by the caller at the
//! service layer — this module only does instance projection using the same
//! join semantics as `query_worklists`.

use uuid::Uuid;

use crate::application::workflow_instance::query_types::*;

use super::query_visibility::map_storage;

/// Hard page-size bounds shared with other worklist endpoints.
const DEFAULT_LIMIT: u32 = 20;
const MAX_LIMIT: u32 = 100;

fn parse_limit(limit: Option<u32>) -> Result<usize, WorkflowQueryError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    if limit == 0 || limit > MAX_LIMIT {
        return Err(WorkflowQueryError::InvalidPagination(format!(
            "limit must be between 1 and {MAX_LIMIT}"
        )));
    }
    Ok(limit as usize)
}

/// Map the lifecycle filter to a SQL WHERE clause fragment.
///
/// Semantics (same as existing worklist contract):
/// - `Active`   → node_type <> 'TERMINAL'
/// - `Terminal` → node_type = 'TERMINAL'
/// - `All`      → no filter
fn lifecycle_where(lifecycle: Option<LifecycleFilter>) -> &'static str {
    match lifecycle {
        Some(LifecycleFilter::Active) => " AND nd.node_type <> 'TERMINAL'",
        Some(LifecycleFilter::Terminal) => " AND nd.node_type = 'TERMINAL'",
        Some(LifecycleFilter::All) | None => "",
    }
}

#[derive(Debug, sqlx::FromRow)]
struct DomainInstanceRow {
    workflow_instance_id: Uuid,
    domain_id: Uuid,
    definition_version_id: Uuid,
    definition_key: String,
    created_by_principal_id: Uuid,
    current_assignee_principal_id: Option<Uuid>,
    node_id: Uuid,
    node_key: String,
    node_display_name: String,
    node_type: String,
    is_terminal: bool,
    title: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<DomainInstanceRow> for DomainInstanceSummary {
    fn from(row: DomainInstanceRow) -> Self {
        Self {
            workflow_instance_id: row.workflow_instance_id,
            domain_id: row.domain_id,
            definition_version_id: row.definition_version_id,
            definition_key: row.definition_key,
            created_by_principal_id: row.created_by_principal_id,
            current_assignee_principal_id: row.current_assignee_principal_id,
            current_node: PublicNodeSummary {
                node_id: row.node_id,
                node_key: row.node_key,
                display_name: row.node_display_name,
                node_type: row.node_type,
            },
            is_terminal: row.is_terminal,
            title: row.title,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

pub(crate) async fn list_domain_instances(
    pool: &sqlx::PgPool,
    query: ListDomainInstances,
) -> Result<Page<DomainInstanceSummary>, WorkflowQueryError> {
    let limit = parse_limit(query.limit)?;

    // Use a REPEATABLE READ snapshot for consistency — same isolation
    // level as other worklist queries.
    let mut tx = pool.begin().await.map_err(map_storage)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await
        .map_err(map_storage)?;

    // Build the SQL with a fixed parameter scheme:
    //   $1  domain_id
    //   $2  definition_key (or NULL to skip)
    //   $3  current_node_key (or NULL to skip)
    //   $4  assignee_principal_id (or NULL to skip)
    //   $5  cursor.created_at (or NULL)
    //   $6  cursor.id (or NULL)
    //
    // lifecycle is injected into the SQL string since it's an enum
    // with a small, fixed set of values. This avoids dynamic parameter
    // index shifting.

    let lifecycle_clause = lifecycle_where(query.lifecycle);

    let sql = format!(
        "SELECT wi.workflow_instance_id, wi.domain_id,
                wi.definition_version_id, wd.definition_key,
                wi.created_by_principal_id,
                v.assignee_principal_id AS current_assignee_principal_id,
                nd.node_id, nd.node_key,
                nd.display_name AS node_display_name,
                nd.node_type::text,
                (nd.node_type = 'TERMINAL') AS is_terminal,
                cr.payload->>'title' AS title,
                wi.created_at, wi.updated_at
         FROM workflow_instances wi
         JOIN workflow_definition_versions wdv
           ON wdv.definition_version_id = wi.definition_version_id
         JOIN workflow_definitions wd
           ON wd.workflow_definition_id = wdv.workflow_definition_id
         JOIN workflow_node_visits v
           ON v.node_visit_id = wi.current_node_visit_id
          AND v.workflow_instance_id = wi.workflow_instance_id
         JOIN workflow_node_definitions nd
           ON nd.node_id = v.node_id
          AND nd.definition_version_id = wi.definition_version_id
         LEFT JOIN workflow_context_revisions cr
           ON cr.context_revision_id = wi.current_context_revision_id
          AND cr.workflow_instance_id = wi.workflow_instance_id
         WHERE wi.domain_id = $1
           AND ($2::text IS NULL OR wd.definition_key = $2)
           {lifecycle_clause}
           AND ($3::text IS NULL OR nd.node_key = $3)
           AND ($4::uuid IS NULL OR v.assignee_principal_id = $4)
           AND ($5::timestamptz IS NULL OR (wi.created_at, wi.workflow_instance_id) < ($5, $6))
         ORDER BY wi.created_at DESC, wi.workflow_instance_id DESC
         LIMIT {}",
        (limit + 1) as i64
    );

    let rows = sqlx::query_as::<_, DomainInstanceRow>(&sql)
        .bind(query.domain_id)
        .bind(&query.definition_key)
        .bind(&query.current_node_key)
        .bind(query.assignee_principal_id)
        .bind(query.before.map(|c| c.created_at))
        .bind(query.before.map(|c| c.id))
        .fetch_all(&mut *tx)
        .await
        .map_err(map_storage)?;

    let has_more = rows.len() > limit;
    let selected: Vec<_> = rows.into_iter().take(limit).collect();
    let items: Vec<DomainInstanceSummary> = selected
        .into_iter()
        .map(DomainInstanceSummary::from)
        .collect();

    let next_cursor = has_more.then(|| {
        // SAFETY: selected is non-empty when has_more is true because the
        // API guarantees limit >= 1.
        let last = items.last().expect("non-empty page");
        TimeUuidCursor {
            created_at: last.created_at,
            id: last.workflow_instance_id,
        }
    });

    tx.commit().await.map_err(map_storage)?;
    Ok(Page { items, next_cursor })
}
