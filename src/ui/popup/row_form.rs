use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::app_state::RowFormState;
use crate::data::column::ColumnMeta;
use crate::theme::EverforestTheme as T;
use crate::ui::popup::centered_rect;

/// The new-row form (`O`): one line per column, the focused field carrying the cursor.
///
/// Which fields are on screen is worked out here from the focus rather than kept on
/// the state: only the renderer knows how tall the popup came out, and a scroll offset
/// stored alongside it would be a second copy of the same fact, one frame stale.
pub fn render_row_form_popup(
    frame: &mut Frame,
    state: &RowFormState,
    columns: &[ColumnMeta],
    area: Rect,
) {
    let n = state.fields.len().min(columns.len());

    // A table of four columns should not get the popup a table of forty needs, so the
    // box is only as tall as its fields — two borders and the footer line on top.
    let base = centered_rect(60, 70, area);
    let wanted = (n as u16).saturating_add(3).clamp(4, area.height);
    let h = wanted.min(base.height.max(4)).min(area.height);
    let popup_area = Rect {
        x: base.x,
        y: base.y + base.height.saturating_sub(h) / 2,
        width: base.width,
        height: h,
    };
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" New row (↑↓/Tab field, ←→ cursor, Enter insert, Esc cancel) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(T::PURPLE));

    // Two borders, and the last inner line is the hint or the error.
    let inner_h = popup_area.height.saturating_sub(2) as usize;
    let rows = inner_h.saturating_sub(1).max(1);

    // Keep the focused field roughly in the middle of the window, clamped at both ends.
    let start = if n <= rows {
        0
    } else {
        state.focus.saturating_sub(rows / 2).min(n - rows)
    };
    let end = (start + rows).min(n);

    let name_w = columns[..n]
        .iter()
        .map(|c| c.name.width())
        .max()
        .unwrap_or(4)
        .clamp(4, 20);

    let mut lines: Vec<Line> = Vec::with_capacity(rows + 1);
    for (i, meta) in columns.iter().enumerate().take(end).skip(start) {
        let focused = i == state.focus;
        let bad = state.errors.get(i).and_then(|e| e.as_ref()).is_some();

        let mut name = meta.name.clone();
        if name.width() > name_w {
            name.truncate(
                name.char_indices()
                    .map(|(b, _)| b)
                    .nth(name_w - 1)
                    .unwrap_or(name.len()),
            );
            name.push('…');
        }
        let pad = name_w.saturating_sub(name.width());

        let label_style = if bad {
            Style::default().fg(T::RED)
        } else if focused {
            Style::default().fg(T::YELLOW)
        } else {
            Style::default().fg(T::FG)
        };
        let value_style = if bad {
            Style::default().fg(T::RED)
        } else {
            Style::default().fg(T::FG)
        };
        let line_style = if focused {
            label_style.bg(T::BG2).add_modifier(Modifier::BOLD)
        } else {
            label_style
        };

        lines.push(Line::from(vec![
            Span::styled(if focused { "> " } else { "  " }, line_style),
            Span::styled(format!("{} ", meta.col_type.icon()), line_style),
            Span::styled(format!("{}{} ", name, " ".repeat(pad)), line_style),
            Span::styled("│ ", Style::default().fg(T::BG2)),
            Span::styled(state.fields[i].as_str().to_string(), value_style),
        ]));
    }

    // The bottom line explains the focused field, says what is wrong with it, or — once
    // Enter has asked — carries the question about the blank fields.
    let empty = state
        .fields
        .iter()
        .filter(|f| f.as_str().trim().is_empty())
        .count();
    let footer = if state.confirm_empty {
        Line::from(Span::styled(
            format!(
                "  {} of {} fields empty → NULL.  Enter again to insert.",
                empty, n
            ),
            Style::default().fg(T::ORANGE).add_modifier(Modifier::BOLD),
        ))
    } else {
        match state.errors.get(state.focus).and_then(|e| e.as_ref()) {
            Some(err) => Line::from(Span::styled(
                format!("  {}", err),
                Style::default().fg(T::RED),
            )),
            None => {
                let hint = columns
                    .get(state.focus)
                    .map(|c| format!("  {} · {} — empty leaves NULL", c.name, c.col_type.name()))
                    .unwrap_or_default();
                Line::from(Span::styled(hint, Style::default().fg(T::GREY1)))
            }
        }
    };
    lines.push(footer);

    frame.render_widget(Paragraph::new(lines).block(block), popup_area);

    // "> " + icon and space + the name column and its space + "│ "
    let value_x = 2 + 2 + name_w as u16 + 1 + 2;
    if (start..end).contains(&state.focus) {
        let row = (state.focus - start) as u16;
        frame.set_cursor_position((
            popup_area.x + 1 + value_x + state.fields[state.focus].cursor_pos(),
            popup_area.y + 1 + row,
        ));
    }
}
