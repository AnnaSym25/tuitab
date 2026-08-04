//! Duplicate detection and removal, shared by both surfaces.

use std::path::Path;
use std::sync::Arc;
use tuitab::data::dataframe::DataFrame;
use tuitab::data::dedup::{deduplicate, duplicate_rows, Keep};
use tuitab::data::io::load_file;

/// `sample.csv` has no duplicates, so build one by repeating rows: physical
/// rows 0, 2, 4 are Engineering; listing 0 twice makes a duplicate department
/// key without touching the file.
fn with_repeats() -> DataFrame {
    let mut df = load_file(Path::new("test_data/sample.csv"), None).unwrap();
    // Alice(Eng), Bob(Mgmt), Carol(Eng), David(Mkt), Eve(Eng)
    df.row_order = Arc::new(vec![0, 1, 2, 3, 4]);
    df
}

fn department(df: &DataFrame) -> usize {
    df.column_index("department").unwrap()
}

#[test]
fn duplicates_are_the_rows_whose_key_repeats() {
    let df = with_repeats();
    let dept = department(&df);
    let mut found = duplicate_rows(&df, &[dept]);
    found.sort();
    // The three Engineering rows repeat; Management and Marketing appear once.
    assert_eq!(found, vec![0, 2, 4]);
}

#[test]
fn whole_row_duplicates_find_nothing_when_every_row_differs() {
    let df = with_repeats();
    assert!(
        duplicate_rows(&df, &[]).is_empty(),
        "no two rows are identical"
    );
}

#[test]
fn keeping_first_and_last_pick_the_ends_of_each_group() {
    let df = with_repeats();
    let dept = department(&df);

    assert_eq!(
        deduplicate(&df, &[dept], Keep::First).unwrap(),
        vec![0, 1, 3],
        "Alice for Engineering, then the two singletons"
    );
    assert_eq!(
        deduplicate(&df, &[dept], Keep::Last).unwrap(),
        vec![1, 3, 4],
        "Eve is the last Engineering row"
    );
}

#[test]
fn min_and_max_use_the_tiebreaker_column_numerically() {
    let df = with_repeats();
    let dept = department(&df);
    let age = df.column_index("age").unwrap();

    // Engineering ages here are Alice 30, Carol 28, Eve 42.
    assert_eq!(
        deduplicate(&df, &[dept], Keep::Min(age)).unwrap(),
        vec![1, 2, 3]
    );
    assert_eq!(
        deduplicate(&df, &[dept], Keep::Max(age)).unwrap(),
        vec![1, 3, 4]
    );
}

/// The random keeper must be reproducible, or a result nobody can repeat is
/// not much of a result.
#[test]
fn the_random_keeper_is_reproducible_from_its_seed() {
    let df = with_repeats();
    let dept = department(&df);

    let once = deduplicate(&df, &[dept], Keep::Random(42)).unwrap();
    let again = deduplicate(&df, &[dept], Keep::Random(42)).unwrap();
    assert_eq!(once, again);
    assert_eq!(once.len(), 3, "one row per department");
}

#[test]
fn deduplicating_without_keys_is_refused() {
    let df = with_repeats();
    assert!(deduplicate(&df, &[], Keep::First).is_err());
}

#[test]
fn an_out_of_range_column_is_refused_rather_than_ignored() {
    let df = with_repeats();
    assert!(deduplicate(&df, &[99], Keep::First).is_err());
    assert!(deduplicate(&df, &[0], Keep::Min(99)).is_err());
}

/// `gD` in the terminal and `{"dedup": …}` over MCP run the same function.
#[test]
fn the_gd_key_produces_the_shared_dedup() {
    use tuitab::types::Action;

    let mut app = tuitab::app::App::new_as(Path::new("test_data/sample.csv"), None, None).unwrap();
    {
        let s = app.stack.active_mut();
        let dept = s.dataframe.column_index("department").unwrap();
        s.dataframe.columns[dept].pinned = true;
    }
    app.handle_action(Action::DeduplicateByPinned);
    app.handle_action(Action::DeduplicateByPinned);

    let produced = &app.stack.active().dataframe;
    // Four departments, so four rows survive — the first of each.
    assert_eq!(produced.visible_row_count(), 4);

    let full = load_file(Path::new("test_data/sample.csv"), None).unwrap();
    let dept = full.column_index("department").unwrap();
    let expected = deduplicate(&full, &[dept], Keep::First).unwrap();
    assert_eq!(produced.row_order.to_vec(), expected);
}

