use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::app::App;
use crate::input::login::database_missing;
use crate::util::default_new_database_path;

pub fn draw_login(frame: &mut Frame, app: &mut App) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(20),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .split(frame.area());

    let logo_area = vertical[1];
    let input_row = vertical[2];

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(50),
            Constraint::Fill(1),
        ])
        .split(input_row);

    let input_area = horizontal[1];

    // Calculate available input characters
    app.max_len = (input_area.width - 2) as usize;

    let missing = database_missing(app);

    enum BoxContent<'a> {
        Masked(&'a str),
        Static(&'a str),
    }

    let (title, content) = if app.creating_database {
        if app.confirming_new_db_password {
            (
                " Confirm master password ",
                BoxContent::Masked(&app.new_db_confirm),
            )
        } else {
            (" New master password ", BoxContent::Masked(&app.password))
        }
    } else if missing {
        (
            " No database found ",
            BoxContent::Static("Press [n] to create a new database"),
        )
    } else {
        ("", BoxContent::Masked(&app.password))
    };

    let input_text = match content {
        BoxContent::Masked(buf) => "•".repeat(buf.chars().count()),
        BoxContent::Static(text) => text.to_string(),
    };

    let input = Paragraph::new(input_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.border))
                .title(title),
        );

    let claws_style = Style::default().fg(app.theme.claws);
    let claws_light_style = Style::default().fg(app.theme.claws_light);
    let claws_shadow_style = Style::default().fg(app.theme.claws_shadow);
    let shell_style = Style::default().fg(app.theme.shell);
    let shell_light_style = Style::default().fg(app.theme.shell_light);
    let shell_shadow_style = Style::default().fg(app.theme.shell_shadow);

    let logo = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("         ███", claws_light_style),
            Span::styled("                  ███         ", claws_style),
        ]),
        Line::from(vec![
            Span::styled("        ██", claws_light_style),
            Span::styled("█                    ███        ", claws_style),
        ]),
        Line::from(vec![
            Span::styled("       ██", claws_light_style),
            Span::styled("█   █              █   ███       ", claws_style),
        ]),
        Line::from(vec![
            Span::styled("      ██", claws_light_style),
            Span::styled("█  ██                ██  ███      ", claws_style),
        ]),
        Line::from(vec![
            Span::styled("      ██", claws_light_style),
            Span::styled("███                    █████      ", claws_style),
        ]),
        Line::from(vec![
            Span::styled(" ██", claws_light_style),
            Span::styled("    ██          ", claws_style),
            Span::styled("█  █", claws_light_style),
            Span::styled("          ██    ██ ", claws_style),
        ]),
        
        // --------------here
        Line::from(vec![
            Span::styled("██", claws_light_style),
            Span::styled("█     ██         ", claws_style),
            Span::styled("█  █", claws_style),
            Span::styled("         ██     ███", claws_style),
        ]),


        Line::from(vec![
            Span::styled("██", claws_light_style),
            Span::styled("██     ██  ", claws_style),
            Span::styled("████████████████", shell_light_style),
            Span::styled("  ██     ████", claws_style),
        ]),
        Line::from(vec![
            Span::styled("  ██", claws_light_style),
            Span::styled("██     ", claws_style),
            Span::styled("██", shell_light_style),
            Span::styled("██████████████████", shell_style),
            Span::styled("     ████  ", claws_style),
        ]),
        Line::from(vec![
            Span::styled("    ████  ", claws_style),
            Span::styled("██", shell_light_style),
            Span::styled("████████████████████", shell_style),
            Span::styled("  ████    ", claws_style),
        ]),
        Line::from(vec![
            Span::styled("       ███", claws_style),
            Span::styled("██", shell_light_style),
            Span::styled("████████████████████", shell_style),
            Span::styled("███       ", claws_style),
        ]),
        Line::from(vec![
            Span::styled("          ████████████████████", shell_style),
            Span::styled("██          ", shell_shadow_style),
        ]),
        Line::from(vec![
            Span::styled("       ███", claws_style),
            Span::styled("████████████████████", shell_style),
            Span::styled("██", shell_shadow_style),
            Span::styled("███       ", claws_shadow_style),
        ]),
        Line::from(vec![
            Span::styled("     ███  ██", claws_style),
            Span::styled("████████████████", shell_style),
            Span::styled("██", shell_shadow_style),
            Span::styled("██  ███     ", claws_shadow_style),
        ]),
        Line::from(vec![
            Span::styled("    ██   ██  ", claws_style),
            Span::styled("████████████████", shell_shadow_style),
            Span::styled("  ██   ██    ", claws_shadow_style),
        ]),
        Line::from(vec![
            Span::styled("     █  ██    ", claws_style),
            Span::styled("██          ██    ██   █    ", claws_shadow_style),
        ]),
        Line::from(vec![
            Span::styled("         █   ", claws_style),
            Span::styled("██            ██    █        ", claws_shadow_style),
        ]),
        Line::from(vec![Span::styled(
            "              █            █              ",
            claws_shadow_style,
        )]),
    ])
    .alignment(Alignment::Center)
    .style(claws_style);

    frame.render_widget(logo, logo_area);

    frame.render_widget(input, input_area);

    if let Some(error) = &app.login_error {
        let error_area = Rect {
            x: input_area.x,
            y: input_area.y + input_area.height,
            width: input_area.width,
            height: 3,
        };

        let error_text = Paragraph::new(error.as_str())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(Style::new().fg(app.theme.error));

        frame.render_widget(error_text, error_area);
    } else if app.creating_database {
        let path = app
            .config
            .default_database
            .clone()
            .unwrap_or_else(default_new_database_path);

        let hint_area = Rect {
            x: input_area.x,
            y: input_area.y + input_area.height,
            width: input_area.width,
            height: 2,
        };

        let hint = Paragraph::new(vec![
            Line::from(format!("will create {}", path.display())),
            Line::from("[enter] confirm  [esc] cancel"),
        ])
        .alignment(Alignment::Center)
        .style(Style::new().fg(app.theme.warning));

        frame.render_widget(hint, hint_area);
    } else if missing {
        let hint_area = Rect {
            x: input_area.x,
            y: input_area.y + input_area.height,
            width: input_area.width,
            height: 1,
        };

        let hint = Paragraph::new("[n] create a new database  [esc] quit")
            .alignment(Alignment::Center)
            .style(Style::new().fg(app.theme.warning));

        frame.render_widget(hint, hint_area);
    }

    // Keep cursor inside the box, but only while actually typing into it.
    let typed_len = if app.creating_database {
        if app.confirming_new_db_password {
            app.new_db_confirm.chars().count() as u16
        } else {
            app.password.chars().count() as u16
        }
    } else if missing {
        // Static prompt, nothing to place a cursor in.
        return;
    } else {
        app.password.chars().count() as u16
    };

    let inner_width = input_area.width - 1;
    let cursor_x = input_area.x + (inner_width.saturating_sub(typed_len) / 2) + typed_len;

    frame.set_cursor_position((cursor_x, input_area.y + 1));
}
