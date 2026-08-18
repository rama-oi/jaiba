use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
};

use crate::app::App;
use crate::input::settings::{AUTO_LOCK_ROW, CLIPBOARD_TIMEOUT_ROW, DATABASE_ROW, KEYFILE_ROW};
use crate::util::wrap_help_items;

const NAV_HELP_ITEMS: &[&str] = &["[↑↓] navigate", "[enter] edit / choose theme", "[esc] back"];
const FIELD_HELP_ITEMS: &[&str] = &["[enter] save", "[esc] cancel"];
const THEME_PICKER_HELP_ITEMS: &[&str] = &["[↑↓] navigate", "[enter] apply", "[esc] cancel"];
const CHANGE_PASSWORD_HELP_ITEMS: &[&str] = &["[enter] confirm", "[esc] cancel"];
const EXPORT_HELP_ITEMS: &[&str] = &["[enter] continue", "[esc] cancel"];

pub fn draw_settings(frame: &mut Frame, app: &mut App) {
    if app.exporting_database {
        draw_export(frame, app);
    } else if app.importing_database {
        draw_import(frame, app);
    } else if app.changing_password {
        draw_change_password(frame, app);
    } else if app.choosing_theme {
        draw_theme_picker(frame, app);
    } else {
        draw_main_settings(frame, app);
    }
}

fn draw_main_settings(frame: &mut Frame, app: &mut App) {
    let full_area = frame.area();

    let editing_field = app.editing_field;
    let field_buffer = app.field_buffer.clone();
    let selected = app.settings_state.selected().unwrap_or(0);

    let help_items = if editing_field {
        FIELD_HELP_ITEMS
    } else {
        NAV_HELP_ITEMS
    };
    let help_width = full_area.width.saturating_sub(2);
    let help_lines = wrap_help_items(help_items, help_width);
    let help_height = help_lines.len() as u16;

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(help_height)])
        .split(full_area);

    let theme = &app.theme;
    let normal = Style::new().fg(theme.text);
    let accent = Style::new().fg(theme.accent);
    let warning = Style::new().fg(theme.warning);
    let border_style = Style::new().fg(theme.border);
    let placeholder = Style::new().fg(theme.border).italic();
    let editing_style = Style::new().fg(theme.selection_fg).bg(theme.selection_bg);
    let label_style = Style::new().fg(theme.header).bold();

    let render_value = |field_index: usize, value: String| -> Line<'static> {
        if editing_field && selected == field_index {
            return Line::from(vec![
                Span::styled(field_buffer.clone(), editing_style),
                Span::styled("▏", accent),
            ]);
        }

        if value.is_empty() {
            return Line::from(Span::styled("(not set)", placeholder));
        }

        Line::from(Span::styled(value, normal))
    };

    let field = |label: &'static str, value: Line<'static>| -> ListItem<'static> {
        ListItem::new(vec![
            Line::from(Span::styled(label, label_style)),
            value,
            Line::from(""), // spacer
        ])
    };

    let database_value = app
        .active_vault_config()
        .map(|vault| vault.path.display().to_string())
        .unwrap_or_default();
    let keyfile_value = app
        .active_vault_config()
        .and_then(|vault| vault.keyfile.as_ref())
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    let auto_lock_value = app.config.auto_lock.as_secs().to_string();
    let clipboard_timeout_value = app.config.clipboard_timeout.as_secs().to_string();

    let theme_value = if app.available_themes.is_empty() {
        Line::from(Span::styled(
            "no themes found in ~/.config/jaiba/themes",
            warning,
        ))
    } else {
        match app.config.theme.as_deref() {
            Some(name) => Line::from(Span::styled(name.to_string(), normal)),
            None => Line::from(Span::styled("(not set)", placeholder)),
        }
    };

    let items = vec![
        field(
            "Active vault database",
            render_value(DATABASE_ROW, database_value),
        ),
        field(
            "Keyfile (optional)",
            render_value(KEYFILE_ROW, keyfile_value),
        ),
        field(
            "Auto-lock (seconds)",
            render_value(AUTO_LOCK_ROW, auto_lock_value),
        ),
        field(
            "Clipboard timeout (seconds)",
            render_value(CLIPBOARD_TIMEOUT_ROW, clipboard_timeout_value),
        ),
        field("Theme", theme_value),
        field(
            "Change master password",
            Line::from(Span::styled("[enter] to change", placeholder)),
        ),
        field(
            "Import database",
            Line::from(Span::styled("[enter] to import", placeholder)),
        ),
        field(
            "Export database",
            Line::from(Span::styled("[enter] to export", placeholder)),
        ),
    ];

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .border_style(border_style)
                .title(" Settings "),
        )
        .highlight_style(editing_style.bold())
        .highlight_symbol("→ ");

    frame.render_stateful_widget(list, vertical[0], &mut app.settings_state);

    let help = if let Some(status) = &app.status {
        Paragraph::new(format!("  {status}")).style(warning)
    } else {
        let help_text = help_lines
            .iter()
            .map(|line| format!("  {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        Paragraph::new(help_text).style(accent)
    };

    frame.render_widget(help, vertical[1]);
}

fn draw_change_password(frame: &mut Frame, app: &mut App) {
    let full_area = frame.area();

    let help_width = full_area.width.saturating_sub(2);
    let help_lines = wrap_help_items(CHANGE_PASSWORD_HELP_ITEMS, help_width);
    let help_height = help_lines.len() as u16;

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(help_height),
        ])
        .split(full_area);

    let input_row = vertical[1];

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(50),
            Constraint::Fill(1),
        ])
        .split(input_row);

    let input_area = horizontal[1];
    app.max_len = (input_area.width - 2) as usize;

    let title = match app.password_change_step {
        crate::app::PasswordChangeStep::CurrentPassword => " Current master password ",
        crate::app::PasswordChangeStep::NewPassword => " New master password ",
        crate::app::PasswordChangeStep::ConfirmNewPassword => " Confirm new master password ",
    };

    let buf = match app.password_change_step {
        crate::app::PasswordChangeStep::CurrentPassword => &app.current_password_buffer,
        crate::app::PasswordChangeStep::NewPassword => &app.new_password_buffer,
        crate::app::PasswordChangeStep::ConfirmNewPassword => &app.new_password_confirm,
    };

    let input = Paragraph::new("•".repeat(buf.chars().count()))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.border))
                .title(title),
        );

    frame.render_widget(input, input_area);

    let hint_area = Rect {
        x: input_area.x,
        y: input_area.y + input_area.height,
        width: input_area.width,
        height: 1,
    };

    let default_hint = match app.password_change_step {
        crate::app::PasswordChangeStep::CurrentPassword => {
            "verify it's you before changing the master password"
        }
        crate::app::PasswordChangeStep::NewPassword
        | crate::app::PasswordChangeStep::ConfirmNewPassword => {
            "this re-encrypts your database file with the new password"
        }
    };
    let hint_text = app
        .status
        .clone()
        .unwrap_or_else(|| default_hint.to_string());
    let hint_style = if app.status.is_some() {
        Style::new().fg(app.theme.error)
    } else {
        Style::new().fg(app.theme.warning)
    };

    let hint = Paragraph::new(hint_text)
        .alignment(Alignment::Center)
        .style(hint_style);

    frame.render_widget(hint, hint_area);

    let typed_len = buf.chars().count() as u16;
    let inner_width = input_area.width - 1;
    let cursor_x = input_area.x + (inner_width.saturating_sub(typed_len) / 2) + typed_len;
    frame.set_cursor_position((cursor_x, input_area.y + 1));

    let accent = Style::new().fg(app.theme.accent);
    let help_text = help_lines
        .iter()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    frame.render_widget(Paragraph::new(help_text).style(accent), vertical[3]);
}

