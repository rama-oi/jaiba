use std::time::Instant;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::App;
use crate::db::current_totp_code;
use crate::util::wrap_help_items;

fn nav_help_items(slim_mode: bool) -> &'static [&'static str] {
    if slim_mode {
        &["[↑↓]", "[enter]", "[v]", "[d]", "[esc]"]
    } else {
        &[
            "[↑↓] navigate",
            "[enter] edit field",
            "[v] toggle visibility",
            "[d] delete entry",
            "[esc] close",
        ]
    }
}

fn field_help_items(slim_mode: bool) -> &'static [&'static str] {
    if slim_mode {
        &["[enter]", "[esc]"]
    } else {
        &["[enter] save field", "[esc] cancel"]
    }
}

fn wrap_notes(notes: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut wrapped = Vec::new();

    for line in notes.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let mut current = String::new();

        for word in line.split_whitespace() {
            let separator_width = usize::from(!current.is_empty());

            if UnicodeWidthStr::width(current.as_str())
                + separator_width
                + UnicodeWidthStr::width(word)
                <= width
            {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(word);
                continue;
            }

            if !current.is_empty() {
                wrapped.push(std::mem::take(&mut current));
            }

            for character in word.chars() {
                let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
                if !current.is_empty()
                    && UnicodeWidthStr::width(current.as_str()) + character_width > width
                {
                    wrapped.push(std::mem::take(&mut current));
                }
                current.push(character);
            }
        }

        if !current.is_empty() {
            wrapped.push(current);
        } else if line.trim().is_empty() {
            wrapped.push(String::new());
        }
    }

    wrapped
}

