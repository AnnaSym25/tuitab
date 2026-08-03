//! Resolving a `source` argument into a loaded [`DataFrame`].
//!
//! Every tool takes the same source shape — a path, plus an optional container
//! for the formats that hold more than one table (Excel sheets, SQLite and
//! DuckDB tables).  Loading goes through [`crate::data::io`], so the MCP layer
//! inherits tuitab's format support and type inference rather than restating it.

use crate::data::dataframe::DataFrame;
use crate::data::doc::{Doc, Format};
use crate::data::io;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A source as the model described it.
#[derive(Clone, PartialEq, Eq)]
pub struct Source {
    pub path: PathBuf,
    /// Sheet name (Excel) or table name (SQLite/DuckDB).
    pub container: Option<String>,
    pub delimiter: Option<u8>,
    /// Overrides the extension, so a `.conf` file can be read as YAML.
    pub format: Option<String>,
}

impl Source {
    /// Read a source out of tool arguments.  Accepts either the object form or a
    /// bare string path, because a model that only needs a path will write one.
    pub fn from_json(value: &Value) -> Result<Self, String> {
        if let Some(path) = value.as_str() {
            return Ok(Self {
                path: PathBuf::from(path),
                container: None,
                delimiter: None,
                format: None,
            });
        }

        let obj = value.as_object().ok_or_else(|| {
            "'source' must be an object with a 'path', or a path string".to_string()
        })?;

        let path = obj
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "'source' requires a 'path'".to_string())?;

        let delimiter = match obj.get("delimiter").and_then(Value::as_str) {
            Some(d) => {
                let mut chars = d.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) if c.is_ascii() => Some(c as u8),
                    _ => {
                        return Err(format!(
                            "'delimiter' must be a single ASCII character, got {:?}",
                            d
                        ))
                    }
                }
            }
            None => None,
        };

        Ok(Self {
            path: PathBuf::from(path),
            container: obj
                .get("container")
                .and_then(Value::as_str)
                .map(str::to_string),
            delimiter,
            format: obj
                .get("format")
                .and_then(Value::as_str)
                .map(|s| s.to_lowercase()),
        })
    }

    fn extension(&self) -> String {
        self.format.clone().unwrap_or_else(|| {
            self.path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default()
                .to_lowercase()
        })
    }
}

/// The single cached source.  See [`crate::mcp::Server::cache`] for why one.
pub struct Cached {
    source: Source,
    mtime: Option<SystemTime>,
    df: DataFrame,
}

fn mtime_of(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

/// Load `source`, reusing the cached frame when the file has not changed.
pub fn load(server: &mut super::Server, source: &Source) -> Result<DataFrame, String> {
    let mtime = mtime_of(&source.path);

    if let Some(cached) = &server.cache {
        if cached.source == *source && cached.mtime == mtime {
            return Ok(cached.df.clone());
        }
    }

    let df = load_once(source)?;
    server.cache = Some(Cached {
        source: source.clone(),
        mtime,
        df: df.clone(),
    });
    Ok(df)
}

/// Load without consulting or filling the cache.  `join` uses this: its
/// right-hand side would otherwise evict the frame the pipeline is built on.
pub fn load_once(source: &Source) -> Result<DataFrame, String> {
    if !source.path.exists() {
        return Err(format!("No such file: {}", source.path.display()));
    }

    let ext = source.extension();

    if let Some(container) = &source.container {
        return load_container(&source.path, &ext, container);
    }

    // An explicit csv/tsv format has no route through `load_file_as`, whose
    // `forced` parameter only covers the document formats.
    if matches!(ext.as_str(), "csv" | "tsv") && source.format.is_some() {
        return crate::data::loader::load_csv(&source.path, source.delimiter)
            .map_err(|e| e.to_string());
    }

    let forced = source.format.as_deref().and_then(Format::from_name);
    io::load_file_as(&source.path, source.delimiter, forced)
        .map(|(df, _)| df)
        .map_err(|e| e.to_string())
}

fn load_container(path: &Path, ext: &str, container: &str) -> Result<DataFrame, String> {
    match ext {
        "xlsx" | "xls" => io::load_excel_sheet_by_name(path, container),
        "sqlite" | "sqlite3" => io::load_sqlite_table_by_name(path, container),
        "duckdb" | "ddb" => io::load_duckdb_table_by_name(path, container),
        "db" => io::load_sqlite_table_by_name(path, container)
            .or_else(|_| io::load_duckdb_table_by_name(path, container)),
        other => Err(color_eyre::eyre::eyre!(
            "'.{}' files hold a single table — drop 'container'",
            other
        )),
    }
    .map_err(|e| e.to_string())
}

/// List the tables or sheets a file holds, or `None` when the format holds one
/// unnamed table.
pub fn containers(path: &Path, ext: &str) -> Option<Vec<String>> {
    match ext {
        "xlsx" | "xls" => io::excel_sheet_names(path).ok(),
        "sqlite" | "sqlite3" => io::sqlite_table_names(path).ok(),
        "duckdb" | "ddb" => io::duckdb_table_names(path).ok(),
        "db" => io::sqlite_table_names(path)
            .ok()
            .filter(|n| !n.is_empty())
            .or_else(|| io::duckdb_table_names(path).ok()),
        _ => None,
    }
}

/// The extension a source resolves to, for callers outside this module.
pub fn extension_of(source: &Source) -> String {
    source.extension()
}

/// Load a source as a document tree.  Only the structured formats have one; a
/// CSV has no nesting for jq to walk.
pub fn load_doc(source: &Source) -> Result<Doc, String> {
    let ext = source.extension();
    let format = Format::from_name(&ext).ok_or_else(|| {
        format!(
            "jq needs a JSON, JSONL, YAML or TOML source; '{}' is not one. \
             Use tuitab_query for tabular data.",
            if ext.is_empty() {
                "(no extension)"
            } else {
                &ext
            }
        )
    })?;
    Doc::load(&source.path, format).map_err(|e| e.to_string())
}
