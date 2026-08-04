//! Behaviour that only shows up when keys go through the real dispatch.
//!
//! See `tests/keys/mod.rs` for why these exist separately from the tests that
//! call `handle_action` directly.

mod keys;

use crossterm::event::KeyCode;
use keys::{columns, key, open, press};
use tuitab::types::AppMode;

const SAMPLE: &str = "test_data/sample.csv";

// ── the z-prefix must not stay open ─────────────────────────────────────────

/// Every z-command returns to Normal. `z[` and `z]` did not, which left the
/// next keystroke to be read as a z-command.
#[test]
fn adding_a_sort_key_returns_to_normal_mode() {
    let mut app = open(SAMPLE, "department");
    press(&mut app, "z[");
    assert_eq!(
        app.mode,
        AppMode::Normal,
        "a completed z-command must close the prefix"
    );
}

/// The consequence of leaving `ZPrefix` open: `d` means "delete selected rows"
/// in Normal and "delete this column" in the prefix. A user building a compound
/// sort and then pressing `d` lost a column.
#[test]
fn a_key_after_the_sort_chord_is_not_read_as_a_z_command() {
    let mut app = open(SAMPLE, "department");
    let before = columns(&app);

    press(&mut app, "z[");
    press(&mut app, "d");

    assert_eq!(
        columns(&app),
        before,
        "`d` after `z[` must not delete a column"
    );
}

/// And the feature itself: two chords in a row have to build a two-key sort.
/// While the prefix stayed open, the `z` of the second chord was swallowed and
/// the `]` replaced the sort instead of extending it.
#[test]
fn two_chords_build_a_compound_sort() {
    let mut app = open(SAMPLE, "department");
    press(&mut app, "z[");

    let age = app.stack.active().dataframe.column_index("age").unwrap();
    app.stack.active_mut().cursor_col = age;
    press(&mut app, "z]");

    assert_eq!(
        app.stack.active().sort_keys,
        vec![("department".to_string(), false), ("age".to_string(), true)],
        "both keys must survive"
    );
}

// ── a sort key that outlived its column ─────────────────────────────────────

/// Sorting by a column and then deleting it left `sort_keys` pointing past the
/// end of `columns`, and the next `z[` indexed it — a panic with the terminal
/// still in raw mode.
#[test]
fn deleting_a_sorted_column_does_not_leave_a_key_that_panics() {
    let mut app = open(SAMPLE, "department");
    press(&mut app, "]");
    assert_eq!(app.stack.active().sort_keys.len(), 1);

    press(&mut app, "zd");

    let age = app.stack.active().dataframe.column_index("age").unwrap();
    app.stack.active_mut().cursor_col = age;
    press(&mut app, "z[");

    // A key naming a column that is gone simply does not resolve.
    let resolved = app.stack.active().resolved_sort_keys();
    for (col, _) in &resolved {
        assert!(
            *col < app.stack.active().dataframe.columns.len(),
            "a resolved key must point at a live column"
        );
    }
}

/// Moving a column has to carry its sort key with it, or the arrow marks one
/// column while a different one is actually sorted.
#[test]
fn moving_a_column_carries_its_sort_key() {
    let mut app = open(SAMPLE, "name");
    press(&mut app, "]");

    assert_eq!(
        app.stack.active().sort_keys,
        vec![("name".to_string(), true)]
    );

    // z← swaps `name` leftwards, into index 0.
    key(&mut app, KeyCode::Char('z'));
    key(&mut app, KeyCode::Left);

    let moved = app.stack.active().dataframe.column_index("name").unwrap();
    assert_eq!(moved, 0, "the column moved");
    assert_eq!(
        app.stack.active().resolved_sort_keys(),
        vec![(moved, true)],
        "the sort key must follow the column it names"
    );
}

// ── the window-function picker ──────────────────────────────────────────────

