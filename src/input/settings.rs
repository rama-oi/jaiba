use std::path::Path;
use std::time::Duration;

use crossterm::event::KeyCode;

use crate::app::{App, ExportStep, ImportStep, PasswordChangeStep, Screen, clear_secret};
use crate::config::save_config;
use crate::db::{Entry, build_database_key, calculate_warnings, save_database, unlock_database};
use crate::theme::load_theme;
use crate::util::expand_tilde;

pub const ROW_COUNT: usize = 8;

pub const DATABASE_ROW: usize = 0;
pub const KEYFILE_ROW: usize = 1;
pub const AUTO_LOCK_ROW: usize = 2;
pub const CLIPBOARD_TIMEOUT_ROW: usize = 3;
pub const THEME_ROW: usize = 4;
pub const CHANGE_PASSWORD_ROW: usize = 5;
pub const IMPORT_ROW: usize = 6;
pub const EXPORT_ROW: usize = 7;

pub fn handle_settings_input(app: &mut App, key: KeyCode) {
    if app.exporting_database {
        handle_export_input(app, key);
        return;
    }

    if app.importing_database {
        handle_import_input(app, key);
        return;
    }

    if app.changing_password {
        handle_change_password_input(app, key);
        return;
    }

    if app.choosing_theme {
        handle_theme_picker_input(app, key);
        return;
    }

    if app.editing_field {
        handle_field_input(app, key);
        return;
    }

    match key {
        KeyCode::Down => {
            app.status = None;
            let selected = app.settings_state.selected().unwrap_or(0);
            let next = if selected + 1 >= ROW_COUNT {
                0
            } else {
                selected + 1
            };
            app.settings_state.select(Some(next));
        }

        KeyCode::Up => {
            app.status = None;
            let selected = app.settings_state.selected().unwrap_or(0);
            let prev = if selected == 0 {
                ROW_COUNT - 1
            } else {
                selected - 1
            };
            app.settings_state.select(Some(prev));
        }

        KeyCode::Enter => activate_selected(app),

        KeyCode::Esc => {
            app.status = None;
            app.screen = Screen::Index;
        }

        _ => {}
    }
}

fn activate_selected(app: &mut App) {
    let Some(selected) = app.settings_state.selected() else {
        return;
    };

    match selected {
        DATABASE_ROW => start_editing_database(app),
        KEYFILE_ROW => start_editing_keyfile(app),
        AUTO_LOCK_ROW => start_editing_auto_lock(app),
        CLIPBOARD_TIMEOUT_ROW => start_editing_clipboard_timeout(app),
        THEME_ROW => start_choosing_theme(app),
        CHANGE_PASSWORD_ROW => start_change_password(app),
        IMPORT_ROW => start_import(app),
        EXPORT_ROW => start_export(app),
        _ => {}
    }
}

fn start_editing_database(app: &mut App) {
    app.field_buffer = app
        .active_vault_config()
        .map(|vault| vault.path.display().to_string())
        .unwrap_or_default();
    app.editing_field = true;
    app.status = None;
}

fn start_editing_keyfile(app: &mut App) {
    app.field_buffer = app
        .active_vault_config()
        .and_then(|vault| vault.keyfile.as_ref())
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    app.editing_field = true;
    app.status = None;
}

fn start_editing_auto_lock(app: &mut App) {
    app.field_buffer = app.config.auto_lock.as_secs().to_string();
    app.editing_field = true;
    app.status = None;
}

fn start_editing_clipboard_timeout(app: &mut App) {
    app.field_buffer = app.config.clipboard_timeout.as_secs().to_string();
    app.editing_field = true;
    app.status = None;
}

fn handle_field_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.editing_field = false;
            app.field_buffer.clear();
            app.status = None;
        }

        KeyCode::Enter => commit_field(app),

        KeyCode::Char(c) => app.field_buffer.push(c),

        KeyCode::Backspace => {
            app.field_buffer.pop();
        }

        _ => {}
    }
}

fn commit_field(app: &mut App) {
    let selected = app.settings_state.selected().unwrap_or(0);
    let value = std::mem::take(&mut app.field_buffer);
    app.editing_field = false;

    match selected {
        DATABASE_ROW => {
            if value.trim().is_empty() {
                app.status = Some("database path can't be empty".to_string());
                return;
            }
            let Some(vault) = app.active_vault_config_mut() else {
                app.status = Some("no active vault".to_string());
                return;
            };
            vault.path = expand_tilde(value.trim());
        }

        KEYFILE_ROW => {
            let Some(vault) = app.active_vault_config_mut() else {
                app.status = Some("no active vault".to_string());
                return;
            };
            vault.keyfile = if value.trim().is_empty() {
                None
            } else {
                Some(expand_tilde(value.trim()))
            };
        }

        AUTO_LOCK_ROW => match parse_seconds(&value) {
            Ok(secs) => app.config.auto_lock = Duration::from_secs(secs),
            Err(err) => {
                app.status = Some(err);
                return;
            }
        },

        CLIPBOARD_TIMEOUT_ROW => match parse_seconds(&value) {
            Ok(secs) => app.config.clipboard_timeout = Duration::from_secs(secs),
            Err(err) => {
                app.status = Some(err);
                return;
            }
        },

        _ => return,
    }

    app.sync_active_session_metadata();

    app.status = Some(match save_config(&app.config) {
        Ok(()) => "saved".to_string(),
        Err(err) => format!("couldn't save config: {err}"),
    });
}

