//! That a popup reaches the screen.
//!
//! Every other test here drives state: which mode the app is in, what the frame
//! holds. None of them draw anything, so a picker wired into the key layer but
//! left out of `ui::render`'s dispatch passes all of them and shows the user
//! nothing. That is a whole class of bug — "the picker never appeared" — with
//! no coverage at all, so these render into a `TestBackend` and read the text
//! back off the buffer.

mod keys;

use crossterm::event::KeyCode;
use keys::{key, open, press};
use ratatui::{backend::TestBackend, Terminal};

const SAMPLE: &str = "test_data/sample.csv";

/// Draw a frame and return everything on it as text.
fn screen(app: &mut tuitab::app::App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal.draw(|f| tuitab::ui::render(f, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn the_window_function_picker_reaches_the_screen() {
    let mut app = open(SAMPLE, "salary");
    press(&mut app, "zw");
    let text = screen(&mut app);
    assert!(
        text.contains("pct_of_total"),
        "the function list did not draw"
    );
}

#[test]
fn the_direction_picker_reaches_the_screen() {
    let mut app = open(SAMPLE, "salary");
    press(&mut app, "zw");
    key(&mut app, KeyCode::Down); // rank
    key(&mut app, KeyCode::Enter);

    let text = screen(&mut app);
    assert!(
        text.contains("Ascending") && text.contains("Descending"),
        "the direction picker did not draw"
    );
    assert!(
        text.contains("rank"),
        "it must name the function it is asking about"
    );
}

/// The header is where the sort arrow lives, and it was being truncated away.
#[test]
fn a_sorted_column_shows_its_arrow_on_screen() {
    let mut app = open(SAMPLE, "age");
    press(&mut app, "]");
    assert!(
        screen(&mut app).contains('▼'),
        "no arrow on a sorted column"
    );

    press(&mut app, "[");
    assert!(screen(&mut app).contains('▲'), "no arrow after re-sorting");
}

#[test]
fn the_order_picker_reaches_the_screen() {
    let mut app = open("test_data/growth.csv", "amount");
    press(&mut app, "zw");
    for _ in 0..3 {
        key(&mut app, KeyCode::Down); // cum_sum
    }
    key(&mut app, KeyCode::Enter);

    let text = screen(&mut app);
    assert!(
        text.contains("the table's order"),
        "the order picker did not draw"
    );
    assert!(
        text.contains("date"),
        "it must list the columns to order by"
    );
    assert!(text.contains("cum_sum"), "and name the function it is for");
}
