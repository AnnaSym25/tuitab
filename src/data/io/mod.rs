use crate::data::column::ColumnMeta;
use crate::data::dataframe::DataFrame;
use crate::types::ColumnType;
use color_eyre::{eyre::eyre, Result};
use polars::prelude::*;
use std::collections::HashSet;
use std::fs::File;
use std::path::Path;

mod directory;
pub mod doc_io;
mod duckdb;
mod excel;

mod parquet;
mod sqlite;
mod txt;

pub use directory::{load_directory, load_files_list};
pub use duckdb::{duckdb_table_names, load_duckdb_overview, load_duckdb_table_by_name};
pub use excel::{excel_sheet_names, load_excel_overview, load_excel_sheet_by_name};
pub use sqlite::{load_sqlite_overview, load_sqlite_table_by_name, sqlite_table_names};

pub use directory::format_file_size_pub;

pub fn load_file(path: &Path, delimiter: Option<u8>) -> Result<DataFrame> {
    load_file_with_doc(path, delimiter).map(|(df, _)| df)
}

/// Load `path`, additionally returning the document tree when the file is one of the
/// structured formats.  Sheets that keep the [`doc_io::DocState`] can edit and re-save
/// the real structure; callers that drop it get a read-only projection.
///
/// `forced` overrides the extension, which is how `--type` opens a file whose name says
/// nothing useful (`deploy.conf` as YAML).
pub fn load_file_with_doc(
    path: &Path,
    delimiter: Option<u8>,
) -> Result<(DataFrame, Option<doc_io::DocState>)> {
    load_file_as(path, delimiter, None)
}

pub fn load_file_as(
    path: &Path,
    delimiter: Option<u8>,
    forced: Option<crate::data::doc::Format>,
) -> Result<(DataFrame, Option<doc_io::DocState>)> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("csv")
        .to_lowercase();

    if let Some(fmt) = forced.or_else(|| crate::data::doc::Format::from_ext(&ext)) {
        let (df, state) = doc_io::DocState::open(path, fmt)?;
        return Ok((df, Some(state)));
    }

    // The extension said nothing useful.  Look at the contents before giving up — or,
    // for a file with no extension, before falling back to the CSV default.
    let has_ext = path.extension().is_some();
    let known_tabular = matches!(
        ext.as_str(),
        "csv" | "tsv" | "txt" | "parquet" | "xlsx" | "xls" | "db" | "sqlite" | "sqlite3"
            | "duckdb" | "ddb"
    );
    if !has_ext || !known_tabular {
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Some(fmt) = crate::data::doc::sniff(&text, !has_ext) {
                let mut doc = crate::data::doc::Doc::from_str(&text, fmt)?;
                doc.path = Some(path.to_path_buf());
                let (df, state) = doc_io::DocState::from_doc(doc)?;
                return Ok((df, Some(state)));
            }
        }
    }

    load_tabular(path, delimiter, &ext).map(|df| (df, None))
}

fn load_tabular(path: &Path, delimiter: Option<u8>, ext: &str) -> Result<DataFrame> {
    match ext {
        "csv" | "tsv" => crate::data::loader::load_csv(path, delimiter),
        "txt" => txt::load_txt(path),
        "parquet" => parquet::load_parquet(path),
        "xlsx" | "xls" => excel::load_excel(path),
        "db" => sqlite::load_sqlite_overview(path).or_else(|_| duckdb::load_duckdb_overview(path)),
        "sqlite" | "sqlite3" => sqlite::load_sqlite_overview(path),
        "duckdb" | "ddb" => duckdb::load_duckdb_overview(path),
        _ => Err(eyre!("Unsupported file format: .{}", ext)),
    }
}

pub fn save_file(df: &DataFrame, path: &Path) -> Result<()> {
    save_file_as(df, None, path, doc_io::Shape::Records, "rows")
}

