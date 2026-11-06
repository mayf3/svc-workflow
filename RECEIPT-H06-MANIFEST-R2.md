# RECEIPT-H06-MANIFEST-R2

Recorded at `2026-11-05T21:55:42.085917+00:00`.

**TIER_1 = PASS; TIER_2 = PENDING-DOCKER; MERGE_READY = NO.**
This is a mechanical source candidate, not full database replay conformance,
maintainer acceptance, merge, installation, production admission or business acceptance.

## Exact coordinates and mandate

- Repository: `mayf3/svc-workflow`.
- Base SHA: `59daec042786f0416a6a9061c0b638ba56c0d83e` (fresh remote main readback before preparation matched).
- Tested source commit: `6e1b6862c31558f17cdd02de75d5b780f34be4f2`.
- Branch: `ac-h06-migration-manifest`.
- Isolated worktree: `/private/tmp/svc-h06-manifest`.
- Attributable mandate: user H06 R2 on 2026-11-06, persisted in
  `BRIEF-H06-MANIFEST-R2.md`, inheriting `BRIEF-H06-MANIFEST.md`.
- R1 stop record remains unchanged history; R2 changes only its gates as instructed.
- Governance/adoption and mechanical routing: `PREFLIGHT-H06-MANIFEST-R2.md`.
  The cutover child Spec remains proposed/none. No new Product Authority is claimed.
- All Cargo commands used `CARGO_TARGET_DIR=/private/tmp/svc-h06-manifest/target`; variables containing
  `DATABASE_URL` and `MIGRATION_ACTOR_PRINCIPAL_ID` were removed from child environments.
  No substitute database, runner, plan artifact or runtime configuration was introduced.

## Change and exact-byte proof

One shared `migration_manifest.rs` holds the fixed plan SHA256 and all original
86 tuples. The operator and replay import those same constants under their old
local names. Two immutable text snapshots preserve each base initializer's exact
14,773 bytes, without a terminal LF. They are test evidence, never runtime paths.
The new test binds their raw hashes and the manifest's 344 ordered UTF-8 fields,
with one NUL after every field, then compares each field byte-for-byte to each
historical initializer. There is no sorting, normalization or identity inference.

Executed comparison additionally proves both consumer bodies are byte-identical
to base after removing only the old declarations and new imports. Module wiring
adds only `#[doc(hidden)] pub mod migration_manifest` for the separate binary
crate; the shared module has only two runtime constants, with checks under cfg(test).
No dynamic query, SQL, control flow, CLI behavior, error or historical mapping changed.

Raw pre/post comparison output (rc0):

```text
src/bin/trusted_fleet_principal_cutover_v1.rs
groups=86 strings=344 initializer_bytes=14773 initializer_sha256=689c316408c22b206f4de62f7c491b1e9970e06c467880b8e744706c09ca36e6
ordered_utf8_nul_terminated_bytes=9868 ordered_utf8_nul_terminated_sha256=3177906d54ad75f499a58de7621efd1ccd81b4e5d923f5a27de78d8136ca850e
CONSUMER_BODY_BYTES_UNCHANGED_AFTER_CONST_IMPORT_SUBSTITUTION=YES
src/store/postgres/admin_recovery_repository/event_replay.rs
groups=86 strings=344 initializer_bytes=14773 initializer_sha256=689c316408c22b206f4de62f7c491b1e9970e06c467880b8e744706c09ca36e6
ordered_utf8_nul_terminated_bytes=9868 ordered_utf8_nul_terminated_sha256=3177906d54ad75f499a58de7621efd1ccd81b4e5d923f5a27de78d8136ca850e
CONSUMER_BODY_BYTES_UNCHANGED_AFTER_CONST_IMPORT_SUBSTITUTION=YES
PRE_EXTRACTION_INITIALIZER_BYTES_EQUAL=YES
SHARED_INITIALIZER_EQUALS_BOTH_BASE_INITIALIZERS=YES
BOTH_TRACKED_SNAPSHOTS_EQUAL_EXACT_BASE_INITIALIZER_BYTES=YES
FROZEN_FIXTURE_EQUALS_BASE_BLOB=YES
FROZEN_FIXTURE_SHA256=0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606
BYTE_PROOF_RC=0
```

