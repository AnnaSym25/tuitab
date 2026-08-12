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
csv, tsv, txt, parquet, arrow/feather/ipc, xlsx/xls, sqlite, duckdb, json, jsonl, \
yaml, toml, and a directory path (which lists its files). For xlsx, sqlite and duckdb, pass \
'container' to pick a sheet or table; tuitab_inspect lists them with their row and \
column counts, and for a database also the CREATE statement and each column's \
declared SQL type, NOT NULL, PRIMARY KEY and DEFAULT. Views are listed and readable; \
they cannot be written to. A database source always needs a 'container' — without one \
there is nothing to read.

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

NOT SUPPORTED — do not attempt: subqueries, self-joins, and SQL of any kind. You \
never write SQL here. When a table has to change, tuitab composes the SQL for you \
and shows it to you before anything runs.

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
.xlsx, .csv, .tsv, .parquet, .arrow, .json, .jsonl, .yaml, .toml, .sqlite, .duckdb. \
That file is formatted for a person to read. Use it whenever the user wants a \
deliverable, or when a result is too big to return. For .sqlite and .duckdb, \
output.table names the table (default 'result'). Writing into a database file that \
already exists changes data the user already has, so it needs the server to have been \
started with --mcp-write; a database file that does not exist yet is created without \
it. Replacing a table that is already there additionally needs output.overwrite, and \
the file a query read cannot be the file it writes.

NESTED DATA
tuitab_query flattens JSON/YAML/TOML into a table. When the structure is deeper \
than that survives, use tuitab_jq with a jq program instead.";

/// The half of the documentation that only exists when writing is allowed.
pub const WRITE_INSTRUCTIONS: &str = "\
CHANGING A DATABASE TABLE
This server was started with writing enabled, so two more tools exist. They work on \
.sqlite and .duckdb sources with a 'container', and on tables only — never views.

