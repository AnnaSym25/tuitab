//! Coverage for the MCP server.
//!
//! Most of these drive `handle_message` directly — it is the whole request path
//! minus the pipe, so a test gets the same answer a client would without paying
//! for a process.  One test at the end does spawn the binary, to check the
//! framing and that nothing leaks onto stdout.

use serde_json::{json, Value};
use std::path::PathBuf;
use tuitab::mcp::{handle_message, Server};

fn tmp(name: &str) -> PathBuf {
    // Not the system temp dir: test output belongs to the project, under the
    // gitignored tmp/ directory.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tmp")
        .join("mcp-tests");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn send(server: &mut Server, message: Value) -> Value {
    handle_message(server, &message.to_string()).expect("a request must get a response")
}

/// Call a tool and return its structured payload, failing on a tool error.
fn call(server: &mut Server, name: &str, arguments: Value) -> Value {
    let response = send(
        server,
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
               "params": {"name": name, "arguments": arguments}}),
    );
    let result = response
        .get("result")
        .unwrap_or_else(|| panic!("{} returned a protocol error: {}", name, response));
    assert_eq!(
        result.get("isError"),
        Some(&json!(false)),
        "{} failed: {}",
        name,
        result
    );
    result.get("structuredContent").cloned().unwrap()
}

/// Call a tool expecting it to run and fail, returning the message.
fn call_expecting_failure(server: &mut Server, name: &str, arguments: Value) -> String {
    let response = send(
        server,
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
               "params": {"name": name, "arguments": arguments}}),
    );
    let result = response
        .get("result")
        .expect("a tool error is still a result");
    assert_eq!(
        result.get("isError"),
        Some(&json!(true)),
        "expected {} to fail, got {}",
        name,
        result
    );
    result["content"][0]["text"].as_str().unwrap().to_string()
}

fn query(server: &mut Server, ops: Value) -> Value {
    call(
        server,
        "tuitab_query",
        json!({"source": {"path": "test_data/sample.csv"}, "ops": ops}),
    )
}

/// Cells of a result row, keyed by column name.
fn row(result: &Value, index: usize) -> Vec<(String, Value)> {
    let names: Vec<String> = result["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_string())
        .collect();
    names
        .into_iter()
        .zip(result["rows"][index].as_array().unwrap().iter().cloned())
        .collect()
}

fn cell(result: &Value, index: usize, column: &str) -> Value {
    row(result, index)
        .into_iter()
        .find(|(name, _)| name == column)
        .unwrap_or_else(|| panic!("no column '{}' in {}", column, result["columns"]))
        .1
}

// ── protocol ────────────────────────────────────────────────────────────────

#[test]
fn initialize_answers_with_the_requested_version_and_the_instructions() {
    let mut server = Server::new();
    let response = send(
        &mut server,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                          "clientInfo": {"name": "test", "version": "0"}}}),
    );

    let result = &response["result"];
    assert_eq!(result["protocolVersion"], "2025-06-18");
    assert_eq!(result["serverInfo"]["name"], "tuitab");
    assert!(result["capabilities"]["tools"].is_object());

    let instructions = result["instructions"].as_str().unwrap();
    assert!(instructions.contains("tuitab_inspect"), "{}", instructions);
    assert!(instructions.contains("group_by"), "{}", instructions);
}

#[test]
fn an_unsupported_protocol_version_falls_back_to_one_we_speak() {
    let mut server = Server::new();
    let response = send(
        &mut server,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocolVersion": "1.0.0"}}),
    );
    assert_eq!(response["result"]["protocolVersion"], "2025-06-18");
}