Tested source SHA256 bindings:

```text
aedc7cbfbc16ae3f2541f0ef474d359d97c795539f71cf24075d53cef13e1538  src/bin/trusted_fleet_principal_cutover_v1.rs
7df1474fd6cb22e11a5cc6b9b83baafa2501d21609549509c49a7f4e37b2d97b  src/store/postgres/admin_recovery_repository/event_replay.rs
474f758a52e6f4abf40fb20b47358865fd4a95278dce8ef5d34abe8667f8a088  src/store/postgres/admin_recovery_repository/mod.rs
58b5cf28836af4b0ff269d5ab81ec414bbe061148ba32bdc323d6c909019ecb1  src/store/postgres/admin_recovery_repository/migration_manifest.rs
689c316408c22b206f4de62f7c491b1e9970e06c467880b8e744706c09ca36e6  tests/fixtures/h06-manifest/cutover-inline.txt
689c316408c22b206f4de62f7c491b1e9970e06c467880b8e744706c09ca36e6  tests/fixtures/h06-manifest/event-replay-inline.txt
```

Source change diff stat, base to tested source commit:

```text
 src/bin/trusted_fleet_principal_cutover_v1.rs      | 522 +------------------
 .../admin_recovery_repository/event_replay.rs      | 521 +------------------
 .../migration_manifest.rs                          | 578 +++++++++++++++++++++
 .../postgres/admin_recovery_repository/mod.rs      |   2 +
 tests/fixtures/h06-manifest/cutover-inline.txt     | 518 ++++++++++++++++++
 .../fixtures/h06-manifest/event-replay-inline.txt  | 518 ++++++++++++++++++
 6 files changed, 1620 insertions(+), 1039 deletions(-)
```

## Tier-1 executed gates — all rc0

Only the audited non-DB tests were selected. No ignored or filtered test is
counted as a pass. There are six cargo test invocations and one cargo build gate;
the library filters intentionally overlap. All five distinct selected tests
executed, with zero failures and zero ignored tests.

The 44 DB-backed admin_recovery integration tests were NOT_RUN, not silently
skipped and not counted as green. Their shared helper can connect to a default
local DB and apply migrations. The DB-backed cutover test was also NOT_RUN.
Actual DB replay/rebuild and cutover result invariance remain UNKNOWN in this
lane; byte identity and the existing pure replay test are the bounded evidence.
The full cutover runner is explicitly Tier-2 below.

The following output is the complete raw combined stdout/stderr of each gate,
including compiler warnings. No output lines are omitted.

### Gate 1

Command: `cargo build`

Started: `2026-11-05T21:52:49.683947+00:00`; ended: `2026-11-05T21:53:27.409714+00:00`; **rc=0**.