// ── sampling ────────────────────────────────────────────────────────────────

use tuitab::data::dedup::sample_rows;

#[test]
fn a_sample_is_reproducible_from_its_seed_and_keeps_table_order() {
    let df = load_file(Path::new("test_data/sample.csv"), None).unwrap();

    let once = sample_rows(&df, 5, 7);
    let again = sample_rows(&df, 5, 7);
    assert_eq!(once, again, "the same seed must give the same rows");
    assert_eq!(once.len(), 5);

    let mut sorted = once.clone();
    sorted.sort();
    assert_eq!(
        once, sorted,
        "rows come back in table order, not draw order"
    );
}

#[test]
fn asking_for_more_rows_than_exist_returns_all_of_them() {
    let df = load_file(Path::new("test_data/sample.csv"), None).unwrap();
    assert_eq!(sample_rows(&df, 500, 1).len(), 20);
}

#[test]
fn a_sample_draws_only_from_the_visible_rows() {
    let mut df = load_file(Path::new("test_data/sample.csv"), None).unwrap();
    df.row_order = Arc::new(vec![3, 7, 11]);

    let chosen = sample_rows(&df, 2, 99);
    assert_eq!(chosen.len(), 2);
    assert!(
        chosen.iter().all(|r| [3, 7, 11].contains(r)),
        "a filtered-out row must not reappear: {:?}",
        chosen
    );
}

/// Sampling from a large table has to finish. Restoring the table's order used
/// a linear scan per comparison, which is quadratic in the frame — fine on the
/// twenty-row fixture above and hopeless on anything real.
///
/// Not a timing assertion, which would be flaky; the old implementation simply
/// did not return in a reasonable time here.
#[test]
fn sampling_from_a_large_table_finishes() {
    use polars::prelude::{Column, NamedFrom, Series};
    use tuitab::data::column::ColumnMeta;

    let n = 200_000;
    let values: Vec<i64> = (0..n as i64).collect();
    let pdf = polars::prelude::DataFrame::new_infer_height(vec![Column::from(Series::new(
        "id".into(),
        &values,
    ))])
    .unwrap();
    let df = DataFrame::from_parts(pdf, vec![ColumnMeta::new("id".to_string())]);

    let chosen = sample_rows(&df, 2_000, 11);
    assert_eq!(chosen.len(), 2_000);

    let mut sorted = chosen.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted, chosen, "distinct rows, in table order");
}

/// Deduplication compares stored values, not the rounded text on screen.
/// Displaying two decimals makes 1.504 and 1.496 look alike; treating them as
/// duplicates would throw away one of two distinct numbers.
#[test]
fn rows_that_merely_display_alike_are_not_duplicates() {
    use polars::prelude::{Column, NamedFrom, Series};
    use tuitab::data::column::ColumnMeta;
    use tuitab::types::ColumnType;

    let pdf = polars::prelude::DataFrame::new_infer_height(vec![Column::from(Series::new(
        "amount".into(),
        &[1.504f64, 1.496, 1.504],
    ))])
    .unwrap();
    let mut meta = ColumnMeta::new("amount".to_string());
    meta.col_type = ColumnType::Float;
    meta.precision = 2;
    let df = DataFrame::from_parts(pdf, vec![meta]);

    // On screen all three read "1.50".
    let shown: Vec<String> = (0..3).map(|r| df.format_display(r, 0)).collect();
    assert_eq!(shown, vec!["1.50", "1.50", "1.50"]);

    // Only the two that are genuinely equal are duplicates.
    let mut duplicates = duplicate_rows(&df, &[0]);
    duplicates.sort();
    assert_eq!(duplicates, vec![0, 2]);

    assert_eq!(
        deduplicate(&df, &[0], Keep::First).unwrap(),
        vec![0, 1],
        "1.496 survives; it is not the same number as 1.504"
    );
}
