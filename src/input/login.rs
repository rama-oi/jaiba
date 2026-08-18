use std::time::Instant;

use crossterm::event::KeyCode;

use crate::app::{App, Screen, UnlockedVault, clear_secret};
use crate::config::save_config;
use crate::db::{calculate_warnings, create_database, unlock_database};
use crate::util::default_new_database_path;

pub fn handle_login_input(app: &mut App, key: KeyCode) {
    if app.creating_database {
        handle_create_database_input(app, key);
        return;
    }

    if database_missing(app) {
        handle_missing_database_input(app, key);
        return;
    }

    match key {
        KeyCode::Char(c) => {
            app.login_error = None;

            if app.password.len() < app.max_len {
                app.password.push(c);
            }
        }

        KeyCode::Backspace => {
            app.login_error = None;
            app.password.pop();
        }

        KeyCode::Enter => {
            attempt_unlock(app);
        }

        KeyCode::Esc => {
            app.should_quit = true;
        }

        _ => {}
    }
}

pub fn database_missing(app: &App) -> bool {
    app.active_vault_config()
        .is_none_or(|vault| !vault.path.is_file())
}

fn handle_missing_database_input(app: &mut App, key: KeyCode) {
    match crate::input::normalize_shortcut(key) {
        KeyCode::Char('n') => start_create_database(app),

        KeyCode::Esc => {
            app.should_quit = true;
        }

        _ => {}
    }
}

fn start_create_database(app: &mut App) {
    app.creating_database = true;
    app.confirming_new_db_password = false;
    clear_secret(&mut app.password);
    clear_secret(&mut app.new_db_confirm);
    app.login_error = None;
}

fn cancel_create_database(app: &mut App) {
    app.creating_database = false;
    app.confirming_new_db_password = false;
    clear_secret(&mut app.password);
    clear_secret(&mut app.new_db_confirm);
    app.login_error = None;
}

fn handle_create_database_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char(c) => {
            app.login_error = None;
            let limit = app.max_len;

            if app.confirming_new_db_password {
                if app.new_db_confirm.len() < limit {
                    app.new_db_confirm.push(c);
                }
            } else if app.password.len() < limit {
                app.password.push(c);
            }
        }

        KeyCode::Backspace => {
            app.login_error = None;

            if app.confirming_new_db_password {
                app.new_db_confirm.pop();
            } else {
                app.password.pop();
            }
        }

        KeyCode::Enter => {
            if app.confirming_new_db_password {
                finish_create_database(app);
            } else if app.password.is_empty() {
                app.login_error = Some("Master password can't be empty".to_string());
            } else {
                app.confirming_new_db_password = true;
            }
        }

        KeyCode::Esc => cancel_create_database(app),

        _ => {}
    }
}

fn finish_create_database(app: &mut App) {
    if app.new_db_confirm != app.password {
        app.login_error = Some("Passwords don't match".to_string());
        clear_secret(&mut app.new_db_confirm);
        app.confirming_new_db_password = false;
        return;
    }

    let path = app
        .active_vault_config()
        .map(|vault| vault.path.clone())
        .unwrap_or_else(default_new_database_path);

    if app.active_vault_config().is_none() {
        let vault = app.config.ensure_default_vault(path.clone());
        app.active_vault_name = Some(vault.name.clone());
    }

    let Some(vault) = app.active_vault_config().cloned() else {
        app.login_error = Some("No vault configured".to_string());
        return;
    };

    if path.is_file() {
        let _ = save_config(&app.config);

        clear_secret(&mut app.new_db_confirm);
        app.creating_database = false;
        app.confirming_new_db_password = false;
        app.login_error = Some(format!(
            "A vault already exists at {}. Press Enter to try unlocking it with this password.",
            path.display()
        ));
        return;
    }

    let result = create_database(&path, &app.password, vault.keyfile.as_deref());

    match result {
        Ok((db, key, entries)) => {
            let save_result = save_config(&app.config);

            app.unlock_vault(UnlockedVault {
                name: vault.name,
                path: path.clone(),
                keyfile: vault.keyfile,
                database: db,
                db_key: key,
                entries,
            });
            clear_secret(&mut app.password);
            clear_secret(&mut app.new_db_confirm);
            app.creating_database = false;
            app.confirming_new_db_password = false;
            app.login_error = None;
            app.last_activity = Instant::now();
            app.refresh_filter();
            app.screen = Screen::Index;

            app.status = Some(match save_result {
                Ok(()) => format!("Created new database at {}", path.display()),
                Err(err) => format!("Database created, but couldn't save config: {err}"),
            });
        }

        Err(err) => {
            app.login_error = Some(format!("Couldn't create database: {err:#}"));
            clear_secret(&mut app.password);
            clear_secret(&mut app.new_db_confirm);
            app.confirming_new_db_password = false;
        }
    }
}

fn attempt_unlock(app: &mut App) {
    let Some(vault) = app.active_vault_config().cloned() else {
        app.login_error = Some("No vault configured in config.toml".to_string());
        return;
    };

    let mut password = std::mem::take(&mut app.password);
    let result = unlock_database(&vault.path, &password, vault.keyfile.as_deref());
    clear_secret(&mut password);

    match result {
        Ok((db, key, mut entries)) => {
            calculate_warnings(&mut entries);
            app.unlock_vault(UnlockedVault {
                name: vault.name,
                path: vault.path,
                keyfile: vault.keyfile,
                database: db,
                db_key: key,
                entries,
            });
            app.login_error = None;
            app.last_activity = Instant::now();
            app.refresh_filter();
            app.screen = Screen::Index;
        }
        Err(err) => {
            app.login_error = Some(format!("{err}"));
        }
    }
}