```text
   Compiling proc-macro2 v1.0.106
   Compiling unicode-ident v1.0.24
   Compiling quote v1.0.46
   Compiling libc v0.2.186
   Compiling serde v1.0.228
   Compiling cfg-if v1.0.4
   Compiling serde_core v1.0.228
   Compiling pin-project-lite v0.2.17
   Compiling memchr v2.8.3
   Compiling parking_lot_core v0.9.12
   Compiling once_cell v1.21.4
   Compiling futures-core v0.3.32
   Compiling stable_deref_trait v1.2.1
   Compiling itoa v1.0.18
   Compiling bytes v1.12.1
   Compiling autocfg v1.5.1
   Compiling scopeguard v1.2.0
   Compiling version_check v0.9.5
   Compiling lock_api v0.4.14
   Compiling writeable v0.6.3
   Compiling litemap v0.8.2
   Compiling utf8_iter v1.0.4
   Compiling icu_normalizer_data v2.2.0
   Compiling equivalent v1.0.2
   Compiling shlex v2.0.1
   Compiling percent-encoding v2.3.2
   Compiling allocator-api2 v0.2.21
   Compiling subtle v2.6.1
   Compiling find-msvc-tools v0.1.9
   Compiling icu_properties_data v2.2.0
   Compiling zmij v1.0.22
   Compiling log v0.4.33
   Compiling num-traits v0.2.19
   Compiling generic-array v0.14.7
   Compiling tracing-core v0.1.36
   Compiling futures-task v0.3.32
   Compiling futures-io v0.3.32
   Compiling core-foundation-sys v0.8.7
   Compiling futures-sink v0.3.32
   Compiling typenum v1.20.1
   Compiling pkg-config v0.3.33
   Compiling slab v0.4.12
   Compiling foldhash v0.2.0
   Compiling fs_extra v1.3.0
   Compiling dunce v1.0.5
   Compiling hashbrown v0.17.1
   Compiling serde_json v1.0.150
   Compiling zerocopy v0.8.54
   Compiling http v1.4.2
   Compiling crossbeam-utils v0.8.22
   Compiling indexmap v2.14.0
   Compiling block-buffer v0.10.4
   Compiling crypto-common v0.1.7
   Compiling digest v0.10.7
   Compiling futures-util v0.3.32
   Compiling base64 v0.22.1
   Compiling thiserror v2.0.18
   Compiling bitflags v2.13.0
   Compiling zeroize v1.9.0
   Compiling aws-lc-rs v1.17.1
   Compiling http-body v1.0.1
   Compiling form_urlencoded v1.2.2
   Compiling untrusted v0.9.0
   Compiling getrandom v0.4.3
   Compiling rustls-pki-types v1.15.0
   Compiling iana-time-zone v0.1.65
   Compiling futures-channel v0.3.32
   Compiling foldhash v0.1.5
   Compiling httparse v1.10.1
   Compiling tower-service v0.3.3
   Compiling hashbrown v0.15.5
   Compiling atomic-waker v1.1.2
   Compiling syn v2.0.118
   Compiling fnv v1.0.7
   Compiling parking v2.2.1
   Compiling try-lock v0.2.5
   Compiling tinyvec_macros v0.1.1
   Compiling rustls v0.23.41
   Compiling crc-catalog v2.5.0
   Compiling crc v3.4.0
   Compiling num-integer v0.1.46
   Compiling tinyvec v1.12.0
   Compiling want v0.3.1
   Compiling hashlink v0.10.0
   Compiling sync_wrapper v1.0.2
   Compiling httpdate v1.0.3
   Compiling regex-syntax v0.8.11
   Compiling tower-layer v0.3.3
   Compiling unicode-normalization v0.1.25
   Compiling num-bigint v0.4.8
   Compiling mio v1.2.1
   Compiling synstructure v0.13.2
   Compiling socket2 v0.6.4
   Compiling jobserver v0.1.35
   Compiling errno v0.3.14
   Compiling cc v1.2.67
   Compiling getrandom v0.2.17
   Compiling signal-hook-registry v1.4.8
   Compiling cpufeatures v0.2.17
   Compiling sha2 v0.10.9
   Compiling rand_core v0.6.4
   Compiling tokio v1.52.3
   Compiling http-body-util v0.1.3
   Compiling hmac v0.12.1
   Compiling serde_derive v1.0.228
   Compiling zerofrom-derive v0.1.7
   Compiling yoke-derive v0.8.2
   Compiling zerovec-derive v0.11.3
   Compiling displaydoc v0.2.6
   Compiling tracing-attributes v0.1.31
   Compiling tokio-macros v2.7.0
   Compiling cmake v0.1.58
   Compiling ring v0.17.14
   Compiling zerofrom v0.1.8
   Compiling aws-lc-sys v0.42.0
   Compiling thiserror-impl v2.0.18
   Compiling tracing v0.1.44
   Compiling aho-corasick v1.1.4
   Compiling concurrent-queue v2.5.0
   Compiling event-listener v5.4.1
   Compiling crossbeam-queue v0.3.13
   Compiling uuid v1.23.5
   Compiling unicode-bidi v0.3.18
   Compiling unicode-properties v0.1.4
   Compiling ppv-lite86 v0.2.21
   Compiling ipnet v2.12.0
   Compiling hex v0.4.3
   Compiling rand_chacha v0.3.1
   Compiling stringprep v0.1.5
   Compiling rand v0.8.7
   Compiling regex-automata v0.4.15
   Compiling tokio-stream v0.1.18
   Compiling hkdf v0.12.4
   Compiling md-5 v0.10.6
   Compiling byteorder v1.5.0
   Compiling getrandom v0.3.4
   Compiling whoami v1.6.1
   Compiling ref-cast v1.0.25
   Compiling dotenvy v0.15.7
   Compiling home v0.5.12
   Compiling chrono v0.4.45
   Compiling atoi v2.0.0
   Compiling ref-cast-impl v1.0.25
   Compiling security-framework-sys v2.17.0
   Compiling core-foundation v0.10.1
   Compiling yoke v0.8.3
   Compiling webpki-roots v1.0.8
   Compiling ahash v0.8.12
   Compiling time-core v0.1.9
   Compiling num-conv v0.2.2
   Compiling lazy_static v1.5.0
   Compiling security-framework v3.7.0
   Compiling time-macros v0.2.31
   Compiling num-rational v0.4.2
   Compiling num-iter v0.1.46
   Compiling num-complex v0.4.6
   Compiling bit-vec v0.8.0
   Compiling heck v0.5.0
   Compiling borrow-or-share v0.2.4
   Compiling ryu v1.0.23
   Compiling anyhow v1.0.103
   Compiling powerfmt v0.2.0
   Compiling deranged v0.5.8
   Compiling unicode-general-category v1.1.0
   Compiling bit-set v0.8.0
   Compiling num v0.4.3
   Compiling ryu-js v0.2.2
   Compiling vsimd v0.8.0
   Compiling micromap v0.3.0
   Compiling mime v0.3.17
   Compiling outref v0.5.2
   Compiling uuid-simd v0.8.0
   Compiling time v0.3.53
   Compiling axum-core v0.5.6
   Compiling fancy-regex v0.18.0
   Compiling fraction v0.15.4
   Compiling sharded-slab v0.1.7
   Compiling smallvec v1.15.2
   Compiling either v1.16.0
   Compiling parking_lot v0.12.5
   Compiling serde_urlencoded v0.7.1
   Compiling email_address v0.2.9
   Compiling futures-intrusive v0.5.0
   Compiling serde_jcs v0.2.0
   Compiling matchers v0.2.0
   Compiling regex v1.13.0
   Compiling serde_path_to_error v0.1.20
   Compiling axum-macros v0.5.1
   Compiling jsonschema-regex v0.47.0
   Compiling pem v3.0.6
   Compiling tracing-log v0.2.0
   Compiling thread_local v1.1.10
   Compiling data-encoding v2.11.0
   Compiling svc-workflow v0.1.0 (/private/tmp/svc-h06-manifest)
   Compiling matchit v0.8.4
   Compiling nu-ansi-term v0.50.3
   Compiling bytecount v0.6.9
   Compiling num-cmp v0.1.0
   Compiling tracing-subscriber v0.3.23
   Compiling zerovec v0.11.6
   Compiling zerotrie v0.2.4
   Compiling jcs-canonicalize v0.2.1
   Compiling simple_asn1 v0.6.4
   Compiling jsonwebtoken v9.3.1
   Compiling fluent-uri v0.4.1
   Compiling referencing v0.47.0
   Compiling tokio-util v0.7.18
   Compiling tower v0.5.3
   Compiling h2 v0.4.15
   Compiling tinystr v0.8.3
   Compiling potential_utf v0.1.5
   Compiling icu_collections v2.2.0
   Compiling icu_locale_core v2.2.0
   Compiling icu_provider v2.2.0
   Compiling icu_properties v2.2.0
   Compiling icu_normalizer v2.2.0
   Compiling idna_adapter v1.2.2
   Compiling idna v1.1.0
   Compiling url v2.5.8
   Compiling hyper v1.10.1
   Compiling tower-http v0.6.11
   Compiling sqlx-core v0.8.6
   Compiling hyper-util v0.1.20
   Compiling sqlx-postgres v0.8.6
   Compiling axum v0.8.9
   Compiling sqlx-macros-core v0.8.6
   Compiling sqlx-macros v0.8.6
   Compiling sqlx v0.8.6
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling rustls-platform-verifier v0.7.0
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.13.4
   Compiling reqwest v0.12.28
   Compiling jsonschema v0.47.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 37.67s
```

