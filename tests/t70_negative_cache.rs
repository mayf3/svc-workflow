//! T70 regression — unknown-kid negative-result memory.
//! 32 concurrent requests carrying the SAME unknown kid must cause ~1 remote
//! JWKS fetch (shared negative result), not 32 repeated fetches.
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig, JwksVerifier};

fn b64url(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

fn jwt_with_kid(kid: &str) -> String {
    let header = b64url(format!(r#"{{"alg":"RS256","kid":"{kid}"}}"#).as_bytes());
    let claims = b64url(br#"{"sub":"t70"}"#);
    format!("{header}.{claims}.c2ln")
}

fn good_jwks() -> &'static str {
    r#"{"keys":[{"kty":"RSA","kid":"k1","n":"dGVzdA","e":"AQAB","use":"sig","alg":"RS256"}]}"#
}

fn spawn_counting_server(listener: std::net::TcpListener, fetches: Arc<AtomicU64>) {
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let fetches = fetches.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                let mut seen = Vec::new();
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) => return,
                        Ok(n) => {
                            seen.extend_from_slice(&buf[..n]);
                            if seen.windows(4).any(|w| w == b"\r\n\r\n") {
                                break;
                            }
                        }
                        Err(_) => return,
                    }
                }
                fetches.fetch_add(1, Ordering::SeqCst);
                let body = good_jwks();
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(resp.as_bytes());
                let _ = stream.write_all(body.as_bytes());
                let _ = stream.flush();
            });
        }
    });
}

fn verifier_for(url: &str) -> JwksVerifier {
    let config = JwksConfig {
        jwks_url: url.to_string(),
        issuer: "https://t70.invalid".to_string(),
        audience: "svc-workflow".to_string(),
        cache_ttl_secs: 600,
        http_timeout_secs: 5,
        max_stale_secs: 3600,
        clock_skew_seconds: 5,
    };
    let mut canary = AuthV1CanaryConfig::default();
    canary.enabled = true;
    JwksVerifier::new(&config, &canary)
}

#[tokio::test]
async fn unknown_kid_negative_result_is_shared_not_repeated() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let fetches = Arc::new(AtomicU64::new(0));
    spawn_counting_server(listener, fetches.clone());
    let verifier = verifier_for(&format!("http://127.0.0.1:{port}/jwks"));
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Warm cache with k1, then measure ONLY the unknown-kid window.
    assert!(verifier.is_ready().await);
    fetches.store(0, Ordering::SeqCst);

    const N: usize = 32;
    let token = jwt_with_kid("unknown-shared-kid");
    let mut handles = Vec::new();
    for _ in 0..N {
        let v = verifier.clone();
        let t = token.clone();
        handles.push(tokio::spawn(async move {
            v.verify(&t).await.err().map(|e| e.code().to_string())
        }));
    }
    let mut unknown = 0usize;
    for h in handles {
        if h.await.unwrap().as_deref() == Some("unknown_kid") {
            unknown += 1;
        }
    }
    let n = fetches.load(Ordering::SeqCst);
    println!("T70 REGRESSION: fetches={n}, unknown_kid responses={unknown}");
    assert_eq!(unknown, N, "every caller must get the unknown_kid answer");
    assert!(n <= 1, "negative result must be shared: one refresh, not {n} fetches");
}

/// r2 (DAY_OWNER_REPAIR_RECONCILIATION_AND_MERGE_20260918_V1 §4): negative
/// entries must be JWKS-GENERATION-bound. After a kid is confirmed absent
/// (generation N), a NEW refresh generation (any successful fetch — here a
/// second distinct unknown kid triggering one) invalidates the entry, and
/// the SAME kid is retried against the fresh keys. The pre-r2 fixed-TTL
/// memory kept blocking the kid for the full 30s regardless of refreshes.
#[tokio::test]
async fn negative_entries_are_generation_bound_next_refresh_allows_retry() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let fetches = Arc::new(AtomicU64::new(0));
    spawn_counting_server(listener, fetches.clone());
    let verifier = verifier_for(&format!("http://127.0.0.1:{port}/jwks"));
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(verifier.is_ready().await);
    fetches.store(0, Ordering::SeqCst);

    // Generation N: kid A confirmed absent -> negative entry recorded.
    let token_a = jwt_with_kid("rotating-kid-a");
    let err = verifier.verify(&token_a).await.err().expect("unknown kid a");
    assert_eq!(err.code(), "unknown_kid");
    let after_a = fetches.load(Ordering::SeqCst);
    assert!(after_a >= 1, "first unknown-kid check must fetch");

    // Share check: an immediate repeat of A is served from the negative
    // memory of the SAME generation (no new fetch).
    let err = verifier.verify(&token_a).await.err().expect("unknown kid a 2");
    assert_eq!(err.code(), "unknown_kid");
    assert_eq!(fetches.load(Ordering::SeqCst), after_a, "same-generation repeat must not re-fetch");

    // Trigger a NEW generation: kid B (unknown) causes one fresh fetch;
    // the server publishes the same keys, but the generation advances.
    let token_b = jwt_with_kid("trigger-kid-b");
    let err = verifier.verify(&token_b).await.err().expect("unknown kid b");
    assert_eq!(err.code(), "unknown_kid");
    let after_b = fetches.load(Ordering::SeqCst);
    assert_eq!(after_b, after_a + 1, "kid B miss must trigger exactly one new fetch (new generation)");

    // Owner-frozen semantics: the next legitimate refresh generation allows
    // the previously-absent kid A to be RETRIED — the stale negative entry
    // must not block it (pre-r2 it stayed blocked for the full 30s TTL).
    let err = verifier.verify(&token_a).await.err().expect("unknown kid a 3");
    assert_eq!(err.code(), "unknown_kid");
    assert!(
        fetches.load(Ordering::SeqCst) >= after_b + 1,
        "generation advance must invalidate the negative entry and re-fetch for kid A"
    );
}
