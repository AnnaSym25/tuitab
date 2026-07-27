//! A plain "pick one of these" popup, shared by the pickers that only need a list.

use crate::theme::EverforestTheme as T;
use crate::ui::popup::centered_rect;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};

/// Render a single-selection list. `items` are `(label, hint)`; the hint is dimmed and
/// explains what the choice produces, which is the whole point for save shapes.
pub fn render_choice_popup(
    frame: &mut Frame,
    title: &str,
    items: &[(String, String)],
    index: usize,
    area: Rect,
) {
    let popup_area = centered_rect(60, 30, area);
    frame.render_widget(Clear, popup_area);

    let width = items.iter().map(|(l, _)| l.chars().count()).max().unwrap_or(0);
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, (label, hint))| {
            let active = i == index;
            let text = format!(
                "{}{:<width$}  {}",
                if active { "▶ " } else { "  " },
                label,
                hint,
                width = width
            );
            let style = if active {
                Style::default().fg(T::YELLOW).bg(T::BG2)
            } else {
                Style::default().fg(T::FG)
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(list_items).block(
        Block::default()
            .title(format!(" {} (↑↓ navigate, Enter apply, Esc cancel) ", title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(T::PURPLE)),
    );
    frame.render_widget(list, popup_area);
}