/// `zw` arms a function for the partition picker. Escaping out of that picker
/// has to disarm it, or the next `zF` computes whatever was left behind instead
/// of a share.
#[test]
fn escaping_the_partition_picker_disarms_the_window_function() {
    let mut app = open(SAMPLE, "salary");

    press(&mut app, "zw");
    key(&mut app, KeyCode::Down); // move off pct_of_total
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Esc);

    assert!(
        app.pending_window_fn.is_none(),
        "cancelling must not leave a function armed"
    );

    // zF now means what it says.
    press(&mut app, "zF");
    key(&mut app, KeyCode::Enter);
    assert!(
        columns(&app).iter().any(|c| c.ends_with("pct_of_total")),
        "zF must produce a share: {:?}",
        columns(&app)
    );
}

/// A doc-backed sheet keeps a table that mirrors the document. Adding a column
/// breaks that correspondence permanently — editing dies for the session and
/// the column disappears at the next reprojection. `zf` refuses; `zw` must too.
#[test]
fn the_window_picker_refuses_a_document_sheet() {
    let mut app = open("test_data/nested.json", "id");
    let before = columns(&app);

    press(&mut app, "zw");

    assert_ne!(
        app.mode,
        AppMode::WindowFnSelect,
        "the picker must not open on a document sheet"
    );
    assert_eq!(columns(&app), before, "no column may be added");
    assert!(
        app.stack.active().doc_mapping_ok(),
        "the table must still match the document"
    );

    // The same key on a plain sheet does open the picker — otherwise this test
    // would pass on a `zw` that simply stopped working.
    let mut plain = open(SAMPLE, "salary");
    press(&mut plain, "zw");
    assert_eq!(plain.mode, AppMode::WindowFnSelect);
}

// ── adding a column must not throw the sheet's state away ───────────────────

/// A window column is an addition, not a reload. Sorting and then adding one
/// rebased `original_order` onto the sorted order, so `r` no longer restored
/// the file's own order — the row the user was looking for had moved for good.
#[test]
fn adding_a_window_column_keeps_the_original_row_order() {
    let mut app = open(SAMPLE, "age");

    let first_before = app
        .stack
        .active()
        .dataframe
        .get_physical(app.stack.active().dataframe.original_order[0], 1);

    press(&mut app, "]"); // sort by age, descending
    press(&mut app, "zf"); // add a share-of-total column
    press(&mut app, "r"); // and ask for the original order back

    let sheet = app.stack.active();
    assert_eq!(
        sheet
            .dataframe
            .get_physical(sheet.dataframe.row_order[0], 1),
        first_before,
        "`r` must restore the file's order, not the order at the time the column was added"
    );
}

/// Selecting rows and then adding a column silently dropped the selection, so
/// the `d` that followed deleted nothing.
#[test]
fn adding_a_window_column_keeps_the_selection() {
    let mut app = open(SAMPLE, "salary");
    // Sort first, so the rows are not in file order — otherwise carrying the
    // indices across unchanged would pass without meaning anything.
    press(&mut app, "]");
    press(&mut app, "s"); // select the row under the cursor
    press(&mut app, "j");
    press(&mut app, "s");
    assert_eq!(app.stack.active().dataframe.selected_rows.len(), 2);

    let marked: Vec<String> = {
        let df = &app.stack.active().dataframe;
        let mut names: Vec<String> = df
            .selected_rows
            .iter()
            .map(|p| df.get_physical(*p, 1))
            .collect();
        names.sort();
        names
    };

    press(&mut app, "zf");

    let df = &app.stack.active().dataframe;
    let mut still_marked: Vec<String> = df
        .selected_rows
        .iter()
        .map(|p| df.get_physical(*p, 1))
        .collect();
    still_marked.sort();

    assert_eq!(
        still_marked, marked,
        "the same rows must still be selected, not merely the same number"
    );
}

