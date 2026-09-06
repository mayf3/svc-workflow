## Definition diagnostic compatibility (1.6.0)

Previously opaque canonical graph failures now return 422 graph_validation_failed.
The accepted diagnostic Spec freezes bounded static details and readable rule names.
New completed diagnostic receipts require the new safe decoder. A rollback artifact
must retain that decoder; no destructive receipt cleanup or automatic unknown-attempt
replay is allowed. Historical replacement receipt hash and success/error replay remain unchanged.
Only newly completed graph diagnostics store and check an additional full graph
input hash, so a corrected input using that failed attempt key conflicts. Existing
incomplete successful-write input binding remains outside this diagnostic scope.

# Compatibility Policy

**Policy:** `strict_backward_compatible`

## Rules

1. **No breaking changes to existing endpoints.** Every existing request that is valid today must remain valid and produce the same response structure.

2. **Wire format must be preserved:**
   - Field naming conventions (`camelCase` vs `snake_case`) are frozen per endpoint
   - `deny_unknown_fields` remains on all request DTOs
   - Error envelope structure (`{"error": {"code", "message", ...}}`) is stable

3. **Pagination contract is frozen:**
   - Worklist and domain list: composite `beforeCreatedAt` + `beforeId` cursor
   - Timeline: numeric `after` cursor
   - Response cursor field names (`next_cursor` / `nextCursor`) are per-endpoint
   - Default and maximum limits are fixed

4. **Authorization model must not regress:**
   - Domain isolation semantics are stable
   - Scope requirements (`workflow.read`, `workflow.execute`) are stable
   - DOMAIN_OWNER check for domain list is stable

5. **Error codes must not change meaning.** New error codes may be added but existing codes must remain stable.

6. **Idempotency contract must be preserved:**
   - `Idempotency-Key` header requirement is stable
   - `idempotency_conflict` response (409, no details) is stable

7. **Detail visibility 404 hiding is stable.**

## Out of Scope for Compatibility

- Control-plane admin endpoints (`/internal/v1/admin/**`)
- Internal data model changes that don't affect the HTTP contract
- Performance characteristics
