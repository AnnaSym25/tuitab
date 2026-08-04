use crate::theme::EverforestTheme as T;
use crate::ui::popup::centered_rect;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};

/// Ask which column puts the rows in order for the window.
///
/// Shown for the functions where [`crate::data::window::WindowFn::uses_order_by`]
/// is true — a running total, a neighbour, a position. The first entry keeps the
/// table's own order, which is what these did before there was anywhere to say
/// otherwise.
///
/// The table is not re-sorted either way: the order is used to compute the
/// column and the answer comes back where the rows already are.
pub fn render_window_order_popup(frame: &mut Frame, app: &crate::app::App, area: Rect) {
    let popup_area = centered_rect(56, 60, area);
    frame.render_widget(Clear, popup_area);

    let function = app
        .pending_window_fn
        .map(|f| f.name())
        .unwrap_or("window function");

    let mut rows: Vec<(String, String)> = vec![(
        "(the table's order)".to_string(),
        "rows as they sit now".to_string(),
    )];
    rows.extend(
        app.stack
            .active()
            .dataframe
            .columns
            .iter()
            .map(|c| (c.name.clone(), format!("order by {}", c.col_type.name()))),
    );

    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(i, (label, hint))| {
            let active = i == app.window_fn.order_index;
            let text = format!("{}{:<24} {}", if active { "> " } else { "  " }, label, hint);
            let mut style = Style::default().fg(T::FG);
            if active {
                style = style.bg(T::BG2);
            }
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(format!(
                " {}: order the rows by (Enter, Esc to cancel) ",
                function
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(T::PURPLE)),
    );
    frame.render_widget(list, popup_area);
}
