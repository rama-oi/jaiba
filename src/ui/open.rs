use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::Style,
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{App, OpenDatabaseStep};

pub fn draw_open_database(frame: &mut Frame, app: &mut App) {
    let area = frame.area().inner(Margin {
        horizontal: 2,
        vertical: 2,
    });
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(5),
            Constraint::Fill(1),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(60),
            Constraint::Fill(1),
        ])
        .split(vertical[1]);
    let input_area = horizontal[1];

    let (title, value, hint) = match app.open_database_step {
        OpenDatabaseStep::Path => (
            " Open .kdbx database ",
            app.open_path_buffer.as_str(),
            "path to the database",
        ),
        OpenDatabaseStep::Password => (
            " Master password ",
            app.open_password_buffer.as_str(),
            "password for this database",
        ),
        OpenDatabaseStep::Keyfile => (
            " Keyfile (optional) ",
            app.open_keyfile_buffer.as_str(),
            "leave empty for password-only databases",
        ),
    };
    let display = if app.open_database_step == OpenDatabaseStep::Password {
        "•".repeat(value.chars().count())
    } else {
        value.to_string()
    };

    let content = Paragraph::new(display).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.border))
            .title(title),
    );
    frame.render_widget(content, input_area);

    let hint_area = vertical[2];
    frame.render_widget(
        Paragraph::new(format!("{hint}\n[enter] next  [esc] cancel"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(app.theme.accent)),
        hint_area,
    );

    if !app.open_completion_candidates.is_empty() {
        let candidates = app
            .open_completion_candidates
            .iter()
            .take(5)
            .map(|candidate| format!("  {candidate}"))
            .collect::<Vec<_>>()
            .join("\n");
        frame.render_widget(
            Paragraph::new(candidates).style(Style::default().fg(app.theme.text)),
            vertical[3],
        );
    }
}
