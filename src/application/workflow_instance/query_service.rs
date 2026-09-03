//! Read-only application service over the authoritative PostgreSQL projection and facts.

use sqlx::PgPool;

use super::query_types::*;
use crate::store::postgres::workflow_instance_repository::{
    query_detail, query_domain_instances, query_global_instances, query_worklists,
};

#[derive(Clone)]
pub struct WorkflowQueryService {
    pool: PgPool,
}

impl WorkflowQueryService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_workflow_instance_detail(
        &self,
        query: GetWorkflowInstanceDetail,
    ) -> Result<WorkflowInstanceDetail, WorkflowQueryError> {
        query_detail::get_workflow_instance_detail(&self.pool, query).await
    }

    pub async fn list_workflow_timeline(
        &self,
        query: ListWorkflowTimeline,
    ) -> Result<Page<WorkflowEventItem, i32>, WorkflowQueryError> {
        query_detail::list_workflow_timeline(&self.pool, query).await
    }

    pub async fn list_context_revisions(
        &self,
        query: ListContextRevisions,
    ) -> Result<Page<ContextRevisionItem, i32>, WorkflowQueryError> {
        query_detail::list_context_revisions(&self.pool, query).await
    }

    pub async fn list_node_visits(
        &self,
        query: ListNodeVisits,
    ) -> Result<Page<NodeVisitItem>, WorkflowQueryError> {
        query_detail::list_node_visits(&self.pool, query).await
    }

    pub async fn list_submission_history(
        &self,
        query: ListSubmissionHistory,
    ) -> Result<Page<SubmissionHistoryItem>, WorkflowQueryError> {
        query_detail::list_submission_history(&self.pool, query).await
    }

    pub async fn list_assigned_to_me(
        &self,
        query: ListAssignedToMe,
    ) -> Result<Page<AssignedWorkItem>, WorkflowQueryError> {
        query_worklists::list_assigned_to_me(&self.pool, query).await
    }

    pub async fn list_creator_owned_drafts(
        &self,
        query: ListCreatorOwnedDrafts,
    ) -> Result<Page<CreatorDraftItem>, WorkflowQueryError> {
        query_worklists::list_creator_owned_drafts(&self.pool, query).await
    }

    /// List all instances in a domain (authorized domain owners only).
    ///
    /// Performs DOMAIN_OWNER authorization check before querying. Returns
    /// `WorkflowQueryError::WorkflowInstanceNotFoundOrNotVisible` (mapped to
    /// 403 by the HTTP handler) when the actor is not a domain owner.
    pub async fn list_domain_instances(
        &self,
        query: ListDomainInstances,
    ) -> Result<Page<DomainInstanceSummary>, WorkflowQueryError> {
        use crate::store::postgres::workflow_instance_repository::query_visibility;

        let mut tx = query_visibility::begin_snapshot(&self.pool).await?;
        let is_owner = query_visibility::check_domain_owner(
            &mut tx,
            query.actor_principal_id,
            query.domain_id,
        )
        .await?;
        if !is_owner {
            tx.commit()
                .await
                .map_err(|e| WorkflowQueryError::StorageError(e.to_string()))?;
            return Err(WorkflowQueryError::WorkflowInstanceNotFoundOrNotVisible);
        }
        tx.commit()
            .await
            .map_err(|e| WorkflowQueryError::StorageError(e.to_string()))?;

        // Delegate to the repository for the actual query
        query_domain_instances::list_domain_instances(&self.pool, query).await
    }

    /// List instance summaries across ALL domains (global read roles only).
    ///
    /// Performs the global read-role authorization check before querying:
    /// an enabled `GLOBAL_WORKFLOW_READER` or `GLOBAL_WORKFLOW_COORDINATOR`
    /// binding satisfies the gate. Returns
    /// `WorkflowQueryError::GlobalCoordinatorRequired` (mapped to 403
    /// `global_read_role_required` by the HTTP handler) when the actor
    /// holds neither role. The projection is identical to
    /// `DomainInstanceSummary` — no detail / submission payload is ever
    /// exposed.
    /// List active due Dispatch Intents (VISIT_ACTIVATION_V1 scheduler read).
    ///
    /// The fail-closed `GLOBAL_SCHEDULER_READ` role check runs inside the
    /// same read snapshot as the query. The projection is the minimum
    /// Scheduler-facing record of v0.4.0 §5.7 — no workflow content is
    /// exposed.
    pub async fn list_due_dispatch_intents(
        &self,
        actor_principal_id: uuid::Uuid,
        limit: i64,
    ) -> Result<Vec<crate::store::postgres::workflow_instance_repository::query_dispatch_intents::DueDispatchIntent>, WorkflowQueryError>
    {
        crate::store::postgres::workflow_instance_repository::query_dispatch_intents::list_due_dispatch_intents(
            &self.pool,
            actor_principal_id,
            limit,
        )
        .await
    }

    pub async fn list_global_instances(
        &self,
        query: ListGlobalInstances,
    ) -> Result<Page<DomainInstanceSummary>, WorkflowQueryError> {
        use crate::store::postgres::workflow_instance_repository::query_visibility;

        let mut tx = query_visibility::begin_snapshot(&self.pool).await?;
        let has_read_role =
            query_visibility::check_global_workflow_read_role(&mut tx, query.actor_principal_id)
                .await?;
        if !has_read_role {
            tx.commit()
                .await
                .map_err(|e| WorkflowQueryError::StorageError(e.to_string()))?;
            return Err(WorkflowQueryError::GlobalCoordinatorRequired);
        }
        tx.commit()
            .await
            .map_err(|e| WorkflowQueryError::StorageError(e.to_string()))?;

        query_global_instances::list_global_instances(&self.pool, query).await
    }
}
