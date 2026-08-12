//! The one thing that has to work for copy and paste to mean anything: a value
//! copied out of a cell can be pasted into another one.
//!
//! This test talks to the real system clipboard — there is no other way to cover
//! the bug it guards, which was tuitab closing its clipboard connection the moment
//! a copy finished, so the text was gone before anyone could paste it. On a machine
//! with no clipboard at all (a headless CI runner) it skips rather than fails, and
//! it puts back whatever the user had copied before it ran.

mod keys;

use crossterm::event::KeyCode;
use keys::{key, open, press};

#[test]
fn a_copied_cell_can_be_pasted_into_another_cell() {
    let saved = tuitab::clipboard::paste_from_clipboard().ok();
    if tuitab::clipboard::copy_text("tuitab clipboard probe").is_err() {
        eprintln!("no system clipboard here — skipping");
        return;
    }

    // salary is a float column shown to two decimals; the copy must carry the
    // value, not the rendering of it.
    let mut app = open("test_data/sample.csv", "salary");
    app.stack.active_mut().table_state.select(Some(0));
    press(&mut app, "yc");
    assert_eq!(
        tuitab::clipboard::paste_from_clipboard().unwrap(),
        "75000",
        "the clipboard must still hold the copied value after the copy returns"
    );

    app.stack.active_mut().table_state.select(Some(1));
    key(&mut app, KeyCode::Char('P'));
    let df = &app.stack.active().dataframe;
    let salary = df.column_index("salary").unwrap();
    assert_eq!(
        df.get_editable(1, salary),
        "75000",
        "P must write the clipboard into the cell under the cursor, got: {}",
        app.status_message
    );

    if let Some(s) = saved {
        let _ = tuitab::clipboard::copy_text(&s);
    }
}
