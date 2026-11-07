#!/usr/bin/env python3
"""Build identical diagnostic fixtures on the clean base and candidate.

The ordinary healthy-channel latency gates remain in the normal test path.
Only the explicitly selected diagnostic path calls one channel once, to
reach the same original backoff assertion on both product revisions.
"""
import difflib
import hashlib
import json
import pathlib
import subprocess

root = pathlib.Path(__file__).resolve().parents[3]
original = subprocess.check_output([
    "git", "show", "062fe82301eadbf23b5b6279643355c20ca4218c:tests/48_outbox_three_channels.rs"
], cwd=root, text=True)
marker = "    let background_pool = pool.clone();"
start = original.index("async fn assert_sustained_channel_isolation(")
point = original.index(marker, start)
branch = '''    if std::env::var("H1359_BACKOFF_ATTRIBUTION").as_deref() == Ok("1") {
        // Diagnostic only: remove peer latency from this attribution question.
        // Keep the real HTTP timeout and original settlement/backoff code.
        if slow_kind != "FORUM_EVENT" { config.forum = None; }
        if slow_kind != "EXECUTION_KICK" { config.kick = None; }
        if slow_kind != "OWNER_ASSISTANCE_WAKE" { config.owner_assistance_wake = Ok(None); }
        let began = Instant::now();
        svc_workflow::forum_sync::run_once(&pool, &ForumClient::new(config)).await;
        let slow_state = row_state(&pool, slow_row).await;
        let (next_attempt_at, remaining_seconds): (Timestamp, f64) = sqlx::query_as(
            "SELECT next_attempt_at, EXTRACT(EPOCH FROM (next_attempt_at - clock_timestamp()))::float8 FROM workflow_outbox WHERE outbox_id=$1")
            .bind(slow_row).fetch_one(&pool).await.unwrap();
        println!("H1359_BACKOFF_ATTRIBUTION slow={slow_kind} elapsed_ms={} attempts={} next_attempt_at={next_attempt_at} remaining_seconds={remaining_seconds:.3} next_attempt_future={}",
            began.elapsed().as_millis(), slow_state.attempt_count, slow_state.next_attempt_future);
        server.abort();
        let _ = server.await;
        pool.close().await;
        assert_eq!(slow_state.attempt_count, 1);
        assert!(slow_state.delivered_at.is_none());
        assert!(slow_state.dead_lettered_at.is_none());
        assert!(slow_state.last_error.is_some());
        assert!(slow_state.next_attempt_future, "original backoff is retained");
        return;
    }
'''
diagnostic = original[:point] + branch + original[point:]
out = pathlib.Path(__file__).resolve().parent
(out / "attribution-only.patch").write_text("".join(difflib.unified_diff(
    original.splitlines(True), diagnostic.splitlines(True),
    fromfile="R2/tests/48_outbox_three_channels.rs", tofile="diagnostic/tests/48_outbox_three_channels.rs")))
for tree in ("/private/tmp/h1359-r3-base", "/private/tmp/h1359-r3-candidate"):
    pathlib.Path(tree, "tests/48_outbox_three_channels.rs").write_text(diagnostic)
print(json.dumps({"original_test_sha256": hashlib.sha256(original.encode()).hexdigest(),
                  "diagnostic_test_sha256": hashlib.sha256(diagnostic.encode()).hexdigest()}))
