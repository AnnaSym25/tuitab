use std::path::Path;
use tuitab::data::dataframe::DataFrame;
use tuitab::data::loader::load_csv;

fn load_sample() -> DataFrame {
    load_csv(Path::new("test_data/sample.csv"), None).expect("Failed to load sample.csv")
}

#[test]
fn test_sort_integer_ascending() {
    let mut df = load_sample();
    df.sort_by(0, false); // sort by 'id' ascending
    let values: Vec<String> = (0..df.visible_row_count())
        .map(|i| DataFrame::anyvalue_to_string_fmt(&df.get_val(i, 0)))
        .collect();
    let ids: Vec<i64> = values.iter().map(|s| s.parse::<i64>().unwrap()).collect();
    let mut expected = ids.clone();
    expected.sort();
    assert_eq!(ids, expected, "id column should be sorted ascending");
}

#[test]
fn test_sort_integer_descending() {
    let mut df = load_sample();
    df.sort_by(2, true); // sort by 'age' descending
    let first_age: i64 = DataFrame::anyvalue_to_string_fmt(&df.get_val(0, 2))
        .parse::<i64>()
        .unwrap();
    let last_age: i64 =
        DataFrame::anyvalue_to_string_fmt(&df.get_val(df.visible_row_count() - 1, 2))
            .parse::<i64>()
            .unwrap();
    assert!(
        first_age >= last_age,
        "First age ({}) should be >= last age ({}) in descending sort",
        first_age,
        last_age
    );
}

#[test]
fn test_sort_float() {
    let mut df = load_sample();
    df.sort_by(3, false); // sort by 'salary' ascending
    let values: Vec<f64> = (0..df.visible_row_count())
        .map(|i| {
            DataFrame::anyvalue_to_string_fmt(&df.get_val(i, 3))
                .parse::<f64>()
                .unwrap()
        })
        .collect();
    for w in values.windows(2) {
        assert!(w[0] <= w[1], "salary values should be non-decreasing");
    }
}

#[test]
fn test_reset_sort() {
    let mut df = load_sample();
    let original_first = DataFrame::anyvalue_to_string_fmt(&df.get_val(0, 0));
    df.sort_by(2, true); // sort by age desc
    df.reset_sort();
    assert_eq!(
        DataFrame::anyvalue_to_string_fmt(&df.get_val(0, 0)),
        original_first,
        "After reset, first row should be back to original"
    );
}

// ── multi-key sort ──────────────────────────────────────────────────────────

/// Read a column of the visible rows, in display order.
fn column_values(df: &DataFrame, col: usize) -> Vec<String> {
    (0..df.visible_row_count())
        .map(|i| DataFrame::anyvalue_to_string_fmt(&df.get_val(i, col)))
        .collect()
}

#[test]
fn multi_key_sort_orders_by_the_second_key_within_the_first() {
    let mut df = load_sample();
    // department ascending, then age descending inside each department.
    df.sort_by_keys(&[(4, false), (2, true)]).unwrap();

    let departments = column_values(&df, 4);
    let ages: Vec<i64> = column_values(&df, 2)
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let mut sorted_departments = departments.clone();
    sorted_departments.sort();
    assert_eq!(
        departments, sorted_departments,
        "primary key must be ordered"
    );

    // Within each run of one department, ages must be descending.
    for window in departments.windows(2).enumerate() {
        let (i, pair) = window;
        if pair[0] == pair[1] {
            assert!(
                ages[i] >= ages[i + 1],
                "age must descend inside '{}': {} then {}",
                pair[0],
                ages[i],
                ages[i + 1]
            );
        }
    }

    // Engineering comes first alphabetically and holds ages 52, 42, 37, 34, 33, 30, 28.
    assert_eq!(&departments[0..7], &["Engineering"; 7]);
    assert_eq!(&ages[0..7], &[52, 42, 37, 34, 33, 30, 28]);
}

