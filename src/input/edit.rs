use crossterm::event::KeyCode;

use crate::app::{App, Screen};
use crate::db::{calculate_warnings, delete_entry, save_database};

const FIELD_COUNT: usize = 7;
const LAST_MODIFIED_INDEX: usize = 6;

pub fn handle_edit_input(app: &mut App, key: KeyCode) {
    if app.editing_field {
        handle_field_input(app, key);
        return;
    }

    if app.confirm_exit {
        handle_exit_confirmation(app, key);
        return;
    }

    if app.confirm_delete {
        handle_delete_confirmation(app, key);
        return;
    }

    match crate::input::normalize_shortcut(key) {
        KeyCode::Esc => request_close_edit(app),

        KeyCode::Enter => start_editing_field(app),

        KeyCode::Char('v') => {
            app.reveal_password = !app.reveal_password;
        }

        KeyCode::Char('d') => start_delete_confirmation(app),

        KeyCode::Down => move_selection(app, 1),
        KeyCode::Up => move_selection(app, -1),

        _ => {}
    }
}

fn handle_field_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.editing_field = false;
            app.field_buffer.clear();
        }

        KeyCode::Enter => commit_field(app),

        KeyCode::Char(c) => app.field_buffer.push(c),

        KeyCode::Backspace => {
            app.field_buffer.pop();
        }

        _ => {}
    }
}

fn move_selection(app: &mut App, delta: i32) {
    let current = app.edit_state.selected().unwrap_or(0) as i32;
    let next = (current + delta).rem_euclid(FIELD_COUNT as i32) as usize;
    app.edit_state.select(Some(next));
}

fn start_editing_field(app: &mut App) {
    let selected = app.edit_state.selected().unwrap_or(0);

    if selected == LAST_MODIFIED_INDEX {
        app.status = Some("Last modified is set automatically".to_string());
        return;
    }

    let Some(entry) = app.edit_entry.as_ref() else {
        return;
    };

    app.field_buffer = match selected {
        0 => entry.name.clone(),
        1 => entry.user.clone(),
        2 => entry.password.clone(),
        3 => entry.url.clone(),
        4 => entry.totp.clone(),
        5 => entry.notes.clone(),
        _ => return,
    };

    app.editing_field = true;
    app.status = None;

    if selected == 2 {
        app.reveal_password = true;
    }
}

fn commit_field(app: &mut App) {
    let selected = app.edit_state.selected().unwrap_or(0);
    let value = std::mem::take(&mut app.field_buffer);

    if let Some(entry) = app.edit_entry.as_mut() {
        match selected {
            0 => entry.name = value,
            1 => entry.user = value,
            2 => entry.password = value,
            3 => entry.url = value,
            4 => entry.totp = value,
            5 => entry.notes = value,
            _ => {}
        }
    }

    app.editing_field = false;
}

fn request_close_edit(app: &mut App) {
    if has_unsaved_changes(app) {
        app.confirm_exit = true;
        app.status = Some("Save changes? [y] yes  [n] no".to_string());
    } else {
        reset_edit_state(app);
    }
}

fn has_unsaved_changes(app: &App) -> bool {
    let (Some(entry), Some(original)) = (app.edit_entry.as_ref(), app.edit_original.as_ref())
    else {
        return false;
    };

    entry.name != original.name
        || entry.user != original.user
        || entry.password != original.password
        || entry.url != original.url
        || entry.totp != original.totp
        || entry.notes != original.notes
}

fn handle_exit_confirmation(app: &mut App, key: KeyCode) {
    match crate::input::normalize_shortcut(key) {
        KeyCode::Char('y') => {
            app.confirm_exit = false;
            close_edit(app);
        }

        KeyCode::Char('n') => {
            app.confirm_exit = false;
            reset_edit_state(app);
            app.status = Some("Changes discarded".to_string());
        }

        KeyCode::Esc => {
            app.confirm_exit = false;
            app.status = None;
        }

        _ => {}
    }
}

fn close_edit(app: &mut App) {
    save_edit(app);
    reset_edit_state(app);
}

fn reset_edit_state(app: &mut App) {
    app.edit_entry = None;
    app.edit_original = None;
    app.edit_target = None;
    app.editing_field = false;
    app.field_buffer.clear();
    app.confirm_delete = false;
    app.confirm_exit = false;
    app.reveal_password = false;
    app.edit_state.select(None);
    app.screen = Screen::Index;
}

fn save_edit(app: &mut App) {
    let Some(entry) = app.edit_entry.take() else {
        return;
    };

    let edit_target = app.edit_target;
    let saved_idx = {
        let Some(entries) = app.entries_mut() else {
            app.status = Some("Not saved: database is locked".to_string());
            return;
        };

        match edit_target {
            Some(idx) => {
                if let Some(slot) = entries.get_mut(idx) {
                    *slot = entry;
                }
                Some(idx)
            }
            None => {
                if entry.name.trim().is_empty() {
                    None
                } else {
                    entries.push(entry);
                    Some(entries.len() - 1)
                }
            }
        }
    };

    if let Some(entries) = app.entries_mut() {
        calculate_warnings(entries);
    }
    app.refresh_filter();

    if let Some(idx) = saved_idx {
        persist_entry(app, idx);
    }
}

fn persist_entry(app: &mut App, idx: usize) {
    let Some(session) = app.active_session_mut() else {
        app.status = Some("Not saved: database is locked".to_string());
        return;
    };

    let Some(target) = session.entries.get_mut(idx) else {
        return;
    };

    match save_database(
        &session.path,
        &session.db_key,
        &mut session.database,
        std::slice::from_mut(target),
    ) {
        Ok(()) => app.status = Some("Saved".to_string()),
        Err(err) => app.status = Some(format!("Failed to save: {err:#}")),
    }
}

fn start_delete_confirmation(app: &mut App) {
    if app.edit_entry.is_none() {
        return;
    }

    app.confirm_delete = true;
    app.status = Some("Delete this entry? [y] confirm  [any other key] cancel".to_string());
}

fn handle_delete_confirmation(app: &mut App, key: KeyCode) {
    app.confirm_delete = false;
    app.status = None;

    if let KeyCode::Char('y') = crate::input::normalize_shortcut(key) {
        delete_current_entry(app);
    }
}

fn delete_current_entry(app: &mut App) {
    let Some(entry) = app.edit_entry.take() else {
        reset_edit_state(app);
        return;
    };

    if let Some(id) = entry.id {
        let Some(session) = app.active_session_mut() else {
            app.status = Some("Not deleted: database is locked".to_string());
            app.edit_entry = Some(entry);
            return;
        };

        if let Err(err) = delete_entry(&session.path, &session.db_key, &mut session.database, id) {
            app.status = Some(format!("Failed to delete: {err:#}"));
            app.edit_entry = Some(entry);
            return;
        }
    }

    if let Some(idx) = app.edit_target {
        if let Some(entries) = app.entries_mut()
            && idx < entries.len()
        {
            entries.remove(idx);
        }
    }

    if let Some(entries) = app.entries_mut() {
        calculate_warnings(entries);
    }
    app.refresh_filter();

    reset_edit_state(app);
    app.status = Some("Entry deleted".to_string());
}
