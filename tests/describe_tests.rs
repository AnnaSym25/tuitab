//! Coverage for the per-column profile behind the `I` key and `tuitab_describe`.
//!
//! The expected numbers are worked out from `test_data/sample.csv` by hand, so
//! this fails if the arithmetic changes — not merely if the output changes shape.

use std::path::PathBuf;
use tuitab::data::describe::{describe, METRICS};
use tuitab::data::io::load_file;

fn sample() -> tuitab::data::dataframe::DataFrame {
    load_file(&PathBuf::from("test_data/sample.csv"), None).unwrap()
}

/// Look up one metric for one column of a describe result.
fn metric(profile: &tuitab::data::dataframe::DataFrame, name: &str, column: &str) -> String {
    let row = METRICS
        .iter()
        .position(|m| *m == name)
        .expect("known metric");
    let col = profile
        .columns
        .iter()
        .position(|c| c.name == column)
        .unwrap_or_else(|| panic!("no column '{}' in the profile", column));
    profile.get_physical(row, col)
}

#[test]
fn the_profile_is_one_row_per_metric_and_one_column_per_source_column() {
    let profile = describe(&sample());

    assert_eq!(profile.visible_row_count(), METRICS.len());
    // The source has 5 columns; the profile adds the pinned `metric` column.
    assert_eq!(profile.columns.len(), 6);
    assert_eq!(profile.columns[0].name, "metric");
    assert!(profile.columns[0].pinned, "metric column must stay visible");
}

#[test]
fn numeric_metrics_match_hand_computed_values() {
    let profile = describe(&sample());

    // 20 ages summing to 733 → mean 36.65; sorted, the middle pair is 35 and 37.
    assert_eq!(metric(&profile, "count", "age"), "20");
    assert_eq!(metric(&profile, "nulls", "age"), "0");
    assert_eq!(metric(&profile, "unique", "age"), "20");
    assert_eq!(metric(&profile, "mean", "age"), "36.65");
    assert_eq!(metric(&profile, "median", "age"), "36.00");
    assert_eq!(metric(&profile, "min", "age"), "25.00");
    assert_eq!(metric(&profile, "max", "age"), "52.00");
    assert_eq!(metric(&profile, "range", "age"), "27.00");
}

#[test]
fn stdev_is_the_population_figure() {
    let profile = describe(&sample());

    // Squared deviations from 36.65 sum to 1274.55.  Dividing by n gives
    // 63.7275 and a standard deviation of 7.98; dividing by n-1 — what the
    // footer aggregator of the same name does — would give 8.19 instead.
    assert_eq!(metric(&profile, "stdev", "age"), "7.98");
}

#[test]
fn non_numeric_columns_get_lexicographic_bounds_and_no_mean() {
    let profile = describe(&sample());

    assert_eq!(metric(&profile, "unique", "department"), "4");
    assert_eq!(metric(&profile, "min", "department"), "Engineering");
    assert_eq!(metric(&profile, "max", "department"), "Marketing");
    assert_eq!(
        metric(&profile, "range", "department"),
        "Engineering → Marketing"
    );
    assert_eq!(metric(&profile, "mean", "department"), "");
    assert_eq!(metric(&profile, "median", "department"), "");
}

#[test]
fn mode_is_the_most_frequent_value() {
    let profile = describe(&sample());
    // Engineering appears 7 times, more than any other department.
    assert_eq!(metric(&profile, "mode", "department"), "Engineering");
}

/// Currency and Percentage count as numeric and carry their own precision.
/// `sample.csv` has neither, so this is the branch no other test reaches.
#[test]
fn currency_and_percentage_columns_are_numeric_and_use_their_precision() {
    use tuitab::types::ColumnType;

    let mut df = sample();
    let salary = df.columns.iter().position(|c| c.name == "salary").unwrap();
    df.columns[salary].col_type = ColumnType::Currency;
    df.columns[salary].precision = 0;

    let profile = describe(&df);

    // Salaries sum to 1624003.25 over 20 people — a mean of 81200.1625, shown
    // with the column's own precision of 0 rather than a fixed two places.
    assert_eq!(metric(&profile, "mean", "salary"), "81200");
    assert_eq!(metric(&profile, "min", "salary"), "61000");
    assert_eq!(metric(&profile, "max", "salary"), "110000");
    assert_ne!(
        metric(&profile, "stdev", "salary"),
        "",
        "Currency must be treated as numeric, not as text"
    );
}

/// A profile describes the rows that are *visible*, not every row loaded — so a
/// sheet derived from a subset reports that subset's numbers.
#[test]
fn the_profile_follows_row_order_rather_than_the_whole_frame() {
    let mut df = sample();
    // Keep the first four rows: ages 30, 45, 28 and 35, summing to 138.
    df.row_order = std::sync::Arc::new(vec![0, 1, 2, 3]);

    let profile = describe(&df);
    assert_eq!(metric(&profile, "count", "age"), "4");
    assert_eq!(metric(&profile, "mean", "age"), "34.50");
    assert_eq!(metric(&profile, "max", "age"), "45.00");
}

// ── the `I` key ─────────────────────────────────────────────────────────────

// `describe` is reached from the TUI through one small piece of wiring in
// `App`.  These drive the real action so the wiring cannot rot unnoticed.

fn describe_via_the_ui(path: &str) -> tuitab::app::App {
    let mut app = tuitab::app::App::new_as(&PathBuf::from(path), None, None).unwrap();
    app.handle_action(tuitab::types::Action::DescribeSheet);
    app
}

#[test]
fn the_describe_key_pushes_a_titled_sheet_with_the_metric_column_pinned() {
    let app = describe_via_the_ui("test_data/sample.csv");
    let sheet = app.stack.active();

    assert_eq!(sheet.title, "Describe: sample.csv");
    assert_eq!(sheet.dataframe.visible_row_count(), METRICS.len());
    assert_eq!(sheet.dataframe.columns[0].name, "metric");
    assert!(
        sheet.dataframe.columns[0].pinned,
        "the metric names must stay put while scrolling sideways"
    );
    assert!(
        sheet.dataframe.columns.iter().all(|c| c.width > 0),
        "widths must be calculated or every column renders collapsed"
    );
    assert_eq!(app.status_message, "Describe: 5 columns");
}

#[test]
fn describing_a_document_sheet_works_on_its_projected_table() {
    let app = describe_via_the_ui("test_data/nested.json");
    let sheet = app.stack.active();

    assert_eq!(sheet.dataframe.visible_row_count(), METRICS.len());
    // nested.json projects to id / name / tags / meta, plus the metric column.
    assert!(sheet.dataframe.columns.len() > 1, "{:?}", sheet.title);
}

#[test]
fn describing_a_describe_sheet_does_not_panic() {
    let mut app = describe_via_the_ui("test_data/sample.csv");
    app.handle_action(tuitab::types::Action::DescribeSheet);

    let sheet = app.stack.active();
    assert_eq!(sheet.title, "Describe: Describe: sample.csv");
    // Six columns in, six columns out — metric plus the five it profiled.
    assert_eq!(app.status_message, "Describe: 6 columns");
}

#[test]
fn mode_breaks_ties_by_first_appearance_and_not_by_hash_order() {
    // Every name is unique, so all 20 values tie at a count of one.  The answer
    // must still be the same on every run: the first row's name.
    let first = describe(&sample());
    let second = describe(&sample());

    assert_eq!(metric(&first, "mode", "name"), "Alice Johnson");
    assert_eq!(metric(&second, "mode", "name"), "Alice Johnson");
}
