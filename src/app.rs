use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use arboard::Clipboard;
use crossterm::event::{self, Event, MouseButton, MouseEvent, MouseEventKind};
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
use crate::theme::{Theme, load_theme};
use crate::ui::edit::draw_edit;
use crate::ui::index::draw_index;
use crate::ui::login::draw_login;
use crate::ui::settings::draw_settings;

pub enum Screen {
    Login,
    Index,
    Edit,
    Settings,
    OpenDatabase,
}

#[derive(PartialEq, Eq)]
pub enum OpenDatabaseStep {
    Path,
    Password,
    Keyfile,
}

pub struct VaultTab {
    pub path: PathBuf,
    pub keyfile_path: Option<PathBuf>,
    pub label: String,
    pub kdbx: Option<Database>,
    pub db_key: Option<DatabaseKey>,
    pub entries: Vec<Entry>,
    pub filtered: Vec<usize>,
    pub query: String,
    pub index_state: TableState,
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
    pub entries: Vec<Entry>,
    pub filtered: Vec<usize>,
    pub kdbx: Option<Database>,
    pub db_key: Option<DatabaseKey>,
    pub theme: Theme,
    pub config: Config,

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

    pub status: Option<String>,

    pub clipboard: Option<Clipboard>,

    pub clipboard_timer: Option<ClipboardTimer>,

    pub should_quit: bool,
    pub slim_mode: bool,

    pub tabs: Vec<VaultTab>,
    pub active_tab: usize,
    pub open_database_step: OpenDatabaseStep,
    pub open_path_buffer: String,
    pub open_password_buffer: String,
    pub open_keyfile_buffer: String,
    pub open_completion_candidates: Vec<String>,
    pub open_completion_index: usize,
}

fn handle_mouse_input(app: &mut App, mouse: MouseEvent) {
    if !matches!(app.screen, Screen::Index | Screen::Edit | Screen::Settings)
        || mouse.kind != MouseEventKind::Down(MouseButton::Left)
        || mouse.row >= 2
    {
        return;
    }

    if let Some(tab) = crate::ui::tabs::tab_at_column(app, mouse.column) {
        app.switch_tab(tab);
    }
}

impl App {
    fn compute_filtered(&self) -> Vec<usize> {
        let query = self.query.to_lowercase();

        let mut indices: Vec<usize> = self
            .entries
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
            let ea = &self.entries[a];
            let eb = &self.entries[b];

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
        self.entries.get(entry_idx)
    }

    fn store_active_tab(&mut self) {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };

        tab.kdbx = self.kdbx.take();
        tab.db_key = self.db_key.take();
        tab.entries = std::mem::take(&mut self.entries);
        tab.filtered = std::mem::take(&mut self.filtered);
        tab.query = std::mem::take(&mut self.query);
        tab.index_state = std::mem::take(&mut self.index_state);
    }

    fn load_active_tab(&mut self) {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };

        self.kdbx = tab.kdbx.take();
        self.db_key = tab.db_key.take();
        self.entries = std::mem::take(&mut tab.entries);
        self.filtered = std::mem::take(&mut tab.filtered);
        self.query = std::mem::take(&mut tab.query);
        self.index_state = std::mem::take(&mut tab.index_state);
    }

    pub fn active_database_path(&self) -> Option<PathBuf> {
        self.tabs.get(self.active_tab).map(|tab| tab.path.clone())
    }

    pub fn active_keyfile_path(&self) -> Option<PathBuf> {
        self.tabs
            .get(self.active_tab)
            .and_then(|tab| tab.keyfile_path.clone())
    }

    pub fn add_tab(
        &mut self,
        path: PathBuf,
        keyfile_path: Option<PathBuf>,
        db: Database,
        key: DatabaseKey,
        entries: Vec<Entry>,
    ) {
        if !self.tabs.is_empty() {
            self.store_active_tab();
        }

        let label = tab_label(&path);
        self.tabs.push(VaultTab {
            path,
            keyfile_path,
            label,
            kdbx: Some(db),
            db_key: Some(key),
            entries,
            filtered: Vec::new(),
            query: String::new(),
            index_state: TableState::default().with_selected(Some(0)),
        });
        self.refresh_tab_labels();
        self.active_tab = self.tabs.len() - 1;
        self.load_active_tab();
        self.refresh_filter();
    }

    pub fn switch_tab(&mut self, target: usize) {
        if target >= self.tabs.len() || target == self.active_tab {
            return;
        }

        self.store_active_tab();
        self.active_tab = target;
        self.load_active_tab();
    }

    pub fn close_active_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }

        self.store_active_tab();
        self.tabs.remove(self.active_tab);

        if self.tabs.is_empty() {
            self.active_tab = 0;
            self.kdbx = None;
            self.db_key = None;
            self.entries.clear();
            self.filtered.clear();
            self.query.clear();
            self.screen = Screen::Login;
            return;
        }

        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        self.load_active_tab();
    }

    pub fn reset_open_database(&mut self) {
        self.open_database_step = OpenDatabaseStep::Path;
        self.open_path_buffer.clear();
        self.open_password_buffer.clear();
        self.open_keyfile_buffer.clear();
        self.open_completion_candidates.clear();
        self.open_completion_index = 0;
    }

    fn refresh_tab_labels(&mut self) {
        for index in 0..self.tabs.len() {
            let base = tab_label(&self.tabs[index].path);
            let duplicate = self
                .tabs
                .iter()
                .enumerate()
                .any(|(other, tab)| other != index && tab_label(&tab.path) == base);

            self.tabs[index].label = if duplicate {
                self.tabs[index]
                    .path
                    .parent()
                    .map(|parent| format!("{base} ({})", parent.display()))
                    .unwrap_or(base)
            } else {
                base
            };
        }
    }
}