### Gate 2

Command: `cargo test --lib event_replay -- --nocapture`

Started: `2026-11-05T21:53:27.410250+00:00`; ended: `2026-11-05T21:53:43.176625+00:00`; **rc=0**.

```text
   Compiling libm v0.2.16
   Compiling num-traits v0.2.19
   Compiling const-oid v0.9.6
   Compiling bitflags v2.13.0
   Compiling getrandom v0.3.4
   Compiling digest v0.10.7
   Compiling spin v0.9.8
   Compiling base64ct v1.8.3
   Compiling rustix v1.1.4
   Compiling rand v0.8.7
   Compiling bit-vec v0.8.0
   Compiling fastrand v2.5.0
   Compiling num-bigint-dig v0.8.6
   Compiling lazy_static v1.5.0
   Compiling hmac v0.12.1
   Compiling sha2 v0.10.9
   Compiling pem-rfc7468 v0.7.0
   Compiling md-5 v0.10.6
   Compiling bit-set v0.8.0
   Compiling security-framework v3.7.0
   Compiling tower-http v0.6.11
   Compiling hkdf v0.12.4
   Compiling der v0.7.10
   Compiling sqlx-core v0.8.6
   Compiling wait-timeout v0.2.1
   Compiling quick-error v1.2.3
   Compiling fancy-regex v0.18.0
   Compiling signature v2.2.0
   Compiling sharded-slab v0.1.7
   Compiling unarray v0.1.4
   Compiling svc-workflow v0.1.0 (/private/tmp/svc-h06-manifest)
   Compiling jcs-canonicalize v0.2.1
   Compiling reqwest v0.12.28
   Compiling tracing-subscriber v0.3.23
   Compiling rustls-platform-verifier v0.7.0
   Compiling spki v0.7.3
   Compiling pkcs8 v0.10.2
   Compiling reqwest v0.13.4
   Compiling pkcs1 v0.7.5
   Compiling rand_core v0.9.5
   Compiling ahash v0.8.12
   Compiling rand_chacha v0.9.0
   Compiling rand v0.9.5
   Compiling sqlx-postgres v0.8.6
   Compiling rand_xorshift v0.4.0
   Compiling referencing v0.47.0
   Compiling num-integer v0.1.46
   Compiling chrono v0.4.45
   Compiling num-complex v0.4.6
   Compiling atoi v2.0.0
   Compiling num-bigint v0.4.8
   Compiling num-iter v0.1.46
   Compiling tempfile v3.27.0
   Compiling num-rational v0.4.2
   Compiling simple_asn1 v0.6.4
   Compiling rusty-fork v0.3.1
   Compiling proptest v1.11.0
   Compiling jsonwebtoken v9.3.1
   Compiling rsa v0.9.10
   Compiling num v0.4.3
   Compiling fraction v0.15.4
   Compiling jsonschema v0.47.0
   Compiling sqlx-macros-core v0.8.6
   Compiling sqlx-macros v0.8.6
   Compiling sqlx v0.8.6
warning: unnecessary parentheses around pattern
   --> src/domain/definition/acceptance_binding.rs:746:13
    |
746 |         let (other_tid) = TransitionId::from_uuid(Uuid::new_v4());
    |             ^         ^
    |
    = note: `#[warn(unused_parens)]` (part of `#[warn(unused)]`) on by default
