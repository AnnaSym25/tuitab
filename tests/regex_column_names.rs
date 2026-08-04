//! Column names that look like regular expressions.
//!
//! Polars' `regex` feature makes `col("^…$")` select *every column matching the
//! pattern* instead of the one column with that literal name.  tuitab passes
//! user-supplied column names to `col()` from several places — the aggregator
//! (`aggregator.rs`), frequency tables and pivots (`dataframe.rs`), and the MCP
//! filter — so a file with such a header would misbehave across the TUI too,
//! not just the server.
//!
//! `test_data/regex_name.csv` has headers `^total$`, `plain`, `.*` and `amount`.
//! These tests pin what happens to them.

use std::path::Path;
use tuitab::data::aggregator::AggregatorKind;
use tuitab::data::dataframe::DataFrame;
use tuitab::data::io::load_file;

fn load() -> DataFrame {
    load_file(Path::new("test_data/regex_name.csv"), None).unwrap()
}

#[test]
fn a_regex_shaped_header_survives_loading_intact() {
    let df = load();
    let names: Vec<&str> = df.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["^total$", "plain", ".*", "amount"]);
    assert_eq!(df.visible_row_count(), 3);
}

#[test]
fn grouping_by_a_regex_shaped_column_groups_by_that_column() {
    let df = load();
    let col = df.columns.iter().position(|c| c.name == "^total$").unwrap();

    let (pdf, metas) = df.build_frequency_table(col, &[]).unwrap();

    // Two distinct values, `a` twice and `b` once.
    assert_eq!(pdf.height(), 2, "frequency must see two distinct values");
    assert_eq!(metas[0].name, "^total$");
    assert_eq!(metas[1].name, "Count");
}

#[test]
fn aggregating_over_a_regex_shaped_column_uses_that_column() {
    let df = load();
    let group = df.columns.iter().position(|c| c.name == ".*").unwrap();
    let amount = df.columns.iter().position(|c| c.name == "amount").unwrap();

    // Group by the `.*` column — every row is distinct — summing `amount`.
    let (pdf, _) = df
        .build_frequency_table(group, &[(amount, vec![AggregatorKind::Sum])])
        .unwrap();

    assert_eq!(pdf.height(), 3, "p, q and r are three distinct values");
    let sums = pdf.column("amount:sum").unwrap();
    let total: f64 = (0..3)
        .map(|i| {
            sums.get(i)
                .unwrap()
                .try_extract::<f64>()
                .unwrap_or_default()
        })
        .sum();
    assert_eq!(total, 60.0, "10 + 20 + 30 regardless of the column's name");
}

#[test]
fn a_pivot_over_regex_shaped_columns_addresses_the_right_ones() {
    use tuitab::data::expression::Expr;

    let df = load();
    let formula = Expr::parse("sum(amount)").unwrap();
    let (pdf, _) = df
        .create_pivot_table(&["^total$".to_string()], "plain", &formula)
        .unwrap();

    // Two index values (`a`, `b`) spread across three `plain` values.
    assert_eq!(pdf.height(), 2);
}