/// `zw` reached the partition picker through a gate written for `zF`, which
/// only lets numeric columns through. Eight of the twelve window functions read
/// no numbers — `row_number` reads no column at all — so most of the picker was
/// unusable on a text column, and refused with a message about percentages the
/// user had not asked for.
#[test]
fn the_window_picker_accepts_a_text_column_for_functions_that_do_not_need_numbers() {
    let mut app = open(SAMPLE, "name");

    press(&mut app, "zw");
    key(&mut app, KeyCode::Enter); // row_number, first in the list
    key(&mut app, KeyCode::Enter); // keep the table's order
    key(&mut app, KeyCode::Enter); // apply with no partition

    assert!(
        columns(&app).iter().any(|c| c.contains("row_number")),
        "row_number does not read the column at all: {:?}",
        columns(&app)
    );
}

/// A function that genuinely needs numbers still says so — and names itself
/// rather than talking about percent columns.
#[test]
fn a_numeric_window_on_a_text_column_names_the_function_that_refused() {
    let mut app = open(SAMPLE, "name");
    let before = columns(&app);

    press(&mut app, "zw");
    for _ in 0..3 {
        key(&mut app, KeyCode::Down); // cum_sum
    }
    key(&mut app, KeyCode::Enter); // pick it
    key(&mut app, KeyCode::Enter); // keep the table's order
    key(&mut app, KeyCode::Enter); // and try to apply

    assert_eq!(columns(&app), before, "nothing may be added");
    assert!(
        app.status_message.contains("cum_sum"),
        "the message must name what was refused: {}",
        app.status_message
    );
}

/// The new column belongs next to the one it describes. Appending it to the far
/// right puts it off-screen on a wide table.
#[test]
fn a_window_column_lands_beside_its_source() {
    let mut app = open(SAMPLE, "age");
    press(&mut app, "zf");

    let names = columns(&app);
    let age = names.iter().position(|c| c == "age").unwrap();
    assert_eq!(
        names[age + 1],
        "age_pct_of_total",
        "the new column should follow its source: {:?}",
        names
    );
}

// ── document sheets ─────────────────────────────────────────────────────────

/// A doc-backed sheet's table mirrors its document row for row and column for
/// column. `zf` has always refused to add a column to one; `zF` and `T` never
/// did, and after either the sheet was read-only for the rest of the session
/// and the result vanished at the next reprojection.
#[test]
fn the_operations_that_reshape_a_table_refuse_a_document_sheet() {
    for keys in ["zF", "T"] {
        let mut app = open("test_data/nested.json", "id");
        let before = columns(&app);

        press(&mut app, keys);
        // `zF` opens a picker first; carry it through to the point where the
        // column would be added, or the test proves nothing.
        if app.mode == AppMode::PartitionSelect {
            key(&mut app, KeyCode::Enter);
        }

        assert_eq!(
            columns(&app),
            before,
            "`{}` reshaped a document sheet",
            keys
        );
        assert!(
            app.stack.active().doc_mapping_ok(),
            "`{}` left the table disagreeing with its document",
            keys
        );
    }
}

/// And both still work where there is no document to break.
#[test]
fn they_still_work_on_a_plain_sheet() {
    let mut app = open(SAMPLE, "salary");
    press(&mut app, "zF");
    assert_eq!(app.mode, AppMode::PartitionSelect);

    let mut app = open(SAMPLE, "salary");
    press(&mut app, "T");
    assert_eq!(
        app.stack.active().dataframe.visible_row_count(),
        5,
        "five columns become five rows"
    );
}

// ── non-Latin layouts ───────────────────────────────────────────────────────

/// The layout map translates `х` and `ъ` to the brackets, but the re-dispatch
/// table had no bracket arms, so sorting was unreachable on a Cyrillic layout —
/// and with it the `z[` chord.
#[test]
fn sorting_works_on_a_cyrillic_layout() {
    let mut app = open(SAMPLE, "age");
    press(&mut app, "ъ"); // ] — sort descending

    assert_eq!(
        app.stack.active().sort_keys,
        vec![("age".to_string(), true)],
        "`ъ` is `]` on a Cyrillic layout"
    );
}