help: remove these parentheses
    |
746 -         let (other_tid) = TransitionId::from_uuid(Uuid::new_v4());
746 +         let other_tid = TransitionId::from_uuid(Uuid::new_v4());
    |

warning: `svc-workflow` (lib test) generated 1 warning (run `cargo fix --lib -p svc-workflow --tests` to apply 1 suggestion)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 15.02s
     Running unittests src/lib.rs (target/debug/deps/svc_workflow-51a4d00b093396f1)

running 1 test
test store::postgres::admin_recovery_repository::event_replay::trusted_fleet_successor_tests::exact_pair_allowlist_has_unique_deterministic_migration_ids ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 269 filtered out; finished in 0.00s

```

### Gate 3

Command: `cargo test --lib admin_recovery -- --nocapture`

Started: `2026-11-05T21:53:43.176966+00:00`; ended: `2026-11-05T21:53:46.091454+00:00`; **rc=0**.

```text
   Compiling svc-workflow v0.1.0 (/private/tmp/svc-h06-manifest)
warning: unnecessary parentheses around pattern
   --> src/domain/definition/acceptance_binding.rs:746:13
    |
746 |         let (other_tid) = TransitionId::from_uuid(Uuid::new_v4());
    |             ^         ^
    |
    = note: `#[warn(unused_parens)]` (part of `#[warn(unused)]`) on by default
