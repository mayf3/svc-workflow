//! Task-local directory fixture for tests that exercise business paths.
//! Authentication, HTTP mapping, admission and database transactions stay real;
//! only the two external identity directories are replaced by this loopback stub.
use std::io::{Read, Write};
use std::sync::OnceLock;
use svc_workflow::auth::admission::{AdmissionClient, AdmissionConfig, SecretString};
use svc_workflow::store::postgres::admission_gate::AdmissionGate;

pub fn admitting_config() -> AdmissionConfig {
    static BASE: OnceLock<String> = OnceLock::new();
    let base = BASE.get_or_init(|| {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                std::thread::spawn(move || {
                    stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
                    let mut bytes=Vec::new(); let mut buffer=[0;4096];
                    let (end,length) = loop {
                        let Ok(n)=stream.read(&mut buffer) else {return};
                        if n==0 {return}; bytes.extend_from_slice(&buffer[..n]);
                        if let Some(end)=bytes.windows(4).position(|w| w==b"\r\n\r\n") {
                            let header=String::from_utf8_lossy(&bytes[..end]);
                            let length=header.lines().find_map(|line| line.split_once(':').filter(|(k,_)| k.eq_ignore_ascii_case("content-length")).and_then(|(_,v)| v.trim().parse::<usize>().ok())).unwrap_or(0);
                            break (end+4,length);
                        }
                    };
                    while bytes.len()<end+length { let Ok(n)=stream.read(&mut buffer) else{return}; if n==0{return};bytes.extend_from_slice(&buffer[..n]); }
                    let header=String::from_utf8_lossy(&bytes[..end]);
                    let path=header.lines().next().and_then(|l| l.split_whitespace().nth(1)).unwrap_or("");
                    let (status,body) = if path=="/oauth/token" {
                        let request=String::from_utf8_lossy(&bytes[end..end+length]);
                        let scope=request.split('&').find_map(|p| p.strip_prefix("scope=")).unwrap_or("");
                        (200,serde_json::json!({"access_token":"owned-test-token","token_type":"Bearer","expires_in":3600,"scope":scope}))
                    } else if let Some(p)=path.strip_prefix("/api/v1/directory/principals/").and_then(|p|p.strip_suffix("/agent")) {
                        (200,serde_json::json!({"principalId":p,"agentId":format!("agent-{p}"),"principalStatus":"active"}))
                    } else if let Some(id)=path.strip_prefix("/v1/directory/agents/") {
                        (200,serde_json::json!({"agentId":id,"exists":true,"enabled":true}))
                    } else {(404,serde_json::json!({"error":"unexpected_fixture_route"}))};
                    let body=body.to_string();
                    let response=format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
                    let _=stream.write_all(response.as_bytes());
                });
            }
        });
        base
    });
    AdmissionConfig {
        enabled: true,
        auth_base_url: base.clone(),
        core_base_url: base.clone(),
        client_id: "owned-test-workflow".into(),
        client_secret: SecretString::new("owned-test-secret"),
        ..AdmissionConfig::disabled()
    }
}

pub fn admitting_gate() -> AdmissionGate<'static> {
    static CLIENT: OnceLock<AdmissionClient> = OnceLock::new();
    AdmissionGate::new(Some(
        CLIENT.get_or_init(|| AdmissionClient::new(admitting_config()).unwrap()),
    ))
}
