use std::time::Instant;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table},
};

use crate::app::App;
use crate::db::Entry;
use crate::theme::Theme;
use crate::util::wrap_help_items;

fn help_items(slim_mode: bool) -> &'static [&'static str] {
    if slim_mode {
        &[
            "[↑↓]", "[^u]", "[^p]", "[^t]", "[^r]", "[^a]", "[^s]", "[enter]", "[esc]",
        ]
    } else {
        &[
            "[↑↓] navigate",
            "[^u] cp_user",
            "[^p] cp_password",
            "[^t] cp_totp",
            "[^r] cp_url",
            "[^a] add_entry",
            "[^s] settings",
            "[enter] expand_entry",
            "[esc] quit",
        ]
    }
}

fn masked_password<'a>(entry: &'a Entry, theme: &Theme) -> Line<'a> {
    let normal = Style::new().fg(theme.text);
    let warning = Style::new().fg(theme.warning);

    let mut spans = vec![Span::styled("•••••", normal)];

    if entry.password_reuse_count > 1 {
        spans.push(Span::styled(
            format!(" [{}]", entry.password_reuse_count),
            warning,
        ));
    }

    Line::from(spans)
}

fn masked_user<'a>(entry: &'a Entry, theme: &Theme) -> Line<'a> {
    let normal = Style::new().fg(theme.text);
    let warning = Style::new().fg(theme.warning);

    let mut spans = vec![Span::styled(entry.user.as_str(), normal)];

    if entry.duplicate_user_count > 1 {
        spans.push(Span::styled(
            format!(" [{}]", entry.duplicate_user_count),
            warning,
        ));
    }

    Line::from(spans)
}

fn slim_row<'a>(entry: &'a Entry, theme: &Theme) -> Line<'a> {
    let normal = Style::new().fg(theme.text);
    let warning = Style::new().fg(theme.warning);

    let mut spans = vec![Span::styled(format!("  {}", entry.name), normal)];

    if entry.duplicate_user_count > 1 {
        spans.push(Span::styled(
            format!(" u[{}]", entry.duplicate_user_count),
            warning,
        ));
    }

    if entry.password_reuse_count > 1 {
        spans.push(Span::styled(
            format!(" p[{}]", entry.password_reuse_count),
            warning,
        ));
    }

    Line::from(spans)
}

pub fn draw_index(frame: &mut Frame, app: &mut App) {
    let full_area = frame.area();

    let help_width = full_area.width.saturating_sub(2);
    let help_lines = wrap_help_items(help_items(app.slim_mode), help_width);
    let help_height = help_lines.len() as u16;

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(help_height),
        ])
        .split(full_area);

    let query_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(vertical[0]);

    let separator = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::new().fg(app.theme.border));

    frame.render_widget(separator, vertical[2]);

    let help_text = help_lines
        .iter()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n");

    let help = if let Some(timer) = &app.clipboard_timer {
        let remaining = timer
            .clear_at
            .saturating_duration_since(Instant::now())
            .as_secs()
            + 1; // round up so it doesn't flash "0s" before the tick that clears it

        Paragraph::new(format!(
            "  Copied {} :: clearing in {remaining}s",
            timer.label
        ))
        .style(Style::new().fg(app.theme.warning))
    } else if let Some(status) = &app.status {
        // Replace the help text with the latest status message.
        Paragraph::new(format!("  {status}")).style(Style::new().fg(app.theme.warning))
    } else {
        Paragraph::new(help_text).style(Style::new().fg(app.theme.accent))
    };

    frame.render_widget(help, vertical[3]);

    let rows = app.filtered.iter().map(|&i| {
        let entry = &app.entries[i];
        let password = masked_password(entry, &app.theme);
        let user = masked_user(entry, &app.theme);

        if app.slim_mode {
            Row::new([Cell::from(slim_row(entry, &app.theme))])
        } else {
            Row::new([
                Cell::from(format!("  {}", entry.name)),
                Cell::from(user),
                Cell::from(password),
                Cell::from(entry.date_last_modify.as_str()),
            ])
        }
    });

    let column_widths = if app.slim_mode {
        vec![Constraint::Percentage(100)]
    } else {
        vec![
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
        ]
    };

    let mut table_index = Table::new(rows, column_widths)
        .column_spacing(1)
        .style(Style::new().fg(app.theme.text))
        .row_highlight_style(
            Style::new()
                .fg(app.theme.selection_fg)
                .bg(app.theme.selection_bg)
                .bold(),
        );

    if !app.slim_mode {
        table_index = table_index.header(
            Row::new(["  Name", "User", "Password", "Last Modify"])
                .style(Style::new().bold().fg(app.theme.header))
                .bottom_margin(1),
        );
    }

    let index_area = vertical[1];

    frame.render_stateful_widget(table_index, index_area, &mut app.index_state);

    let query_area = query_row[1];

    app.max_len = query_area.width.saturating_sub(4) as usize;

    let input_text = app.query.clone();
    let input_style = Style::default().fg(app.theme.text);

    let input = Paragraph::new(input_text.as_str())
        .style(input_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(app.theme.border)),
        );

    frame.set_cursor_position((
        query_area.x + 2 + input_text.chars().count() as u16,
        query_area.y + 1,
    ));

    frame.render_widget(input, query_area);
}
