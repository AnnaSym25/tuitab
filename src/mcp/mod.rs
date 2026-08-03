//! MCP server exposing tuitab's engine to language models.
//!
//! A model that is handed a data file either computes over it in its head — and
//! invents numbers — or writes a throwaway script.  tuitab already has a tested
//! engine for exactly this work: it loads a dozen formats, infers types, and can
//! group, pivot, join and export.  This module is the way to call that engine
//! without a terminal.
//!
//! The model expresses a request as a **structured pipeline of operations**, not
//! as SQL.  Two reasons: every operation maps onto an existing tuitab function
//! (so the arithmetic is Polars', not a reimplementation), and a JSON-schema'd
//! operation is validated where a SQL string is not — a typo in a schema is an
//! error, a typo in SQL is a silently wrong number.
//!
//! Transport is stdio, so **nothing may be written to stdout except protocol
//! messages**.  Diagnostics go to stderr.

pub mod pipeline;
pub mod render;
pub mod rpc;
pub mod source;
pub mod tools;

use color_eyre::Result;
use serde_json::{json, Value};
use std::io::{BufRead, Write};

/// Server state that outlives a single request.
#[derive(Default)]
pub struct Server {
    /// Last loaded source.  The model's working loop is inspect → query → query
    /// → describe against one file; without this every call re-reads the
    /// workbook.  One entry, not an LRU: the only operation needing two files at
    /// once is `join`, and that loads its right-hand side itself.
    pub cache: Option<source::Cached>,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Handle one line of the transport, returning the line to write back.
///
/// `None` means the message was a notification or a response — nothing is sent.
/// Exposed for tests, which drive this directly instead of spawning a process.
pub fn handle_message(server: &mut Server, line: &str) -> Option<Value> {
    let request = match rpc::parse(line) {
        rpc::Incoming::Call(r) => r,
        rpc::Incoming::Ignore => return None,
        rpc::Incoming::Invalid { id, code } => {
            let message = if code == rpc::PARSE_ERROR {
                "Parse error: message is not valid JSON"
            } else {
                "Invalid request"
            };
            return Some(rpc::error(id, code, message));
        }
    };

    // Notifications get no response no matter what they are.
    let id = request.id?;

    let result: Result<Value, String> = match request.method.as_str() {
        "initialize" => Ok(initialize_result(&request.params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools::definitions()})),
        "tools/call" => return Some(call_tool(server, id, &request.params)),
        other => {
            return Some(rpc::error(
                Some(id),
                rpc::METHOD_NOT_FOUND,
                format!("Unknown method: {}", other),
            ))
        }
    };

    match result {
        Ok(value) => Some(rpc::success(id, value)),
        Err(message) => Some(rpc::error(Some(id), rpc::INTERNAL_ERROR, message)),
    }
}

fn initialize_result(params: &Value) -> Value {
    let requested = params.get("protocolVersion").and_then(Value::as_str);
    json!({
        "protocolVersion": rpc::negotiate_version(requested),
        "capabilities": {"tools": {}},
        "serverInfo": {
            "name": "tuitab",
            "title": "tuitab — tabular data engine",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "instructions": tools::INSTRUCTIONS,
    })
}

fn call_tool(server: &mut Server, id: Value, params: &Value) -> Value {
    let name = match params.get("name").and_then(Value::as_str) {
        Some(n) => n,
        None => {
            return rpc::error(
                Some(id),
                rpc::INVALID_PARAMS,
                "tools/call requires a 'name' parameter",
            )
        }
    };

    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    match tools::call(server, name, &arguments) {
        Ok(payload) => rpc::success(id, rpc::tool_success(payload)),
        // An unknown tool breaks the protocol contract; a tool that ran and
        // failed is a result the model should see and react to.
        Err(tools::CallError::UnknownTool(n)) => rpc::error(
            Some(id),
            rpc::INVALID_PARAMS,
            format!("Unknown tool: {}", n),
        ),
        Err(tools::CallError::Failed(message)) => rpc::success(id, rpc::tool_error(message)),
    }
}

/// Read JSON-RPC messages from stdin until EOF, writing responses to stdout.
pub fn serve() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut server = Server::new();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = handle_message(&mut server, &line) {
            // Messages must not contain embedded newlines, which `to_string`
            // guarantees — it never pretty-prints.
            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
            stdout.flush()?;
        }
    }

    Ok(())
}
