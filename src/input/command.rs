use crate::app::App;
use crate::clipboard::{cp_password, cp_totp, cp_url, cp_user};
use crate::db::{Entry, calculate_warnings};

pub fn handle_shortcut(app: &mut App, c: char) {
    match c {
        'u' => cp_user(app),
        'p' => cp_password(app),
        't' => cp_totp(app),
        'r' => cp_url(app),
        'a' => add_entry(app),
        's' => open_settings(app),
        'h' => toggle_archive_selected(app),
        'l' => toggle_archived_view(app),
        _ => {}
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
    let Some(entry) = app.entries.get(entry_idx) else {
        return;
    };

    app.edit_entry = Some(entry.clone());
    app.edit_original = Some(entry.clone());
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
        let current = crate::theme::slugify(current);
        app.available_themes
            .iter()
            .position(|name| crate::theme::slugify(name) == current)
    });

    let selected = current_idx.or(if app.available_themes.is_empty() {
        None
    } else {
        Some(0)
    });

    app.settings_state.select(selected);
    app.screen = crate::app::Screen::Settings;
}

fn toggle_archived_view(app: &mut App) {
    app.show_archived = !app.show_archived;
    app.refresh_filter();
    app.status = Some(
        if app.show_archived {
            "Showing archived entries"
        } else {
            "Showing regular entries"
        }
        .to_string(),
    );
}

/// Ctrl+H: archive the selected entry (or restore it when viewing the archive).
fn toggle_archive_selected(app: &mut App) {
    let Some(selected) = app.index_state.selected() else {
        return;
    };
    let Some(&entry_idx) = app.filtered.get(selected) else {
        return;
    };
    let Some(entry) = app.entries.get_mut(entry_idx) else {
        return;
    };

    entry.archived = !entry.archived;
    let archived = entry.archived;

    calculate_warnings(&mut app.entries);
    app.refresh_filter();

    // Keep the cursor near where it was instead of jumping back to the top.
    if !app.filtered.is_empty() {
        app.index_state
            .select(Some(selected.min(app.filtered.len() - 1)));
    }

    crate::input::edit::persist_entry(app, entry_idx);

    if app.status.as_deref() == Some("Saved") {
        app.status = Some(
            if archived {
                "Entry archived"
            } else {
                "Entry restored from archive"
            }
            .to_string(),
        );
    }
}