1. tuitab_write says what would happen. It writes nothing. Give it one of:
     {\"set\": {\"status\": \"archived\"}, \"where\": [{\"col\":\"id\",\"op\":\"in\",\"value\":[3,7,11]}]}
     {\"delete\": true, \"where\": [{\"col\":\"state\",\"op\":\"eq\",\"value\":\"draft\"}]}
     {\"insert\": [{\"name\":\"ann\",\"score\":10}]}
     {\"alter\": {\"add\": [{\"name\":\"tier\",\"type\":\"text\"}], \"drop\": [\"old\"], \"rename\": {\"nm\":\"name\"}}}
   'where' takes the same predicates as tuitab_query's filter, and omitting it on \
   'set' changes every row — the plan tells you how many that is. 'delete' always \
   needs a 'where'. A JSON null writes a real NULL. Columns left out of an insert are \
   written as NULL, not as the column's DEFAULT.
   It answers with the exact statements, how many rows they touch, those rows as they \
   stand now, and a plan_id. Long plans are cut short: 'statements' holds the first \
   twenty, 'statements_total' and 'statements_not_shown' say how many there are, and \
   'show_statements' asks for more (up to 200) when the user wants to see them. \
   'warnings' names anything the statements do not spell out.
2. Read the statements and the affected rows back to the user, with the counts when \
   the list was cut. This is the only confirmation there is.
3. tuitab_write_apply with that plan_id runs exactly those statements, all of them or \
   none. If the table changed since step 1, nothing is written and you start over.

One change per call, and calling tuitab_write again discards the previous plan. Raw SQL \
is still not available. alter adds, drops and renames columns; changing an existing \
column's type and reordering columns are only in the terminal.";

/// The documentation the model gets, with the writing half only when it applies.
///
/// Telling a model about a tool it cannot call wastes tokens and invites failed calls.
pub fn instructions(write: bool) -> String {
    if write {
        format!("{}\n\n{}", INSTRUCTIONS, WRITE_INSTRUCTIONS)
    } else {
        INSTRUCTIONS.to_string()
    }
}

/// The `tools/list` payload.
pub fn definitions(write: bool) -> Vec<Value> {
    let source_schema = json!({
        "type": "object",
        "description": "The file to read. A bare path string also works.",
        "properties": {
            "path": {"type": "string", "description": "Path to the file, or a directory to list."},
            "container": {
                "type": "string",
                "description": "Sheet name (Excel), or table or view name (SQLite/DuckDB). A database source needs one. Call tuitab_inspect to see what is available."
            },
            "delimiter": {
                "type": "string",
                "description": "Single character overriding CSV delimiter auto-detection."
            },
            "format": {
                "type": "string",
                "description": "Overrides the file extension, e.g. read a .conf file as 'yaml'.",
                "enum": ["csv", "tsv", "json", "jsonl", "ndjson", "yaml", "yml", "toml",
                         "sqlite", "sqlite3", "db", "duckdb", "ddb", "xlsx", "xls",
                         "parquet", "arrow"]
            }
        },
        "required": ["path"]
    });

    let mut tools = vec![
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
                                     picks the format: .xlsx, .csv, .tsv, .parquet, .arrow, .json, \
                                     .jsonl, .yaml, .toml, .sqlite."
                            },
                            "table": {
                                "type": "string",
                                "description":
                                    "Name for the table this creates in a .sqlite/.duckdb file \
                                     (default 'result'). Also the top-level key for wrapped \
                                     JSON/YAML/TOML."
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
    ];

    if write {
        tools.push(write_tool_schema(&source_schema));
        tools.push(apply_tool_schema());
    }

    tools
}

/// Phase one: says what would happen, and is genuinely read-only.
fn write_tool_schema(source_schema: &Value) -> Value {
    json!({
        "name": "tuitab_write",
        "title": "Plan a change to a database table",
        "description":
            "Work out what changing a database table would do, and return the exact SQL —              nothing is written. Only .sqlite/.duckdb sources with a 'container', and only              tables, never views. Give one of 'set', 'delete', 'insert' or 'alter' per call.              Read the statements and the affected rows back to the user, then call              tuitab_write_apply with the returned plan_id to run exactly that plan.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "source": source_schema,
                "set": {
                    "type": "object",
                    "description":
                        "Column name to new value, for every row 'where' matches. A JSON null                          writes a real NULL. Several columns become one UPDATE."
                },
                "delete": {
                    "type": "boolean",
                    "description": "Delete the rows 'where' matches. 'where' is required."
                },
                "insert": {
                    "type": "array",
                    "description":
                        "New rows, each an object of column name to value. Columns left out are                          written as NULL, not as the column's DEFAULT.",
                    "items": {"type": "object"},
                    "maxItems": 1000
                },
                "alter": {
                    "type": "object",
                    "description":
                        "Change the table's shape. Types are string, integer, float, boolean,                          date, datetime.",
                    "properties": {
                        "add": {"type": "array", "items": {"type": "object"}},
                        "drop": {"type": "array", "items": {"type": "string"}},
                        "rename": {"type": "object"}
                    }
                },
                "where": {
                    "type": "array",
                    "description":
                        "Which rows to change, in the same shape tuitab_query's 'filter' takes.                          Omit it on 'set' to change every row — the plan says how many that is.                          Ignored by 'insert' and 'alter'.",
                    "items": {"type": "object"}
                },
                "preview_rows": {
                    "type": "integer",
                    "description": "Affected rows to show back (default 10).",
                    "minimum": 0,
                    "maximum": 100
                },
                "show_statements": {
                    "type": "integer",
                    "description":
                        "How many statements to return (default 20). Raise it when the user asks to see all of them; the reply always says how many were not shown.",
                    "minimum": 1,
                    "maximum": 200
                }
            },
            "required": ["source"]
        },
        "annotations": {"readOnlyHint": true, "openWorldHint": false}
    })
}

/// Phase two: destructive, and annotated as such so a client can gate it.
fn apply_tool_schema() -> Value {
    json!({
        "name": "tuitab_write_apply",
        "title": "Run a plan from tuitab_write",
        "description":
            "Execute exactly the statements tuitab_write returned, in one transaction. The plan              is checked against the database first: if a row changed since the plan was made,              nothing is written.",
        "inputSchema": {
            "type": "object",
            "properties": {"plan_id": {"type": "string"}},
            "required": ["plan_id"]
        },
        "annotations": {
            "readOnlyHint": false,
            "destructiveHint": true,
            "idempotentHint": false,
            "openWorldHint": false
        }
    })
}

pub fn call(server: &mut Server, name: &str, args: &Value) -> Result<Value, CallError> {
    match name {
        "tuitab_inspect" => inspect(server, args),
        "tuitab_query" => query(server, args),
        "tuitab_describe" => describe_tool(server, args),
        "tuitab_jq" => jq(args),
        // Named but unavailable: a model that learned the name elsewhere deserves the
        // reason rather than a bafflement about an unknown tool.
        "tuitab_write" | "tuitab_write_apply" if !server.write => Err(CallError::Failed(format!(
            "{} is off. Restart the server with --mcp-write to allow it to change rows.",
            name
        ))),
        "tuitab_write" => super::write::plan(server, args),
        "tuitab_write_apply" => super::write::apply(server, args),
        other => Err(CallError::UnknownTool(other.to_string())),
    }
}

/// The `where` of a write: the same predicate grammar `filter` takes in a pipeline, so
/// a model writes what it already knows.
pub fn where_arg(args: &Value) -> Result<Vec<crate::data::filter::Clause>, CallError> {
    match args.get("where") {
        None => Ok(Vec::new()),
        Some(v) => pipeline::parse_predicates(v).map_err(CallError::Failed),
    }
}

pub fn source_arg(args: &Value) -> Result<source::Source, CallError> {
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
    if let Some(listed) = &containers {
        if src.container.is_none() {
            let tables = listed.iter().filter(|c| !c.view).count();
            let views = listed.len() - tables;
            return Ok(json!({
                "path": src.path.to_string_lossy(),
                "containers": containers_json(listed),
                "note": format!(
                    "This file holds {} table(s){}. Call tuitab_inspect again with \
                     'container' set to one of them to see its columns.",
                    tables,
                    if views > 0 {
                        format!(" and {} view(s)", views)
                    } else {
                        String::new()
                    }
                ),
            }));
        }
    }

    // A database container is loaded through the path that keeps its declared types —
    // the ones a model actually needs, since every value arrives as text and the
    // inferred type is a guess made over that text.
    let is_db = crate::data::io::db_write::is_db_ext(&src.path);
    let (df, table_source) = match (&src.container, is_db) {
        (Some(container), true) => source::load_db_table(&src.path, container)?,
        _ => (source::load(server, &src)?, None),
    };
    let table = render::table(&df, sample)?;

    let mut out = json!({
        "path": src.path.to_string_lossy(),
        "columns": columns_json_with_schema(&df, table_source.as_ref()),
        "row_count": df.row_order.len(),
        "sample_rows": table.get("rows").cloned().unwrap_or(Value::Array(vec![])),
    });
    if let Some(obj) = out.as_object_mut() {
        if let Some(listed) = &containers {
            obj.insert("containers".into(), containers_json(listed));
            obj.insert("container".into(), json!(src.container));
            if let Some(here) = listed
                .iter()
                .find(|c| Some(&c.name) == src.container.as_ref())
            {
                if let Some(sql) = &here.sql {
                    obj.insert("create_sql".into(), json!(sql));
                }
            }
        }
        if is_db && src.container.is_some() {
            obj.insert("writable".into(), json!(table_source.is_some()));
            if table_source.is_none() {
                obj.insert(
                    "note".into(),
                    json!("This is a view: it can be read but not written to."),
                );
            }
        }
    }
    Ok(out)
}

fn containers_json(listed: &[crate::data::io::ContainerInfo]) -> Value {
    Value::Array(
        listed
            .iter()
            .map(|c| {
                json!({
                    "name": c.name,
                    "kind": if c.view { "view" } else { "table" },
                    "rows": c.rows,
                    "columns": c.columns,
                    "create_sql": c.sql,
                })
            })
            .collect(),
    )
}

/// Columns, with what the database declares them to be when there is a database.
///
/// The inferred `type` is what tuitab worked out; `declared` is what the table says,
/// and for anything the loader could not turn into a number the two disagree on
/// purpose — a column declared INTEGER that holds text reads as text.
fn columns_json_with_schema(
    df: &crate::data::dataframe::DataFrame,
    src: Option<&crate::data::io::db_write::TableSource>,
) -> Value {
    let mut cols = render::columns_json(df);
    let Some(src) = src else {
        return Value::Array(cols);
    };
    for (entry, col) in cols.iter_mut().zip(&src.columns) {
        if let Some(obj) = entry.as_object_mut() {
            obj.insert("declared".into(), json!(col.decl_raw));
            obj.insert("not_null".into(), json!(col.notnull));
            obj.insert("primary_key".into(), json!(col.pk));
            obj.insert("default".into(), json!(col.default_sql));
            obj.insert("generated".into(), json!(col.generated));
        }
    }
    Value::Array(cols)
}

// ── tuitab_query ────────────────────────────────────────────────────────────

struct Output {
    limit: usize,
    path: Option<std::path::PathBuf>,
    /// Name for the table a database output creates.
    table: Option<String>,
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
        // `~` is the shell's job everywhere else; nothing expands it here, and without
        // this `~/out.csv` quietly creates a directory called `~`.
        path: output
            .and_then(|o| o.get("path"))
            .and_then(Value::as_str)
            .map(crate::app::expand_tilde),
        // Trimmed, because `validate_table_name` validates the trimmed form: without
        // this `"  x  "` passes the check and creates a table with the spaces in it.
        table: output
            .and_then(|o| o.get("table"))
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string()),
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

    // Everything about the destination that can be known without the rows, checked
    // before the rows are computed: an unwritable extension or a missing directory used
    // to surface only after the joins and group-bys had run.
    //
    // Not checked, deliberately: where the path leads.  The person who started this
    // server chose what it can reach, and a root it cannot escape would be a pretence of
    // isolation in a process that has none.
    if let Some(path) = &output.path {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        if !crate::data::io::writable_ext(ext) {
            return Err(CallError::Failed(format!(
                "There is no writer for '.{}'. Use .csv, .tsv, .json, .jsonl, .yaml, \
                 .toml, .parquet, .arrow, .xlsx, .sqlite or .duckdb.",
                ext
            )));
        }
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            if !dir.is_dir() {
                return Err(CallError::Failed(format!(
                    "There is no directory {} to write into.",
                    dir.display()
                )));
            }
        }
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
            Some(path) => {
                // The cached frame may be of the file just written — an inspect right
                // after would answer from a snapshot taken before the write.
                server.cache.clear();
                write_result(
                    &df,
                    path,
                    output.table.as_deref(),
                    output.overwrite,
                    server.write,
                    &src.path,
                )?
            }
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
    table: Option<&str>,
    overwrite: bool,
    write_enabled: bool,
    source: &std::path::Path,
) -> Result<Value, CallError> {
    use crate::data::io::db_write;

    let table = table.unwrap_or("result");
    if let Err(why) = db_write::validate_table_name(table) {
        return Err(CallError::Failed(format!("output.table: {}", why)));
    }

    if db_write::is_db_ext(path) {
        let kind = db_write::kind_for_path(path);

        // Writing into the file the pipeline just read is not an export, it is an edit
        // of the source — and this path has no plan to show and no drift check.
        if db_write::same_file(source, path) {
            return Err(CallError::Failed(format!(
                "{} is the source this query read. Write the result to a different file.",
                path.display()
            )));
        }

        // A database that already exists holds somebody's data, and writing into it is
        // the thing --mcp-write gates.  A file that does not exist yet has nothing to
        // lose, which is the same rule the terminal uses to skip its own confirmation.
        if path.exists() && !write_enabled {
            return Err(CallError::Failed(format!(
                "{} already exists, and changing a database the user already has needs \
                 the server to be started with --mcp-write. Write to a new file instead.",
                path.display()
            )));
        }

        // For a database the unit at stake is the table, not the file: adding one beside
        // what is already there loses nothing, so only replacing one needs permission.
        if db_write::table_exists(kind, path, table) {
            if !overwrite {
                return Err(CallError::Failed(format!(
                    "'{}' already exists in {}. Pass output.overwrite to replace it, or \
                     choose another table name.",
                    table,
                    path.display()
                )));
            }
            if db_write::is_view(kind, path, table) {
                return Err(CallError::Failed(format!(
                    "'{}' is a view in {}, not a table, and cannot be replaced by one. \
                     Choose another table name.",
                    table,
                    path.display()
                )));
            }
        }
    } else if path.exists() && !overwrite {
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
        table,
    )
    .map_err(|e| CallError::Failed(format!("Could not write {}: {}", path.display(), e)))?;

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
        .map_err(|e| CallError::Failed(format!("The jq program failed: {}", e)))?;

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
    .map_err(|e| CallError::Failed(format!("Could not render the jq result: {}", e)))?;

    let value: Value = serde_json::from_str(&text)
        .map_err(|e| CallError::Failed(format!("jq result was not valid JSON: {}", e)))?;

    Ok(json!({"result": value}))
}