fn parse_seconds(value: &str) -> Result<u64, String> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| format!("\"{}\" isn't a whole number of seconds", value.trim()))
}

fn start_change_password(app: &mut App) {
    clear_secret(&mut app.current_password_buffer);
    clear_secret(&mut app.new_password_buffer);
    clear_secret(&mut app.new_password_confirm);
    app.password_change_step = PasswordChangeStep::CurrentPassword;
    app.changing_password = true;
    app.status = None;
}

fn cancel_change_password(app: &mut App) {
    app.changing_password = false;
    app.password_change_step = PasswordChangeStep::CurrentPassword;
    clear_secret(&mut app.current_password_buffer);
    clear_secret(&mut app.new_password_buffer);
    clear_secret(&mut app.new_password_confirm);
    app.status = None;
}

fn handle_change_password_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char(c) => {
            app.status = None;
            let limit = app.max_len.max(1);

            let buffer = match app.password_change_step {
                PasswordChangeStep::CurrentPassword => &mut app.current_password_buffer,
                PasswordChangeStep::NewPassword => &mut app.new_password_buffer,
                PasswordChangeStep::ConfirmNewPassword => &mut app.new_password_confirm,
            };

            if buffer.len() < limit {
                buffer.push(c);
            }
        }

        KeyCode::Backspace => {
            app.status = None;

            let buffer = match app.password_change_step {
                PasswordChangeStep::CurrentPassword => &mut app.current_password_buffer,
                PasswordChangeStep::NewPassword => &mut app.new_password_buffer,
                PasswordChangeStep::ConfirmNewPassword => &mut app.new_password_confirm,
            };

            buffer.pop();
        }

        KeyCode::Enter => match app.password_change_step {
            PasswordChangeStep::CurrentPassword => verify_current_password(app),
            PasswordChangeStep::NewPassword => {
                if app.new_password_buffer.is_empty() {
                    app.status = Some("new password can't be empty".to_string());
                } else {
                    app.password_change_step = PasswordChangeStep::ConfirmNewPassword;
                }
            }
            PasswordChangeStep::ConfirmNewPassword => commit_password_change(app),
        },

        KeyCode::Esc => cancel_change_password(app),

        _ => {}
    }
}

fn verify_current_password(app: &mut App) {
    let Some((path, keyfile)) = app
        .active_session()
        .map(|vault| (vault.path.clone(), vault.keyfile.clone()))
    else {
        app.status = Some("no unlocked database".to_string());
        cancel_change_password(app);
        return;
    };

    let mut password = std::mem::take(&mut app.current_password_buffer);
    let result = unlock_database(&path, &password, keyfile.as_deref());
    clear_secret(&mut password);

    match result {
        Ok(_) => {
            app.status = None;
            app.password_change_step = PasswordChangeStep::NewPassword;
        }
        Err(err) => {
            app.status = Some(if keyfile.is_some() {
                format!("couldn't verify current password and keyfile: {err}")
            } else {
                "that isn't the current master password".to_string()
            });
        }
    }
}

fn commit_password_change(app: &mut App) {
    if app.new_password_confirm != app.new_password_buffer {
        app.status = Some("passwords don't match".to_string());
        clear_secret(&mut app.new_password_confirm);
        app.password_change_step = PasswordChangeStep::NewPassword;
        return;
    }

    let Some(keyfile) = app.active_session().map(|vault| vault.keyfile.clone()) else {
        app.status = Some("no unlocked database".to_string());
        cancel_change_password(app);
        return;
    };

    let new_key = match build_database_key(&app.new_password_buffer, keyfile.as_deref()) {
        Ok(key) => key,
        Err(err) => {
            app.status = Some(format!("couldn't change password: {err:#}"));
            clear_secret(&mut app.new_password_buffer);
            clear_secret(&mut app.new_password_confirm);
            app.password_change_step = PasswordChangeStep::NewPassword;
            return;
        }
    };

    let Some(session) = app.active_session_mut() else {
        app.status = Some("no unlocked database".to_string());
        cancel_change_password(app);
        return;
    };

    let result = save_database(
        &session.path,
        &new_key,
        &mut session.database,
        &mut session.entries,
    );
    if result.is_ok() {
        session.db_key = new_key;
    }

    app.status = Some(match result {
        Ok(()) => "master password changed".to_string(),
        Err(err) => format!("couldn't change password: {err:#}"),
    });

    app.changing_password = false;
    app.password_change_step = PasswordChangeStep::CurrentPassword;
    clear_secret(&mut app.current_password_buffer);
    clear_secret(&mut app.new_password_buffer);
    clear_secret(&mut app.new_password_confirm);
}

