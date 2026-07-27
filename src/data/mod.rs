//! Data model and I/O layer for tuitab.
//!
//! This module contains everything needed to load, store, and manipulate tabular data:
//!
//! | Sub-module | Responsibility |
//! |---|---|
//! | [`io`] | Format-aware file loader and saver (CSV, JSON, Parquet, Excel, SQLite, directory) |
//! | [`loader`] | Low-level CSV/TSV reader with auto-delimiter detection |
//! | [`async_loader`] | Background thread loader for files larger than 10 MB |
//! | [`dataframe`] | [`dataframe::DataFrame`] — Polars-backed in-memory store with view state |
//! | [`doc`] | [`doc::Doc`] — document tree shared by JSON/JSONL/YAML/TOML, source of truth for those formats |
//! | [`mod@column`] | [`column::ColumnMeta`] — per-column metadata (type, width, aggregators) |
//! | [`expression`] | Expression AST and recursive-descent parser for computed columns and row filters |
//! | [`aggregator`] | [`aggregator::AggregatorKind`] enum and compatibility rules |
//! | [`sort`] | Sort-by-column implementation using Polars `arg_sort` |
//! | [`swap`] | Serialize/deserialize a `DataFrame` to disk to free memory when sheets are stacked |
//! | [`view`] | [`view::View`] — projection of a `Doc` subtree into a table, and the cell→node mapping behind editing |

pub mod aggregator;
pub mod async_loader;
pub mod column;
pub mod dataframe;
pub mod doc;
pub mod expression;
pub mod io;
pub mod join;
pub mod loader;
pub mod sort;
pub mod swap;
pub mod view;

/// Naive datetime format strings tried when parsing a string to `NaiveDateTime`.
/// Fractional-seconds variants come first so strings with microseconds don't
/// fail the plain `%H:%M:%S` format before reaching the `%.f` one.
pub const DATETIME_FORMATS: &[&str] = &[
    "%Y-%m-%d %H:%M:%S%.f",
    "%Y-%m-%d %H:%M:%S",
    "%Y-%m-%dT%H:%M:%S%.f",
    "%Y-%m-%dT%H:%M:%S",
];