pub fn draw_edit(frame: &mut Frame, app: &mut App) {
    let full_area = frame.area();
    let tab_height = if app.tabs.len() >= 2 { 2 } else { 0 };
    crate::ui::tabs::draw_tabs(
        frame,
        app,
        ratatui::layout::Rect {
            x: full_area.x,
            y: full_area.y,
            width: full_area.width,
            height: tab_height,
        },
    );
    let full_area = ratatui::layout::Rect {
        x: full_area.x,
        y: full_area.y.saturating_add(tab_height),
        width: full_area.width,
        height: full_area.height.saturating_sub(tab_height),
    };

    let Some(entry) = app.edit_entry.as_ref() else {
        return;
    };

    let name = entry.name.clone();
    let user = entry.user.clone();
    let url = entry.url.clone();
    let totp_raw = entry.totp.clone();
    let notes = entry.notes.clone();
    let last_modified = entry.date_last_modify.clone();
    let duplicate_user_count = entry.duplicate_user_count;
    let password_reuse_count = entry.password_reuse_count;
    let password_is_set = !entry.password.is_empty();

    let password_text = if !password_is_set {
        String::new()
    } else if app.reveal_password {
        entry.password.clone()
    } else {
        "•".repeat(entry.password.chars().count().max(8))
    };

    let is_new_entry = app.edit_target.is_none();
    let editing_field = app.editing_field;
    let field_buffer = app.field_buffer.clone();
    let selected = app.edit_state.selected().unwrap_or(0);
    let reveal_password = app.reveal_password;

    let theme = &app.theme;
    let label_style = Style::new().fg(theme.header).bold();
    let normal = Style::new().fg(theme.text);
    let warning = Style::new().fg(theme.warning);
    let placeholder = Style::new().fg(theme.border).italic();
    let editing_style = Style::new().fg(theme.selection_fg).bg(theme.selection_bg);
    let accent_style = Style::new().fg(theme.accent);
    let border_style = Style::new().fg(theme.border);

    let help_items = if editing_field {
        field_help_items(app.slim_mode)
    } else {
        nav_help_items(app.slim_mode)
    };
    let help_width = full_area.width.saturating_sub(2);
    let help_lines = wrap_help_items(help_items, help_width);
    let help_height = help_lines.len() as u16;

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(help_height)])
        .split(full_area);

    let render_value =
        |field_index: usize, value: String, extra: Vec<Span<'static>>| -> Line<'static> {
            if editing_field && selected == field_index {
                return Line::from(vec![
                    Span::styled(field_buffer.clone(), editing_style),
                    Span::styled("▏", accent_style),
                ]);
            }

            if value.is_empty() && extra.is_empty() {
                return Line::from(Span::styled("(empty)", placeholder));
            }

            let mut spans = vec![Span::styled(value, normal)];
            spans.extend(extra);
            Line::from(spans)
        };

    let user_extra = if duplicate_user_count > 1 {
        vec![Span::styled(format!(" [{duplicate_user_count}]"), warning)]
    } else {
        vec![]
    };

    let mut password_extra = if password_reuse_count > 1 {
        vec![Span::styled(format!(" [{password_reuse_count}]"), warning)]
    } else {
        vec![]
    };
    if password_is_set && reveal_password {
        password_extra.push(Span::styled("  [visible]", warning));
    }

    let totp_code_line: Line<'static> = if editing_field && selected == 4 {
        Line::from("")
    } else {
        match current_totp_code(&totp_raw) {
            Some(totp) => Line::from(vec![
                Span::styled(totp.code, normal.bold()),
                Span::styled(
                    format!("  (expires in {}s)", totp.valid_for.as_secs() + 1),
                    warning,
                ),
            ]),
            None if totp_raw.trim().is_empty() => Line::from(""),
            None => Line::from(Span::styled(
                "couldn't generate a code from the stored TOTP value",
                warning,
            )),
        }
    };

    let field = |label: &'static str, value: Line<'static>| -> ListItem<'static> {
        ListItem::new(vec![
            Line::from(Span::styled(label, label_style)),
            value,
            Line::from(""), // spacer between fields
        ])
    };

    let notes_width = vertical[0].width.saturating_sub(6) as usize;
    let notes_lines: Vec<Line<'static>> = if editing_field && selected == 5 {
        vec![Line::from(vec![
            Span::styled(field_buffer.clone(), editing_style),
            Span::styled("▏", accent_style),
        ])]
    } else if notes.is_empty() {
        vec![Line::from(Span::styled("(empty)", placeholder))]
    } else {
        wrap_notes(&notes, notes_width)
            .into_iter()
            .map(|line| Line::from(Span::styled(line, normal)))
            .collect()
    };

    let notes_field = |label: &'static str, lines: Vec<Line<'static>>| -> ListItem<'static> {
        let mut content = vec![Line::from(Span::styled(label, label_style))];
        content.extend(lines);
        content.push(Line::from("")); // spacer between fields
        ListItem::new(content)
    };

    let totp_field = |label: &'static str,
                      value: Line<'static>,
                      code_line: Line<'static>|
     -> ListItem<'static> {
        ListItem::new(vec![
            Line::from(Span::styled(label, label_style)),
            value,
            code_line,
            Line::from(""), // spacer between fields
        ])
    };

    let items = vec![
        field("Name", render_value(0, name, vec![])),
        field("User", render_value(1, user, user_extra)),
        field("Password", render_value(2, password_text, password_extra)),
        field("URL", render_value(3, url, vec![])),
        totp_field("TOTP", render_value(4, totp_raw, vec![]), totp_code_line),
        notes_field("Notes", notes_lines),
        field("Last modified", render_value(6, last_modified, vec![])),
    ];

    let title = if is_new_entry {
        " New Entry "
    } else {
        " Entry "
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .padding(Padding::horizontal(1))
                .border_style(border_style)
                .title(title),
        )
        .highlight_style(editing_style);

    frame.render_stateful_widget(list, vertical[0], &mut app.edit_state);

    let help = if let Some(timer) = &app.clipboard_timer {
        let remaining = timer
            .clear_at
            .saturating_duration_since(Instant::now())
            .as_secs()
            + 1;

        Paragraph::new(format!(
            "  Copied {} :: clearing in {remaining}s",
            timer.label
        ))
        .style(Style::new().fg(app.theme.warning))
    } else if let Some(status) = &app.status {
        Paragraph::new(format!("  {status}")).style(Style::new().fg(app.theme.warning))
    } else {
        let help_text = help_lines
            .iter()
            .map(|line| format!("  {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        Paragraph::new(help_text).style(Style::new().fg(app.theme.accent))
    };

    frame.render_widget(help, vertical[1]);
}

#[cfg(test)]
mod tests {
    use super::wrap_notes;

    #[test]
    fn wraps_multiline_notes_to_the_available_width() {
        assert_eq!(
            wrap_notes("first line\nsecond line with more text", 12),
            ["first line", "second line", "with more", "text"]
        );
    }

    #[test]
    fn wraps_long_words_instead_of_clipping_them() {
        assert_eq!(wrap_notes("averylongword", 5), ["avery", "longw", "ord"]);
    }

    #[test]
    fn preserves_blank_lines_in_notes() {
        assert_eq!(wrap_notes("first\n\nthird", 20), ["first", "", "third"]);
    }
}
