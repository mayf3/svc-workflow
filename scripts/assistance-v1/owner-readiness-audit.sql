\set ON_ERROR_STOP on

-- Workflow Assistance V1 pre-launch gate.
-- A clean result set is required before enabling Assistance write routes.
-- Repair every returned domain through the existing atomic owner replacement API:
--   PUT /internal/v1/admin/domains/{domainId}/owner
-- or, for an authorized GLOBAL_WORKFLOW_COORDINATOR:
--   PUT /internal/v1/domains/{domainId}/owner
-- Never infer a replacement from DOMAIN_MEMBER bindings.

BEGIN TRANSACTION READ ONLY;

WITH effective_owners AS (
    SELECT
        d.domain_id,
        COUNT(b.binding_id) FILTER (WHERE b.enabled) AS enabled_owner_bindings,
        COUNT(b.binding_id) FILTER (
            WHERE b.enabled AND p.enabled
        ) AS effective_owner_count
    FROM domains d
    LEFT JOIN domain_role_bindings b
        ON b.domain_id = d.domain_id
       AND b.role_key = 'DOMAIN_OWNER'
    LEFT JOIN principals p
        ON p.principal_id = b.principal_id
    WHERE d.enabled = TRUE
    GROUP BY d.domain_id
),
active_instances AS (
    SELECT
        wi.domain_id,
        COUNT(*) AS active_instance_count
    FROM workflow_instances wi
    JOIN workflow_node_visits nv
        ON nv.node_visit_id = wi.current_node_visit_id
    JOIN workflow_node_definitions nd
        ON nd.node_id = nv.node_id
    -- Deliberately use only the original runtime columns so this audit can
    -- also diagnose a pre-0015 database before migration. This is conservative:
    -- a cancelled legacy row may be counted as active, never hidden.
    WHERE nd.node_type <> 'TERMINAL'
    GROUP BY wi.domain_id
)
SELECT
    d.domain_id,
    d.domain_key,
    eo.enabled_owner_bindings,
    eo.effective_owner_count,
    COALESCE(ai.active_instance_count, 0) AS active_instance_count
FROM effective_owners eo
JOIN domains d ON d.domain_id = eo.domain_id
LEFT JOIN active_instances ai ON ai.domain_id = eo.domain_id
WHERE eo.effective_owner_count <> 1
ORDER BY COALESCE(ai.active_instance_count, 0) DESC, d.domain_key;

COMMIT;
