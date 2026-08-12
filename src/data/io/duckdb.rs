use crate::data::dataframe::DataFrame;
use crate::data::io::db_write::{DbColumn, DbKind, DbRows, DeclType, TableSource};
use crate::data::io::sqlite::quote_ident;
use crate::data::io::{wrap_polars_df, ContainerInfo};
use color_eyre::{eyre::eyre, Result};
use polars::prelude::*;
use std::path::Path;

/// Every table and view of the database — see [`super::sqlite::sqlite_containers`].
pub fn duckdb_containers(path: &Path) -> Result<Vec<ContainerInfo>> {
    let conn = crate::data::io::db_write::open_duckdb(path)?;

    let mut stmt = conn.prepare(
        "SELECT table_name, table_type \
         FROM information_schema.tables \
         WHERE table_schema = 'main' AND table_type IN ('BASE TABLE', 'VIEW') \
         ORDER BY table_name",
    )?;
    let mut listed: Vec<(String, bool)> = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(0)?;
        let view = row.get::<_, String>(1)? == "VIEW";
        listed.push((name, view));
    }

    // DuckDB keeps view definitions in their own catalogue rather than beside the
    // tables, so this is the only way to give the listing a `CREATE …` at all.
    let mut view_sql: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Ok(mut vs) = conn.prepare("SELECT view_name, sql FROM duckdb_views() WHERE NOT internal")
    {
        if let Ok(mut vr) = vs.query([]) {
            while let Ok(Some(row)) = vr.next() {
                if let (Ok(n), Ok(q)) = (row.get::<_, String>(0), row.get::<_, String>(1)) {
                    view_sql.insert(n, q);
                }
            }
        }
    }

    let mut out = Vec::new();
    for (name, view) in listed {
        let columns: i64 = conn
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM information_schema.columns \
                     WHERE table_schema = 'main' AND table_name = '{}'",
                    name.replace('\'', "''")
                ),
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let rows_count = if view {
            None
        } else {
            conn.query_row(
                &format!("SELECT COUNT(*) FROM \"{}\"", quote_ident(&name)),
                [],
                |r| r.get::<_, i64>(0),
            )
            .ok()
        };
        let sql = view_sql.get(&name).cloned();
        out.push(ContainerInfo {
            name,
            view,
            rows: rows_count,
            columns: columns as usize,
            sql,
        });
    }
    Ok(out)
}

pub fn load_duckdb_overview(path: &Path) -> Result<DataFrame> {
    let containers = duckdb_containers(path)?;
    if containers.is_empty() {
        return Err(eyre!("No tables found in DuckDB database"));
    }

    let names: Vec<String> = containers.iter().map(|c| c.name.clone()).collect();
    let kinds: Vec<&str> = containers
        .iter()
        .map(|c| if c.view { "view" } else { "table" })
        .collect();
    let row_counts: Vec<String> = containers
        .iter()
        .map(|c| c.rows.map(|n| n.to_string()).unwrap_or_default())
        .collect();
    let col_counts: Vec<String> = containers.iter().map(|c| c.columns.to_string()).collect();
    let sql_defs: Vec<String> = containers
        .iter()
        .map(|c| c.sql.clone().unwrap_or_default())
        .collect();

    let series_vec = vec![
        Series::new("Table".into(), &names).into(),
        Series::new("Kind".into(), &kinds).into(),
        Series::new("Rows".into(), &row_counts).into(),
        Series::new("Columns".into(), &col_counts).into(),
        Series::new("SQL".into(), &sql_defs).into(),
    ];
    let pdf = polars::prelude::DataFrame::new_infer_height(series_vec)?;
    let mut df = wrap_polars_df(pdf)?;
    if df.columns.len() == 5 {
        df.columns[0].width = 40;
        df.columns[1].width = 6;
        df.columns[2].width = 12;
        df.columns[3].width = 12;
        df.columns[4].width = 60;
    }
    Ok(df)
}

pub fn load_duckdb_table_by_name(path: &Path, table_name: &str) -> Result<DataFrame> {
    load_duckdb_table_full(path, table_name).map(|(df, _)| df)
}