/// A chord's second key was never translated, because the layout map is applied
/// only in Normal mode. `я` reaches the z-prefix and then `ц` — which is `w` —
/// fell through to the cancel arm, so `zw`, `gb`, `z[` and `z]` could not be
/// typed at all.
#[test]
fn chords_work_on_a_cyrillic_layout() {
    let mut app = open(SAMPLE, "salary");
    press(&mut app, "яц"); // zw
    assert_eq!(
        app.mode,
        AppMode::WindowFnSelect,
        "`яц` is `zw` on a Cyrillic layout"
    );

    let mut app = open(SAMPLE, "department");
    press(&mut app, "ях"); // z[
    assert_eq!(app.stack.active().sort_keys.len(), 1);
}

// ── the direction picker ────────────────────────────────────────────────────

/// Read one column's values in display order.
fn cells(app: &tuitab::app::App, column: &str) -> Vec<String> {
    let df = &app.stack.active().dataframe;
    let col = df.column_index(column).unwrap();
    (0..df.visible_row_count())
        .map(|r| tuitab::data::dataframe::DataFrame::anyvalue_to_string_fmt(&df.get_val(r, col)))
        .collect()
}

/// A rank has an end it starts from, and the TUI had no way to say which. It
/// always ranked ascending while the server's `desc` could do either — the same
/// question answered differently depending on the surface.
#[test]
fn a_descending_rank_puts_the_largest_value_first() {
    let mut app = open(SAMPLE, "salary");

    press(&mut app, "zw");
    key(&mut app, KeyCode::Down); // rank
    key(&mut app, KeyCode::Enter);
    assert_eq!(
        app.mode,
        AppMode::WindowDirSelect,
        "a rank must ask which end is first"
    );
    key(&mut app, KeyCode::Down); // descending
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Enter); // no partition

    let salaries: Vec<f64> = cells(&app, "salary")
        .iter()
        .map(|v| v.parse().unwrap())
        .collect();
    let ranks = cells(&app, "salary_rank_desc");
    let top = salaries.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let row = salaries.iter().position(|s| *s == top).unwrap();

    assert_eq!(
        ranks[row], "1",
        "the largest salary ranks first: {:?}",
        ranks
    );
}

/// Ascending stays the default, matching what the server does when `desc` is
/// absent from the JSON. Two surfaces, one answer.
#[test]
fn the_default_direction_is_ascending_on_both_surfaces() {
    let mut app = open(SAMPLE, "salary");

    press(&mut app, "zw");
    key(&mut app, KeyCode::Down); // rank
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Enter); // take the highlighted direction as-is
    key(&mut app, KeyCode::Enter); // no partition

    let salaries: Vec<f64> = cells(&app, "salary")
        .iter()
        .map(|v| v.parse().unwrap())
        .collect();
    let ranks = cells(&app, "salary_rank");
    let bottom = salaries.iter().cloned().fold(f64::INFINITY, f64::min);
    let row = salaries.iter().position(|s| *s == bottom).unwrap();

    assert_eq!(
        ranks[row], "1",
        "the smallest salary ranks first: {:?}",
        ranks
    );
}

/// An aggregate over a set gets neither extra step: a sum is a sum whatever
/// order it is added in, and it has no end that comes first.
#[test]
fn an_aggregate_window_is_asked_about_neither_order_nor_direction() {
    let mut app = open(SAMPLE, "salary");

    press(&mut app, "zw");
    for _ in 0..6 {
        key(&mut app, KeyCode::Down); // sum
    }
    key(&mut app, KeyCode::Enter);

    assert_eq!(
        app.mode,
        AppMode::PartitionSelect,
        "a sum has no order and no direction to ask about"
    );
}