/// Chaining two single-key sorts is not a *guaranteed* compound sort, which is
/// why `sort_by_keys` exists.
///
/// `sort_by` leaves `maintain_order` at its default of `false`, which permits
/// Polars to reorder rows that tie on the sort key — it does not oblige it to.
/// On this twenty-row fixture the two happen to agree, and that is the point:
/// the chained form is right by luck, not by contract, and the luck depends on
/// frame size, thread count and Polars version. `sort_by_keys` asks for the
/// compound order outright.
///
/// The assertion is therefore on the primary key only. Asserting that the two
/// *differ* would be asserting the same unspecified behaviour from the other
/// side.
#[test]
fn chaining_sorts_agrees_on_the_primary_key_but_promises_nothing_more() {
    let mut chained = load_sample();
    chained.sort_by(2, true); // age desc
    chained.sort_by(4, false); // then department asc

    let mut compound = load_sample();
    compound.sort_by_keys(&[(4, false), (2, true)]).unwrap();

    assert_eq!(
        column_values(&chained, 4),
        column_values(&compound, 4),
        "the primary key is the only thing both forms are contracted to deliver"
    );
}

/// The TUI reaches the compound sort through `z[`/`z]`, and it must land on the
/// same order the shared engine produces. This is the parity check the plan
/// calls for: one function, driven from the surface, compared cell by cell.
#[test]
fn the_z_bracket_keys_build_the_same_compound_sort() {
    use tuitab::types::Action;

    let mut app = tuitab::app::App::new_as(Path::new("test_data/sample.csv"), None, None).unwrap();

    // Cursor onto `department` (column 4), sort ascending — replacing any sort.
    app.stack.active_mut().cursor_col = 4;
    app.handle_action(Action::SortAscending);
    // Then onto `age` (column 2), appended descending.
    app.stack.active_mut().cursor_col = 2;
    app.handle_action(Action::AddSortKeyDescending);

    let sheet = app.stack.active();
    assert_eq!(
        sheet.sort_keys,
        vec![("department".to_string(), false), ("age".to_string(), true)],
        "both keys must be live, in the order they were pressed"
    );

    let mut expected = load_sample();
    expected.sort_by_keys(&[(4, false), (2, true)]).unwrap();

    for row in 0..expected.visible_row_count() {
        for col in 0..expected.columns.len() {
            assert_eq!(
                DataFrame::anyvalue_to_string_fmt(&sheet.dataframe.get_val(row, col)),
                DataFrame::anyvalue_to_string_fmt(&expected.get_val(row, col)),
                "cell ({}, {})",
                row,
                col
            );
        }
    }
}

/// Appending a column that is already a sort key changes its direction rather
/// than listing it twice — otherwise repeated presses would grow the key list
/// without bound while meaning nothing new.
#[test]
fn appending_a_column_already_sorted_replaces_its_direction() {
    use tuitab::types::Action;

    let mut app = tuitab::app::App::new_as(Path::new("test_data/sample.csv"), None, None).unwrap();
    app.stack.active_mut().cursor_col = 4;
    app.handle_action(Action::SortAscending);

    app.stack.active_mut().cursor_col = 2;
    app.handle_action(Action::AddSortKeyAscending);
    assert_eq!(
        app.stack.active().sort_keys,
        vec![
            ("department".to_string(), false),
            ("age".to_string(), false)
        ]
    );

    app.handle_action(Action::AddSortKeyDescending);
    assert_eq!(
        app.stack.active().sort_keys,
        vec![("department".to_string(), false), ("age".to_string(), true)],
        "age flips direction; it does not appear twice"
    );
}

#[test]
fn multi_key_sort_composes_with_a_prior_filter() {
    let mut df = load_sample();
    // Keep only the four HR people (physical rows 6, 9, 12, 16).
    df.row_order = std::sync::Arc::new(vec![6, 9, 12, 16]);
    df.sort_by_keys(&[(2, false)]).unwrap();

    assert_eq!(df.visible_row_count(), 4, "sorting must not resurrect rows");
    let ages: Vec<i64> = column_values(&df, 2)
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    assert_eq!(ages, vec![25, 26, 27, 29]);
}

/// A sort polars refuses used to return quietly, leaving the rows as they were
/// — the caller could not tell an unsortable column from an already-sorted one.
#[test]
fn a_sort_that_cannot_run_says_so() {
    let mut df = load_sample();
    assert!(
        df.sort_by_keys(&[(99, false)]).is_err(),
        "a column that does not exist is not a no-op"
    );
    assert!(df.sort_by_keys(&[]).is_err(), "no keys is not a sort");
    assert!(df.sort_by_keys(&[(2, false)]).is_ok());
}
