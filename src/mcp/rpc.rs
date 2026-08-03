//! JSON-RPC 2.0 framing for the MCP stdio transport.
//!
//! The MCP stdio transport is newline-delimited JSON-RPC 2.0 (spec 2025-06-18,
//! `basic/transports`): one message per line, UTF-8, no embedded newlines.  A
//! server that only exposes tools needs five methods — `initialize`,
//! `notifications/initialized`, `ping`, `tools/list` and `tools/call` — which is
//! small enough to implement directly on `serde_json` rather than pull in an SDK
//! and an async runtime for it.
//!
//! Requests are handled one at a time.  The spec permits this: the client simply
//! waits for each response, and no tool here is long-running enough to need
//! interleaving.

use serde_json::{json, Value};

/// Protocol versions this server speaks.  The tool surface is identical across
/// all three, so the only thing negotiation decides is which string to echo.
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];
pub const LATEST_PROTOCOL_VERSION: &str = "2025-06-18";

// JSON-RPC 2.0 error codes.
pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;

/// An incoming JSON-RPC message, already known to be a request or notification.
pub struct Request {
    /// `None` marks a notification — the caller must not send a response.
    pub id: Option<Value>,
    pub method: String,
    pub params: Value,
}

/// Classification of one incoming line.
pub enum Incoming {
    Call(Request),
    /// A response to something we sent.  We never send requests, so this is
    /// ignored rather than treated as an error.
    Ignore,
    /// Malformed JSON or a message that is not a valid JSON-RPC request.
    Invalid {
        id: Option<Value>,
        code: i64,
    },
}

/// Parse one line of the transport.
pub fn parse(line: &str) -> Incoming {
    let value: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => {
            return Incoming::Invalid {
                id: None,
                code: PARSE_ERROR,
            }
        }
    };

    let obj = match value.as_object() {
        Some(o) => o,
        None => {
            return Incoming::Invalid {
                id: None,
                code: INVALID_REQUEST,
            }
        }
    };

    // A message carrying `result` or `error` is a response, not a request.
    if obj.contains_key("result") || obj.contains_key("error") {
        return Incoming::Ignore;
    }

    let id = obj.get("id").filter(|v| !v.is_null()).cloned();

    let method = match obj.get("method").and_then(Value::as_str) {
        Some(m) => m.to_string(),
        None => {
            return Incoming::Invalid {
                id,
                code: INVALID_REQUEST,
            }
        }
    };

    let params = obj.get("params").cloned().unwrap_or(Value::Null);

    Incoming::Call(Request { id, method, params })
}

/// Build a successful response envelope.
pub fn success(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// Build a protocol-level error envelope.
///
/// Reserved for things that break the contract itself — unknown method, bad
/// arguments, unknown tool.  A tool that runs and fails reports that through
/// [`tool_error`] instead, so the model sees the message and can correct itself.
pub fn error(id: Option<Value>, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": {"code": code, "message": message.into()},
    })
}

/// A `tools/call` result carrying a structured payload.
///
/// The JSON text block duplicates `structuredContent` — the spec asks for it so
/// clients that predate structured content still see something useful.
pub fn tool_success(payload: Value) -> Value {
    let text = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    json!({
        "content": [{"type": "text", "text": text}],
        "structuredContent": payload,
        "isError": false,
    })
}

/// A `tools/call` result reporting that the tool ran and failed.
pub fn tool_error(message: impl Into<String>) -> Value {
    json!({
        "content": [{"type": "text", "text": message.into()}],
        "isError": true,
    })
}

/// Negotiate the protocol version: echo the client's if we speak it, otherwise
/// answer with our latest and let the client decide whether to continue.
pub fn negotiate_version(requested: Option<&str>) -> &'static str {
    match requested {
        Some(v) => SUPPORTED_PROTOCOL_VERSIONS
            .iter()
            .find(|s| **s == v)
            .copied()
            .unwrap_or(LATEST_PROTOCOL_VERSION),
        None => LATEST_PROTOCOL_VERSION,
    }
}