#[test]
fn notifications_and_responses_get_no_reply() {
    let mut server = Server::new();
    assert!(handle_message(
        &mut server,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#
    )
    .is_none());
    assert!(handle_message(&mut server, r#"{"jsonrpc":"2.0","id":7,"result":{}}"#).is_none());
}

#[test]
fn malformed_input_is_a_parse_error_rather_than_a_crash() {
    let mut server = Server::new();
    let response = handle_message(&mut server, "not json at all").unwrap();
    assert_eq!(response["error"]["code"], -32700);
}

#[test]
fn tools_list_offers_the_four_tools() {
    let mut server = Server::new();
    let response = send(
        &mut server,
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
    );

    let names: Vec<&str> = response["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec![
            "tuitab_inspect",
            "tuitab_query",
            "tuitab_describe",
            "tuitab_jq"
        ]
    );

    for tool in response["result"]["tools"].as_array().unwrap() {
        assert!(
            tool["inputSchema"]["properties"].is_object(),
            "{}",
            tool["name"]
        );
        assert!(
            !tool["description"].as_str().unwrap().is_empty(),
            "{}",
            tool["name"]
        );
    }
}

#[test]
fn an_unknown_method_is_a_protocol_error() {
    let mut server = Server::new();
    let response = send(
        &mut server,
        json!({"jsonrpc": "2.0", "id": 1, "method": "nope"}),
    );
    assert_eq!(response["error"]["code"], -32601);
}

/// An unknown *tool* breaks the contract, so it is a protocol error — unlike a
/// tool that runs and fails, which comes back as a result the model can read.
#[test]
fn an_unknown_tool_is_a_protocol_error() {
    let mut server = Server::new();
    let response = send(
        &mut server,
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
               "params": {"name": "tuitab_nonesuch", "arguments": {}}}),
    );
    assert_eq!(response["error"]["code"], -32602);
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("tuitab_nonesuch"));
}

// ── tuitab_inspect ──────────────────────────────────────────────────────────

