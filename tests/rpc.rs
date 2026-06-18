//! KC-6 — JSON-RPC server: mandatory methods live, custom methods dispatch.

use okf_pack::rpc::Server;
use serde_json::Value;

fn call(server: &Server, line: &str) -> Value {
    serde_json::from_str(&server.handle(line)).expect("response is JSON")
}

#[test]
fn rpc_ping_returns_pong() {
    let server = Server::new();
    let r = call(&server, r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#);
    assert_eq!(r["result"]["pong"], true);
    assert_eq!(r["id"], 1);
}

#[test]
fn rpc_capabilities_echoes_manifest() {
    let server = Server::new();
    let r = call(
        &server,
        r#"{"jsonrpc":"2.0","id":2,"method":"capabilities"}"#,
    );
    // cwd is the repo root, so package.toml is read.
    assert_eq!(r["result"]["fs"], "space-dir");
    assert_eq!(r["result"]["net"], "frontier");
}

#[test]
fn rpc_unknown_method_is_method_not_found() {
    let server = Server::new();
    let r = call(
        &server,
        r#"{"jsonrpc":"2.0","id":3,"method":"does-not-exist"}"#,
    );
    assert_eq!(r["error"]["code"], -32601);
}

#[test]
fn rpc_custom_methods_all_dispatch() {
    let server = Server::new();
    for method in [
        "index_space",
        "query",
        "export_okf",
        "import_okf",
        "validate",
        "stats",
    ] {
        let line = format!(r#"{{"jsonrpc":"2.0","id":4,"method":"{method}"}}"#);
        let r = call(&server, &line);
        // Registered (so it dispatches): a -32000 stub, NOT -32601 method-not-found.
        assert_eq!(
            r["error"]["code"], -32000,
            "method `{method}` should dispatch to a stub"
        );
    }
}

#[test]
fn rpc_parse_error_is_handled() {
    let server = Server::new();
    let r = call(&server, "{ not json");
    assert_eq!(r["error"]["code"], -32700);
}