fn start_import(app: &mut App) {
    reset_import_state(app);
    app.importing_database = true;
    app.status = None;
}

fn reset_import_state(app: &mut App) {
    app.importing_database = false;
    app.import_step = ImportStep::Path;
    app.import_path_buffer.clear();
    clear_secret(&mut app.import_kdbx_password_buffer);
    app.pending_import_path = None;
}

fn cancel_import(app: &mut App) {
    reset_import_state(app);
    app.status = None;
}

fn handle_import_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char(c) => {
            app.status = None;
            let limit = app.max_len.max(1);

            let buffer = match app.import_step {
                ImportStep::Path => &mut app.import_path_buffer,
                ImportStep::KdbxPassword => &mut app.import_kdbx_password_buffer,
            };

            if buffer.len() < limit {
                buffer.push(c);
            }
        }

        KeyCode::Backspace => {
            app.status = None;

            let buffer = match app.import_step {
                ImportStep::Path => &mut app.import_path_buffer,
                ImportStep::KdbxPassword => &mut app.import_kdbx_password_buffer,
            };

            buffer.pop();
        }

        KeyCode::Enter => match app.import_step {
            ImportStep::Path => confirm_import_path(app),
            ImportStep::KdbxPassword => confirm_import_kdbx_password(app),
        },

        KeyCode::Esc => cancel_import(app),

        _ => {}
    }
}

fn confirm_import_path(app: &mut App) {
    let raw = app.import_path_buffer.trim();

    if raw.is_empty() {
        app.status = Some("enter a file path".to_string());
        return;
    }

    let path = expand_tilde(raw);

    if !path.is_file() {
        app.status = Some(format!("no file found at {}", path.display()));
        return;
    }

    match crate::import::detect_format(&path) {
        Ok(crate::import::ImportFormat::Kdbx) => {
            app.pending_import_path = Some(path);
            clear_secret(&mut app.import_kdbx_password_buffer);
            app.import_step = ImportStep::KdbxPassword;
            app.status = None;
        }

        Ok(crate::import::ImportFormat::Csv) => match crate::import::import_csv(&path) {
            Ok(entries) => finish_import(app, entries, &path),
            Err(err) => app.status = Some(format!("couldn't import: {err:#}")),
        },

        Ok(crate::import::ImportFormat::Json) => match crate::import::import_json(&path) {
            Ok(entries) => finish_import(app, entries, &path),
            Err(err) => app.status = Some(format!("couldn't import: {err:#}")),
        },

        Err(err) => app.status = Some(format!("{err:#}")),
    }
}

fn confirm_import_kdbx_password(app: &mut App) {
    let Some(path) = app.pending_import_path.clone() else {
        cancel_import(app);
        return;
    };

    let mut password = std::mem::take(&mut app.import_kdbx_password_buffer);
    let result = crate::import::import_kdbx(&path, &password);
    clear_secret(&mut password);

    match result {
        Ok(entries) => finish_import(app, entries, &path),
        Err(err) => {
            app.status = Some(format!("{err:#}"));
        }
    }
}

fn finish_import(app: &mut App, imported: Vec<Entry>, source: &Path) {
    let count = imported.len();

    let Some(entries) = app.entries_mut() else {
        app.status = Some("couldn't import: database is locked".to_string());
        reset_import_state(app);
        return;
    };

    for mut entry in imported {
        entry.id = None;
        entries.push(entry);
    }

    calculate_warnings(entries);
    app.refresh_filter();

    let Some(session) = app.active_session_mut() else {
        app.status = Some(format!(
            "imported {count} entries from {}, but couldn't save: database is locked",
            source.display()
        ));
        reset_import_state(app);
        return;
    };

    app.status = Some(
        match save_database(
            &session.path,
            &session.db_key,
            &mut session.database,
            &mut session.entries,
        ) {
            Ok(()) => format!("imported {count} entries from {}", source.display()),
            Err(err) => format!("imported {count} entries, but failed to save: {err:#}"),
        },
    );

    reset_import_state(app);
}

fn start_export(app: &mut App) {
    reset_export_state(app);
    app.exporting_database = true;
    app.status = None;
}