const IMPORT_HELP_ITEMS: &[&str] = &["[enter] continue", "[esc] cancel"];

fn draw_import(frame: &mut Frame, app: &mut App) {
    let full_area = frame.area();

    let help_width = full_area.width.saturating_sub(2);
    let help_lines = wrap_help_items(IMPORT_HELP_ITEMS, help_width);
    let help_height = help_lines.len() as u16;

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(help_height),
        ])
        .split(full_area);

    let input_row = vertical[1];

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(60),
            Constraint::Fill(1),
        ])
        .split(input_row);

    let input_area = horizontal[1];
    app.max_len = (input_area.width - 2) as usize;

    let masked = matches!(app.import_step, crate::app::ImportStep::KdbxPassword);

    let title = match app.import_step {
        crate::app::ImportStep::Path => " Import from file (.kdbx / .csv / .json) ",
        crate::app::ImportStep::KdbxPassword => " Password for that vault ",
    };

    let buf = match app.import_step {
        crate::app::ImportStep::Path => &app.import_path_buffer,
        crate::app::ImportStep::KdbxPassword => &app.import_kdbx_password_buffer,
    };

    let display_text = if masked {
        "•".repeat(buf.chars().count())
    } else {
        buf.clone()
    };

    let input = Paragraph::new(display_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.border))
                .title(title),
        );

    frame.render_widget(input, input_area);

    let hint_area = Rect {
        x: input_area.x,
        y: input_area.y + input_area.height,
        width: input_area.width,
        height: 1,
    };

    let default_hint = match app.import_step {
        crate::app::ImportStep::Path => {
            "only name, user, password, url, totp and notes are imported"
        }
        crate::app::ImportStep::KdbxPassword => "that file's own master password, not this vault's",
    };
    let hint_text = app
        .status
        .clone()
        .unwrap_or_else(|| default_hint.to_string());
    let hint_style = if app.status.is_some() {
        Style::new().fg(app.theme.error)
    } else {
        Style::new().fg(app.theme.warning)
    };

    let hint = Paragraph::new(hint_text)
        .alignment(Alignment::Center)
        .style(hint_style);

    frame.render_widget(hint, hint_area);

    let typed_len = buf.chars().count() as u16;
    let inner_width = input_area.width - 1;
    let cursor_x = input_area.x + (inner_width.saturating_sub(typed_len) / 2) + typed_len;
    frame.set_cursor_position((cursor_x, input_area.y + 1));

    let accent = Style::new().fg(app.theme.accent);
    let help_text = help_lines
        .iter()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    frame.render_widget(Paragraph::new(help_text).style(accent), vertical[3]);
}