#[test]
fn inspect_reports_columns_types_and_a_sample() {
    let mut server = Server::new();
    let result = call(
        &mut server,
        "tuitab_inspect",
        json!({"source": "test_data/sample.csv"}),
    );

    assert_eq!(result["row_count"], 20);
    let columns: Vec<&str> = result["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(columns, vec!["id", "name", "age", "salary", "department"]);

    let types: Vec<&str> = result["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["type"].as_str().unwrap())
        .collect();
    assert_eq!(types[2], "integer", "age");
    assert_eq!(types[4], "string", "department");

    assert_eq!(result["sample_rows"].as_array().unwrap().len(), 5);
}

/// Multi-table formats are the reason `container` exists, and the repository
/// ships no SQLite fixture — so this makes one, which also exercises the saver.
#[test]
fn inspect_lists_tables_before_showing_columns() {
    let mut server = Server::new();
    let db = tmp("sample.sqlite");
    let _ = std::fs::remove_file(&db);

    call(
        &mut server,
        "tuitab_query",
        json!({"source": "test_data/sample.csv", "ops": [],
               "output": {"path": db.to_string_lossy()}}),
    );

    // Without a container, the answer is the list of tables, not rows.
    let listing = call(
        &mut server,
        "tuitab_inspect",
        json!({"source": {"path": db.to_string_lossy()}}),
    );
    let tables: Vec<&str> = listing["containers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect();
    assert_eq!(
        tables,
        vec!["data"],
        "save_sqlite writes one table named 'data'"
    );
    assert!(
        listing.get("columns").is_none(),
        "no columns without a container"
    );
    assert!(listing["note"].as_str().unwrap().contains("container"));

    // With one, it is an ordinary table.
    let table = call(
        &mut server,
        "tuitab_inspect",
        json!({"source": {"path": db.to_string_lossy(), "container": "data"}}),
    );
    assert_eq!(table["row_count"], 20);

    // And it queries like any other source.
    let result = call(
        &mut server,
        "tuitab_query",
        json!({"source": {"path": db.to_string_lossy(), "container": "data"},
               "ops": [{"frequency": {"by": ["department"]}}]}),
    );
    assert_eq!(result["row_count"], 4);

    std::fs::remove_file(&db).unwrap();
}

#[test]
fn inspect_reports_a_missing_file_as_a_tool_error() {
    let mut server = Server::new();
    let message = call_expecting_failure(
        &mut server,
        "tuitab_inspect",
        json!({"source": "test_data/does_not_exist.csv"}),
    );
    assert!(message.contains("No such file"), "{}", message);
}

// ── filtering and sorting ───────────────────────────────────────────────────

#[test]
fn filter_keeps_only_matching_rows() {
    let mut server = Server::new();
    // Seven people are over 40: ages 45, 42, 52, 47, 44, 50 and 41.
    let result = query(
        &mut server,
        json!([{"filter": [{"col": "age", "op": "gt", "value": 40}]}]),
    );
    assert_eq!(result["row_count"], 7);
}

#[test]
fn filter_predicates_combine_with_and() {
    let mut server = Server::new();
    // Engineering ages are 30, 28, 42, 52, 33, 37 and 34 — two are over 40.
    // Seven people company-wide are, so this only passes if both predicates
    // apply rather than the last one winning.
    let result = query(
        &mut server,
        json!([{"filter": [
            {"col": "department", "op": "eq", "value": "Engineering"},
            {"col": "age", "op": "gt", "value": 40}
        ]}]),
    );
    assert_eq!(result["row_count"], 2);
}

#[test]
fn filter_supports_in_between_and_contains() {
    let mut server = Server::new();

    let in_list = query(
        &mut server,
        json!([{"filter": [{"col": "department", "op": "in", "value": ["HR", "Marketing"]}]}]),
    );
    assert_eq!(in_list["row_count"], 8, "4 in HR and 4 in Marketing");

    let between = query(
        &mut server,
        json!([{"filter": [{"col": "age", "op": "between", "value": [30, 35]}]}]),
    );
    assert_eq!(
        between["row_count"], 5,
        "ages 30, 35, 31, 33 and 34, bounds included"
    );

    let contains = query(
        &mut server,
        json!([{"filter": [{"col": "name", "op": "contains", "value": "^A"}]}]),
    );
    assert_eq!(contains["row_count"], 1, "only Alice Johnson starts with A");
}

#[test]
fn a_filter_on_an_unknown_column_names_the_ones_that_exist() {
    let mut server = Server::new();
    let message = call_expecting_failure(
        &mut server,
        "tuitab_query",
        json!({"source": "test_data/sample.csv",
               "ops": [{"filter": [{"col": "salary_usd", "op": "gt", "value": 1}]}]}),
    );
    assert!(message.contains("salary_usd"), "{}", message);
    assert!(
        message.contains("department"),
        "must list what is available: {}",
        message
    );
}

#[test]
fn sort_orders_rows_and_composes_with_filter() {
    let mut server = Server::new();
    let result = query(
        &mut server,
        json!([
            {"filter": [{"col": "department", "op": "eq", "value": "HR"}]},
            {"sort": {"col": "age", "desc": true}}
        ]),
    );
    // HR ages are 25, 29, 26 and 27.
    assert_eq!(result["row_count"], 4);
    assert_eq!(cell(&result, 0, "age"), json!(29));
    assert_eq!(cell(&result, 3, "age"), json!(25));
}

// ── aggregation ─────────────────────────────────────────────────────────────

#[test]
fn group_by_returns_exactly_the_requested_aggregates() {
    let mut server = Server::new();
    let result = query(
        &mut server,
        json!([{"group_by": {"by": ["department"],
                             "agg": [{"col": "salary", "fn": "sum"},
                                     {"col": "*", "fn": "count"}]}}]),
    );

    let columns: Vec<&str> = result["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        columns,
        vec!["department", "salary:sum", "count"],
        "no Count, Pct or Bar columns anyone did not ask for"
    );

    // group_by_stable keeps first-appearance order: Engineering, Management,
    // Marketing, HR.
    assert_eq!(cell(&result, 0, "department"), json!("Engineering"));
    assert_eq!(cell(&result, 0, "salary:sum"), json!(563001.5));
    assert_eq!(cell(&result, 0, "count"), json!(7));
    assert_eq!(cell(&result, 1, "department"), json!("Management"));
    assert_eq!(cell(&result, 1, "salary:sum"), json!(480000.5));
}

/// `preserves_col_type` hands an average the source column's type, which is
/// right for TUI formatting and wrong as a label on a fractional number.
#[test]
fn an_average_over_integers_is_reported_as_a_float() {
    let mut server = Server::new();
    let result = query(
        &mut server,
        json!([{"group_by": {"by": ["department"], "agg": [{"col": "age", "fn": "avg"}]}}]),
    );

    let age_avg = result["columns"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "age:avg")
        .unwrap();
    assert_eq!(age_avg["type"], "float");

    // Engineering ages 30, 28, 42, 52, 33, 37, 34 sum to 256 over 7 people.
    assert_eq!(cell(&result, 0, "age:avg"), json!(256.0 / 7.0));
}

#[test]
fn group_by_after_filter_aggregates_only_the_surviving_rows() {
    let mut server = Server::new();
    let result = query(
        &mut server,
        json!([
            {"filter": [{"col": "age", "op": "gt", "value": 40}]},
            {"group_by": {"by": ["department"], "agg": [{"col": "*", "fn": "count"}]}}
        ]),
    );
    // Of the seven over-40s: Management 4 (45, 47, 50, 41), Engineering 2
    // (42, 52), Marketing 1 (44).
    let total: i64 = result["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r[1].as_i64().unwrap())
        .sum();
    assert_eq!(total, 7);
}

/// build_frequency_table silently drops an incompatible aggregator
/// (dataframe.rs:1445).  That would hand the model a short answer with no way
/// to notice, so the MCP layer refuses instead.
#[test]
fn an_aggregate_that_cannot_apply_is_an_error_not_a_silent_omission() {
    let mut server = Server::new();
    let message = call_expecting_failure(
        &mut server,
        "tuitab_query",
        json!({"source": "test_data/sample.csv",
               "ops": [{"group_by": {"by": ["department"],
                                     "agg": [{"col": "name", "fn": "sum"}]}}]}),
    );
    assert!(message.contains("name"), "{}", message);
    assert!(message.contains("sum"), "{}", message);
    assert!(message.contains("string"), "must say why: {}", message);
}

#[test]
fn frequency_ranks_by_count_and_carries_a_share_column() {
    let mut server = Server::new();
    let result = query(&mut server, json!([{"frequency": {"by": ["department"]}}]));

    let columns: Vec<&str> = result["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        columns,
        vec!["department", "Count", "Pct"],
        "the ASCII Bar column is dropped"
    );

    assert_eq!(result["row_count"], 4);
    assert_eq!(cell(&result, 0, "department"), json!("Engineering"));
    assert_eq!(cell(&result, 0, "Count"), json!(7));
    // Percentages are fractions, not "35%".
    assert_eq!(cell(&result, 0, "Pct"), json!(0.35));
}

#[test]
fn compute_adds_a_derived_column() {
    let mut server = Server::new();
    let result = query(
        &mut server,
        json!([
            {"compute": {"name": "decade", "expr": "age / 10"}},
            {"filter": [{"col": "department", "op": "eq", "value": "HR"}]},
            {"sort": {"col": "age", "desc": false}}
        ]),
    );
    // The youngest HR person is 25.
    assert_eq!(cell(&result, 0, "decade"), json!(2.5));
}

// ── join ────────────────────────────────────────────────────────────────────

#[test]
fn join_brings_in_columns_from_a_second_file() {
    let mut server = Server::new();
    let result = query(
        &mut server,
        json!([
            {"join": {"source": {"path": "test_data/prices.csv"},
                      "left_on": ["id"], "how": "inner"}},
            {"sort": {"col": "id", "desc": false}}
        ]),
    );
    // prices.csv holds ids 1 and 2 only.
    assert_eq!(result["row_count"], 2);
    assert_eq!(cell(&result, 0, "value"), json!(100));
    assert_eq!(cell(&result, 1, "value"), json!(200));
}

// ── output shaping ──────────────────────────────────────────────────────────

#[test]
fn a_truncated_result_says_so_and_still_reports_the_real_count() {
    let mut server = Server::new();
    let result = call(
        &mut server,
        "tuitab_query",
        json!({"source": "test_data/sample.csv", "ops": [], "output": {"limit": 2}}),
    );

    assert_eq!(result["returned"], 2);
    assert_eq!(result["row_count"], 20);
    assert_eq!(result["truncated"], true);
    assert!(result["note"].as_str().unwrap().contains("output.path"));
}

#[test]
fn several_pipelines_run_against_one_load() {
    let mut server = Server::new();
    let result = call(
        &mut server,
        "tuitab_query",
        json!({"source": "test_data/sample.csv", "pipelines": [
            {"name": "headcount", "ops": [{"frequency": {"by": ["department"]}}]},
            {"name": "seniors", "ops": [{"filter": [{"col": "age", "op": "ge", "value": 45}]}]}
        ]}),
    );

    let results = result["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["name"], "headcount");
    assert_eq!(results[0]["row_count"], 4);
    assert_eq!(results[1]["name"], "seniors");
    assert_eq!(results[1]["row_count"], 4, "ages 45, 52, 47 and 50");
}

#[test]
fn output_path_writes_a_file_and_refuses_to_clobber_one() {
    let mut server = Server::new();
    let path = tmp("headcount.csv");
    let _ = std::fs::remove_file(&path);

    let ops = json!([{"frequency": {"by": ["department"]}}]);
    let result = call(
        &mut server,
        "tuitab_query",
        json!({"source": "test_data/sample.csv", "ops": ops,
               "output": {"path": path.to_string_lossy()}}),
    );

    assert_eq!(result["row_count"], 4);
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("Engineering"), "{}", written);

    // A second run must not quietly replace it.
    let message = call_expecting_failure(
        &mut server,
        "tuitab_query",
        json!({"source": "test_data/sample.csv", "ops": ops,
               "output": {"path": path.to_string_lossy()}}),
    );
    assert!(message.contains("already exists"), "{}", message);

    // Unless asked to.
    call(
        &mut server,
        "tuitab_query",
        json!({"source": "test_data/sample.csv", "ops": ops,
               "output": {"path": path.to_string_lossy(), "overwrite": true}}),
    );

    std::fs::remove_file(&path).unwrap();
}

// ── describe and jq ─────────────────────────────────────────────────────────

#[test]
fn describe_labels_the_population_standard_deviation_unambiguously() {
    let mut server = Server::new();
    let result = call(
        &mut server,
        "tuitab_describe",
        json!({"source": "test_data/sample.csv", "columns": ["age"]}),
    );

    let metrics: Vec<&str> = result["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r[0].as_str().unwrap())
        .collect();
    assert!(metrics.contains(&"stdev_pop"), "{:?}", metrics);
    assert!(
        !metrics.contains(&"stdev"),
        "the ambiguous label is gone: {:?}",
        metrics
    );

    let mean = result["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r[0] == "mean")
        .unwrap();
    assert_eq!(mean[1], "36.65");
}

#[test]
fn jq_walks_a_nested_document() {
    let mut server = Server::new();
    let result = call(
        &mut server,
        "tuitab_jq",
        json!({"source": "test_data/nested.json", "program": "map(.name)"}),
    );
    assert_eq!(result["result"], json!(["alpha", "beta"]));
}

#[test]
fn jq_on_a_csv_points_at_the_right_tool() {
    let mut server = Server::new();
    let message = call_expecting_failure(
        &mut server,
        "tuitab_jq",
        json!({"source": "test_data/sample.csv", "program": "."}),
    );
    assert!(message.contains("tuitab_query"), "{}", message);
}

// ── the wire ────────────────────────────────────────────────────────────────

/// The one test that pays for a process: it checks the newline framing and that
/// nothing but protocol messages reaches stdout.
#[test]
fn the_binary_speaks_the_protocol_over_stdio() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(env!("CARGO_BIN_EXE_tuitab"))
        .arg("--mcp")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let script = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"tuitab_inspect","arguments":{"source":"test_data/sample.csv"}}}"#,
    ]
    .join("\n");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("{}\n", script).as_bytes())
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "server exited with {}",
        output.status
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();

    // Three requests, three replies — the notification gets none.
    assert_eq!(lines.len(), 3, "unexpected stdout:\n{}", stdout);

    for line in &lines {
        let value: Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("stdout line is not a message: {} ({})", line, e));
        assert_eq!(value["jsonrpc"], "2.0");
    }

    let ids: Vec<u64> = lines
        .iter()
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["id"]
                .as_u64()
                .unwrap()
        })
        .collect();
    assert_eq!(ids, vec![1, 2, 3]);
}
