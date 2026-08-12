//! Caller-scoped Assistance V1 inbox and detail projections.

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistanceNodeSummary {
    pub node_id: Uuid,
    pub node_key: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistanceCaseView {
    pub assistance_case_id: Uuid,
    pub workflow_instance_id: Uuid,
    pub node_visit_id: Uuid,
    pub status: AssistanceCaseStatus,
    pub domain_id: Uuid,
    pub definition_key: String,
    pub node: AssistanceNodeSummary,
    pub requested_by_principal_id: Uuid,
    pub request: AssistancePayload,
    pub escalated_by_principal_id: Option<Uuid>,
    pub escalation: Option<AssistancePayload>,
    pub resolved_by_principal_id: Option<Uuid>,
    pub resolution: Option<AssistancePayload>,
    pub workflow_state_version: i32,
    pub current_node_visit_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub escalated_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub voided_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct AssistanceViewRow {
    assistance_case_id: Uuid,
    workflow_instance_id: Uuid,
    node_visit_id: Uuid,
    status: String,
    domain_id: Uuid,
    definition_key: String,
    node_id: Uuid,
    node_key: String,
    display_name: String,
    requested_by_principal_id: Uuid,
    request_payload: serde_json::Value,
    escalated_by_principal_id: Option<Uuid>,
    escalation_payload: Option<serde_json::Value>,
    resolved_by_principal_id: Option<Uuid>,
    resolution_payload: Option<serde_json::Value>,
    workflow_state_version: i32,
    current_node_visit_id: Uuid,
    created_at: DateTime<Utc>,
    escalated_at: Option<DateTime<Utc>>,
    resolved_at: Option<DateTime<Utc>>,
    voided_at: Option<DateTime<Utc>>,
}

impl TryFrom<AssistanceViewRow> for AssistanceCaseView {
    type Error = AssistanceError;
    fn try_from(row: AssistanceViewRow) -> Result<Self, Self::Error> {
        Ok(Self {
            assistance_case_id: row.assistance_case_id,
            workflow_instance_id: row.workflow_instance_id,
            node_visit_id: row.node_visit_id,
            status: AssistanceCaseStatus::parse(&row.status)?,
            domain_id: row.domain_id,
            definition_key: row.definition_key,
            node: AssistanceNodeSummary {
                node_id: row.node_id,
                node_key: row.node_key,
                display_name: row.display_name,
            },
            requested_by_principal_id: row.requested_by_principal_id,
            request: serde_json::from_value(row.request_payload)
                .map_err(|error| AssistanceError::InternalConsistency(error.to_string()))?,
            escalated_by_principal_id: row.escalated_by_principal_id,
            escalation: row
                .escalation_payload
                .map(serde_json::from_value)
                .transpose()
                .map_err(|error| AssistanceError::InternalConsistency(error.to_string()))?,
            resolved_by_principal_id: row.resolved_by_principal_id,
            resolution: row
                .resolution_payload
                .map(serde_json::from_value)
                .transpose()
                .map_err(|error| AssistanceError::InternalConsistency(error.to_string()))?,
            workflow_state_version: row.workflow_state_version,
            current_node_visit_id: row.current_node_visit_id,
            created_at: row.created_at,
            escalated_at: row.escalated_at,
            resolved_at: row.resolved_at,
            voided_at: row.voided_at,
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistanceCursor {
    pub at: DateTime<Utc>,
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistancePage {
    pub items: Vec<AssistanceCaseView>,
    pub next_cursor: Option<AssistanceCursor>,
}

/// Deliberately minimal cross-Domain projection for the coordinator inbox.
/// It must not grow workflow context, submission history, resolution data, or
/// any field that could be used to perform a workflow write.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HumanRequiredAssistanceCaseView {
    pub assistance_case_id: Uuid,
    pub status: AssistanceCaseStatus,
    pub created_at: DateTime<Utc>,
    pub escalated_at: DateTime<Utc>,
    pub domain_id: Uuid,
    pub workflow_instance_id: Uuid,
    pub definition_key: String,
    pub node: AssistanceNodeSummary,
    pub requested_by_principal_id: Uuid,
    pub request: AssistancePayload,
    pub escalation: AssistancePayload,
}

impl TryFrom<AssistanceCaseView> for HumanRequiredAssistanceCaseView {
    type Error = AssistanceError;

    fn try_from(value: AssistanceCaseView) -> Result<Self, Self::Error> {
        if value.status != AssistanceCaseStatus::HumanRequired {
            return Err(AssistanceError::InternalConsistency(
                "coordinator projection received a non-HUMAN_REQUIRED case".to_string(),
            ));
        }
        Ok(Self {
            assistance_case_id: value.assistance_case_id,
            status: value.status,
            created_at: value.created_at,
            escalated_at: value.escalated_at.ok_or_else(|| {
                AssistanceError::InternalConsistency(
                    "HUMAN_REQUIRED case has no escalated_at".to_string(),
                )
            })?,
            domain_id: value.domain_id,
            workflow_instance_id: value.workflow_instance_id,
            definition_key: value.definition_key,
            node: value.node,
            requested_by_principal_id: value.requested_by_principal_id,
            request: value.request,
            escalation: value.escalation.ok_or_else(|| {
                AssistanceError::InternalConsistency(
                    "HUMAN_REQUIRED case has no escalation payload".to_string(),
                )
            })?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HumanRequiredAssistancePage {
    pub items: Vec<HumanRequiredAssistanceCaseView>,
    pub next_cursor: Option<AssistanceCursor>,
}

#[derive(Debug, Clone, Copy)]
pub enum AssistanceListView {
    OwnerInbox,
    RequestedByMe,
}

async fn actor_enabled(pool: &PgPool, actor: Uuid) -> Result<(), AssistanceError> {
    match sqlx::query_scalar::<_, bool>("SELECT enabled FROM principals WHERE principal_id=$1")
        .bind(actor)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
    {
        None => Err(AssistanceError::PrincipalNotFound),
        Some(false) => Err(AssistanceError::PrincipalDisabled),
        Some(true) => Ok(()),
    }
}

const VIEW_SELECT: &str =
    "SELECT ac.assistance_case_id, ac.workflow_instance_id, ac.node_visit_id, ac.status,
            wi.domain_id, wd.definition_key, nd.node_id, nd.node_key, nd.display_name,
            ac.requested_by_principal_id, ac.request_payload,
            ac.escalated_by_principal_id, ac.escalation_payload,
            ac.resolved_by_principal_id, ac.resolution_payload,
            wi.workflow_state_version, wi.current_node_visit_id,
            ac.created_at, ac.escalated_at, ac.resolved_at, ac.voided_at
     FROM workflow_assistance_cases ac
     JOIN workflow_instances wi ON wi.workflow_instance_id=ac.workflow_instance_id
     JOIN workflow_definition_versions wdv ON wdv.definition_version_id=wi.definition_version_id
     JOIN workflow_definitions wd ON wd.workflow_definition_id=wdv.workflow_definition_id
     JOIN workflow_node_visits nv ON nv.node_visit_id=ac.node_visit_id
     JOIN workflow_node_definitions nd ON nd.node_id=nv.node_id";

pub(crate) async fn list_assistance(
    pool: &PgPool,
    actor: Uuid,
    view: AssistanceListView,
    status: Option<AssistanceCaseStatus>,
    before: Option<AssistanceCursor>,
    limit: u32,
) -> Result<AssistancePage, AssistanceError> {
    actor_enabled(pool, actor).await?;
    if !(1..=100).contains(&limit) {
        return Err(AssistanceError::InvalidPagination(
            "limit must be between 1 and 100".to_string(),
        ));
    }
    let (predicate, order_column) = match view {
        AssistanceListView::OwnerInbox => (
            "ac.status IN ('OWNER_PENDING','HUMAN_REQUIRED')
             AND EXISTS(SELECT 1 FROM domain_role_bindings b
               JOIN domains d ON d.domain_id=b.domain_id AND d.enabled=TRUE
               JOIN principals p ON p.principal_id=b.principal_id AND p.enabled=TRUE
               WHERE b.domain_id=wi.domain_id AND b.principal_id=$1
                 AND b.role_key='DOMAIN_OWNER' AND b.enabled=TRUE)
             AND $2::text IS NULL",
            "ac.created_at",
        ),
        AssistanceListView::RequestedByMe => (
            "ac.requested_by_principal_id=$1 AND ($2::text IS NULL OR ac.status=$2)",
            "ac.created_at",
        ),
    };
    let sql = format!(
        "{VIEW_SELECT} WHERE {predicate}
         AND ($3::timestamptz IS NULL OR ({order_column}, ac.assistance_case_id) < ($3,$4))
         ORDER BY {order_column} DESC, ac.assistance_case_id DESC LIMIT $5"
    );
    let rows = sqlx::query_as::<_, AssistanceViewRow>(&sql)
        .bind(actor)
        .bind(status.as_ref().map(AssistanceCaseStatus::as_str))
        .bind(before.map(|cursor| cursor.at))
        .bind(before.map(|cursor| cursor.id))
        .bind(i64::from(limit) + 1)
        .fetch_all(pool)
        .await
        .map_err(storage)?;
    let has_more = rows.len() > limit as usize;
    let items: Vec<AssistanceCaseView> = rows
        .into_iter()
        .take(limit as usize)
        .map(AssistanceCaseView::try_from)
        .collect::<Result<_, _>>()?;
    let next_cursor = if has_more {
        items.last().map(|item| AssistanceCursor {
            at: item.created_at,
            id: item.assistance_case_id,
        })
    } else {
        None
    };
    Ok(AssistancePage { items, next_cursor })
}

async fn require_global_coordinator(pool: &PgPool, actor: Uuid) -> Result<(), AssistanceError> {
    actor_enabled(pool, actor).await?;
    let coordinator: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM global_role_bindings
         WHERE principal_id=$1 AND role_key='GLOBAL_WORKFLOW_COORDINATOR' AND enabled=TRUE)",
    )
    .bind(actor)
    .fetch_one(pool)
    .await
    .map_err(storage)?;
    if coordinator {
        Ok(())
    } else {
        Err(AssistanceError::GlobalCoordinatorRequired)
    }
}

pub(crate) async fn list_human_required_assistance(
    pool: &PgPool,
    actor: Uuid,
    before: Option<AssistanceCursor>,
    limit: u32,
) -> Result<HumanRequiredAssistancePage, AssistanceError> {
    require_global_coordinator(pool, actor).await?;
    if !(1..=100).contains(&limit) {
        return Err(AssistanceError::InvalidPagination(
            "limit must be between 1 and 100".to_string(),
        ));
    }
    let sql = format!(
        "{VIEW_SELECT}
         WHERE ac.status='HUMAN_REQUIRED'
           AND ($1::timestamptz IS NULL OR (ac.escalated_at, ac.assistance_case_id) < ($1,$2))
         ORDER BY ac.escalated_at DESC, ac.assistance_case_id DESC LIMIT $3"
    );
    let rows = sqlx::query_as::<_, AssistanceViewRow>(&sql)
        .bind(before.map(|cursor| cursor.at))
        .bind(before.map(|cursor| cursor.id))
        .bind(i64::from(limit) + 1)
        .fetch_all(pool)
        .await
        .map_err(storage)?;
    let has_more = rows.len() > limit as usize;
    let items: Vec<HumanRequiredAssistanceCaseView> = rows
        .into_iter()
        .take(limit as usize)
        .map(AssistanceCaseView::try_from)
        .map(|view| view.and_then(HumanRequiredAssistanceCaseView::try_from))
        .collect::<Result<_, _>>()?;
    let next_cursor = if has_more {
        items.last().map(|item| AssistanceCursor {
            at: item.escalated_at,
            id: item.assistance_case_id,
        })
    } else {
        None
    };
    Ok(HumanRequiredAssistancePage { items, next_cursor })
}

pub(crate) async fn get_assistance_case(
    pool: &PgPool,
    actor: Uuid,
    case_id: Uuid,
) -> Result<AssistanceCaseView, AssistanceError> {
    actor_enabled(pool, actor).await?;
    let sql = format!(
        "{VIEW_SELECT}
         WHERE ac.assistance_case_id=$2 AND (
           ac.requested_by_principal_id=$1
           OR EXISTS(SELECT 1 FROM domain_role_bindings b
             JOIN domains d ON d.domain_id=b.domain_id AND d.enabled=TRUE
             JOIN principals p ON p.principal_id=b.principal_id AND p.enabled=TRUE
             WHERE b.domain_id=wi.domain_id AND b.principal_id=$1
               AND b.role_key='DOMAIN_OWNER' AND b.enabled=TRUE))"
    );
    let row = sqlx::query_as::<_, AssistanceViewRow>(&sql)
        .bind(actor)
        .bind(case_id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or(AssistanceError::AssistanceCaseNotFoundOrNotVisible)?;
    AssistanceCaseView::try_from(row)
}

pub(crate) async fn get_human_required_assistance_case(
    pool: &PgPool,
    actor: Uuid,
    case_id: Uuid,
) -> Result<HumanRequiredAssistanceCaseView, AssistanceError> {
    require_global_coordinator(pool, actor).await?;
    let sql = format!(
        "{VIEW_SELECT}
         WHERE ac.assistance_case_id=$1 AND ac.status='HUMAN_REQUIRED'"
    );
    let row = sqlx::query_as::<_, AssistanceViewRow>(&sql)
        .bind(case_id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or(AssistanceError::AssistanceCaseNotFoundOrNotVisible)?;
    HumanRequiredAssistanceCaseView::try_from(AssistanceCaseView::try_from(row)?)
}