fn draw_export(frame: &mut Frame, app: &mut App) {
    let full_area = frame.area();

    let help_width = full_area.width.saturating_sub(2);
    let help_lines = wrap_help_items(EXPORT_HELP_ITEMS, help_width);
    let help_height = help_lines.len() as u16;

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(help_height),
        ])
        .split(full_area);

    let input_row = vertical[1];

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(60),
            Constraint::Fill(1),
        ])
        .split(input_row);

    let input_area = horizontal[1];
    app.max_len = (input_area.width - 2) as usize;

    let title = match app.export_step {
        crate::app::ExportStep::Path => " Export to file (.kdbx / .csv / .json) ",
        crate::app::ExportStep::Confirm => " Confirm export ",
    };

    let input = Paragraph::new(app.export_path_buffer.clone())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.border))
                .title(title),
        );

    frame.render_widget(input, input_area);

    let hint_area = Rect {
        x: input_area.x,
        y: input_area.y + input_area.height,
        width: input_area.width,
        height: 1,
    };

    let default_hint = match app.export_step {
        crate::app::ExportStep::Path => {
            "only name, user, password, url, and totp are exported".to_string()
        }
        crate::app::ExportStep::Confirm => {
            let path = app.pending_export_path.as_deref();
            let plaintext = path
                .and_then(|p| crate::export::detect_format(p).ok())
                .map(|f| {
                    matches!(
                        f,
                        crate::export::ExportFormat::Csv | crate::export::ExportFormat::Json
                    )
                })
                .unwrap_or(false);
            let overwrite = path.map(|p| p.exists()).unwrap_or(false);

            match (plaintext, overwrite) {
                (true, true) => {
                    "this overwrites an existing file with all passwords in PLAINTEXT — press enter to confirm".to_string()
                }
                (true, false) => {
                    "this writes all passwords in PLAINTEXT to disk — press enter to confirm".to_string()
                }
                (false, true) => {
                    "this overwrites an existing file — press enter to confirm".to_string()
                }
                (false, false) => "press enter to confirm".to_string(),
            }
        }
    };
    let hint_text = app.status.clone().unwrap_or(default_hint);
    let hint_style = if app.status.is_some() {
        Style::new().fg(app.theme.error)
    } else {
        Style::new().fg(app.theme.warning)
    };

    let hint = Paragraph::new(hint_text)
        .alignment(Alignment::Center)
        .style(hint_style);

    frame.render_widget(hint, hint_area);

    let typed_len = app.export_path_buffer.chars().count() as u16;
    let inner_width = input_area.width - 1;
    let cursor_x = input_area.x + (inner_width.saturating_sub(typed_len) / 2) + typed_len;
    frame.set_cursor_position((cursor_x, input_area.y + 1));

    let accent = Style::new().fg(app.theme.accent);
    let help_text = help_lines
        .iter()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    frame.render_widget(Paragraph::new(help_text).style(accent), vertical[3]);
}

fn draw_theme_picker(frame: &mut Frame, app: &mut App) {
    let full_area = frame.area();

    let help_width = full_area.width.saturating_sub(2);
    let help_lines = wrap_help_items(THEME_PICKER_HELP_ITEMS, help_width);
    let help_height = help_lines.len() as u16;

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(help_height)])
        .split(full_area);

    let theme = &app.theme;
    let normal = Style::new().fg(theme.text);
    let accent = Style::new().fg(theme.accent);
    let warning = Style::new().fg(theme.warning);
    let border_style = Style::new().fg(theme.border);
    let current_theme = app.config.theme.clone();

    let items: Vec<ListItem> = app
        .available_themes
        .iter()
        .map(|name| {
            let is_active = current_theme
                .as_deref()
                .is_some_and(|current| current.eq_ignore_ascii_case(name));

            let marker = if is_active { "◉" } else { "○" };
            let marker_style = if is_active { accent } else { normal };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{marker} "), marker_style),
                Span::styled(name.clone(), normal),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .border_style(border_style)
                .title(" Theme "),
        )
        .highlight_style(
            Style::new()
                .fg(theme.selection_fg)
                .bg(theme.selection_bg)
                .bold(),
        )
        .highlight_symbol("→ ");

    frame.render_stateful_widget(list, vertical[0], &mut app.theme_state);

    let help = if let Some(status) = &app.status {
        Paragraph::new(format!("  {status}")).style(warning)
    } else {
        let help_text = help_lines
            .iter()
            .map(|line| format!("  {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        Paragraph::new(help_text).style(accent)
    };

    frame.render_widget(help, vertical[1]);
}
