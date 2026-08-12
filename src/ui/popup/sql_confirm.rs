//! The last thing between an edited sheet and the user's database.
//!
//! Shows the statements that will run, not a summary of them: the whole point is that
//! nothing reaches the file the user did not read first.  Long lists scroll rather than
//! being truncated, because a confirmation the user could not see all of is not one —
//! and a plan too large to hold readable text for says how many it is not showing.

use crate::data::io::db_write::StmtKind;
use crate::theme::EverforestTheme as T;
use crate::ui::popup::centered_rect;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Break a statement across lines at `width`, indenting the continuations so the start
/// of each statement stays findable while scrolling.
fn wrap(sql: &str, width: usize) -> Vec<String> {
    if width < 8 {
        return vec![sql.to_string()];
    }
    let mut out = Vec::new();
    let mut rest: Vec<char> = sql.chars().collect();
    let mut first = true;
    while !rest.is_empty() {
        let budget = if first {
            width
        } else {
            width.saturating_sub(4)
        };
        let take = budget.min(rest.len());
        let chunk: String = rest.drain(..take).collect();
        out.push(if first {
            chunk
        } else {
            format!("    {}", chunk)
        });
        first = false;
    }
    out
}

/// Lines to draw, and how many there are — the caller clamps its scroll against the
/// count, which only this function knows.
fn build_lines(app: &crate::app::App, width: usize) -> Vec<Line<'static>> {
    let Some(plan) = app.sql.plan.as_ref() else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    if plan.rebuild {
        // The user is about to read a DROP TABLE. Say what it is part of, first.
        for text in wrap(
            "-- this table will be rebuilt: dropped and created again, with its rows copied",
            width,
        ) {
            lines.push(Line::from(Span::styled(
                text,
                Style::default().fg(T::RED).add_modifier(Modifier::BOLD),
            )));
        }
    }
    // What the statements do not spell out, above them where it is read first.
    for warning in &plan.warnings {
        for text in wrap(&format!("-- {}", warning), width) {
            lines.push(Line::from(Span::styled(
                text,
                Style::default().fg(T::RED).add_modifier(Modifier::BOLD),
            )));
        }
    }
    for stmt in &plan.stmts {
        // Past the cap the plan keeps no readable text; those are counted below rather
        // than shown as blank lines.
        if stmt.display.is_empty() {
            continue;
        }
        let colour = match stmt.kind {
            StmtKind::Update => T::YELLOW,
            StmtKind::Insert => T::GREEN,
            StmtKind::Delete => T::RED,
            StmtKind::Schema => T::PURPLE,
        };
        for text in wrap(&stmt.display, width) {
            lines.push(Line::from(Span::styled(text, Style::default().fg(colour))));
        }
    }
    // A limit the user can see beats one they cannot: the statements are still going to
    // run, and saying nothing would read as "that was all of them".
    let hidden = plan.hidden_stmts();
    if hidden > 0 {
        for text in wrap(
            &format!(
                "-- and {} more statement{} not shown — they will run too",
                hidden,
                if hidden == 1 { "" } else { "s" }
            ),
            width,
        ) {
            lines.push(Line::from(Span::styled(
                text,
                Style::default().fg(T::RED).add_modifier(Modifier::BOLD),
            )));
        }
    }
    lines
}

pub fn render_sql_confirm(frame: &mut Frame, app: &crate::app::App, area: Rect) {
    let popup_area = centered_rect(80, 70, area);
    frame.render_widget(Clear, popup_area);

    let inner_width = popup_area.width.saturating_sub(2) as usize;
    let visible = popup_area.height.saturating_sub(2) as usize;
    let lines = build_lines(app, inner_width);

    let max_scroll = lines.len().saturating_sub(visible);
    // The key handler has no idea how tall the terminal is or how far the statements
    // wrapped, so it reads this back rather than guessing.
    app.sql.max_scroll.set(max_scroll);
    let scroll = app.sql.scroll.min(max_scroll);
    let above = scroll;
    let below = lines.len().saturating_sub(scroll + visible);

    let (target, summary) = match (app.sql.path.as_ref(), app.sql.plan.as_ref()) {
        (Some(p), Some(plan)) => (
            p.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.to_string_lossy().into_owned()),
            plan.summary(),
        ),
        _ => (String::new(), String::new()),
    };
    let table = app
        .stack
        .active()
        .table_source
        .as_ref()
        .map(|s| s.table.clone())
        .unwrap_or_default();

    let mut title = format!(" {} → {} · {} ", target, table, summary);
    match (above > 0, below > 0) {
        (true, true) => title.push_str(&format!("↑{} ↓{} ", above, below)),
        (true, false) => title.push_str(&format!("↑{} ", above)),
        (false, true) => title.push_str(&format!("↓{} ", below)),
        (false, false) => {}
    }

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        // Red, like the quit confirmation: this one writes to the user's database.
        .border_style(Style::default().fg(T::RED))
        .title_bottom(Line::from(Span::styled(
            " ↑↓ · PgUp/PgDn · g/G · Enter run · Esc cancel ",
            Style::default().fg(T::GREY1).add_modifier(Modifier::ITALIC),
        )));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(T::BG0))
        .scroll((scroll as u16, 0));
    frame.render_widget(paragraph, popup_area);
}
