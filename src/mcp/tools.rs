//! Tool definitions and dispatch.
//!
//! Four tools, each a whole step of the model's working loop rather than a
//! mirror of a tuitab keybinding: find out what is in the file, compute over it,
//! profile it, or walk a nested document.

use super::{pipeline, render, source, Server};
use crate::data::describe;
use serde_json::{json, Value};

/// Why a `tools/call` did not produce a payload.
pub enum CallError {
    /// The tool does not exist — a protocol-level mistake.
    UnknownTool(String),
    /// The tool ran and failed.  The model should see this and correct itself.
    Failed(String),
}

impl From<String> for CallError {
    fn from(message: String) -> Self {
        CallError::Failed(message)
    }
}

/// Text handed to the model alongside the tool list.  This is the documentation
/// that ships with the server, and how well it reads decides whether the tools
/// get used correctly.
pub const INSTRUCTIONS: &str = "\
tuitab computes over data files so you do not have to. Send it the operations; \
read back the numbers. Never calculate over a file yourself when you can call \
these tools — that is the entire point of them.

WORKFLOW
1. tuitab_inspect first, always. It tells you the real column names, their \
   inferred types, and how many rows there are. Guessing column names wastes a \
   round trip.
2. tuitab_query to compute. Operations run in order, each on the previous \
   result.
3. Explain the returned numbers to the user. Quote them; do not re-derive them.

FORMATS
csv, tsv, txt, parquet, xlsx/xls, sqlite, duckdb, json, jsonl, yaml, toml, and \
a directory path (which lists its files). For xlsx, sqlite and duckdb, pass \
'container' to pick a sheet or table; tuitab_inspect lists them.