/// Save to `path`, choosing the writer from its extension.
///
/// For the structured formats the rule is: a sheet that carries a document tree is
/// written by re-serialising that tree, so the original structure survives and
/// converting between formats is just picking a different extension.  A sheet with no
/// tree behind it (CSV, Parquet, SQL, pivot) is first turned into a tree using `shape`.
pub fn save_file_as(
    df: &DataFrame,
    doc: Option<&doc_io::DocState>,
    path: &Path,
    shape: doc_io::Shape,
    sheet_name: &str,
) -> Result<()> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("csv")
        .to_lowercase();

    if let Some(fmt) = crate::data::doc::Format::from_ext(&ext) {
        let opts = crate::data::doc::SaveOpts::default();
        return match doc {
            Some(state) => state.save_wrapped(path, fmt, &opts, sheet_name),
            None => doc_io::table_to_doc(df, shape, fmt, sheet_name)?.save_as(path, fmt, &opts),
        };
    }

    match ext.as_str() {
        "csv" => save_csv(df, path, b','),
        "tsv" => save_csv(df, path, b'\t'),
        "parquet" => parquet::save_parquet(df, path),
        "db" | "sqlite" | "sqlite3" => sqlite::save_sqlite(df, path),
        "xlsx" | "xls" => excel::save_xlsx(df, path),
        _ => Err(eyre!("Unsupported save format: .{}", ext)),
    }
}

pub fn load_from_stdin_typed(data_type: &str, delimiter: Option<u8>) -> Result<DataFrame> {
    load_from_stdin_with_doc(data_type, delimiter).map(|(df, _)| df)
}

pub fn load_from_stdin_with_doc(
    data_type: &str,
    delimiter: Option<u8>,
) -> Result<(DataFrame, Option<doc_io::DocState>)> {
    use std::io::{Read, Write};
    use tempfile::NamedTempFile;

    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;

    if let Some(fmt) = crate::data::doc::Format::from_name(data_type) {
        let text = String::from_utf8(buf)?;
        let (df, state) = doc_io::DocState::from_doc(crate::data::doc::Doc::from_str(&text, fmt)?)?;
        return Ok((df, Some(state)));
    }

    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(&buf)?;
    let temp_path = temp_file.path().to_path_buf();

    let pdf = match data_type.to_lowercase().as_str() {
        "csv" | "txt" => {
            let sep = delimiter.unwrap_or(b',');
            polars::prelude::CsvReadOptions::default()
                .with_has_header(true)
                .map_parse_options(|o| o.with_separator(sep))
                .try_into_reader_with_file_path(Some(temp_path))?
                .finish()?
        }
        "tsv" => polars::prelude::CsvReadOptions::default()
            .with_has_header(true)
            .map_parse_options(|o| o.with_separator(b'\t'))
            .try_into_reader_with_file_path(Some(temp_path))?
            .finish()?,
        _ => return Err(eyre!("Unsupported stdin data type: {}", data_type)),
    };

    drop(temp_file);
    wrap_polars_df(pdf).map(|df| (df, None))
}

pub(crate) fn wrap_polars_df(pdf: polars::prelude::DataFrame) -> Result<DataFrame> {
    let col_count = pdf.width();
    let row_count = pdf.height();
    let mut columns = Vec::with_capacity(col_count);

    for series in pdf.columns() {
        let name = series.name().to_string();
        let mut col_meta = ColumnMeta::new(name);

        col_meta.col_type = match series.dtype() {
            DataType::Int8
            | DataType::Int16
            | DataType::Int32
            | DataType::Int64
            | DataType::UInt8
            | DataType::UInt16
            | DataType::UInt32
            | DataType::UInt64 => ColumnType::Integer,
            DataType::Float32 | DataType::Float64 => ColumnType::Float,
            DataType::Date => ColumnType::Date,
            DataType::Datetime(_, _) => ColumnType::Datetime,
            _ => ColumnType::String,
        };

        columns.push(col_meta);
    }

    let row_order: Vec<usize> = (0..row_count).collect();
    let original_order = row_order.clone();

    let mut df = DataFrame {
        df: pdf,
        columns,
        row_order: std::sync::Arc::new(row_order),
        original_order: std::sync::Arc::new(original_order),
        selected_rows: HashSet::new(),
        modified: false,
        aggregates_cache: None,
    };
    df.calc_widths(40, 1000);
    Ok(df)
}

fn save_csv(df: &DataFrame, path: &Path, delimiter: u8) -> Result<()> {
    let mut out_df = df.to_display_polars_df();
    let mut file = File::create(path)?;
    CsvWriter::new(&mut file)
        .include_header(true)
        .with_separator(delimiter)
        .finish(&mut out_df)?;
    Ok(())
}