/// Load a table together with everything needed to write edits back into it.
///
/// The source comes back `None` when the table has a column of its own named `rowid`,
/// which shadows the pseudocolumn: the key would then be ordinary editable data, and a
/// user editing it would silently retarget every statement.  Read-only is the honest
/// answer.
pub fn load_duckdb_table_full(
    path: &Path,
    table_name: &str,
) -> Result<(DataFrame, Option<TableSource>)> {
    let conn = crate::data::io::db_write::open_duckdb(path)?;

    let columns = duckdb_columns(&conn, table_name)?;
    if columns.is_empty() {
        return Err(eyre!("No columns found in table: {}", table_name));
    }
    let shadowed = columns.iter().any(|c| c.name.eq_ignore_ascii_case("rowid"));
    if shadowed {
        let (mut df, _) = read_duckdb_table(&conn, table_name, &columns, false)?;
        crate::data::io::db_write::apply_declared_types(&mut df, &columns);
        return Ok((df, None));
    }

    // A view answers `PRAGMA table_info` but has no `rowid`, so the read is what fails,
    // not the metadata.  Fall back to reading without one and hand back no source: the
    // view is perfectly readable, just not addressable row by row.
    //
    // Only the rowid error, though: a locked file would otherwise come back as a
    // read-only sheet and be reported as a view.
    let (mut df, ids) = match read_duckdb_table(&conn, table_name, &columns, true) {
        Ok(pair) => pair,
        Err(e) if is_missing_rowid(&e) => {
            // Still typed: a filter must not depend on whether the same column was read
            // through a table or through a view over it.
            let (mut df, _) = read_duckdb_table(&conn, table_name, &columns, false)?;
            crate::data::io::db_write::apply_declared_types(&mut df, &columns);
            return Ok((df, None));
        }
        Err(e) => return Err(e),
    };

    // See load_sqlite_table_full: before the snapshot, so both agree on dtype.
    crate::data::io::db_write::apply_declared_types(&mut df, &columns);
    let source = TableSource {
        kind: DbKind::DuckDb,
        db_path: path.to_path_buf(),
        table: table_name.to_string(),
        key_col: "rowid".to_string(),
        columns,
        original: df.df.clone(),
    };
    // See load_sqlite_table_full: this is what tells a rename from a drop-and-add.
    for c in &mut df.columns {
        c.db_origin = Some(c.name.clone());
    }
    df.db_rows = Some(DbRows::new(ids));
    Ok((df, Some(source)))
}

/// Column metadata from `PRAGMA table_info`.
///
/// DuckDB exposes no flag for `GENERATED ALWAYS` columns — neither here nor in
/// `duckdb_columns()` — so `generated` stays false and the engine rejects the write
/// itself, at bind time, before any row is touched.
/// Whether DuckDB refused the query because this object has no `rowid`.
///
/// Its own phrasing — `Referenced column "rowid" not found in FROM clause!` — matched
/// whole for the same reason as the SQLite side: the bare word also turns up in
/// `Table with name rowid_map does not exist!`.
fn is_missing_rowid(err: &color_eyre::Report) -> bool {
    err.to_string()
        .to_ascii_lowercase()
        .contains("column \"rowid\" not found")
}

fn duckdb_columns(conn: &duckdb::Connection, table_name: &str) -> Result<Vec<DbColumn>> {
    // Which columns are computed, since no catalogue will say.  A view has no row in
    // `duckdb_tables()`, and a view has no generated columns either, so an empty set is
    // the right answer there rather than a failure.
    let generated = conn
        .query_row(
            "SELECT sql FROM duckdb_tables() WHERE table_name = ?",
            [table_name],
            |r| r.get::<_, String>(0),
        )
        .map(|ddl| generated_columns(&ddl))
        .unwrap_or_default();

    let mut stmt = conn.prepare(&format!(
        "PRAGMA table_info('{}')",
        table_name.replace('\'', "''")
    ))?;
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        let decl_raw: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
        out.push(DbColumn {
            decl: DeclType::from_sql(&decl_raw),
            notnull: row.get::<_, Option<bool>>(3)?.unwrap_or(false),
            pk: row.get::<_, Option<bool>>(5)?.unwrap_or(false),
            default_sql: row.get::<_, Option<String>>(4)?,
            generated: generated.contains(&name),
            name,
            decl_raw,
        });
    }
    Ok(out)
}

