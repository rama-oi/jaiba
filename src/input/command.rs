use crossterm::event::KeyCode;

use crate::app::App;
use crate::app::lock_all_vaults;
use crate::clipboard::{cp_password, cp_totp, cp_url, cp_user};
use crate::db::Entry;
use crate::input::vaults::open_vaults;

const COMMANDS: &[(&str, Command)] = &[
    ("u", Command::CopyUser),
    ("p", Command::CopyPassword),
    ("t", Command::CopyTotp),
    ("r", Command::CopyUrl),
    ("a", Command::AddEntry),
    ("q", Command::Quit),
    ("s", Command::Settings),
    ("v", Command::Vaults),
    ("l", Command::Lock),
];

#[derive(Clone, Copy)]
enum Command {
    CopyUser,
    CopyPassword,
    CopyTotp,
    CopyUrl,
    AddEntry,
    Quit,
    Settings,
    Vaults,
    Lock,
}

enum CommandMatch {
    Exact(Command),
    Prefix,
    Invalid,
}

fn match_command(buffer: &str) -> CommandMatch {
    let mut exact = None;
    let mut is_prefix = false;

    for (name, cmd) in COMMANDS {
        if *name == buffer {
            exact = Some(*cmd);
        } else if name.starts_with(buffer) {
            is_prefix = true;
        }
    }

    match (exact, is_prefix) {
        (Some(cmd), _) => CommandMatch::Exact(cmd),
        (None, true) => CommandMatch::Prefix,
        (None, false) => CommandMatch::Invalid,
    }
}

pub fn handle_command_input(app: &mut App, key: KeyCode) {
    match crate::input::normalize_shortcut(key) {
        KeyCode::Esc => {
            app.command_mode = false;
            app.command_buffer.clear();
        }

        KeyCode::Backspace => {
            app.command_buffer.pop();

            if app.command_buffer.is_empty() {
                app.command_mode = false;
            }
        }

        KeyCode::Char(c) => {
            app.command_buffer.push(c);

            match match_command(&app.command_buffer) {
                CommandMatch::Exact(cmd) => {
                    app.command_mode = false;
                    app.command_buffer.clear();
                    execute_command(app, cmd);
                }

                CommandMatch::Prefix => {
                    // Keep collecting characters.
                }

                CommandMatch::Invalid => {
                    app.command_mode = false;
                    app.command_buffer.clear();
                }
            }
        }

        _ => {}
    }
}

fn execute_command(app: &mut App, cmd: Command) {
    match cmd {
        Command::CopyUser => cp_user(app),
        Command::CopyPassword => cp_password(app),
        Command::CopyTotp => cp_totp(app),
        Command::CopyUrl => cp_url(app),
        Command::AddEntry => add_entry(app),
        Command::Quit => app.should_quit = true,
        Command::Settings => open_settings(app),
        Command::Vaults => open_vaults(app),
        Command::Lock => lock_all_vaults(app, "Locked"),
    }
}

fn add_entry(app: &mut App) {
    app.edit_entry = Some(Entry::default());
    app.edit_original = Some(Entry::default());
    app.edit_target = None;
    app.edit_state.select(Some(0));
    app.reveal_password = false;
    app.confirm_delete = false;
    app.confirm_exit = false;
    app.screen = crate::app::Screen::Edit;
}

pub fn preview_entry(app: &mut App) {
    let Some(selected) = app.index_state.selected() else {
        return;
    };
    let Some(&entry_idx) = app.filtered.get(selected) else {
        return;
    };
    let Some(entry) = app.entries().get(entry_idx).cloned() else {
        return;
    };

    app.edit_entry = Some(entry.clone());
    app.edit_original = Some(entry);
    app.edit_target = Some(entry_idx);
    app.edit_state.select(Some(0));
    app.reveal_password = false;
    app.confirm_delete = false;
    app.confirm_exit = false;
    app.screen = crate::app::Screen::Edit;
}

fn open_settings(app: &mut App) {
    app.status = None;
    app.available_themes = crate::theme::list_theme_names().unwrap_or_default();

    let current_idx = app.config.theme.as_deref().and_then(|current| {
        app.available_themes
            .iter()
            .position(|name| name.eq_ignore_ascii_case(current))
    });

    let selected = current_idx.or(if app.available_themes.is_empty() {
        None
    } else {
        Some(0)
    });

    app.settings_state.select(selected);
    app.screen = crate::app::Screen::Settings;
}
