//! KC-10 — the embedder seam: explicit-required (D3) + HTTP path against a
//! localhost stub (no real network).

use okf_pack::embed::{KnowledgeConfig, approx_tokens};
use std::io::{Read, Write};
use std::net::TcpListener;

/// Spawn a one-shot localhost HTTP server returning a fixed embeddings JSON.
fn stub_embedding_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 2048];
            let _ = stream.read(&mut buf); // drain the request (best-effort)
            let body = r#"{"embeddings":[[0.1,0.2,0.3]]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    format!("http://{addr}/embed")
}

#[test]
fn embedder_explicit_required_errors_when_unset() {
    // D3: no embedder configured → resolution fails clearly.
    let config = KnowledgeConfig::default();
    let msg = config
        .resolve()
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(
        msg.contains("no embedder configured"),
        "clear message: {msg}"
    );
}

#[test]
fn embedder_local_declares_net_mode_none() {
    let config = KnowledgeConfig {
        embedder: Some("local".into()),
        endpoint: Some("http://localhost:1234".into()),
        ..KnowledgeConfig::default()
    };
    assert_eq!(config.net_mode(), "none", "local embedder is offline");
    assert!(config.resolve().is_ok());
}

#[test]
fn embedder_frontier_resolves_and_is_external() {
    let config = KnowledgeConfig {
        embedder: Some("frontier".into()),
        endpoint: Some("http://localhost:1234".into()),
        ..KnowledgeConfig::default()
    };
    assert_eq!(config.net_mode(), "frontier");
    assert!(config.resolve().is_ok());
}

#[test]
fn embedder_local_requires_endpoint() {
    let config = KnowledgeConfig {
        embedder: Some("local".into()),
        ..KnowledgeConfig::default()
    };
    assert!(
        config.resolve().is_err(),
        "local without endpoint must error"
    );
}

#[test]
fn embedder_embeds_against_stub_server() {
    let url = stub_embedding_server();
    let config = KnowledgeConfig {
        embedder: Some("local".into()),
        endpoint: Some(url),
        ..KnowledgeConfig::default()
    };
    let embedder = config.resolve().unwrap();
    let vectors = embedder.embed(&["hello world".to_string()]).unwrap();
    assert_eq!(vectors, vec![vec![0.1, 0.2, 0.3]]);
    assert_eq!(embedder.count_tokens("hello world foo"), 3);
}

#[test]
fn embedder_approx_tokens_counts_words() {
    assert_eq!(approx_tokens("one two  three"), 3);
    assert_eq!(approx_tokens(""), 0);
}