/// The names of the `GENERATED ALWAYS AS` columns in a `CREATE TABLE`.
///
/// DuckDB has no flag for this anywhere: `duckdb_columns()` has no such field, and
/// `information_schema.columns` leaves both `is_generated` and `generation_expression`
/// NULL.  `column_default` does hold the expression — but it holds a real DEFAULT the
/// same way, so it cannot tell the two apart.  The stored `CREATE TABLE` can.
///
/// Not a SQL parser, and does not need to be: the text comes from the engine, already
/// normalised — `CREATE TABLE t(a INTEGER, "b b" INTEGER GENERATED ALWAYS AS((a * 2)))`.
/// So this splits the column list at the commas that are not inside brackets or quotes,
/// takes the identifier each piece starts with, and asks whether the rest of that piece
/// says GENERATED.  A table whose DDL this misreads gets a column treated as ordinary,
/// which is where it started.
fn generated_columns(ddl: &str) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let Some(open) = ddl.find('(') else {
        return out;
    };
    let body = &ddl[open + 1..];

    let mut depth = 0usize;
    let mut quoted = false;
    let mut piece = String::new();
    let mut pieces = Vec::new();
    for c in body.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                piece.push(c);
            }
            '(' if !quoted => {
                depth += 1;
                piece.push(c);
            }
            ')' if !quoted => {
                // The one that closes the column list ends the whole thing.
                if depth == 0 {
                    break;
                }
                depth -= 1;
                piece.push(c);
            }
            ',' if !quoted && depth == 0 => pieces.push(std::mem::take(&mut piece)),
            _ => piece.push(c),
        }
    }
    pieces.push(piece);

    for piece in pieces {
        let piece = piece.trim();
        let (name, rest) = match piece.strip_prefix('"') {
            Some(after) => match after.find('"') {
                Some(end) => (after[..end].to_string(), &after[end + 1..]),
                None => continue,
            },
            None => match piece.find(char::is_whitespace) {
                Some(end) => (piece[..end].to_string(), &piece[end..]),
                None => continue,
            },
        };
        if super::db_write::mentions_word(rest, "GENERATED") {
            out.insert(name);
        }
    }
    out
}

/// Read a whole table, every column cast to text on the engine side.
///
/// The drift check has to issue the same `CAST(… AS VARCHAR)` — comparing against any
/// other rendering of a DOUBLE would report drift on rows nobody touched.
fn read_duckdb_table(
    conn: &duckdb::Connection,
    table_name: &str,
    columns: &[DbColumn],
    with_rowid: bool,
) -> Result<(DataFrame, Vec<Option<i64>>)> {
    let mut selects: Vec<String> = Vec::with_capacity(columns.len() + 1);
    if with_rowid {
        selects.push("rowid".to_string());
    }
    selects.extend(
        columns
            .iter()
            .map(|c| format!("CAST(\"{}\" AS VARCHAR)", quote_ident(&c.name))),
    );
    let query = format!(
        "SELECT {} FROM \"{}\"",
        selects.join(", "),
        quote_ident(table_name)
    );

    let mut stmt = conn.prepare(&query)?;
    let skip = usize::from(with_rowid);
    let mut cols_data: Vec<Vec<Option<String>>> = vec![Vec::new(); columns.len()];
    let mut ids: Vec<Option<i64>> = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        if with_rowid {
            ids.push(Some(row.get(0)?));
        }
        for (ci, col_vec) in cols_data.iter_mut().enumerate() {
            col_vec.push(row.get::<_, Option<String>>(ci + skip)?);
        }
    }

    let mut series_vec = Vec::new();
    for (i, col_data) in cols_data.into_iter().enumerate() {
        series_vec.push(Series::new(columns[i].name.as_str().into(), col_data).into());
    }
    let pdf = polars::prelude::DataFrame::new_infer_height(series_vec)?;
    wrap_polars_df(pdf).map(|df| (df, ids))
}

pub fn duckdb_table_names(path: &Path) -> Result<Vec<String>> {
    Ok(duckdb_containers(path)?
        .into_iter()
        .map(|c| c.name)
        .collect())
}