help: remove these parentheses
    |
746 -         let (other_tid) = TransitionId::from_uuid(Uuid::new_v4());
746 +         let other_tid = TransitionId::from_uuid(Uuid::new_v4());
    |

warning: `svc-workflow` (lib test) generated 1 warning (run `cargo fix --lib -p svc-workflow --tests` to apply 1 suggestion)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.19s
     Running unittests src/lib.rs (target/debug/deps/svc_workflow-51a4d00b093396f1)

running 2 tests
test store::postgres::admin_recovery_repository::migration_manifest::tests::manifest_matches_both_historical_inline_bytes ... ok
test store::postgres::admin_recovery_repository::event_replay::trusted_fleet_successor_tests::exact_pair_allowlist_has_unique_deterministic_migration_ids ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 268 filtered out; finished in 0.00s

```

### Gate 4

Command: `cargo test --lib manifest_matches_both_historical_inline_bytes -- --nocapture`

Started: `2026-11-05T21:53:46.091844+00:00`; ended: `2026-11-05T21:53:48.755714+00:00`; **rc=0**.

```text
   Compiling svc-workflow v0.1.0 (/private/tmp/svc-h06-manifest)
warning: unnecessary parentheses around pattern
   --> src/domain/definition/acceptance_binding.rs:746:13
    |
746 |         let (other_tid) = TransitionId::from_uuid(Uuid::new_v4());
    |             ^         ^
    |
    = note: `#[warn(unused_parens)]` (part of `#[warn(unused)]`) on by default
help: remove these parentheses
    |
746 -         let (other_tid) = TransitionId::from_uuid(Uuid::new_v4());
746 +         let other_tid = TransitionId::from_uuid(Uuid::new_v4());
    |

warning: `svc-workflow` (lib test) generated 1 warning (run `cargo fix --lib -p svc-workflow --tests` to apply 1 suggestion)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.92s
     Running unittests src/lib.rs (target/debug/deps/svc_workflow-51a4d00b093396f1)

running 1 test
test store::postgres::admin_recovery_repository::migration_manifest::tests::manifest_matches_both_historical_inline_bytes ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 269 filtered out; finished in 0.00s

