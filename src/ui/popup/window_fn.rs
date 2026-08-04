use crate::theme::EverforestTheme as T;
use crate::ui::popup::centered_rect;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};

/// Pick which window function `zw` should add.
///
/// The list is [`crate::data::window::WindowFn::all`], so a function added for
/// the MCP server shows up here too rather than quietly being server-only.
pub fn render_window_fn_popup(frame: &mut Frame, app: &crate::app::App, area: Rect) {
    let popup_area = centered_rect(46, 60, area);
    frame.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = crate::data::window::WindowFn::all()
        .iter()
        .enumerate()
        .map(|(i, function)| {
            let active = i == app.window_fn.select_index;
            let text = format!(
                "{}{:<14} {}",
                if active { "> " } else { "  " },
                function.name(),
                function.describe()
            );
            let mut style = Style::default().fg(T::FG);
            if active {
                style = style.bg(T::BG2);
            }
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Window function (Enter to pick, Esc to cancel) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(T::PURPLE)),
    );
    frame.render_widget(list, popup_area);
}
