-- ============================================================
-- Global Role Bindings
--
-- Domain-independent (cross-domain) role bindings. The first and
-- only supported role is GLOBAL_WORKFLOW_COORDINATOR: a read-only
-- business role that may list workflow instance summaries across
-- ALL domains. It deliberately grants no provisioning / transition /
-- cancel / archive powers — those remain domain-owner or admin gated.
--
-- The role_key is free text at the schema level (mirroring
-- domain_role_bindings); supported values are validated at the API
-- layer.
-- ============================================================

CREATE TABLE global_role_bindings (
    binding_id      UUID        NOT NULL PRIMARY KEY,
    principal_id    UUID        NOT NULL REFERENCES principals(principal_id),
    role_key        TEXT        NOT NULL CHECK (char_length(role_key) >= 1 AND char_length(role_key) <= 128),
    enabled         BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    disabled_at     TIMESTAMPTZ
);

-- Constraint: each principal can hold at most one binding per role.
CREATE UNIQUE INDEX idx_grb_principal_role
    ON global_role_bindings (principal_id, role_key);

CREATE INDEX idx_grb_principal ON global_role_bindings (principal_id);