```

### Gate 5

Command: `cargo test --test 17_workflow_runtime admin_recovery_digest_migration::before_snapshot_v1_jcs_and_sha256_are_golden -- --exact --nocapture`

Started: `2026-11-05T21:53:48.756082+00:00`; ended: `2026-11-05T21:54:10.046930+00:00`; **rc=0**.

```text
   Compiling svc-workflow v0.1.0 (/private/tmp/svc-h06-manifest)
warning: unused import: `tokio::net::TcpListener`
  --> tests/common/mod.rs:30:5
   |
30 | use tokio::net::TcpListener;
   |     ^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: unused import: `svc_workflow::domain::ids::*`
  --> tests/17_workflow_runtime/http/auth_obo_e2e.rs:18:5
   |
18 | use svc_workflow::domain::ids::*;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `ExecuteWorkflowTransitionCommand`
  --> tests/17_workflow_runtime/http/auth_obo_e2e.rs:20:36
   |
20 |     CreateWorkflowInstanceCommand, ExecuteWorkflowTransitionCommand,
   |                                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `svc_workflow::domain::ids::*`
  --> tests/17_workflow_runtime/http/domain_list.rs:17:5
   |
17 | use svc_workflow::domain::ids::*;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `svc_workflow::domain::ids::*`
  --> tests/17_workflow_runtime/http/worklists.rs:15:5
   |
15 | use svc_workflow::domain::ids::*;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `WorkflowDefinitionId`
  --> tests/17_workflow_runtime/v2_dogfood.rs:25:76
   |
25 | use svc_workflow::domain::ids::{DefinitionVersionId, NodeId, TransitionId, WorkflowDefinitionId};
   |                                                                            ^^^^^^^^^^^^^^^^^^^^

warning: unused import: `tokio::io::AsyncWriteExt`
  --> tests/common/mod.rs:29:5
   |
29 | use tokio::io::AsyncWriteExt;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused variable: `domain_id`
   --> tests/17_workflow_runtime/query/assignee_types.rs:739:18
    |
739 |     let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    |                  ^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_domain_id`
    |
    = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: unused variable: `now`
   --> tests/17_workflow_runtime/http/canary_write.rs:834:9
    |
834 |     let now = chrono::Utc::now().timestamp() as usize;
    |         ^^^ help: if this is intentional, prefix it with an underscore: `_now`

warning: unused variable: `idx`
   --> tests/17_workflow_runtime/minimal_runtime.rs:126:10
    |
126 |     for (idx, (key, source, target, effect)) in edges.iter().enumerate() {
    |          ^^^ help: if this is intentional, prefix it with an underscore: `_idx`

warning: variable does not need to be mutable
   --> tests/17_workflow_runtime/v2_creation.rs:217:9
    |
217 |     let mut step = |actor: Uuid, expected: i32, key: &str| {
    |         ----^^^^
    |         |
    |         help: remove this `mut`
    |
    = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default

warning: field `mock` is never read
  --> tests/17_workflow_runtime/http/e2e/server.rs:14:5
   |
11 | pub(super) struct RunningServer {
   |                   ------------- field in this struct
...
14 |     mock: common::MockJwksServer,
   |     ^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: `svc-workflow` (test "17_workflow_runtime") generated 12 warnings (run `cargo fix --test "17_workflow_runtime" -p svc-workflow` to apply 10 suggestions)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 20.37s
     Running tests/17_workflow_runtime.rs (target/debug/deps/17_workflow_runtime-52b0849c8157633d)

running 1 test
test admin_recovery_digest_migration::before_snapshot_v1_jcs_and_sha256_are_golden ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 454 filtered out; finished in 0.00s

```

### Gate 6

Command: `cargo test --test 27_trusted_fleet_principal_cutover_v1 frozen_plan_digest_and_counts_are_exact -- --exact --nocapture`

Started: `2026-11-05T21:54:10.047284+00:00`; ended: `2026-11-05T21:54:15.875510+00:00`; **rc=0**.

