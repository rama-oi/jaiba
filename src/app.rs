use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use arboard::Clipboard;
use crossterm::event::{self, Event};
use keepass::{Database, DatabaseKey};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    style::Style,
    widgets::{Block, ListState, TableState},
};

use crate::clipboard::{ClipboardTimer, maybe_clear_clipboard};
use crate::config::{Config, load_config};
use crate::db::Entry;
use crate::input::edit::handle_edit_input;
use crate::input::index::handle_index_input;
use crate::input::login::handle_login_input;
use crate::input::settings::handle_settings_input;
use crate::input::vaults::handle_vaults_input;
use crate::theme::{Theme, load_theme};
use crate::ui::edit::draw_edit;
use crate::ui::index::draw_index;
use crate::ui::login::draw_login;
use crate::ui::settings::draw_settings;
use crate::ui::vaults::draw_vaults;
use zeroize::Zeroize;

pub enum Screen {
    Login,
    Index,
    Edit,
    Settings,
    Vaults,
}

#[derive(PartialEq, Eq)]
pub enum PasswordChangeStep {
    CurrentPassword,
    NewPassword,
    ConfirmNewPassword,
}

#[derive(PartialEq, Eq)]
pub enum ImportStep {
    Path,
    KdbxPassword,
}

#[derive(PartialEq, Eq)]
pub enum ExportStep {
    Path,
    Confirm,
}

pub struct UnlockedVault {
    pub name: String,
    pub path: PathBuf,
    pub keyfile: Option<PathBuf>,
    pub database: Database,
    pub db_key: DatabaseKey,
    pub entries: Vec<Entry>,
}

pub struct App {
    pub screen: Screen,
    pub password: String,
    pub query: String,
    pub max_len: usize,
    pub index_state: TableState,
    pub edit_state: ListState,
    pub reveal_password: bool,
    pub edit_entry: Option<Entry>,
    pub edit_original: Option<Entry>,
    pub edit_target: Option<usize>,
    pub editing_field: bool,
    pub field_buffer: String,
    pub confirm_delete: bool,
    pub confirm_exit: bool,
    pub filtered: Vec<usize>,
    pub theme: Theme,
    pub config: Config,

    pub vault_sessions: Vec<UnlockedVault>,
    pub active_vault_name: Option<String>,
    pub vault_state: ListState,
    pub vault_unlocking: Option<String>,
    pub vault_password: String,

    pub creating_database: bool,
    pub confirming_new_db_password: bool,
    pub new_db_confirm: String,

    pub available_themes: Vec<String>,
    pub settings_state: ListState,
    pub choosing_theme: bool,
    pub theme_state: ListState,

    pub changing_password: bool,
    pub password_change_step: PasswordChangeStep,
    pub current_password_buffer: String,
    pub new_password_buffer: String,
    pub new_password_confirm: String,

    pub importing_database: bool,
    pub import_step: ImportStep,
    pub import_path_buffer: String,
    pub import_kdbx_password_buffer: String,
    pub pending_import_path: Option<PathBuf>,

    pub exporting_database: bool,
    pub export_step: ExportStep,
    pub export_path_buffer: String,
    pub pending_export_path: Option<PathBuf>,

    pub login_error: Option<String>,

    pub last_activity: Instant,

    pub command_mode: bool,
    pub command_buffer: String,

    pub status: Option<String>,

    pub clipboard: Option<Clipboard>,

    pub clipboard_timer: Option<ClipboardTimer>,

    pub should_quit: bool,
}

impl App {
    pub fn new(config: Config, theme: Theme) -> Self {
        let active_vault_name = config.default_vault().map(|vault| vault.name.clone());

        Self {
            screen: Screen::Login,
            password: String::new(),
            query: String::new(),
            max_len: 0,
            theme,
            config,
            vault_sessions: Vec::new(),
            active_vault_name,
            vault_state: ListState::default(),
            vault_unlocking: None,
            vault_password: String::new(),
            available_themes: Vec::new(),
            settings_state: ListState::default(),
            choosing_theme: false,
            theme_state: ListState::default(),
            changing_password: false,
            password_change_step: PasswordChangeStep::CurrentPassword,
            current_password_buffer: String::new(),
            new_password_buffer: String::new(),
            new_password_confirm: String::new(),
            importing_database: false,
            import_step: ImportStep::Path,
            import_path_buffer: String::new(),
            import_kdbx_password_buffer: String::new(),
            pending_import_path: None,
            exporting_database: false,
            export_step: ExportStep::Path,
            export_path_buffer: String::new(),
            pending_export_path: None,
            login_error: None,
            last_activity: Instant::now(),
            index_state: TableState::default().with_selected(Some(0)),
            edit_state: ListState::default(),
            reveal_password: false,
            edit_entry: None,
            edit_original: None,
            edit_target: None,
            editing_field: false,
            field_buffer: String::new(),
            confirm_delete: false,
            confirm_exit: false,
            filtered: Vec::new(),
            creating_database: false,
            confirming_new_db_password: false,
            new_db_confirm: String::new(),
            command_mode: false,
            command_buffer: String::new(),
            status: None,
            clipboard: Clipboard::new().ok(),
            clipboard_timer: None,
            should_quit: false,
        }
    }