/// The third picker is a third place to press Esc, and the last one leaked:
/// a function left armed turned the next `zF` into a rank. Drive it to the
/// point where that would show.
#[test]
fn escaping_the_direction_picker_disarms_the_window_function() {
    let mut app = open(SAMPLE, "salary");

    press(&mut app, "zw");
    key(&mut app, KeyCode::Down); // rank
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Esc);

    assert!(
        app.pending_window_fn.is_none(),
        "cancelling must not leave a function armed"
    );

    press(&mut app, "zF");
    key(&mut app, KeyCode::Enter);
    assert!(
        columns(&app).iter().any(|c| c.ends_with("pct_of_total")),
        "zF must still mean a share: {:?}",
        columns(&app)
    );
}

/// A descending choice must not outlive its `zw`. It lives in `WindowFnState`
/// so that opening the picker resets it in the same line that resets the
/// highlighted row — but that only helps if the reset is actually there.
#[test]
fn a_direction_does_not_carry_over_to_the_next_window() {
    let mut app = open(SAMPLE, "salary");

    press(&mut app, "zw");
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Down); // descending
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Enter);

    // Second window, direction untouched: it must be ascending again.
    press(&mut app, "zw");
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Enter);
    assert!(
        !app.window_fn.desc,
        "the picker must open on ascending, not on the last choice"
    );
}

// ── repeating the column-move chord ─────────────────────────────────────────

/// `z` + arrow moves a column and leaves the app in `ColumnMove`, where a run of
/// bare arrows keeps reordering. But the chord people repeat is the whole `z` +
/// arrow, and that `z` used to fall through to the mode's catch-all and leave
/// it — so the arrow behind it moved the *cursor* instead. Every second chord
/// did nothing, and worse, the cursor came to rest on a neighbour.
#[test]
fn repeating_the_move_chord_keeps_moving_the_same_column() {
    let mut app = open(SAMPLE, "salary");
    let start = app.stack.active().cursor_col;
    assert!(
        start >= 3,
        "the fixture must have room to move left three times"
    );

    for _ in 0..3 {
        key(&mut app, KeyCode::Char('z'));
        key(&mut app, KeyCode::Left);
    }

    assert_eq!(
        columns(&app).iter().position(|c| c == "salary"),
        Some(start - 3),
        "three chords must move the column three places: {:?}",
        columns(&app)
    );
    assert_eq!(
        app.stack.active().cursor_col,
        start - 3,
        "the cursor must travel with the column it is moving"
    );
}

/// What made a pre-existing mode quirk into a visible bug: the compound sort is
/// new, so the drifted cursor now had `z[` to land on. After moving a column,
/// a sort chord must still mean the column that was moved.
#[test]
fn a_sort_chord_after_a_move_still_means_the_moved_column() {
    let mut app = open(SAMPLE, "salary");
    press(&mut app, "z]"); // sort by salary
    assert_eq!(
        app.stack.active().sort_keys,
        vec![("salary".to_string(), true)]
    );

    for _ in 0..3 {
        key(&mut app, KeyCode::Char('z'));
        key(&mut app, KeyCode::Left);
    }
    // And a further sort chord, which is what the user reaches for next.
    press(&mut app, "z[");

    assert_eq!(
        app.stack.active().sort_keys,
        vec![("salary".to_string(), false)],
        "the sort must stay on the column under the cursor, not wander"
    );
}

/// `z` inside `ColumnMove` re-opens the prefix rather than dismissing the mode.
#[test]
fn a_second_z_reopens_the_prefix_instead_of_leaving_column_move() {
    let mut app = open(SAMPLE, "salary");
    key(&mut app, KeyCode::Char('z'));
    key(&mut app, KeyCode::Left);
    assert_eq!(app.mode, AppMode::ColumnMove);

    key(&mut app, KeyCode::Char('z'));
    assert_eq!(app.mode, AppMode::ZPrefix);

    // And thinking better of it still gets out.
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.mode, AppMode::Normal, "Esc must leave the prefix");
}

