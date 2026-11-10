use std::fs;
use std::path::Path;

use crossterm::event::KeyCode;

use crate::app::{App, OpenDatabaseStep, Screen};
use crate::db::{calculate_warnings, unlock_database};
use crate::util::expand_tilde;

pub fn handle_open_database_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.reset_open_database();
            app.screen = if app.tabs.is_empty() {
                Screen::Login
            } else {
                Screen::Index
            };
        }
        KeyCode::Enter => advance_open_database(app),
        KeyCode::Tab if app.open_database_step == OpenDatabaseStep::Path => {
            complete_path(app);
        }
        KeyCode::Backspace => {
            current_buffer(app).pop();
            clear_completion(app);
        }
        KeyCode::Char(c) => {
            current_buffer(app).push(c);
            clear_completion(app);
        }
        _ => {}
    }
}

fn clear_completion(app: &mut App) {
    app.open_completion_candidates.clear();
    app.open_completion_index = 0;
}

fn complete_path(app: &mut App) {
    let raw = app.open_path_buffer.clone();
    let (directory_text, prefix) = split_path_input(&raw);
    let directory = expand_tilde(&directory_text);

    let mut matches = fs::read_dir(&directory)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with(&prefix) {
                return None;
            }

            let is_directory = entry.file_type().ok()?.is_dir();
            let is_kdbx = Path::new(&name)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("kdbx"));

            (is_directory || is_kdbx).then_some(name)
        })
        .collect::<Vec<_>>();
    matches.sort_unstable_by_key(|name| name.to_lowercase());

    if matches.is_empty() {
        app.status = Some("no matching .kdbx files or directories".to_string());
        clear_completion(app);
        return;
    }

    let same_matches = app.open_completion_candidates.len() == matches.len()
        && app
            .open_completion_candidates
            .iter()
            .map(|candidate| candidate.rsplit('/').next().unwrap_or(candidate))
            .eq(matches.iter());

    if !same_matches {
        app.open_completion_candidates = matches
            .iter()
            .map(|name| join_completion_path(&directory_text, name))
            .collect();
        app.open_completion_index = 0;

        let common = common_prefix(&matches);
        if common.len() > prefix.len() {
            app.open_path_buffer = completion_path(&directory_text, &directory, &common);
            app.status = None;
            return;
        }
    }

    let candidate = &matches[app.open_completion_index % matches.len()];
    app.open_path_buffer = completion_path(&directory_text, &directory, candidate);
    app.open_completion_index = (app.open_completion_index + 1) % matches.len();
    app.status = None;
}

fn split_path_input(raw: &str) -> (String, String) {
    if raw.ends_with('/') {
        return (raw.to_string(), String::new());
    }

    match raw.rsplit_once('/') {
        Some((directory, prefix)) => (format!("{directory}/"), prefix.to_string()),
        None => (String::new(), raw.to_string()),
    }
}

fn join_completion_path(directory: &str, name: &str) -> String {
    if directory.is_empty() {
        name.to_string()
    } else {
        format!("{directory}{name}")
    }
}

fn completion_path(directory_text: &str, directory: &Path, name: &str) -> String {
    let mut path = join_completion_path(directory_text, name);
    if directory.join(name).is_dir() {
        path.push('/');
    }
    path
}

fn common_prefix(values: &[String]) -> String {
    let Some(first) = values.first() else {
        return String::new();
    };

    first
        .chars()
        .enumerate()
        .take_while(|(index, character)| {
            values
                .iter()
                .all(|value| value.chars().nth(*index) == Some(*character))
        })
        .map(|(_, character)| character)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{common_prefix, join_completion_path, split_path_input};

    #[test]
    fn splits_file_name_from_directory_prefix() {
        assert_eq!(
            split_path_input("~/vaults/wo"),
            ("~/vaults/".to_string(), "wo".to_string())
        );
        assert_eq!(
            split_path_input("vaults/"),
            ("vaults/".to_string(), String::new())
        );
        assert_eq!(split_path_input("wo"), (String::new(), "wo".to_string()));
    }

    #[test]
    fn finds_shared_completion_prefix() {
        assert_eq!(
            common_prefix(&["one.kdbx".into(), "only.kdbx".into()]),
            "on"
        );
        assert_eq!(common_prefix(&["vault.kdbx".into()]), "vault.kdbx");
    }

    #[test]
    fn preserves_typed_directory_prefix() {
        assert_eq!(
            join_completion_path("~/vaults/", "main.kdbx"),
            "~/vaults/main.kdbx"
        );
        assert_eq!(join_completion_path("", "main.kdbx"), "main.kdbx");
    }
}

fn current_buffer(app: &mut App) -> &mut String {
    match app.open_database_step {
        OpenDatabaseStep::Path => &mut app.open_path_buffer,
        OpenDatabaseStep::Password => &mut app.open_password_buffer,
        OpenDatabaseStep::Keyfile => &mut app.open_keyfile_buffer,
    }
}

fn advance_open_database(app: &mut App) {
    match app.open_database_step {
        OpenDatabaseStep::Path => {
            if app.open_path_buffer.trim().is_empty() {
                app.status = Some("database path can't be empty".to_string());
                return;
            }
            app.open_database_step = OpenDatabaseStep::Password;
            app.status = None;
        }
        OpenDatabaseStep::Password => {
            if app.open_password_buffer.is_empty() {
                app.status = Some("master password can't be empty".to_string());
                return;
            }
            app.open_database_step = OpenDatabaseStep::Keyfile;
            app.status = None;
        }
        OpenDatabaseStep::Keyfile => finish_open_database(app),
    }
}

fn finish_open_database(app: &mut App) {
    let path = expand_tilde(app.open_path_buffer.trim());
    let keyfile = if app.open_keyfile_buffer.trim().is_empty() {
        None
    } else {
        Some(expand_tilde(app.open_keyfile_buffer.trim()))
    };

    let comparison_path = path.canonicalize().unwrap_or_else(|_| path.clone());
    if app
        .tabs
        .iter()
        .any(|tab| tab.path.canonicalize().unwrap_or_else(|_| tab.path.clone()) == comparison_path)
    {
        app.status = Some("that database is already open".to_string());
        return;
    }

    match unlock_database(&path, &app.open_password_buffer, keyfile.as_deref()) {
        Ok((db, key, mut entries)) => {
            calculate_warnings(&mut entries);
            app.add_tab(path, keyfile, db, key, entries);
            app.reset_open_database();
            app.status = Some("database opened".to_string());
            app.screen = Screen::Index;
        }
        Err(err) => {
            app.open_password_buffer.clear();
            app.status = Some(format!("couldn't open database: {err:#}"));
        }
    }
}
