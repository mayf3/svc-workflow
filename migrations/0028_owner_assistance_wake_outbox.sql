-- DOMAIN_OWNER_ASSISTANCE_V1
-- Extend the durable workflow outbox with the Domain Owner wake fact.
-- Existing rows remain valid; no business table semantics change.

ALTER TABLE workflow_outbox
    DROP CONSTRAINT IF EXISTS workflow_outbox_outbox_kind_check;

ALTER TABLE workflow_outbox
    ADD CONSTRAINT workflow_outbox_outbox_kind_check
    CHECK (outbox_kind IN ('FORUM_EVENT', 'EXECUTION_KICK', 'OWNER_ASSISTANCE_WAKE'));
