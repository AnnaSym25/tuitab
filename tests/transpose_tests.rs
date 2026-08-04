//! Transposition, and that both surfaces get the same table.

use std::path::Path;
use tuitab::data::dataframe::DataFrame;
use tuitab::data::io::load_file;
use tuitab::data::transpose::{is_transposed, transpose_row, transpose_table};

fn sample() -> DataFrame {
    load_file(Path::new("test_data/sample.csv"), None).unwrap()
}

fn cell(df: &DataFrame, row: usize, col: usize) -> String {
    DataFrame::anyvalue_to_string_fmt(&df.get_val(row, col))
}

#[test]
fn transposing_a_row_gives_a_name_value_pair_per_column() {
    let df = sample();
    let out = transpose_row(&df, 0).unwrap();

    assert_eq!(out.columns.len(), 2);
    assert_eq!(out.columns[0].name, "Column");
    assert_eq!(out.columns[1].name, "Value");
    // Five source columns become five rows.
    assert_eq!(out.visible_row_count(), 5);
    assert_eq!(cell(&out, 1, 0), "name");
    assert_eq!(cell(&out, 1, 1), "Alice Johnson");
}

#[test]
fn transposing_a_row_out_of_range_is_refused() {
    let df = sample();
    assert!(transpose_row(&df, 999).is_err());
}

#[test]
fn transposing_the_table_swaps_rows_and_columns() {
    let df = sample();
    let out = transpose_table(&df).unwrap();

    // Five source columns become five rows; twenty rows become twenty data
    // columns, plus the label column.
    assert_eq!(out.visible_row_count(), 5);
    assert_eq!(out.columns.len(), 21);
    assert_eq!(out.columns[0].name, "column");
    assert!(out.columns[0].pinned, "the labels must stay on screen");
    assert_eq!(cell(&out, 1, 0), "name");
    assert_eq!(cell(&out, 1, 1), "Alice Johnson");
}

/// Transposing twice must return the original shape, not a table of a table.
#[test]
fn transposing_twice_inverts_rather_than_nesting() {
    let df = sample();
    let once = transpose_table(&df).unwrap();
    assert!(is_transposed(&once));

    let twice = transpose_table(&once).unwrap();
    assert!(!is_transposed(&twice));

    assert_eq!(twice.visible_row_count(), df.visible_row_count());
    assert_eq!(twice.columns.len(), df.columns.len());
    for (a, b) in twice.columns.iter().zip(df.columns.iter()) {
        assert_eq!(a.name, b.name);
    }
    for row in 0..df.visible_row_count() {
        for col in 0..df.columns.len() {
            assert_eq!(
                cell(&twice, row, col),
                cell(&df, row, col),
                "({}, {})",
                row,
                col
            );
        }
    }
}

#[test]
fn transposing_an_empty_table_is_refused_rather_than_silent() {
    let mut df = sample();
    df.row_order = std::sync::Arc::new(Vec::new());
    assert!(transpose_table(&df).is_err());
}

/// `T` in the terminal and `{"transpose": {}}` over MCP run the same function.
#[test]
fn the_t_key_produces_the_shared_transpose() {
    use tuitab::types::Action;

    let mut app = tuitab::app::App::new_as(Path::new("test_data/sample.csv"), None, None).unwrap();
    app.handle_action(Action::TransposeTable);

    let produced = &app.stack.active().dataframe;
    let expected = transpose_table(&sample()).unwrap();

    assert_eq!(produced.visible_row_count(), expected.visible_row_count());
    assert_eq!(produced.columns.len(), expected.columns.len());
    for row in 0..expected.visible_row_count() {
        for col in 0..expected.columns.len() {
            assert_eq!(
                cell(produced, row, col),
                cell(&expected, row, col),
                "({}, {})",
                row,
                col
            );
        }
    }
}

// ── recognising its own output ──────────────────────────────────────────────

/// The inverse is detected by a marker on the frame. A pinned column named
/// `column` is not one: pinning is ordinary user state, and
/// `build_multi_frequency_table` pins its group columns too — so a table that
/// merely happens to have such a column was transposed inside out, losing one.
#[test]
fn a_pinned_column_called_column_is_not_a_transpose() {
    let mut df = load_file(Path::new("test_data/sample.csv"), None).unwrap();

    // Rename `id` to `column` and pin it, exactly what `ze` then `!` would do.
    df.rename_column(0, "column").unwrap();
    df.columns[0].pinned = true;

    assert!(
        !is_transposed(&df),
        "a loaded table is not the output of a transpose, whatever its columns are called"
    );

    let out = transpose_table(&df).unwrap();
    // A real transpose of 5 columns gives 5 rows and keeps every column's data.
    assert_eq!(out.visible_row_count(), 5);
    assert_eq!(
        cell(&out, 0, 0),
        "column",
        "the renamed column is still there"
    );
}

/// And the round trip still works on the frames that genuinely are transposes.
#[test]
fn a_real_transpose_is_still_recognised() {
    let df = sample();
    let once = transpose_table(&df).unwrap();
    assert!(is_transposed(&once));
    assert!(!is_transposed(&transpose_table(&once).unwrap()));
}