OPERATIONS for tuitab_query
  {\"filter\": [{\"col\":\"region\",\"op\":\"eq\",\"value\":\"North\"}]}
      Entries are combined with AND. Operators: eq, ne, gt, ge, lt, le, in, \
      not_in, contains (regex, case-sensitive — write (?i) yourself), between \
      ([low, high]), is_empty, not_empty.
      For OR, wrap predicates in any_of — one entry, several alternatives:
        {\"filter\": [{\"any_of\": [{\"col\":\"region\",\"op\":\"eq\",\"value\":\"North\"},
                                {\"col\":\"region\",\"op\":\"eq\",\"value\":\"South\"}]},
                    {\"col\":\"amount\",\"op\":\"gt\",\"value\":1000}]}
      reads as (North OR South) AND amount > 1000. One level of nesting only.
      To compare two columns, give a column instead of a constant:
        {\"col\":\"revenue\",\"op\":\"gt\",\"value\":{\"col\":\"cost\"}}
  {\"select\": [\"a\",\"b\"]}          keep these columns, in this order
  {\"sort\": {\"col\":\"amount\",\"desc\":true}}
  {\"sort\": {\"by\":[{\"col\":\"region\"},{\"col\":\"amount\",\"desc\":true}]}}
      Use 'by' for several keys, first the most significant. Do NOT chain two \
      sort operations to get that — the second is free to reorder rows that tie \
      on its own key, so the first ordering is not preserved.
  {\"compute\": {\"name\":\"margin\",\"expr\":\"revenue - cost\"}}
      Expression syntax: arithmetic, comparisons, and/or/not, if(cond, a, b), \
      concat, substring, len, contains(col, regex), year/month/day, \
      date_format, and 'x in (a, b)'. 'or' binds loosest, then 'and', then \
      'not'.
  {\"group_by\": {\"by\":[\"region\"],\"agg\":[{\"col\":\"amount\",\"fn\":\"sum\"}]}}
      Exactly the aggregates you ask for. Result columns are named 'col:fn'. \
      Use {\"col\":\"*\",\"fn\":\"count\"} for a row count.
  {\"aggregate\": [{\"col\":\"amount\",\"fn\":\"sum\"},{\"col\":\"*\",\"fn\":\"count\"}]}
      A grand total over every remaining row — one row out, no grouping.
  {\"frequency\": {\"by\":[\"product\"]}}
      Distribution ranked by count, descending, with Count and Pct columns. Use \
      this for 'how many of each' / 'most common'; use group_by when you want \
      specific aggregates and your own ordering.
  {\"pivot\": {\"index\":[\"region\"],\"on\":\"quarter\",\"formula\":\"sum(amount)\"}}
  {\"join\": {\"source\":{\"path\":\"prices.csv\"},\"left_on\":[\"id\"],\"how\":\"left\"}}
      how: inner, left, right, outer. 'right_on' defaults to 'left_on'.
  {\"dedup\": {\"by\":[\"id\"],\"keep\":\"first\"}}
      keep: first, last, min, max (both need \"on\": column), random (pass \
      \"seed\" to repeat the same choice).
  {\"duplicates\": {\"by\":[\"email\"]}}   keep only rows whose key repeats
  {\"window\": {\"fn\":\"rank\",\"col\":\"salary\",\"over\":[\"department\"],\"desc\":true}}
      fn: row_number, rank, dense_rank, cum_sum, lag, lead, sum, avg, min, \
      max, count, pct_of_total. 'over' restarts the window per group; without \
      it the window is the whole table. 'as' names the new column. \
      IMPORTANT: cum_sum, lag, lead and row_number read the rows in their \
      current order — sort first, or a running total accumulates in load order.
  {\"sample\": {\"n\":100,\"seed\":42}}   keep n rows at random; the seed makes it repeatable
  {\"transpose\": {}}            stand the table on end; {\"row\": 3} for one row
  {\"limit\": 20}
Aggregate functions: count, distinct, sum, avg, min, max, median, stdev, p5, \
p25, p50, p75, p95.
To filter groups (SQL's HAVING), put a filter after group_by.

NOT SUPPORTED — do not attempt: subqueries, self-joins, and SQL of any kind.

PERCENT OF TOTAL
Use window with 'over' for a share within a group; use compute with \
'amount / sum(amount)' for a share of the whole table.

READING RESULTS
Rows come back as arrays matching the 'columns' list. Percentages are \
fractions: 0.42 means 42%. Currency and float values are raw numbers, not \
formatted strings. Always check 'truncated' — when it is true you are seeing \
part of the answer.

LARGE OR SHAREABLE RESULTS
Set output.path to write the result to a file instead of returning rows: \
.xlsx, .csv, .parquet, .json, .yaml, .toml, .sqlite. That file is formatted for \
a person to read. Use it whenever the user wants a deliverable, or when a \
result is too big to return.

NESTED DATA
tuitab_query flattens JSON/YAML/TOML into a table. When the structure is deeper \
than that survives, use tuitab_jq with a jq program instead.";

/// The `tools/list` payload.
pub fn definitions() -> Vec<Value> {
    let source_schema = json!({
        "type": "object",
        "description": "The file to read. A bare path string also works.",
        "properties": {
            "path": {"type": "string", "description": "Path to the file, or a directory to list."},
            "container": {
                "type": "string",
                "description": "Sheet name (Excel) or table name (SQLite/DuckDB). Call tuitab_inspect to see what is available."
            },
            "delimiter": {
                "type": "string",
                "description": "Single character overriding CSV delimiter auto-detection."
            },
            "format": {
                "type": "string",
                "description": "Overrides the file extension, e.g. read a .conf file as 'yaml'.",
                "enum": ["csv", "tsv", "json", "jsonl", "ndjson", "yaml", "yml", "toml"]
            }
        },
        "required": ["path"]
    });

    vec![
        json!({
            "name": "tuitab_inspect",
            "title": "Inspect a data file",
            "description":
                "Read a file's structure: its sheets or tables, its column names with the types \
                 tuitab inferred, its row count, and a few sample rows. Call this before \
                 tuitab_query so you use real column names instead of guessing.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": source_schema,
                    "sample_rows": {
                        "type": "integer",
                        "description": "Sample rows to return (default 5).",
                        "minimum": 0,
                        "maximum": 100
                    }
                },
                "required": ["source"]
            },
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        }),
        json!({
            "name": "tuitab_query",
            "title": "Compute over a data file",
            "description":
                "Run one or more pipelines of operations over a file and return the results. \
                 Operations apply in order: filter, select, sort, compute, group_by, aggregate, \
                 frequency, pivot, join, limit. Several pipelines in one call share a single load of the \
                 file. Use this for every calculation over tabular data — the numbers it returns \
                 are computed, not estimated.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": source_schema,
                    "ops": {
                        "type": "array",
                        "description": "Operations for a single pipeline. Use this or 'pipelines', not both.",
                        "items": {"type": "object"}
                    },
                    "pipelines": {
                        "type": "array",
                        "description": "Several independent pipelines, each starting from the source.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string", "description": "Label echoed back with this pipeline's result."},
                                "ops": {"type": "array", "items": {"type": "object"}}
                            },
                            "required": ["ops"]
                        }
                    },
                    "output": {
                        "type": "object",
                        "properties": {
                            "limit": {
                                "type": "integer",
                                "description": "Maximum rows to return (default 100).",
                                "minimum": 1
                            },
                            "path": {
                                "type": "string",
                                "description":
                                    "Write the result here instead of returning rows. The extension \
                                     picks the format: .xlsx, .csv, .tsv, .parquet, .json, .jsonl, \
                                     .yaml, .toml, .sqlite."
                            },
                            "overwrite": {
                                "type": "boolean",
                                "description": "Allow replacing an existing file at 'path' (default false)."
                            }
                        }
                    }
                },
                "required": ["source"]
            }
        }),
        json!({
            "name": "tuitab_describe",
            "title": "Profile every column",
            "description":
                "Statistical profile of each column: type, count, nulls, unique, min, max, mean, \
                 median, mode, stdev_pop (population standard deviation), range, and the 5th, \
                 25th, 50th, 75th and 95th percentiles. Use it to understand a dataset before \
                 querying it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": source_schema,
                    "columns": {
                        "type": "array",
                        "description": "Restrict the profile to these columns. Omit for all of them.",
                        "items": {"type": "string"}
                    }
                },
                "required": ["source"]
            },
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        }),
        json!({
            "name": "tuitab_jq",
            "title": "Query a nested document with jq",
            "description":
                "Run a jq program over a JSON, JSONL, YAML or TOML file and return the result as \
                 JSON. Use this when the structure is too nested for a table — otherwise prefer \
                 tuitab_query.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": source_schema,
                    "program": {"type": "string", "description": "A jq program, e.g. '.items | map(.price) | add'."}
                },
                "required": ["source", "program"]
            },
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        }),
    ]
}

pub fn call(server: &mut Server, name: &str, args: &Value) -> Result<Value, CallError> {
    match name {
        "tuitab_inspect" => inspect(server, args),
        "tuitab_query" => query(server, args),
        "tuitab_describe" => describe_tool(server, args),
        "tuitab_jq" => jq(args),
        other => Err(CallError::UnknownTool(other.to_string())),
    }
}

fn source_arg(args: &Value) -> Result<source::Source, CallError> {
    let value = args
        .get("source")
        .ok_or_else(|| CallError::Failed("missing required argument 'source'".to_string()))?;
    Ok(source::Source::from_json(value)?)
}

// ── tuitab_inspect ──────────────────────────────────────────────────────────

fn inspect(server: &mut Server, args: &Value) -> Result<Value, CallError> {
    let src = source_arg(args)?;
    let sample = args
        .get("sample_rows")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .min(100) as usize;

    let ext = source::extension_of(&src);
    let containers = source::containers(&src.path, &ext);

    // A workbook or database opened without a container gives an overview sheet
    // listing what is inside, not data — say so rather than presenting the
    // listing as if it were the table.
    if let Some(names) = &containers {
        if src.container.is_none() {
            return Ok(json!({
                "path": src.path.to_string_lossy(),
                "containers": names,
                "note": format!(
                    "This file holds {} tables. Call tuitab_inspect again with \
                     'container' set to one of them to see its columns.",
                    names.len()
                ),
            }));
        }
    }

    let df = source::load(server, &src)?;
    let table = render::table(&df, sample)?;

    let mut out = json!({
        "path": src.path.to_string_lossy(),
        "columns": render::columns_json(&df),
        "row_count": df.row_order.len(),
        "sample_rows": table.get("rows").cloned().unwrap_or(Value::Array(vec![])),
    });
    if let (Some(names), Some(obj)) = (containers, out.as_object_mut()) {
        obj.insert("containers".into(), json!(names));
        obj.insert("container".into(), json!(src.container));
    }
    Ok(out)
}

// ── tuitab_query ────────────────────────────────────────────────────────────

struct Output {
    limit: usize,
    path: Option<std::path::PathBuf>,
    overwrite: bool,
}

fn output_arg(args: &Value) -> Output {
    let output = args.get("output");
    Output {
        limit: output
            .and_then(|o| o.get("limit"))
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .unwrap_or(render::DEFAULT_LIMIT),
        path: output
            .and_then(|o| o.get("path"))
            .and_then(Value::as_str)
            .map(std::path::PathBuf::from),
        overwrite: output
            .and_then(|o| o.get("overwrite"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

fn query(server: &mut Server, args: &Value) -> Result<Value, CallError> {
    let src = source_arg(args)?;
    let output = output_arg(args);

    // Accept a single `ops` array as well as `pipelines` — a model asking one
    // question should not have to wrap it in a list.
    let pipelines: Vec<(Option<String>, Vec<pipeline::Op>)> = match args.get("pipelines") {
        Some(Value::Array(items)) => {
            let mut parsed = Vec::with_capacity(items.len());
            for (i, item) in items.iter().enumerate() {
                let name = item.get("name").and_then(Value::as_str).map(str::to_string);
                let ops = pipeline::parse_ops(item.get("ops").unwrap_or(&Value::Null))
                    .map_err(|e| CallError::Failed(format!("pipelines[{}]: {}", i, e)))?;
                parsed.push((name, ops));
            }
            parsed
        }
        _ => match args.get("ops") {
            Some(ops) => vec![(None, pipeline::parse_ops(ops)?)],
            None => {
                return Err(CallError::Failed(
                    "provide either 'ops' or 'pipelines'".to_string(),
                ))
            }
        },
    };

    if pipelines.is_empty() {
        return Err(CallError::Failed("no pipelines to run".to_string()));
    }

    if output.path.is_some() && pipelines.len() > 1 {
        return Err(CallError::Failed(
            "'output.path' writes one file, so it cannot be combined with several pipelines"
                .to_string(),
        ));
    }

    let base = source::load(server, &src)?;
    let mut results = Vec::with_capacity(pipelines.len());

    for (index, (name, ops)) in pipelines.iter().enumerate() {
        let label = name
            .clone()
            .unwrap_or_else(|| format!("pipeline_{}", index));
        let (df, seeds) = pipeline::apply_all_reporting_seeds(base.clone(), ops)
            .map_err(|e| CallError::Failed(format!("{}: {}", label, e)))?;

        let mut entry = match &output.path {
            Some(path) => write_result(&df, path, output.overwrite)?,
            None => render::table(&df, output.limit)?,
        };
        if let (Some(name), Some(obj)) = (name, entry.as_object_mut()) {
            obj.insert("name".into(), json!(name));
        }
        // Any seed drawn for the caller, so the same random result can be had
        // again by passing it back.
        if !seeds.0.is_empty() {
            if let Some(obj) = entry.as_object_mut() {
                obj.insert(
                    "seeds".into(),
                    json!(seeds
                        .0
                        .iter()
                        .map(|(op, seed)| json!({"op": op, "seed": seed}))
                        .collect::<Vec<_>>()),
                );
            }
        }
        results.push(entry);
    }

    Ok(if results.len() == 1 {
        results.pop().expect("length checked")
    } else {
        json!({"results": results})
    })
}

/// Write a result to disk through tuitab's own saver.
///
/// This deliberately takes a different frame than [`render::table`] does: the
/// saver formats Currency, Percentage and Float columns for a reader, which is
/// right for a file someone opens and wrong for JSON a model computes over.
fn write_result(
    df: &crate::data::dataframe::DataFrame,
    path: &std::path::Path,
    overwrite: bool,
) -> Result<Value, CallError> {
    if path.exists() && !overwrite {
        return Err(CallError::Failed(format!(
            "{} already exists. Pass output.overwrite to replace it, or choose another path.",
            path.display()
        )));
    }

    crate::data::io::save_file_as(
        df,
        None,
        path,
        crate::data::io::doc_io::Shape::Records,
        "result",
    )
    .map_err(|e| CallError::Failed(e.to_string()))?;

    Ok(json!({
        "written": path.to_string_lossy(),
        "row_count": df.row_order.len(),
        "columns": render::columns_json(df),
        "note": "Values in the file are formatted for reading (currency symbols, fixed decimals).",
    }))
}

// ── tuitab_describe ─────────────────────────────────────────────────────────

fn describe_tool(server: &mut Server, args: &Value) -> Result<Value, CallError> {
    let src = source_arg(args)?;
    let mut df = source::load(server, &src)?;

    if let Some(Value::Array(names)) = args.get("columns") {
        let wanted: Vec<String> = names
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
        if !wanted.is_empty() {
            df = pipeline::apply_all(df, &[pipeline::Op::Select(wanted)])?;
        }
    }

    let mut profile = describe::describe(&df);

    // The shared function labels the row "stdev", which is ambiguous: this one
    // is the population figure, while the footer aggregator of the same name is
    // the sample one. Renaming it here keeps the TUI untouched.
    rename_metric(&mut profile, "stdev", "stdev_pop");

    render::table(&profile, describe::METRICS.len()).map_err(CallError::Failed)
}

fn rename_metric(df: &mut crate::data::dataframe::DataFrame, from: &str, to: &str) {
    let row = (0..df.visible_row_count()).find(|r| df.get_physical(df.row_order[*r], 0) == from);
    if let Some(row) = row {
        let _ = df.set_cell(df.row_order[row], 0, to.to_string());
    }
}

// ── tuitab_jq ───────────────────────────────────────────────────────────────

fn jq(args: &Value) -> Result<Value, CallError> {
    let src = source_arg(args)?;
    let program = args
        .get("program")
        .and_then(Value::as_str)
        .ok_or_else(|| CallError::Failed("missing required argument 'program'".to_string()))?;

    let doc = source::load_doc(&src)?;
    let result = crate::data::query::run_jq(&doc.root, program)
        .map_err(|e| CallError::Failed(e.to_string()))?;

    // Round-tripping through the document serialiser keeps this module out of
    // the business of mapping Node to serde_json by hand.
    let text = crate::data::doc::serialize(
        &result,
        crate::data::doc::Format::Json,
        false,
        &crate::data::doc::SaveOpts {
            indent: false,
            sort_keys: false,
        },
    )
    .map_err(|e| CallError::Failed(e.to_string()))?;

    let value: Value = serde_json::from_str(&text)
        .map_err(|e| CallError::Failed(format!("jq result was not valid JSON: {}", e)))?;

    Ok(json!({"result": value}))
}
