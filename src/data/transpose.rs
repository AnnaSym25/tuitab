//! Turning rows into columns.
//!
//! Two related operations. [`transpose_row`] takes a single row and stands it
//! on end as a two-column name/value table — the natural way to read one wide
//! record. [`transpose_table`] does the whole visible table.
//!
//! Everything becomes text. A transposed row holds one cell from each of the
//! source columns, so the column it lands in has no single type to be.

use crate::data::column::ColumnMeta;
use crate::data::dataframe::DataFrame;
use polars::prelude::{Column, NamedFrom, Series};

/// The marker a transposed table carries: a pinned first column called
/// `column`, holding what used to be the column names.
///
/// [`transpose_table`] looks for it so that transposing twice returns the
/// original shape rather than nesting one transpose inside another.
const LABEL_COLUMN: &str = "column";

/// Stand one row on end: a `Column` / `Value` pair per source column.
///
/// `physical_row` is an index into the underlying frame, not a display
/// position — callers holding a display row should map it through `row_order`
/// first.
pub fn transpose_row(df: &DataFrame, physical_row: usize) -> Result<DataFrame, String> {
    if physical_row >= df.df.height() {
        return Err(format!(
            "row {} is out of range; the table has {} rows",
            physical_row,
            df.df.height()
        ));
    }

    let names: Vec<String> = df.columns.iter().map(|c| c.name.clone()).collect();
    let values: Vec<String> = (0..df.columns.len())
        .map(|i| df.get_physical(physical_row, i))
        .collect();

    let pdf = polars::prelude::DataFrame::new_infer_height(vec![
        Column::from(Series::new("Column".into(), &names)),
        Column::from(Series::new("Value".into(), &values)),
    ])
    .map_err(|e| e.to_string())?;

    let mut out = DataFrame::from_parts(
        pdf,
        vec![
            ColumnMeta::new("Column".to_string()),
            ColumnMeta::new("Value".to_string()),
        ],
    );
    out.calc_widths(40, 500);
    Ok(out)
}

/// Whether this frame is already the output of [`transpose_table`].
pub fn is_transposed(df: &DataFrame) -> bool {
    df.transposed
}

/// Transpose the visible rows.
///
/// Applied to a table this function produced, it inverts instead — the labels
/// in the `column` column become headers again and the headers become labels,
/// so `T` twice is the identity rather than a table of a table.
pub fn transpose_table(df: &DataFrame) -> Result<DataFrame, String> {
    let ncols = df.columns.len();
    let nrows = df.visible_row_count();

    if ncols == 0 || nrows == 0 {
        return Err("nothing to transpose: the table is empty".to_string());
    }

    let inverting = is_transposed(df);

    // `row_labels` become the values of the output's label column, and
    // `new_col_names` its headers. Inverting swaps where each comes from, and
    // skips the label column itself as a data source.
    let (row_labels, new_col_names, data_start): (Vec<String>, Vec<String>, usize) = if inverting {
        (
            df.columns[1..].iter().map(|c| c.name.clone()).collect(),
            (0..nrows)
                .map(|r| df.get_physical(df.row_order[r], 0))
                .collect(),
            1,
        )
    } else {
        (
            df.columns.iter().map(|c| c.name.clone()).collect(),
            (0..nrows)
                .map(|r| format!("row_{}", df.row_order[r]))
                .collect(),
            0,
        )
    };

    let data_ncols = ncols - data_start;

    // `cells[i][r]` is source column `data_start + i` at display row `r`.
    let cells: Vec<Vec<String>> = (data_start..ncols)
        .map(|i| {
            (0..nrows)
                .map(|r| df.get_physical(df.row_order[r], i))
                .collect()
        })
        .collect();

    let mut series_vec: Vec<Column> = Vec::with_capacity(new_col_names.len() + 1);
    if !inverting {
        series_vec.push(Column::from(Series::new(LABEL_COLUMN.into(), &row_labels)));
    }
    for (col_idx, name) in new_col_names.iter().enumerate() {
        let values: Vec<String> = (0..data_ncols).map(|i| cells[i][col_idx].clone()).collect();
        series_vec.push(Column::from(Series::new(name.as_str().into(), &values)));
    }

    let pdf =
        polars::prelude::DataFrame::new_infer_height(series_vec).map_err(|e| e.to_string())?;

    let mut metas: Vec<ColumnMeta> = if inverting {
        new_col_names.iter().cloned().map(ColumnMeta::new).collect()
    } else {
        std::iter::once(LABEL_COLUMN.to_string())
            .chain(new_col_names.iter().cloned())
            .map(ColumnMeta::new)
            .collect()
    };
    if !inverting {
        if let Some(first) = metas.first_mut() {
            // Pinned both so it stays visible while scrolling and so the next
            // transpose recognises its own output.
            first.pinned = true;
        }
    }

    let mut out = DataFrame::from_parts(pdf, metas);
    out.transposed = !inverting;
    out.calc_widths(40, 500);
    Ok(out)
}