fn reset_export_state(app: &mut App) {
    app.exporting_database = false;
    app.export_step = ExportStep::Path;
    app.export_path_buffer.clear();
    app.pending_export_path = None;
}

fn cancel_export(app: &mut App) {
    reset_export_state(app);
    app.status = None;
}

fn handle_export_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char(c) => {
            app.status = None;

            if app.export_step == ExportStep::Path {
                let limit = app.max_len.max(1);
                if app.export_path_buffer.len() < limit {
                    app.export_path_buffer.push(c);
                }
            }
        }

        KeyCode::Backspace => {
            app.status = None;

            if app.export_step == ExportStep::Path {
                app.export_path_buffer.pop();
            }
        }

        KeyCode::Enter => match app.export_step {
            ExportStep::Path => confirm_export_path(app),
            ExportStep::Confirm => do_export(app),
        },

        KeyCode::Esc => cancel_export(app),

        _ => {}
    }
}

fn confirm_export_path(app: &mut App) {
    let raw = app.export_path_buffer.trim();

    if raw.is_empty() {
        app.status = Some("enter a file path".to_string());
        return;
    }

    let path = expand_tilde(raw);

    let format = match crate::export::detect_format(&path) {
        Ok(format) => format,
        Err(err) => {
            app.status = Some(format!("{err:#}"));
            return;
        }
    };

    app.pending_export_path = Some(path.clone());

    let plaintext = matches!(
        format,
        crate::export::ExportFormat::Csv | crate::export::ExportFormat::Json
    );
    let overwrite = path.exists();

    if plaintext || overwrite {
        app.export_step = ExportStep::Confirm;
        app.status = None;
    } else {
        do_export(app);
    }
}

fn do_export(app: &mut App) {
    let Some(path) = app.pending_export_path.clone() else {
        cancel_export(app);
        return;
    };

    let format = match crate::export::detect_format(&path) {
        Ok(format) => format,
        Err(err) => {
            app.status = Some(format!("{err:#}"));
            reset_export_state(app);
            return;
        }
    };

    let count = app.entries().len();

    let result = match format {
        crate::export::ExportFormat::Kdbx => {
            let Some(session) = app.active_session_mut() else {
                app.status = Some("no unlocked database".to_string());
                reset_export_state(app);
                return;
            };

            save_database(
                &path,
                &session.db_key,
                &mut session.database,
                &mut session.entries,
            )
        }
        crate::export::ExportFormat::Csv => crate::export::export_csv(&path, app.entries()),
        crate::export::ExportFormat::Json => crate::export::export_json(&path, app.entries()),
    };

    app.status = Some(match result {
        Ok(()) => format!("exported {count} entries to {}", path.display()),
        Err(err) => format!("couldn't export: {err:#}"),
    });

    reset_export_state(app);
}

fn start_choosing_theme(app: &mut App) {
    if app.available_themes.is_empty() {
        app.status = Some("no themes found in ~/.config/jaiba/themes".to_string());
        return;
    }

    let current = app.config.theme.as_deref();
    let start = app
        .available_themes
        .iter()
        .position(|name| current.is_some_and(|c| c.eq_ignore_ascii_case(name)))
        .unwrap_or(0);

    app.theme_state.select(Some(start));
    app.choosing_theme = true;
    app.status = None;
}

fn handle_theme_picker_input(app: &mut App, key: KeyCode) {
    let count = app.available_themes.len();

    match key {
        KeyCode::Down => {
            if count == 0 {
                return;
            }
            let selected = app.theme_state.selected().unwrap_or(0);
            let next = if selected + 1 >= count {
                0
            } else {
                selected + 1
            };
            app.theme_state.select(Some(next));
        }

        KeyCode::Up => {
            if count == 0 {
                return;
            }
            let selected = app.theme_state.selected().unwrap_or(0);
            let prev = if selected == 0 {
                count - 1
            } else {
                selected - 1
            };
            app.theme_state.select(Some(prev));
        }

        KeyCode::Enter => {
            apply_selected_theme(app);
            app.choosing_theme = false;
        }

        KeyCode::Esc => {
            app.choosing_theme = false;
            app.status = None;
        }

        _ => {}
    }
}

fn apply_selected_theme(app: &mut App) {
    let Some(selected) = app.theme_state.selected() else {
        return;
    };
    let Some(name) = app.available_themes.get(selected).cloned() else {
        return;
    };

    match load_theme(Some(&name)) {
        Ok(theme) => {
            app.theme = theme;
            app.config.theme = Some(name.clone());

            app.status = Some(match save_config(&app.config) {
                Ok(()) => format!("theme set to {name}"),
                Err(err) => format!("theme applied, but couldn't save config: {err}"),
            });
        }

        Err(err) => {
            app.status = Some(format!("couldn't load theme \"{name}\": {err}"));
        }
    }
}