fn tab_label(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

fn maybe_auto_lock(app: &mut App) {
    if !matches!(app.screen, Screen::Index | Screen::Edit | Screen::Settings) {
        return;
    }

    if app.last_activity.elapsed() < app.config.auto_lock {
        return;
    }

    app.store_active_tab();
    app.tabs.clear();
    app.active_tab = 0;
    app.entries.clear();
    app.filtered.clear();
    app.password.clear();
    app.query.clear();
    app.kdbx = None;
    app.db_key = None;
    app.edit_entry = None;
    app.edit_original = None;
    app.edit_target = None;
    app.editing_field = false;
    app.field_buffer.clear();
    app.confirm_delete = false;
    app.confirm_exit = false;
    app.reveal_password = false;
    app.available_themes.clear();
    app.settings_state.select(None);
    app.choosing_theme = false;
    app.theme_state.select(None);
    app.creating_database = false;
    app.confirming_new_db_password = false;
    app.new_db_confirm.clear();
    app.changing_password = false;
    app.password_change_step = PasswordChangeStep::CurrentPassword;
    app.current_password_buffer.clear();
    app.new_password_buffer.clear();
    app.new_password_confirm.clear();
    app.importing_database = false;
    app.import_step = ImportStep::Path;
    app.import_path_buffer.clear();
    app.import_kdbx_password_buffer.clear();
    app.pending_import_path = None;
    app.exporting_database = false;
    app.export_step = ExportStep::Path;
    app.export_path_buffer.clear();
    app.pending_export_path = None;
    app.reset_open_database();
    app.screen = Screen::Login;
    app.login_error = Some("Locked after inactivity".to_string());
}

pub fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    slim_mode: bool,
) -> io::Result<()> {
    let _ = crate::theme::ensure_default_themes();

    let mut config = load_config().unwrap_or_default();

    if config.default_database.is_none() {
        let default_path = crate::util::default_new_database_path();
        if default_path.is_file() {
            config.default_database = Some(default_path);
            let _ = crate::config::save_config(&config);
        }
    }

    let theme = load_theme(config.theme.as_deref()).unwrap_or_default();

    let mut app = App {
        screen: Screen::Login,
        password: String::new(),
        query: String::new(),
        max_len: 0,
        theme,
        config,
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
        entries: Vec::new(),
        filtered: Vec::new(),
        kdbx: None,
        db_key: None,
        creating_database: false,
        confirming_new_db_password: false,
        new_db_confirm: String::new(),
        status: None,
        clipboard: Clipboard::new().ok(),
        clipboard_timer: None,
        should_quit: false,
        slim_mode,
        tabs: Vec::new(),
        active_tab: 0,
        open_database_step: OpenDatabaseStep::Path,
        open_path_buffer: String::new(),
        open_password_buffer: String::new(),
        open_keyfile_buffer: String::new(),
        open_completion_candidates: Vec::new(),
        open_completion_index: 0,
    };

    if app.config.default_database.is_none() {
        app.screen = Screen::OpenDatabase;
    }

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
                Screen::OpenDatabase => crate::ui::open::draw_open_database(frame, &mut app),
            }
        })?;

        if event::poll(TICK_RATE)? {
            match event::read()? {
                Event::Key(key) => {
                    app.last_activity = Instant::now();

                    match app.screen {
                        Screen::Login => handle_login_input(&mut app, key.code),
                        Screen::Index => handle_index_input(&mut app, key),
                        Screen::Edit => handle_edit_input(&mut app, key.code),
                        Screen::Settings => handle_settings_input(&mut app, key.code),
                        Screen::OpenDatabase => {
                            crate::input::open::handle_open_database_input(&mut app, key.code)
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    app.last_activity = Instant::now();
                    handle_mouse_input(&mut app, mouse);
                }
                _ => {}
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