```text
   Compiling svc-workflow v0.1.0 (/private/tmp/svc-h06-manifest)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5.34s
     Running tests/27_trusted_fleet_principal_cutover_v1.rs (target/debug/deps/27_trusted_fleet_principal_cutover_v1-792472f8973ee891)

running 1 test
test frozen_plan_digest_and_counts_are_exact ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.04s

```

### Gate 7

Command: `cargo test --test 27_trusted_fleet_principal_cutover_v1 cli_exposes_only_closed_modes_and_scopes -- --exact --nocapture`

Started: `2026-11-05T21:54:15.875879+00:00`; ended: `2026-11-05T21:54:21.746823+00:00`; **rc=0**.

```text
   Compiling svc-workflow v0.1.0 (/private/tmp/svc-h06-manifest)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.67s
     Running tests/27_trusted_fleet_principal_cutover_v1.rs (target/debug/deps/27_trusted_fleet_principal_cutover_v1-792472f8973ee891)

running 1 test
test cli_exposes_only_closed_modes_and_scopes ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.76s

```

## Tier-2 — PENDING-DOCKER, explicitly NOT_RUN

Exact command from this worktree:

```sh
scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh
```

- Execution status: `NOT_RUN` under the user R2 instruction.
- Result / exit code: `UNKNOWN` / `NOT_AVAILABLE`; no PASS is claimed.
- Dependency: working Docker daemon and PostgreSQL `postgres:16-alpine`, Docker CLI,
  `psql`, Cargo/Rust, the exact restored frozen v2 fixture and disposable databases.
- The existing script creates a disposable container and two DBs, applies Workflow
  migrations, sets `TEST_WORKFLOW_DATABASE_URL`, `TEST_AUTH_DATABASE_URL` and the
  `trusted_fleet_cutover_conformance` cfg, then runs the full cutover integration target.
- Docker EOF / backend present is the regulator's supplied R2 observation.
  R1 independently recorded EOF. This turn did not probe or repair Docker.
  Daemon root cause and recovery time remain `UNKNOWN`.
- Follow-up owner: regulator, after daemon recovery, runs the unchanged full script
  and appends its exact command/environment/commit/output/rc evidence independently.
- **This delivery must not merge before the regulator's Tier-2 full run rc0.**
  Mechanical-exemption final acceptance remains with an authorized maintainer.

## Independent affected-contract review

Two independent read-only agents reviewed the actual source candidate against
base. Authority/consumer review and manifest/test review both returned PASS with
zero blockers. Both independently reproduced exact initializer/string hashes;
the authority reviewer also proved unchanged consumer and module skeletons and
plan hashes. Neither agent edited files or ran Cargo/DB/Docker/production commands.
These are review recommendations, not Owner acceptance or full conformance.
The authority reviewer completed an additional fixed-HEAD recheck of source
commit `6e1b6862c31558f17cdd02de75d5b780f34be4f2`: PASS, zero blockers.
Base ancestry, all six committed source/snapshot byte bindings and all seven
raw gate logs were independently checked. No build/test rerun was required.
This receipt/provenance-only successor commit preserves those source bytes;
final delivery HEAD and remote PR coordinates are recorded in the PR handoff.

## Effects and delivery boundary

Zero production DB/service writes; no DB connection or container was started;
no production service command, core repository edit or window tool invocation.
No plan fixture, Cargo dependency/lockfile, SQL, existing test expectation or runner changed.
All original historical mappings remain in the shared manifest and snapshots.

The authorized remaining delivery steps are to commit this receipt and preserved
mandate/provenance, push `origin ac-h06-migration-manifest`, and create a draft PR
via gh referencing #1360 with the exact marker **Tier-2 conformance pending Docker**.
No merge is attempted. If gh times out, stop on the committed candidate and report
remote outcome UNKNOWN without replay. A final remote readback belongs in the PR
handoff/final response; this receipt does not claim a push or PR not yet executed.