    pub fn active_vault_config(&self) -> Option<&crate::config::VaultConfig> {
        let name = self.active_vault_name.as_deref()?;
        self.config.vault(name)
    }

    pub fn active_vault_config_mut(&mut self) -> Option<&mut crate::config::VaultConfig> {
        let name = self.active_vault_name.clone()?;
        self.config.vault_mut(&name)
    }

    pub fn active_session(&self) -> Option<&UnlockedVault> {
        let name = self.active_vault_name.as_deref()?;
        self.vault_sessions.iter().find(|vault| vault.name == name)
    }

    pub fn active_session_mut(&mut self) -> Option<&mut UnlockedVault> {
        let name = self.active_vault_name.as_deref()?;
        self.vault_sessions
            .iter_mut()
            .find(|vault| vault.name == name)
    }

    pub fn entries(&self) -> &[Entry] {
        self.active_session()
            .map(|vault| vault.entries.as_slice())
            .unwrap_or_default()
    }

    pub fn entries_mut(&mut self) -> Option<&mut Vec<Entry>> {
        self.active_session_mut().map(|vault| &mut vault.entries)
    }

    pub fn is_vault_unlocked(&self, name: &str) -> bool {
        self.vault_sessions.iter().any(|vault| vault.name == name)
    }

    pub fn unlock_vault(&mut self, vault: UnlockedVault) {
        let name = vault.name.clone();

        if let Some(existing) = self
            .vault_sessions
            .iter_mut()
            .find(|existing| existing.name == name)
        {
            *existing = vault;
        } else {
            self.vault_sessions.push(vault);
        }

        self.activate_vault(&name);
    }

    pub fn activate_vault(&mut self, name: &str) -> bool {
        if !self.is_vault_unlocked(name) {
            return false;
        }

        self.active_vault_name = Some(name.to_string());
        self.query.clear();
        self.refresh_filter();
        true
    }

    pub fn sync_active_session_metadata(&mut self) {
        let Some(config) = self.active_vault_config().cloned() else {
            return;
        };
        let Some(session) = self.active_session_mut() else {
            return;
        };

        session.path = config.path;
        session.keyfile = config.keyfile;
    }

    fn compute_filtered(&self) -> Vec<usize> {
        let query = self.query.to_lowercase();

        let mut indices: Vec<usize> = self
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                query.is_empty()
                    || entry.name.to_lowercase().contains(&query)
                    || entry.user.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();

        indices.sort_by(|&a, &b| {
            let entries = self.entries();
            let ea = &entries[a];
            let eb = &entries[b];

            eb.password_reuse_count
                .cmp(&ea.password_reuse_count)
                .then_with(|| eb.duplicate_user_count.cmp(&ea.duplicate_user_count))
                .then_with(|| ea.name.to_lowercase().cmp(&eb.name.to_lowercase()))
        });

        indices
    }

    pub fn refresh_filter(&mut self) {
        self.filtered = self.compute_filtered();

        let count = self.filtered.len();
        self.index_state
            .select(if count == 0 { None } else { Some(0) });
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        let selected = self.index_state.selected()?;
        let entry_idx = *self.filtered.get(selected)?;
        self.entries().get(entry_idx)
    }
}

fn maybe_auto_lock(app: &mut App) {
    if !matches!(
        app.screen,
        Screen::Index | Screen::Edit | Screen::Settings | Screen::Vaults
    ) {
        return;
    }

    if app.last_activity.elapsed() < app.config.auto_lock {
        return;
    }

    lock_all_vaults(app, "Locked after inactivity");
}

pub fn clear_secret(secret: &mut String) {
    secret.zeroize();
}

pub fn lock_all_vaults(app: &mut App, message: &str) {
    app.vault_sessions.clear();
    app.filtered.clear();
    clear_secret(&mut app.password);
    clear_secret(&mut app.vault_password);
    app.query.clear();
    app.edit_entry = None;
    app.edit_original = None;
    app.edit_target = None;
    app.editing_field = false;
    clear_secret(&mut app.field_buffer);
    app.confirm_delete = false;
    app.confirm_exit = false;
    app.reveal_password = false;
    app.available_themes.clear();
    app.settings_state.select(None);
    app.choosing_theme = false;
    app.theme_state.select(None);
    app.creating_database = false;
    app.confirming_new_db_password = false;
    clear_secret(&mut app.new_db_confirm);
    app.changing_password = false;
    app.password_change_step = PasswordChangeStep::CurrentPassword;
    clear_secret(&mut app.current_password_buffer);
    clear_secret(&mut app.new_password_buffer);
    clear_secret(&mut app.new_password_confirm);
    app.importing_database = false;
    app.import_step = ImportStep::Path;
    app.import_path_buffer.clear();
    clear_secret(&mut app.import_kdbx_password_buffer);
    app.pending_import_path = None;
    app.exporting_database = false;
    app.export_step = ExportStep::Path;
    app.export_path_buffer.clear();
    app.pending_export_path = None;
    app.vault_unlocking = None;
    app.vault_state.select(None);
    app.command_mode = false;
    app.command_buffer.clear();
    app.status = None;
    app.clipboard_timer = None;
    app.screen = Screen::Login;
    app.login_error = Some(message.to_string());
}

