//! KC-6 — the Spaces pack RPC server (line-delimited JSON-RPC 2.0 over stdio).
//!
//! Dispatch is an **open, data-driven handler table** (a map of method name →
//! handler), never a closed `match` — adding a method is inserting an entry, per
//! ADR 0002's hot-swap invariant. The mandatory methods (`ping` / `shutdown` /
//! `capabilities`) are live; the knowledge methods are registered as stubs that
//! return a server error (`-32000`) so they *dispatch* (distinct from an unknown
//! method, `-32601`) until their queue items land.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, Write};

#[derive(Deserialize)]
struct Request {
    #[serde(default)]
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Serialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Serialize)]
struct Response {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

type Handler = fn(&Value) -> Result<Value, RpcError>;

/// JSON-RPC dispatcher for the pack.
pub struct Server {
    handlers: HashMap<&'static str, Handler>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    /// Build the dispatcher with the mandatory methods live and the knowledge
    /// methods registered as `-32000` stubs.
    #[must_use]
    pub fn new() -> Self {
        let mut handlers: HashMap<&'static str, Handler> = HashMap::new();
        handlers.insert("ping", |_| Ok(json!({ "pong": true })));
        handlers.insert("capabilities", |_| Ok(capabilities()));
        handlers.insert("shutdown", |_| Ok(json!({ "ok": true })));
        for method in [
            "index_space",
            "query",
            "export_okf",
            "import_okf",
            "validate",
            "stats",
        ] {
            handlers.insert(method, |_| {
                Err(RpcError {
                    code: -32000,
                    message: "not yet implemented".into(),
                })
            });
        }
        Self { handlers }
    }

    /// Handle one JSON-RPC line, returning the response line.
    #[must_use]
    pub fn handle(&self, line: &str) -> String {
        let request: Request = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => return error_response(Value::Null, -32700, &format!("parse error: {e}")),
        };
        match self.handlers.get(request.method.as_str()) {
            Some(handler) => match handler(&request.params) {
                Ok(result) => ok_response(request.id, result),
                Err(error) => serialize(Response {
                    jsonrpc: "2.0",
                    id: request.id,
                    result: None,
                    error: Some(error),
                }),
            },
            None => error_response(
                request.id,
                -32601,
                &format!("method not found: {}", request.method),
            ),
        }
    }
}

/// Run the server over stdin/stdout until EOF or a `shutdown` request.
pub fn serve_stdio() -> anyhow::Result<()> {
    let server = Server::new();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        writeln!(stdout, "{}", server.handle(&line))?;
        stdout.flush()?;
        if is_shutdown(&line) {
            break;
        }
    }
    Ok(())
}

fn is_shutdown(line: &str) -> bool {
    serde_json::from_str::<Request>(line)
        .map(|r| r.method == "shutdown")
        .unwrap_or(false)
}

/// Echo the pack's declared `[capabilities]` from `package.toml` in the cwd.
fn capabilities() -> Value {
    let (fs, net) = std::fs::read_to_string("package.toml")
        .ok()
        .and_then(|s| toml::from_str::<toml::Value>(&s).ok())
        .and_then(|t| {
            let caps = t.get("capabilities")?;
            Some((
                caps.get("fs")?.as_str()?.to_string(),
                caps.get("net")?.as_str()?.to_string(),
            ))
        })
        .unwrap_or_else(|| ("space-dir".into(), "frontier".into()));
    json!({ "fs": fs, "net": net })
}

fn ok_response(id: Value, result: Value) -> String {
    serialize(Response {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    })
}

fn error_response(id: Value, code: i64, message: &str) -> String {
    serialize(Response {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(RpcError {
            code,
            message: message.to_string(),
        }),
    })
}

fn serialize(response: Response) -> String {
    serde_json::to_string(&response).expect("response serializes")
}
