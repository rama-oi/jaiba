use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
};

use crate::app::App;

pub fn draw_vaults(frame: &mut Frame, app: &mut App) {
    let unlocking = app.vault_unlocking.is_some();
    let input_height = if unlocking { 3 } else { 0 };
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(input_height),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let items = app.config.vaults.iter().map(|vault| {
        let unlocked = app.is_vault_unlocked(&vault.name);
        let active = app.active_vault_name.as_deref() == Some(vault.name.as_str());
        let state = if unlocked {
            "● unlocked"
        } else {
            "○ locked"
        };
        let active_label = if active { "  active" } else { "" };
        let state_style = if unlocked {
            Style::new().fg(app.theme.accent)
        } else {
            Style::new().fg(app.theme.warning)
        };

        ListItem::new(vec![
            Line::from(vec![
                Span::styled(format!("{state:<10}"), state_style),
                Span::styled(vault.name.clone(), Style::new().fg(app.theme.text).bold()),
                Span::styled(active_label, Style::new().fg(app.theme.accent)),
            ]),
            Line::from(Span::styled(
                format!("           {}", vault.path.display()),
                Style::new().fg(app.theme.header),
            )),
        ])
    });

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .border_style(Style::new().fg(app.theme.border))
                .title(" Vaults "),
        )
        .highlight_style(
            Style::new()
                .fg(app.theme.selection_fg)
                .bg(app.theme.selection_bg)
                .bold(),
        )
        .highlight_symbol("→ ");

    frame.render_stateful_widget(list, vertical[0], &mut app.vault_state);

    if let Some(name) = &app.vault_unlocking {
        let input_area = vertical[1];
        app.max_len = input_area.width.saturating_sub(4) as usize;
        let masked = "•".repeat(app.vault_password.chars().count());
        let input = Paragraph::new(masked.clone())
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .padding(Padding::horizontal(1))
                    .border_style(Style::new().fg(app.theme.border))
                    .title(format!(" Unlock {name} ")),
            );
        frame.render_widget(input, input_area);

        let typed_len = masked.chars().count() as u16;
        let cursor_x = input_area.x + 2 + typed_len.min(input_area.width.saturating_sub(4));
        frame.set_cursor_position((cursor_x, input_area.y + 1));
    }

    let footer = if let Some(status) = &app.status {
        Paragraph::new(format!("  {status}")).style(Style::new().fg(app.theme.error))
    } else if unlocking {
        Paragraph::new("  [enter] unlock  [esc] cancel").style(Style::new().fg(app.theme.accent))
    } else {
        Paragraph::new("  [↑↓] navigate  [enter] switch / unlock  [esc] back")
            .style(Style::new().fg(app.theme.accent))
    };
    frame.render_widget(footer, vertical[2]);
}