/// And on a Cyrillic layout, where `я` is the `z` key. `ColumnMove` was not in
/// the set of modes the layout is translated for.
#[test]
fn the_move_chord_repeats_on_a_cyrillic_layout() {
    let mut app = open(SAMPLE, "salary");
    let start = app.stack.active().cursor_col;

    for _ in 0..2 {
        key(&mut app, KeyCode::Char('я'));
        key(&mut app, KeyCode::Left);
    }

    assert_eq!(
        columns(&app).iter().position(|c| c == "salary"),
        Some(start - 2),
        "`я` must reach the prefix from ColumnMove too: {:?}",
        columns(&app)
    );
}

// ── the order picker ────────────────────────────────────────────────────────

const GROWTH: &str = "test_data/growth.csv";

/// What the whole step exists for: a running total by date on a table the user
/// wants left exactly as it is. Before this, the only way to total by date was
/// to sort the table first — a change nobody asked for.
#[test]
fn a_running_total_can_be_ordered_by_a_column_from_the_picker() {
    let mut app = open(GROWTH, "amount");
    let dates_before = cells(&app, "date");

    press(&mut app, "zw");
    for _ in 0..3 {
        key(&mut app, KeyCode::Down); // cum_sum
    }
    key(&mut app, KeyCode::Enter);
    assert_eq!(
        app.mode,
        AppMode::WindowOrderSelect,
        "a running total must ask what 'before' means"
    );

    key(&mut app, KeyCode::Down); // off "(the table's order)" onto dept
    key(&mut app, KeyCode::Down); // date
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Enter); // ascending
    key(&mut app, KeyCode::Enter); // no partition

    let name = columns(&app)
        .into_iter()
        .find(|c| c.contains("cum_sum"))
        .expect("a column must have been added");
    assert!(
        name.contains("date"),
        "the order belongs in the name: {}",
        name
    );

    // No partition was picked, so this is the whole table. In date order the
    // amounts are 10, 7, 20, 8, 30, 9 and the total runs 10, 17, 37, 45, 75, 84
    // — then each answer goes back to the row it belongs to.
    assert_eq!(
        cells(&app, &name),
        vec!["75", "17", "10", "84", "37", "45"],
        "totals must follow the dates, not the file"
    );
    assert_eq!(
        cells(&app, "date"),
        dates_before,
        "and the table's own order must be untouched"
    );
}

/// Keeping the table's order is the first entry and skips the direction step,
/// because there is nothing to run in a direction.
#[test]
fn keeping_the_tables_order_goes_straight_to_the_partitions() {
    let mut app = open(GROWTH, "amount");
    press(&mut app, "zw");
    for _ in 0..3 {
        key(&mut app, KeyCode::Down); // cum_sum
    }
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Enter); // "(the table's order)"

    assert_eq!(app.mode, AppMode::PartitionSelect);
}

/// The fourth place to press Esc, and the same leak each time: a function left
/// armed turns the next `zF` into something else.
#[test]
fn escaping_the_order_picker_disarms_the_window_function() {
    let mut app = open(GROWTH, "amount");
    press(&mut app, "zw");
    for _ in 0..3 {
        key(&mut app, KeyCode::Down);
    }
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Esc);

    assert!(app.pending_window_fn.is_none());
    press(&mut app, "zF");
    key(&mut app, KeyCode::Enter);
    assert!(
        columns(&app).iter().any(|c| c.ends_with("pct_of_total")),
        "zF must still mean a share: {:?}",
        columns(&app)
    );
}

/// A chosen order must not outlive its `zw`, the same way the direction must not.
#[test]
fn an_order_does_not_carry_over_to_the_next_window() {
    let mut app = open(GROWTH, "amount");
    press(&mut app, "zw");
    for _ in 0..3 {
        key(&mut app, KeyCode::Down);
    }
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Down); // date
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Enter);

    press(&mut app, "zw");
    assert!(app.window_fn.order_by.is_none(), "the order must reset");
    assert_eq!(app.window_fn.order_index, 0, "and so must the highlight");
}
