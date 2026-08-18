use crossterm::event::KeyCode;

use crate::app::{App, Screen, UnlockedVault, clear_secret};
use crate::db::{calculate_warnings, unlock_database};

pub fn open_vaults(app: &mut App) {
    let selected = app
        .active_vault_name
        .as_deref()
        .and_then(|active| {
            app.config
                .vaults
                .iter()
                .position(|vault| vault.name == active)
        })
        .or(if app.config.vaults.is_empty() {
            None
        } else {
            Some(0)
        });

    app.vault_state.select(selected);
    app.vault_unlocking = None;
    clear_secret(&mut app.vault_password);
    app.status = None;
    app.screen = Screen::Vaults;
}

pub fn handle_vaults_input(app: &mut App, key: KeyCode) {
    if app.vault_unlocking.is_some() {
        handle_password_input(app, key);
        return;
    }

    match key {
        KeyCode::Down => move_selection(app, 1),
        KeyCode::Up => move_selection(app, -1),
        KeyCode::Enter => select_vault(app),
        KeyCode::Esc => close_vaults(app),
        _ => {}
    }
}

fn move_selection(app: &mut App, delta: i32) {
    let count = app.config.vaults.len();
    if count == 0 {
        return;
    }

    app.status = None;
    let selected = app.vault_state.selected().unwrap_or(0) as i32;
    let next = (selected + delta).rem_euclid(count as i32) as usize;
    app.vault_state.select(Some(next));
}

fn select_vault(app: &mut App) {
    let Some(selected) = app.vault_state.selected() else {
        return;
    };
    let Some(name) = app
        .config
        .vaults
        .get(selected)
        .map(|vault| vault.name.clone())
    else {
        return;
    };

    if app.activate_vault(&name) {
        app.screen = Screen::Index;
        app.status = Some(format!("Switched to {name}"));
        return;
    }

    app.vault_unlocking = Some(name);
    clear_secret(&mut app.vault_password);
    app.status = None;
}

fn handle_password_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char(c) => {
            app.status = None;
            if app.vault_password.len() < app.max_len.max(1) {
                app.vault_password.push(c);
            }
        }
        KeyCode::Backspace => {
            app.status = None;
            app.vault_password.pop();
        }
        KeyCode::Enter => attempt_unlock(app),
        KeyCode::Esc => {
            clear_secret(&mut app.vault_password);
            app.vault_unlocking = None;
            app.status = None;
        }
        _ => {}
    }
}

fn attempt_unlock(app: &mut App) {
    let Some(name) = app.vault_unlocking.clone() else {
        return;
    };
    let Some(vault) = app.config.vault(&name).cloned() else {
        app.status = Some(format!("Vault {name:?} is no longer configured"));
        app.vault_unlocking = None;
        clear_secret(&mut app.vault_password);
        return;
    };

    let mut password = std::mem::take(&mut app.vault_password);
    let result = unlock_database(&vault.path, &password, vault.keyfile.as_deref());
    clear_secret(&mut password);

    match result {
        Ok((database, db_key, mut entries)) => {
            calculate_warnings(&mut entries);
            app.unlock_vault(UnlockedVault {
                name: vault.name.clone(),
                path: vault.path,
                keyfile: vault.keyfile,
                database,
                db_key,
                entries,
            });
            app.vault_unlocking = None;
            app.screen = Screen::Index;
            app.status = Some(format!("Unlocked and switched to {}", vault.name));
        }
        Err(err) => {
            app.status = Some(format!("Couldn't unlock {}: {err}", vault.name));
        }
    }
}

fn close_vaults(app: &mut App) {
    clear_secret(&mut app.vault_password);
    app.vault_unlocking = None;
    app.status = None;
    app.screen = Screen::Index;
}

#[cfg(test)]
mod tests {
    use super::{handle_vaults_input, open_vaults};
    use crate::app::{App, Screen, UnlockedVault};
    use crate::config::{Config, VaultConfig};
    use crate::db::create_database;
    use crate::theme::Theme;
    use crossterm::event::KeyCode;
    use keepass::{Database, DatabaseKey};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "jaiba-vault-switch-test-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("test directory should be created");
            Self(path)
        }

        fn join(&self, path: impl AsRef<Path>) -> PathBuf {
            self.0.join(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn vault_config(name: &str, path: PathBuf) -> VaultConfig {
        VaultConfig {
            name: name.to_string(),
            path,
            keyfile: None,
        }
    }

    fn unlocked(name: &str, path: PathBuf) -> UnlockedVault {
        UnlockedVault {
            name: name.to_string(),
            path,
            keyfile: None,
            database: Database::new(),
            db_key: DatabaseKey::new().with_password("test"),
            entries: Vec::new(),
        }
    }

    #[test]
    fn switches_to_cached_vault_without_password_prompt() {
        let config = Config {
            vaults: vec![
                vault_config("personal", PathBuf::from("personal.kdbx")),
                vault_config("work", PathBuf::from("work.kdbx")),
            ],
            default_vault: Some("personal".to_string()),
            ..Config::default()
        };
        let mut app = App::new(config, Theme::default());
        app.unlock_vault(unlocked("personal", PathBuf::from("personal.kdbx")));
        app.vault_sessions
            .push(unlocked("work", PathBuf::from("work.kdbx")));

        open_vaults(&mut app);
        handle_vaults_input(&mut app, KeyCode::Down);
        handle_vaults_input(&mut app, KeyCode::Enter);

        assert_eq!(app.active_vault_name.as_deref(), Some("work"));
        assert!(app.vault_unlocking.is_none());
        assert!(matches!(app.screen, Screen::Index));
    }

    #[test]
    fn locked_vault_prompts_once_then_is_cached() {
        let dir = TestDir::new();
        let personal_path = dir.join("personal.kdbx");
        let work_path = dir.join("work.kdbx");
        create_database(&work_path, "work secret", None)
            .expect("locked test vault should be created");

        let config = Config {
            vaults: vec![
                vault_config("personal", personal_path.clone()),
                vault_config("work", work_path.clone()),
            ],
            default_vault: Some("personal".to_string()),
            ..Config::default()
        };
        let mut app = App::new(config, Theme::default());
        app.max_len = 64;
        app.unlock_vault(unlocked("personal", personal_path));

        open_vaults(&mut app);
        handle_vaults_input(&mut app, KeyCode::Down);
        handle_vaults_input(&mut app, KeyCode::Enter);
        assert_eq!(app.vault_unlocking.as_deref(), Some("work"));

        for character in "work secret".chars() {
            handle_vaults_input(&mut app, KeyCode::Char(character));
        }
        handle_vaults_input(&mut app, KeyCode::Enter);

        assert_eq!(app.active_vault_name.as_deref(), Some("work"));
        assert!(app.is_vault_unlocked("work"));
        assert!(app.vault_password.is_empty());
        assert!(matches!(app.screen, Screen::Index));

        open_vaults(&mut app);
        handle_vaults_input(&mut app, KeyCode::Enter);
        assert!(app.vault_unlocking.is_none());
    }
}