pub fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let _ = crate::theme::ensure_default_themes();

    let mut config = load_config().unwrap_or_default();

    if config.vaults.is_empty() {
        let default_path = crate::util::default_new_database_path();
        if default_path.is_file() {
            config.ensure_default_vault(default_path);
            let _ = crate::config::save_config(&config);
        }
    }

    let theme = load_theme(config.theme.as_deref()).unwrap_or_default();

    let mut app = App::new(config, theme);

    const TICK_RATE: Duration = Duration::from_millis(200);

    loop {
        terminal.draw(|frame| {
            frame.render_widget(
                Block::default().style(Style::default().bg(app.theme.background)),
                frame.area(),
            );

            match app.screen {
                Screen::Login => draw_login(frame, &mut app),
                Screen::Index => draw_index(frame, &mut app),
                Screen::Edit => draw_edit(frame, &mut app),
                Screen::Settings => draw_settings(frame, &mut app),
                Screen::Vaults => draw_vaults(frame, &mut app),
            }
        })?;

        if event::poll(TICK_RATE)? {
            if let Event::Key(key) = event::read()? {
                app.last_activity = Instant::now();

                match app.screen {
                    Screen::Login => handle_login_input(&mut app, key.code),
                    Screen::Index => handle_index_input(&mut app, key.code),
                    Screen::Edit => handle_edit_input(&mut app, key.code),
                    Screen::Settings => handle_settings_input(&mut app, key.code),
                    Screen::Vaults => handle_vaults_input(&mut app, key.code),
                }
            }
        }

        maybe_clear_clipboard(&mut app);
        maybe_auto_lock(&mut app);

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{App, Screen, UnlockedVault, maybe_auto_lock};
    use crate::config::{Config, VaultConfig};
    use crate::input::command::handle_command_input;
    use crate::theme::Theme;
    use crossterm::event::KeyCode;
    use keepass::{Database, DatabaseKey};
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    fn config() -> Config {
        Config {
            vaults: vec![
                VaultConfig {
                    name: "personal".to_string(),
                    path: PathBuf::from("personal.kdbx"),
                    keyfile: None,
                },
                VaultConfig {
                    name: "work".to_string(),
                    path: PathBuf::from("work.kdbx"),
                    keyfile: None,
                },
            ],
            default_vault: Some("personal".to_string()),
            ..Config::default()
        }
    }

    fn unlocked(name: &str) -> UnlockedVault {
        UnlockedVault {
            name: name.to_string(),
            path: PathBuf::from(format!("{name}.kdbx")),
            keyfile: None,
            database: Database::new(),
            db_key: DatabaseKey::new().with_password("test"),
            entries: Vec::new(),
        }
    }

    fn app_with_two_sessions() -> App {
        let mut app = App::new(config(), Theme::default());
        app.unlock_vault(unlocked("personal"));
        app.vault_sessions.push(unlocked("work"));
        app.screen = Screen::Index;
        app
    }

    #[test]
    fn manual_lock_command_clears_every_unlocked_vault() {
        let mut app = app_with_two_sessions();

        handle_command_input(&mut app, KeyCode::Char('l'));

        assert!(app.vault_sessions.is_empty());
        assert!(matches!(app.screen, Screen::Login));
        assert_eq!(app.login_error.as_deref(), Some("Locked"));
        assert_eq!(app.active_vault_name.as_deref(), Some("personal"));
    }

    #[test]
    fn auto_lock_clears_every_unlocked_vault_from_switcher() {
        let mut app = app_with_two_sessions();
        app.screen = Screen::Vaults;
        app.config.auto_lock = Duration::ZERO;
        app.last_activity = Instant::now() - Duration::from_secs(1);
        app.vault_unlocking = Some("work".to_string());
        app.vault_password = "plaintext secret".to_string();

        maybe_auto_lock(&mut app);

        assert!(app.vault_sessions.is_empty());
        assert!(app.vault_password.is_empty());
        assert!(app.vault_unlocking.is_none());
        assert!(matches!(app.screen, Screen::Login));
        assert_eq!(app.login_error.as_deref(), Some("Locked after inactivity"));
    }
}
