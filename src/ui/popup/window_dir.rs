use crate::theme::EverforestTheme as T;
use crate::ui::popup::centered_rect;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};

/// Ask which end of the column ranks first.
///
/// Shown only for the functions where [`crate::data::window::WindowFn::uses_direction`]
/// is true, so the other ten keep the two-step flow they had.
pub fn render_window_dir_popup(frame: &mut Frame, app: &crate::app::App, area: Rect) {
    let popup_area = centered_rect(46, 22, area);
    frame.render_widget(Clear, popup_area);

    let function = app
        .pending_window_fn
        .map(|f| f.name())
        .unwrap_or("window function");

    let items: Vec<ListItem> = [
        ("▲ Ascending", "smallest value ranks 1", false),
        ("▼ Descending", "largest value ranks 1", true),
    ]
    .iter()
    .map(|(label, hint, desc)| {
        let active = *desc == app.window_fn.desc;
        let text = format!("{}{:<14} {}", if active { "> " } else { "  " }, label, hint);
        let mut style = Style::default().fg(T::FG);
        if active {
            style = style.bg(T::BG2);
        }
        ListItem::new(text).style(style)
    })
    .collect();

    let list = List::new(items).block(
        Block::default()
            .title(format!(" {}: direction (Enter, Esc to cancel) ", function))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(T::PURPLE)),
    );
    frame.render_widget(list, popup_area);
}
